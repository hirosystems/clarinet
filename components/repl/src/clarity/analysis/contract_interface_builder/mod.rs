use crate::clarity::analysis::types::ContractAnalysis;
use crate::clarity::types::{
    FixedFunction, FunctionArg, FunctionType, TupleTypeSignature, TypeSignature,
};
use crate::clarity::ClarityName;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub fn build_contract_interface(contract_analysis: &ContractAnalysis) -> ContractInterface {
    let mut contract_interface = ContractInterface::new();

    let ContractAnalysis {
        private_function_types,
        public_function_types,
        read_only_function_types,
        variable_types,
        persisted_variable_types,
        map_types,
        fungible_tokens,
        non_fungible_tokens,
        defined_traits: _,
        implemented_traits: _,
        expressions: _,
        contract_identifier: _,
        type_map: _,
        cost_track: _,
        contract_interface: _,
        is_cost_contract_eligible: _,
        dependencies: _,
    } = contract_analysis;

    contract_interface
        .functions
        .append(&mut ContractInterfaceFunction::from_map(
            private_function_types,
            ContractInterfaceFunctionAccess::private,
        ));

    contract_interface
        .functions
        .append(&mut ContractInterfaceFunction::from_map(
            public_function_types,
            ContractInterfaceFunctionAccess::public,
        ));

    contract_interface
        .functions
        .append(&mut ContractInterfaceFunction::from_map(
            read_only_function_types,
            ContractInterfaceFunctionAccess::read_only,
        ));

    contract_interface
        .variables
        .append(&mut ContractInterfaceVariable::from_map(
            variable_types,
            ContractInterfaceVariableAccess::constant,
        ));

    contract_interface
        .variables
        .append(&mut ContractInterfaceVariable::from_map(
            persisted_variable_types,
            ContractInterfaceVariableAccess::variable,
        ));

    contract_interface
        .maps
        .append(&mut ContractInterfaceMap::from_map(map_types));

    contract_interface.non_fungible_tokens.append(
        &mut ContractInterfaceNonFungibleTokens::from_map(non_fungible_tokens),
    );

    contract_interface
        .fungible_tokens
        .append(&mut ContractInterfaceFungibleTokens::from_set(
            fungible_tokens,
        ));

    contract_interface
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContractInterfaceFunctionAccess {
    private,
    public,
    read_only,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractInterfaceTupleEntryType {
    pub name: String,
    #[serde(rename = "type")]
    pub type_f: ContractInterfaceAtomType,
}

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
pub struct ContractInterfaceFungibleTokens {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractInterfaceNonFungibleTokens {
    pub name: String,
    #[serde(rename = "type")]
    pub type_f: ContractInterfaceAtomType,
}

impl ContractInterfaceAtomType {
    pub fn from_tuple_type(tuple_type: &TupleTypeSignature) -> ContractInterfaceAtomType {
        ContractInterfaceAtomType::tuple(Self::vec_from_tuple_type(&tuple_type))
    }

    pub fn vec_from_tuple_type(
        tuple_type: &TupleTypeSignature,
    ) -> Vec<ContractInterfaceTupleEntryType> {
        tuple_type
            .get_type_map()
            .iter()
            .map(|(name, sig)| ContractInterfaceTupleEntryType {
                name: name.to_string(),
                type_f: Self::from_type_signature(sig),
            })
            .collect()
    }

    pub fn from_type_signature(sig: &TypeSignature) -> ContractInterfaceAtomType {
        use crate::clarity::types::TypeSignature::*;
        use crate::clarity::types::{SequenceSubtype::*, StringSubtype::*};

        match sig {
            NoType => ContractInterfaceAtomType::none,
            IntType => ContractInterfaceAtomType::int128,
            UIntType => ContractInterfaceAtomType::uint128,
            BoolType => ContractInterfaceAtomType::bool,
            PrincipalType => ContractInterfaceAtomType::principal,
            TraitReferenceType(_) => ContractInterfaceAtomType::trait_reference,
            TupleType(sig) => ContractInterfaceAtomType::from_tuple_type(sig),
            SequenceType(StringType(ASCII(len))) => {
                ContractInterfaceAtomType::string_ascii { length: len.into() }
            }
            SequenceType(StringType(UTF8(len))) => {
                ContractInterfaceAtomType::string_utf8 { length: len.into() }
            }
            SequenceType(BufferType(len)) => {
                ContractInterfaceAtomType::buffer { length: len.into() }
            }
            SequenceType(ListType(list_data)) => {
                let (type_f, length) = list_data.clone().destruct();
                ContractInterfaceAtomType::list {
                    type_f: Box::new(Self::from_type_signature(&type_f)),
                    length,
                }
            }
            OptionalType(sig) => {
                ContractInterfaceAtomType::optional(Box::new(Self::from_type_signature(&sig)))
            }
            TypeSignature::ResponseType(boxed_sig) => {
                let (ok_sig, err_sig) = boxed_sig.as_ref();
                ContractInterfaceAtomType::response {
                    ok: Box::new(Self::from_type_signature(&ok_sig)),
                    error: Box::new(Self::from_type_signature(&err_sig)),
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractInterfaceFunctionArg {
    pub name: String,
    #[serde(rename = "type")]
    pub type_f: ContractInterfaceAtomType,
}

impl ContractInterfaceFunctionArg {
    pub fn from_function_args(fnArgs: &Vec<FunctionArg>) -> Vec<ContractInterfaceFunctionArg> {
        let mut args: Vec<ContractInterfaceFunctionArg> = Vec::new();
        for ref fnArg in fnArgs.iter() {
            args.push(ContractInterfaceFunctionArg {
                name: fnArg.name.to_string(),
                type_f: ContractInterfaceAtomType::from_type_signature(&fnArg.signature),
            });
        }
        args
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractInterfaceFunctionOutput {
    #[serde(rename = "type")]
    pub type_f: ContractInterfaceAtomType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractInterfaceFunction {
    pub name: String,
    pub access: ContractInterfaceFunctionAccess,
    pub args: Vec<ContractInterfaceFunctionArg>,
    pub outputs: ContractInterfaceFunctionOutput,
}

impl ContractInterfaceFunction {
    pub fn from_map(
        map: &BTreeMap<ClarityName, FunctionType>,
        access: ContractInterfaceFunctionAccess,
    ) -> Vec<ContractInterfaceFunction> {
        map.iter()
            .map(|(name, function_type)| ContractInterfaceFunction {
                name: name.clone().into(),
                access: access.to_owned(),
                outputs: ContractInterfaceFunctionOutput {
                    type_f: match function_type {
                        FunctionType::Fixed(FixedFunction { returns, .. }) => {
                            ContractInterfaceAtomType::from_type_signature(&returns)
                        }
                        _ => panic!(
                            "Contract functions should only have fixed function return types!"
                        ),
                    },
                },
                args: match function_type {
                    FunctionType::Fixed(FixedFunction { args, .. }) => {
                        ContractInterfaceFunctionArg::from_function_args(&args)
                    }
                    _ => panic!("Contract functions should only have fixed function arguments!"),
                },
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ContractInterfaceVariableAccess {
    constant,
    variable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractInterfaceVariable {
    pub name: String,
    #[serde(rename = "type")]
    pub type_f: ContractInterfaceAtomType,
    pub access: ContractInterfaceVariableAccess,
}

impl ContractInterfaceFungibleTokens {
    pub fn from_set(tokens: &BTreeSet<ClarityName>) -> Vec<Self> {
        tokens
            .iter()
            .map(|name| Self {
                name: name.to_string(),
            })
            .collect()
    }
}

impl ContractInterfaceNonFungibleTokens {
    pub fn from_map(assets: &BTreeMap<ClarityName, TypeSignature>) -> Vec<Self> {
        assets
            .iter()
            .map(|(name, type_sig)| Self {
                name: name.clone().into(),
                type_f: ContractInterfaceAtomType::from_type_signature(type_sig),
            })
            .collect()
    }
}

impl ContractInterfaceVariable {
    pub fn from_map(
        map: &BTreeMap<ClarityName, TypeSignature>,
        access: ContractInterfaceVariableAccess,
    ) -> Vec<ContractInterfaceVariable> {
        map.iter()
            .map(|(name, type_sig)| ContractInterfaceVariable {
                name: name.clone().into(),
                access: access.to_owned(),
                type_f: ContractInterfaceAtomType::from_type_signature(type_sig),
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractInterfaceMap {
    pub name: String,
    pub key: ContractInterfaceAtomType,
    pub value: ContractInterfaceAtomType,
}

impl ContractInterfaceMap {
    pub fn from_map(
        map: &BTreeMap<ClarityName, (TypeSignature, TypeSignature)>,
    ) -> Vec<ContractInterfaceMap> {
        map.iter()
            .map(|(name, (key_sig, val_sig))| ContractInterfaceMap {
                name: name.clone().into(),
                key: ContractInterfaceAtomType::from_type_signature(key_sig),
                value: ContractInterfaceAtomType::from_type_signature(val_sig),
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContractInterface {
    pub functions: Vec<ContractInterfaceFunction>,
    pub variables: Vec<ContractInterfaceVariable>,
    pub maps: Vec<ContractInterfaceMap>,
    pub fungible_tokens: Vec<ContractInterfaceFungibleTokens>,
    pub non_fungible_tokens: Vec<ContractInterfaceNonFungibleTokens>,
}

impl ContractInterface {
    pub fn new() -> Self {
        Self {
            functions: Vec::new(),
            variables: Vec::new(),
            maps: Vec::new(),
            fungible_tokens: Vec::new(),
            non_fungible_tokens: Vec::new(),
        }
    }

    pub fn serialize(&self) -> String {
        serde_json::to_string(self).expect("Failed to serialize contract interface")
    }
}
