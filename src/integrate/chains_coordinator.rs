use super::DevnetEvent;
use crate::chainhooks::load_chainhooks;
use crate::deployment::{apply_on_chain_deployment, DeploymentCommand, DeploymentEvent};
use crate::integrate::{ServiceStatusData, Status};
use crate::types::ChainsCoordinatorCommand;
use crate::utils;
use base58::FromBase58;
use bitcoincore_rpc::{Auth, Client, RpcApi};
use clarinet_deployments::types::DeploymentSpecification;
use clarinet_files::{self, AccountConfig, DevnetConfig, NetworkManifest, ProjectManifest};
use clarity_repl::clarity::representations::ClarityName;
use clarity_repl::clarity::types::{BuffData, SequenceData, TupleData, Value as ClarityValue};
use clarity_repl::clarity::util::address::AddressHashMode;
use clarity_repl::clarity::util::hash::{hex_bytes, Hash160};

use orchestra_event_observer::observer::{
    start_event_observer, EventObserverConfig, ObserverEvent,
};
use orchestra_types::{BitcoinChainEvent, BitcoinNetwork, StacksChainEvent, StacksNetwork};
use stacks_rpc_client::{transactions, PoxInfo, StacksRpc};
use std::collections::HashMap;
use std::convert::TryFrom;

use std::str;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use tracing::info;

#[derive(Deserialize)]
pub struct NewTransaction {
    pub txid: String,
    pub status: String,
    pub raw_result: String,
    pub raw_tx: String,
}

#[derive(Clone, Debug)]
pub struct DevnetEventObserverConfig {
    pub devnet_config: DevnetConfig,
    pub event_observer_config: EventObserverConfig,
    pub accounts: Vec<AccountConfig>,
    pub deployment: DeploymentSpecification,
    pub manifest: ProjectManifest,
    pub deployment_fee_rate: u64,
}

#[derive(Clone, Debug)]
pub struct DevnetInitializationStatus {
    pub should_deploy_protocol: bool,
}

#[derive(Deserialize, Debug)]
pub struct ContractReadonlyCall {
    pub okay: bool,
    pub result: String,
}

#[allow(dead_code)]
pub enum BitcoinMiningCommand {
    Start,
    Pause,
    Mine,
    InvalidateChainTip,
}

impl DevnetEventObserverConfig {
    pub fn new(
        devnet_config: DevnetConfig,
        manifest: ProjectManifest,
        deployment: DeploymentSpecification,
    ) -> Self {
        info!("Checking contracts...");
        let network_manifest = NetworkManifest::from_project_manifest_location(
            &manifest.location,
            &StacksNetwork::Devnet.get_networks(),
        )
        .expect("unable to load network manifest");

        info!("Checking hooks...");
        let hooks = match load_chainhooks(
            &manifest.location,
            &(BitcoinNetwork::Regtest, StacksNetwork::Devnet),
        ) {
            Ok(hooks) => hooks,
            Err(e) => {
                println!("{}", e);
                std::process::exit(1);
            }
        };

        let event_observer_config = EventObserverConfig {
            normalization_enabled: true,
            grpc_server_enabled: false,
            hooks_enabled: true,
            bitcoin_rpc_proxy_enabled: true,
            event_handlers: vec![],
            initial_hook_formation: Some(hooks),
            ingestion_port: devnet_config.orchestrator_ingestion_port,
            control_port: devnet_config.orchestrator_control_port,
            bitcoin_node_username: devnet_config.bitcoin_node_username.clone(),
            bitcoin_node_password: devnet_config.bitcoin_node_password.clone(),
            bitcoin_node_rpc_host: "http://localhost".into(),
            bitcoin_node_rpc_port: devnet_config.bitcoin_node_rpc_port,
            stacks_node_rpc_host: "http://localhost".into(),
            stacks_node_rpc_port: devnet_config.stacks_node_rpc_port,
            operators: HashMap::new(),
        };

        DevnetEventObserverConfig {
            devnet_config,
            event_observer_config,
            accounts: network_manifest.accounts.into_values().collect::<Vec<_>>(),
            manifest,
            deployment,
            deployment_fee_rate: network_manifest.network.deployment_fee_rate,
        }
    }
}

pub async fn start_chains_coordinator(
    config: DevnetEventObserverConfig,
    devnet_event_tx: Sender<DevnetEvent>,
    chains_coordinator_commands_rx: Receiver<ChainsCoordinatorCommand>,
    chains_coordinator_commands_tx: Sender<ChainsCoordinatorCommand>,
    chains_coordinator_terminator_tx: Sender<bool>,
) -> Result<(), String> {
    let (deployment_events_tx, deployment_events_rx) = channel();
    let (deployment_commands_tx, deployments_command_rx) = channel();

    prepare_protocol_deployment(
        &config.manifest,
        &config.deployment,
        deployment_events_tx,
        deployments_command_rx,
    );

    if let Some(ref hooks) = config.event_observer_config.initial_hook_formation {
        devnet_event_tx
            .send(DevnetEvent::info(format!(
                "{} hooks registered",
                hooks.bitcoin_chainhooks.len() + hooks.stacks_chainhooks.len()
            )))
            .expect("Unable to terminate event observer");
    }

    // Spawn event observer
    let (observer_command_tx, observer_command_rx) = channel();
    let (observer_event_tx, observer_event_rx) = channel();
    let event_observer_config = config.event_observer_config.clone();
    let observer_event_tx_moved = observer_event_tx.clone();
    let _ = std::thread::spawn(move || {
        let future = start_event_observer(
            event_observer_config,
            observer_command_tx,
            observer_command_rx,
            Some(observer_event_tx_moved),
        );
        let _ = utils::nestable_block_on(future);
    });

    // Spawn bitcoin miner controller
    let (mining_command_tx, mining_command_rx) = channel();
    let devnet_config = config.devnet_config.clone();
    std::thread::spawn(move || {
        handle_bitcoin_mining(mining_command_rx, &devnet_config);
    });

    // Loop over events being received from Bitcoin and Stacks,
    // and orchestrate the 2 chains + protocol.
    let mut should_deploy_protocol = true;
    let protocol_deployed = Arc::new(AtomicBool::new(false));

    let mut deployment_events_rx = Some(deployment_events_rx);

    loop {
        // Did we receive a termination notice?
        if let Ok(ChainsCoordinatorCommand::Terminate) = chains_coordinator_commands_rx.try_recv() {
            let _ = chains_coordinator_terminator_tx.send(true);
            break;
        }
        let command = match observer_event_rx.recv() {
            Ok(cmd) => cmd,
            Err(_e) => {
                // TODO(lgalabru): cascade termination
                continue;
            }
        };
        match command {
            ObserverEvent::Fatal(msg) => {
                devnet_event_tx
                    .send(DevnetEvent::error(msg))
                    .expect("Unable to terminate event observer");
                // Terminate
            }
            ObserverEvent::Error(msg) => {
                devnet_event_tx
                    .send(DevnetEvent::error(msg))
                    .expect("Unable to terminate event observer");
            }
            ObserverEvent::Info(msg) => {
                devnet_event_tx
                    .send(DevnetEvent::info(msg))
                    .expect("Unable to terminate event observer");
            }
            ObserverEvent::BitcoinChainEvent(chain_update) => {
                // Contextual shortcut: Devnet is an environment under control,
                // with 1 miner. As such we will ignore Reorgs handling.
                let (log, status) = match &chain_update {
                    BitcoinChainEvent::ChainUpdatedWithBlock(block) => {
                        let log =
                            format!("Bitcoin block #{} received", block.block_identifier.index);
                        let status = format!(
                            "mining blocks (chaintip = #{})",
                            block.block_identifier.index
                        );
                        (log, status)
                    }
                    BitcoinChainEvent::ChainUpdatedWithReorg(_old_blocks, new_blocks) => {
                        let tip = new_blocks.last().unwrap();
                        let log = format!(
                            "Bitcoin reorg received (new height: {})",
                            tip.block_identifier.index
                        );
                        let status =
                            format!("mining blocks (chaintip = #{})", tip.block_identifier.index);
                        (log, status)
                    }
                };

                let _ = devnet_event_tx.send(DevnetEvent::debug(log));

                let _ = devnet_event_tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
                    order: 0,
                    status: Status::Green,
                    name: "bitcoin-node".into(),
                    comment: status,
                }));
                let _ = devnet_event_tx.send(DevnetEvent::BitcoinChainEvent(chain_update.clone()));
            }
            ObserverEvent::StacksChainEvent(chain_event) => {
                if should_deploy_protocol {
                    should_deploy_protocol = false;

                    let automining_disabled =
                        config.devnet_config.bitcoin_controller_automining_disabled;
                    let mining_command_tx_moved = mining_command_tx.clone();
                    let protocol_deployed_moved = protocol_deployed.clone();
                    let (deployment_progress_tx, deployment_progress_rx) = channel();

                    if let Some(deployment_events_rx) = deployment_events_rx.take() {
                        perform_protocol_deployment(
                            deployment_events_rx,
                            deployment_progress_tx,
                            &deployment_commands_tx,
                            &devnet_event_tx,
                            &chains_coordinator_commands_tx,
                        )
                    }

                    std::thread::spawn(move || loop {
                        match deployment_progress_rx.recv() {
                            Ok(DeploymentEvent::ProtocolDeployed) => {
                                protocol_deployed_moved.store(true, Ordering::SeqCst);
                                if !automining_disabled {
                                    let _ =
                                        mining_command_tx_moved.send(BitcoinMiningCommand::Start);
                                }
                            }
                            _ => continue,
                        }
                    });
                }

                let update = match &chain_event {
                    StacksChainEvent::ChainUpdatedWithBlock(block) => block.clone(),
                    StacksChainEvent::ChainUpdatedWithMicroblock(_) => {
                        continue;
                        // TODO(lgalabru): good enough for now - code path unreachable in the context of Devnet
                    }
                    StacksChainEvent::ChainUpdatedWithMicroblockReorg(_) => {
                        unreachable!() // TODO(lgalabru): good enough for now - code path unreachable in the context of Devnet
                    }
                    StacksChainEvent::ChainUpdatedWithReorg(_) => {
                        unreachable!() // TODO(lgalabru): good enough for now - code path unreachable in the context of Devnet
                    }
                };

                let _ = devnet_event_tx.send(DevnetEvent::StacksChainEvent(chain_event));

                // Partially update the UI. With current approach a full update
                // would requires either cloning the block, or passing ownership.
                let _ = devnet_event_tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
                    order: 1,
                    status: Status::Green,
                    name: "stacks-node".into(),
                    comment: format!(
                        "mining blocks (chaintip = #{})",
                        update.new_block.block_identifier.index
                    ),
                }));
                let _ = devnet_event_tx.send(DevnetEvent::info(format!(
                    "Block #{} anchored in Bitcoin block #{} includes {} transactions",
                    update.new_block.block_identifier.index,
                    update
                        .new_block
                        .metadata
                        .bitcoin_anchor_block_identifier
                        .index,
                    update.new_block.transactions.len(),
                )));

                let should_submit_pox_orders = update.new_block.metadata.pox_cycle_position
                    == (update.new_block.metadata.pox_cycle_length - 2);
                if should_submit_pox_orders {
                    let bitcoin_block_height = update.new_block.block_identifier.index;
                    let res = publish_stacking_orders(
                        &config.devnet_config,
                        &config.accounts,
                        config.deployment_fee_rate,
                        bitcoin_block_height as u32,
                    )
                    .await;
                    if let Some(tx_count) = res {
                        let _ = devnet_event_tx.send(DevnetEvent::success(format!(
                            "Will broadcast {} stacking orders",
                            tx_count
                        )));
                    }
                }
            }
            ObserverEvent::NotifyBitcoinTransactionProxied => {
                if !protocol_deployed.load(Ordering::SeqCst) {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    mine_bitcoin_block(
                        config.devnet_config.bitcoin_node_rpc_port,
                        config.devnet_config.bitcoin_node_username.as_str(),
                        &config.devnet_config.bitcoin_node_password.as_str(),
                        &config.devnet_config.miner_btc_address.as_str(),
                    );
                }
            }
            ObserverEvent::HookRegistered(hook) => {
                let _ = devnet_event_tx.send(DevnetEvent::info(format!(
                    "New hook \"{}\" registered",
                    hook.name()
                )));
            }
            ObserverEvent::HookDeregistered(_hook) => {}
            ObserverEvent::HooksTriggered(count) => {
                let _ =
                    devnet_event_tx.send(DevnetEvent::info(format!("{} hooks triggered", count)));
            }
            ObserverEvent::Terminate => {}
        }
    }
    Ok(())
}

pub fn prepare_protocol_deployment(
    manifest: &ProjectManifest,
    deployment: &DeploymentSpecification,
    deployment_event_tx: Sender<DeploymentEvent>,
    deployment_command_rx: Receiver<DeploymentCommand>,
) {
    let manifest = manifest.clone();
    let deployment = deployment.clone();

    std::thread::spawn(move || {
        apply_on_chain_deployment(
            &manifest,
            deployment,
            deployment_event_tx,
            deployment_command_rx,
            false,
        );
    });
}

pub fn perform_protocol_deployment(
    deployment_events_rx: Receiver<DeploymentEvent>,
    deployment_events_tx: Sender<DeploymentEvent>,
    deployment_commands_tx: &Sender<DeploymentCommand>,
    devnet_event_tx: &Sender<DevnetEvent>,
    chains_coordinator_commands_tx: &Sender<ChainsCoordinatorCommand>,
) {
    let devnet_event_tx = devnet_event_tx.clone();
    let chains_coordinator_commands_tx = chains_coordinator_commands_tx.clone();

    let _ = deployment_commands_tx.send(DeploymentCommand::Start);

    std::thread::spawn(move || {
        loop {
            let event = match deployment_events_rx.recv() {
                Ok(event) => event,
                Err(_e) => break,
            };
            match event {
                DeploymentEvent::TransactionUpdate(_) => {}
                DeploymentEvent::Interrupted(_) => {
                    // Terminate
                    break;
                }
                DeploymentEvent::ProtocolDeployed => {
                    let _ = chains_coordinator_commands_tx
                        .send(ChainsCoordinatorCommand::ProtocolDeployed);
                    let _ = devnet_event_tx.send(DevnetEvent::ProtocolDeployed);
                    let _ = deployment_events_tx.send(DeploymentEvent::ProtocolDeployed);
                    break;
                }
            }
        }
    });
}

pub async fn publish_stacking_orders(
    devnet_config: &DevnetConfig,
    accounts: &Vec<AccountConfig>,
    fee_rate: u64,
    bitcoin_block_height: u32,
) -> Option<usize> {
    if devnet_config.pox_stacking_orders.len() == 0 {
        return None;
    }

    let stacks_node_rpc_url = format!("http://localhost:{}", devnet_config.stacks_node_rpc_port);

    let mut transactions = 0;
    let pox_info: PoxInfo = reqwest::get(format!("{}/v2/pox", stacks_node_rpc_url))
        .await
        .expect("Unable to retrieve pox info")
        .json()
        .await
        .expect("Unable to parse contract");

    for pox_stacking_order in devnet_config.pox_stacking_orders.iter() {
        if pox_stacking_order.start_at_cycle == (pox_info.reward_cycle_id + 1) {
            let mut account = None;
            let mut accounts_iter = accounts.iter();
            while let Some(e) = accounts_iter.next() {
                if e.label == pox_stacking_order.wallet {
                    account = Some(e.clone());
                    break;
                }
            }
            let account = match account {
                Some(account) => account,
                _ => continue,
            };

            transactions += 1;

            let stx_amount = pox_info.next_cycle.min_threshold_ustx * pox_stacking_order.slots;
            let addr_bytes = pox_stacking_order
                .btc_address
                .from_base58()
                .expect("Unable to get bytes from btc address");
            let duration = pox_stacking_order.duration.into();
            let node_url = stacks_node_rpc_url.clone();
            let pox_contract_id = pox_info.contract_id.clone();

            std::thread::spawn(move || {
                let default_fee = fee_rate * 1000;
                let stacks_rpc = StacksRpc::new(&node_url);
                let nonce = stacks_rpc
                    .get_nonce(&account.stx_address)
                    .expect("Unable to retrieve nonce");

                let (_, _, account_secret_key) = clarinet_files::compute_addresses(
                    &account.mnemonic,
                    &account.derivation,
                    &StacksNetwork::Devnet.get_networks(),
                );

                let addr_bytes = Hash160::from_bytes(&addr_bytes[1..21]).unwrap();
                let addr_version = AddressHashMode::SerializeP2PKH;
                let stack_stx_tx = transactions::build_contrat_call_transaction(
                    pox_contract_id,
                    "stack-stx".into(),
                    vec![
                        ClarityValue::UInt(stx_amount.into()),
                        ClarityValue::Tuple(
                            TupleData::from_data(vec![
                                (
                                    ClarityName::try_from("version".to_owned()).unwrap(),
                                    ClarityValue::buff_from_byte(addr_version as u8),
                                ),
                                (
                                    ClarityName::try_from("hashbytes".to_owned()).unwrap(),
                                    ClarityValue::Sequence(SequenceData::Buffer(BuffData {
                                        data: addr_bytes.as_bytes().to_vec(),
                                    })),
                                ),
                            ])
                            .unwrap(),
                        ),
                        ClarityValue::UInt((bitcoin_block_height - 1).into()),
                        ClarityValue::UInt(duration),
                    ],
                    nonce,
                    default_fee,
                    &hex_bytes(&account_secret_key).unwrap(),
                );
                let _ = stacks_rpc
                    .post_transaction(&stack_stx_tx)
                    .expect("Unable to broadcast transaction");
            });
        }
    }
    if transactions > 0 {
        Some(transactions)
    } else {
        None
    }
}

pub fn invalidate_bitcoin_chain_tip(
    bitcoin_node_rpc_port: u16,
    bitcoin_node_username: &str,
    bitcoin_node_password: &str,
) {
    let rpc = Client::new(
        &format!("http://localhost:{}", bitcoin_node_rpc_port),
        Auth::UserPass(
            bitcoin_node_username.to_string(),
            bitcoin_node_password.to_string(),
        ),
    )
    .unwrap();

    let chain_tip = rpc.get_best_block_hash().expect("Unable to get chain tip");
    let _ = rpc
        .invalidate_block(&chain_tip)
        .expect("Unable to invalidate chain tip");
}

pub fn mine_bitcoin_block(
    bitcoin_node_rpc_port: u16,
    bitcoin_node_username: &str,
    bitcoin_node_password: &str,
    miner_btc_address: &str,
) {
    use bitcoincore_rpc::bitcoin::Address;
    use std::str::FromStr;
    let rpc = Client::new(
        &format!("http://localhost:{}", bitcoin_node_rpc_port),
        Auth::UserPass(
            bitcoin_node_username.to_string(),
            bitcoin_node_password.to_string(),
        ),
    )
    .unwrap();
    let miner_address = Address::from_str(miner_btc_address).unwrap();
    let _ = rpc.generate_to_address(1, &miner_address);
}

fn handle_bitcoin_mining(
    mining_command_rx: Receiver<BitcoinMiningCommand>,
    devnet_config: &DevnetConfig,
) {
    let stop_miner = Arc::new(AtomicBool::new(false));
    loop {
        let command = match mining_command_rx.recv() {
            Ok(cmd) => cmd,
            Err(_e) => {
                // TODO(lgalabru): cascade termination
                continue;
            }
        };
        match command {
            BitcoinMiningCommand::Start => {
                stop_miner.store(false, Ordering::SeqCst);
                let stop_miner_reader = stop_miner.clone();
                let devnet_config = devnet_config.clone();
                std::thread::spawn(move || loop {
                    std::thread::sleep(std::time::Duration::from_millis(
                        devnet_config.bitcoin_controller_block_time.into(),
                    ));
                    mine_bitcoin_block(
                        devnet_config.bitcoin_node_rpc_port,
                        &devnet_config.bitcoin_node_username,
                        &devnet_config.bitcoin_node_password,
                        &devnet_config.miner_btc_address,
                    );
                    if stop_miner_reader.load(Ordering::SeqCst) {
                        break;
                    }
                });
            }
            BitcoinMiningCommand::Pause => {
                stop_miner.store(true, Ordering::SeqCst);
            }
            BitcoinMiningCommand::Mine => {
                mine_bitcoin_block(
                    devnet_config.bitcoin_node_rpc_port,
                    devnet_config.bitcoin_node_username.as_str(),
                    &devnet_config.bitcoin_node_password.as_str(),
                    &devnet_config.miner_btc_address.as_str(),
                );
            }
            BitcoinMiningCommand::InvalidateChainTip => {
                invalidate_bitcoin_chain_tip(
                    devnet_config.bitcoin_node_rpc_port,
                    &devnet_config.bitcoin_node_username.as_str(),
                    &devnet_config.bitcoin_node_password.as_str(),
                );
            }
        }
    }
}
