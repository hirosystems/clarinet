use std::borrow::Borrow;
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::io::{Read, Write};
use std::{cmp, error, fmt, str};

use serde_json::Value as JSONValue;

use crate::clarity::database::{ClarityDeserializable, ClaritySerializable};
use crate::clarity::errors::{
    CheckErrors, Error as ClarityError, IncomparableError, InterpreterError, InterpreterResult,
    RuntimeErrorType,
};
use crate::clarity::representations::{ClarityName, ContractName, MAX_STRING_LEN};
use crate::clarity::types::{
    BufferLength, CharType, OptionalData, PrincipalData, QualifiedContractIdentifier, ResponseData,
    SequenceData, SequenceSubtype, StandardPrincipalData, StringSubtype, StringUTF8Length,
    TupleData, TypeSignature, Value, BOUND_VALUE_SERIALIZATION_BYTES, MAX_VALUE_SIZE,
};
use crate::clarity::util::hash::{hex_bytes, to_hex};
use crate::clarity::util::retry::BoundReader;

use crate::clarity::codec::{Error as codec_error, StacksMessageCodec};
use crate::clarity::types::byte_len_of_serialization;

/// Errors that may occur in serialization or deserialization
/// If deserialization failed because the described type is a bad type and
///   a CheckError is thrown, it gets wrapped in BadTypeError.
/// Any IOErrrors from the supplied buffer will manifest as IOError variants,
///   except for EOF -- if the deserialization code experiences an EOF, it is caught
///   and rethrown as DeserializationError
#[derive(Debug, PartialEq)]
pub enum SerializationError {
    IOError(IncomparableError<std::io::Error>),
    BadTypeError(CheckErrors),
    DeserializationError(String),
    DeserializeExpected(TypeSignature),
    LeftoverBytesInDeserialization,
}

lazy_static! {
    pub static ref NONE_SERIALIZATION_LEN: u64 = Value::none().serialize_to_vec().len() as u64;
}

impl std::fmt::Display for SerializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SerializationError::IOError(e) => {
                write!(f, "Serialization error caused by IO: {}", e.err)
            }
            SerializationError::BadTypeError(e) => {
                write!(f, "Deserialization error, bad type, caused by: {}", e)
            }
            SerializationError::DeserializationError(e) => {
                write!(f, "Deserialization error: {}", e)
            }
            SerializationError::DeserializeExpected(e) => write!(
                f,
                "Deserialization expected the type of the input to be: {}",
                e
            ),
            SerializationError::LeftoverBytesInDeserialization => {
                write!(f, "Deserialization error: bytes left over in buffer")
            }
        }
    }
}

impl error::Error for SerializationError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            SerializationError::IOError(e) => Some(&e.err),
            SerializationError::BadTypeError(e) => Some(e),
            _ => None,
        }
    }
}

// Note: a byte stream that describes a longer type than
//   there are available bytes to read will result in an IOError(UnexpectedEOF)
impl From<std::io::Error> for SerializationError {
    fn from(err: std::io::Error) -> Self {
        SerializationError::IOError(IncomparableError { err })
    }
}

impl From<&str> for SerializationError {
    fn from(e: &str) -> Self {
        SerializationError::DeserializationError(e.into())
    }
}

impl From<CheckErrors> for SerializationError {
    fn from(e: CheckErrors) -> Self {
        SerializationError::BadTypeError(e)
    }
}

define_u8_enum!(TypePrefix {
    Int = 0,
    UInt = 1,
    Buffer = 2,
    BoolTrue = 3,
    BoolFalse = 4,
    PrincipalStandard = 5,
    PrincipalContract = 6,
    ResponseOk = 7,
    ResponseErr = 8,
    OptionalNone = 9,
    OptionalSome = 10,
    List = 11,
    Tuple = 12,
    StringASCII = 13,
    StringUTF8 = 14
});

impl From<&PrincipalData> for TypePrefix {
    fn from(v: &PrincipalData) -> TypePrefix {
        use super::PrincipalData::*;
        match v {
            Standard(_) => TypePrefix::PrincipalStandard,
            Contract(_) => TypePrefix::PrincipalContract,
        }
    }
}

impl From<&Value> for TypePrefix {
    fn from(v: &Value) -> TypePrefix {
        use super::CharType;
        use super::SequenceData::*;
        use super::Value::*;

        match v {
            Int(_) => TypePrefix::Int,
            UInt(_) => TypePrefix::UInt,
            Bool(value) => {
                if *value {
                    TypePrefix::BoolTrue
                } else {
                    TypePrefix::BoolFalse
                }
            }
            Principal(p) => TypePrefix::from(p),
            Response(response) => {
                if response.committed {
                    TypePrefix::ResponseOk
                } else {
                    TypePrefix::ResponseErr
                }
            }
            Optional(OptionalData { data: None }) => TypePrefix::OptionalNone,
            Optional(OptionalData { data: Some(_) }) => TypePrefix::OptionalSome,
            Tuple(_) => TypePrefix::Tuple,
            Sequence(Buffer(_)) => TypePrefix::Buffer,
            Sequence(List(_)) => TypePrefix::List,
            Sequence(String(CharType::ASCII(_))) => TypePrefix::StringASCII,
            Sequence(String(CharType::UTF8(_))) => TypePrefix::StringUTF8,
        }
    }
}

/// Not a public trait,
///   this is just used to simplify serializing some types that
///   are repeatedly serialized or deserialized.
trait ClarityValueSerializable<T: std::marker::Sized> {
    fn serialize_write<W: Write>(&self, w: &mut W) -> std::io::Result<()>;
    fn deserialize_read<R: Read>(r: &mut R) -> Result<T, SerializationError>;
}

impl ClarityValueSerializable<StandardPrincipalData> for StandardPrincipalData {
    fn serialize_write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        w.write_all(&[self.0])?;
        w.write_all(&self.1)
    }

    fn deserialize_read<R: Read>(r: &mut R) -> Result<Self, SerializationError> {
        let mut version = [0; 1];
        let mut data = [0; 20];
        r.read_exact(&mut version)?;
        r.read_exact(&mut data)?;
        Ok(StandardPrincipalData(version[0], data))
    }
}

macro_rules! serialize_guarded_string {
    ($Name:ident) => {
        impl ClarityValueSerializable<$Name> for $Name {
            fn serialize_write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
                w.write_all(&self.len().to_be_bytes())?;
                // self.as_bytes() is always len bytes, because this is only used for GuardedStrings
                //   which are a subset of ASCII
                w.write_all(self.as_str().as_bytes())
            }

            fn deserialize_read<R: Read>(r: &mut R) -> Result<Self, SerializationError> {
                let mut len = [0; 1];
                r.read_exact(&mut len)?;
                let len = u8::from_be_bytes(len);
                if len > MAX_STRING_LEN {
                    return Err(SerializationError::DeserializationError(
                        "String too long".to_string(),
                    ));
                }

                let mut data = vec![0; len as usize];
                r.read_exact(&mut data)?;

                String::from_utf8(data)
                    .map_err(|_| "Non-UTF8 string data".into())
                    .and_then(|x| $Name::try_from(x).map_err(|_| "Illegal Clarity string".into()))
            }
        }
    };
}

serialize_guarded_string!(ClarityName);
serialize_guarded_string!(ContractName);

impl PrincipalData {
    fn inner_consensus_serialize<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        w.write_all(&[TypePrefix::from(self) as u8])?;
        match self {
            PrincipalData::Standard(p) => p.serialize_write(w),
            PrincipalData::Contract(contract_identifier) => {
                contract_identifier.issuer.serialize_write(w)?;
                contract_identifier.name.serialize_write(w)
            }
        }
    }

    fn inner_consensus_deserialize<R: Read>(
        r: &mut R,
    ) -> Result<PrincipalData, SerializationError> {
        let mut header = [0];
        r.read_exact(&mut header)?;

        let prefix = TypePrefix::from_u8(header[0]).ok_or_else(|| "Bad principal prefix")?;

        match prefix {
            TypePrefix::PrincipalStandard => {
                StandardPrincipalData::deserialize_read(r).map(PrincipalData::from)
            }
            TypePrefix::PrincipalContract => {
                let issuer = StandardPrincipalData::deserialize_read(r)?;
                let name = ContractName::deserialize_read(r)?;
                Ok(PrincipalData::from(QualifiedContractIdentifier {
                    issuer,
                    name,
                }))
            }
            _ => Err("Bad principal prefix".into()),
        }
    }
}

impl StacksMessageCodec for PrincipalData {
    fn consensus_serialize<W: Write>(&self, fd: &mut W) -> Result<(), codec_error> {
        self.inner_consensus_serialize(fd)
            .map_err(codec_error::WriteError)
    }

    fn consensus_deserialize<R: Read>(fd: &mut R) -> Result<PrincipalData, codec_error> {
        PrincipalData::inner_consensus_deserialize(fd)
            .map_err(|e| codec_error::DeserializeError(e.to_string()))
    }
}

macro_rules! check_match {
    ($item:expr, $Pattern:pat) => {
        match $item {
            None => Ok(()),
            Some($Pattern) => Ok(()),
            Some(x) => Err(SerializationError::DeserializeExpected(x.clone())),
        }
    };
}

impl TypeSignature {
    /// Return the maximum length of the consensus serialization of a
    /// Clarity value of this type. The returned length *may* not fit
    /// in a Clarity buffer! For example, the maximum serialized
    /// size of a `(buff 1024*1024)` is `1+1024*1024` because of the
    /// type prefix byte. However, that is 1 byte larger than the maximum
    /// buffer size in Clarity.
    pub fn max_serialized_size(&self) -> Result<u32, CheckErrors> {
        let type_prefix_size = 1;

        let max_output_size = match self {
            TypeSignature::NoType => {
                // A `NoType` should *never* actually be evaluated
                // (`NoType` corresponds to the Some branch of a
                // `none` that is never matched with a corresponding
                // `some` or similar with `result` types).  So, when
                // serializing an object with a `NoType`, the other
                // branch should always be used.
                return Err(CheckErrors::CouldNotDetermineSerializationType);
            }
            TypeSignature::IntType => 16,
            TypeSignature::UIntType => 16,
            TypeSignature::BoolType => 0,
            TypeSignature::SequenceType(SequenceSubtype::ListType(list_type)) => {
                // u32 length as big-endian bytes
                let list_length_encode = 4;
                list_type
                    .get_max_len()
                    .checked_mul(list_type.get_list_item_type().max_serialized_size()?)
                    .and_then(|x| x.checked_add(list_length_encode))
                    .ok_or_else(|| CheckErrors::ValueTooLarge)?
            }
            TypeSignature::SequenceType(SequenceSubtype::BufferType(buff_length)) => {
                // u32 length as big-endian bytes
                let buff_length_encode = 4;
                u32::from(buff_length)
                    .checked_add(buff_length_encode)
                    .ok_or_else(|| CheckErrors::ValueTooLarge)?
            }
            TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::ASCII(
                length,
            ))) => {
                // u32 length as big-endian bytes
                let str_length_encode = 4;
                // ascii is 1-byte per character
                u32::from(length)
                    .checked_add(str_length_encode)
                    .ok_or_else(|| CheckErrors::ValueTooLarge)?
            }
            TypeSignature::SequenceType(SequenceSubtype::StringType(StringSubtype::UTF8(
                length,
            ))) => {
                // u32 length as big-endian bytes
                let str_length_encode = 4;
                // utf-8 is maximum 4 bytes per codepoint (which is the length)
                u32::from(length)
                    .checked_mul(4)
                    .and_then(|x| x.checked_add(str_length_encode))
                    .ok_or_else(|| CheckErrors::ValueTooLarge)?
            }
            TypeSignature::PrincipalType => {
                // version byte + 20 byte hash160
                let maximum_issuer_size = 21;
                let contract_name_length_encode = 1;
                // contract name maximum length is `MAX_STRING_LEN` (128), and ASCII
                let maximum_contract_name = MAX_STRING_LEN as u32;
                maximum_contract_name + maximum_issuer_size + contract_name_length_encode
            }
            TypeSignature::TupleType(tuple_type) => {
                let type_map = tuple_type.get_type_map();
                // u32 length as big-endian bytes
                let tuple_length_encode: u32 = 4;
                let mut total_size = tuple_length_encode;
                for (key, value) in type_map.iter() {
                    let value_size = value.max_serialized_size()?;
                    total_size = total_size
                        .checked_add(1) // length of key-name
                        .and_then(|x| x.checked_add(key.len() as u32)) // ClarityName is ascii-only, so 1 byte per length
                        .and_then(|x| x.checked_add(value_size))
                        .ok_or_else(|| CheckErrors::ValueTooLarge)?;
                }
                total_size
            }
            TypeSignature::OptionalType(ref some_type) => {
                match some_type.max_serialized_size() {
                    Ok(size) => size,
                    // if NoType, then this is just serializing a none
                    // value, which is only the type prefix
                    Err(CheckErrors::CouldNotDetermineSerializationType) => 0,
                    Err(e) => return Err(e),
                }
            }
            TypeSignature::ResponseType(ref response_types) => {
                let (ok_type, err_type) = response_types.as_ref();
                let (ok_type_max_size, no_ok_type) = match ok_type.max_serialized_size() {
                    Ok(size) => (size, false),
                    Err(CheckErrors::CouldNotDetermineSerializationType) => (0, true),
                    Err(e) => return Err(e),
                };
                let err_type_max_size = match err_type.max_serialized_size() {
                    Ok(size) => size,
                    Err(CheckErrors::CouldNotDetermineSerializationType) => {
                        if no_ok_type {
                            // if both the ok type and the error type are NoType,
                            //  throw a CheckError. This should not be possible, but the check
                            //  is done out of caution.
                            return Err(CheckErrors::CouldNotDetermineSerializationType);
                        } else {
                            0
                        }
                    }
                    Err(e) => return Err(e),
                };
                cmp::max(ok_type_max_size, err_type_max_size)
            }
            TypeSignature::TraitReferenceType(_) => {
                return Err(CheckErrors::CouldNotDetermineSerializationType)
            }
        };

        max_output_size
            .checked_add(type_prefix_size)
            .ok_or_else(|| CheckErrors::ValueTooLarge)
    }
}

impl Value {
    pub fn deserialize_read<R: Read>(
        r: &mut R,
        expected_type: Option<&TypeSignature>,
    ) -> Result<Value, SerializationError> {
        Self::deserialize_read_count(r, expected_type).map(|(value, _)| value)
    }

    /// Deserialize just like `deserialize_read` but also
    ///  return the bytes read
    pub fn deserialize_read_count<R: Read>(
        r: &mut R,
        expected_type: Option<&TypeSignature>,
    ) -> Result<(Value, u64), SerializationError> {
        let mut bound_reader = BoundReader::from_reader(r, BOUND_VALUE_SERIALIZATION_BYTES as u64);
        let value = Value::inner_deserialize_read(&mut bound_reader, expected_type, 0)?;
        let bytes_read = bound_reader.num_read();
        if let Some(expected_type) = expected_type {
            let expect_size = match expected_type.max_serialized_size() {
                Ok(x) => x,
                Err(e) => {
                    return Ok((value, bytes_read));
                }
            };

            assert!(
                expect_size as u64 >= bytes_read,
                "Deserialized more bytes than expected size during deserialization. Expected size = {}, bytes read = {}, type = {}",
                expect_size,
                bytes_read,
                expected_type,
            );
        }

        Ok((value, bytes_read))
    }

    fn inner_deserialize_read<R: Read>(
        r: &mut R,
        expected_type: Option<&TypeSignature>,
        depth: u8,
    ) -> Result<Value, SerializationError> {
        use super::PrincipalData::*;
        use super::Value::*;

        if depth >= 16 {
            return Err(CheckErrors::TypeSignatureTooDeep.into());
        }

        let mut header = [0];
        r.read_exact(&mut header)?;

        let prefix = TypePrefix::from_u8(header[0]).ok_or_else(|| "Bad type prefix")?;

        match prefix {
            TypePrefix::Int => {
                check_match!(expected_type, TypeSignature::IntType)?;
                let mut buffer = [0; 16];
                r.read_exact(&mut buffer)?;
                Ok(Int(i128::from_be_bytes(buffer)))
            }
            TypePrefix::UInt => {
                check_match!(expected_type, TypeSignature::UIntType)?;
                let mut buffer = [0; 16];
                r.read_exact(&mut buffer)?;
                Ok(UInt(u128::from_be_bytes(buffer)))
            }
            TypePrefix::Buffer => {
                let mut buffer_len = [0; 4];
                r.read_exact(&mut buffer_len)?;
                let buffer_len = BufferLength::try_from(u32::from_be_bytes(buffer_len))?;

                if let Some(x) = expected_type {
                    let passed_test = match x {
                        TypeSignature::SequenceType(SequenceSubtype::BufferType(expected_len)) => {
                            u32::from(&buffer_len) <= u32::from(expected_len)
                        }
                        _ => false,
                    };
                    if !passed_test {
                        return Err(SerializationError::DeserializeExpected(x.clone()));
                    }
                }

                let mut data = vec![0; u32::from(buffer_len) as usize];

                r.read_exact(&mut data[..])?;

                Value::buff_from(data).map_err(|_| "Bad buffer".into())
            }
            TypePrefix::BoolTrue => {
                check_match!(expected_type, TypeSignature::BoolType)?;
                Ok(Bool(true))
            }
            TypePrefix::BoolFalse => {
                check_match!(expected_type, TypeSignature::BoolType)?;
                Ok(Bool(false))
            }
            TypePrefix::PrincipalStandard => {
                check_match!(expected_type, TypeSignature::PrincipalType)?;
                StandardPrincipalData::deserialize_read(r).map(Value::from)
            }
            TypePrefix::PrincipalContract => {
                check_match!(expected_type, TypeSignature::PrincipalType)?;
                let issuer = StandardPrincipalData::deserialize_read(r)?;
                let name = ContractName::deserialize_read(r)?;
                Ok(Value::from(QualifiedContractIdentifier { issuer, name }))
            }
            TypePrefix::ResponseOk | TypePrefix::ResponseErr => {
                let committed = prefix == TypePrefix::ResponseOk;

                let expect_contained_type = match expected_type {
                    None => None,
                    Some(x) => {
                        let contained_type = match (committed, x) {
                            (true, TypeSignature::ResponseType(types)) => Ok(&types.0),
                            (false, TypeSignature::ResponseType(types)) => Ok(&types.1),
                            _ => Err(SerializationError::DeserializeExpected(x.clone())),
                        }?;
                        Some(contained_type)
                    }
                };

                let data = Value::inner_deserialize_read(r, expect_contained_type, depth + 1)?;
                let value = if committed {
                    Value::okay(data)
                } else {
                    Value::error(data)
                }
                .map_err(|_x| "Value too large")?;

                Ok(value)
            }
            TypePrefix::OptionalNone => {
                check_match!(expected_type, TypeSignature::OptionalType(_))?;
                Ok(Value::none())
            }
            TypePrefix::OptionalSome => {
                let expect_contained_type = match expected_type {
                    None => None,
                    Some(x) => {
                        let contained_type = match x {
                            TypeSignature::OptionalType(some_type) => Ok(some_type.as_ref()),
                            _ => Err(SerializationError::DeserializeExpected(x.clone())),
                        }?;
                        Some(contained_type)
                    }
                };

                let value = Value::some(Value::inner_deserialize_read(
                    r,
                    expect_contained_type,
                    depth + 1,
                )?)
                .map_err(|_x| "Value too large")?;

                Ok(value)
            }
            TypePrefix::List => {
                let mut len = [0; 4];
                r.read_exact(&mut len)?;
                let len = u32::from_be_bytes(len);

                if len > MAX_VALUE_SIZE {
                    return Err("Illegal list type".into());
                }

                let (list_type, entry_type) = match expected_type {
                    None => (None, None),
                    Some(TypeSignature::SequenceType(SequenceSubtype::ListType(list_type))) => {
                        if len > list_type.get_max_len() {
                            return Err(SerializationError::DeserializeExpected(
                                expected_type.unwrap().clone(),
                            ));
                        }
                        (Some(list_type), Some(list_type.get_list_item_type()))
                    }
                    Some(x) => return Err(SerializationError::DeserializeExpected(x.clone())),
                };

                let mut items = Vec::with_capacity(len as usize);
                for _i in 0..len {
                    items.push(Value::inner_deserialize_read(r, entry_type, depth + 1)?);
                }

                if let Some(list_type) = list_type {
                    Value::list_with_type(items, list_type.clone())
                        .map_err(|_| "Illegal list type".into())
                } else {
                    Value::list_from(items).map_err(|_| "Illegal list type".into())
                }
            }
            TypePrefix::Tuple => {
                let mut len = [0; 4];
                r.read_exact(&mut len)?;
                let len = u32::from_be_bytes(len);

                if len > MAX_VALUE_SIZE {
                    return Err(SerializationError::DeserializationError(
                        "Illegal tuple type".to_string(),
                    ));
                }

                let tuple_type = match expected_type {
                    None => None,
                    Some(TypeSignature::TupleType(tuple_type)) => {
                        if len as u64 != tuple_type.len() {
                            return Err(SerializationError::DeserializeExpected(
                                expected_type.unwrap().clone(),
                            ));
                        }
                        Some(tuple_type)
                    }
                    Some(x) => return Err(SerializationError::DeserializeExpected(x.clone())),
                };

                let mut items = Vec::with_capacity(len as usize);
                for _i in 0..len {
                    let key = ClarityName::deserialize_read(r)?;

                    let expected_field_type = match tuple_type {
                        None => None,
                        Some(some_tuple) => Some(some_tuple.field_type(&key).ok_or_else(|| {
                            SerializationError::DeserializeExpected(expected_type.unwrap().clone())
                        })?),
                    };

                    let value = Value::inner_deserialize_read(r, expected_field_type, depth + 1)?;
                    items.push((key, value))
                }

                if let Some(tuple_type) = tuple_type {
                    TupleData::from_data_typed(items, tuple_type)
                        .map_err(|_| "Illegal tuple type".into())
                        .map(Value::from)
                } else {
                    TupleData::from_data(items)
                        .map_err(|_| "Illegal tuple type".into())
                        .map(Value::from)
                }
            }
            TypePrefix::StringASCII => {
                let mut buffer_len = [0; 4];
                r.read_exact(&mut buffer_len)?;
                let buffer_len = BufferLength::try_from(u32::from_be_bytes(buffer_len))?;

                if let Some(x) = expected_type {
                    let passed_test = match x {
                        TypeSignature::SequenceType(SequenceSubtype::StringType(
                            StringSubtype::ASCII(expected_len),
                        )) => u32::from(&buffer_len) <= u32::from(expected_len),
                        _ => false,
                    };
                    if !passed_test {
                        return Err(SerializationError::DeserializeExpected(x.clone()));
                    }
                }

                let mut data = vec![0; u32::from(buffer_len) as usize];

                r.read_exact(&mut data[..])?;

                Value::string_ascii_from_bytes(data).map_err(|_| "Bad string".into())
            }
            TypePrefix::StringUTF8 => {
                let mut total_len = [0; 4];
                r.read_exact(&mut total_len)?;
                let total_len = BufferLength::try_from(u32::from_be_bytes(total_len))?;

                let mut data: Vec<u8> = vec![0; u32::from(total_len) as usize];

                r.read_exact(&mut data[..])?;

                let value = Value::string_utf8_from_bytes(data)
                    .map_err(|_| "Illegal string_utf8 type".into());

                if let Some(x) = expected_type {
                    let passed_test = match (x, &value) {
                        (
                            TypeSignature::SequenceType(SequenceSubtype::StringType(
                                StringSubtype::UTF8(expected_len),
                            )),
                            Ok(Value::Sequence(SequenceData::String(CharType::UTF8(utf8)))),
                        ) => utf8.data.len() as u32 <= u32::from(expected_len),
                        _ => false,
                    };
                    if !passed_test {
                        return Err(SerializationError::DeserializeExpected(x.clone()));
                    }
                }

                value
            }
        }
    }

    pub fn serialize_write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
        use super::CharType::*;
        use super::PrincipalData::*;
        use super::SequenceData::{self, *};
        use super::Value::*;

        w.write_all(&[TypePrefix::from(self) as u8])?;
        match self {
            Int(value) => w.write_all(&value.to_be_bytes())?,
            UInt(value) => w.write_all(&value.to_be_bytes())?,
            Principal(Standard(data)) => data.serialize_write(w)?,
            Principal(Contract(contract_identifier)) => {
                contract_identifier.issuer.serialize_write(w)?;
                contract_identifier.name.serialize_write(w)?;
            }
            Response(response) => response.data.serialize_write(w)?,
            // Bool types don't need any more data.
            Bool(_) => {}
            // None types don't need any more data.
            Optional(OptionalData { data: None }) => {}
            Optional(OptionalData { data: Some(value) }) => {
                value.serialize_write(w)?;
            }
            Sequence(List(data)) => {
                w.write_all(&data.len().to_be_bytes())?;
                for item in data.data.iter() {
                    item.serialize_write(w)?;
                }
            }
            Sequence(Buffer(value)) => {
                w.write_all(&(u32::from(value.len()).to_be_bytes()))?;
                w.write_all(&value.data)?
            }
            Sequence(SequenceData::String(UTF8(value))) => {
                let total_len: u32 = value.data.iter().fold(0u32, |len, c| len + c.len() as u32);
                w.write_all(&(total_len.to_be_bytes()))?;
                for bytes in value.data.iter() {
                    w.write_all(&bytes)?
                }
            }
            Sequence(SequenceData::String(ASCII(value))) => {
                w.write_all(&(u32::from(value.len()).to_be_bytes()))?;
                w.write_all(&value.data)?
            }
            Tuple(data) => {
                w.write_all(&u32::try_from(data.data_map.len()).unwrap().to_be_bytes())?;
                for (key, value) in data.data_map.iter() {
                    key.serialize_write(w)?;
                    value.serialize_write(w)?;
                }
            }
        };

        Ok(())
    }

    /// This function attempts to deserialize a byte buffer into a Clarity Value.
    /// The `expected_type` parameter tells the deserializer to expect (and enforce)
    /// a particular type. `ClarityDB` uses this to ensure that lists, tuples, etc. loaded from the database
    /// have their max-length and other type information set by the type declarations in the contract.
    pub fn try_deserialize_bytes(
        bytes: &Vec<u8>,
        expected: &TypeSignature,
    ) -> Result<Value, SerializationError> {
        Value::deserialize_read(&mut bytes.as_slice(), Some(expected))
    }

    /// This function attempts to deserialize a hex string into a Clarity Value.
    /// The `expected_type` parameter tells the deserializer to expect (and enforce)
    /// a particular type. `ClarityDB` uses this to ensure that lists, tuples, etc. loaded from the database
    /// have their max-length and other type information set by the type declarations in the contract.
    pub fn try_deserialize_hex(
        hex: &str,
        expected: &TypeSignature,
    ) -> Result<Value, SerializationError> {
        let mut data = hex_bytes(hex).map_err(|_| "Bad hex string")?;
        Value::try_deserialize_bytes(&mut data, expected)
    }

    /// This function attempts to deserialize a byte buffer into a
    /// Clarity Value, while ensuring that the whole byte buffer is
    /// consumed by the deserialization, erroring if it is not. The
    /// `expected_type` parameter tells the deserializer to expect
    /// (and enforce) a particular type. `ClarityDB` uses this to
    /// ensure that lists, tuples, etc. loaded from the database have
    /// their max-length and other type information set by the type
    /// declarations in the contract.
    pub fn try_deserialize_bytes_exact(
        bytes: &Vec<u8>,
        expected: &TypeSignature,
    ) -> Result<Value, SerializationError> {
        let input_length = bytes.len();
        let (value, read_count) =
            Value::deserialize_read_count(&mut bytes.as_slice(), Some(expected))?;
        if read_count != (input_length as u64) {
            Err(SerializationError::LeftoverBytesInDeserialization)
        } else {
            Ok(value)
        }
    }

    pub fn try_deserialize_bytes_untyped(bytes: &Vec<u8>) -> Result<Value, SerializationError> {
        Value::deserialize_read(&mut bytes.as_slice(), None)
    }

    pub fn try_deserialize_hex_untyped(hex: &str) -> Result<Value, SerializationError> {
        let hex = if hex.starts_with("0x") {
            &hex[2..]
        } else {
            &hex
        };
        let mut data = hex_bytes(hex).map_err(|_| "Bad hex string")?;
        Value::try_deserialize_bytes_untyped(&mut data)
    }

    pub fn deserialize(hex: &str, expected: &TypeSignature) -> Self {
        Value::try_deserialize_hex(hex, expected)
            .expect("ERROR: Failed to parse Clarity hex string")
    }

    pub fn serialized_size(&self) -> u32 {
        let mut counter = WriteCounter { count: 0 };
        self.serialize_write(&mut counter)
            .expect("Error: Failed to count serialization length of Clarity value");
        counter.count
    }
}

/// A writer that just counts the bytes written
struct WriteCounter {
    count: u32,
}

impl Write for WriteCounter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let input: u32 = buf.len().try_into().map_err(|_e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Serialization size would overflow u32",
            )
        })?;
        self.count = self.count.checked_add(input).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Serialization size would overflow u32",
            )
        })?;
        Ok(input as usize)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl ClaritySerializable for Value {
    fn serialize(&self) -> String {
        let mut byte_serialization = Vec::new();
        self.serialize_write(&mut byte_serialization)
            .expect("IOError filling byte buffer.");
        to_hex(byte_serialization.as_slice())
    }
}

impl ClarityDeserializable<Value> for Value {
    fn deserialize(hex: &str) -> Self {
        Value::try_deserialize_hex_untyped(hex).expect("ERROR: Failed to parse Clarity hex string")
    }
}

impl ClaritySerializable for u32 {
    fn serialize(&self) -> String {
        let mut buffer = Vec::new();
        buffer
            .write_all(&self.to_be_bytes())
            .expect("u32 serialization: failed writing.");
        to_hex(buffer.as_slice())
    }
}

impl ClarityDeserializable<u32> for u32 {
    fn deserialize(input: &str) -> Self {
        let bytes = hex_bytes(&input).expect("u32 deserialization: failed decoding bytes.");
        assert_eq!(bytes.len(), 4);
        u32::from_be_bytes(
            bytes[0..4]
                .try_into()
                .expect("u32 deserialization: failed reading."),
        )
    }
}

impl std::hash::Hash for Value {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let mut s = vec![];
        self.consensus_serialize(&mut s)
            .expect("FATAL: failed to serialize to vec");
        s.hash(state);
    }
}
