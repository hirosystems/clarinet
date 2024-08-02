use super::ChainsCoordinatorCommand;

use crate::event::send_status_update;
use crate::event::DevnetEvent;
use crate::event::Status;
use crate::orchestrator::ServicesMapHosts;

use base58::FromBase58;
use bitcoincore_rpc::bitcoin::Address;
use chainhook_sdk::chainhooks::types::ChainhookStore;
use chainhook_sdk::observer::{
    start_event_observer, EventObserverConfig, ObserverCommand, ObserverEvent,
    StacksChainMempoolEvent,
};
use chainhook_sdk::types::BitcoinBlockSignaling;
use chainhook_sdk::types::BitcoinChainEvent;
use chainhook_sdk::types::StacksChainEvent;
use chainhook_sdk::types::StacksNodeConfig;
use chainhook_sdk::types::{BitcoinNetwork, StacksNetwork};
use chainhook_sdk::utils::Context;
use clarinet_deployments::onchain::TransactionStatus;
use clarinet_deployments::onchain::{
    apply_on_chain_deployment, DeploymentCommand, DeploymentEvent,
};
use clarinet_deployments::types::DeploymentSpecification;
use clarinet_files::PoxStackingOrder;
use clarinet_files::DEFAULT_FIRST_BURN_HEADER_HEIGHT;
use clarinet_files::{self, AccountConfig, DevnetConfig, NetworkManifest, ProjectManifest};
use clarity::address::AddressHashMode;
use clarity::types::PublicKey;
use clarity::util::hash::{hex_bytes, Hash160};
use clarity::vm::types::{BuffData, SequenceData, TupleData};
use clarity::vm::ClarityName;
use clarity::vm::Value as ClarityValue;
use hiro_system_kit;
use hiro_system_kit::slog;
use hiro_system_kit::yellow;
use serde_json::json;
use stacks_rpc_client::rpc_client::PoxInfo;
use stacks_rpc_client::StacksRpc;
use stackslib::chainstate::stacks::address::PoxAddress;
use stackslib::core::CHAIN_ID_TESTNET;
use stackslib::types::chainstate::StacksPrivateKey;
use stackslib::types::chainstate::StacksPublicKey;
use stackslib::util_lib::signed_structured_data::pox4::make_pox_4_signer_key_signature;
use stackslib::util_lib::signed_structured_data::pox4::Pox4SignatureTopic;
use std::convert::TryFrom;
use std::str;
use std::str::FromStr;
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
    pub network_manifest: NetworkManifest,
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
        network_manifest: Option<NetworkManifest>,
        deployment: DeploymentSpecification,
        chainhooks: ChainhookStore,
        ctx: &Context,
        services_map_hosts: ServicesMapHosts,
    ) -> Self {
        ctx.try_log(|logger| slog::info!(logger, "Checking contracts"));
        let network_manifest = match network_manifest {
            Some(n) => n,
            None => match NetworkManifest::from_project_manifest_location(
                &manifest.location,
                &StacksNetwork::Devnet.get_networks(),
                Some(&manifest.project.cache_location),
                None,
            ) {
                Ok(manifest) => manifest,
                Err(_err) => NetworkManifest::default(),
            },
        };
        let event_observer_config = EventObserverConfig {
            bitcoin_rpc_proxy_enabled: true,
            registered_chainhooks: chainhooks,
            bitcoind_rpc_username: devnet_config.bitcoin_node_username.clone(),
            bitcoind_rpc_password: devnet_config.bitcoin_node_password.clone(),
            bitcoind_rpc_url: format!("http://{}", services_map_hosts.bitcoin_node_host),
            bitcoin_block_signaling: BitcoinBlockSignaling::Stacks(StacksNodeConfig {
                rpc_url: format!("http://{}", services_map_hosts.stacks_node_host),
                ingestion_port: devnet_config.orchestrator_ingestion_port,
            }),

            display_stacks_ingestion_logs: true,
            bitcoin_network: BitcoinNetwork::Regtest,
            stacks_network: StacksNetwork::Devnet,
            prometheus_monitoring_port: None,
        };

        DevnetEventObserverConfig {
            devnet_config,
            event_observer_config,
            accounts: network_manifest
                .accounts
                .clone()
                .into_values()
                .collect::<Vec<_>>(),
            manifest,
            deployment,
            deployment_fee_rate: network_manifest.network.deployment_fee_rate,
            services_map_hosts,
            network_manifest,
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
    mining_command_tx: Sender<BitcoinMiningCommand>,
    mining_command_rx: Receiver<BitcoinMiningCommand>,
    ctx: Context,
) -> Result<(), String> {
    let mut should_deploy_protocol = true; // Will change when `stacks-network` components becomes compatible with Testnet / Mainnet setups
    let boot_completed = Arc::new(AtomicBool::new(false));

    let (deployment_commands_tx, deployments_command_rx) = channel();
    let (deployment_events_tx, deployment_events_rx) = channel();

    // Set-up the background task in charge of serializing / signing / publishing the contracts.
    // This tasks can take several seconds to minutes, depending on the complexity of the project.
    // We start this process as soon as possible, as a background task.
    // This thread becomes dormant once the encoding is done, and proceed to the actual deployment once
    // the event DeploymentCommand::Start is received.
    perform_protocol_deployment(
        &config.network_manifest,
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

    let chainhooks_count = config
        .event_observer_config
        .registered_chainhooks
        .stacks_chainhooks
        .len()
        + config
            .event_observer_config
            .registered_chainhooks
            .bitcoin_chainhooks
            .len();
    if chainhooks_count > 0 {
        devnet_event_tx
            .send(DevnetEvent::info(format!(
                "{chainhooks_count} chainhooks registered",
            )))
            .expect("Unable to terminate event observer");
    }

    // Spawn event observer
    let (observer_event_tx, observer_event_rx) = crossbeam_channel::unbounded();
    let event_observer_config = config.event_observer_config.clone();
    let observer_event_tx_moved = observer_event_tx.clone();
    let observer_command_tx_moved = observer_command_tx.clone();
    let ctx_moved = ctx.clone();
    let _ = hiro_system_kit::thread_named("Event observer").spawn(move || {
        let _ = start_event_observer(
            event_observer_config,
            observer_command_tx_moved,
            observer_command_rx,
            Some(observer_event_tx_moved),
            None,
            None,
            ctx_moved,
        );
    });

    // Spawn bitcoin miner controller
    let devnet_event_tx_moved = devnet_event_tx.clone();
    let devnet_config = config.clone();
    let _ = hiro_system_kit::thread_named("Bitcoin mining").spawn(move || {
        let future =
            handle_bitcoin_mining(mining_command_rx, &devnet_config, &devnet_event_tx_moved);
        hiro_system_kit::nestable_block_on(future);
    });

    // Loop over events being received from Bitcoin and Stacks,
    // and orchestrate the 2 chains + protocol.
    let mut deployment_commands_tx = Some(deployment_commands_tx);
    let mut subnet_initialized = false;

    let mut sel = crossbeam_channel::Select::new();
    let chains_coordinator_commands_oper = sel.recv(&chains_coordinator_commands_rx);
    let observer_event_oper = sel.recv(&observer_event_rx);

    let DevnetConfig {
        enable_subnet_node, ..
    } = config.devnet_config;

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
            ObserverEvent::PredicateInterrupted(_data) => {
                devnet_event_tx
                    .send(DevnetEvent::error("predicate interrupt".to_string())) // need to use data for the message
                    .expect("Event observer received predicate interrupt");
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
                let (log, comment) = match &chain_update {
                    BitcoinChainEvent::ChainUpdatedWithBlocks(event) => {
                        let tip = event.new_blocks.last().unwrap();
                        let bitcoin_block_height = tip.block_identifier.index;
                        let log = format!("Bitcoin block #{} received", bitcoin_block_height);
                        let comment =
                            format!("mining blocks (chain_tip = #{})", bitcoin_block_height);

                        // Stacking orders can't be published until devnet is ready
                        if bitcoin_block_height >= DEFAULT_FIRST_BURN_HEADER_HEIGHT + 10 {
                            let res = publish_stacking_orders(
                                &config.devnet_config,
                                &devnet_event_tx,
                                &config.accounts,
                                &config.services_map_hosts,
                                config.deployment_fee_rate,
                                bitcoin_block_height as u32,
                            )
                            .await;
                            if let Some(tx_count) = res {
                                let _ = devnet_event_tx.send(DevnetEvent::success(format!(
                                    "Broadcasted {} stacking orders",
                                    tx_count
                                )));
                            }
                        }

                        (log, comment)
                    }
                    BitcoinChainEvent::ChainUpdatedWithReorg(events) => {
                        let tip = events.blocks_to_apply.last().unwrap();
                        let log = format!(
                            "Bitcoin reorg received (new height: {})",
                            tip.block_identifier.index
                        );
                        let status = format!(
                            "mining blocks (chain_tip = #{})",
                            tip.block_identifier.index
                        );
                        (log, status)
                    }
                };

                let _ = devnet_event_tx.send(DevnetEvent::debug(log));

                send_status_update(
                    &devnet_event_tx,
                    enable_subnet_node,
                    "bitcoin-node",
                    Status::Green,
                    &comment,
                );
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
                                    .map_err(|e| format!("unable to start deployment: {}", e))
                                    .unwrap();
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
                send_status_update(
                    &devnet_event_tx,
                    enable_subnet_node,
                    "stacks-node",
                    Status::Green,
                    &format!(
                        "mining blocks (chain_tip = #{})",
                        known_tip.block.block_identifier.index
                    ),
                );

                // devnet_event_tx.send(DevnetEvent::send_status_update(status_update_data));

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
                        "Stacks block #{} mined including {} transaction{}",
                        known_tip.block.block_identifier.index,
                        known_tip.block.transactions.len(),
                        if known_tip.block.transactions.len() <= 1 {
                            ""
                        } else {
                            "s"
                        },
                    )
                };
                let _ = devnet_event_tx.send(DevnetEvent::info(message));
            }
            ObserverEvent::NotifyBitcoinTransactionProxied => {
                if !boot_completed.load(Ordering::SeqCst) {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    let res = mine_bitcoin_block(
                        &config.services_map_hosts.bitcoin_node_host,
                        config.devnet_config.bitcoin_node_username.as_str(),
                        config.devnet_config.bitcoin_node_password.as_str(),
                        config.devnet_config.miner_btc_address.as_str(),
                    )
                    .await;
                    if let Err(e) = res {
                        let _ = devnet_event_tx.send(DevnetEvent::error(e));
                    }
                }
            }
            ObserverEvent::PredicateRegistered(hook) => {
                let message = format!("New hook \"{}\" registered", hook.key());
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
                                send_status_update(
                                    &devnet_event_tx,
                                    enable_subnet_node,
                                    "subnet-node",
                                    Status::Green,
                                    "⚡️",
                                );
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
    network_manifest: &NetworkManifest,
    deployment: &DeploymentSpecification,
    deployment_event_tx: Sender<DeploymentEvent>,
    deployment_command_rx: Receiver<DeploymentCommand>,
    override_bitcoin_rpc_url: Option<String>,
    override_stacks_rpc_url: Option<String>,
) {
    let deployment = deployment.clone();
    let network_manifest = network_manifest.clone();
    let _ = hiro_system_kit::thread_named("Deployment execution").spawn(move || {
        apply_on_chain_deployment(
            network_manifest,
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

fn should_publish_stacking_orders(
    current_cycle: &u32,
    pox_stacking_order: &PoxStackingOrder,
) -> bool {
    let PoxStackingOrder {
        duration,
        start_at_cycle,
        ..
    } = pox_stacking_order;

    let is_higher_than_start_cycle = *current_cycle >= (start_at_cycle - 1);
    if !is_higher_than_start_cycle {
        return false;
    }

    let offset = (current_cycle + duration).saturating_sub(*start_at_cycle);
    let should_stack = (offset % duration) == (duration - 1);
    if !should_stack {
        return false;
    }

    true
}

#[allow(clippy::items_after_test_module)]
#[cfg(test)]
mod tests {
    use super::*;

    fn build_pox_stacking_order(duration: u32, start_at_cycle: u32) -> PoxStackingOrder {
        PoxStackingOrder {
            duration,
            start_at_cycle,
            wallet: "wallet_1".to_string(),
            slots: 1,
            btc_address: "address_1".to_string(),
            auto_extend: Some(true),
        }
    }

    #[test]
    fn test_should_publish_stacking_orders_basic() {
        let pox_stacking_order = build_pox_stacking_order(12, 6);

        // cycle just before start_at_cycle
        assert!(should_publish_stacking_orders(&5, &pox_stacking_order));

        // cycle before start_at_cycle + duration
        assert!(should_publish_stacking_orders(&17, &pox_stacking_order),);

        // cycle before start_at_cycle + duration * 42
        assert!(should_publish_stacking_orders(&509, &pox_stacking_order));

        // cycle equal to start_at_cycle
        assert!(!should_publish_stacking_orders(&6, &pox_stacking_order));

        // cycle after to start_at_cycle
        assert!(!should_publish_stacking_orders(&8, &pox_stacking_order));
    }

    #[test]
    fn test_should_publish_stacking_orders_edge_cases() {
        // duration is one cycle
        let pox_stacking_order = build_pox_stacking_order(1, 4);
        assert!(!should_publish_stacking_orders(&2, &pox_stacking_order));

        for i in 3..=20 {
            assert!(should_publish_stacking_orders(&i, &pox_stacking_order));
        }

        // duration is low and start_at_cycle is high
        let pox_stacking_order = build_pox_stacking_order(2, 100);
        for i in 0..=98 {
            assert!(!should_publish_stacking_orders(&i, &pox_stacking_order));
        }
        assert!(should_publish_stacking_orders(&99, &pox_stacking_order));
        assert!(!should_publish_stacking_orders(&100, &pox_stacking_order));
        assert!(should_publish_stacking_orders(&101, &pox_stacking_order));
    }
}

pub async fn publish_stacking_orders(
    devnet_config: &DevnetConfig,
    devnet_event_tx: &Sender<DevnetEvent>,
    accounts: &[AccountConfig],
    services_map_hosts: &ServicesMapHosts,
    fee_rate: u64,
    bitcoin_block_height: u32,
) -> Option<usize> {
    let node_rpc_url = format!("http://{}", &services_map_hosts.stacks_node_host);
    let pox_info: PoxInfo = match reqwest::get(format!("{}/v2/pox", node_rpc_url)).await {
        Ok(result) => match result.json().await {
            Ok(pox_info) => Some(pox_info),
            Err(e) => {
                let _ = devnet_event_tx.send(DevnetEvent::warning(format!(
                    "Unable to parse pox info: {}",
                    e
                )));

                None
            }
        },
        Err(e) => {
            let _ = devnet_event_tx.send(DevnetEvent::warning(format!(
                "unable to retrieve pox info: {}",
                e
            )));
            None
        }
    }?;

    let effective_height =
        u32::saturating_sub(bitcoin_block_height, pox_info.first_burnchain_block_height);

    let current_cycle = effective_height / pox_info.reward_cycle_length;
    let pox_cycle_length = pox_info.reward_cycle_length;
    let pox_cycle_position = effective_height % pox_cycle_length;

    if pox_cycle_position != 10 {
        return None;
    }

    let pox_contract_id = pox_info.contract_id;
    let pox_version = pox_contract_id
        .rsplit('-')
        .next()
        .and_then(|version| version.parse().ok())
        .unwrap_or(1); // pox 1 contract is `pox.clar`

    let default_signing_keys = [
        StacksPrivateKey::from_hex(
            "7287ba251d44a4d3fd9276c88ce34c5c52a038955511cccaf77e61068649c17801",
        )
        .unwrap(),
        StacksPrivateKey::from_hex(
            "530d9f61984c888536871c6573073bdfc0058896dc1adfe9a6a10dfacadc209101",
        )
        .unwrap(),
    ];

    let mut transactions = 0;
    for (i, pox_stacking_order) in devnet_config.pox_stacking_orders.iter().enumerate() {
        if !should_publish_stacking_orders(&current_cycle, pox_stacking_order) {
            continue;
        }

        // if the is not the first cycle of this stacker, then stacking order will be extended
        let extend_stacking = current_cycle != pox_stacking_order.start_at_cycle - 1;
        if extend_stacking && !pox_stacking_order.auto_extend.unwrap_or_default() {
            continue;
        }

        let account = match accounts
            .iter()
            .find(|e| e.label == pox_stacking_order.wallet)
            .cloned()
        {
            Some(account) => account,
            _ => continue,
        };

        transactions += 1;

        let stx_amount = pox_info.next_cycle.min_threshold_ustx * pox_stacking_order.slots;

        let node_rpc_url_moved = node_rpc_url.clone();
        let pox_contract_id_moved = pox_contract_id.clone();
        let btc_address_moved = pox_stacking_order.btc_address.clone();
        let duration = pox_stacking_order.duration;

        let stacking_result =
            hiro_system_kit::thread_named("Stacking orders handler").spawn(move || {
                let default_fee = fee_rate * 1000;
                let stacks_rpc = StacksRpc::new(&node_rpc_url_moved);
                let nonce = stacks_rpc.get_nonce(&account.stx_address)?;

                let (_, _, account_secret_key) = clarinet_files::compute_addresses(
                    &account.mnemonic,
                    &account.derivation,
                    &StacksNetwork::Devnet.get_networks(),
                );

                let (method, arguments) = get_stacking_tx_method_and_args(
                    pox_version,
                    bitcoin_block_height,
                    current_cycle.into(),
                    &default_signing_keys[i % 2],
                    extend_stacking,
                    &btc_address_moved,
                    stx_amount,
                    duration,
                    i.try_into().unwrap(),
                );

                let tx = stacks_codec::codec::build_contract_call_transaction(
                    pox_contract_id_moved,
                    method,
                    arguments,
                    nonce,
                    default_fee,
                    &hex_bytes(&account_secret_key).unwrap(),
                );

                stacks_rpc.post_transaction(&tx)
            });

        match stacking_result {
            Ok(result) => {
                if let Ok(result) = result.join() {
                    match result {
                        Ok(_) => {
                            let _ = devnet_event_tx.send(DevnetEvent::success(format!(
                                "Stacking order for {} STX submitted",
                                stx_amount
                            )));
                        }
                        Err(e) => {
                            let _ = devnet_event_tx
                                .send(DevnetEvent::error(format!("Unable to stack: {}", e)));
                        }
                    }
                };
            }
            Err(e) => {
                let _ = devnet_event_tx.send(DevnetEvent::error(format!("Unable to stack: {}", e)));
            }
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
    let miner_address = Address::from_str(miner_btc_address).unwrap();
    let _ = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
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
        .map_err(|e| format!("unable to generate bitcoin block: ({})", e))?;
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
            Err(e) => {
                print!("{} {}", yellow!("unexpected error:"), e);
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
                        if stop_miner_reader.load(Ordering::SeqCst) {
                            break;
                        }
                        if let Err(e) = res {
                            let _ = devnet_event_tx_moved.send(DevnetEvent::error(e));
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
                    config.devnet_config.bitcoin_node_password.as_str(),
                    config.devnet_config.miner_btc_address.as_str(),
                )
                .await;
                if let Err(e) = res {
                    let _ = devnet_event_tx.send(DevnetEvent::error(e));
                }
            }
            BitcoinMiningCommand::InvalidateChainTip => {
                invalidate_bitcoin_chain_tip(
                    &config.services_map_hosts.bitcoin_node_host,
                    config.devnet_config.bitcoin_node_username.as_str(),
                    config.devnet_config.bitcoin_node_password.as_str(),
                );
            }
        }
    }
}

fn get_stacking_tx_method_and_args(
    pox_version: u32,
    bitcoin_block_height: u32,
    cycle: u128,
    signer_key: &StacksPrivateKey,
    extend_stacking: bool,
    btc_address: &str,
    stx_amount: u64,
    duration: u32,
    auth_id: u128,
) -> (String, Vec<ClarityValue>) {
    let addr_bytes = btc_address
        .from_base58()
        .expect("Unable to get bytes from btc address");
    let pox_addr_tuple = ClarityValue::Tuple(
        TupleData::from_data(vec![
            (
                ClarityName::try_from("version".to_owned()).unwrap(),
                ClarityValue::buff_from_byte(AddressHashMode::SerializeP2PKH as u8),
            ),
            (
                ClarityName::try_from("hashbytes".to_owned()).unwrap(),
                ClarityValue::Sequence(SequenceData::Buffer(BuffData {
                    data: Hash160::from_bytes(&addr_bytes[1..21])
                        .unwrap()
                        .as_bytes()
                        .to_vec(),
                })),
            ),
        ])
        .unwrap(),
    );
    let pox_addr = PoxAddress::try_from_pox_tuple(false, &pox_addr_tuple).unwrap();

    let burn_block_height: u128 = (bitcoin_block_height - 1).into();

    let method = if extend_stacking {
        "stack-extend"
    } else {
        "stack-stx"
    };

    let mut arguments = if extend_stacking {
        vec![ClarityValue::UInt(duration.into()), pox_addr_tuple]
    } else {
        vec![
            ClarityValue::UInt(stx_amount.into()),
            pox_addr_tuple,
            ClarityValue::UInt(burn_block_height),
            ClarityValue::UInt(duration.into()),
        ]
    };

    if pox_version >= 4 {
        // extra arguments for pox-4 (for both stack-stx and stack-extend)
        //   (signer-sig (optional (buff 65)))
        //   (signer-key (buff 33))
        //   (max-amount uint)
        //   (auth-id uint)
        let topic = if extend_stacking {
            Pox4SignatureTopic::StackExtend
        } else {
            Pox4SignatureTopic::StackStx
        };

        let signature = make_pox_4_signer_key_signature(
            &pox_addr,
            signer_key,
            cycle,
            &topic,
            CHAIN_ID_TESTNET,
            duration.into(),
            stx_amount.into(),
            auth_id,
        )
        .expect("Unable to make pox 4 signature");

        let signer_sig = signature.to_rsv();

        let pub_key = StacksPublicKey::from_private(signer_key);
        arguments.push(ClarityValue::some(ClarityValue::buff_from(signer_sig).unwrap()).unwrap());
        arguments.push(ClarityValue::buff_from(pub_key.to_bytes()).unwrap());
        arguments.push(ClarityValue::UInt(stx_amount.into()));
        arguments.push(ClarityValue::UInt(auth_id));
    };

    (method.to_string(), arguments)
}
