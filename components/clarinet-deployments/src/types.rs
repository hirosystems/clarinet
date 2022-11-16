use clarinet_files::FileLocation;
use clarity_repl::clarity::util::hash::{hex_bytes, to_hex};
use clarity_repl::clarity::vm::analysis::ContractAnalysis;
use clarity_repl::clarity::vm::ast::ContractAST;
use clarity_repl::clarity::vm::diagnostic::Diagnostic;
use clarity_repl::clarity::vm::types::{
    PrincipalData, QualifiedContractIdentifier, StandardPrincipalData,
};

use clarity_repl::clarity::{ClarityName, ClarityVersion, ContractName};

use chainhook_types::StacksNetwork;
use serde::{Deserialize, Serialize};
use serde_yaml;
use std::collections::BTreeMap;

use clarity_repl::analysis::ast_dependency_detector::DependencySet;
use clarity_repl::repl::{Session, DEFAULT_CLARITY_VERSION};
use std::collections::HashMap;

pub struct DeploymentGenerationArtifacts {
    pub asts: HashMap<QualifiedContractIdentifier, ContractAST>,
    pub deps: HashMap<QualifiedContractIdentifier, DependencySet>,
    pub diags: HashMap<QualifiedContractIdentifier, Vec<Diagnostic>>,
    pub analysis: HashMap<QualifiedContractIdentifier, ContractAnalysis>,
    pub session: Session,
    pub success: bool,
}

#[derive(Debug, PartialEq, Clone)]
pub struct TransactionPlanSpecification {
    pub batches: Vec<TransactionsBatchSpecification>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TransactionPlanSpecificationFile {
    pub batches: Vec<TransactionsBatchSpecificationFile>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TransactionsBatchSpecificationFile {
    pub id: usize,
    pub transactions: Vec<TransactionSpecificationFile>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransactionSpecificationFile {
    ContractCall(ContractCallSpecificationFile),
    ContractPublish(ContractPublishSpecificationFile),
    EmulatedContractCall(EmulatedContractCallSpecificationFile),
    EmulatedContractPublish(EmulatedContractPublishSpecificationFile),
    RequirementPublish(RequirementPublishSpecificationFile),
    BtcTransfer(BtcTransferSpecificationFile),
    StxTransfer(StxTransferSpecificationFile),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct StxTransferSpecificationFile {
    pub expected_sender: String,
    pub recipient: String,
    pub mstx_amount: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<String>,
    pub cost: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor_block_only: Option<bool>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BtcTransferSpecificationFile {
    pub expected_sender: String,
    pub recipient: String,
    pub sats_amount: u64,
    pub sats_per_byte: u64,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ContractCallSpecificationFile {
    pub contract_id: String,
    pub expected_sender: String,
    pub method: String,
    pub parameters: Vec<String>,
    pub cost: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor_block_only: Option<bool>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RequirementPublishSpecificationFile {
    pub contract_id: String,
    pub remap_sender: String,
    pub remap_principals: Option<BTreeMap<String, String>>,
    pub cost: u64,
    #[serde(flatten)]
    pub location: Option<FileLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clarity_version: Option<u8>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ContractPublishSpecificationFile {
    pub contract_name: String,
    pub expected_sender: String,
    pub cost: u64,
    #[serde(flatten)]
    pub location: Option<FileLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor_block_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clarity_version: Option<u8>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct EmulatedContractCallSpecificationFile {
    pub contract_id: String,
    pub emulated_sender: String,
    pub method: String,
    pub parameters: Vec<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct EmulatedContractPublishSpecificationFile {
    pub contract_name: String,
    pub emulated_sender: String,
    #[serde(flatten)]
    pub location: Option<FileLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clarity_version: Option<u8>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct TransactionsBatchSpecification {
    pub id: usize,
    pub transactions: Vec<TransactionSpecification>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TransactionSpecification {
    ContractCall(ContractCallSpecification),
    ContractPublish(ContractPublishSpecification),
    RequirementPublish(RequirementPublishSpecification),
    EmulatedContractCall(EmulatedContractCallSpecification),
    EmulatedContractPublish(EmulatedContractPublishSpecification),
    BtcTransfer(BtcTransferSpecification),
    StxTransfer(StxTransferSpecification),
}

#[derive(Debug, PartialEq, Clone)]
pub struct StxTransferSpecification {
    pub expected_sender: StandardPrincipalData,
    pub recipient: PrincipalData,
    pub mstx_amount: u64,
    pub memo: [u8; 34],
    pub cost: u64,
    pub anchor_block_only: bool,
}

impl StxTransferSpecification {
    pub fn from_specifications(
        specs: &StxTransferSpecificationFile,
    ) -> Result<StxTransferSpecification, String> {
        let expected_sender = match PrincipalData::parse_standard_principal(&specs.expected_sender)
        {
            Ok(res) => res,
            Err(_) => {
                return Err(format!(
                    "unable to parse expected sender '{}' as a valid Stacks address",
                    specs.expected_sender
                ))
            }
        };

        let recipient = match PrincipalData::parse(&specs.recipient) {
            Ok(res) => res,
            Err(_) => {
                return Err(format!(
                    "unable to parse recipient '{}' as a valid Stacks address",
                    specs.expected_sender
                ))
            }
        };

        let mut memo = [0u8; 34];
        if let Some(ref hex_memo) = specs.memo {
            if !hex_memo.is_empty() && !hex_memo.starts_with("0x") {
                return Err(format!(
                    "unable to parse memo (up to 34 bytes, starting with '0x')",
                ));
            }
            match hex_bytes(&hex_memo[2..]) {
                Ok(ref mut bytes) => {
                    bytes.resize(34, 0);
                    memo.copy_from_slice(&bytes);
                }
                Err(_) => return Err(format!("unable to parse memo (up to 34 bytes)",)),
            }
        }

        Ok(StxTransferSpecification {
            expected_sender,
            recipient,
            memo,
            mstx_amount: specs.mstx_amount,
            cost: specs.cost,
            anchor_block_only: specs.anchor_block_only.unwrap_or(true),
        })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct BtcTransferSpecification {
    pub expected_sender: String,
    pub recipient: String,
    pub sats_amount: u64,
    pub sats_per_byte: u64,
}

impl BtcTransferSpecification {
    pub fn from_specifications(
        specs: &BtcTransferSpecificationFile,
    ) -> Result<BtcTransferSpecification, String> {
        // TODO(lgalabru): Data validation
        Ok(BtcTransferSpecification {
            expected_sender: specs.expected_sender.clone(),
            recipient: specs.recipient.clone(),
            sats_amount: specs.sats_amount,
            sats_per_byte: specs.sats_per_byte,
        })
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ContractCallSpecification {
    pub contract_id: QualifiedContractIdentifier,
    pub expected_sender: StandardPrincipalData,
    pub method: ClarityName,
    pub parameters: Vec<String>,
    pub cost: u64,
    pub anchor_block_only: bool,
}

impl ContractCallSpecification {
    pub fn from_specifications(
        specs: &ContractCallSpecificationFile,
    ) -> Result<ContractCallSpecification, String> {
        let contract_id = match QualifiedContractIdentifier::parse(&specs.contract_id) {
            Ok(res) => res,
            Err(_) => {
                return Err(format!(
                    "unable to parse '{}' as a valid contract identifier",
                    specs.contract_id
                ))
            }
        };

        let expected_sender = match PrincipalData::parse_standard_principal(&specs.expected_sender)
        {
            Ok(res) => res,
            Err(_) => {
                return Err(format!(
                    "unable to parse emulated sender '{}' as a valid Stacks address",
                    specs.expected_sender
                ))
            }
        };

        let method = match ClarityName::try_from(specs.method.to_string()) {
            Ok(res) => res,
            Err(_) => {
                return Err(format!(
                    "unable to parse '{}' as a valid contract name",
                    specs.method
                ))
            }
        };

        Ok(ContractCallSpecification {
            contract_id,
            expected_sender,
            method,
            parameters: specs.parameters.clone(),
            cost: specs.cost,
            anchor_block_only: specs.anchor_block_only.unwrap_or(true),
        })
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct ContractPublishSpecification {
    pub contract_name: ContractName,
    pub expected_sender: StandardPrincipalData,
    pub location: FileLocation,
    pub source: String,
    pub clarity_version: ClarityVersion,
    pub cost: u64,
    pub anchor_block_only: bool,
}

impl ContractPublishSpecification {
    pub fn from_specifications(
        specs: &ContractPublishSpecificationFile,
        project_root_location: &FileLocation,
    ) -> Result<ContractPublishSpecification, String> {
        let contract_name = match ContractName::try_from(specs.contract_name.to_string()) {
            Ok(res) => res,
            Err(_) => {
                return Err(format!(
                    "unable to parse '{}' as a valid contract name",
                    specs.contract_name
                ))
            }
        };

        let expected_sender = match PrincipalData::parse_standard_principal(&specs.expected_sender)
        {
            Ok(res) => res,
            Err(_) => {
                return Err(format!(
                    "unable to parse expected sender '{}' as a valid Stacks address",
                    specs.expected_sender
                ))
            }
        };

        let location = match (&specs.path, &specs.url) {
            (Some(location_string), None) | (None, Some(location_string)) => {
                FileLocation::try_parse(location_string, Some(project_root_location))
            }
            _ => None,
        }
        .ok_or(format!(
            "unable to parse file location (can either be 'path' or 'url'",
        ))?;

        let source = location.read_content_as_utf8()?;

        let clarity_version = match specs.clarity_version {
            Some(clarity_version) => {
                if clarity_version.eq(&1) {
                    Ok(ClarityVersion::Clarity1)
                } else if clarity_version.eq(&2) {
                    Ok(ClarityVersion::Clarity2)
                } else {
                    Err(format!(
                        "unable to parse clarity_version (can either be '1' or '2'",
                    ))
                }
            }
            _ => Ok(DEFAULT_CLARITY_VERSION),
        }?;

        Ok(ContractPublishSpecification {
            contract_name,
            expected_sender,
            source,
            location: location,
            cost: specs.cost,
            anchor_block_only: specs.anchor_block_only.unwrap_or(true),
            clarity_version,
        })
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct RequirementPublishSpecification {
    pub contract_id: QualifiedContractIdentifier,
    pub remap_sender: StandardPrincipalData,
    pub remap_principals: BTreeMap<StandardPrincipalData, StandardPrincipalData>,
    pub source: String,
    pub clarity_version: ClarityVersion,
    pub cost: u64,
    pub location: FileLocation,
}

impl RequirementPublishSpecification {
    pub fn from_specifications(
        specs: &RequirementPublishSpecificationFile,
        project_root_location: &FileLocation,
    ) -> Result<RequirementPublishSpecification, String> {
        let contract_id = match QualifiedContractIdentifier::parse(&specs.contract_id) {
            Ok(res) => res,
            Err(_) => {
                return Err(format!(
                    "unable to parse '{}' as a valid contract identifier",
                    specs.contract_id
                ))
            }
        };

        let remap_sender = match PrincipalData::parse_standard_principal(&specs.remap_sender) {
            Ok(res) => res,
            Err(_) => {
                return Err(format!(
                    "unable to parse remap sender '{}' as a valid Stacks address",
                    specs.remap_sender
                ))
            }
        };

        let mut remap_principals = BTreeMap::new();
        if let Some(ref remap_principals_spec) = specs.remap_principals {
            for (src_spec, dst_spec) in remap_principals_spec {
                let src = match PrincipalData::parse_standard_principal(&src_spec) {
                    Ok(res) => res,
                    Err(_) => {
                        return Err(format!(
                            "unable to parse remap source '{}' as a valid Stacks address",
                            specs.remap_sender
                        ))
                    }
                };
                let dst = match PrincipalData::parse_standard_principal(&dst_spec) {
                    Ok(res) => res,
                    Err(_) => {
                        return Err(format!(
                            "unable to parse remap destination '{}' as a valid Stacks address",
                            specs.remap_sender
                        ))
                    }
                };
                remap_principals.insert(src, dst);
            }
        }

        let location = match (&specs.path, &specs.url) {
            (Some(location_string), None) | (None, Some(location_string)) => {
                FileLocation::try_parse(location_string, Some(project_root_location))
            }
            _ => None,
        }
        .ok_or(format!(
            "unable to parse file location (can either be 'path' or 'url'",
        ))?;

        let source = location.read_content_as_utf8()?;

        let clarity_version = match specs.clarity_version {
            Some(clarity_version) => {
                if clarity_version.eq(&1) {
                    Ok(ClarityVersion::Clarity1)
                } else if clarity_version.eq(&2) {
                    Ok(ClarityVersion::Clarity2)
                } else {
                    Err(format!(
                        "unable to parse clarity_version (can either be '1' or '2'",
                    ))
                }
            }
            _ => Ok(DEFAULT_CLARITY_VERSION),
        }?;

        Ok(RequirementPublishSpecification {
            contract_id,
            remap_sender,
            remap_principals,
            source,
            clarity_version,
            location: location,
            cost: specs.cost,
        })
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct EmulatedContractCallSpecification {
    pub contract_id: QualifiedContractIdentifier,
    pub emulated_sender: StandardPrincipalData,
    pub method: ClarityName,
    pub parameters: Vec<String>,
}

impl EmulatedContractCallSpecification {
    pub fn from_specifications(
        specs: &EmulatedContractCallSpecificationFile,
    ) -> Result<EmulatedContractCallSpecification, String> {
        let contract_id = match QualifiedContractIdentifier::parse(&specs.contract_id) {
            Ok(res) => res,
            Err(_) => {
                return Err(format!(
                    "unable to parse '{}' as a valid contract_id",
                    specs.contract_id
                ))
            }
        };

        let emulated_sender = match PrincipalData::parse_standard_principal(&specs.emulated_sender)
        {
            Ok(res) => res,
            Err(_) => {
                return Err(format!(
                    "unable to parse emulated sender '{}' as a valid Stacks address",
                    specs.emulated_sender
                ))
            }
        };

        let method = match ClarityName::try_from(specs.method.to_string()) {
            Ok(res) => res,
            Err(_) => {
                return Err(format!(
                    "unable to parse '{}' as a valid contract name",
                    specs.method
                ))
            }
        };

        Ok(EmulatedContractCallSpecification {
            contract_id,
            emulated_sender,
            method,
            parameters: specs.parameters.clone(),
        })
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct EmulatedContractPublishSpecification {
    pub contract_name: ContractName,
    pub emulated_sender: StandardPrincipalData,
    pub source: String,
    pub clarity_version: ClarityVersion,
    pub location: FileLocation,
}

impl EmulatedContractPublishSpecification {
    pub fn from_specifications(
        specs: &EmulatedContractPublishSpecificationFile,
        project_root_location: &FileLocation,
    ) -> Result<EmulatedContractPublishSpecification, String> {
        let contract_name = match ContractName::try_from(specs.contract_name.to_string()) {
            Ok(res) => res,
            Err(_) => {
                return Err(format!(
                    "unable to parse '{}' as a valid contract name",
                    specs.contract_name
                ))
            }
        };

        let emulated_sender = match PrincipalData::parse_standard_principal(&specs.emulated_sender)
        {
            Ok(res) => res,
            Err(_) => {
                return Err(format!(
                    "unable to parse emulated sender '{}' as a valid Stacks address",
                    specs.emulated_sender
                ))
            }
        };

        let location = match (&specs.path, &specs.url) {
            (Some(location_string), None) | (None, Some(location_string)) => {
                FileLocation::try_parse(location_string, Some(project_root_location))
            }
            _ => None,
        }
        .ok_or(format!(
            "unable to parse file location (can either be 'path' or 'url'",
        ))?;

        let clarity_version = match specs.clarity_version {
            Some(clarity_version) => {
                if clarity_version.eq(&1) {
                    Ok(ClarityVersion::Clarity1)
                } else if clarity_version.eq(&2) {
                    Ok(ClarityVersion::Clarity2)
                } else {
                    Err(format!(
                        "unable to parse clarity_version (can either be '1' or '2'",
                    ))
                }
            }
            _ => Ok(DEFAULT_CLARITY_VERSION),
        }?;

        let source = location.read_content_as_utf8()?;

        Ok(EmulatedContractPublishSpecification {
            contract_name,
            emulated_sender,
            source,
            location,
            clarity_version,
        })
    }
}

#[derive(Debug, Clone)]
pub struct DeploymentSpecification {
    pub id: u32,
    pub name: String,
    pub network: StacksNetwork,
    pub stacks_node: Option<String>,
    pub bitcoin_node: Option<String>,
    pub genesis: Option<GenesisSpecification>,
    pub plan: TransactionPlanSpecification,
    // Keep a cache of contract's (source, relative_path)
    pub contracts: BTreeMap<QualifiedContractIdentifier, (String, FileLocation)>,
}

impl DeploymentSpecification {
    pub fn from_config_file(
        deployment_location: &FileLocation,
        project_root_location: &FileLocation,
    ) -> Result<DeploymentSpecification, String> {
        let spec_file_content = deployment_location.read_content()?;

        let specification_file: DeploymentSpecificationFile =
            match serde_yaml::from_slice(&spec_file_content[..]) {
                Ok(res) => res,
                Err(msg) => return Err(format!("unable to read file {}", msg)),
            };

        let network = match specification_file.network.to_lowercase().as_str() {
            "simnet" => StacksNetwork::Simnet,
            "devnet" => StacksNetwork::Devnet,
            "testnet" => StacksNetwork::Testnet,
            "mainnet" => StacksNetwork::Mainnet,
            _ => {
                return Err(format!(
                    "network '{}' not supported (simnet, devnet, testnet, mainnet)",
                    specification_file.network
                ));
            }
        };

        let deployment_spec = DeploymentSpecification::from_specifications(
            &specification_file,
            &network,
            project_root_location,
        )?;

        Ok(deployment_spec)
    }

    pub fn from_specifications(
        specs: &DeploymentSpecificationFile,
        network: &StacksNetwork,
        project_root_location: &FileLocation,
    ) -> Result<DeploymentSpecification, String> {
        let mut contracts = BTreeMap::new();
        let (plan, genesis) = match network {
            StacksNetwork::Simnet => {
                let mut batches = vec![];
                let mut genesis = None;
                if let Some(ref plan) = specs.plan {
                    for batch in plan.batches.iter() {
                        let mut transactions = vec![];
                        for tx in batch.transactions.iter() {
                            let transaction = match tx {
                                TransactionSpecificationFile::EmulatedContractCall(spec) => {
                                    TransactionSpecification::EmulatedContractCall(EmulatedContractCallSpecification::from_specifications(spec)?)
                                }
                                TransactionSpecificationFile::EmulatedContractPublish(spec) => {
                                    let spec = EmulatedContractPublishSpecification::from_specifications(spec, project_root_location)?;
                                    let contract_id = QualifiedContractIdentifier::new(spec.emulated_sender.clone(), spec.contract_name.clone());
                                    contracts.insert(contract_id, (spec.source.clone(), spec.location.clone()));
                                    TransactionSpecification::EmulatedContractPublish(spec)
                                }
                                TransactionSpecificationFile::StxTransfer(spec) => {
                                    let spec = StxTransferSpecification::from_specifications(spec)?;
                                    TransactionSpecification::StxTransfer(spec)
                                }
                                TransactionSpecificationFile::BtcTransfer(_) | TransactionSpecificationFile::ContractCall(_) | TransactionSpecificationFile::ContractPublish(_) | TransactionSpecificationFile::RequirementPublish(_) => {
                                    return Err(format!("{} only supports transactions of type 'emulated-contract-call' and 'emulated-contract-publish", specs.network.to_lowercase()))
                                }
                            };
                            transactions.push(transaction);
                        }
                        batches.push(TransactionsBatchSpecification {
                            id: batch.id,
                            transactions,
                        });
                    }
                }
                if let Some(ref genesis_specs) = specs.genesis {
                    let genesis_specs = GenesisSpecification::from_specifications(genesis_specs)?;
                    genesis = Some(genesis_specs);
                }
                (TransactionPlanSpecification { batches }, genesis)
            }
            StacksNetwork::Devnet | StacksNetwork::Testnet | StacksNetwork::Mainnet => {
                let mut batches = vec![];
                if let Some(ref plan) = specs.plan {
                    for batch in plan.batches.iter() {
                        let mut transactions = vec![];
                        for tx in batch.transactions.iter() {
                            let transaction = match tx {
                                TransactionSpecificationFile::ContractCall(spec) => {
                                    TransactionSpecification::ContractCall(ContractCallSpecification::from_specifications(spec)?)
                                }
                                TransactionSpecificationFile::RequirementPublish(spec) => {
                                    if network.is_mainnet() {
                                        return Err(format!("{} only supports transactions of type 'contract-call' and 'contract-publish", specs.network.to_lowercase()))
                                    }
                                    let spec = RequirementPublishSpecification::from_specifications(spec, project_root_location)?;
                                    TransactionSpecification::RequirementPublish(spec)
                                }
                                TransactionSpecificationFile::ContractPublish(spec) => {
                                    let spec = ContractPublishSpecification::from_specifications(spec, project_root_location)?;
                                    let contract_id = QualifiedContractIdentifier::new(spec.expected_sender.clone(), spec.contract_name.clone());
                                    contracts.insert(contract_id, (spec.source.clone(), spec.location.clone()));
                                    TransactionSpecification::ContractPublish(spec)
                                }
                                TransactionSpecificationFile::BtcTransfer(spec) => {
                                    let spec = BtcTransferSpecification::from_specifications(spec)?;
                                    TransactionSpecification::BtcTransfer(spec)
                                }
                                TransactionSpecificationFile::StxTransfer(spec) => {
                                    let spec = StxTransferSpecification::from_specifications(spec)?;
                                    TransactionSpecification::StxTransfer(spec)
                                }
                                TransactionSpecificationFile::EmulatedContractCall(_) | TransactionSpecificationFile::EmulatedContractPublish(_) => {
                                    return Err(format!("{} only supports transactions of type 'contract-call' and 'contract-publish'", specs.network.to_lowercase()))
                                }
                            };
                            transactions.push(transaction);
                        }
                        batches.push(TransactionsBatchSpecification {
                            id: batch.id,
                            transactions,
                        });
                    }
                }
                (TransactionPlanSpecification { batches }, None)
            }
        };
        let stacks_node = match (&specs.stacks_node, &specs.node) {
            (Some(node), _) | (None, Some(node)) => Some(node.clone()),
            _ => None,
        };
        let bitcoin_node = match (&specs.bitcoin_node, &specs.node) {
            (Some(node), _) | (None, Some(node)) => Some(node.clone()),
            _ => None,
        };

        Ok(DeploymentSpecification {
            id: specs.id.unwrap_or(0),
            stacks_node,
            bitcoin_node,
            name: specs.name.to_string(),
            network: network.clone(),
            genesis,
            plan,
            contracts,
        })
    }

    pub fn to_specification_file(&self) -> DeploymentSpecificationFile {
        DeploymentSpecificationFile {
            id: Some(self.id),
            name: self.name.clone(),
            network: match self.network {
                StacksNetwork::Simnet => "simnet".to_string(),
                StacksNetwork::Devnet => "devnet".to_string(),
                StacksNetwork::Testnet => "testnet".to_string(),
                StacksNetwork::Mainnet => "mainnet".to_string(),
            },
            stacks_node: self.stacks_node.clone(),
            bitcoin_node: self.bitcoin_node.clone(),
            node: None,
            genesis: match self.genesis {
                Some(ref g) => Some(g.to_specification_file()),
                None => None,
            },
            plan: Some(self.plan.to_specification_file()),
        }
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]

pub struct DeploymentSpecificationFile {
    pub id: Option<u32>,
    pub name: String,
    pub network: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stacks_node: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitcoin_node: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genesis: Option<GenesisSpecificationFile>,
    pub plan: Option<TransactionPlanSpecificationFile>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GenesisSpecificationFile {
    pub wallets: Vec<WalletSpecificationFile>,
    pub contracts: Vec<String>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct WalletSpecificationFile {
    pub name: String,
    pub address: String,
    pub balance: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct GenesisSpecification {
    pub wallets: Vec<WalletSpecification>,
    pub contracts: Vec<String>,
}

impl GenesisSpecification {
    pub fn from_specifications(
        specs: &GenesisSpecificationFile,
    ) -> Result<GenesisSpecification, String> {
        let mut wallets = vec![];
        for wallet in specs.wallets.iter() {
            wallets.push(WalletSpecification::from_specifications(wallet)?);
        }

        Ok(GenesisSpecification {
            wallets,
            contracts: specs.contracts.clone(),
        })
    }

    pub fn to_specification_file(&self) -> GenesisSpecificationFile {
        let mut wallets = vec![];
        for wallet in self.wallets.iter() {
            wallets.push(WalletSpecificationFile {
                name: wallet.name.to_string(),
                address: wallet.address.to_string(),
                balance: format!("{}", wallet.balance),
            })
        }

        GenesisSpecificationFile {
            wallets,
            contracts: self.contracts.clone(),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct WalletSpecification {
    pub name: String,
    pub address: StandardPrincipalData,
    pub balance: u128,
}

impl WalletSpecification {
    pub fn from_specifications(
        specs: &WalletSpecificationFile,
    ) -> Result<WalletSpecification, String> {
        let address = match PrincipalData::parse_standard_principal(&specs.address) {
            Ok(res) => res,
            Err(_) => {
                return Err(format!(
                    "unable to parse {}'s principal as a valid Stacks address",
                    specs.name
                ))
            }
        };

        let balance = match u128::from_str_radix(&specs.balance, 10) {
            Ok(res) => res,
            Err(_) => {
                return Err(format!(
                    "unable to parse {}'s balance as a u128",
                    specs.name
                ))
            }
        };

        Ok(WalletSpecification {
            name: specs.name.to_string(),
            address,
            balance,
        })
    }
}

impl TransactionPlanSpecification {
    pub fn to_specification_file(&self) -> TransactionPlanSpecificationFile {
        let mut batches = vec![];
        for batch in self.batches.iter() {
            let mut transactions = vec![];
            for tx in batch.transactions.iter() {
                let tx = match tx {
                    TransactionSpecification::ContractCall(tx) => {
                        TransactionSpecificationFile::ContractCall(ContractCallSpecificationFile {
                            contract_id: tx.contract_id.to_string(),
                            expected_sender: tx.expected_sender.to_address(),
                            method: tx.method.to_string(),
                            parameters: tx.parameters.clone(),
                            cost: tx.cost,
                            anchor_block_only: Some(tx.anchor_block_only),
                        })
                    }
                    TransactionSpecification::ContractPublish(tx) => {
                        TransactionSpecificationFile::ContractPublish(
                            ContractPublishSpecificationFile {
                                contract_name: tx.contract_name.to_string(),
                                expected_sender: tx.expected_sender.to_address(),
                                location: Some(tx.location.clone()),
                                path: None,
                                url: None,
                                cost: tx.cost,
                                anchor_block_only: Some(tx.anchor_block_only),
                                clarity_version: match tx.clarity_version {
                                    ClarityVersion::Clarity1 => Some(1),
                                    ClarityVersion::Clarity2 => Some(2),
                                },
                            },
                        )
                    }
                    TransactionSpecification::EmulatedContractCall(tx) => {
                        TransactionSpecificationFile::EmulatedContractCall(
                            EmulatedContractCallSpecificationFile {
                                contract_id: tx.contract_id.to_string(),
                                emulated_sender: tx.emulated_sender.to_address(),
                                method: tx.method.to_string(),
                                parameters: tx.parameters.clone(),
                            },
                        )
                    }
                    TransactionSpecification::EmulatedContractPublish(tx) => {
                        TransactionSpecificationFile::EmulatedContractPublish(
                            EmulatedContractPublishSpecificationFile {
                                contract_name: tx.contract_name.to_string(),
                                emulated_sender: tx.emulated_sender.to_address(),
                                location: Some(tx.location.clone()),
                                path: None,
                                url: None,
                                clarity_version: match tx.clarity_version {
                                    ClarityVersion::Clarity1 => Some(1),
                                    ClarityVersion::Clarity2 => Some(2),
                                },
                            },
                        )
                    }
                    TransactionSpecification::RequirementPublish(tx) => {
                        let mut remap_principals = BTreeMap::new();
                        for (src, dst) in tx.remap_principals.iter() {
                            remap_principals.insert(src.to_address(), dst.to_address());
                        }
                        TransactionSpecificationFile::RequirementPublish(
                            RequirementPublishSpecificationFile {
                                contract_id: tx.contract_id.to_string(),
                                remap_sender: tx.remap_sender.to_address(),
                                remap_principals: Some(remap_principals),
                                location: Some(tx.location.clone()),
                                path: None,
                                url: None,
                                cost: tx.cost,
                                clarity_version: match tx.clarity_version {
                                    ClarityVersion::Clarity1 => Some(1),
                                    ClarityVersion::Clarity2 => Some(2),
                                },
                            },
                        )
                    }
                    TransactionSpecification::BtcTransfer(tx) => {
                        TransactionSpecificationFile::BtcTransfer(BtcTransferSpecificationFile {
                            expected_sender: tx.expected_sender.to_string(),
                            recipient: tx.recipient.clone(),
                            sats_amount: tx.sats_amount,
                            sats_per_byte: tx.sats_per_byte,
                        })
                    }
                    TransactionSpecification::StxTransfer(tx) => {
                        TransactionSpecificationFile::StxTransfer(StxTransferSpecificationFile {
                            expected_sender: tx.expected_sender.to_address(),
                            recipient: tx.recipient.to_string(),
                            mstx_amount: tx.mstx_amount,
                            memo: if tx.memo == [0; 34] {
                                None
                            } else {
                                Some(format!("0x{}", to_hex(&tx.memo)))
                            },
                            cost: tx.cost,
                            anchor_block_only: Some(tx.anchor_block_only),
                        })
                    }
                };
                transactions.push(tx);
            }

            batches.push(TransactionsBatchSpecificationFile {
                id: batch.id,
                transactions,
            });
        }

        TransactionPlanSpecificationFile { batches }
    }
}
