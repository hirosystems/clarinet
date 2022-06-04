#[allow(unused_imports)]
#[macro_use]
extern crate error_chain;

mod serde;

use clarinet_lib::bip39::{Language, Mnemonic};
use clarinet_lib::deployment;
use clarinet_lib::integrate::{self, DevnetEvent, DevnetOrchestrator};
use clarinet_lib::types::{
    compute_addresses, AccountConfig, DevnetConfigFile, PoxStackingOrder, ProjectManifest,
    DEFAULT_DERIVATION_PATH,
};
use orchestra_types::{
    BitcoinBlockData, BitcoinChainEvent, ChainUpdatedWithBlockData, StacksChainEvent, StacksNetwork
};

use core::panic;
use neon::prelude::*;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::{env, process};

type DevnetCallback = Box<dyn FnOnce(&Channel) + Send>;

struct StacksDevnet {
    tx: mpsc::Sender<DevnetCommand>,
    bitcoin_block_rx: mpsc::Receiver<BitcoinBlockData>,
    stacks_block_rx: mpsc::Receiver<ChainUpdatedWithBlockData>,
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
        manifest_path: String,
        logs_enabled: bool,
        _accounts: BTreeMap<String, AccountConfig>,
        devnet_overrides: DevnetConfigFile,
    ) -> Self
    where
        C: Context<'a>,
    {
        let (tx, rx) = mpsc::channel::<DevnetCommand>();
        let (meta_tx, meta_rx) = mpsc::channel();
        let (log_tx, _log_rx) = mpsc::channel();
        let (bitcoin_block_tx, bitcoin_block_rx) = mpsc::channel();
        let (stacks_block_tx, stacks_block_rx) = mpsc::channel();

        let channel = cx.channel();

        let manifest_path = get_manifest_path_or_exit(Some(manifest_path.into()));
        let manifest =
            ProjectManifest::from_path(&manifest_path).expect("Syntax error in Clarinet.toml.");
        let (deployment, _) =
            deployment::read_deployment_or_generate_default(&manifest, &StacksNetwork::Devnet)
                .expect("Unable to generate deployment");
        let devnet = DevnetOrchestrator::new(manifest, Some(devnet_overrides));

        let node_url = devnet.get_stacks_node_url();

        thread::spawn(move || {
            if let Ok(DevnetCommand::Start(callback)) = rx.recv() {
                // Start devnet
                let (devnet_events_rx, terminator_tx) =
                    match integrate::run_devnet(devnet, deployment, Some(log_tx), false) {
                        Ok((Some(devnet_events_rx), Some(terminator_tx), _)) => {
                            (devnet_events_rx, terminator_tx)
                        }
                        _ => std::process::exit(1),
                    };
                meta_tx
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
            if let Ok(ref devnet_rx) = meta_rx.recv() {
                while let Ok(ref event) = devnet_rx.recv() {
                    match event {
                        DevnetEvent::BitcoinChainEvent(
                            BitcoinChainEvent::ChainUpdatedWithBlock(block),
                        ) => {
                            bitcoin_block_tx
                                .send(block.clone())
                                .expect("Unable to transmit bitcoin block");
                        }
                        DevnetEvent::StacksChainEvent(StacksChainEvent::ChainUpdatedWithBlock(
                            block,
                        )) => {
                            stacks_block_tx
                                .send(block.clone())
                                .expect("Unable to transmit stacks block");
                        }
                        DevnetEvent::Log(log) => {
                            if logs_enabled {
                                println!("{:?}", log);
                            }
                        }
                        _ => {}
                    }
                }
            }
        });

        Self {
            tx,
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
        let manifest_path = cx.argument::<JsString>(0)?.value(&mut cx);

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

            let id = account_settings
                .get(&mut cx, "id")?
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

            let (stx_address, btc_address, _) =
                compute_addresses(&mnemonic, &derivation, &StacksNetwork::Devnet.get_networks());

            let account = AccountConfig {
                label,
                mnemonic,
                stx_address,
                btc_address,
                derivation,
                is_mainnet,
                balance: balance as u64,
            };
            genesis_accounts.insert(id, account);
        }

        let mut overrides = DevnetConfigFile::default();

        if let Ok(res) = devnet_settings
            .get(&mut cx, "orchestrator_port")?
            .downcast::<JsNumber, _>(&mut cx)
        {
            overrides.orchestrator_port = Some(res.value(&mut cx) as u16);
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
            .get(&mut cx, "postgres_database")?
            .downcast::<JsString, _>(&mut cx)
        {
            overrides.postgres_database = Some(res.value(&mut cx));
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

        if let Ok(res) = devnet_settings
            .get(&mut cx, "disable_electrum")?
            .downcast::<JsBoolean, _>(&mut cx)
        {
            overrides.disable_electrum = Some(res.value(&mut cx));
        } else {
            overrides.disable_electrum = Some(true);
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
            manifest_path,
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

        let block = match devnet.stacks_block_rx.recv() {
            Ok(obj) => obj.new_block,
            Err(err) => panic!("{:?}", err),
        };

        let js_block = serde::to_value(&mut cx, &block).expect("Unable to serialize block");

        Ok(js_block)
    }

    fn js_on_bitcoin_block(mut cx: FunctionContext) -> JsResult<JsValue> {
        let devnet = cx
            .this()
            .downcast_or_throw::<JsBox<StacksDevnet>, _>(&mut cx)?;

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

fn get_manifest_path_or_exit(path: Option<String>) -> PathBuf {
    if let Some(path) = path {
        let manifest_path = PathBuf::from(path);
        if !manifest_path.exists() {
            println!("Could not find Clarinet.toml");
            process::exit(1);
        }
        manifest_path
    } else {
        let mut current_dir = env::current_dir().unwrap();
        loop {
            current_dir.push("Clarinet.toml");

            if current_dir.exists() {
                break current_dir;
            }
            current_dir.pop();

            if !current_dir.pop() {
                println!("Could not find Clarinet.toml");
                process::exit(1);
            }
        }
    }
}
