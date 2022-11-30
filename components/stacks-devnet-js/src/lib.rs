#[allow(unused_imports)]
#[macro_use]
extern crate error_chain;

mod serde;

use chainhook_types::{
    BitcoinChainEvent, BitcoinChainUpdatedWithBlocksData, StacksChainEvent,
    StacksChainUpdatedWithBlocksData, StacksNetwork,
};
use clarinet_deployments::{get_default_deployment_path, load_deployment};
use clarinet_files::bip39::{Language, Mnemonic};
use clarinet_files::{
    compute_addresses, AccountConfig, DevnetConfigFile, FileLocation, PoxStackingOrder,
    ProjectManifest, DEFAULT_DERIVATION_PATH,
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

use std::sync::mpsc::Sender;

use clarinet_deployments::types::{DeploymentGenerationArtifacts, DeploymentSpecification};
use stacks_network::{do_run_devnet, ChainsCoordinatorCommand, LogData};

pub fn run_devnet(
    devnet: DevnetOrchestrator,
    deployment: DeploymentSpecification,
    log_tx: Option<Sender<LogData>>,
    display_dashboard: bool,
) -> Result<
    (
        Option<mpsc::Receiver<DevnetEvent>>,
        Option<mpsc::Sender<bool>>,
        Option<mpsc::Sender<ChainsCoordinatorCommand>>,
    ),
    String,
> {
    match hiro_system_kit::nestable_block_on(do_run_devnet(
        devnet,
        deployment,
        &mut None,
        log_tx,
        display_dashboard,
        None,
    )) {
        Err(_e) => std::process::exit(1),
        Ok(res) => Ok(res),
    }
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

struct StacksDevnet {
    tx: mpsc::Sender<DevnetCommand>,
    mining_tx: mpsc::Sender<BitcoinMiningCommand>,
    bitcoin_block_rx: mpsc::Receiver<BitcoinChainUpdatedWithBlocksData>,
    stacks_block_rx: mpsc::Receiver<StacksChainUpdatedWithBlocksData>,
    node_url: String,
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
        let (tx, rx) = mpsc::channel::<DevnetCommand>();
        let (meta_devnet_command_tx, meta_devnet_command_rx) = mpsc::channel();

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
        let devnet = match DevnetOrchestrator::new(manifest, Some(devnet_overrides)) {
            Ok(devnet) => devnet,
            Err(message) => {
                println!("{}", message);
                std::process::exit(1);
            }
        };

        let node_url = devnet.get_stacks_node_url();

        thread::spawn(move || {
            if let Ok(DevnetCommand::Start(callback)) = rx.recv() {
                // Start devnet
                let (devnet_events_rx, terminator_tx) =
                    match run_devnet(devnet, deployment, Some(log_tx), false) {
                        Ok((Some(devnet_events_rx), Some(terminator_tx), _)) => {
                            (devnet_events_rx, terminator_tx)
                        }
                        _ => std::process::exit(1),
                    };
                meta_devnet_command_tx
                    .send(devnet_events_rx)
                    .expect("Unable to transmit event receiver");

                if let Some(c) = callback {
                    c(&channel);
                }

                // Start run loop
                while let Ok(message) = rx.recv() {
                    match message {
                        DevnetCommand::Stop(callback) => {
                            terminator_tx
                                .send(true)
                                .expect("Unable to terminate Devnet");
                            if let Some(c) = callback {
                                c(&channel);
                            }
                            break;
                        }
                        DevnetCommand::Start(_) => break,
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
                                println!("{:?}", log);
                            }
                        }
                        DevnetEvent::BootCompleted(mining_tx) => {
                            let _ = meta_mining_command_tx.send(mining_tx);
                        }
                        _ => {}
                    }
                }
            }
        });

        // Bitcoin mining command relaying - threading model 1
        // Keeping this model around, for eventual future usage
        // thread::spawn(move || {
        //     if let Ok(ref mining_tx) = meta_mining_command_rx.recv() {
        //         while let Ok(command) = relaying_mining_rx.recv() {
        //             let _ = mining_tx.send(command);
        //         }
        //     }
        // });

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
            mining_tx: relaying_mining_tx,
            bitcoin_block_rx,
            stacks_block_rx,
            node_url,
        }
    }

    fn start(
        &self,
        callback: Option<DevnetCallback>,
    ) -> Result<(), mpsc::SendError<DevnetCommand>> {
        self.tx.send(DevnetCommand::Start(callback))
    }

    fn stop(&self, callback: Option<DevnetCallback>) -> Result<(), mpsc::SendError<DevnetCommand>> {
        self.tx.send(DevnetCommand::Stop(callback))
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
            .get(&mut cx, "enable_next_features")?
            .downcast::<JsBoolean, _>(&mut cx)
        {
            overrides.enable_next_features = Some(res.value(&mut cx));
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
        // Get the first argument as a `JsFunction`
        // let callback = cx.argument::<JsFunction>(0)?.root(&mut cx);
        // let callback = callback.into_inner(&mut cx);

        cx.this()
            .downcast_or_throw::<JsBox<StacksDevnet>, _>(&mut cx)?
            .start(None)
            .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    fn js_stop(mut cx: FunctionContext) -> JsResult<JsUndefined> {
        cx.this()
            .downcast_or_throw::<JsBox<StacksDevnet>, _>(&mut cx)?
            .stop(None)
            .or_else(|err| cx.throw_error(err.to_string()))?;

        Ok(cx.undefined())
    }

    fn js_on_stacks_block(mut cx: FunctionContext) -> JsResult<JsValue> {
        let devnet = cx
            .this()
            .downcast_or_throw::<JsBox<StacksDevnet>, _>(&mut cx)?;

        // Keeping, for eventual future usage
        // let _ = devnet.mining_tx.send(BitcoinMiningCommand::Mine);

        let blocks = match devnet.stacks_block_rx.recv() {
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

        // Keeping, for eventual future usage
        // let _ = devnet.mining_tx.send(BitcoinMiningCommand::Mine);

        let block = match devnet.bitcoin_block_rx.recv() {
            Ok(obj) => obj,
            Err(err) => panic!("{:?}", err),
        };

        let js_block = serde::to_value(&mut cx, &block).expect("Unable to serialize block");

        Ok(js_block)
    }

    fn js_get_stacks_node_url(mut cx: FunctionContext) -> JsResult<JsString> {
        let devnet = cx
            .this()
            .downcast_or_throw::<JsBox<StacksDevnet>, _>(&mut cx)?;

        let val = JsString::new(&mut cx, devnet.node_url.to_string());
        Ok(val)
    }
}

#[neon::main]
fn main(mut cx: ModuleContext) -> NeonResult<()> {
    cx.export_function("stacksDevnetNew", StacksDevnet::js_new)?;
    cx.export_function("stacksDevnetStart", StacksDevnet::js_start)?;
    cx.export_function("stacksDevnetStop", StacksDevnet::js_stop)?;
    cx.export_function(
        "stacksDevnetWaitForStacksBlock",
        StacksDevnet::js_on_stacks_block,
    )?;
    cx.export_function(
        "stacksDevnetWaitForBitcoinBlock",
        StacksDevnet::js_on_bitcoin_block,
    )?;
    cx.export_function(
        "stacksDevnetGetStacksNodeUrl",
        StacksDevnet::js_get_stacks_node_url,
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
