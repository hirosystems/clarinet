use clarity_repl::clarity::types::{
    PrincipalData, QualifiedContractIdentifier, StandardPrincipalData,
};

use clarity_repl::clarity::{ClarityName, ContractName};

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;

use std::fs;

use orchestra_types::StacksNetwork;

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
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RequirementPublishSpecificationFile {
    pub contract_id: String,
    pub remap_sender: String,
    pub remap_principals: Option<BTreeMap<String, String>>,
    pub path: String,
    pub cost: u64,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ContractPublishSpecificationFile {
    pub contract_name: String,
    pub expected_sender: String,
    pub path: String,
    pub cost: u64,
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
    pub path: String,
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
        })
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct ContractPublishSpecification {
    pub contract_name: ContractName,
    pub expected_sender: StandardPrincipalData,
    pub relative_path: String,
    pub source: String,
    pub cost: u64,
}

impl ContractPublishSpecification {
    pub fn from_specifications(
        specs: &ContractPublishSpecificationFile,
        base_path: &PathBuf,
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

        let path = match PathBuf::try_from(&specs.path) {
            Ok(res) => res,
            Err(_) => return Err(format!("unable to parse '{}' as a valid path", specs.path)),
        };

        let mut contract_path = base_path.clone();
        contract_path.push(path);

        let source = match fs::read_to_string(&contract_path) {
            Ok(code) => code,
            Err(err) => {
                return Err(format!(
                    "unable to read contract at path {:?}: {}",
                    contract_path, err
                ))
            }
        };

        Ok(ContractPublishSpecification {
            contract_name,
            expected_sender,
            source,
            relative_path: specs.path.clone(),
            cost: specs.cost,
        })
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct RequirementPublishSpecification {
    pub contract_id: QualifiedContractIdentifier,
    pub remap_sender: StandardPrincipalData,
    pub remap_principals: BTreeMap<StandardPrincipalData, StandardPrincipalData>,
    pub relative_path: String,
    pub source: String,
    pub cost: u64,
}

impl RequirementPublishSpecification {
    pub fn from_specifications(
        specs: &RequirementPublishSpecificationFile,
        base_path: &PathBuf,
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

        let path = match PathBuf::try_from(&specs.path) {
            Ok(res) => res,
            Err(_) => return Err(format!("unable to parse '{}' as a valid path", specs.path)),
        };

        let mut contract_path = base_path.clone();
        contract_path.push(path);

        let source = match fs::read_to_string(&contract_path) {
            Ok(code) => code,
            Err(err) => {
                return Err(format!(
                    "unable to read contract at path {:?}: {}",
                    contract_path, err
                ))
            }
        };

        Ok(RequirementPublishSpecification {
            contract_id,
            remap_sender,
            remap_principals,
            source,
            relative_path: specs.path.clone(),
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
    pub relative_path: String,
}

impl EmulatedContractPublishSpecification {
    pub fn from_specifications(
        specs: &EmulatedContractPublishSpecificationFile,
        base_path: &PathBuf,
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

        let path = match PathBuf::try_from(&specs.path) {
            Ok(res) => res,
            Err(_) => return Err(format!("unable to parse '{}' as a valid path", specs.path)),
        };

        let mut contract_path = base_path.clone();
        contract_path.push(path);

        let source = match fs::read_to_string(&contract_path) {
            Ok(code) => code,
            Err(err) => {
                return Err(format!(
                    "unable to read contract at path {:?}: {}",
                    contract_path, err
                ))
            }
        };

        Ok(EmulatedContractPublishSpecification {
            contract_name,
            emulated_sender,
            source,
            relative_path: specs.path.clone(),
        })
    }
}

pub struct DeploymentSynthesis {
    pub blocks_count: u64,
    pub total_cost: u64,
    pub content: String,
}

impl std::fmt::Display for DeploymentSynthesis {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let base: u64 = 10;
        let int_part = self.total_cost / base.pow(6);
        let frac_part = self.total_cost % base.pow(6);
        let formatted_total_cost = format!("{}.{:08}", int_part, frac_part);
        write!(
            f,
            "{}\n\n{}\n{}",
            green!(format!("{}", self.content)),
            blue!(format!("Total cost:\t{} STX", formatted_total_cost)),
            blue!(format!("Duration:\t{} blocks", self.blocks_count))
        )
    }
}

#[derive(Debug, Clone)]
pub struct DeploymentSpecification {
    pub id: u32,
    pub name: String,
    pub network: StacksNetwork,
    pub node: Option<String>,
    pub genesis: Option<GenesisSpecification>,
    pub plan: TransactionPlanSpecification,
    // Keep a cache of contract's (source, relative_path)
    pub contracts: BTreeMap<QualifiedContractIdentifier, (String, String)>,
}

impl DeploymentSpecification {
    pub fn from_config_file(
        path: &PathBuf,
        base_path: &PathBuf,
    ) -> Result<DeploymentSpecification, String> {
        let path = match File::open(path) {
            Ok(path) => path,
            Err(_e) => {
                panic!("unable to locate {}", path.display());
            }
        };
        let mut spec_file_reader = BufReader::new(path);
        let mut spec_file_buffer = vec![];
        spec_file_reader.read_to_end(&mut spec_file_buffer).unwrap();

        let specification_file: DeploymentSpecificationFile =
            match serde_yaml::from_slice(&spec_file_buffer[..]) {
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

        let deployment_spec =
            DeploymentSpecification::from_specifications(&specification_file, &network, base_path)?;

        Ok(deployment_spec)
    }

    pub fn from_specifications(
        specs: &DeploymentSpecificationFile,
        network: &StacksNetwork,
        base_path: &PathBuf,
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
                                    let spec = EmulatedContractPublishSpecification::from_specifications(spec, base_path)?;
                                    let contract_id = QualifiedContractIdentifier::new(spec.emulated_sender.clone(), spec.contract_name.clone());
                                    contracts.insert(contract_id, (spec.source.clone(), spec.relative_path.clone()));
                                    TransactionSpecification::EmulatedContractPublish(spec)
                                }
                                _ => {
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
                                    let spec = RequirementPublishSpecification::from_specifications(spec, base_path)?;
                                    TransactionSpecification::RequirementPublish(spec)
                                }
                                TransactionSpecificationFile::ContractPublish(spec) => {
                                    let spec = ContractPublishSpecification::from_specifications(spec, base_path)?;
                                    let contract_id = QualifiedContractIdentifier::new(spec.expected_sender.clone(), spec.contract_name.clone());
                                    contracts.insert(contract_id, (spec.source.clone(), spec.relative_path.clone()));
                                    TransactionSpecification::ContractPublish(spec)
                                }
                                TransactionSpecificationFile::BtcTransfer(spec) => {
                                    let spec = BtcTransferSpecification::from_specifications(spec)?;
                                    TransactionSpecification::BtcTransfer(spec)
                                }
                                _ => {
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
        Ok(DeploymentSpecification {
            id: specs.id.unwrap_or(0),
            node: specs.node.clone(),
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
            node: self.node.clone(),
            genesis: match self.genesis {
                Some(ref g) => Some(g.to_specification_file()),
                None => None,
            },
            plan: Some(self.plan.to_specification_file()),
        }
    }

    pub fn get_synthesis(&self) -> DeploymentSynthesis {
        let mut blocks_count = 0;
        let mut total_cost = 0;
        for batch in self.plan.batches.iter() {
            blocks_count += 1;
            for tx in batch.transactions.iter() {
                match tx {
                    TransactionSpecification::ContractCall(tx) => {
                        total_cost += tx.cost;
                    }
                    TransactionSpecification::ContractPublish(tx) => {
                        total_cost += tx.cost;
                    }
                    _ => {}
                }
            }
        }

        let file = self.to_specification_file();
        let content = match serde_yaml::to_string(&file) {
            Ok(res) => res,
            Err(err) => panic!("unable to serialize deployment {}", err),
        };

        return DeploymentSynthesis {
            total_cost,
            blocks_count,
            content,
        };
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]

pub struct DeploymentSpecificationFile {
    pub id: Option<u32>,
    pub name: String,
    pub network: String,
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
                        })
                    }
                    TransactionSpecification::ContractPublish(tx) => {
                        TransactionSpecificationFile::ContractPublish(
                            ContractPublishSpecificationFile {
                                contract_name: tx.contract_name.to_string(),
                                expected_sender: tx.expected_sender.to_address(),
                                path: tx.relative_path.clone(),
                                cost: tx.cost,
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
                                path: tx.relative_path.clone(),
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
                                path: tx.relative_path.clone(),
                                cost: tx.cost,
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
