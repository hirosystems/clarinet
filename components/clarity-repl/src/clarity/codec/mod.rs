#[macro_use]
pub mod macros;
pub mod transaction;

pub use transaction::StacksTransaction;

use std::convert::TryFrom;
use std::io::prelude::*;
use std::io::{Read, Write};
use std::ops::Deref;
use std::ops::DerefMut;
use std::{error, fmt, io, mem};

use crate::clarity::ast::parser::{
    lex, LexItem, CONTRACT_MAX_NAME_LENGTH, CONTRACT_MIN_NAME_LENGTH,
};
use crate::clarity::representations::{
    ClarityName, ContractName, MAX_STRING_LEN as CLARITY_MAX_STRING_LENGTH,
};
use crate::clarity::types::{PrincipalData, Value};

use crate::clarity::util::hash::Hash160;
use crate::clarity::util::retry::BoundReader;
use crate::clarity::util::secp256k1::Secp256k1PublicKey;
use crate::clarity::util::StacksAddress;

pub const HASH160_ENCODED_SIZE: u32 = 20;
pub const BURNCHAIN_HEADER_HASH_ENCODED_SIZE: u32 = 32;
pub const MESSAGE_SIGNATURE_ENCODED_SIZE: u32 = 65;
/// P2P preamble length (addands correspond to fields above)
pub const PREAMBLE_ENCODED_SIZE: u32 = 4
    + 4
    + 4
    + 8
    + BURNCHAIN_HEADER_HASH_ENCODED_SIZE
    + 8
    + BURNCHAIN_HEADER_HASH_ENCODED_SIZE
    + 4
    + MESSAGE_SIGNATURE_ENCODED_SIZE
    + 4;
pub const PEER_ADDRESS_ENCODED_SIZE: u32 = 16;
pub const NEIGHBOR_ADDRESS_ENCODED_SIZE: u32 = PEER_ADDRESS_ENCODED_SIZE + 2 + HASH160_ENCODED_SIZE;
pub const RELAY_DATA_ENCODED_SIZE: u32 = NEIGHBOR_ADDRESS_ENCODED_SIZE + 4;
// maximum number of relayers that can be included in a message
pub const MAX_RELAYERS_LEN: u32 = 16;
// number of peers to relay to, depending on outbound or inbound
pub const MAX_BROADCAST_OUTBOUND_RECEIVERS: usize = 8;
pub const MAX_BROADCAST_INBOUND_RECEIVERS: usize = 16;
// messages can't be bigger than 16MB plus the preamble and relayers
pub const MAX_PAYLOAD_LEN: u32 = 1 + 16 * 1024 * 1024;
pub const MAX_MESSAGE_LEN: u32 =
    MAX_PAYLOAD_LEN + (PREAMBLE_ENCODED_SIZE + MAX_RELAYERS_LEN * RELAY_DATA_ENCODED_SIZE);

// pub const CONTRACT_MIN_NAME_LENGTH: usize = 1;
// pub const CONTRACT_MAX_NAME_LENGTH: usize = 40;

pub const MAX_BLOCK_LEN: u32 = 2 * 1024 * 1024;
pub const MAX_TRANSACTION_LEN: u32 = MAX_BLOCK_LEN;

#[derive(Debug)]
pub enum Error {
    /// Failed to encode
    SerializeError(String),
    /// Failed to read
    ReadError(io::Error),
    /// Failed to decode
    DeserializeError(String),
    /// Failed to write
    WriteError(io::Error),
    /// Underflow -- not enough bytes to form the message
    UnderflowError(String),
    /// Overflow -- message too big
    OverflowError(String),
    /// Array is too big
    ArrayTooLong,
    /// Failed to sign
    SigningError(String),
    /// Generic error
    GenericError(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::SerializeError(ref s) => fmt::Display::fmt(s, f),
            Error::DeserializeError(ref s) => fmt::Display::fmt(s, f),
            Error::ReadError(ref io) => fmt::Display::fmt(io, f),
            Error::WriteError(ref io) => fmt::Display::fmt(io, f),
            Error::UnderflowError(ref s) => fmt::Display::fmt(s, f),
            Error::OverflowError(ref s) => fmt::Display::fmt(s, f),
            Error::SigningError(ref s) => fmt::Display::fmt(s, f),
            Error::GenericError(ref s) => fmt::Display::fmt(s, f),
            Error::ArrayTooLong => write!(f, "Array too long"),
        }
    }
}

impl error::Error for Error {
    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            Error::SerializeError(ref _s) => None,
            Error::ReadError(ref io) => Some(io),
            Error::DeserializeError(ref _s) => None,
            Error::WriteError(ref io) => Some(io),
            Error::UnderflowError(ref _s) => None,
            Error::OverflowError(ref _s) => None,
            Error::SigningError(ref _s) => None,
            Error::GenericError(ref _s) => None,
            Error::ArrayTooLong => None,
        }
    }
}

/// Helper trait for various primitive types that make up Stacks messages
pub trait StacksMessageCodec {
    /// serialize implementors _should never_ error unless there is an underlying
    ///   failure in writing to the `fd`
    fn consensus_serialize<W: Write>(&self, fd: &mut W) -> Result<(), Error>
    where
        Self: Sized;
    fn consensus_deserialize<R: Read>(fd: &mut R) -> Result<Self, Error>
    where
        Self: Sized;
    /// Convenience for serialization to a vec.
    ///  this function unwraps any underlying serialization error
    fn serialize_to_vec(&self) -> Vec<u8>
    where
        Self: Sized,
    {
        let mut bytes = vec![];
        self.consensus_serialize(&mut bytes)
            .expect("BUG: serialization to buffer failed.");
        bytes
    }
}

pub fn write_next<T: StacksMessageCodec, W: Write>(fd: &mut W, item: &T) -> Result<(), Error> {
    item.consensus_serialize(fd)
}

pub fn read_next<T: StacksMessageCodec, R: Read>(fd: &mut R) -> Result<T, Error> {
    let item: T = T::consensus_deserialize(fd)?;
    Ok(item)
}

pub fn read_next_vec<T: StacksMessageCodec + Sized, R: Read>(
    fd: &mut R,
    num_items: u32,
    max_items: u32,
) -> Result<Vec<T>, Error> {
    let len = u32::consensus_deserialize(fd)?;

    if max_items > 0 {
        if len > max_items {
            // too many items
            return Err(Error::DeserializeError(format!(
                "Array has too many items ({} > {}",
                len, max_items
            )));
        }
    } else {
        if len != num_items {
            // inexact item count
            return Err(Error::DeserializeError(format!(
                "Array has incorrect number of items ({} != {})",
                len, num_items
            )));
        }
    }

    if (mem::size_of::<T>() as u128) * (len as u128) > MAX_MESSAGE_LEN as u128 {
        return Err(Error::DeserializeError(format!(
            "Message occupies too many bytes (tried to allocate {}*{}={})",
            mem::size_of::<T>() as u128,
            len,
            (mem::size_of::<T>() as u128) * (len as u128)
        )));
    }

    let mut ret = Vec::with_capacity(len as usize);
    for _i in 0..len {
        let next_item = T::consensus_deserialize(fd)?;
        ret.push(next_item);
    }

    Ok(ret)
}

macro_rules! impl_stacks_message_codec_for_int {
    ($typ:ty; $array:expr) => {
        impl StacksMessageCodec for $typ {
            fn consensus_serialize<W: Write>(&self, fd: &mut W) -> Result<(), Error> {
                fd.write_all(&self.to_be_bytes()).map_err(Error::WriteError)
            }
            fn consensus_deserialize<R: Read>(fd: &mut R) -> Result<Self, Error> {
                let mut buf = $array;
                fd.read_exact(&mut buf).map_err(Error::ReadError)?;
                Ok(<$typ>::from_be_bytes(buf))
            }
        }
    };
}

impl_stacks_message_codec_for_int!(u8; [0; 1]);
impl_stacks_message_codec_for_int!(u16; [0; 2]);
impl_stacks_message_codec_for_int!(u32; [0; 4]);
impl_stacks_message_codec_for_int!(u64; [0; 8]);
impl_stacks_message_codec_for_int!(i64; [0; 8]);

pub fn read_next_at_most<R: Read, T: StacksMessageCodec + Sized>(
    fd: &mut R,
    max_items: u32,
) -> Result<Vec<T>, Error> {
    read_next_vec::<T, R>(fd, 0, max_items)
}

pub fn read_next_exact<R: Read, T: StacksMessageCodec + Sized>(
    fd: &mut R,
    num_items: u32,
) -> Result<Vec<T>, Error> {
    read_next_vec::<T, R>(fd, num_items, 0)
}

impl<T> StacksMessageCodec for Vec<T>
where
    T: StacksMessageCodec + Sized,
{
    fn consensus_serialize<W: Write>(&self, fd: &mut W) -> Result<(), Error> {
        let len = self.len() as u32;
        write_next(fd, &len)?;
        for i in 0..self.len() {
            write_next(fd, &self[i])?;
        }
        Ok(())
    }

    fn consensus_deserialize<R: Read>(fd: &mut R) -> Result<Vec<T>, Error> {
        read_next_at_most::<R, T>(fd, u32::max_value())
    }
}

/// printable-ASCII-only string, but encodable.
/// Note that it cannot be longer than ARRAY_MAX_LEN (4.1 billion bytes)
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct StacksString(Vec<u8>);

impl fmt::Display for StacksString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(String::from_utf8_lossy(&self).into_owned().as_str())
    }
}

impl fmt::Debug for StacksString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(String::from_utf8_lossy(&self).into_owned().as_str())
    }
}

impl Deref for StacksString {
    type Target = Vec<u8>;
    fn deref(&self) -> &Vec<u8> {
        &self.0
    }
}

impl DerefMut for StacksString {
    fn deref_mut(&mut self) -> &mut Vec<u8> {
        &mut self.0
    }
}

impl From<ClarityName> for StacksString {
    fn from(clarity_name: ClarityName) -> StacksString {
        // .unwrap() is safe since StacksString is less strict
        StacksString::from_str(&clarity_name).unwrap()
    }
}

impl From<ContractName> for StacksString {
    fn from(contract_name: ContractName) -> StacksString {
        // .unwrap() is safe since StacksString is less strict
        StacksString::from_str(&contract_name).unwrap()
    }
}

impl StacksString {
    /// Is the given string a valid Clarity string?
    pub fn is_valid_string(s: &String) -> bool {
        s.is_ascii() && StacksString::is_printable(s)
    }

    pub fn is_printable(s: &String) -> bool {
        if !s.is_ascii() {
            return false;
        }
        // all characters must be ASCII "printable" characters, excluding "delete".
        // This is 0x20 through 0x7e, inclusive, as well as '\t' and '\n'
        // TODO: DRY up with vm::representations
        for c in s.as_bytes().iter() {
            if (*c < 0x20 && *c != ('\t' as u8) && *c != ('\n' as u8)) || (*c > 0x7e) {
                return false;
            }
        }
        true
    }

    pub fn is_clarity_variable(&self) -> bool {
        // must parse to a single Clarity variable
        match lex(&self.to_string()) {
            Ok(lexed) => {
                if lexed.len() != 1 {
                    return false;
                }
                match lexed[0].0 {
                    LexItem::Variable(_) => true,
                    _ => false,
                }
            }
            Err(_) => false,
        }
    }

    pub fn from_string(s: &String) -> Option<StacksString> {
        if !StacksString::is_valid_string(s) {
            return None;
        }
        Some(StacksString(s.as_bytes().to_vec()))
    }

    pub fn from_str(s: &str) -> Option<StacksString> {
        if !StacksString::is_valid_string(&String::from(s)) {
            return None;
        }
        Some(StacksString(s.as_bytes().to_vec()))
    }

    pub fn to_string(&self) -> String {
        // guaranteed to always succeed because the string is ASCII
        String::from_utf8(self.0.clone()).unwrap()
    }
}

impl StacksMessageCodec for StacksString {
    fn consensus_serialize<W: Write>(&self, fd: &mut W) -> Result<(), Error> {
        write_next(fd, &self.0)
    }

    fn consensus_deserialize<R: Read>(fd: &mut R) -> Result<StacksString, Error> {
        let bytes: Vec<u8> = {
            let mut bound_read = BoundReader::from_reader(fd, MAX_MESSAGE_LEN as u64);
            read_next(&mut bound_read)
        }?;

        // must encode a valid string
        let s = String::from_utf8(bytes.clone()).map_err(|_e| {
            Error::DeserializeError("Invalid Stacks string: could not build from utf8".to_string())
        })?;

        if !StacksString::is_valid_string(&s) {
            // non-printable ASCII or not ASCII
            return Err(Error::DeserializeError(
                "Invalid Stacks string: non-printable or non-ASCII string".to_string(),
            ));
        }

        Ok(StacksString(bytes))
    }
}

impl StacksMessageCodec for ClarityName {
    fn consensus_serialize<W: Write>(&self, fd: &mut W) -> Result<(), Error> {
        // ClarityName can't be longer than vm::representations::MAX_STRING_LEN, which itself is
        // a u8, so we should be good here.
        if self.as_bytes().len() > CLARITY_MAX_STRING_LENGTH as usize {
            return Err(Error::SerializeError(
                "Failed to serialize clarity name: too long".to_string(),
            ));
        }
        write_next(fd, &(self.as_bytes().len() as u8))?;
        fd.write_all(self.as_bytes()).map_err(Error::WriteError)?;
        Ok(())
    }

    fn consensus_deserialize<R: Read>(fd: &mut R) -> Result<ClarityName, Error> {
        let len_byte: u8 = read_next(fd)?;
        if len_byte > CLARITY_MAX_STRING_LENGTH {
            return Err(Error::DeserializeError(
                "Failed to deserialize clarity name: too long".to_string(),
            ));
        }
        let mut bytes = vec![0u8; len_byte as usize];
        fd.read_exact(&mut bytes).map_err(Error::ReadError)?;

        // must encode a valid string
        let s = String::from_utf8(bytes).map_err(|_e| {
            Error::DeserializeError(
                "Failed to parse Clarity name: could not contruct from utf8".to_string(),
            )
        })?;

        // must decode to a clarity name
        let name = ClarityName::try_from(s).map_err(|e| {
            Error::DeserializeError(format!("Failed to parse Clarity name: {:?}", e))
        })?;
        Ok(name)
    }
}

impl StacksMessageCodec for ContractName {
    fn consensus_serialize<W: Write>(&self, fd: &mut W) -> Result<(), Error> {
        if self.as_bytes().len() < CONTRACT_MIN_NAME_LENGTH as usize
            || self.as_bytes().len() > CONTRACT_MAX_NAME_LENGTH as usize
        {
            return Err(Error::SerializeError(format!(
                "Failed to serialize contract name: too short or too long: {}",
                self.as_bytes().len()
            )));
        }
        write_next(fd, &(self.as_bytes().len() as u8))?;
        fd.write_all(self.as_bytes()).map_err(Error::WriteError)?;
        Ok(())
    }

    fn consensus_deserialize<R: Read>(fd: &mut R) -> Result<ContractName, Error> {
        let len_byte: u8 = read_next(fd)?;
        if (len_byte as usize) < CONTRACT_MIN_NAME_LENGTH
            || (len_byte as usize) > CONTRACT_MAX_NAME_LENGTH
        {
            return Err(Error::DeserializeError(format!(
                "Failed to deserialize contract name: too short or too long: {}",
                len_byte
            )));
        }
        let mut bytes = vec![0u8; len_byte as usize];
        fd.read_exact(&mut bytes).map_err(Error::ReadError)?;

        // must encode a valid string
        let s = String::from_utf8(bytes).map_err(|_e| {
            Error::DeserializeError(
                "Failed to parse Contract name: could not construct from utf8".to_string(),
            )
        })?;

        let name = ContractName::try_from(s).map_err(|e| {
            Error::DeserializeError(format!("Failed to parse Contract name: {:?}", e))
        })?;
        Ok(name)
    }
}

impl StacksMessageCodec for StacksAddress {
    fn consensus_serialize<W: Write>(&self, fd: &mut W) -> Result<(), Error> {
        write_next(fd, &self.version)?;
        fd.write_all(self.bytes.as_bytes())
            .map_err(Error::WriteError)
    }

    fn consensus_deserialize<R: Read>(fd: &mut R) -> Result<StacksAddress, Error> {
        let version: u8 = read_next(fd)?;
        let hash160: Hash160 = read_next(fd)?;
        Ok(StacksAddress {
            version: version,
            bytes: hash160,
        })
    }
}
