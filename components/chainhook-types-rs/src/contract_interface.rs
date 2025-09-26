// NOTE: This module is a very slightly simplified version of the
// `clarity-vm` repository's [ContractInterface](https://github.com/stacks-network/stacks-blockchain/blob/eca1cfe81f0c0989ebd3e53c32e3e5d70ed83757/clarity/src/vm/analysis/contract_interface_builder/mod.rs#L368) type.
// We've copied it here rather than using `clarity-vm` as a dependency to avoid circular dependencies.

use std::{fmt, str::FromStr};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractInterface {
    pub functions: Vec<ContractInterfaceFunction>,
    pub variables: Vec<ContractInterfaceVariable>,
    pub maps: Vec<ContractInterfaceMap>,
    pub fungible_tokens: Vec<ContractInterfaceFungibleTokens>,
    pub non_fungible_tokens: Vec<ContractInterfaceNonFungibleTokens>,
    pub epoch: String,
    pub clarity_version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractInterfaceFunction {
    pub name: String,
    pub access: ContractInterfaceFunctionAccess,
    pub args: Vec<ContractInterfaceFunctionArg>,
    pub outputs: ContractInterfaceFunctionOutput,
}
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContractInterfaceFunctionAccess {
    private,
    public,
    read_only,
}
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContractInterfaceVariableAccess {
    constant,
    variable,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractInterfaceFunctionArg {
    pub name: String,
    #[serde(rename = "type")]
    pub type_f: ContractInterfaceAtomType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractInterfaceFunctionOutput {
    #[serde(rename = "type")]
    pub type_f: ContractInterfaceAtomType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractInterfaceVariable {
    pub name: String,
    #[serde(rename = "type")]
    pub type_f: ContractInterfaceAtomType,
    pub access: ContractInterfaceVariableAccess,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractInterfaceMap {
    pub name: String,
    pub key: ContractInterfaceAtomType,
    pub value: ContractInterfaceAtomType,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContractInterfaceAtomType {
    none,
    int128,
    uint128,
    bool,
    principal,
    buffer {
        length: u32,
    },
    #[serde(rename = "string-utf8")]
    string_utf8 {
        length: u32,
    },
    #[serde(rename = "string-ascii")]
    string_ascii {
        length: u32,
    },
    tuple(Vec<ContractInterfaceTupleEntryType>),
    optional(Box<ContractInterfaceAtomType>),
    response {
        ok: Box<ContractInterfaceAtomType>,
        error: Box<ContractInterfaceAtomType>,
    },
    list {
        #[serde(rename = "type")]
        type_f: Box<ContractInterfaceAtomType>,
        length: u32,
    },
    trait_reference,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractInterfaceTupleEntryType {
    pub name: String,
    #[serde(rename = "type")]
    pub type_f: ContractInterfaceAtomType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractInterfaceFungibleTokens {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractInterfaceNonFungibleTokens {
    pub name: String,
    #[serde(rename = "type")]
    pub type_f: ContractInterfaceAtomType,
}
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum ClarityVersion {
    Clarity1,
    Clarity2,
}

impl fmt::Display for ClarityVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClarityVersion::Clarity1 => write!(f, "Clarity 1"),
            ClarityVersion::Clarity2 => write!(f, "Clarity 2"),
        }
    }
}

impl FromStr for ClarityVersion {
    type Err = String;
    fn from_str(version: &str) -> Result<ClarityVersion, String> {
        let s = version.to_string().to_lowercase();
        if s == "clarity1" {
            Ok(ClarityVersion::Clarity1)
        } else if s == "clarity2" {
            Ok(ClarityVersion::Clarity2)
        } else {
            Err(format!(
                "Invalid clarity version. Valid versions are: Clarity1, Clarity2."
            ))
        }
    }
}
#[repr(u32)]
#[derive(Debug, Clone, Eq, PartialEq, PartialOrd, Ord, Hash, Copy, Serialize, Deserialize)]
pub enum StacksEpochId {
    Epoch10 = 0x01000,
    Epoch20 = 0x02000,
    Epoch2_05 = 0x02005,
    Epoch21 = 0x0200a,
    Epoch22 = 0x0200f,
    Epoch23 = 0x02014,
    Epoch24 = 0x02019,
}

impl std::fmt::Display for StacksEpochId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StacksEpochId::Epoch10 => write!(f, "1.0"),
            StacksEpochId::Epoch20 => write!(f, "2.0"),
            StacksEpochId::Epoch2_05 => write!(f, "2.05"),
            StacksEpochId::Epoch21 => write!(f, "2.1"),
            StacksEpochId::Epoch22 => write!(f, "2.2"),
            StacksEpochId::Epoch23 => write!(f, "2.3"),
            StacksEpochId::Epoch24 => write!(f, "2.4"),
        }
    }
}
