use clarity_repl::clarity::util::hash::hex_bytes;
use orchestra_event_observer::hooks::types::*;
use orchestra_types::{BitcoinNetwork, StacksNetwork};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;

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

impl HookSpecificationFile {
    pub fn parse(path: &PathBuf) -> Result<HookSpecification, String> {
        let file = HookSpecificationFile::new(path)?;
        file.to_specification()
    }

    pub fn new(path: &PathBuf) -> Result<HookSpecificationFile, String> {
        let path = match File::open(path) {
            Ok(path) => path,
            Err(_e) => {
                return Err(format!("unable to locate {}", path.display()));
            }
        };
        let mut hook_spec_file_reader = BufReader::new(path);
        let mut hook_spec_file_buffer = vec![];
        hook_spec_file_reader
            .read_to_end(&mut hook_spec_file_buffer)
            .unwrap();

        let specification: HookSpecificationFile =
            match serde_yaml::from_slice(&hook_spec_file_buffer[..]) {
                Ok(res) => res,
                Err(msg) => return Err(format!("unable to read file {}", msg)),
            };

        Ok(specification)
    }

    pub fn to_specification(&self) -> Result<HookSpecification, String> {
        let res = if self.chain.to_lowercase() == "stacks" {
            let res = self.to_stacks_specification()?;
            HookSpecification::Stacks(res)
        } else if self.chain.to_lowercase() == "bitcoin" {
            let res = self.to_bitcoin_specification()?;
            HookSpecification::Bitcoin(res)
        } else {
            return Err(format!(
                "chain '{}' not supported (stacks, bitcoin)",
                self.chain
            ));
        };
        Ok(res)
    }

    pub fn to_bitcoin_specification(&self) -> Result<BitcoinHookSpecification, String> {
        let network = if self.network.to_lowercase() == "regtest" {
            BitcoinNetwork::Regtest
        } else if self.network.to_lowercase() == "testnet" {
            BitcoinNetwork::Testnet
        } else if self.network.to_lowercase() == "mainnet" {
            BitcoinNetwork::Mainnet
        } else {
            return Err(format!(
                "network '{}' not supported (devnet, testnet or mainnet)",
                self.network
            ));
        };

        Ok(BitcoinHookSpecification {
            id: self.id.unwrap_or(1),
            name: self.name.to_string(),
            network: network,
            start_block: self.start_block,
            end_block: self.end_block,
            version: self.version.unwrap_or(1),
            predicate: self.predicate.to_bitcoin_predicate()?,
            action: self.action.to_specifications()?,
        })
    }

    pub fn to_stacks_specification(&self) -> Result<StacksHookSpecification, String> {
        let network = if self.network.to_lowercase() == "devnet" {
            StacksNetwork::Devnet
        } else if self.network.to_lowercase() == "testnet" {
            StacksNetwork::Testnet
        } else if self.network.to_lowercase() == "mainnet" {
            StacksNetwork::Mainnet
        } else {
            return Err(format!(
                "network '{}' not supported (devnet, testnet or mainnet)",
                self.network
            ));
        };

        Ok(StacksHookSpecification {
            id: self.id.unwrap_or(1),
            name: self.name.to_string(),
            network: network,
            start_block: self.start_block,
            end_block: self.end_block,
            version: self.version.unwrap_or(1),
            predicate: self.predicate.to_stacks_predicate()?,
            action: self.action.to_specifications()?,
        })
    }
}

impl HookActionFile {
    pub fn to_specifications(&self) -> Result<HookAction, String> {
        if let Some(ref specs) = self.http_hook {
            let url = match specs.get("url") {
                Some(url) => Ok(url.to_string()),
                None => Err(format!("url missing for http-hook")),
            }?;
            let method = match specs.get("method") {
                Some(method) => Ok(method.to_string()),
                None => Err(format!("method missing for http-hook")),
            }?;
            Ok(HookAction::HttpHook(HttpHook { url, method }))
        } else {
            Err(format!("action not supported (http-hook)"))
        }
    }
}

impl HookPredicateFile {
    pub fn to_bitcoin_predicate(&self) -> Result<BitcoinHookPredicate, String> {
        if let Some(ref specs) = self.tx_in {
            let predicate = self.extract_bitcoin_predicate(specs)?;
            return Ok(BitcoinHookPredicate::TxIn(predicate));
        } else if let Some(ref specs) = self.tx_out {
            let predicate = self.extract_bitcoin_predicate(specs)?;
            return Ok(BitcoinHookPredicate::TxOut(predicate));
        }
        return Err(format!("trigger not specified (contract-call, event)"));
    }

    pub fn extract_bitcoin_predicate(
        &self,
        specs: &BTreeMap<String, BTreeMap<String, String>>,
    ) -> Result<BitcoinPredicate, String> {
        if let Some(rule) = specs.get("hex") {
            let rule = self.extract_matching_rule(rule)?;
            return Ok(BitcoinPredicate::Hex(rule));
        };

        if let Some(rule) = specs.get("p2pkh") {
            let rule = self.extract_matching_rule(rule)?;
            return Ok(BitcoinPredicate::P2pkh(rule));
        };

        if let Some(rule) = specs.get("p2sh") {
            let rule = self.extract_matching_rule(rule)?;
            return Ok(BitcoinPredicate::P2sh(rule));
        };

        if let Some(rule) = specs.get("p2wpkh") {
            let rule = self.extract_matching_rule(rule)?;
            return Ok(BitcoinPredicate::P2wpkh(rule));
        };

        if let Some(rule) = specs.get("p2wsh") {
            let rule = self.extract_matching_rule(rule)?;
            return Ok(BitcoinPredicate::P2wsh(rule));
        };

        if let Some(rule) = specs.get("script") {
            if let Some(raw) = rule.get("template") {
                let script = ScriptTemplate::parse(raw)?;
                return Ok(BitcoinPredicate::Script(script));
            }
            return Err(format!("predicate rule not specified (template)"));
        };

        return Err(format!(
            "predicate rule not specified (hex, p2pkh, p2sh, p2wpkh, p2wsh, script)"
        ));
    }

    pub fn extract_matching_rule(
        &self,
        specs: &BTreeMap<String, String>,
    ) -> Result<MatchingRule, String> {
        if let Some(rule) = specs.get("starts-with") {
            return Ok(MatchingRule::StartsWith(rule.to_string()));
        };

        if let Some(rule) = specs.get("ends-with") {
            return Ok(MatchingRule::EndsWith(rule.to_string()));
        };

        if let Some(rule) = specs.get("equals") {
            return Ok(MatchingRule::Equals(rule.to_string()));
        };

        return Err(format!(
            "predicate rule not specified (starts-with, ends-with, equals)"
        ));
    }

    pub fn to_stacks_predicate(&self) -> Result<StacksHookPredicate, String> {
        if let Some(ref specs) = self.contract_call {
            let predicate = self.extract_contract_call_predicate(specs)?;
            return Ok(StacksHookPredicate::ContractCall(predicate));
        } else if let Some(ref specs) = self.event {
            let predicate = self.extract_event_predicate(specs)?;
            return Ok(StacksHookPredicate::Event(predicate));
        }
        return Err(format!("trigger not specified (contract-call, event)"));
    }

    pub fn extract_contract_call_predicate(
        &self,
        specs: &BTreeMap<String, String>,
    ) -> Result<StacksContractCallBasedPredicate, String> {
        let contract = match specs.get("contract-id") {
            Some(contract) => Ok(contract.to_string()),
            None => Err(format!("contract missing for predicate.contract-call")),
        }?;
        let method = match specs.get("method") {
            Some(method) => Ok(method.to_string()),
            None => Err(format!("method missing for predicate.contract-call")),
        }?;
        Ok(StacksContractCallBasedPredicate { contract, method })
    }

    pub fn extract_event_predicate(
        &self,
        specs: &BTreeMap<String, BTreeMap<String, String>>,
    ) -> Result<StacksEventBasedPredicate, String> {
        let print_event = match specs.get("print-event") {
            Some(rule) => Some(rule),
            None => None,
        };
        let nft_rule = match specs.get("nft-event") {
            Some(rule) => Some(rule),
            None => None,
        };
        let ft_rule = match specs.get("ft-event") {
            Some(rule) => Some(rule),
            None => None,
        };
        let stx_rule = match specs.get("stx-event") {
            Some(rule) => Some(rule),
            None => None,
        };
        Ok(StacksEventBasedPredicate {})
    }
}
