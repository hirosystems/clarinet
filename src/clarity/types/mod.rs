pub mod signatures;

use std::{fmt, cmp};
use std::convert::{TryInto, TryFrom};
use std::collections::BTreeMap;
use serde::{Serialize, Deserialize};

use super::util::c32;
use super::representations::{ClarityName, ContractName, SymbolicExpression, SymbolicExpressionType};
use super::errors::{RuntimeErrorType, CheckErrors, InterpreterResult as Result};
use super::util::hash;
use super::functions::{BlockInfoProperty, DefineFunctions};

pub use super::types::signatures::{
    TupleTypeSignature, AssetIdentifier, FixedFunction,
    TypeSignature, FunctionType, ListTypeData, FunctionArg, parse_name_type_pairs,
    BUFF_64, BUFF_32, BUFF_20, BufferLength
};

pub const MAX_VALUE_SIZE: u32 = 1024 * 1024; // 1MB
pub const BOUND_VALUE_SERIALIZATION_BYTES: u32 = MAX_VALUE_SIZE * 2;
pub const BOUND_VALUE_SERIALIZATION_HEX: u32 = BOUND_VALUE_SERIALIZATION_BYTES * 2;

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub enum DefineType {
    ReadOnly,
    Public,
    Private
}

#[derive(Clone,Serialize, Deserialize)]
pub struct DefinedFunction {
    identifier: FunctionIdentifier,
    name: ClarityName,
    arg_types: Vec<TypeSignature>,
    pub define_type: DefineType,
    arguments: Vec<ClarityName>,
    body: SymbolicExpression
}


impl DefineFunctions {
    pub fn try_parse(expression: &SymbolicExpression) -> Option<(DefineFunctions, &[SymbolicExpression])> {
        let expression = expression.match_list()?;
        let (function_name, args) = expression.split_first()?;
        let function_name = function_name.match_atom()?;
        let define_type = DefineFunctions::lookup_by_name(function_name)?;
        Some((define_type, args))
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct FunctionIdentifier {
    identifier: String
}
pub const MAX_TYPE_DEPTH: u8 = 32;
// this is the charged size for wrapped values, i.e., response or optionals
pub const WRAPPER_VALUE_SIZE: u32 = 1;

#[derive(Debug, Clone, Eq, Serialize, Deserialize)]
pub struct TupleData {
    // todo: remove type_signature
    pub type_signature: TupleTypeSignature,
    pub data_map: BTreeMap<ClarityName, Value>
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuffData {
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Eq, Serialize, Deserialize)]
pub struct ListData {
    pub data: Vec<Value>,
    // todo: remove type_signature
    pub type_signature: ListTypeData
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct StandardPrincipalData(pub u8, pub [u8; 20]);

impl StandardPrincipalData {

    pub fn transient() -> StandardPrincipalData {
        Self(1, [1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1])
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct QualifiedContractIdentifier {
    pub issuer: StandardPrincipalData,
    pub name: ContractName
}

impl QualifiedContractIdentifier {

    pub fn new(issuer: StandardPrincipalData, name: ContractName) -> QualifiedContractIdentifier {
        Self { issuer, name }
    }

    pub fn local(name: &str) -> Result<QualifiedContractIdentifier> {
        let name = name.to_string().try_into()?;
        Ok(Self::new(StandardPrincipalData::transient(), name))
    }

    pub fn transient() -> QualifiedContractIdentifier {
        let name = String::from("__transient").try_into().unwrap();
        Self { 
            issuer: StandardPrincipalData::transient(), 
            name
        }
    }

    pub fn parse(literal: &str) -> Result<QualifiedContractIdentifier> {
        let split: Vec<_> = literal.splitn(2, ".").collect();
        if split.len() != 2 {
            return Err(RuntimeErrorType::ParseError(
                "Invalid principal literal: expected a `.` in a qualified contract name".to_string()).into());
        }
        let sender = PrincipalData::parse_standard_principal(split[0])?;
        let name = split[1].to_string().try_into()?;
        Ok(QualifiedContractIdentifier::new(sender, name))
    }

    pub fn to_string(&self) -> String {
        format!("{}.{}", self.issuer, self.name.to_string())
    }
}

impl fmt::Display for QualifiedContractIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub enum PrincipalData {
    Standard(StandardPrincipalData),
    Contract(QualifiedContractIdentifier),
}

pub enum ContractIdentifier {
    Relative(ContractName),
    Qualified(QualifiedContractIdentifier)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OptionalData {
    pub data: Option<Box<Value>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResponseData {
    pub committed: bool,
    pub data: Box<Value>,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub struct TraitIdentifier {
    pub name: ClarityName,
    pub contract_identifier: QualifiedContractIdentifier,
}

impl TraitIdentifier {

    pub fn new(issuer: StandardPrincipalData, contract_name: ContractName, name: ClarityName) -> TraitIdentifier {
        Self { 
            name, 
            contract_identifier: QualifiedContractIdentifier {
                issuer,
                name: contract_name
            }
        }
    }

    pub fn parse_fully_qualified(literal: &str) -> Result<TraitIdentifier> {
        let (issuer, contract_name, name) = Self::parse(literal)?;
        let issuer = issuer.ok_or(RuntimeErrorType::BadTypeConstruction)?;
        Ok(TraitIdentifier::new(issuer, contract_name, name))
    }

    pub fn parse_sugared_syntax(literal: &str) -> Result<(ContractName, ClarityName)> {
        let (_ , contract_name, name) = Self::parse(literal)?;
        Ok((contract_name, name))
    }

    pub fn parse(literal: &str) -> Result<(Option<StandardPrincipalData>, ContractName, ClarityName)> {
        let split: Vec<_> = literal.splitn(3, ".").collect();
        if split.len() != 3 {
            return Err(RuntimeErrorType::ParseError(
                "Invalid principal literal: expected a `.` in a qualified contract name".to_string()).into());
        }

        let issuer = match split[0].len() {
            0 => None,
            _ => Some(PrincipalData::parse_standard_principal(split[0])?),
        };
        let contract_name = split[1].to_string().try_into()?;
        let name = split[2].to_string().try_into()?;

        Ok((issuer, contract_name, name))
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Int(i128),
    UInt(u128),
    Bool(bool),
    Buffer(BuffData),
    List(ListData),
    Principal(PrincipalData),
    Tuple(TupleData),
    Optional(OptionalData),
    Response(ResponseData),
}

impl OptionalData {
    pub fn type_signature(&self) -> TypeSignature {
        let type_result = match self.data {
            Some(ref v) => TypeSignature::new_option(TypeSignature::type_of(&v)),
            None => TypeSignature::new_option(TypeSignature::NoType)
        };
        type_result.expect("Should not have constructed too large of a type.")
    }
}

impl ResponseData {
    pub fn type_signature(&self) -> TypeSignature {
        let type_result = match self.committed {
            true => TypeSignature::new_response(
                TypeSignature::type_of(&self.data), TypeSignature::NoType),
            false => TypeSignature::new_response(
                TypeSignature::NoType, TypeSignature::type_of(&self.data))
        };
        type_result.expect("Should not have constructed too large of a type.")        
    }
}

impl BlockInfoProperty {
    pub fn type_result(&self) -> TypeSignature {
        use self::BlockInfoProperty::*;
        match self {
            Time => TypeSignature::UIntType,
            IdentityHeaderHash | VrfSeed | HeaderHash | BurnchainHeaderHash => BUFF_32.clone(),
            MinerAddress => TypeSignature::PrincipalType,
        }
    }
}

impl PartialEq for ListData {
    fn eq(&self, other: &ListData) -> bool {
        self.data == other.data
    }
}

impl PartialEq for TupleData {
    fn eq(&self, other: &TupleData) -> bool {
        self.data_map == other.data_map
    }
}

pub const NONE: Value = Value::Optional(OptionalData { data: None });

impl Value {
    pub fn some(data: Value) -> Result<Value> {
        if data.size() + WRAPPER_VALUE_SIZE > MAX_VALUE_SIZE {
            Err(CheckErrors::ValueTooLarge.into())
        } else if data.depth() + 1 > MAX_TYPE_DEPTH {
            Err(CheckErrors::TypeSignatureTooDeep.into())
        } else {
            Ok(Value::Optional(OptionalData {
                data: Some(Box::new(data)) }))
        }
    }

    pub fn none() -> Value {
        NONE.clone()
    }

    pub fn okay_true() -> Value {
        Value::Response(ResponseData { committed: true, data: Box::new(Value::Bool(true)) })
    }

    pub fn err_uint(ecode: u128) -> Value {
        Value::Response(ResponseData { committed: false, data: Box::new(Value::UInt(ecode)) })
    }

    pub fn err_none() -> Value {
        Value::Response(ResponseData { committed: false, data: Box::new(NONE.clone()) })
    }

    pub fn okay(data: Value) -> Result<Value> {
        if data.size() + WRAPPER_VALUE_SIZE > MAX_VALUE_SIZE {
            Err(CheckErrors::ValueTooLarge.into())
        } else if data.depth() + 1 > MAX_TYPE_DEPTH {
            Err(CheckErrors::TypeSignatureTooDeep.into())
        } else {
            Ok(Value::Response(ResponseData { 
                committed: true,
                data: Box::new(data) }))
        }
    }

    pub fn error(data: Value) -> Result<Value> {
        if data.size() + WRAPPER_VALUE_SIZE > MAX_VALUE_SIZE {
            Err(CheckErrors::ValueTooLarge.into())
        } else if data.depth() + 1 > MAX_TYPE_DEPTH {
            Err(CheckErrors::TypeSignatureTooDeep.into())
        } else {
            Ok(Value::Response(ResponseData { 
                committed: false,
                data: Box::new(data) }))
        }
    }

    pub fn size(&self) -> u32 {
        TypeSignature::type_of(self).size()
    }

    pub fn depth(&self) -> u8 {
        TypeSignature::type_of(self).depth()
    }

    pub fn list_from(list_data: Vec<Value>) -> Result<Value> {
        // Constructors for TypeSignature ensure that the size of the Value cannot
        //   be greater than MAX_VALUE_SIZE (they error on such constructions)
        // Aaron: at this point, we've _already_ allocated memory for this type.
        //     (e.g., from a (map...) call, or a (list...) call.
        //     this is a problem _if_ the static analyzer cannot already prevent
        //     this case. This applies to all the constructor size checks.
        let type_sig = TypeSignature::construct_parent_list_type(&list_data)?;
        Ok(Value::List(ListData { data: list_data, type_signature: type_sig }))
    }

    pub fn buff_from(buff_data: Vec<u8>) -> Result<Value> {
        // check the buffer size
        BufferLength::try_from(buff_data.len())?;
        // construct the buffer
        Ok(Value::Buffer(BuffData { data: buff_data }))
    }

    pub fn buff_from_byte(byte: u8) -> Value {
        Value::Buffer(BuffData { data: vec![byte] })
    }
}

impl BuffData {
    pub fn len(&self) -> BufferLength {
        self.data.len().try_into().unwrap()
    }
}

impl ListData {
    pub fn len(&self) -> u32 {
        self.data.len().try_into().unwrap()
    }
}

impl fmt::Display for OptionalData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.data {
            Some(ref x) => write!(f, "(some {})", x),
            None => write!(f, "none")
        }
    }
}

impl fmt::Display for ResponseData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.committed {
            true => write!(f, "(ok {})", self.data),
            false => write!(f, "(err {})", self.data)
        }
    }
}

impl fmt::Display for BuffData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", hash::to_hex(&self.data))
    }
}

impl fmt::Debug for BuffData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Int(int) => write!(f, "{}", int),
            Value::UInt(int) => write!(f, "u{}", int),
            Value::Bool(boolean) => write!(f, "{}", boolean),
            Value::Buffer(vec_bytes) => write!(f, "0x{}", &vec_bytes),
            Value::Tuple(data) => write!(f, "{}", data),
            Value::Principal(principal_data) => write!(f, "{}", principal_data),
            Value::Optional(opt_data) => write!(f, "{}", opt_data),
            Value::Response(res_data) => write!(f, "{}", res_data),
            Value::List(list_data) => {
                write!(f, "(")?;
                for (ix, v) in list_data.data.iter().enumerate() {
                    if ix > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, ")")
            }
        }
    }
}

impl PrincipalData {
    pub fn version(&self) -> u8 {
        match self {
            PrincipalData::Standard(StandardPrincipalData(version, _)) => *version,
            PrincipalData::Contract(QualifiedContractIdentifier { issuer, name: _ }) => {
                issuer.0
            }
        }
    }

    pub fn parse(literal: &str) -> Result<PrincipalData> {
        // be permissive about leading single-quote
        let literal = if literal.starts_with("'") {
            &literal[1..]
        } else {
            literal
        };

        if literal.contains(".") {
            PrincipalData::parse_qualified_contract_principal(literal)
        } else {
            PrincipalData::parse_standard_principal(literal)
                .map(PrincipalData::from)
        }
    }

    pub fn parse_qualified_contract_principal(literal: &str) -> Result<PrincipalData> {
        let contract_id = QualifiedContractIdentifier::parse(literal)?;
        Ok(PrincipalData::Contract(contract_id))
    }

    pub fn parse_standard_principal(literal: &str) -> Result<StandardPrincipalData> {
        let (version, data) = c32::c32_address_decode(&literal)
            .map_err(|x| { RuntimeErrorType::ParseError(format!("Invalid principal literal: {:?}", x)) })?;
        if data.len() != 20 {
            return Err(RuntimeErrorType::ParseError(
                "Invalid principal literal: Expected 20 data bytes.".to_string()).into());
        }
        let mut fixed_data = [0; 20];
        fixed_data.copy_from_slice(&data[..20]);
        Ok(StandardPrincipalData(version, fixed_data))
    }
}

impl StandardPrincipalData {
    pub fn to_address(&self) -> String {
        c32::c32_address(self.0, &self.1[..])
            .unwrap_or_else(|_| "INVALID_C32_ADD".to_string())
    }
}

impl fmt::Display for StandardPrincipalData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let c32_str = self.to_address();
        write!(f, "{}", c32_str)
    }
}

impl fmt::Display for PrincipalData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PrincipalData::Standard(sender) => {
                write!(f, "{}", sender)                
            },
            PrincipalData::Contract(contract_identifier) => {
                write!(f, "{}.{}", contract_identifier.issuer, contract_identifier.name.to_string())
            }
        }
    }
}

impl fmt::Display for TraitIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}", self.contract_identifier, self.name.to_string())
    }
}

impl From<StandardPrincipalData> for Value {
    fn from(principal: StandardPrincipalData) -> Self {
        Value::Principal(PrincipalData::from(principal))
    }
}

impl From<QualifiedContractIdentifier> for Value {
    fn from(principal: QualifiedContractIdentifier) -> Self {
        Value::Principal(PrincipalData::Contract(principal))
    }
}

impl From<PrincipalData> for Value {
    fn from(p: PrincipalData) -> Self {
        Value::Principal(p)
    }
}

impl From<StandardPrincipalData> for PrincipalData {
    fn from(p: StandardPrincipalData) -> Self {
        PrincipalData::Standard(p)
    }
}

impl From<QualifiedContractIdentifier> for PrincipalData {
    fn from(principal: QualifiedContractIdentifier) -> Self {
        PrincipalData::Contract(principal)
    }
}

impl From<TupleData> for Value {
    fn from(t: TupleData) -> Self {
        Value::Tuple(t)
    }
}

impl TupleData {
    fn new(type_signature: TupleTypeSignature, data_map: BTreeMap<ClarityName, Value>) -> Result<TupleData> {
        let t = TupleData { type_signature, data_map };
        Ok(t)
    }

    pub fn len(&self) -> u64 {
        self.data_map.len() as u64
    }

    pub fn from_data(mut data: Vec<(ClarityName, Value)>) -> Result<TupleData> {
        let mut type_map = BTreeMap::new();
        let mut data_map = BTreeMap::new();
        for (name, value) in data.drain(..) {
            let type_info = TypeSignature::type_of(&value);
            if type_map.contains_key(&name) {
                return Err(CheckErrors::NameAlreadyUsed(name.into()).into());
            } else {
                type_map.insert(name.clone(), type_info);
            }
            data_map.insert(name, value);
        }

        Self::new(TupleTypeSignature::try_from(type_map)?, data_map)
    }

    pub fn get(&self, name: &str) -> Result<&Value> {
        self.data_map.get(name)
            .ok_or_else(|| CheckErrors::NoSuchTupleField(name.to_string(), self.type_signature.clone()).into())
    }

    pub fn get_owned(mut self, name: &str) -> Result<Value> {
        self.data_map.remove(name)
            .ok_or_else(|| CheckErrors::NoSuchTupleField(name.to_string(), self.type_signature.clone()).into())
    }
}

impl fmt::Display for TupleData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(tuple")?;
        for (name, value) in self.data_map.iter() {
            write!(f, " ")?;
            write!(f, "({} {})", &**name, value)?;
        }
        write!(f, ")")
    }
}

pub enum TupleDefinitionType {
    Implicit(Box<[SymbolicExpression]>),
    Explicit,
}

pub fn get_definition_type_of_tuple_argument(args: &SymbolicExpression) -> TupleDefinitionType {
    if let SymbolicExpressionType::List(ref outer_expr) = args.expr {
        if let SymbolicExpressionType::List(_) = (&outer_expr[0]).expr {
            return TupleDefinitionType::Implicit(outer_expr.clone());
        }
    }
    TupleDefinitionType::Explicit
}

