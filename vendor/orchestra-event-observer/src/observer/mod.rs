use crate::indexer::{chains, Indexer, IndexerConfig};
use crate::utils;
use orchestra_types::{BitcoinChainEvent, StacksNetwork, StacksChainEvent};
use crate::hooks::types::{HookFormation, HookSpecification};
use crate::hooks::{evaluate_stacks_hooks_on_chain_event, evaluate_bitcoin_hooks_on_chain_event, handle_stacks_hook_action, handle_bitcoin_hook_action};
use stacks_rpc_client::{PoxInfo, StacksRpc};
use rocket::config::{Config, LogLevel};
use rocket::serde::json::{json, Json, Value as JsonValue};
use rocket::serde::Deserialize;
use rocket::State;
use std::collections::{VecDeque, HashMap};
use std::convert::TryFrom;
use std::error::Error;
use std::iter::FromIterator;
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::str;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};
use reqwest::Client as HttpClient;
use std::str::FromStr;
use bitcoincore_rpc::bitcoin::{Txid, BlockHash};
use bitcoincore_rpc::{Auth, Client, RpcApi};
use clarity_repl::clarity::util::hash::bytes_to_hex;

pub const DEFAULT_INGESTION_PORT: u16 = 20445;
pub const DEFAULT_CONTROL_PORT: u16 = 20446;

#[derive(Deserialize)]
pub struct NewTransaction {
    pub txid: String,
    pub status: String,
    pub raw_result: String,
    pub raw_tx: String,
}

#[derive(Clone, Debug)]
pub enum Event {
    BitcoinChainEvent(BitcoinChainEvent),
    StacksChainEvent(StacksChainEvent),
}

#[derive(Clone, Debug)]
pub enum EventHandler {
    WebHook(String),
    GrpcStream(u64),
}

impl EventHandler {

    async fn propagate_stacks_event(&self, stacks_event: &StacksChainEvent) {
        match self {
            EventHandler::WebHook(host) => {
                let path = "chain-events/stacks";
                let url = format!("{}/{}", host, path);
                let body = rocket::serde::json::serde_json::to_vec(&stacks_event).unwrap();
                let http_client = HttpClient::builder().build().expect("Unable to build http client");
                let _ = http_client
                    .post(url)
                    .header("Content-Type", "application/json")
                    .body(body)
                    .send()
                    .await;
            }
            EventHandler::GrpcStream(stream) => {
                
            }
        }
    }

    async fn propagate_bitcoin_event(&self, bitcoin_event: &BitcoinChainEvent) {
        match self {
            EventHandler::WebHook(host) => {
                let path = "chain-events/bitcoin";
                let url = format!("{}/{}", host, path);
                let body = rocket::serde::json::serde_json::to_vec(&bitcoin_event).unwrap();
                let http_client = HttpClient::builder().build().expect("Unable to build http client");
                let res = http_client
                    .post(url)
                    .header("Content-Type", "application/json")
                    .body(body)
                    .send()
                    .await;
            }
            EventHandler::GrpcStream(stream) => {

            }
        }
    }

    async fn notify_bitcoin_transaction_proxied(&self) {

    }
}

#[derive(Clone, Debug)]
pub struct EventObserverConfig {
    pub normalization_enabled: bool,
    pub grpc_server_enabled: bool,
    pub hooks_enabled: bool,
    pub initial_hook_formation: Option<HookFormation>,
    pub bitcoin_rpc_proxy_enabled: bool,
    pub event_handlers: Vec<EventHandler>,
    pub ingestion_port: u16,
    pub control_port: u16,
    pub bitcoin_node_username: String,
    pub bitcoin_node_password: String,
    pub bitcoin_node_rpc_host: String,
    pub bitcoin_node_rpc_port: u16,
    pub stacks_node_rpc_host: String,
    pub stacks_node_rpc_port: u16,
}

#[derive(Deserialize, Debug)]
pub struct ContractReadonlyCall {
    pub okay: bool,
    pub result: String,
}

#[derive(Clone, Debug)]
pub enum ObserverCommand {
    PropagateBitcoinChainEvent(BitcoinChainEvent),
    PropagateStacksChainEvent(StacksChainEvent),
    SubscribeStreamer(u64),
    UnsubscribeStreamer(u64),
    RegisterHook(HookSpecification),
    DeregisterHook(u64),
    NotifyBitcoinTransactionProxied,
    Terminate,
}

#[derive(Clone, Debug)]
pub enum ObserverEvent {
    Error(String),
    Fatal(String),
    Info(String),
    BitcoinChainEvent(BitcoinChainEvent),
    StacksChainEvent(StacksChainEvent),
    NotifyBitcoinTransactionProxied,
    Terminate
}

#[derive(Debug, Clone, Deserialize, Serialize)]
/// JSONRPC Request
pub struct BitcoinRPCRequest {
    /// The name of the RPC call
    pub method: String,
    /// Parameters to the RPC call
    pub params: serde_json::Value,
    /// Identifier for this Request, which should appear in the response
    pub id: serde_json::Value,
    /// jsonrpc field, MUST be "2.0"
    pub jsonrpc: serde_json::Value,
}

pub async fn start_event_observer(
    mut config: EventObserverConfig,
    observer_commands_tx: Sender<ObserverCommand>,
    observer_commands_rx: Receiver<ObserverCommand>,
    observer_events_tx: Option<Sender<ObserverEvent>>,
) -> Result<(), Box<dyn Error>> {

    let indexer = Indexer::new(IndexerConfig {
        stacks_node_rpc_url: format!(
            "{}:{}",
            config.stacks_node_rpc_host,
            config.stacks_node_rpc_port
        ),
        bitcoin_node_rpc_url: format!(
            "{}:{}",
            config.bitcoin_node_rpc_host,
            config.bitcoin_node_rpc_port
        ),
        bitcoin_node_rpc_username: config.bitcoin_node_username.clone(),
        bitcoin_node_rpc_password: config.bitcoin_node_password.clone(),
    });

    let config_mutex = Arc::new(Mutex::new(config.clone()));
    let indexer_rw_lock = Arc::new(RwLock::new(indexer));

    let background_job_tx_mutex = Arc::new(Mutex::new(observer_commands_tx.clone()));

    let ingestion_config = Config {
        port: config.ingestion_port,
        workers: 3,
        address: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
        keep_alive: 5,
        temp_dir: std::env::temp_dir(),
        log_level: LogLevel::Debug,
        ..Config::default()
    };

    let mut routes = routes![
        handle_ping,
        handle_new_bitcoin_block,
        handle_new_stacks_block,
        handle_new_microblocks,
        handle_new_mempool_tx,
        handle_drop_mempool_tx,
    ];

    if config.bitcoin_rpc_proxy_enabled {
        routes.append(&mut routes![handle_bitcoin_rpc_call]);
    }

    let _ = std::thread::spawn(move || {
        let future = rocket::custom(ingestion_config)
            .manage(indexer_rw_lock)
            .manage(config_mutex)
            .manage(background_job_tx_mutex)
            .mount(
                "/",
                routes,
            )
            .launch();

        utils::nestable_block_on(future);
    });

    let control_config = Config {
        port: config.control_port,
        workers: 1,
        address: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
        keep_alive: 5,
        temp_dir: std::env::temp_dir(),
        log_level: LogLevel::Debug,
        ..Config::default()
    };

    let routes = routes![
        handle_ping,
        handle_create_hook,
        handle_delete_hook,
    ];

    let background_job_tx_mutex = Arc::new(Mutex::new(observer_commands_tx.clone()));

    let _ = std::thread::spawn(move || {

        let future = rocket::custom(control_config)
            .manage(background_job_tx_mutex)
            .mount(
                "/",
                routes,
            )
            .launch();

            utils::nestable_block_on(future);
        });

    // This loop is used for handling background jobs, emitted by HTTP calls.
    let stop_miner = Arc::new(AtomicBool::new(false));
    let mut event_handlers = config.event_handlers.clone();
    let mut hook_formation = HookFormation::new();

    if let Some(ref mut initial_hook_formation) = config.initial_hook_formation {
        hook_formation.stacks_hooks.append(&mut initial_hook_formation.stacks_hooks);
        hook_formation.bitcoin_hooks.append(&mut initial_hook_formation.bitcoin_hooks); 
    }

    loop {
        let command = match observer_commands_rx.recv() {
            Ok(cmd) => cmd,
            Err(e) => {
                if let Some(ref tx) = observer_events_tx {
                    let _ = tx.send(ObserverEvent::Error(format!("Chanel error: {:?}", e)));
                }
                continue;
            }
        };
        match command {
            ObserverCommand::Terminate => {
                if let Some(ref tx) = observer_events_tx {
                    let _ = tx.send(ObserverEvent::Info("Terminating event observer".into()));
                    let _ = tx.send(ObserverEvent::Terminate);
                }
                break;
            }
            ObserverCommand::PropagateBitcoinChainEvent(chain_event) => {
                for event_handler in event_handlers.iter() {
                    event_handler.propagate_bitcoin_event(&chain_event).await;
                }
                // process hooks
                if config.hooks_enabled {
                    let hooks_to_trigger = evaluate_bitcoin_hooks_on_chain_event(&chain_event, &hook_formation.bitcoin_hooks);
                    let mut proofs = HashMap::new();
                    for (_, transaction, block_identifier) in hooks_to_trigger.iter() {
                        if !proofs.contains_key(&transaction.transaction_identifier.hash) {

                            let rpc = Client::new(
                                &format!("http://localhost:{}", config.bitcoin_node_rpc_port),
                                Auth::UserPass(
                                    config.bitcoin_node_username.to_string(),
                                    config.bitcoin_node_password.to_string(),
                                ),
                            )
                            .unwrap();
                            let txid = Txid::from_str(&transaction.transaction_identifier.hash).expect("Unable to retrieve txid");
                            let block_hash = BlockHash::from_str(&block_identifier.hash).expect("Unable to retrieve txid");
                            let res = rpc.get_tx_out_proof(&vec![txid], Some(&block_hash));
                            if let Ok(proof) = res {
                                proofs.insert(transaction.transaction_identifier.hash.clone(), bytes_to_hex(&proof));
                            }
                        }
                    }
                    for (hook, transaction, block_identifier) in hooks_to_trigger.into_iter() {
                        handle_bitcoin_hook_action(hook, transaction, block_identifier, proofs.get(&transaction.transaction_identifier.hash)).await;
                    }
                }

                if let Some(ref tx) = observer_events_tx {
                    let _ = tx.send(ObserverEvent::BitcoinChainEvent(chain_event));
                }
            }
            ObserverCommand::PropagateStacksChainEvent(chain_event) => {
                for event_handler in event_handlers.iter() {
                    event_handler.propagate_stacks_event(&chain_event).await;
                }
                if config.hooks_enabled {
                    // process hooks
                    let hooks_to_trigger = evaluate_stacks_hooks_on_chain_event(&chain_event, &hook_formation.stacks_hooks);
                    for (hook, transaction, block_identifier) in hooks_to_trigger.into_iter() {
                        handle_stacks_hook_action(hook, transaction).await;
                    }    
                }
                if let Some(ref tx) = observer_events_tx {
                    let _ = tx.send(ObserverEvent::StacksChainEvent(chain_event));
                }
            }
            ObserverCommand::NotifyBitcoinTransactionProxied => {
                for event_handler in event_handlers.iter() {
                    event_handler.notify_bitcoin_transaction_proxied().await;
                }
                if let Some(ref tx) = observer_events_tx {
                    let _ = tx.send(ObserverEvent::NotifyBitcoinTransactionProxied);
                }
            }
            ObserverCommand::SubscribeStreamer(stream) => {
                event_handlers.push(EventHandler::GrpcStream(stream));
            }
            ObserverCommand::UnsubscribeStreamer(stream) => {
            }
            ObserverCommand::RegisterHook(hook) => {
                match hook {
                    HookSpecification::Stacks(hook) => hook_formation.stacks_hooks.push(hook),
                    HookSpecification::Bitcoin(hook) => hook_formation.bitcoin_hooks.push(hook),
                }
            }
            ObserverCommand::DeregisterHook(hook_id) => {
            }
        }
    }
    Ok(())
}

#[get("/ping", format = "application/json")]
pub fn handle_ping() -> Json<JsonValue> {
    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}

#[post("/new_burn_block", format = "json", data = "<marshalled_block>")]
pub fn handle_new_bitcoin_block(
    indexer_rw_lock: &State<Arc<RwLock<Indexer>>>,
    marshalled_block: Json<JsonValue>,
    background_job_tx: &State<Arc<Mutex<Sender<ObserverCommand>>>>,
) -> Json<JsonValue> {

    // Standardize the structure of the block, and identify the
    // kind of update that this new block would imply, taking
    // into account the last 7 blocks.
    let chain_update = match indexer_rw_lock.inner().write() {
        Ok(mut indexer) => indexer.handle_bitcoin_block(marshalled_block.into_inner()),
        _ => {
            return Json(json!({
                "status": 200,
                "result": "Ok",
            }))
        }
    };

    let background_job_tx = background_job_tx.inner();
    match background_job_tx.lock() {
        Ok(tx) => {
            let _ = tx.send(ObserverCommand::PropagateBitcoinChainEvent(chain_update));
        }
        _ => {}
    };

    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}

#[post("/new_block", format = "application/json", data = "<marshalled_block>")]
pub fn handle_new_stacks_block(
    indexer_rw_lock: &State<Arc<RwLock<Indexer>>>,
    marshalled_block: Json<JsonValue>,
    background_job_tx: &State<Arc<Mutex<Sender<ObserverCommand>>>>,
) -> Json<JsonValue> {
    // Standardize the structure of the block, and identify the
    // kind of update that this new block would imply, taking
    // into account the last 7 blocks.
    let (pox_info, chain_event) = match indexer_rw_lock.inner().write() {
        Ok(mut indexer) => {
            let pox_info = indexer.get_pox_info();
            let chain_event = indexer.handle_stacks_block(marshalled_block.into_inner());
            (pox_info, chain_event)
        }
        _ => {
            return Json(json!({
                "status": 200,
                "result": "Ok",
            }))
        }
    };

    let background_job_tx = background_job_tx.inner();
    match background_job_tx.lock() {
        Ok(tx) => {
            let _ = tx.send(ObserverCommand::PropagateStacksChainEvent(chain_event));
        }
        _ => {}
    };

    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}

#[post(
    "/new_microblocks",
    format = "application/json",
    data = "<marshalled_microblock>"
)]
pub fn handle_new_microblocks(
    indexer_rw_lock: &State<Arc<RwLock<Indexer>>>,
    marshalled_microblock: Json<JsonValue>,
    background_job_tx: &State<Arc<Mutex<Sender<ObserverCommand>>>>,
) -> Json<JsonValue> {

    // Standardize the structure of the microblock, and identify the
    // kind of update that this new microblock would imply
    let chain_event = match indexer_rw_lock.inner().write() {
        Ok(mut indexer) => {
            let chain_event = indexer.handle_stacks_microblock(marshalled_microblock.into_inner());
            chain_event
        }
        _ => {
            return Json(json!({
                "status": 200,
                "result": "Ok",
            }))
        }
    };

    let background_job_tx = background_job_tx.inner();
    match background_job_tx.lock() {
        Ok(tx) => {
            let _ = tx.send(ObserverCommand::PropagateStacksChainEvent(chain_event));
        }
        _ => {}
    };

    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}

#[post("/new_mempool_tx", format = "application/json", data = "<raw_txs>")]
pub fn handle_new_mempool_tx(
    raw_txs: Json<Vec<String>>,
    background_job_tx: &State<Arc<Mutex<Sender<ObserverCommand>>>>,
) -> Json<JsonValue> {
    let decoded_transactions = raw_txs
        .iter()
        .map(|t| {
            let (txid, ..) =
                chains::stacks::get_tx_description(t).expect("unable to parse transaction");
            txid
        })
        .collect::<Vec<String>>();

    // if let Ok(tx_sender) = devnet_events_tx.lock() {
    //     for tx in decoded_transactions.into_iter() {
    //         let _ = tx_sender.send(DevnetEvent::MempoolAdmission(MempoolAdmissionData { tx }));
    //     }
    // }

    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}

#[post("/drop_mempool_tx", format = "application/json")]
pub fn handle_drop_mempool_tx() -> Json<JsonValue> {
    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}

#[post("/", format = "application/json", data = "<bitcoin_rpc_call>")]
pub async fn handle_bitcoin_rpc_call(
    config: &State<Arc<Mutex<EventObserverConfig>>>,
    bitcoin_rpc_call: Json<BitcoinRPCRequest>,
    background_job_tx: &State<Arc<Mutex<Sender<ObserverCommand>>>>,
) -> Json<JsonValue> {
    use base64::encode;
    use reqwest::Client;

    let bitcoin_rpc_call = bitcoin_rpc_call.into_inner().clone();
    let method = bitcoin_rpc_call.method.clone();
    let body = rocket::serde::json::serde_json::to_vec(&bitcoin_rpc_call).unwrap();

    let builder = match config.inner().lock() {
        Ok(config) => {
            let token = encode(format!(
                "{}:{}",
                config.bitcoin_node_username,
                config.bitcoin_node_password
            ));
            let client = Client::new();
            client
                .post(format!(
                    "{}:{}/",
                    config.bitcoin_node_rpc_host,
                    config.bitcoin_node_rpc_port
                ))
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Basic {}", token))
        }
        _ => unreachable!(),
    };

    if method == "sendrawtransaction" {
        let background_job_tx = background_job_tx.inner();
        match background_job_tx.lock() {
            Ok(tx) => {
                let _ = tx.send(ObserverCommand::NotifyBitcoinTransactionProxied);
            }
            _ => {}
        };
    }

    let res = builder.body(body).send().await.unwrap();

    Json(res.json().await.unwrap())
}

#[post("/v1/hooks", format = "application/json", data = "<hook>")]
pub fn handle_create_hook(
    hook: Json<HookSpecification>,
    background_job_tx: &State<Arc<Mutex<Sender<ObserverCommand>>>>,
) -> Json<JsonValue> {

    let hook = hook.into_inner();

    let background_job_tx = background_job_tx.inner();
    match background_job_tx.lock() {
        Ok(tx) => {
            let _ = tx.send(ObserverCommand::RegisterHook(hook));
        }
        _ => {}
    };

    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}

#[delete("/v1/hooks", format = "application/json", data = "<hook_id>")]
pub fn handle_delete_hook(
    hook_id: Json<JsonValue>,
    background_job_tx: &State<Arc<Mutex<Sender<ObserverCommand>>>>,
) -> Json<JsonValue> {

    let background_job_tx = background_job_tx.inner();
    match background_job_tx.lock() {
        Ok(tx) => {
            let _ = tx.send(ObserverCommand::DeregisterHook(1));
        }
        _ => {}
    };

    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}
