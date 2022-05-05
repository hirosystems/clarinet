mod types;
mod ui;

use self::types::DeploymentPlan;
use crate::integrate::DevnetEvent;
use crate::poke::{load_session, load_session_settings};
use crate::types::{DeploymentEvent, ProjectManifest, StacksNetwork};
use crate::utils::mnemonic;
use clarity_repl::clarity::codec::transaction::{
    StacksTransaction, StacksTransactionSigner, TransactionAnchorMode, TransactionAuth,
    TransactionPayload, TransactionPostConditionMode, TransactionPublicKeyEncoding,
    TransactionSmartContract, TransactionSpendingCondition,
};
use clarity_repl::clarity::codec::StacksMessageCodec;
use clarity_repl::clarity::util::{
    C32_ADDRESS_VERSION_MAINNET_SINGLESIG, C32_ADDRESS_VERSION_TESTNET_SINGLESIG,
};
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
use clarity_repl::repl::settings::{Account, InitialContract};
use libsecp256k1::{PublicKey, SecretKey};
use serde_yaml;
use stacks_rpc_client::StacksRpc;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use tiny_hderive::bip32::ExtendedPrivKey;

#[derive(Deserialize, Debug)]
pub struct Balance {
    pub balance: String,
    pub nonce: u64,
    pub balance_proof: String,
    pub nonce_proof: String,
}

pub enum PublishUpdate {
    ContractUpdate(ContractUpdate),
    Completed,
}

#[derive(Clone, Debug)]
pub struct ContractUpdate {
    pub contract_id: String,
    pub status: ContractStatus,
    pub comment: Option<String>,
}

#[derive(Clone, Debug)]
pub enum ContractStatus {
    Queued,
    Encoded,
    Broadcasted,
    Published,
    Error,
}

pub fn build_plan(manifest_path: &PathBuf, network: &StacksNetwork) -> DeploymentPlan {
    // Load manifest

    // Load global network state, if any

    // Load contracts
    // -> Compare hash, any update?

    // Load hooks
    // -> Compare hash, any update?

    // If transactions are present between outdated contracts, raise a warning
    //

    DeploymentPlan {
        network: network.clone(),
        actions: vec![],
    }
}

pub fn endode_contract(
    contract: &InitialContract,
    account: &Account,
    nonce: u64,
    deployment_fee_rate: u64,
    network: &StacksNetwork,
) -> Result<(StacksTransaction, StacksAddress), String> {
    let contract_name = contract.name.clone().unwrap();

    let payload = TransactionSmartContract {
        name: contract_name.as_str().into(),
        code_body: StacksString::from_string(&contract.code).unwrap(),
    };

    let bip39_seed = match mnemonic::get_bip39_seed_from_mnemonic(&account.mnemonic, "") {
        Ok(bip39_seed) => bip39_seed,
        Err(_) => panic!(),
    };
    let ext = ExtendedPrivKey::derive(&bip39_seed[..], account.derivation.as_str()).unwrap();
    let secret_key = SecretKey::parse_slice(&ext.secret()).unwrap();
    let public_key = PublicKey::from_secret_key(&secret_key);

    let wrapped_public_key =
        Secp256k1PublicKey::from_slice(&public_key.serialize_compressed()).unwrap();
    let wrapped_secret_key = Secp256k1PrivateKey::from_slice(&ext.secret()).unwrap();

    let anchor_mode = TransactionAnchorMode::OnChainOnly;
    let tx_fee = deployment_fee_rate * contract.code.len() as u64;

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
        payload: TransactionPayload::SmartContract(payload),
    };

    let mut unsigned_tx_bytes = vec![];
    unsigned_tx
        .consensus_serialize(&mut unsigned_tx_bytes)
        .expect("FATAL: invalid transaction");

    let mut tx_signer = StacksTransactionSigner::new(&unsigned_tx);
    tx_signer.sign_origin(&wrapped_secret_key).unwrap();
    let signed_tx = tx_signer.get_tx().unwrap();

    Ok((signed_tx, signer_addr))
}

#[allow(dead_code)]
pub fn publish_contract(
    contract: &InitialContract,
    deployers_lookup: &HashMap<String, Account>,
    deployers_nonces: &mut HashMap<String, u64>,
    node_url: &str,
    deployment_fee_rate: u64,
    network: &StacksNetwork,
) -> Result<(String, u64, String, String), String> {
    let contract_name = contract.name.clone().unwrap();

    let stacks_rpc = StacksRpc::new(&node_url);

    let deployer = match deployers_lookup.get(&contract_name) {
        Some(deployer) => deployer,
        None => deployers_lookup.get("*").unwrap(),
    };

    let nonce = match deployers_nonces.get(&deployer.name) {
        Some(nonce) => *nonce,
        None => {
            let nonce = stacks_rpc
                .get_nonce(&deployer.address)
                .expect("Unable to retrieve account");
            deployers_nonces.insert(deployer.name.clone(), nonce);
            nonce
        }
    };

    let (signed_tx, signer_addr) =
        endode_contract(contract, deployer, nonce, deployment_fee_rate, network)?;
    let txid = match stacks_rpc.post_transaction(signed_tx) {
        Ok(res) => res.txid,
        Err(e) => return Err(format!("{:?}", e)),
    };
    deployers_nonces.insert(deployer.name.clone(), nonce + 1);
    Ok((txid, nonce, signer_addr.to_string(), contract_name))
}

/// publish_all_contracts publish all the contracts referenced by the manifest passed.
/// this method is being used by developers
pub fn publish_all_contracts(
    manifest_path: &PathBuf,
    network: &StacksNetwork,
    analysis_enabled: bool,
    delay_between_checks: u32,
    devnet_event_tx: Option<&Sender<DevnetEvent>>,
    deployment_event_tx: Option<Sender<DeploymentEvent>>,
) -> Result<(Vec<String>, ProjectManifest), Vec<String>> {
    let (settings, chain, project_manifest) = if analysis_enabled {
        let start_repl = false;
        let (session, chain, project_manifest, output) =
            match load_session(manifest_path, start_repl, &network) {
                Ok((session, chain, project_manifest, output)) => {
                    (session, chain, project_manifest, output)
                }
                Err((_, e)) => return Err(vec![e]),
            };

        if let Some(message) = output {
            println!("{}", message);
            println!("{}", yellow!("Would you like to continue [Y/n]:"));
            let mut buffer = String::new();
            std::io::stdin().read_line(&mut buffer).unwrap();
            if buffer == "n\n" {
                println!("{}", red!("Contracts deployment aborted"));
                std::process::exit(1);
            }
        }
        (session.settings, chain, project_manifest)
    } else {
        let (settings, chain, project_manifest) =
            match load_session_settings(manifest_path, &network) {
                Ok((session, chain, project_manifest)) => (session, chain, project_manifest),
                Err(e) => return Err(vec![e]),
            };
        (settings, chain, project_manifest)
    };

    let (tx, rx) = channel();
    let (per_contract_event_tx, per_contract_event_rx) = channel();

    let contracts_to_deploy = settings.initial_contracts.len();
    let node_url = settings.node.clone();

    // Approach's description: after getting all the contracts indexed by the manifest, we build a
    // a sorted set that we will be splitting in batches of 25 contracts, which is the number of
    // transactions that you can have for one user at a given time in a mempool.
    // We then keep fetching the stacks-node every `delay_between_checks` seconds and wait for
    // all the contracts to be published, and then move on to the next batch.
    // We're using a channel here because this routine can be used in 2 different contexts:
    // - clarinet contracts publish: display a UI that let developers tracking the progress
    // - clarinet integrate / orchestra: nothing displayed, but the events are being used
    // for coordinating the chains setup.
    let per_contract_event_tx_moved = per_contract_event_tx.clone();
    let deploying_thread_handle = std::thread::spawn(move || {
        let mut total_contracts_deployed = 0;
        let stacks_rpc = StacksRpc::new(&node_url);

        while contracts_to_deploy != total_contracts_deployed {
            let mut current_block_height = 0;
            let mut contracts_batch: Vec<(StacksTransaction, StacksAddress, String)> =
                rx.recv().unwrap();
            let mut batch_deployed = false;
            let mut contracts_being_deployed: BTreeMap<(String, String), bool> = BTreeMap::new();
            loop {
                let new_block_height = match stacks_rpc.get_info() {
                    Ok(info) => info.burn_block_height,
                    _ => {
                        std::thread::sleep(std::time::Duration::from_secs(
                            delay_between_checks.into(),
                        ));
                        continue;
                    }
                };

                if new_block_height <= current_block_height {
                    std::thread::sleep(std::time::Duration::from_secs(delay_between_checks.into()));
                    continue;
                }

                current_block_height = new_block_height;

                if !batch_deployed {
                    batch_deployed = true;
                    for (tx, deployer, contract_name) in contracts_batch.drain(..) {
                        let _ = match stacks_rpc.post_transaction(tx) {
                            Ok(res) => {
                                let _ = per_contract_event_tx_moved.send(
                                    PublishUpdate::ContractUpdate(ContractUpdate {
                                        contract_id: format!("{}.{}", deployer, contract_name),
                                        status: ContractStatus::Broadcasted,
                                        comment: Some(format!(
                                            "Contract broadcasted: {}",
                                            res.txid.clone()
                                        )),
                                    }),
                                );
                                contracts_being_deployed
                                    .insert((deployer.to_string(), contract_name), true);
                                res.txid
                            }
                            Err(e) => {
                                let _ = per_contract_event_tx_moved.send(
                                    PublishUpdate::ContractUpdate(ContractUpdate {
                                        contract_id: format!("{}.{}", deployer, contract_name),
                                        status: ContractStatus::Error,
                                        comment: Some(format!("Broadcast error: {:?}", e)),
                                    }),
                                );
                                std::process::exit(1);
                            }
                        };
                    }
                    std::thread::sleep(std::time::Duration::from_secs(delay_between_checks.into()));
                    continue;
                }

                let mut keep_looping = false;

                for ((deployer, contract_name), value) in contracts_being_deployed.iter_mut() {
                    if *value {
                        let res = stacks_rpc.get_contract_source(&deployer, &contract_name);
                        if let Ok(_contract) = res {
                            *value = false;
                            total_contracts_deployed += 1;
                            let _ = per_contract_event_tx_moved.send(
                                PublishUpdate::ContractUpdate(ContractUpdate {
                                    contract_id: format!("{}.{}", deployer, contract_name),
                                    status: ContractStatus::Published,
                                    comment: None,
                                }),
                            );
                        } else {
                            keep_looping = true;
                            break;
                        }
                    }
                }

                if !keep_looping {
                    break;
                }
            }
        }
        let _ = per_contract_event_tx_moved.send(PublishUpdate::Completed);
    });

    let results = vec![];
    let mut deployers_nonces = HashMap::new();
    let mut deployers_lookup: HashMap<String, Account> = HashMap::new();

    for account in settings.initial_accounts.iter() {
        deployers_lookup.insert(account.name.to_string(), account.clone());
        if account.name == "deployer" {
            deployers_lookup.insert("*".into(), account.clone());
        }
        // Let's avoid fetching nonces in the case of initial Devnet setup.
        if devnet_event_tx.is_some() {
            deployers_nonces.insert(account.name.clone(), 0);
        }
    }

    let node_url = settings.node.clone();
    let stacks_rpc = StacksRpc::new(&node_url);

    for batch in settings.initial_contracts.chunks(25) {
        let mut encoded_contracts = vec![];

        for contract in batch.iter() {
            let contract_name = contract.name.clone().unwrap();

            let deployer = match deployers_lookup.get(&contract_name) {
                Some(deployer) => deployer,
                None => deployers_lookup.get("*").unwrap(),
            };

            let nonce = match deployers_nonces.get(&deployer.name) {
                Some(nonce) => *nonce,
                None => {
                    let nonce = stacks_rpc
                        .get_nonce(&deployer.address)
                        .expect("Unable to retrieve account");
                    deployers_nonces.insert(deployer.name.clone(), nonce);
                    nonce
                }
            };

            let (signed_tx, signer_addr) = endode_contract(
                contract,
                deployer,
                nonce,
                chain.network.deployment_fee_rate,
                network,
            )
            .expect("Unable to encode contract");

            let _ = per_contract_event_tx.send(PublishUpdate::ContractUpdate(ContractUpdate {
                contract_id: format!("{}.{}", deployer.address, contract_name),
                status: ContractStatus::Encoded,
                comment: Some(format!("Contract encoded and queued")),
            }));

            encoded_contracts.push((signed_tx, signer_addr, contract_name));
            deployers_nonces.insert(deployer.name.clone(), nonce + 1);
        }

        let _ = tx.send(encoded_contracts);
    }

    if devnet_event_tx.is_none() {
        let mut contracts = Vec::new();
        for contract in settings.initial_contracts.iter() {
            let deployer = {
                let deployer = contract.deployer.clone().unwrap_or("deployer".to_string());
                match deployers_lookup.get(&deployer) {
                    Some(deployer) => deployer,
                    None => deployers_lookup.get("*").unwrap(),
                }
            };
            contracts.push((deployer.address.to_string(), contract.name.clone().unwrap()));
        }

        ctrlc::set_handler(move || {
            let _ = per_contract_event_tx.send(PublishUpdate::Completed);
        })
        .expect("Error setting Ctrl-C handler");

        let _ = ui::start_ui(&node_url, per_contract_event_rx, contracts);
    } else {
        let _ = deploying_thread_handle.join();
    }

    // TODO(lgalabru): if devnet, we should be pulling all the links.

    if let Some(deployment_event_tx) = deployment_event_tx {
        let _ = deployment_event_tx.send(DeploymentEvent::ProtocolDeployed);
    }

    if let Some(devnet_event_tx) = devnet_event_tx {
        let _ = devnet_event_tx.send(DevnetEvent::ProtocolDeployed);
    } else {
        println!(
            "{} Contracts successfully deployed on {:?}",
            green!("✔"),
            network
        );
    }

    Ok((results, project_manifest))
}
