use super::ChainsCoordinatorCommand;
use crate::event::DevnetEvent;
use crate::event::ServiceStatusData;
use crate::event::Status;
use crate::orchestrator::ServicesMapHosts;
use base58::FromBase58;
use chainhook_sdk::chainhook_types::BitcoinBlockSignaling;
use chainhook_sdk::chainhook_types::BitcoinNetwork;
use chainhook_sdk::chainhooks::types::ChainhookConfig;
use chainhook_sdk::utils::Context;
use clarinet_deployments::onchain::TransactionStatus;
use clarinet_deployments::onchain::{
    apply_on_chain_deployment, DeploymentCommand, DeploymentEvent,
};
use clarinet_deployments::types::DeploymentSpecification;
use clarinet_files::{self, AccountConfig, DevnetConfig, NetworkManifest, ProjectManifest};
use hiro_system_kit;
use hiro_system_kit::slog;

use chainhook_sdk::chainhook_types::{BitcoinChainEvent, StacksChainEvent};
use chainhook_sdk::observer::{
    start_event_observer, EventObserverConfig, ObserverCommand, ObserverEvent,
    StacksChainMempoolEvent,
};
use clarinet_files::chainhook_types::StacksNetwork;

use clarity_repl::clarity::address::AddressHashMode;
use clarity_repl::clarity::util::hash::{hex_bytes, Hash160};
use clarity_repl::clarity::vm::types::{BuffData, SequenceData, TupleData};
use clarity_repl::clarity::vm::ClarityName;
use clarity_repl::clarity::vm::Value as ClarityValue;
use clarity_repl::codec;
use stacks_rpc_client::{PoxInfo, StacksRpc};
use std::convert::TryFrom;

use std::str;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;

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
    pub services_map_hosts: ServicesMapHosts,
}

impl DevnetEventObserverConfig {
    pub fn consolidated_stacks_rpc_url(&self) -> String {
        format!("http://{}", self.services_map_hosts.stacks_node_host)
    }

    pub fn consolidated_bitcoin_rpc_url(&self) -> String {
        format!("http://{}", self.services_map_hosts.bitcoin_node_host)
    }
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

#[derive(Debug)]
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
        chainhooks: ChainhookConfig,
        ctx: &Context,
        services_map_hosts: ServicesMapHosts,
    ) -> Self {
        ctx.try_log(|logger| slog::info!(logger, "Checking contracts"));
        let network_manifest = NetworkManifest::from_project_manifest_location(
            &manifest.location,
            &StacksNetwork::Devnet.get_networks(),
            Some(&manifest.project.cache_location),
            None,
        )
        .expect("unable to load network manifest");

        let event_observer_config = EventObserverConfig {
            bitcoin_rpc_proxy_enabled: true,
            event_handlers: vec![],
            chainhook_config: Some(chainhooks),
            ingestion_port: devnet_config.orchestrator_ingestion_port,
            bitcoind_rpc_username: devnet_config.bitcoin_node_username.clone(),
            bitcoind_rpc_password: devnet_config.bitcoin_node_password.clone(),
            bitcoind_rpc_url: format!("http://{}", services_map_hosts.bitcoin_node_host),
            bitcoin_block_signaling: BitcoinBlockSignaling::Stacks("http://0.0.0.0:20443".into()),
            stacks_node_rpc_url: format!("http://{}", services_map_hosts.stacks_node_host),
            display_logs: true,
            cache_path: devnet_config.working_dir.to_string(),
            bitcoin_network: BitcoinNetwork::Regtest,
            stacks_network: chainhook_sdk::chainhook_types::StacksNetwork::Devnet,
        };

        DevnetEventObserverConfig {
            devnet_config,
            event_observer_config,
            accounts: network_manifest.accounts.into_values().collect::<Vec<_>>(),
            manifest,
            deployment,
            deployment_fee_rate: network_manifest.network.deployment_fee_rate,
            services_map_hosts,
        }
    }
}

pub async fn start_chains_coordinator(
    config: DevnetEventObserverConfig,
    devnet_event_tx: Sender<DevnetEvent>,
    chains_coordinator_commands_rx: crossbeam_channel::Receiver<ChainsCoordinatorCommand>,
    _chains_coordinator_commands_tx: crossbeam_channel::Sender<ChainsCoordinatorCommand>,
    orchestrator_terminator_tx: Sender<bool>,
    observer_command_tx: Sender<ObserverCommand>,
    observer_command_rx: Receiver<ObserverCommand>,
    ctx: Context,
) -> Result<(), String> {
    let mut should_deploy_protocol = true; // Will change when `stacks-network` components becomes compatible with Testnet / Mainnet setups
    let boot_completed = Arc::new(AtomicBool::new(false));

    let (deployment_commands_tx, deployments_command_rx) = channel();
    let (deployment_events_tx, deployment_events_rx) = channel();
    let (mining_command_tx, mining_command_rx) = channel();

    // Set-up the background task in charge of serializing / signing / publishing the contracts.
    // This tasks can take several seconds to minutes, depending on the complexity of the project.
    // We start this process as soon as possible, as a background task.
    // This thread becomes dormant once the encoding is done, and proceed to the actual deployment once
    // the event DeploymentCommand::Start is received.
    perform_protocol_deployment(
        &config.manifest,
        &config.deployment,
        deployment_events_tx,
        deployments_command_rx,
        Some(config.consolidated_bitcoin_rpc_url()),
        Some(config.consolidated_stacks_rpc_url()),
    );

    // Set-up the background task in charge of monitoring contracts deployments.
    // This thread will be waiting and relaying events emitted by the thread above.
    relay_devnet_protocol_deployment(
        deployment_events_rx,
        &devnet_event_tx,
        Some(mining_command_tx.clone()),
        &boot_completed,
    );

    if let Some(ref hooks) = config.event_observer_config.chainhook_config {
        let chainhooks_count = hooks.bitcoin_chainhooks.len() + hooks.stacks_chainhooks.len();
        if chainhooks_count > 0 {
            devnet_event_tx
                .send(DevnetEvent::info(format!(
                    "{chainhooks_count} chainhooks registered",
                )))
                .expect("Unable to terminate event observer");
        }
    }

    // Spawn event observer
    let (observer_event_tx, observer_event_rx) = crossbeam_channel::unbounded();
    let event_observer_config = config.event_observer_config.clone();
    let observer_event_tx_moved = observer_event_tx.clone();
    let observer_command_tx_moved = observer_command_tx.clone();
    let ctx_moved = ctx.clone();
    let _ = hiro_system_kit::thread_named("Event observer").spawn(move || {
        let future = start_event_observer(
            event_observer_config,
            observer_command_tx_moved,
            observer_command_rx,
            Some(observer_event_tx_moved),
            ctx_moved,
        );
        let _ = hiro_system_kit::nestable_block_on(future);
    });

    // Spawn bitcoin miner controller
    let devnet_event_tx_moved = devnet_event_tx.clone();
    let devnet_config = config.clone();
    let _ = hiro_system_kit::thread_named("Bitcoin mining").spawn(move || {
        let future =
            handle_bitcoin_mining(mining_command_rx, &devnet_config, &devnet_event_tx_moved);
        let _ = hiro_system_kit::nestable_block_on(future);
    });

    // Loop over events being received from Bitcoin and Stacks,
    // and orchestrate the 2 chains + protocol.
    let mut deployment_commands_tx = Some(deployment_commands_tx);
    let mut subnet_initialized = false;

    let mut sel = crossbeam_channel::Select::new();
    let chains_coordinator_commands_oper = sel.recv(&chains_coordinator_commands_rx);
    let observer_event_oper = sel.recv(&observer_event_rx);

    loop {
        let oper = sel.select();
        let command = match oper.index() {
            i if i == chains_coordinator_commands_oper => {
                match oper.recv(&chains_coordinator_commands_rx) {
                    Ok(ChainsCoordinatorCommand::Terminate) => {
                        let _ = orchestrator_terminator_tx.send(true);
                        let _ = observer_command_tx.send(ObserverCommand::Terminate);
                        let _ = mining_command_tx.send(BitcoinMiningCommand::Pause);
                        break;
                    }
                    Err(_e) => {
                        continue;
                    }
                }
            }
            i if i == observer_event_oper => match oper.recv(&observer_event_rx) {
                Ok(cmd) => cmd,
                Err(_e) => {
                    continue;
                }
            },
            _ => unreachable!(),
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
            ObserverEvent::BitcoinChainEvent((chain_update, _)) => {
                // Contextual shortcut: Devnet is an environment under control,
                // with 1 miner. As such we will ignore Reorgs handling.
                let (log, status) = match &chain_update {
                    BitcoinChainEvent::ChainUpdatedWithBlocks(event) => {
                        let tip = event.new_blocks.last().unwrap();
                        let log = format!("Bitcoin block #{} received", tip.block_identifier.index);
                        let status =
                            format!("mining blocks (chaintip = #{})", tip.block_identifier.index);
                        (log, status)
                    }
                    BitcoinChainEvent::ChainUpdatedWithReorg(events) => {
                        let tip = events.blocks_to_apply.last().unwrap();
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
            ObserverEvent::StacksChainEvent((chain_event, _)) => {
                if should_deploy_protocol {
                    if let Some(block_identifier) = chain_event.get_latest_block_identifier() {
                        if block_identifier.index == 1 {
                            should_deploy_protocol = false;
                            if let Some(deployment_commands_tx) = deployment_commands_tx.take() {
                                deployment_commands_tx
                                    .send(DeploymentCommand::Start)
                                    .expect("unable to trigger deployment");
                            }
                        }
                    }
                }

                let known_tip = match &chain_event {
                    StacksChainEvent::ChainUpdatedWithBlocks(block) => {
                        match block.new_blocks.last() {
                            Some(known_tip) => known_tip.clone(),
                            None => unreachable!(),
                        }
                    }
                    StacksChainEvent::ChainUpdatedWithMicroblocks(_) => {
                        let _ = devnet_event_tx.send(DevnetEvent::StacksChainEvent(chain_event));
                        continue;
                        // TODO(lgalabru): good enough for now
                    }
                    StacksChainEvent::ChainUpdatedWithMicroblocksReorg(_) => {
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
                    name: format!("stacks-node 2.1",),
                    comment: format!(
                        "mining blocks (chaintip = #{})",
                        known_tip.block.block_identifier.index
                    ),
                }));
                let message = if known_tip.block.block_identifier.index == 0 {
                    format!(
                        "Genesis Stacks block anchored in Bitcoin block #{} includes {} transactions",
                        known_tip
                            .block
                            .metadata
                            .bitcoin_anchor_block_identifier
                            .index,
                        known_tip.block.transactions.len(),
                    )
                } else {
                    format!(
                        "Stacks block #{} anchored in Bitcoin block #{} includes {} transactions",
                        known_tip.block.block_identifier.index,
                        known_tip
                            .block
                            .metadata
                            .bitcoin_anchor_block_identifier
                            .index,
                        known_tip.block.transactions.len(),
                    )
                };
                let _ = devnet_event_tx.send(DevnetEvent::info(message));

                let stacks_rpc = StacksRpc::new(&config.consolidated_stacks_rpc_url());
                let _ = stacks_rpc.get_pox_info();

                let should_submit_pox_orders = known_tip.block.metadata.pox_cycle_position
                    == (known_tip.block.metadata.pox_cycle_length - 2);
                if should_submit_pox_orders {
                    let bitcoin_block_height = known_tip
                        .block
                        .metadata
                        .bitcoin_anchor_block_identifier
                        .index;
                    let res = publish_stacking_orders(
                        &config.devnet_config,
                        &config.accounts,
                        &config.services_map_hosts,
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
                if !boot_completed.load(Ordering::SeqCst) {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    let res = mine_bitcoin_block(
                        &config.services_map_hosts.bitcoin_node_host,
                        config.devnet_config.bitcoin_node_username.as_str(),
                        &config.devnet_config.bitcoin_node_password.as_str(),
                        &config.devnet_config.miner_btc_address.as_str(),
                    )
                    .await;
                    if let Err(e) = res {
                        let _ = devnet_event_tx.send(DevnetEvent::error(e));
                    }
                }
            }
            ObserverEvent::PredicateRegistered(hook) => {
                let message = format!("New hook \"{}\" registered", hook.name());
                let _ = devnet_event_tx.send(DevnetEvent::info(message));
            }
            ObserverEvent::PredicateDeregistered(_hook) => {}
            ObserverEvent::PredicatesTriggered(count) => {
                if count > 0 {
                    let _ = devnet_event_tx
                        .send(DevnetEvent::info(format!("{} hooks triggered", count)));
                }
            }
            ObserverEvent::Terminate => {
                break;
            }
            ObserverEvent::StacksChainMempoolEvent(mempool_event) => match mempool_event {
                StacksChainMempoolEvent::TransactionsAdmitted(transactions) => {
                    // Temporary UI patch
                    if config.devnet_config.enable_subnet_node && !subnet_initialized {
                        for tx in transactions.iter() {
                            if tx.tx_description.contains("::commit-block") {
                                let _ = devnet_event_tx.send(DevnetEvent::ServiceStatus(
                                    ServiceStatusData {
                                        order: 5,
                                        status: Status::Green,
                                        name: "subnet-node".into(),
                                        comment: format!("⚡️"),
                                    },
                                ));
                                subnet_initialized = true;
                                break;
                            }
                        }
                    }
                    for tx in transactions.into_iter() {
                        let _ = devnet_event_tx.send(DevnetEvent::MempoolAdmission(tx));
                    }
                }
                StacksChainMempoolEvent::TransactionDropped(ref _transactions) => {}
            },
            ObserverEvent::BitcoinPredicateTriggered(_) => {}
            ObserverEvent::StacksPredicateTriggered(_) => {}
            ObserverEvent::PredicateEnabled(_) => {}
        }
    }
    Ok(())
}

pub fn perform_protocol_deployment(
    manifest: &ProjectManifest,
    deployment: &DeploymentSpecification,
    deployment_event_tx: Sender<DeploymentEvent>,
    deployment_command_rx: Receiver<DeploymentCommand>,
    override_bitcoin_rpc_url: Option<String>,
    override_stacks_rpc_url: Option<String>,
) {
    let manifest = manifest.clone();
    let deployment = deployment.clone();

    let _ = hiro_system_kit::thread_named("Deployment execution").spawn(move || {
        apply_on_chain_deployment(
            &manifest,
            deployment,
            deployment_event_tx,
            deployment_command_rx,
            false,
            override_bitcoin_rpc_url,
            override_stacks_rpc_url,
        );
    });
}

pub fn relay_devnet_protocol_deployment(
    deployment_events_rx: Receiver<DeploymentEvent>,
    devnet_event_tx: &Sender<DevnetEvent>,
    bitcoin_mining_tx: Option<Sender<BitcoinMiningCommand>>,
    boot_completed: &Arc<AtomicBool>,
) {
    let devnet_event_tx = devnet_event_tx.clone();
    let boot_completed = boot_completed.clone();
    let _ = hiro_system_kit::thread_named("Deployment monitoring").spawn(move || {
        loop {
            let event = match deployment_events_rx.recv() {
                Ok(event) => event,
                Err(_e) => break,
            };
            match event {
                DeploymentEvent::TransactionUpdate(tracker) => {
                    if let TransactionStatus::Error(ref message) = tracker.status {
                        let _ = devnet_event_tx.send(DevnetEvent::error(message.into()));
                        break;
                    }
                }
                DeploymentEvent::Interrupted(_) => {
                    // Terminate
                    break;
                }
                DeploymentEvent::DeploymentCompleted => {
                    boot_completed.store(true, Ordering::SeqCst);
                    if let Some(bitcoin_mining_tx) = bitcoin_mining_tx {
                        let _ = devnet_event_tx.send(DevnetEvent::BootCompleted(bitcoin_mining_tx));
                    }
                    break;
                }
            }
        }
    });
}

pub async fn publish_stacking_orders(
    devnet_config: &DevnetConfig,
    accounts: &Vec<AccountConfig>,
    services_map_hosts: &ServicesMapHosts,
    fee_rate: u64,
    bitcoin_block_height: u32,
) -> Option<usize> {
    if devnet_config.pox_stacking_orders.len() == 0 {
        return None;
    }

    let stacks_node_rpc_url = format!("http://{}", &services_map_hosts.stacks_node_host);

    let mut transactions = 0;
    let pox_info: PoxInfo = reqwest::get(format!("{}/v2/pox", stacks_node_rpc_url))
        .await
        .expect("Unable to retrieve pox info")
        .json()
        .await
        .expect("Unable to parse contract");

    for pox_stacking_order in devnet_config.pox_stacking_orders.iter() {
        if pox_stacking_order.start_at_cycle - 1 == pox_info.reward_cycle_id {
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

            let _ = hiro_system_kit::thread_named("Stacking orders handler").spawn(move || {
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
                let stack_stx_tx = codec::build_contrat_call_transaction(
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
    _bitcoin_node_host: &str,
    _bitcoin_node_username: &str,
    _bitcoin_node_password: &str,
) {
    unimplemented!()
}

pub async fn mine_bitcoin_block(
    bitcoin_node_host: &str,
    bitcoin_node_username: &str,
    bitcoin_node_password: &str,
    miner_btc_address: &str,
) -> Result<(), String> {
    use bitcoincore_rpc::bitcoin::Address;
    use reqwest::Client as HttpClient;
    use serde_json::json;
    use std::str::FromStr;

    let miner_address = Address::from_str(miner_btc_address).unwrap();
    let _ = HttpClient::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("Unable to build http client")
        .post(format!("http://{}", bitcoin_node_host))
        .basic_auth(bitcoin_node_username, Some(bitcoin_node_password))
        .header("Content-Type", "application/json")
        .header("Host", bitcoin_node_host)
        .json(&serde_json::json!({
            "jsonrpc": "1.0",
            "id": "stacks-network",
            "method": "generatetoaddress",
            "params": [json!(1), json!(miner_address)]
        }))
        .send()
        .await
        .map_err(|e| format!("unable to send request ({})", e))?
        .json::<bitcoincore_rpc::jsonrpc::Response>()
        .await
        .map_err(|e| format!("unable to generate bitcoin block: ({})", e.to_string()))?;
    Ok(())
}

async fn handle_bitcoin_mining(
    mining_command_rx: Receiver<BitcoinMiningCommand>,
    config: &DevnetEventObserverConfig,
    devnet_event_tx: &Sender<DevnetEvent>,
) {
    let stop_miner = Arc::new(AtomicBool::new(false));
    loop {
        let command = match mining_command_rx.recv() {
            Ok(cmd) => cmd,
            Err(_e) => {
                // TODO(lgalabru): cascade termination
                break;
            }
        };
        match command {
            BitcoinMiningCommand::Start => {
                stop_miner.store(false, Ordering::SeqCst);
                let stop_miner_reader = stop_miner.clone();
                let devnet_event_tx_moved = devnet_event_tx.clone();
                let config_moved = config.clone();
                let _ =
                    hiro_system_kit::thread_named("Bitcoin mining runloop").spawn(move || loop {
                        std::thread::sleep(std::time::Duration::from_millis(
                            config_moved
                                .devnet_config
                                .bitcoin_controller_block_time
                                .into(),
                        ));
                        let future = mine_bitcoin_block(
                            &config_moved.services_map_hosts.bitcoin_node_host,
                            &config_moved.devnet_config.bitcoin_node_username,
                            &config_moved.devnet_config.bitcoin_node_password,
                            &config_moved.devnet_config.miner_btc_address,
                        );
                        let res = hiro_system_kit::nestable_block_on(future);
                        if let Err(e) = res {
                            let _ = devnet_event_tx_moved.send(DevnetEvent::error(e));
                        }
                        if stop_miner_reader.load(Ordering::SeqCst) {
                            break;
                        }
                    });
            }
            BitcoinMiningCommand::Pause => {
                stop_miner.store(true, Ordering::SeqCst);
            }
            BitcoinMiningCommand::Mine => {
                let res = mine_bitcoin_block(
                    &config.services_map_hosts.bitcoin_node_host,
                    config.devnet_config.bitcoin_node_username.as_str(),
                    &config.devnet_config.bitcoin_node_password.as_str(),
                    &config.devnet_config.miner_btc_address.as_str(),
                )
                .await;
                if let Err(e) = res {
                    let _ = devnet_event_tx.send(DevnetEvent::error(e));
                }
            }
            BitcoinMiningCommand::InvalidateChainTip => {
                invalidate_bitcoin_chain_tip(
                    &config.services_map_hosts.bitcoin_node_host,
                    &config.devnet_config.bitcoin_node_username.as_str(),
                    &config.devnet_config.bitcoin_node_password.as_str(),
                );
            }
        }
    }
}
