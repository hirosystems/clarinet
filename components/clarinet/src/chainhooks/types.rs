use orchestra_event_observer::chainhooks::types::*;
use orchestra_types::{BitcoinNetwork, StacksNetwork};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct ChainhookSpecificationFile {
    id: Option<u32>,
    name: String,
    version: Option<u32>,
    chain: String,
    networks: BTreeMap<String, ChainhookNetworkSpecificationFile>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ChainhookNetworkSpecificationFile {
    start_block: Option<u64>,
    end_block: Option<u64>,
    predicate: ChainhookPredicateFile,
    action: HookActionFile,
    oreo_url: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ChainhookPredicateFile {
    print_event: Option<PrintEventPredicateFile>,
    ft_event: Option<FtEventPredicateFile>,
    nft_event: Option<NftEventPredicateFile>,
    stx_event: Option<StxEventPredicateFile>,
    contract_call: Option<BTreeMap<String, String>>,
    hex: Option<BTreeMap<String, String>>,
    p2pkh: Option<BTreeMap<String, String>>,
    p2sh: Option<BTreeMap<String, String>>,
    p2wpkh: Option<BTreeMap<String, String>>,
    p2wsh: Option<BTreeMap<String, String>>,
    script: Option<BTreeMap<String, String>>,
    scope: Option<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PrintEventPredicateFile {
    contract_identifier: String,
    contains: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FtEventPredicateFile {
    asset_identifier: String,
    actions: Vec<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct NftEventPredicateFile {
    asset_identifier: String,
    actions: Vec<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct StxEventPredicateFile {
    actions: Vec<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct HookActionFile {
    http: Option<BTreeMap<String, String>>,
}

impl ChainhookSpecificationFile {
    pub fn parse(
        path: &PathBuf,
        networks: &(BitcoinNetwork, StacksNetwork),
    ) -> Result<ChainhookSpecification, String> {
        let file = ChainhookSpecificationFile::new(path)?;
        file.to_specification(networks)
    }

    pub fn new(path: &PathBuf) -> Result<ChainhookSpecificationFile, String> {
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

        let specification: ChainhookSpecificationFile =
            match serde_yaml::from_slice(&hook_spec_file_buffer[..]) {
                Ok(res) => res,
                Err(msg) => return Err(format!("unable to read file {}", msg)),
            };

        Ok(specification)
    }

    pub fn to_specification(
        &self,
        networks: &(BitcoinNetwork, StacksNetwork),
    ) -> Result<ChainhookSpecification, String> {
        let res = if self.chain.to_lowercase() == "stacks" {
            let res = self.to_stacks_specification(&networks.1)?;
            ChainhookSpecification::Stacks(res)
        } else if self.chain.to_lowercase() == "bitcoin" {
            let res = self.to_bitcoin_specification(&networks.0)?;
            ChainhookSpecification::Bitcoin(res)
        } else {
            return Err(format!(
                "chain '{}' not supported (stacks, bitcoin)",
                self.chain
            ));
        };
        Ok(res)
    }

    pub fn to_bitcoin_specification(
        &self,
        network: &BitcoinNetwork,
    ) -> Result<BitcoinChainhookSpecification, String> {
        let network_ser = format!("{:?}", network).to_lowercase();
        let network_spec = match self.networks.get(&network_ser) {
            Some(entry) => entry,
            None => {
                return Err(format!(
                    "network '{}' not found in chainhook specification file",
                    network_ser
                ))
            }
        };

        Ok(BitcoinChainhookSpecification {
            uuid: format!("{}", self.id.unwrap_or(1)),
            version: self.version.unwrap_or(1),
            name: self.name.to_string(),
            network: network.clone(),
            start_block: network_spec.start_block,
            end_block: network_spec.end_block,
            predicate: network_spec.predicate.to_bitcoin_predicate()?,
            action: network_spec.action.to_specifications()?,
        })
    }

    pub fn to_stacks_specification(
        &self,
        network: &StacksNetwork,
    ) -> Result<StacksChainhookSpecification, String> {
        let network_ser = format!("{:?}", network).to_lowercase();
        let network_spec = match self.networks.get(&network_ser) {
            Some(entry) => entry,
            None => {
                return Err(format!(
                    "network '{}' not found in chainhook specification file",
                    network_ser
                ))
            }
        };

        Ok(StacksChainhookSpecification {
            uuid: format!("{}", self.id.unwrap_or(1)),
            version: self.version.unwrap_or(1),
            name: self.name.to_string(),
            network: network.clone(),
            start_block: network_spec.start_block,
            end_block: network_spec.end_block,
            predicate: network_spec.predicate.to_stacks_predicate()?,
            action: network_spec.action.to_specifications()?,
        })
    }
}

impl HookActionFile {
    pub fn to_specifications(&self) -> Result<HookAction, String> {
        if let Some(ref specs) = self.http {
            let url = match specs.get("url") {
                Some(url) => Ok(url.to_string()),
                None => Err(format!("url missing for http")),
            }?;
            let method = match specs.get("method") {
                Some(method) => Ok(method.to_string()),
                None => Err(format!("method missing for http")),
            }?;
            let authorization_header = match specs.get("authorization-header") {
                Some(authorization_header) => Ok(authorization_header.to_string()),
                None => Err(format!("method missing for http")),
            }?;

            Ok(HookAction::Http(HttpHook {
                url,
                method,
                authorization_header,
            }))
        } else {
            Err(format!("action not supported (http)"))
        }
    }
}

impl ChainhookPredicateFile {
    pub fn to_bitcoin_predicate(&self) -> Result<BitcoinHookPredicate, String> {
        if let Some(ref specs) = self.hex {
            let rule = BitcoinPredicateType::Hex(self.extract_matching_rule(specs)?);
            let scope = self.extract_scope()?;
            return Ok(BitcoinHookPredicate::new(scope, rule));
        } else if let Some(ref specs) = self.p2pkh {
            let rule = BitcoinPredicateType::P2pkh(self.extract_matching_rule(specs)?);
            let scope = self.extract_scope()?;
            return Ok(BitcoinHookPredicate::new(scope, rule));
        } else if let Some(ref specs) = self.p2sh {
            let rule = BitcoinPredicateType::P2sh(self.extract_matching_rule(specs)?);
            let scope = self.extract_scope()?;
            return Ok(BitcoinHookPredicate::new(scope, rule));
        } else if let Some(ref specs) = self.p2wpkh {
            let rule = BitcoinPredicateType::P2wpkh(self.extract_matching_rule(specs)?);
            let scope = self.extract_scope()?;
            return Ok(BitcoinHookPredicate::new(scope, rule));
        } else if let Some(ref specs) = self.p2wsh {
            let rule = BitcoinPredicateType::P2wsh(self.extract_matching_rule(specs)?);
            let scope = self.extract_scope()?;
            return Ok(BitcoinHookPredicate::new(scope, rule));
        } else if let Some(ref _specs) = self.script {
            // let rule = BitcoinPredicateType::Script(self.ex(specs)?);
            // let scope = self.extract_scope()?;
            // return Ok(BitcoinHookPredicate::new(scope, rule));
            return Err(format!("trigger script unimplemented"));
        }
        return Err(format!(
            "trigger not specified (hex, p2pkh, p2sh, p2wpkh, p2wsh, script)"
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

    pub fn extract_scope(&self) -> Result<Scope, String> {
        if let Some(ref scope) = self.scope {
            let scope = match scope.as_str() {
                "inputs" => Scope::Inputs,
                "outputs" => Scope::Outputs,
                _ => return Err(format!("predicate scope not specified (inputs, outputs)")),
            };
            return Ok(scope);
        };
        return Err(format!("predicate scope not specified (inputs, outputs)"));
    }

    pub fn to_stacks_predicate(&self) -> Result<StacksHookPredicate, String> {
        if let Some(ref specs) = self.contract_call {
            let predicate = self.extract_contract_call_predicate(specs)?;
            return Ok(StacksHookPredicate::ContractCall(predicate));
        } else if let Some(ref specs) = self.print_event {
            let predicate = self.extract_print_event_predicate(specs)?;
            return Ok(StacksHookPredicate::PrintEvent(predicate));
        } else if let Some(ref specs) = self.ft_event {
            let predicate = self.extract_ft_event_predicate(specs)?;
            return Ok(StacksHookPredicate::FtEvent(predicate));
        } else if let Some(ref specs) = self.nft_event {
            let predicate = self.extract_nft_event_predicate(specs)?;
            return Ok(StacksHookPredicate::NftEvent(predicate));
        } else if let Some(ref specs) = self.stx_event {
            let predicate = self.extract_stx_event_predicate(specs)?;
            return Ok(StacksHookPredicate::StxEvent(predicate));
        }
        return Err(format!("trigger not specified (contract-call, event)"));
    }

    pub fn extract_contract_call_predicate(
        &self,
        specs: &BTreeMap<String, String>,
    ) -> Result<StacksContractCallBasedPredicate, String> {
        let contract_identifier = match specs.get("contract-identifier") {
            Some(contract) => Ok(contract.to_string()),
            None => Err(format!(
                "contract-identifier missing for predicate.contract-call"
            )),
        }?;
        let method = match specs.get("method") {
            Some(method) => Ok(method.to_string()),
            None => Err(format!("method missing for predicate.contract-call")),
        }?;
        Ok(StacksContractCallBasedPredicate {
            contract_identifier,
            method,
        })
    }

    pub fn extract_print_event_predicate(
        &self,
        specs: &PrintEventPredicateFile,
    ) -> Result<StacksPrintEventBasedPredicate, String> {
        Ok(StacksPrintEventBasedPredicate {
            contract_identifier: specs.contract_identifier.clone(),
            contains: specs.contains.clone(),
        })
    }

    pub fn extract_ft_event_predicate(
        &self,
        specs: &FtEventPredicateFile,
    ) -> Result<StacksFtEventBasedPredicate, String> {
        Ok(StacksFtEventBasedPredicate {
            asset_identifier: specs.asset_identifier.clone(),
            actions: specs.actions.clone(),
        })
    }

    pub fn extract_nft_event_predicate(
        &self,
        specs: &NftEventPredicateFile,
    ) -> Result<StacksNftEventBasedPredicate, String> {
        Ok(StacksNftEventBasedPredicate {
            asset_identifier: specs.asset_identifier.clone(),
            actions: specs.actions.clone(),
        })
    }

    pub fn extract_stx_event_predicate(
        &self,
        specs: &StxEventPredicateFile,
    ) -> Result<StacksStxEventBasedPredicate, String> {
        Ok(StacksStxEventBasedPredicate {
            actions: specs.actions.clone(),
        })
    }
}
