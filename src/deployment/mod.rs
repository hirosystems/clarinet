mod requirements;
pub mod types;
mod ui;

use clarity_repl::clarity::types::StandardPrincipalData;
use clarity_repl::clarity::{ClarityName, Value};
pub use ui::start_ui;

use self::types::{
    DeploymentSpecification, EmulatedContractPublishSpecification, GenesisSpecification,
    TransactionPlanSpecification, TransactionsBatchSpecification, WalletSpecification,
};
use crate::deployment::types::ContractPublishSpecification;
use crate::deployment::types::RequirementPublishSpecification;
use crate::deployment::types::TransactionSpecification;

use crate::types::{AccountConfig, ChainConfig, ProjectManifest, StacksNetwork};
use crate::utils::mnemonic;
use crate::utils::stacks::StacksRpc;
use clarity_repl::analysis::ast_dependency_detector::{ASTDependencyDetector, DependencySet};
use clarity_repl::clarity::analysis::ContractAnalysis;
use clarity_repl::clarity::ast::ContractAST;
use clarity_repl::clarity::codec::transaction::{
    StacksTransaction, StacksTransactionSigner, TransactionAnchorMode, TransactionAuth,
    TransactionContractCall, TransactionPayload, TransactionPostConditionMode,
    TransactionPublicKeyEncoding, TransactionSmartContract, TransactionSpendingCondition,
};
use clarity_repl::clarity::codec::StacksMessageCodec;
use clarity_repl::clarity::diagnostic::Diagnostic;
use clarity_repl::clarity::types::{PrincipalData, QualifiedContractIdentifier};
use clarity_repl::clarity::util::{
    C32_ADDRESS_VERSION_MAINNET_SINGLESIG, C32_ADDRESS_VERSION_TESTNET_SINGLESIG,
};
use clarity_repl::clarity::ContractName;
use clarity_repl::clarity::{
    codec::{
        transaction::{
            RecoverableSignature, SinglesigHashMode, SinglesigSpendingCondition, TransactionVersion,
        },
        StacksString,
    },
    util::{
        address::AddressHashMode,
        secp256k1::{Secp256k1PrivateKey, Secp256k1PublicKey},
        StacksAddress,
    },
};
use clarity_repl::repl::SessionSettings;
use clarity_repl::repl::{ExecutionResult, Session};
use libsecp256k1::{PublicKey, SecretKey};
use serde_yaml;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::mpsc::{Receiver, Sender};
use tiny_hderive::bip32::ExtendedPrivKey;

#[derive(Deserialize, Debug)]
pub struct Balance {
    pub balance: String,
    pub nonce: u64,
    pub balance_proof: String,
    pub nonce_proof: String,
}

pub struct DeploymentGenerationArtifacts {
    pub asts: HashMap<QualifiedContractIdentifier, ContractAST>,
    pub deps: HashMap<QualifiedContractIdentifier, DependencySet>,
    pub diags: HashMap<QualifiedContractIdentifier, Vec<Diagnostic>>,
    pub analysis: HashMap<QualifiedContractIdentifier, ContractAnalysis>,
    pub session: Session,
    pub success: bool,
}

fn get_keypair(account: &AccountConfig) -> (ExtendedPrivKey, Secp256k1PrivateKey, PublicKey) {
    let bip39_seed = match mnemonic::get_bip39_seed_from_mnemonic(&account.mnemonic, "") {
        Ok(bip39_seed) => bip39_seed,
        Err(_) => panic!(),
    };
    let ext = ExtendedPrivKey::derive(&bip39_seed[..], account.derivation.as_str()).unwrap();
    let wrapped_secret_key = Secp256k1PrivateKey::from_slice(&ext.secret()).unwrap();
    let secret_key = SecretKey::parse_slice(&ext.secret()).unwrap();
    let public_key = PublicKey::from_secret_key(&secret_key);
    (ext, wrapped_secret_key, public_key)
}

fn get_stacks_address(public_key: &PublicKey, network: &StacksNetwork) -> StacksAddress {
    let wrapped_public_key =
        Secp256k1PublicKey::from_slice(&public_key.serialize_compressed()).unwrap();

    let signer_addr = StacksAddress::from_public_keys(
        match network {
            StacksNetwork::Mainnet => C32_ADDRESS_VERSION_MAINNET_SINGLESIG,
            _ => C32_ADDRESS_VERSION_TESTNET_SINGLESIG,
        },
        &AddressHashMode::SerializeP2PKH,
        1,
        &vec![wrapped_public_key],
    )
    .unwrap();

    signer_addr
}

fn sign_transaction_payload(
    account: &AccountConfig,
    payload: TransactionPayload,
    nonce: u64,
    tx_fee: u64,
    network: &StacksNetwork,
) -> Result<StacksTransaction, String> {
    let (_, secret_key, public_key) = get_keypair(account);
    let signer_addr = get_stacks_address(&public_key, network);

    let anchor_mode = TransactionAnchorMode::OnChainOnly;

    let spending_condition = TransactionSpendingCondition::Singlesig(SinglesigSpendingCondition {
        signer: signer_addr.bytes.clone(),
        nonce: nonce,
        tx_fee: tx_fee,
        hash_mode: SinglesigHashMode::P2PKH,
        key_encoding: TransactionPublicKeyEncoding::Compressed,
        signature: RecoverableSignature::empty(),
    });

    let auth = TransactionAuth::Standard(spending_condition);
    let unsigned_tx = StacksTransaction {
        version: match network {
            StacksNetwork::Mainnet => TransactionVersion::Mainnet,
            _ => TransactionVersion::Testnet,
        },
        chain_id: match network {
            StacksNetwork::Mainnet => 0x00000001,
            _ => 0x80000000,
        },
        auth: auth,
        anchor_mode: anchor_mode,
        post_condition_mode: TransactionPostConditionMode::Allow,
        post_conditions: vec![],
        payload: payload,
    };

    let mut unsigned_tx_bytes = vec![];
    unsigned_tx
        .consensus_serialize(&mut unsigned_tx_bytes)
        .expect("FATAL: invalid transaction");

    let mut tx_signer = StacksTransactionSigner::new(&unsigned_tx);
    tx_signer.sign_origin(&secret_key).unwrap();
    let signed_tx = tx_signer.get_tx().unwrap();
    Ok(signed_tx)
}

#[allow(dead_code)]
pub fn encode_contract_call(
    contract_id: &QualifiedContractIdentifier,
    function_name: ClarityName,
    function_args: Vec<Value>,
    account: &AccountConfig,
    nonce: u64,
    tx_fee: u64,
    network: &StacksNetwork,
) -> Result<StacksTransaction, String> {
    let (_, _, public_key) = get_keypair(account);
    let signer_addr = get_stacks_address(&public_key, network);

    let payload = TransactionContractCall {
        contract_name: contract_id.name.clone(),
        address: signer_addr,
        function_name: function_name.clone(),
        function_args: function_args.clone(),
    };
    sign_transaction_payload(
        account,
        TransactionPayload::ContractCall(payload),
        nonce,
        tx_fee,
        network,
    )
}

pub fn encode_contract_publish(
    contract_name: &ContractName,
    source: &str,
    account: &AccountConfig,
    nonce: u64,
    tx_fee: u64,
    network: &StacksNetwork,
) -> Result<StacksTransaction, String> {
    let payload = TransactionSmartContract {
        name: contract_name.clone(),
        code_body: StacksString::from_str(source).unwrap(),
    };
    sign_transaction_payload(
        account,
        TransactionPayload::SmartContract(payload),
        nonce,
        tx_fee,
        network,
    )
}

pub fn setup_session_with_deployment(
    manifest: &ProjectManifest,
    deployment: &DeploymentSpecification,
    contracts_asts: Option<&HashMap<QualifiedContractIdentifier, ContractAST>>,
) -> DeploymentGenerationArtifacts {
    let mut session = initiate_session_from_deployment(&manifest);
    update_session_with_genesis_accounts(&mut session, deployment);
    let results =
        update_session_with_contracts_executions(&mut session, deployment, contracts_asts, false);

    let deps = HashMap::new();
    let mut diags = HashMap::new();
    let mut asts = HashMap::new();
    let mut contracts_analysis = HashMap::new();
    let mut success = true;
    for (contract_id, res) in results.into_iter() {
        match res {
            Ok(execution_result) => {
                diags.insert(contract_id.clone(), execution_result.diagnostics);
                if let Some((_, _, _, ast, analysis)) = execution_result.contract {
                    asts.insert(contract_id.clone(), ast);
                    contracts_analysis.insert(contract_id, analysis);
                }
            }
            Err(errors) => {
                success = false;
                diags.insert(contract_id.clone(), errors);
            }
        }
    }

    let artifacts = DeploymentGenerationArtifacts {
        asts,
        deps,
        diags,
        success,
        session,
        analysis: contracts_analysis,
    };
    artifacts
}

pub fn initiate_session_from_deployment(manifest: &ProjectManifest) -> Session {
    let mut settings = SessionSettings::default();
    settings
        .include_boot_contracts
        .append(&mut manifest.project.boot_contracts.clone());
    settings.repl_settings = manifest.repl_settings.clone();
    settings.disk_cache_enabled = true;
    let session = Session::new(settings);
    session
}

pub fn update_session_with_genesis_accounts(
    session: &mut Session,
    deployment: &DeploymentSpecification,
) {
    if let Some(ref spec) = deployment.genesis {
        for wallet in spec.wallets.iter() {
            let _ = session.interpreter.mint_stx_balance(
                wallet.address.clone().into(),
                wallet.balance.try_into().unwrap(),
            );
            if wallet.name == "deployer" {
                session.set_tx_sender(wallet.address.to_address());
            }
        }
        session.load_boot_contracts();
    }
}

pub fn update_session_with_contracts_executions(
    session: &mut Session,
    deployment: &DeploymentSpecification,
    contracts_asts: Option<&HashMap<QualifiedContractIdentifier, ContractAST>>,
    code_coverage_enabled: bool,
) -> BTreeMap<QualifiedContractIdentifier, Result<ExecutionResult, Vec<Diagnostic>>> {
    let mut results = BTreeMap::new();
    // let mut remap_to_perform = vec![];
    for batch in deployment.plan.batches.iter() {
        for transaction in batch.transactions.iter() {
            match transaction {
                TransactionSpecification::RequirementPublish(_)
                | TransactionSpecification::ContractCall(_)
                | TransactionSpecification::ContractPublish(_) => {
                    panic!("requirement-publish, contract-call and contract-publish are the only operations admitted in simnet deployments")
                }
                TransactionSpecification::EmulatedContractPublish(tx) => {
                    let default_tx_sender = session.get_tx_sender();
                    session.set_tx_sender(tx.emulated_sender.to_string());

                    let contract_id = QualifiedContractIdentifier::new(
                        tx.emulated_sender.clone(),
                        tx.contract_name.clone(),
                    );
                    let contract_ast = contracts_asts.as_ref().and_then(|m| m.get(&contract_id));
                    let result = session.interpret(
                        tx.source.clone(),
                        Some(tx.contract_name.to_string()),
                        None,
                        false,
                        match code_coverage_enabled {
                            true => Some("__analysis__".to_string()),
                            false => None,
                        },
                        contract_ast,
                    );
                    results.insert(contract_id, result);
                    session.set_tx_sender(default_tx_sender);
                }
                TransactionSpecification::EmulatedContractCall(tx) => {
                    let _ = session.invoke_contract_call(
                        &tx.contract_id.to_string(),
                        &tx.method.to_string(),
                        &tx.parameters,
                        &tx.emulated_sender.to_string(),
                        "deployment".to_string(),
                    );
                }
            }
        }
        session.advance_chain_tip(1);
    }
    results
}

pub fn get_absolute_deployment_path(
    manifest: &ProjectManifest,
    relative_deployment_path: &str,
) -> PathBuf {
    let base_path = manifest.get_project_root_dir();
    let path = match PathBuf::from_str(relative_deployment_path) {
        Ok(path) => path,
        Err(_e) => {
            println!("unable to read path {}", relative_deployment_path);
            std::process::exit(1);
        }
    };
    base_path.join(path)
}

pub fn get_default_deployment_path(manifest: &ProjectManifest, network: &StacksNetwork) -> PathBuf {
    let mut deployment_path = manifest.get_project_root_dir();
    deployment_path.push("deployments");
    let file_path = match network {
        StacksNetwork::Simnet => "default.simnet-plan.yaml",
        StacksNetwork::Devnet => "default.devnet-plan.yaml",
        StacksNetwork::Testnet => "default.testnet-plan.yaml",
        StacksNetwork::Mainnet => "default.mainnet-plan.yaml",
    };
    deployment_path.push(file_path);
    deployment_path
}

pub fn read_deployment_or_generate_default(
    manifest: &ProjectManifest,
    network: &StacksNetwork,
) -> Result<
    (
        DeploymentSpecification,
        Option<DeploymentGenerationArtifacts>,
    ),
    String,
> {
    let default_deployment_file_path = get_default_deployment_path(&manifest, network);
    let (deployment, artifacts) = if default_deployment_file_path.exists() {
        (
            load_deployment(manifest, &default_deployment_file_path)?,
            None,
        )
    } else {
        let (deployment, artifacts) = generate_default_deployment(manifest, network, false)?;
        (deployment, Some(artifacts))
    };
    Ok((deployment, artifacts))
}

pub enum DeploymentEvent {
    TransactionUpdate(TransactionTracker),
    Interrupted(String),
    ProtocolDeployed,
}

pub enum DeploymentCommand {
    Start,
}

#[derive(Clone, Debug)]
pub enum TransactionStatus {
    Queued,
    Encoded(StacksTransaction, TransactionCheck),
    Broadcasted(TransactionCheck),
    Confirmed,
    Error(String),
}

#[derive(Clone, Debug)]
pub struct TransactionTracker {
    pub index: usize,
    pub name: String,
    pub status: TransactionStatus,
}

#[derive(Clone, Debug)]
pub enum TransactionCheck {
    ContractCall(StandardPrincipalData, u64),
    ContractPublish(StandardPrincipalData, ContractName),
}

pub fn get_initial_transactions_trackers(
    deployment: &DeploymentSpecification,
) -> Vec<TransactionTracker> {
    let mut index = 0;
    let mut trackers = vec![];
    for batch_spec in deployment.plan.batches.iter() {
        for transaction in batch_spec.transactions.iter() {
            let tracker = match transaction {
                TransactionSpecification::ContractCall(tx) => TransactionTracker {
                    index,
                    name: format!("Contract call {}::{}", tx.contract_id, tx.method),
                    status: TransactionStatus::Queued,
                },
                TransactionSpecification::ContractPublish(tx) => TransactionTracker {
                    index,
                    name: format!(
                        "Contract publish {}.{}",
                        tx.expected_sender.to_address(),
                        tx.contract_name
                    ),
                    status: TransactionStatus::Queued,
                },
                TransactionSpecification::RequirementPublish(tx) => {
                    if !deployment.network.either_devnet_or_tesnet() {
                        panic!("Deployment specification malformed - requirements publish not supported on mainnet");
                    }
                    TransactionTracker {
                        index,
                        name: format!(
                            "Contract publish {}.{}",
                            tx.remap_sender.to_address(),
                            tx.contract_id.name
                        ),
                        status: TransactionStatus::Queued,
                    }
                }
                TransactionSpecification::EmulatedContractPublish(_)
                | TransactionSpecification::EmulatedContractCall(_) => continue,
            };
            trackers.push(tracker);
            index += 1;
        }
    }
    trackers
}

pub fn apply_on_chain_deployment(
    manifest: &ProjectManifest,
    deployment: DeploymentSpecification,
    deployment_event_tx: Sender<DeploymentEvent>,
    deployment_command_rx: Receiver<DeploymentCommand>,
    fetch_initial_nonces: bool,
) {
    let chain_config = ChainConfig::from_manifest_path(&manifest.path, &deployment.network);
    let delay_between_checks: u64 = 10;
    // Load deployers, deployment_fee_rate
    // Check fee, balances and deployers

    let mut batches = VecDeque::new();
    let network = deployment.network.clone();
    let mut accounts_cached_nonces: BTreeMap<String, u64> = BTreeMap::new();
    let mut accounts_lookup: BTreeMap<String, &AccountConfig> = BTreeMap::new();

    if !fetch_initial_nonces {
        if network == StacksNetwork::Devnet {
            for (_, account) in chain_config.accounts.iter() {
                accounts_cached_nonces.insert(account.address.clone(), 0);
            }
        }
    }

    for (_, account) in chain_config.accounts.iter() {
        accounts_lookup.insert(account.address.clone(), account);
    }

    let node_url = deployment.node.expect("unable to get node rcp address");
    let stacks_rpc = StacksRpc::new(&node_url);

    // Phase 1: we traverse the deployment plan and encode all the transactions,
    // keeping the order.
    // Using a session to encode + coerce/check (todo) contract calls arguments.
    let mut session = Session::new(SessionSettings::default());
    let mut index = 0;
    let mut contracts_ids_to_remap: HashSet<(String, String)> = HashSet::new();
    for batch_spec in deployment.plan.batches.iter() {
        let mut batch = Vec::new();
        for transaction in batch_spec.transactions.iter() {
            let tracker = match transaction {
                TransactionSpecification::ContractCall(tx) => {
                    let issuer_address = tx.expected_sender.to_address();
                    let nonce = match accounts_cached_nonces.get(&issuer_address) {
                        Some(cached_nonce) => cached_nonce.clone(),
                        None => stacks_rpc
                            .get_nonce(&issuer_address)
                            .expect("Unable to retrieve account"),
                    };
                    let account = accounts_lookup.get(&issuer_address).unwrap();

                    let function_args = tx
                        .parameters
                        .iter()
                        .map(|value| {
                            let execution = session
                                .interpret(value.to_string(), None, None, false, None, None)
                                .unwrap();
                            execution.result.unwrap()
                        })
                        .collect::<Vec<_>>();

                    let transaction = match encode_contract_call(
                        &tx.contract_id,
                        tx.method.clone(),
                        function_args,
                        *account,
                        nonce,
                        tx.cost,
                        &network,
                    ) {
                        Ok(res) => res,
                        Err(e) => {
                            let _ = deployment_event_tx.send(DeploymentEvent::Interrupted(e));
                            return;
                        }
                    };

                    accounts_cached_nonces.insert(issuer_address.clone(), nonce + 1);
                    let name = format!(
                        "Call ({} {} {})",
                        tx.contract_id.to_string(),
                        tx.method,
                        tx.parameters.join(" ")
                    );
                    let check = TransactionCheck::ContractCall(tx.expected_sender.clone(), nonce);
                    TransactionTracker {
                        index,
                        name: name.clone(),
                        status: TransactionStatus::Encoded(transaction, check),
                    }
                }
                TransactionSpecification::ContractPublish(tx) => {
                    // Retrieve nonce for issuer
                    let issuer_address = tx.expected_sender.to_address();
                    let nonce = match accounts_cached_nonces.get(&issuer_address) {
                        Some(cached_nonce) => cached_nonce.clone(),
                        None => stacks_rpc
                            .get_nonce(&issuer_address)
                            .expect("Unable to retrieve account"),
                    };
                    let account = accounts_lookup.get(&issuer_address).unwrap();
                    let source = if deployment.network.either_devnet_or_tesnet() {
                        // Remapping - This is happening
                        let mut source = tx.source.clone();
                        for (old_contract_id, new_contract_id) in contracts_ids_to_remap.iter() {
                            let mut matched_indices = source
                                .match_indices(old_contract_id)
                                .map(|(i, _)| i)
                                .collect::<Vec<usize>>();
                            matched_indices.reverse();
                            for index in matched_indices {
                                source.replace_range(
                                    index..index + old_contract_id.len(),
                                    new_contract_id,
                                );
                            }
                        }
                        source
                    } else {
                        tx.source.clone()
                    };

                    let transaction = match encode_contract_publish(
                        &tx.contract_name,
                        &source,
                        *account,
                        nonce,
                        tx.cost,
                        &network,
                    ) {
                        Ok(res) => res,
                        Err(e) => {
                            let _ = deployment_event_tx.send(DeploymentEvent::Interrupted(e));
                            return;
                        }
                    };

                    accounts_cached_nonces.insert(issuer_address.clone(), nonce + 1);
                    let name = format!(
                        "Publish {}.{}",
                        tx.expected_sender.to_string(),
                        tx.contract_name
                    );
                    let check = TransactionCheck::ContractPublish(
                        tx.expected_sender.clone(),
                        tx.contract_name.clone(),
                    );
                    TransactionTracker {
                        index,
                        name: name.clone(),
                        status: TransactionStatus::Encoded(transaction, check),
                    }
                }
                TransactionSpecification::RequirementPublish(tx) => {
                    if deployment.network.is_mainnet() {
                        panic!("Deployment specification malformed - requirements publish not supported on mainnet");
                    }
                    let old_contract_id = tx.contract_id.to_string();
                    let new_contract_id = QualifiedContractIdentifier::new(
                        tx.remap_sender.clone(),
                        tx.contract_id.name.clone(),
                    )
                    .to_string();
                    contracts_ids_to_remap.insert((old_contract_id, new_contract_id));

                    // Retrieve nonce for issuer
                    let issuer_address = tx.remap_sender.to_address();
                    let nonce = match accounts_cached_nonces.get(&issuer_address) {
                        Some(cached_nonce) => cached_nonce.clone(),
                        None => stacks_rpc
                            .get_nonce(&issuer_address)
                            .expect("Unable to retrieve account"),
                    };
                    let account = accounts_lookup.get(&issuer_address).unwrap();

                    let transaction = match encode_contract_publish(
                        &tx.contract_id.name,
                        &tx.source,
                        *account,
                        nonce,
                        tx.cost,
                        &network,
                    ) {
                        Ok(res) => res,
                        Err(e) => {
                            let _ = deployment_event_tx.send(DeploymentEvent::Interrupted(e));
                            return;
                        }
                    };

                    accounts_cached_nonces.insert(issuer_address.clone(), nonce + 1);
                    let name = format!(
                        "Publish {}.{}",
                        tx.remap_sender.to_string(),
                        tx.contract_id.name
                    );
                    let check = TransactionCheck::ContractPublish(
                        tx.remap_sender.clone(),
                        tx.contract_id.name.clone(),
                    );
                    TransactionTracker {
                        index,
                        name: name.clone(),
                        status: TransactionStatus::Encoded(transaction, check),
                    }
                }
                TransactionSpecification::EmulatedContractPublish(_)
                | TransactionSpecification::EmulatedContractCall(_) => continue,
            };

            batch.push(tracker.clone());
            let _ = deployment_event_tx.send(DeploymentEvent::TransactionUpdate(tracker));
            index += 1;
        }

        batches.push_back(batch);
    }

    let _cmd = match deployment_command_rx.recv() {
        Ok(cmd) => cmd,
        Err(_) => {
            let _ = deployment_event_tx.send(DeploymentEvent::Interrupted(
                "deployment aborted - broken channel".to_string(),
            ));
            return;
        }
    };

    // Phase 2: we submit all the transactions previously encoded,
    // and wait for their inclusion in a block before moving to the next batch.
    let mut current_block_height = 0;
    for batch in batches.into_iter() {
        let mut ongoing_batch = BTreeMap::new();
        for mut tracker in batch.into_iter() {
            let (transaction, check) = match tracker.status {
                TransactionStatus::Encoded(transaction, check) => (transaction, check),
                _ => unreachable!(),
            };
            let _ = match stacks_rpc.post_transaction(&transaction) {
                Ok(res) => {
                    tracker.status = TransactionStatus::Broadcasted(check);

                    let _ = deployment_event_tx
                        .send(DeploymentEvent::TransactionUpdate(tracker.clone()));
                    ongoing_batch.insert(res.txid, tracker);
                }
                Err(e) => {
                    let message = format!("{:?}", e);
                    tracker.status = TransactionStatus::Error(message.clone());

                    let _ = deployment_event_tx
                        .send(DeploymentEvent::TransactionUpdate(tracker.clone()));
                    let _ = deployment_event_tx.send(DeploymentEvent::Interrupted(message));
                    return;
                }
            };
        }

        loop {
            let new_block_height = match stacks_rpc.get_info() {
                Ok(info) => info.burn_block_height,
                _ => {
                    std::thread::sleep(std::time::Duration::from_secs(delay_between_checks.into()));
                    continue;
                }
            };

            // If no block has been mined since `delay_between_checks`,
            // avoid flooding the stacks-node with status update requests.
            if new_block_height <= current_block_height {
                std::thread::sleep(std::time::Duration::from_secs(delay_between_checks.into()));
                continue;
            }

            current_block_height = new_block_height;

            let mut keep_looping = false;

            for (_txid, tracker) in ongoing_batch.iter_mut() {
                match &tracker.status {
                    TransactionStatus::Broadcasted(TransactionCheck::ContractPublish(
                        deployer,
                        contract_name,
                    )) => {
                        let deployer_address = deployer.to_address();
                        let res = stacks_rpc.get_contract_source(&deployer_address, &contract_name);
                        if let Ok(_contract) = res {
                            tracker.status = TransactionStatus::Confirmed;
                            let _ = deployment_event_tx
                                .send(DeploymentEvent::TransactionUpdate(tracker.clone()));
                        } else {
                            keep_looping = true;
                            break;
                        }
                    }
                    TransactionStatus::Broadcasted(TransactionCheck::ContractCall(
                        tx_sender,
                        expected_nonce,
                    )) => {
                        let tx_sender_address = tx_sender.to_address();
                        let res = stacks_rpc.get_nonce(&tx_sender_address);
                        if let Ok(current_nonce) = res {
                            if current_nonce > *expected_nonce {
                                tracker.status = TransactionStatus::Confirmed;
                                let _ = deployment_event_tx
                                    .send(DeploymentEvent::TransactionUpdate(tracker.clone()));
                            } else {
                                keep_looping = true;
                                break;
                            }
                        }
                    }
                    _ => {}
                }
            }
            if !keep_looping {
                break;
            }
        }
    }

    let _ = deployment_event_tx.send(DeploymentEvent::ProtocolDeployed);
}

pub fn check_deployments(manifest: &ProjectManifest) -> Result<(), String> {
    let base_path = manifest.get_project_root_dir();
    let files = get_deployments_files(manifest)?;
    for (path, relative_path) in files.into_iter() {
        let _spec = match DeploymentSpecification::from_config_file(&path, &base_path) {
            Ok(spec) => spec,
            Err(msg) => {
                println!("{} {} syntax incorrect\n{}", red!("x"), relative_path, msg);
                continue;
            }
        };
        println!("{} {} succesfully checked", green!("✔"), relative_path);
    }
    Ok(())
}

pub fn load_deployment_if_exists(
    manifest: &ProjectManifest,
    network: &StacksNetwork,
) -> Option<Result<DeploymentSpecification, String>> {
    let default_deployment_path = get_default_deployment_path(manifest, network);
    if !default_deployment_path.exists() {
        return None;
    }
    Some(load_deployment(manifest, &default_deployment_path))
}

pub fn load_deployment(
    manifest: &ProjectManifest,
    deployment_plan_path: &PathBuf,
) -> Result<DeploymentSpecification, String> {
    let base_path = manifest.get_project_root_dir();
    let spec = match DeploymentSpecification::from_config_file(&deployment_plan_path, &base_path) {
        Ok(spec) => spec,
        Err(msg) => {
            return Err(format!(
                "{} {} syntax incorrect\n{}",
                red!("x"),
                deployment_plan_path.display(),
                msg
            ));
        }
    };
    Ok(spec)
}

fn get_deployments_files(manifest: &ProjectManifest) -> Result<Vec<(PathBuf, String)>, String> {
    let mut project_dir = manifest.get_project_root_dir();
    let suffix_len = project_dir.to_str().unwrap().len() + 1;
    project_dir.push("deployments");
    let paths = match fs::read_dir(&project_dir) {
        Ok(paths) => paths,
        Err(_) => return Ok(vec![]),
    };
    let mut plans_paths = vec![];
    for path in paths {
        let file = path.unwrap().path();
        let is_extension_valid = file
            .extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| Some(ext == "yml" || ext == "yaml"));

        if let Some(true) = is_extension_valid {
            let relative_path = file.clone();
            let (_, relative_path) = relative_path.to_str().unwrap().split_at(suffix_len);
            plans_paths.push((file, relative_path.to_string()));
        }
    }

    Ok(plans_paths)
}

pub fn write_deployment(
    deployment: &DeploymentSpecification,
    target_path: &PathBuf,
    prompt_override: bool,
) -> Result<(), String> {
    if target_path.exists() && prompt_override {
        println!(
            "Deployment {} already exists.\n{}?",
            target_path.display(),
            yellow!("Overwrite [Y/n]")
        );
        let mut buffer = String::new();
        std::io::stdin().read_line(&mut buffer).unwrap();
        if buffer.starts_with("n") {
            return Err(format!("deployment update aborted"));
        }
    } else {
        let mut base_dir = target_path.clone();
        base_dir.pop();
        if !base_dir.exists() {
            if let Err(e) = std::fs::create_dir(&base_dir) {
                return Err(format!(
                    "unable to create directory {}: {:?}",
                    base_dir.display(),
                    e
                ));
            }
        }
    }

    let file = deployment.to_specification_file();

    let content = match serde_yaml::to_string(&file) {
        Ok(res) => res,
        Err(err) => return Err(format!("unable to serialize deployment {}", err)),
    };

    let mut file = match File::create(&target_path) {
        Ok(file) => file,
        Err(e) => {
            return Err(format!(
                "unable to create file {}: {}",
                target_path.display(),
                e
            ));
        }
    };
    match file.write_all(content.as_bytes()) {
        Ok(_) => (),
        Err(e) => {
            return Err(format!(
                "unable to write file {}: {}",
                target_path.display(),
                e
            ));
        }
    };
    Ok(())
}

pub fn generate_default_deployment(
    manifest: &ProjectManifest,
    network: &StacksNetwork,
    no_batch: bool,
) -> Result<(DeploymentSpecification, DeploymentGenerationArtifacts), String> {
    let chain_config = ChainConfig::from_manifest_path(&manifest.path, &network);

    let node = match network {
        StacksNetwork::Simnet => None,
        StacksNetwork::Devnet => Some(
            chain_config
                .network
                .node_rpc_address
                .unwrap_or("http://localhost:20443".to_string()),
        ),
        StacksNetwork::Testnet => Some(
            chain_config
                .network
                .node_rpc_address
                .unwrap_or("http://stacks-node-api.testnet.stacks.co".to_string()),
        ),
        StacksNetwork::Mainnet => Some(
            chain_config
                .network
                .node_rpc_address
                .unwrap_or("http://stacks-node-api.mainnet.stacks.co".to_string()),
        ),
    };

    let deployment_fee_rate = chain_config.network.deployment_fee_rate;

    let default_deployer = match chain_config.accounts.get("deployer") {
        Some(deployer) => deployer,
        None => {
            return Err(format!(
                "{} unable to retrieve default deployer account",
                red!("x")
            ));
        }
    };
    let default_deployer_address =
        match PrincipalData::parse_standard_principal(&default_deployer.address) {
            Ok(res) => res,
            Err(_) => {
                return Err(format!(
                    "unable to turn address {} as a valid Stacks address",
                    default_deployer.address
                ))
            }
        };

    let mut transactions = vec![];
    let mut contracts_map = BTreeMap::new();
    let mut requirements_asts = BTreeMap::new();
    let mut requirements_deps = HashMap::new();

    let parser_version = manifest.repl_settings.parser_version;

    let mut settings = SessionSettings::default();
    settings.include_boot_contracts = manifest.project.boot_contracts.clone();
    settings.repl_settings = manifest.repl_settings.clone();

    let session = Session::new(settings.clone());
    let mut boot_contracts_asts = session.get_boot_contracts_asts();
    let boot_contracts_ids = boot_contracts_asts
        .iter()
        .map(|(k, _)| k.clone())
        .collect::<Vec<QualifiedContractIdentifier>>();
    requirements_asts.append(&mut boot_contracts_asts);

    // Build the ASTs / DependencySet for requirements - step required for Simnet/Devnet/Testnet/Mainnet
    if let Some(ref requirements) = manifest.project.requirements {
        let default_cache_path = match PathBuf::from_str(&manifest.project.cache_dir) {
            Ok(path) => path,
            Err(_) => return Err("unable to get default cache path".to_string()),
        };
        let mut emulated_contracts_publish = HashMap::new();
        let mut requirements_publish = HashMap::new();

        // Load all the requirements
        // Some requirements are explicitly listed, some are discovered as we compute the ASTs.
        let mut queue = VecDeque::new();

        for requirement in requirements.iter() {
            let contract_id = match QualifiedContractIdentifier::parse(&requirement.contract_id) {
                Ok(contract_id) => contract_id,
                Err(_e) => {
                    return Err(format!(
                        "malformatted contract_id: {}",
                        requirement.contract_id
                    ))
                }
            };
            queue.push_front(contract_id);
        }

        while let Some(contract_id) = queue.pop_front() {
            // Extract principal from contract_id
            if requirements_deps.contains_key(&contract_id) {
                continue;
            }

            // Did we already get the source in a prior cycle?
            let ast = match requirements_asts.remove(&contract_id) {
                Some(ast) => ast,
                None => {
                    // Download the code
                    let (source, path) = requirements::retrieve_contract(
                        &contract_id,
                        true,
                        Some(default_cache_path.clone()),
                    )?;

                    let path = if manifest.project.cache_dir_relative {
                        let manifest_dir = format!("{}", manifest.get_project_root_dir().display());
                        let absolute_path = format!("{}", path.display());
                        absolute_path[(manifest_dir.len() + 1)..].to_string()
                    } else {
                        format!("{}", path.display())
                    };

                    // Build the struct representing the requirement in the deployment
                    if network.is_simnet() {
                        let data = EmulatedContractPublishSpecification {
                            contract_name: contract_id.name.clone(),
                            emulated_sender: contract_id.issuer.clone(),
                            source: source.clone(),
                            relative_path: path,
                        };
                        emulated_contracts_publish.insert(contract_id.clone(), data);
                    } else if network.either_devnet_or_tesnet() {
                        let data = RequirementPublishSpecification {
                            contract_id: contract_id.clone(),
                            remap_sender: default_deployer_address.clone(),
                            source: source.clone(),
                            relative_path: path,
                            cost: deployment_fee_rate * source.len() as u64,
                        };
                        requirements_publish.insert(contract_id.clone(), data);
                    }

                    // Compute the AST
                    let (ast, _, _) = session.interpreter.build_ast(
                        contract_id.clone(),
                        source.to_string(),
                        parser_version,
                    );
                    ast
                }
            };

            // Detect the eventual dependencies for this AST
            let mut contract_ast = HashMap::new();
            contract_ast.insert(contract_id.clone(), ast);
            let dependencies =
                ASTDependencyDetector::detect_dependencies(&contract_ast, &requirements_asts);
            let ast = contract_ast
                .remove(&contract_id)
                .expect("unable to retrieve ast");

            // Extract the known / unknown dependencies
            match dependencies {
                Ok(inferable_dependencies) => {
                    // Looping could be confusing - in this case, we submitted a HashMap with one contract, so we have at most one
                    // result in the `inferable_dependencies` map. We will just extract and keep the associated data (source, ast, deps).
                    for (contract_id, dependencies) in inferable_dependencies.into_iter() {
                        for dependency in dependencies.iter() {
                            queue.push_back(dependency.contract_id.clone());
                        }
                        requirements_deps.insert(contract_id.clone(), dependencies);
                        requirements_asts.insert(contract_id.clone(), ast);
                        break;
                    }
                }
                Err((inferable_dependencies, non_inferable_dependencies)) => {
                    // In the case of unknown dependencies, we were unable to construct an exhaustive list of dependencies.
                    // As such, we will re-enqueue the present (front) and push all the unknown contract_ids in front of it,
                    // and we will keep the source in memory to avoid useless disk access.
                    for (_, dependencies) in inferable_dependencies.iter() {
                        for dependency in dependencies.iter() {
                            queue.push_back(dependency.contract_id.clone());
                        }
                    }
                    requirements_asts.insert(contract_id.clone(), ast);
                    queue.push_front(contract_id);

                    for non_inferable_contract_id in non_inferable_dependencies.into_iter() {
                        queue.push_front(non_inferable_contract_id);
                    }
                }
            };
        }

        // Avoid listing requirements as deployment transactions to the deployment specification on Devnet / Testnet / Mainnet
        if !network.is_mainnet() {
            let ordered_contracts_ids =
                match ASTDependencyDetector::order_contracts(&requirements_deps) {
                    Ok(ordered_contracts) => ordered_contracts,
                    Err(e) => return Err(format!("unable to order requirements {}", e)),
                };

            if network.is_simnet() {
                for contract_id in ordered_contracts_ids.iter() {
                    let data = emulated_contracts_publish
                        .remove(contract_id)
                        .expect("unable to retrieve contract");
                    let tx = TransactionSpecification::EmulatedContractPublish(data);
                    transactions.push(tx);
                }
            } else if network.either_devnet_or_tesnet() {
                for contract_id in ordered_contracts_ids.iter() {
                    let data = requirements_publish
                        .remove(contract_id)
                        .expect("unable to retrieve contract");
                    let tx = TransactionSpecification::RequirementPublish(data);
                    transactions.push(tx);
                }
            }
        }
    }

    let mut contracts = HashMap::new();
    let mut contracts_sources = HashMap::new();
    for (name, config) in manifest.contracts.iter() {
        let contract_name = match ContractName::try_from(name.to_string()) {
            Ok(res) => res,
            Err(_) => return Err(format!("unable to use {} as a valid contract name", name)),
        };

        let deployer = match config.deployer {
            Some(ref deployer) => {
                let deployer = match chain_config.accounts.get(deployer) {
                    Some(deployer) => deployer,
                    None => {
                        return Err(format!(
                            "{} unable to retrieve account '{}'",
                            red!("x"),
                            deployer
                        ));
                    }
                };
                deployer
            }
            None => default_deployer,
        };

        let sender = match PrincipalData::parse_standard_principal(&deployer.address) {
            Ok(res) => res,
            Err(_) => {
                return Err(format!(
                    "unable to turn emulated_sender {} as a valid Stacks address",
                    deployer.address
                ))
            }
        };

        let mut path = manifest.get_project_root_dir();
        path.push(&config.path);
        let source = match std::fs::read_to_string(&path) {
            Ok(code) => code,
            Err(err) => {
                return Err(format!(
                    "unable to read contract at path {:?}: {}",
                    config.path, err
                ))
            }
        };

        let contract_id = QualifiedContractIdentifier::new(sender.clone(), contract_name.clone());

        contracts_sources.insert(contract_id.clone(), source.clone());

        let contract_spec = if network.is_simnet() {
            TransactionSpecification::EmulatedContractPublish(
                EmulatedContractPublishSpecification {
                    contract_name,
                    emulated_sender: sender,
                    source,
                    relative_path: config.path.clone(),
                },
            )
        } else {
            TransactionSpecification::ContractPublish(ContractPublishSpecification {
                contract_name,
                expected_sender: sender,
                relative_path: config.path.clone(),
                cost: deployment_fee_rate
                    .saturating_mul(source.as_bytes().len().try_into().unwrap()),
                source,
            })
        };

        contracts.insert(contract_id, contract_spec);
    }

    let session = Session::new(settings);

    let mut contract_asts = HashMap::new();
    let mut contract_diags = HashMap::new();

    let mut asts_success = true;

    for (contract_id, source) in contracts_sources.into_iter() {
        let (ast, diags, ast_success) =
            session
                .interpreter
                .build_ast(contract_id.clone(), source, parser_version);
        contract_asts.insert(contract_id.clone(), ast);
        contract_diags.insert(contract_id, diags);
        asts_success = asts_success && ast_success;
    }

    let dependencies =
        ASTDependencyDetector::detect_dependencies(&contract_asts, &requirements_asts);

    let mut dependencies = match dependencies {
        Ok(dependencies) => dependencies,
        Err(_) => {
            return Err(format!("unable to detect dependencies"));
        }
    };

    for contract_id in boot_contracts_ids.into_iter() {
        dependencies.insert(contract_id.clone(), DependencySet::new());
    }

    dependencies.extend(requirements_deps);

    let ordered_contracts_ids = match ASTDependencyDetector::order_contracts(&dependencies) {
        Ok(ordered_contracts_ids) => ordered_contracts_ids,
        Err(e) => return Err(format!("unable to order contracts {}", e)),
    };

    for contract_id in ordered_contracts_ids.into_iter() {
        if requirements_asts.contains_key(&contract_id) {
            continue;
        }
        let tx = contracts
            .remove(&contract_id)
            .expect("unable to retrieve contract");

        match tx {
            TransactionSpecification::EmulatedContractPublish(ref data) => {
                contracts_map.insert(
                    contract_id.clone(),
                    (data.source.clone(), data.relative_path.clone()),
                );
            }
            TransactionSpecification::ContractPublish(ref data) => {
                contracts_map.insert(
                    contract_id.clone(),
                    (data.source.clone(), data.relative_path.clone()),
                );
            }
            _ => unreachable!(),
        }
        transactions.push(tx);
    }

    let tx_chain_limit = match no_batch {
        true => 100_000,
        false => 25,
    };

    let mut batches = vec![];
    for (id, transactions) in transactions.chunks(tx_chain_limit).enumerate() {
        batches.push(TransactionsBatchSpecification {
            id: id,
            transactions: transactions.to_vec(),
        })
    }

    let mut wallets = vec![];
    if network.is_simnet() {
        for (name, account) in chain_config.accounts.into_iter() {
            let address = match PrincipalData::parse_standard_principal(&account.address) {
                Ok(res) => res,
                Err(_) => {
                    return Err(format!(
                        "unable to parse wallet {} in a valid Stacks address",
                        account.address
                    ))
                }
            };

            wallets.push(WalletSpecification {
                name,
                address,
                balance: account.balance.into(),
            });
        }
    }

    let name = match network {
        StacksNetwork::Simnet => format!("Simulated deployment, used as a default for `clarinet console`, `clarinet test` and `clarinet check`"),
        _ => format!("{:?} deployment", network)
    };

    let deployment = DeploymentSpecification {
        id: 0,
        name,
        node,
        network: network.clone(),
        genesis: if network.is_simnet() {
            Some(GenesisSpecification {
                wallets,
                contracts: manifest.project.boot_contracts.clone(),
            })
        } else {
            None
        },
        plan: TransactionPlanSpecification { batches },
        contracts: contracts_map,
    };

    let artifacts = DeploymentGenerationArtifacts {
        asts: contract_asts,
        deps: dependencies,
        diags: contract_diags,
        success: asts_success,
        analysis: HashMap::new(),
        session,
    };

    Ok((deployment, artifacts))
}

pub fn display_deployment(_deployment: &DeploymentSpecification) {}
