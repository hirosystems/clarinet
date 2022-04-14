use serde::{Serialize, Deserialize};
use std::path::PathBuf;
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, Read};

use crate::types::{BitcoinNetwork, StacksNetwork};

#[derive(Clone, Debug)]
pub struct HookFormation {
    pub stacks_hooks: Vec<StacksHookSpecification>,
    pub bitcoin_hooks: Vec<BitcoinHookSpecification>,
}

pub enum HookSpecification {
    Bitcoin(BitcoinHookSpecification),
    Stacks(StacksHookSpecification),
}

impl HookSpecification {

    pub fn from_config_file(path: &PathBuf) -> Result<HookSpecification, String> {
        let path = match File::open(path) {
            Ok(path) => path,
            Err(_e) => {
                panic!("unable to locate {}", path.display());
            }
        };
        let mut hook_spec_file_reader = BufReader::new(path);
        let mut hook_spec_file_buffer = vec![];
        hook_spec_file_reader
            .read_to_end(&mut hook_spec_file_buffer)
            .unwrap();

        let specification: HookSpecificationFile = match serde_yaml::from_slice(&hook_spec_file_buffer[..]) {
            Ok(res) => res,
            Err(msg) => {
                return Err(format!("unable to read file {}", msg))
            }
        };

        let hook = match HookSpecification::from_specifications(&specification) {
            Ok(hook) => hook,
            Err(msg) => {
                return Err(format!("hook specification incorrect {}", msg))
            }
        };
        Ok(hook)
    }

    pub fn from_specifications(specs: &HookSpecificationFile) -> Result<HookSpecification, String> {
        let res = if specs.chain.to_lowercase() == "stacks" {
            let res = StacksHookSpecification::from_specifications(specs)?;
            HookSpecification::Stacks(res)
        } else if specs.chain.to_lowercase() == "bitcoin" {
            let res = BitcoinHookSpecification::from_specifications(specs)?;
            HookSpecification::Bitcoin(res)
        } else {
            return Err(format!("chain '{}' not supported (stacks, bitcoin)", specs.chain))
        };
        Ok(res)
    }

    pub fn name(&self) -> &str {
        match &self {
            Self::Bitcoin(data) => {
                &data.name
            },
            Self::Stacks(data) => {
                &data.name
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct BitcoinHookSpecification {
    pub id: u32,
    pub name: String,
    pub network: BitcoinNetwork,
    pub version: u32,
    pub start_block: Option<u64>,
    pub end_block: Option<u64>,
    pub predicate: BitcoinHookPredicate,
    pub action: HookAction,
}

impl BitcoinHookSpecification {
    pub fn from_specifications(specs: &HookSpecificationFile) -> Result<BitcoinHookSpecification, String> {
        let network = if specs.network.to_lowercase() == "regtest" {
            BitcoinNetwork::Regtest
        } else if specs.network.to_lowercase() == "testnet" {
            BitcoinNetwork::Testnet
        } else if specs.network.to_lowercase() == "mainnet" {
            BitcoinNetwork::Mainnet
        } else {
            return Err(format!(
                "network '{}' not supported (devnet, testnet or mainnet)", specs.network
            ));
        };

        Ok(BitcoinHookSpecification {
            id: specs.id.unwrap_or(1),
            name: specs.name.to_string(),
            network: network,
            start_block: specs.start_block,
            end_block: specs.end_block,
            version: specs.version.unwrap_or(1),
            predicate: BitcoinHookPredicate::from_specifications(&specs.predicate)?,
            action: HookAction::from_specifications(&specs.action)?,
        })        }
}

#[derive(Clone, Debug)]
pub enum HookAction {
    HttpHook(HttpHook)
}

impl HookAction {
    pub fn from_specifications(specs: &HookActionFile) -> Result<HookAction, String> {
        if let Some(ref specs) = specs.http_hook {
            let url = match specs.get("url") {
                Some(url) => Ok(url.to_string()),
                None => Err(format!("url missing for http-hook"))
            }?;
            let method = match specs.get("method") {
                Some(method) => Ok(method.to_string()),
                None => Err(format!("method missing for http-hook"))
            }?;
            Ok(HookAction::HttpHook(HttpHook {
                url,
                method
            }))
        } else {
            Err(format!("action not supported (http-hook)"))
        }
    }
}

#[derive(Clone, Debug)]
pub struct HttpHook {
    pub url: String,
    pub method: String,
}

#[derive(Clone, Debug)]
pub enum BitcoinHookPredicate {
    TxIn(BitcoinPredicate),
    TxOut(BitcoinPredicate),
}

impl BitcoinHookPredicate {

    pub fn from_specifications(specs: &HookPredicateFile) -> Result<BitcoinHookPredicate, String> {
        if let Some(ref specs) = specs.tx_in {
            let predicate = BitcoinPredicate::from_specifications(specs)?;
            return Ok(BitcoinHookPredicate::TxIn(predicate))
        } else if let Some(ref specs) = specs.tx_out {
            let predicate = BitcoinPredicate::from_specifications(specs)?;
            return Ok(BitcoinHookPredicate::TxOut(predicate))
        }
        return Err(format!("trigger not specified (contract-call, event)"))
    }    
}

#[derive(Clone, Debug)]
pub enum BitcoinPredicate {
    Hex(MatchingRule),
    P2PKH(MatchingRule),
    P2SH(MatchingRule),
    P2WPKH(MatchingRule),
    P2WSH(MatchingRule),
}

impl BitcoinPredicate {
    pub fn from_specifications(specs: &BTreeMap<String, BTreeMap<String, String>>) -> Result<BitcoinPredicate, String> {
        if let Some(rule) = specs.get("hex") {
            let rule = MatchingRule::from_specifications(rule)?;
            return Ok(BitcoinPredicate::Hex(rule))
        };

        if let Some(rule) = specs.get("p2pkh") {
            let rule = MatchingRule::from_specifications(rule)?;
            return Ok(BitcoinPredicate::P2PKH(rule))
        };

        if let Some(rule) = specs.get("p2sh") {
            let rule = MatchingRule::from_specifications(rule)?;
            return Ok(BitcoinPredicate::P2SH(rule))
        };

        if let Some(rule) = specs.get("p2wpkh") {
            let rule = MatchingRule::from_specifications(rule)?;
            return Ok(BitcoinPredicate::P2WPKH(rule))
        };

        if let Some(rule) = specs.get("p2wsh") {
            let rule = MatchingRule::from_specifications(rule)?;
            return Ok(BitcoinPredicate::P2WSH(rule))
        };

        return Err(format!("predicate rule not specified (hex, p2pkh, p2sh, p2wpkh, p2wsh)"));
    }    
}

#[derive(Clone, Debug)]
pub enum MatchingRule {
    Equals(String),
    StartsWith(String),
    EndsWith(String),
}

impl MatchingRule {
    pub fn from_specifications(specs: &BTreeMap<String, String>) -> Result<MatchingRule, String> {
        if let Some(rule) = specs.get("starts-with") {
            return Ok(MatchingRule::StartsWith(rule.to_string()))
        };

        if let Some(rule) = specs.get("ends-with") {
            return Ok(MatchingRule::EndsWith(rule.to_string()))
        };

        if let Some(rule) = specs.get("equals") {
            return Ok(MatchingRule::Equals(rule.to_string()))
        };

        return Err(format!("predicate rule not specified (starts-with, ends-with, equals)"));
    }    
}

#[derive(Clone, Debug)]
pub struct BitcoinTxInBasedPredicate {
    pub rule: BitcoinPredicate,
}

impl BitcoinTxInBasedPredicate {
    pub fn from_specifications(specs: &BTreeMap<String, BTreeMap<String, String>>) -> Result<BitcoinTxInBasedPredicate, String> {
        let rule = BitcoinPredicate::from_specifications(specs)?;
        Ok(BitcoinTxInBasedPredicate {
            rule
        })
    }    
}

#[derive(Clone, Debug)]
pub struct BitcoinTxOutBasedPredicate {
    pub rule: BitcoinPredicate,
}

impl BitcoinTxOutBasedPredicate {
    pub fn from_specifications(specs: &BTreeMap<String, BTreeMap<String, String>>) -> Result<BitcoinTxOutBasedPredicate, String> {
        let rule = BitcoinPredicate::from_specifications(specs)?;
        Ok(BitcoinTxOutBasedPredicate {
            rule
        })
    }
}

#[derive(Clone, Debug)]
pub struct StacksHookSpecification {
    pub id: u32,
    pub name: String,
    pub network: StacksNetwork,
    pub version: u32,
    pub start_block: Option<u64>,
    pub end_block: Option<u64>,
    pub predicate: StacksHookPredicate,
    pub action: HookAction,
}

impl StacksHookSpecification {
    pub fn from_specifications(specs: &HookSpecificationFile) -> Result<StacksHookSpecification, String> {
        let network = if specs.network.to_lowercase() == "devnet" {
            StacksNetwork::Devnet
        } else if specs.network.to_lowercase() == "testnet" {
            StacksNetwork::Testnet
        } else if specs.network.to_lowercase() == "mainnet" {
            StacksNetwork::Mainnet
        } else {
            return Err(format!(
                "network '{}' not supported (devnet, testnet or mainnet)", specs.network
            ));
        };

        Ok(StacksHookSpecification {
            id: specs.id.unwrap_or(1),
            name: specs.name.to_string(),
            network: network,
            start_block: specs.start_block,
            end_block: specs.end_block,
            version: specs.version.unwrap_or(1),
            predicate: StacksHookPredicate::from_specifications(&specs.predicate)?,
            action: HookAction::from_specifications(&specs.action)?,
        })      
    }
}

#[derive(Clone, Debug)]
pub enum StacksHookPredicate {
    ContractCall(StacksContractCallBasedPredicate),
    Event(StacksEventBasedPredicate),
}

#[derive(Clone, Debug)]
pub struct StacksContractCallBasedPredicate {
    pub contract: String,
    pub method: String,
}

impl StacksContractCallBasedPredicate {
    pub fn from_specifications(specs: &BTreeMap<String, String>) -> Result<StacksContractCallBasedPredicate, String> {
        let contract = match specs.get("contract-id") {
            Some(contract) => Ok(contract.to_string()),
            None => Err(format!("contract missing for predicate.contract-call"))
        }?;
        let method = match specs.get("method") {
            Some(method) => Ok(method.to_string()),
            None => Err(format!("method missing for predicate.contract-call"))
        }?;
        Ok(StacksContractCallBasedPredicate {
            contract,
            method,
        })
    }    
}

#[derive(Clone, Debug)]
pub struct StacksEventBasedPredicate {
}

impl StacksEventBasedPredicate {
    pub fn from_specifications(specs: &BTreeMap<String, BTreeMap<String, String>>) -> Result<StacksEventBasedPredicate, String> {
        let print_event = match specs.get("print-event") {
            Some(rule) => Some(rule),
            None => None
        };
        let nft_rule = match specs.get("nft-event") {
            Some(rule) => Some(rule),
            None => None
        };
        let ft_rule = match specs.get("ft-event") {
            Some(rule) => Some(rule),
            None => None
        };
        let stx_rule = match specs.get("stx-event") {
            Some(rule) => Some(rule),
            None => None
        };
        Ok(StacksEventBasedPredicate {
        })
    }    
}

impl StacksHookPredicate {
    pub fn from_specifications(specs: &HookPredicateFile) -> Result<StacksHookPredicate, String> {
        if let Some(ref specs) = specs.contract_call {
            let predicate = StacksContractCallBasedPredicate::from_specifications(specs)?;
            return Ok(StacksHookPredicate::ContractCall(predicate))
        } else if let Some(ref specs) = specs.event {
            let predicate = StacksEventBasedPredicate::from_specifications(specs)?;
            return Ok(StacksHookPredicate::Event(predicate))
        }
        return Err(format!("trigger not specified (contract-call, event)"))
    }    
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct HookSpecificationFile {
    id: Option<u32>,
    name: String,
    network: String,
    version: Option<u32>,
    #[serde(rename = "start-block")]
    start_block: Option<u64>,
    #[serde(rename = "end-block")]
    end_block: Option<u64>,
    chain: String,
    predicate: HookPredicateFile,
    action: HookActionFile,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct HookPredicateFile {
    event: Option<BTreeMap<String, BTreeMap<String, String>>>,
    #[serde(rename = "contract-call")]
    contract_call: Option<BTreeMap<String, String>>,
    #[serde(rename = "tx-in")]
    tx_in: Option<BTreeMap<String, BTreeMap<String, String>>>,
    #[serde(rename = "tx-out")]
    tx_out: Option<BTreeMap<String, BTreeMap<String, String>>>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct HookActionFile {
    #[serde(rename = "http-hook")]
    http_hook: Option<BTreeMap<String, String>>,
}
