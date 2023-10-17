use bitcoincore_rpc::{Auth, Client};
use clarinet_files::chainhook_types::StacksNetwork;
use clarinet_files::{AccountConfig, NetworkManifest};
use clarinet_utils::get_bip39_seed_from_mnemonic;
use clarity_repl::clarity::codec::StacksMessageCodec;
use clarity_repl::clarity::stacks_common::types::chainstate::StacksAddress;
use clarity_repl::clarity::util::secp256k1::{
    MessageSignature, Secp256k1PrivateKey, Secp256k1PublicKey,
};
use clarity_repl::clarity::vm::types::{
    PrincipalData, QualifiedContractIdentifier, StandardPrincipalData,
};
use clarity_repl::clarity::vm::{ClarityName, Value};
use clarity_repl::clarity::{ClarityVersion, ContractName, EvaluationResult};
use clarity_repl::codec::{
    SinglesigHashMode, SinglesigSpendingCondition, StacksString, StacksTransactionSigner,
    TokenTransferMemo, TransactionAuth, TransactionContractCall, TransactionPayload,
    TransactionPostConditionMode, TransactionPublicKeyEncoding, TransactionSmartContract,
    TransactionSpendingCondition, TransactionVersion,
};
use clarity_repl::codec::{StacksTransaction, TransactionAnchorMode};
use clarity_repl::repl::session::{
    BOOT_MAINNET_ADDRESS, BOOT_TESTNET_ADDRESS, V1_BOOT_CONTRACTS, V2_BOOT_CONTRACTS,
    V3_BOOT_CONTRACTS,
};
use clarity_repl::repl::{Session, SessionSettings};
use reqwest::Url;
use stacks_rpc_client::StacksRpc;
use std::collections::{BTreeMap, HashSet, VecDeque};
use std::sync::mpsc::{Receiver, Sender};
use tiny_hderive::bip32::ExtendedPrivKey;

use clarity_repl::clarity::address::{
    AddressHashMode, C32_ADDRESS_VERSION_MAINNET_SINGLESIG, C32_ADDRESS_VERSION_TESTNET_SINGLESIG,
};
use libsecp256k1::{PublicKey, SecretKey};

mod bitcoin_deployment;

use crate::types::{DeploymentSpecification, EpochSpec, TransactionSpecification};

fn get_btc_keypair(
    account: &AccountConfig,
) -> (
    bitcoincore_rpc::bitcoin::secp256k1::SecretKey,
    bitcoincore_rpc::bitcoin::secp256k1::PublicKey,
) {
    use bitcoincore_rpc::bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey};
    let bip39_seed = match get_bip39_seed_from_mnemonic(&account.mnemonic, "") {
        Ok(bip39_seed) => bip39_seed,
        Err(_) => panic!(),
    };
    let secp = Secp256k1::new();
    let ext = ExtendedPrivKey::derive(&bip39_seed[..], account.derivation.as_str()).unwrap();
    let secret_key = SecretKey::from_slice(&ext.secret()).unwrap();
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    (secret_key, public_key)
}

fn get_keypair(account: &AccountConfig) -> (ExtendedPrivKey, Secp256k1PrivateKey, PublicKey) {
    let bip39_seed = match get_bip39_seed_from_mnemonic(&account.mnemonic, "") {
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
    anchor_mode: TransactionAnchorMode,
    network: &StacksNetwork,
) -> Result<StacksTransaction, String> {
    let (_, secret_key, public_key) = get_keypair(account);
    let signer_addr = get_stacks_address(&public_key, network);

    let spending_condition = TransactionSpendingCondition::Singlesig(SinglesigSpendingCondition {
        signer: signer_addr.bytes.clone(),
        nonce: nonce,
        tx_fee: tx_fee,
        hash_mode: SinglesigHashMode::P2PKH,
        key_encoding: TransactionPublicKeyEncoding::Compressed,
        signature: MessageSignature::empty(),
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

pub fn encode_contract_call(
    contract_id: &QualifiedContractIdentifier,
    function_name: ClarityName,
    function_args: Vec<Value>,
    account: &AccountConfig,
    nonce: u64,
    tx_fee: u64,
    anchor_mode: TransactionAnchorMode,
    network: &StacksNetwork,
) -> Result<StacksTransaction, String> {
    let payload = TransactionContractCall {
        contract_name: contract_id.name.clone(),
        address: StacksAddress::from(contract_id.issuer.clone()),
        function_name: function_name.clone(),
        function_args: function_args.clone(),
    };
    sign_transaction_payload(
        account,
        TransactionPayload::ContractCall(payload),
        nonce,
        tx_fee,
        anchor_mode,
        network,
    )
}

pub fn encode_stx_transfer(
    recipient: PrincipalData,
    amount: u64,
    memo: [u8; 34],
    account: &AccountConfig,
    nonce: u64,
    tx_fee: u64,
    anchor_mode: TransactionAnchorMode,
    network: &StacksNetwork,
) -> Result<StacksTransaction, String> {
    let payload = TransactionPayload::TokenTransfer(recipient, amount, TokenTransferMemo(memo));
    sign_transaction_payload(account, payload, nonce, tx_fee, anchor_mode, network)
}

pub fn encode_contract_publish(
    contract_name: &ContractName,
    source: &str,
    clarity_version: Option<ClarityVersion>,
    account: &AccountConfig,
    nonce: u64,
    tx_fee: u64,
    anchor_mode: TransactionAnchorMode,
    network: &StacksNetwork,
) -> Result<StacksTransaction, String> {
    let payload = TransactionSmartContract {
        name: contract_name.clone(),
        code_body: StacksString::from_str(source).unwrap(),
    };
    sign_transaction_payload(
        account,
        TransactionPayload::SmartContract(payload, clarity_version),
        nonce,
        tx_fee,
        anchor_mode,
        network,
    )
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
    NonceCheck(StandardPrincipalData, u64),
    ContractPublish(StandardPrincipalData, ContractName),
    // TODO(lgalabru): Handle Bitcoin checks
    // BtcTransfer(),
}

#[derive(Clone, Debug)]
pub enum DeploymentEvent {
    TransactionUpdate(TransactionTracker),
    Interrupted(String),
    DeploymentCompleted,
}

pub enum DeploymentCommand {
    Start,
}

pub fn update_deployment_costs(
    deployment: &mut DeploymentSpecification,
    priority: usize,
) -> Result<(), String> {
    let stacks_node_url = deployment
        .stacks_node
        .as_ref()
        .expect("unable to get stacks node rcp address");
    let stacks_rpc = StacksRpc::new(&stacks_node_url);
    let mut session = Session::new(SessionSettings::default());

    for batch_spec in deployment.plan.batches.iter_mut() {
        for transaction in batch_spec.transactions.iter_mut() {
            match transaction {
                TransactionSpecification::StxTransfer(tx) => {
                    let transaction_payload = TransactionPayload::TokenTransfer(
                        tx.recipient.clone(),
                        tx.mstx_amount,
                        TokenTransferMemo(tx.memo.clone()),
                    );

                    match stacks_rpc.estimate_transaction_fee(&transaction_payload, priority) {
                        Ok(fee) => {
                            tx.cost = fee;
                        }
                        Err(e) => {
                            println!("unable to estimate fee for transaction: {}", e.to_string());
                            continue;
                        }
                    };
                }
                TransactionSpecification::ContractCall(tx) => {
                    let function_args = tx
                        .parameters
                        .iter()
                        .map(|value| {
                            let execution = session.eval(value.to_string(), None, false).unwrap();
                            match execution.result {
                                EvaluationResult::Snippet(result) => result.result,
                                _ => unreachable!("Contract result from snippet"),
                            }
                        })
                        .collect::<Vec<_>>();

                    let transaction_payload =
                        TransactionPayload::ContractCall(TransactionContractCall {
                            contract_name: tx.contract_id.name.clone(),
                            address: StacksAddress::from(tx.contract_id.issuer.clone()),
                            function_name: tx.method.clone(),
                            function_args: function_args,
                        });

                    match stacks_rpc.estimate_transaction_fee(&transaction_payload, priority) {
                        Ok(fee) => {
                            tx.cost = fee;
                        }
                        Err(e) => {
                            println!("unable to estimate fee for transaction: {}", e.to_string());
                            continue;
                        }
                    };
                }
                TransactionSpecification::ContractPublish(tx) => {
                    let transaction_payload = TransactionPayload::SmartContract(
                        TransactionSmartContract {
                            name: tx.contract_name.clone(),
                            code_body: StacksString::from_str(&tx.source).unwrap(),
                        },
                        None,
                    );

                    match stacks_rpc.estimate_transaction_fee(&transaction_payload, priority) {
                        Ok(fee) => {
                            tx.cost = fee;
                        }
                        Err(e) => {
                            println!("unable to estimate fee for transaction: {}", e.to_string());
                            continue;
                        }
                    };
                }
                TransactionSpecification::RequirementPublish(_)
                | TransactionSpecification::BtcTransfer(_)
                | TransactionSpecification::EmulatedContractPublish(_)
                | TransactionSpecification::EmulatedContractCall(_) => continue,
            };
        }
    }
    Ok(())
}

pub fn apply_on_chain_deployment(
    network_manifest: NetworkManifest,
    deployment: DeploymentSpecification,
    deployment_event_tx: Sender<DeploymentEvent>,
    deployment_command_rx: Receiver<DeploymentCommand>,
    fetch_initial_nonces: bool,
    override_bitcoin_rpc_url: Option<String>,
    override_stacks_rpc_url: Option<String>,
) {
    let network = deployment.network.get_networks();
    let delay_between_checks: u64 = if network.1.is_devnet() { 1 } else { 10 };
    // Load deployers, deployment_fee_rate
    // Check fee, balances and deployers

    let mut batches = VecDeque::new();
    let network = deployment.network.clone();
    let mut accounts_cached_nonces: BTreeMap<String, u64> = BTreeMap::new();
    let mut stx_accounts_lookup: BTreeMap<String, &AccountConfig> = BTreeMap::new();
    let mut btc_accounts_lookup: BTreeMap<String, &AccountConfig> = BTreeMap::new();
    let mut default_epoch = EpochSpec::Epoch2_05;
    if !fetch_initial_nonces {
        for (_, account) in network_manifest.accounts.iter() {
            accounts_cached_nonces.insert(account.stx_address.clone(), 0);
        }
        if network_manifest.devnet.is_some() {
            default_epoch = EpochSpec::Epoch2_1;
        };
    }

    for (_, account) in network_manifest.accounts.iter() {
        stx_accounts_lookup.insert(account.stx_address.clone(), account);
        btc_accounts_lookup.insert(account.btc_address.clone(), account);
    }

    let stacks_node_url = if let Some(url) = override_stacks_rpc_url {
        url
    } else {
        deployment
            .stacks_node
            .expect("unable to get stacks node rcp address")
    };

    let stacks_rpc = StacksRpc::new(&stacks_node_url);

    let bitcoin_node_url = if let Some(url) = override_bitcoin_rpc_url {
        url
    } else {
        deployment
            .bitcoin_node
            .expect("unable to get bitcoin node rcp address")
    };

    // Phase 1: we traverse the deployment plan and encode all the transactions,
    // keeping the order.
    // Using a session to encode + coerce/check (todo) contract calls arguments.
    let mut session = Session::new(SessionSettings::default());
    let mut index = 0;
    let mut contracts_ids_to_remap: HashSet<(String, String)> = HashSet::new();

    for contract in V1_BOOT_CONTRACTS {
        contracts_ids_to_remap.insert((
            format!("{}:{}", BOOT_MAINNET_ADDRESS, contract),
            format!("{}:{}", BOOT_TESTNET_ADDRESS, contract),
        ));
    }
    for contract in V2_BOOT_CONTRACTS {
        contracts_ids_to_remap.insert((
            format!("{}:{}", BOOT_MAINNET_ADDRESS, contract),
            format!("{}:{}", BOOT_TESTNET_ADDRESS, contract),
        ));
    }
    for contract in V3_BOOT_CONTRACTS {
        contracts_ids_to_remap.insert((
            format!("{}:{}", BOOT_MAINNET_ADDRESS, contract),
            format!("{}:{}", BOOT_TESTNET_ADDRESS, contract),
        ));
    }

    for batch_spec in deployment.plan.batches.iter() {
        let epoch = match batch_spec.epoch {
            Some(epoch) => {
                if network != StacksNetwork::Devnet {
                    println!("warning: 'epoch' specified for a deployment batch is ignored when applying a deployment plan. This field should only be specified for deployments plans used to launch a devnet with 'clarinet integrate'.");
                }
                epoch
            }
            None => default_epoch,
        };
        let mut batch = Vec::new();
        for transaction in batch_spec.transactions.iter() {
            let tracker = match transaction {
                TransactionSpecification::StxTransfer(tx) => {
                    let issuer_address = tx.expected_sender.to_address();
                    let nonce = match accounts_cached_nonces.get(&issuer_address) {
                        Some(cached_nonce) => cached_nonce.clone(),
                        None => stacks_rpc
                            .get_nonce(&issuer_address)
                            .expect("Unable to retrieve account"),
                    };
                    let account = stx_accounts_lookup.get(&issuer_address).unwrap();

                    let anchor_mode = match tx.anchor_block_only {
                        true => TransactionAnchorMode::OnChainOnly,
                        false => TransactionAnchorMode::Any,
                    };

                    let transaction = match encode_stx_transfer(
                        tx.recipient.clone(),
                        tx.mstx_amount,
                        tx.memo,
                        *account,
                        nonce,
                        tx.cost,
                        anchor_mode,
                        &network,
                    ) {
                        Ok(res) => res,
                        Err(e) => {
                            let _ = deployment_event_tx.send(DeploymentEvent::Interrupted(
                                format!("unable to encode stx_transfer ({})", e),
                            ));
                            return;
                        }
                    };

                    accounts_cached_nonces.insert(issuer_address.clone(), nonce + 1);
                    let name = format!(
                        "STX transfer ({}µSTX from {} to {})",
                        tx.mstx_amount,
                        issuer_address,
                        tx.recipient.to_string(),
                    );
                    let check = TransactionCheck::NonceCheck(tx.expected_sender.clone(), nonce);
                    TransactionTracker {
                        index,
                        name: name.clone(),
                        status: TransactionStatus::Encoded(transaction, check),
                    }
                }
                TransactionSpecification::BtcTransfer(tx) => {
                    let url = Url::parse(&bitcoin_node_url).expect("Url malformatted");
                    let auth = match url.password() {
                        Some(password) => {
                            Auth::UserPass(url.username().to_string(), password.to_string())
                        }
                        None => Auth::None,
                    };
                    let bitcoin_node_rpc_url = format!(
                        "{}://{}:{}",
                        url.scheme(),
                        url.host().expect("Host unknown"),
                        url.port_or_known_default().expect("Protocol unknown")
                    );
                    let bitcoin_rpc = Client::new(&bitcoin_node_rpc_url, auth.clone()).unwrap();

                    let bitcoin_node_wallet_rpc_url = format!(
                        "{}://{}:{}/wallet/",
                        url.scheme(),
                        url.host().expect("Host unknown"),
                        url.port_or_known_default().expect("Protocol unknown")
                    );
                    let bitcoin_node_wallet_rpc =
                        Client::new(&bitcoin_node_wallet_rpc_url, auth).unwrap();

                    let account = btc_accounts_lookup.get(&tx.expected_sender).unwrap();
                    let (secret_key, _public_key) = get_btc_keypair(account);
                    let _ = bitcoin_deployment::send_transaction_spec(
                        &bitcoin_rpc,
                        &bitcoin_node_wallet_rpc,
                        tx,
                        &secret_key,
                    );
                    continue;
                }
                TransactionSpecification::ContractCall(tx) => {
                    let issuer_address = tx.expected_sender.to_address();
                    let nonce = match accounts_cached_nonces.get(&issuer_address) {
                        Some(cached_nonce) => cached_nonce.clone(),
                        None => stacks_rpc
                            .get_nonce(&issuer_address)
                            .expect("Unable to retrieve account"),
                    };
                    let account = stx_accounts_lookup.get(&issuer_address).unwrap();

                    let mut function_args = vec![];
                    for value in tx.parameters.iter() {
                        let execution = match session.eval(value.to_string(), None, false) {
                            Ok(res) => res,
                            Err(_e) => {
                                let _ = deployment_event_tx.send(DeploymentEvent::Interrupted(
                                    format!(
                                    "unable to process contract-call {}::{}: argument {} invalid",
                                    tx.contract_id, tx.method, value
                                ),
                                ));
                                return;
                            }
                        };
                        match execution.result {
                            EvaluationResult::Snippet(result) => function_args.push(result.result),
                            _ => unreachable!("Contract result from snippet"),
                        };
                    }

                    let anchor_mode = match tx.anchor_block_only {
                        true => TransactionAnchorMode::OnChainOnly,
                        false => TransactionAnchorMode::Any,
                    };

                    let transaction = match encode_contract_call(
                        &tx.contract_id,
                        tx.method.clone(),
                        function_args,
                        *account,
                        nonce,
                        tx.cost,
                        anchor_mode,
                        &network,
                    ) {
                        Ok(res) => res,
                        Err(e) => {
                            let _ =
                                deployment_event_tx.send(DeploymentEvent::Interrupted(format!(
                                    "unable to encode contract_call {}::{} ({})",
                                    tx.contract_id.to_string(),
                                    tx.method,
                                    e
                                )));
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
                    let check = TransactionCheck::NonceCheck(tx.expected_sender.clone(), nonce);
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
                    let account = stx_accounts_lookup.get(&issuer_address).unwrap();
                    let source = if deployment.network.either_devnet_or_testnet() {
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

                    let anchor_mode = match tx.anchor_block_only {
                        true => TransactionAnchorMode::OnChainOnly,
                        false => TransactionAnchorMode::Any,
                    };

                    let clarity_version = if epoch >= EpochSpec::Epoch2_1 {
                        Some(tx.clarity_version.clone())
                    } else {
                        None
                    };

                    let transaction = match encode_contract_publish(
                        &tx.contract_name,
                        &source,
                        clarity_version,
                        *account,
                        nonce,
                        tx.cost,
                        anchor_mode,
                        &network,
                    ) {
                        Ok(res) => res,
                        Err(e) => {
                            let _ =
                                deployment_event_tx.send(DeploymentEvent::Interrupted(format!(
                                    "unable to encode contract_publish {} ({})",
                                    tx.contract_name, e
                                )));
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

                    // Testnet handling: don't re-deploy previously deployed contracts
                    if deployment.network.is_testnet() {
                        let res = stacks_rpc.get_contract_source(
                            &tx.remap_sender.to_address(),
                            &tx.contract_id.name.to_string(),
                        );
                        if let Ok(_contract) = res {
                            continue;
                        }
                    }

                    // Retrieve nonce for issuer
                    let issuer_address = tx.remap_sender.to_address();
                    let nonce = match accounts_cached_nonces.get(&issuer_address) {
                        Some(cached_nonce) => cached_nonce.clone(),
                        None => stacks_rpc
                            .get_nonce(&issuer_address)
                            .expect("Unable to retrieve account"),
                    };
                    let account = stx_accounts_lookup.get(&issuer_address).unwrap();

                    // Remapping principals - This is happening
                    let mut source = tx.source.clone();
                    for (src_principal, dst_principal) in tx
                        .remap_principals
                        .iter()
                        .map(|(src, dst)| (src.to_address(), dst.to_address()))
                        .chain(
                            contracts_ids_to_remap
                                .iter()
                                .map(|(k, v)| (k.clone(), v.clone())),
                        )
                    {
                        let src = src_principal;
                        let dst = dst_principal;
                        let mut matched_indices = source
                            .match_indices(&src)
                            .map(|(i, _)| i)
                            .collect::<Vec<usize>>();
                        matched_indices.reverse();
                        for index in matched_indices {
                            source.replace_range(index..index + src.len(), &dst);
                        }
                    }

                    let anchor_mode = TransactionAnchorMode::OnChainOnly;

                    let transaction = match encode_contract_publish(
                        &tx.contract_id.name,
                        &source,
                        None,
                        *account,
                        nonce,
                        tx.cost,
                        anchor_mode,
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

        batches.push_back((epoch, batch));
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
    let mut current_bitcoin_block_height = 0;
    for (epoch, batch) in batches.into_iter() {
        if network == StacksNetwork::Devnet {
            // Devnet only: ensure we've reached the appropriate epoch for this batch
            let after_bitcoin_block = match epoch {
                EpochSpec::Epoch2_0 => network_manifest.devnet.as_ref().unwrap().epoch_2_0,
                EpochSpec::Epoch2_05 => network_manifest.devnet.as_ref().unwrap().epoch_2_05,
                EpochSpec::Epoch2_1 => network_manifest.devnet.as_ref().unwrap().epoch_2_1,
                EpochSpec::Epoch2_2 => network_manifest.devnet.as_ref().unwrap().epoch_2_2,
                EpochSpec::Epoch2_3 => network_manifest.devnet.as_ref().unwrap().epoch_2_3,
                EpochSpec::Epoch2_4 => network_manifest.devnet.as_ref().unwrap().epoch_2_4,
            };
            let mut epoch_transition_successful =
                current_bitcoin_block_height > after_bitcoin_block;

            while !epoch_transition_successful {
                let (bitcoin_block_tip, stacks_block_tip) = match stacks_rpc.get_info() {
                    Ok(info) => {
                        if info.stacks_tip_height == 0 {
                            // Always loop if we have not yet seen the genesis block.
                            std::thread::sleep(std::time::Duration::from_secs(
                                delay_between_checks.into(),
                            ));
                            continue;
                        }
                        (info.burn_block_height, info.stacks_tip_height)
                    }
                    Err(_e) => {
                        std::thread::sleep(std::time::Duration::from_secs(
                            delay_between_checks.into(),
                        ));
                        continue;
                    }
                };

                // If no bitcoin block has been mined since `delay_between_checks`,
                // avoid flooding the stacks-node with status update requests.
                if bitcoin_block_tip <= current_bitcoin_block_height {
                    std::thread::sleep(std::time::Duration::from_secs(delay_between_checks.into()));
                    continue;
                }

                current_bitcoin_block_height = bitcoin_block_tip;

                // If no stacks block has been mined despite the new bitcoin block,
                // avoid flooding the stacks-node with status update requests.
                if stacks_block_tip <= current_block_height {
                    std::thread::sleep(std::time::Duration::from_secs(delay_between_checks.into()));
                    continue;
                }

                current_block_height = stacks_block_tip;

                if current_bitcoin_block_height > after_bitcoin_block {
                    epoch_transition_successful = true;
                } else {
                    std::thread::sleep(std::time::Duration::from_secs(delay_between_checks.into()));
                }
            }
        }

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
                    let message = format!("unable to post transaction\n{}", e.to_string());
                    tracker.status = TransactionStatus::Error(message.clone());

                    let _ = deployment_event_tx
                        .send(DeploymentEvent::TransactionUpdate(tracker.clone()));
                    let _ = deployment_event_tx.send(DeploymentEvent::Interrupted(message));
                    return;
                }
            };
        }

        loop {
            let (burn_block_height, stacks_tip_height) = match stacks_rpc.get_info() {
                Ok(info) => (info.burn_block_height, info.stacks_tip_height),
                _ => {
                    std::thread::sleep(std::time::Duration::from_secs(delay_between_checks.into()));
                    continue;
                }
            };

            // If no block has been mined since `delay_between_checks`,
            // avoid flooding the stacks-node with status update requests.
            if burn_block_height <= current_bitcoin_block_height {
                std::thread::sleep(std::time::Duration::from_secs(delay_between_checks.into()));
                continue;
            }

            current_bitcoin_block_height = burn_block_height;
            current_block_height = stacks_tip_height;

            let mut keep_looping = false;

            for (_txid, tracker) in ongoing_batch.iter_mut() {
                match &tracker.status {
                    TransactionStatus::Broadcasted(TransactionCheck::ContractPublish(
                        deployer,
                        contract_name,
                    )) => {
                        let deployer_address = deployer.to_address();
                        let res = stacks_rpc.get_contract_source(&deployer_address, &contract_name);
                        match res {
                            Ok(_contract) => {
                                tracker.status = TransactionStatus::Confirmed;
                                let _ = deployment_event_tx
                                    .send(DeploymentEvent::TransactionUpdate(tracker.clone()));
                            }
                            Err(_e) => {
                                keep_looping = true;
                                break;
                            }
                        }
                    }
                    TransactionStatus::Broadcasted(TransactionCheck::NonceCheck(
                        tx_sender,
                        expected_nonce,
                    )) => {
                        let tx_sender_address = tx_sender.to_address();
                        let res = stacks_rpc.get_nonce(&tx_sender_address);
                        if let Ok(current_nonce) = res {
                            if current_nonce.gt(expected_nonce) {
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

    let _ = deployment_event_tx.send(DeploymentEvent::DeploymentCompleted);
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
                    if !deployment.network.either_devnet_or_testnet() {
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
                TransactionSpecification::BtcTransfer(tx) => TransactionTracker {
                    index,
                    name: format!(
                        "BTC transfer {} send {} satoshis to {}",
                        tx.expected_sender, tx.sats_amount, tx.recipient
                    ),
                    status: TransactionStatus::Queued,
                },
                TransactionSpecification::StxTransfer(tx) => TransactionTracker {
                    index,
                    name: format!(
                        "STX transfer {} send {} µSTC to {}",
                        tx.expected_sender.to_address(),
                        tx.mstx_amount,
                        tx.recipient.to_string()
                    ),
                    status: TransactionStatus::Queued,
                },
                TransactionSpecification::EmulatedContractPublish(_)
                | TransactionSpecification::EmulatedContractCall(_) => continue,
            };
            trackers.push(tracker);
            index += 1;
        }
    }
    trackers
}
