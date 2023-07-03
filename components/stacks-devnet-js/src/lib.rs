#[allow(unused_imports)]
#[macro_use]
extern crate error_chain;

mod serde;

use clarinet_deployments::{get_default_deployment_path, load_deployment};
use clarinet_files::bip39::{Language, Mnemonic};
use clarinet_files::chainhook_types::StacksNetwork;
use clarinet_files::{
    compute_addresses, AccountConfig, DevnetConfigFile, FileLocation, PoxStackingOrder,
    ProjectManifest, DEFAULT_DERIVATION_PATH,
};
use stacks_network::chainhook_sdk::chainhook_types::{
    BitcoinChainEvent, BitcoinChainUpdatedWithBlocksData, StacksChainEvent,
    StacksChainUpdatedWithBlocksData,
};
use stacks_network::chains_coordinator::BitcoinMiningCommand;
use stacks_network::{self, DevnetEvent, DevnetOrchestrator};

use core::panic;
use neon::prelude::*;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::{env, process};

type DevnetCallback = Box<dyn FnOnce(&Channel) + Send>;

use clarinet_deployments::types::{DeploymentGenerationArtifacts, DeploymentSpecification};
use stacks_network::{do_run_local_devnet, ChainsCoordinatorCommand};

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
    let default_deployment_file_path = get_default_deployment_path(&manifest, network)?;
    let (deployment, artifacts) = if default_deployment_file_path.exists() {
        (
            load_deployment(manifest, &default_deployment_file_path)?,
            None,
        )
    } else {
        let future =
            clarinet_deployments::generate_default_deployment(manifest, network, false, None, None);

        let (deployment, artifacts) = hiro_system_kit::nestable_block_on(future)?;
        (deployment, Some(artifacts))
    };
    Ok((deployment, artifacts))
}

#[allow(dead_code)]
struct StacksDevnet {
    tx: mpsc::Sender<DevnetCommand>,
    termination_rx: mpsc::Receiver<bool>,
    devnet_ready_rx: mpsc::Receiver<Result<(), String>>,
    mining_tx: mpsc::Sender<BitcoinMiningCommand>,
    bitcoin_block_rx: mpsc::Receiver<BitcoinChainUpdatedWithBlocksData>,
    stacks_block_rx: mpsc::Receiver<StacksChainUpdatedWithBlocksData>,
    bitcoin_node_url: String,
    stacks_node_url: String,
    stacks_api_url: String,
    stacks_explorer_url: String,
    bitcoin_explorer_url: String,
}

enum DevnetCommand {
    Start(Option<DevnetCallback>),
    Stop(Option<DevnetCallback>),
}

impl Finalize for StacksDevnet {}

impl StacksDevnet {
    fn new<'a, C>(
        cx: &mut C,
        manifest_location: String,
        logs_enabled: bool,
        _accounts: BTreeMap<String, AccountConfig>,
        devnet_overrides: DevnetConfigFile,
    ) -> Self
    where
        C: Context<'a>,
    {
        let network_id = devnet_overrides.network_id.clone();
        let (tx, rx) = mpsc::channel::<DevnetCommand>();
        let (devnet_ready_tx, devnet_ready_rx) = mpsc::channel::<_>();
        let (meta_devnet_command_tx, meta_devnet_command_rx) = mpsc::channel();
        let (termination_tx, termination_rx) = mpsc::channel();

        let (relaying_mining_tx, relaying_mining_rx) = mpsc::channel::<BitcoinMiningCommand>();
        let (meta_mining_command_tx, meta_mining_command_rx) = mpsc::channel();

        let (log_tx, _log_rx) = mpsc::channel();
        let (bitcoin_block_tx, bitcoin_block_rx) = mpsc::channel();
        let (stacks_block_tx, stacks_block_rx) = mpsc::channel();

        let channel = cx.channel();

        let manifest_location = get_manifest_location_or_exit(Some(manifest_location.into()));
        let manifest = ProjectManifest::from_location(&manifest_location)
            .expect("Syntax error in Clarinet.toml.");
        let (deployment, _) =
            read_deployment_or_generate_default(&manifest, &StacksNetwork::Devnet)
                .expect("Unable to generate deployment");
        let devnet = match DevnetOrchestrator::new(manifest, Some(devnet_overrides), true) {
            Ok(devnet) => devnet,
            Err(message) => {
                if logs_enabled {
                    println!("Fatal error: {}", message);
                }
                std::process::exit(1);
            }
        };

        let (
            bitcoin_node_url,
            stacks_node_url,
            stacks_api_url,
            stacks_explorer_url,
            bitcoin_explorer_url,
        ) = devnet
            .network_config
            .as_ref()
            .and_then(|config| config.devnet.as_ref())
            .and_then(|devnet| {
                Some((
                    format!("http://localhost:{}", devnet.bitcoin_node_p2p_port),
                    format!("http://localhost:{}", devnet.stacks_node_rpc_port),
                    format!("http://localhost:{}", devnet.stacks_api_port),
                    format!("http://localhost:{}", devnet.stacks_explorer_port),
                    format!("http://localhost:{}", devnet.bitcoin_explorer_port),
                ))
            })
            .expect("unable to read config");

        thread::spawn(move || {
            let chains_coordinator_command_tx = loop {
                match rx.recv() {
                    Ok(DevnetCommand::Start(callback)) => {
                        // Start devnet
                        let res = hiro_system_kit::nestable_block_on(do_run_local_devnet(
                            devnet,
                            deployment,
                            &mut None,
                            Some(log_tx),
                            false,
                            stacks_network::Context::empty(),
                            termination_tx,
                            None,
                        ));

                        let (devnet_events_rx, chains_coordinator_command_tx) = match res {
                            Ok((
                                Some(devnet_events_rx),
                                _,
                                Some(chains_coordinator_command_tx),
                            )) => (devnet_events_rx, chains_coordinator_command_tx),
                            Err(e) => {
                                if logs_enabled {
                                    println!("Fatal error: {}", e);
                                }
                                return;
                            }
                            _ => unreachable!(),
                        };
                        meta_devnet_command_tx
                            .send(devnet_events_rx)
                            .expect("Unable to transmit event receiver");

                        if let Some(c) = callback {
                            c(&channel);
                        }
                        break chains_coordinator_command_tx;
                    }
                    Ok(DevnetCommand::Stop(callback)) => {
                        if let Some(c) = callback {
                            c(&channel);
                        }
                        return;
                    }
                    Err(e) => {
                        if logs_enabled {
                            println!("Fatal error: {}", e.to_string());
                        }
                        return;
                    }
                }
            };

            // Start run loop
            loop {
                let event = rx.recv();
                match event {
                    Ok(DevnetCommand::Stop(callback)) => {
                        let _ =
                            chains_coordinator_command_tx.send(ChainsCoordinatorCommand::Terminate);
                        if let Some(c) = callback {
                            c(&channel);
                        }
                        break;
                    }
                    Ok(DevnetCommand::Start(_)) => {}
                    Err(e) => {
                        if logs_enabled {
                            println!("Fatal error: {}", e.to_string());
                        }
                        return;
                    }
                }
            }
        });

        thread::spawn(move || {
            if let Ok(ref devnet_rx) = meta_devnet_command_rx.recv() {
                while let Ok(event) = devnet_rx.recv() {
                    match event {
                        DevnetEvent::BitcoinChainEvent(
                            BitcoinChainEvent::ChainUpdatedWithBlocks(update),
                        ) => {
                            bitcoin_block_tx
                                .send(update)
                                .expect("Unable to transmit bitcoin block");
                        }
                        DevnetEvent::StacksChainEvent(
                            StacksChainEvent::ChainUpdatedWithBlocks(update),
                        ) => {
                            stacks_block_tx
                                .send(update)
                                .expect("Unable to transmit stacks block");
                        }
                        DevnetEvent::Log(log) => {
                            if logs_enabled {
                                println!(
                                    "{} {}",
                                    log,
                                    match network_id {
                                        Some(network_id) => format!("(network #{})", network_id),
                                        None => "".into(),
                                    }
                                );
                            }
                        }
                        DevnetEvent::BootCompleted(mining_tx) => {
                            let _ = meta_mining_command_tx.send(mining_tx);
                            let _ = devnet_ready_tx.send(Ok(()));
                        }
                        DevnetEvent::FatalError(error) => {
                            let _ = devnet_ready_tx.send(Err(error.clone()));
                            if logs_enabled {
                                println!("[erro] {}", error);
                            }
                            break;
                        }
                        _ => {}
                    }
                }
            }
        });

        // Bitcoin mining command relaying - threading model 2
        thread::spawn(move || {
            let mut relayer_tx = None;
            while let Ok(command) = relaying_mining_rx.recv() {
                if relayer_tx.is_none() {
                    if let Ok(mining_tx) = meta_mining_command_rx.recv() {
                        relayer_tx = Some(mining_tx);
                    }
                }
                if let Some(ref tx) = relayer_tx {
                    let _ = tx.send(command);
                }
            }
        });

        Self {
            tx,
            termination_rx,
            devnet_ready_rx,
            mining_tx: relaying_mining_tx,
            bitcoin_block_rx,
            stacks_block_rx,
            bitcoin_node_url,
            stacks_node_url,
            stacks_api_url,
            stacks_explorer_url,
            bitcoin_explorer_url,
        }
    }

    fn start(&self, timeout: u64, _empty_buffer: bool) -> Result<bool, String> {
        let _ = self.tx.send(DevnetCommand::Start(None));
        let _ = self
            .devnet_ready_rx
            .recv_timeout(std::time::Duration::from_secs(timeout))
            .map_err(|e| format!("broken channel: {}", e.to_string()))??;
        Ok(true)
    }
}

impl StacksDevnet {
    fn js_new(mut cx: FunctionContext) -> JsResult<JsBox<StacksDevnet>> {
        let manifest_location = cx.argument::<JsString>(0)?.value(&mut cx);

        let logs_enabled = cx.argument::<JsBoolean>(1)?.value(&mut cx);

        let accounts = cx.argument::<JsArray>(2)?.to_vec(&mut cx)?;

        let devnet_settings = cx.argument::<JsObject>(3)?;

        let mut genesis_accounts = BTreeMap::new();

        for account in accounts.iter() {
            let account_settings = account.downcast_or_throw::<JsObject, _>(&mut cx)?;
            let label = account_settings
                .get(&mut cx, "label")?
                .downcast_or_throw::<JsString, _>(&mut cx)?
                .value(&mut cx);

            let words = account_settings
                .get(&mut cx, "mnemonic")?
                .downcast_or_throw::<JsString, _>(&mut cx)?
                .value(&mut cx);

            let mnemonic = Mnemonic::parse_in_normalized(Language::English, &words)
                .unwrap()
                .to_string();

            let balance = match account_settings
                .get(&mut cx, "balance")?
                .downcast::<JsNumber, _>(&mut cx)
            {
                Ok(res) => res.value(&mut cx),
                _ => 0.0,
            };

            let is_mainnet = match account_settings
                .get(&mut cx, "is_mainnet")?
                .downcast::<JsBoolean, _>(&mut cx)
            {
                Ok(res) => res.value(&mut cx),
                _ => false,
            };

            let derivation = match account_settings
                .get(&mut cx, "derivation")?
                .downcast::<JsString, _>(&mut cx)
            {
                Ok(res) => res.value(&mut cx),
                _ => DEFAULT_DERIVATION_PATH.to_string(),
            };

            let (stx_address, btc_address, _) = compute_addresses(
                &mnemonic,
                &derivation,
                &StacksNetwork::Devnet.get_networks(),
            );

            let account = AccountConfig {
                label: label.clone(),
                mnemonic: mnemonic.to_string(),
                stx_address,
                btc_address,
                derivation,
                is_mainnet,
                balance: balance as u64,
            };
            genesis_accounts.insert(label, account);
        }

        let mut overrides = DevnetConfigFile::default();

        if let Ok(res) = devnet_settings
            .get(&mut cx, "name")?
            .downcast::<JsString, _>(&mut cx)
        {
            overrides.name = Some(res.value(&mut cx));
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "network_id")?
            .downcast::<JsNumber, _>(&mut cx)
        {
            overrides.network_id = Some(res.value(&mut cx) as u16);
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "orchestrator_port")?
            .downcast::<JsNumber, _>(&mut cx)
        {
            overrides.orchestrator_port = Some(res.value(&mut cx) as u16);
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "orchestrator_control_port")?
            .downcast::<JsNumber, _>(&mut cx)
        {
            overrides.orchestrator_control_port = Some(res.value(&mut cx) as u16);
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "bitcoin_node_p2p_port")?
            .downcast::<JsNumber, _>(&mut cx)
        {
            overrides.bitcoin_node_p2p_port = Some(res.value(&mut cx) as u16);
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "bitcoin_node_rpc_port")?
            .downcast::<JsNumber, _>(&mut cx)
        {
            overrides.bitcoin_node_rpc_port = Some(res.value(&mut cx) as u16);
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "stacks_node_p2p_port")?
            .downcast::<JsNumber, _>(&mut cx)
        {
            overrides.stacks_node_p2p_port = Some(res.value(&mut cx) as u16);
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "stacks_node_rpc_port")?
            .downcast::<JsNumber, _>(&mut cx)
        {
            overrides.stacks_node_rpc_port = Some(res.value(&mut cx) as u16);
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "stacks_api_port")?
            .downcast::<JsNumber, _>(&mut cx)
        {
            overrides.stacks_api_port = Some(res.value(&mut cx) as u16);
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "stacks_api_events_port")?
            .downcast::<JsNumber, _>(&mut cx)
        {
            overrides.stacks_api_events_port = Some(res.value(&mut cx) as u16);
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "bitcoin_explorer_port")?
            .downcast::<JsNumber, _>(&mut cx)
        {
            overrides.bitcoin_explorer_port = Some(res.value(&mut cx) as u16);
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "stacks_explorer_port")?
            .downcast::<JsNumber, _>(&mut cx)
        {
            overrides.stacks_explorer_port = Some(res.value(&mut cx) as u16);
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "bitcoin_node_username")?
            .downcast::<JsString, _>(&mut cx)
        {
            overrides.bitcoin_node_username = Some(res.value(&mut cx));
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "bitcoin_node_password")?
            .downcast::<JsString, _>(&mut cx)
        {
            overrides.bitcoin_node_password = Some(res.value(&mut cx));
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "miner_mnemonic")?
            .downcast::<JsString, _>(&mut cx)
        {
            overrides.miner_mnemonic = Some(res.value(&mut cx));
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "miner_derivation_path")?
            .downcast::<JsString, _>(&mut cx)
        {
            overrides.miner_derivation_path = Some(res.value(&mut cx));
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "bitcoin_controller_block_time")?
            .downcast::<JsNumber, _>(&mut cx)
        {
            overrides.bitcoin_controller_block_time = Some(res.value(&mut cx) as u32);
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "working_dir")?
            .downcast::<JsString, _>(&mut cx)
        {
            overrides.working_dir = Some(res.value(&mut cx));
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "postgres_port")?
            .downcast::<JsNumber, _>(&mut cx)
        {
            overrides.postgres_port = Some(res.value(&mut cx) as u16);
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "postgres_username")?
            .downcast::<JsString, _>(&mut cx)
        {
            overrides.postgres_username = Some(res.value(&mut cx));
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "postgres_password")?
            .downcast::<JsString, _>(&mut cx)
        {
            overrides.postgres_password = Some(res.value(&mut cx));
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "bitcoin_node_image_url")?
            .downcast::<JsString, _>(&mut cx)
        {
            overrides.bitcoin_node_image_url = Some(res.value(&mut cx));
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "bitcoin_explorer_image_url")?
            .downcast::<JsString, _>(&mut cx)
        {
            overrides.bitcoin_explorer_image_url = Some(res.value(&mut cx));
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "stacks_node_image_url")?
            .downcast::<JsString, _>(&mut cx)
        {
            overrides.stacks_node_image_url = Some(res.value(&mut cx));
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "stacks_api_image_url")?
            .downcast::<JsString, _>(&mut cx)
        {
            overrides.stacks_api_image_url = Some(res.value(&mut cx));
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "stacks_explorer_image_url")?
            .downcast::<JsString, _>(&mut cx)
        {
            overrides.stacks_explorer_image_url = Some(res.value(&mut cx));
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "postgres_image_url")?
            .downcast::<JsString, _>(&mut cx)
        {
            overrides.postgres_image_url = Some(res.value(&mut cx));
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "bitcoin_controller_automining_disabled")?
            .downcast::<JsBoolean, _>(&mut cx)
        {
            overrides.bitcoin_controller_automining_disabled = Some(res.value(&mut cx));
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "bind_containers_volumes")?
            .downcast::<JsBoolean, _>(&mut cx)
        {
            overrides.bind_containers_volumes = Some(res.value(&mut cx));
        } else {
            overrides.bind_containers_volumes = Some(false);
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "use_docker_gateway_routing")?
            .downcast::<JsBoolean, _>(&mut cx)
        {
            overrides.use_docker_gateway_routing = Some(res.value(&mut cx));
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "epoch_2_0")?
            .downcast::<JsNumber, _>(&mut cx)
        {
            overrides.epoch_2_0 = Some(res.value(&mut cx) as u64);
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "epoch_2_05")?
            .downcast::<JsNumber, _>(&mut cx)
        {
            overrides.epoch_2_05 = Some(res.value(&mut cx) as u64);
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "epoch_2_1")?
            .downcast::<JsNumber, _>(&mut cx)
        {
            overrides.epoch_2_1 = Some(res.value(&mut cx) as u64);
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "epoch_2_2")?
            .downcast::<JsNumber, _>(&mut cx)
        {
            overrides.epoch_2_2 = Some(res.value(&mut cx) as u64);
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "epoch_2_3")?
            .downcast::<JsNumber, _>(&mut cx)
        {
            overrides.epoch_2_3 = Some(res.value(&mut cx) as u64);
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "epoch_2_4")?
            .downcast::<JsNumber, _>(&mut cx)
        {
            overrides.epoch_2_4 = Some(res.value(&mut cx) as u64);
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "pox_2_activation")?
            .downcast::<JsNumber, _>(&mut cx)
        {
            overrides.pox_2_activation = Some(res.value(&mut cx) as u64);
        }

        // Disable scripts
        overrides.execute_script = Some(vec![]);

        // Disable bitcoin_explorer, stacks_explorer and stacks_api by default:
        if let Ok(res) = devnet_settings
            .get(&mut cx, "disable_bitcoin_explorer")?
            .downcast::<JsBoolean, _>(&mut cx)
        {
            overrides.disable_bitcoin_explorer = Some(res.value(&mut cx));
        } else {
            overrides.disable_bitcoin_explorer = Some(true);
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "disable_stacks_explorer")?
            .downcast::<JsBoolean, _>(&mut cx)
        {
            overrides.disable_stacks_explorer = Some(res.value(&mut cx));
        } else {
            overrides.disable_stacks_explorer = Some(true);
        }

        if let Ok(res) = devnet_settings
            .get(&mut cx, "disable_stacks_api")?
            .downcast::<JsBoolean, _>(&mut cx)
        {
            overrides.disable_stacks_api = Some(res.value(&mut cx));
        } else {
            overrides.disable_stacks_api = Some(true);
        }

        // Disable bitcoin automining default:
        if let Ok(res) = devnet_settings
            .get(&mut cx, "bitcoin_controller_automining_disabled")?
            .downcast::<JsBoolean, _>(&mut cx)
        {
            overrides.bitcoin_controller_automining_disabled = Some(res.value(&mut cx));
        } else {
            overrides.bitcoin_controller_automining_disabled = Some(true);
        }

        // Retrieve stacks_node_events_observers
        if let Ok(res) = devnet_settings
            .get(&mut cx, "stacks_node_events_observers")?
            .downcast::<JsArray, _>(&mut cx)
        {
            let raw_events_observers = res.to_vec(&mut cx)?;
            let mut events_observers = vec![];

            for raw_events_observer in raw_events_observers.iter() {
                let observer_url = raw_events_observer
                    .downcast_or_throw::<JsString, _>(&mut cx)?
                    .value(&mut cx);
                events_observers.push(observer_url);
            }
            overrides.stacks_node_events_observers = Some(events_observers);
        }

        // Retrieve stacking_orders
        if let Ok(res) = devnet_settings
            .get(&mut cx, "pox_stacking_orders")?
            .downcast::<JsArray, _>(&mut cx)
        {
            let raw_stacking_orders = res.to_vec(&mut cx)?;
            let mut stacking_orders = vec![];

            for raw_stacking_order in raw_stacking_orders.iter() {
                let order_settings =
                    raw_stacking_order.downcast_or_throw::<JsObject, _>(&mut cx)?;

                let start_at_cycle = order_settings
                    .get(&mut cx, "start_at_cycle")?
                    .downcast_or_throw::<JsNumber, _>(&mut cx)?
                    .value(&mut cx) as u32;

                let duration = order_settings
                    .get(&mut cx, "duration")?
                    .downcast_or_throw::<JsNumber, _>(&mut cx)?
                    .value(&mut cx) as u32;

                let wallet = order_settings
                    .get(&mut cx, "wallet")?
                    .downcast_or_throw::<JsString, _>(&mut cx)?
                    .value(&mut cx);

                let slots = order_settings
                    .get(&mut cx, "slots")?
                    .downcast_or_throw::<JsNumber, _>(&mut cx)?
                    .value(&mut cx) as u64;

                let btc_address = order_settings
                    .get(&mut cx, "btc_address")?
                    .downcast_or_throw::<JsString, _>(&mut cx)?
                    .value(&mut cx);

                stacking_orders.push(PoxStackingOrder {
                    start_at_cycle,
                    duration,
                    wallet,
                    slots,
                    btc_address,
                });
            }
            overrides.pox_stacking_orders = Some(stacking_orders);
        }

        let devnet = StacksDevnet::new(
            &mut cx,
            manifest_location,
            logs_enabled,
            genesis_accounts,
            overrides,
        );
        Ok(cx.boxed(devnet))
    }

    fn js_start(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        let timeout = cx.argument::<JsNumber>(0)?.value(&mut cx) as u64;
        let empty_buffer = cx.argument::<JsBoolean>(1)?.value(&mut cx);

        cx.this()
            .downcast_or_throw::<JsBox<StacksDevnet>, _>(&mut cx)?
            .start(timeout, empty_buffer)
            .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    fn js_terminate(mut cx: FunctionContext) -> JsResult<JsBoolean> {
        let devnet = cx
            .this()
            .downcast_or_throw::<JsBox<StacksDevnet>, _>(&mut cx)?;

        if let Err(err) = devnet.tx.send(DevnetCommand::Stop(None)) {
            panic!("{}", err.to_string());
        };

        let gratecefully_terminated = match devnet.termination_rx.recv() {
            Ok(res) => res,
            Err(_) => false,
        };
        Ok(cx.boolean(gratecefully_terminated))
    }

    fn js_on_stacks_block(mut cx: FunctionContext) -> JsResult<JsValue> {
        let devnet = cx
            .this()
            .downcast_or_throw::<JsBox<StacksDevnet>, _>(&mut cx)?;
        let timeout = cx.argument::<JsNumber>(0)?.value(&mut cx) as u64;
        let empty_queued_blocks = cx.argument::<JsBoolean>(1)?.value(&mut cx) as bool;

        if empty_queued_blocks {
            loop {
                if devnet.stacks_block_rx.try_recv().is_err() {
                    break;
                }
            }
        }

        let _ = devnet.mining_tx.send(BitcoinMiningCommand::Mine);

        let blocks = match devnet
            .stacks_block_rx
            .recv_timeout(std::time::Duration::from_millis(timeout))
        {
            Ok(obj) => obj,
            Err(_) => return Ok(cx.undefined().as_value(&mut cx)),
        };

        let js_blocks = serde::to_value(&mut cx, &blocks).expect("Unable to serialize block");

        Ok(js_blocks)
    }

    fn js_on_bitcoin_block(mut cx: FunctionContext) -> JsResult<JsValue> {
        let devnet = cx
            .this()
            .downcast_or_throw::<JsBox<StacksDevnet>, _>(&mut cx)?;

        let _ = devnet.mining_tx.send(BitcoinMiningCommand::Mine);

        let block = match devnet
            .bitcoin_block_rx
            .recv_timeout(std::time::Duration::from_secs(10))
        {
            Ok(obj) => obj,
            Err(_) => return Ok(cx.undefined().as_value(&mut cx)),
        };

        let js_block = serde::to_value(&mut cx, &block).expect("Unable to serialize block");

        Ok(js_block)
    }

    fn js_get_bitcoin_node_url(mut cx: FunctionContext) -> JsResult<JsString> {
        let devnet = cx
            .this()
            .downcast_or_throw::<JsBox<StacksDevnet>, _>(&mut cx)?;

        let val = JsString::new(&mut cx, devnet.bitcoin_node_url.to_string());
        Ok(val)
    }

    fn js_get_stacks_node_url(mut cx: FunctionContext) -> JsResult<JsString> {
        let devnet = cx
            .this()
            .downcast_or_throw::<JsBox<StacksDevnet>, _>(&mut cx)?;

        let val = JsString::new(&mut cx, devnet.stacks_node_url.to_string());
        Ok(val)
    }

    fn js_get_bitcoin_explorer_url(mut cx: FunctionContext) -> JsResult<JsString> {
        let devnet = cx
            .this()
            .downcast_or_throw::<JsBox<StacksDevnet>, _>(&mut cx)?;

        let val = JsString::new(&mut cx, devnet.bitcoin_explorer_url.to_string());
        Ok(val)
    }

    fn js_get_stacks_explorer_url(mut cx: FunctionContext) -> JsResult<JsString> {
        let devnet = cx
            .this()
            .downcast_or_throw::<JsBox<StacksDevnet>, _>(&mut cx)?;

        let val = JsString::new(&mut cx, devnet.stacks_explorer_url.to_string());
        Ok(val)
    }

    fn js_get_stacks_api_url(mut cx: FunctionContext) -> JsResult<JsString> {
        let devnet = cx
            .this()
            .downcast_or_throw::<JsBox<StacksDevnet>, _>(&mut cx)?;

        let val = JsString::new(&mut cx, devnet.stacks_api_url.to_string());
        Ok(val)
    }
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("stacksDevnetNew", StacksDevnet::js_new)?;
    cx.export_function("stacksDevnetStart", StacksDevnet::js_start)?;
    cx.export_function("stacksDevnetTerminate", StacksDevnet::js_terminate)?;
    cx.export_function("stacksDevnetStop", StacksDevnet::js_terminate)?;
    cx.export_function(
        "stacksDevnetWaitForStacksBlock",
        StacksDevnet::js_on_stacks_block,
    )?;
    cx.export_function(
        "stacksDevnetWaitForBitcoinBlock",
        StacksDevnet::js_on_bitcoin_block,
    )?;
    cx.export_function(
        "stacksDevnetGetBitcoinNodeUrl",
        StacksDevnet::js_get_bitcoin_node_url,
    )?;
    cx.export_function(
        "stacksDevnetGetStacksNodeUrl",
        StacksDevnet::js_get_stacks_node_url,
    )?;
    cx.export_function(
        "stacksDevnetGetBitcoinExplorerUrl",
        StacksDevnet::js_get_bitcoin_explorer_url,
    )?;
    cx.export_function(
        "stacksDevnetGetStacksExplorerUrl",
        StacksDevnet::js_get_stacks_explorer_url,
    )?;
    cx.export_function(
        "stacksDevnetGetStacksApiUrl",
        StacksDevnet::js_get_stacks_api_url,
    )?;
    Ok(())
}

fn get_manifest_location(path: Option<String>) -> Option<FileLocation> {
    if let Some(path) = path {
        let manifest_path = PathBuf::from(path);
        if !manifest_path.exists() {
            return None;
        }
        Some(FileLocation::from_path(manifest_path))
    } else {
        let mut current_dir = env::current_dir().unwrap();
        loop {
            current_dir.push("Clarinet.toml");

            if current_dir.exists() {
                return Some(FileLocation::from_path(current_dir));
            }
            current_dir.pop();

            if !current_dir.pop() {
                return None;
            }
        }
    }
}

fn get_manifest_location_or_exit(path: Option<String>) -> FileLocation {
    match get_manifest_location(path) {
        Some(manifest_location) => manifest_location,
        None => {
            println!("Could not find Clarinet.toml");
            process::exit(1);
        }
    }
}
