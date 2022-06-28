use crate::chainhooks::types::{ChainhookSpecification, HookFormation};
use crate::chainhooks::{
    evaluate_bitcoin_chainhooks_on_chain_event, evaluate_stacks_chainhooks_on_chain_event,
    handle_bitcoin_hook_action, handle_stacks_hook_action,
};
use crate::indexer::{chains, Indexer, IndexerConfig};
use crate::utils;
use bitcoincore_rpc::bitcoin::{BlockHash, Txid};
use bitcoincore_rpc::{Auth, Client, RpcApi};
use clarity_repl::clarity::util::hash::bytes_to_hex;
use orchestra_types::{
    BitcoinChainEvent, StacksChainEvent, StacksNetwork, StacksTransactionData,
    TransactionIdentifier,
};
use reqwest::Client as HttpClient;
use rocket::config::{Config, LogLevel};
use rocket::http::Status;
use rocket::outcome::IntoOutcome;
use rocket::request::{self, FromRequest, Outcome, Request};
use rocket::serde::json::{json, Json, Value as JsonValue};
use rocket::serde::Deserialize;
use rocket::State;
use stacks_rpc_client::{PoxInfo, StacksRpc};
use std::collections::{HashMap, VecDeque};
use std::convert::TryFrom;
use std::error::Error;
use std::iter::FromIterator;
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::str;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};

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

// TODO(lgalabru): Support for GRPC?
#[derive(Clone, Debug)]
pub enum EventHandler {
    WebHook(String),
}

impl EventHandler {
    async fn propagate_stacks_event(&self, stacks_event: &StacksChainEvent) {
        match self {
            EventHandler::WebHook(host) => {
                let path = "chain-events/stacks";
                let url = format!("{}/{}", host, path);
                let body = rocket::serde::json::serde_json::to_vec(&stacks_event).unwrap();
                let http_client = HttpClient::builder()
                    .build()
                    .expect("Unable to build http client");
                let _ = http_client
                    .post(url)
                    .header("Content-Type", "application/json")
                    .body(body)
                    .send()
                    .await;
                // TODO(lgalabru): handle response errors
            }
        }
    }

    async fn propagate_bitcoin_event(&self, bitcoin_event: &BitcoinChainEvent) {
        match self {
            EventHandler::WebHook(host) => {
                let path = "chain-events/bitcoin";
                let url = format!("{}/{}", host, path);
                let body = rocket::serde::json::serde_json::to_vec(&bitcoin_event).unwrap();
                let http_client = HttpClient::builder()
                    .build()
                    .expect("Unable to build http client");
                let _res = http_client
                    .post(url)
                    .header("Content-Type", "application/json")
                    .body(body)
                    .send()
                    .await;
                // TODO(lgalabru): handle response errors
            }
        }
    }

    async fn notify_bitcoin_transaction_proxied(&self) {}
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
    pub operators: HashMap<Option<String>, HookFormation>,
}

impl EventObserverConfig {
    pub fn is_authorization_required(&self) -> bool {
        !self.operators.is_empty()
    }

    pub fn is_authorized(&self, token: &str) -> bool {
        self.operators.contains_key(&Some(token.to_string()))
    }
}

#[derive(Deserialize, Debug)]
pub struct ContractReadonlyCall {
    pub okay: bool,
    pub result: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ObserverCommand {
    PropagateBitcoinChainEvent(BitcoinChainEvent),
    PropagateStacksChainEvent(StacksChainEvent),
    PropagateStacksMempoolEvent(StacksChainMempoolEvent),
    RegisterHook(ChainhookSpecification, ApiKey),
    DeregisterBitcoinHook(String, ApiKey),
    DeregisterStacksHook(String, ApiKey),
    NotifyBitcoinTransactionProxied,
    Terminate,
}

#[derive(Clone, Debug, PartialEq)]
pub enum StacksChainMempoolEvent {
    TransactionsAdmitted(Vec<MempoolAdmissionData>),
    TransactionDropped(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct MempoolAdmissionData {
    pub tx_data: String,
    pub tx_description: String,
}

#[derive(Clone, Debug)]
pub enum ObserverEvent {
    Error(String),
    Fatal(String),
    Info(String),
    BitcoinChainEvent(BitcoinChainEvent),
    StacksChainEvent(StacksChainEvent),
    NotifyBitcoinTransactionProxied,
    HookRegistered(ChainhookSpecification),
    HookDeregistered(ChainhookSpecification),
    HooksTriggered(usize),
    Terminate,
    StacksChainMempoolEvent(StacksChainMempoolEvent),
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
            config.stacks_node_rpc_host, config.stacks_node_rpc_port
        ),
        bitcoin_node_rpc_url: format!(
            "{}:{}",
            config.bitcoin_node_rpc_host, config.bitcoin_node_rpc_port
        ),
        bitcoin_node_rpc_username: config.bitcoin_node_username.clone(),
        bitcoin_node_rpc_password: config.bitcoin_node_password.clone(),
    });

    let managed_config = config.clone();
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
        handle_new_attachement,
    ];

    if config.bitcoin_rpc_proxy_enabled {
        routes.append(&mut routes![handle_bitcoin_rpc_call]);
    }

    let managed_config_ingestion = managed_config.clone();

    let _ = std::thread::spawn(move || {
        let future = rocket::custom(ingestion_config)
            .manage(indexer_rw_lock)
            .manage(background_job_tx_mutex)
            .manage(managed_config_ingestion)
            .mount("/", routes)
            .launch();

        let _ = utils::nestable_block_on(future);
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
        handle_delete_bitcoin_hook,
        handle_delete_stacks_hook
    ];

    let background_job_tx_mutex = Arc::new(Mutex::new(observer_commands_tx.clone()));

    let _ = std::thread::spawn(move || {
        let future = rocket::custom(control_config)
            .manage(background_job_tx_mutex)
            .manage(managed_config)
            .mount("/", routes)
            .launch();

        let _ = utils::nestable_block_on(future);
    });

    // This loop is used for handling background jobs, emitted by HTTP calls.
    start_observer_commands_handler(&mut config, observer_commands_rx, observer_events_tx).await
}

pub async fn start_observer_commands_handler(
    config: &mut EventObserverConfig,
    observer_commands_rx: Receiver<ObserverCommand>,
    observer_events_tx: Option<Sender<ObserverEvent>>,
) -> Result<(), Box<dyn Error>> {
    let mut chainhooks_occurrences_tracker: HashMap<String, u64> = HashMap::new();
    let event_handlers = config.event_handlers.clone();
    let mut chainhooks_lookup: HashMap<String, ApiKey> = HashMap::new();

    // If authorization not required, we create a default HookFormation
    if !config.is_authorization_required() {
        let mut hook_formation = HookFormation::new();
        if let Some(ref mut initial_hook_formation) = config.initial_hook_formation {
            hook_formation
                .stacks_chainhooks
                .append(&mut initial_hook_formation.stacks_chainhooks);
            hook_formation
                .bitcoin_chainhooks
                .append(&mut initial_hook_formation.bitcoin_chainhooks);
        }
        config.operators.insert(None, hook_formation);
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
                    let bitcoin_chainhooks = config
                        .operators
                        .values()
                        .map(|v| &v.bitcoin_chainhooks)
                        .flatten()
                        .collect();

                    let hooks_to_trigger = evaluate_bitcoin_chainhooks_on_chain_event(
                        &chain_event,
                        bitcoin_chainhooks,
                    );
                    let mut proofs = HashMap::new();
                    for (_, transaction, block_identifier) in hooks_to_trigger.iter() {
                        if !proofs.contains_key(&transaction.transaction_identifier.hash) {
                            let rpc = Client::new(
                                &format!(
                                    "{}:{}",
                                    config.bitcoin_node_rpc_host, config.bitcoin_node_rpc_port
                                ),
                                Auth::UserPass(
                                    config.bitcoin_node_username.to_string(),
                                    config.bitcoin_node_password.to_string(),
                                ),
                            )
                            .unwrap();
                            let txid = Txid::from_str(&transaction.transaction_identifier.hash)?;
                            let block_hash = BlockHash::from_str(&block_identifier.hash)?;
                            let res = rpc.get_tx_out_proof(&vec![txid], Some(&block_hash));
                            if let Ok(proof) = res {
                                proofs.insert(
                                    transaction.transaction_identifier.hash.clone(),
                                    bytes_to_hex(&proof),
                                );
                            }
                        }
                    }
                    for (hook, transaction, block_identifier) in hooks_to_trigger.into_iter() {
                        handle_bitcoin_hook_action(
                            hook,
                            transaction,
                            block_identifier,
                            proofs.get(&transaction.transaction_identifier.hash),
                        )
                        .await;
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
                let mut hooks_ids_to_deregister = vec![];
                if config.hooks_enabled {
                    let stacks_chainhooks = config
                        .operators
                        .values()
                        .map(|v| &v.stacks_chainhooks)
                        .flatten()
                        .collect();

                    // process hooks
                    let hooks_triggerable =
                        evaluate_stacks_chainhooks_on_chain_event(&chain_event, stacks_chainhooks);

                    let mut hooks_to_trigger = vec![];

                    for (hook, tx, block_identifier) in hooks_triggerable.into_iter() {
                        let mut total_occurrences: u64 =
                            *chainhooks_occurrences_tracker.get(&hook.uuid).unwrap_or(&0);
                        total_occurrences += 1;

                        let limit = hook.expire_after_occurrence.unwrap_or(0);
                        if limit == 0 || total_occurrences <= limit {
                            hooks_to_trigger.push((hook, tx, block_identifier));
                            chainhooks_occurrences_tracker
                                .insert(hook.uuid.clone(), total_occurrences);
                        } else {
                            hooks_ids_to_deregister.push(hook.uuid.clone());
                        }
                    }

                    if let Some(ref tx) = observer_events_tx {
                        let _ = tx.send(ObserverEvent::HooksTriggered(hooks_to_trigger.len()));
                    }
                    for (hook, transaction, block_identifier) in hooks_to_trigger.into_iter() {
                        handle_stacks_hook_action(hook, transaction, block_identifier, None).await;
                    }
                }
                for hook_uuid in hooks_ids_to_deregister.iter() {
                    chainhooks_lookup
                        .get(hook_uuid)
                        .and_then(|api_key| config.operators.get_mut(&api_key.0))
                        .and_then(|hook_formation| {
                            hook_formation.deregister_stacks_hook(hook_uuid.clone())
                        })
                        .and_then(|chainhook| {
                            if let Some(ref tx) = observer_events_tx {
                                let _ = tx.send(ObserverEvent::HookDeregistered(
                                    ChainhookSpecification::Stacks(chainhook.clone()),
                                ));
                            }
                            Some(chainhook)
                        });
                }
                if let Some(ref tx) = observer_events_tx {
                    let _ = tx.send(ObserverEvent::StacksChainEvent(chain_event));
                }
            }
            ObserverCommand::PropagateStacksMempoolEvent(mempool_event) => {
                if let Some(ref tx) = observer_events_tx {
                    let _ = tx.send(ObserverEvent::StacksChainMempoolEvent(mempool_event));
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
            ObserverCommand::RegisterHook(hook, api_key) => {
                let hook_formation = match config.operators.get_mut(&api_key.0) {
                    Some(hook_formation) => hook_formation,
                    None => {
                        println!(
                            "Unable to retrieve chainhooks associated with {:?}",
                            api_key
                        );
                        continue;
                    }
                };
                hook_formation.register_hook(hook.clone());
                if let Some(ref tx) = observer_events_tx {
                    let _ = tx.send(ObserverEvent::HookRegistered(hook));
                }
            }
            ObserverCommand::DeregisterStacksHook(hook_uuid, api_key) => {
                let hook_formation = match config.operators.get_mut(&api_key.0) {
                    Some(hook_formation) => hook_formation,
                    None => {
                        println!(
                            "Unable to retrieve chainhooks associated with {:?}",
                            api_key
                        );
                        continue;
                    }
                };
                chainhooks_lookup.insert(hook_uuid.clone(), api_key.clone());
                let hook = hook_formation.deregister_stacks_hook(hook_uuid);
                if let (Some(tx), Some(hook)) = (&observer_events_tx, hook) {
                    let _ = tx.send(ObserverEvent::HookDeregistered(
                        ChainhookSpecification::Stacks(hook),
                    ));
                }
            }
            ObserverCommand::DeregisterBitcoinHook(hook_uuid, api_key) => {
                let hook_formation = match config.operators.get_mut(&api_key.0) {
                    Some(hook_formation) => hook_formation,
                    None => {
                        println!(
                            "Unable to retrieve chainhooks associated with {:?}",
                            api_key
                        );
                        continue;
                    }
                };
                let hook = hook_formation.deregister_bitcoin_hook(hook_uuid);
                if let (Some(tx), Some(hook)) = (&observer_events_tx, hook) {
                    let _ = tx.send(ObserverEvent::HookDeregistered(
                        ChainhookSpecification::Bitcoin(hook),
                    ));
                }
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
                "status": 500,
                "result": "Unable to acquire lock",
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
    // TODO(lgalabru): use _pox_info
    let (_pox_info, mut chain_event) = match indexer_rw_lock.inner().write() {
        Ok(mut indexer) => {
            let pox_info = indexer.get_pox_info();
            let chain_event = indexer.handle_stacks_block(marshalled_block.into_inner());
            (pox_info, chain_event)
        }
        _ => {
            return Json(json!({
                "status": 500,
                "result": "Unable to acquire lock",
            }))
        }
    };

    if let Some(chain_event) = chain_event.take() {
        let background_job_tx = background_job_tx.inner();
        match background_job_tx.lock() {
            Ok(tx) => {
                let _ = tx.send(ObserverCommand::PropagateStacksChainEvent(chain_event));
            }
            _ => {}
        };
    }

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
    let mut chain_event = match indexer_rw_lock.inner().write() {
        Ok(mut indexer) => {
            let chain_event = indexer.handle_stacks_microblock(marshalled_microblock.into_inner());
            chain_event
        }
        _ => {
            return Json(json!({
                "status": 500,
                "result": "Unable to acquire lock",
            }))
        }
    };

    if let Some(chain_event) = chain_event.take() {
        let background_job_tx = background_job_tx.inner();
        match background_job_tx.lock() {
            Ok(tx) => {
                let _ = tx.send(ObserverCommand::PropagateStacksChainEvent(chain_event));
            }
            _ => {}
        };
    }

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
    let transactions = raw_txs
        .iter()
        .map(|tx_data| {
            let (tx_description, ..) =
                chains::stacks::get_tx_description(&tx_data).expect("unable to parse transaction");
            MempoolAdmissionData {
                tx_data: tx_data.clone(),
                tx_description,
            }
        })
        .collect::<Vec<_>>();

    let background_job_tx = background_job_tx.inner();
    match background_job_tx.lock() {
        Ok(tx) => {
            let _ = tx.send(ObserverCommand::PropagateStacksMempoolEvent(
                StacksChainMempoolEvent::TransactionsAdmitted(transactions),
            ));
        }
        _ => {}
    };

    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}

#[post("/drop_mempool_tx", format = "application/json")]
pub fn handle_drop_mempool_tx() -> Json<JsonValue> {
    // TODO(lgalabru): use propagate mempool events
    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}

#[post("/attachments/new", format = "application/json")]
pub fn handle_new_attachement() -> Json<JsonValue> {
    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}

#[post("/", format = "application/json", data = "<bitcoin_rpc_call>")]
pub async fn handle_bitcoin_rpc_call(
    config: &State<EventObserverConfig>,
    bitcoin_rpc_call: Json<BitcoinRPCRequest>,
    background_job_tx: &State<Arc<Mutex<Sender<ObserverCommand>>>>,
) -> Json<JsonValue> {
    use base64::encode;
    use reqwest::Client;

    let bitcoin_rpc_call = bitcoin_rpc_call.into_inner().clone();
    let method = bitcoin_rpc_call.method.clone();

    let body = rocket::serde::json::serde_json::to_vec(&bitcoin_rpc_call).unwrap();

    let token = encode(format!(
        "{}:{}",
        config.bitcoin_node_username, config.bitcoin_node_password
    ));
    let client = Client::new();
    let builder = client
        .post(format!(
            "{}:{}/",
            config.bitcoin_node_rpc_host, config.bitcoin_node_rpc_port
        ))
        .header("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(5))
        .header("Authorization", format!("Basic {}", token));

    if method == "sendrawtransaction" {
        let background_job_tx = background_job_tx.inner();
        match background_job_tx.lock() {
            Ok(tx) => {
                let _ = tx.send(ObserverCommand::NotifyBitcoinTransactionProxied);
            }
            _ => {}
        };
    }

    match builder.body(body).send().await {
        Ok(res) => Json(res.json().await.unwrap()),
        Err(_) => Json(json!({
            "status": 500
        })),
    }
}

#[post("/v1/chainhooks", format = "application/json", data = "<hook>")]
pub fn handle_create_hook(
    hook: Json<ChainhookSpecification>,
    background_job_tx: &State<Arc<Mutex<Sender<ObserverCommand>>>>,
    api_key: ApiKey,
) -> Json<JsonValue> {
    let hook = hook.into_inner();
    let background_job_tx = background_job_tx.inner();
    match background_job_tx.lock() {
        Ok(tx) => {
            let _ = tx.send(ObserverCommand::RegisterHook(hook, api_key));
        }
        _ => {}
    };

    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}

#[delete("/v1/chainhooks/stacks/<hook_uuid>", format = "application/json")]
pub fn handle_delete_stacks_hook(
    hook_uuid: String,
    background_job_tx: &State<Arc<Mutex<Sender<ObserverCommand>>>>,
    api_key: ApiKey,
) -> Json<JsonValue> {
    let background_job_tx = background_job_tx.inner();
    match background_job_tx.lock() {
        Ok(tx) => {
            let _ = tx.send(ObserverCommand::DeregisterStacksHook(hook_uuid, api_key));
        }
        _ => {}
    };

    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}

#[delete("/v1/chainhooks/bitcoin/<hook_uuid>", format = "application/json")]
pub fn handle_delete_bitcoin_hook(
    hook_uuid: String,
    background_job_tx: &State<Arc<Mutex<Sender<ObserverCommand>>>>,
    api_key: ApiKey,
) -> Json<JsonValue> {
    let background_job_tx = background_job_tx.inner();
    match background_job_tx.lock() {
        Ok(tx) => {
            let _ = tx.send(ObserverCommand::DeregisterBitcoinHook(hook_uuid, api_key));
        }
        _ => {}
    };

    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}

#[derive(Clone, Debug, PartialEq)]
pub struct ApiKey(Option<String>);

#[derive(Debug)]
pub enum ApiKeyError {
    Missing,
    Invalid,
    InternalError,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for ApiKey {
    type Error = ApiKeyError;

    async fn from_request(req: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let res = req.rocket().state::<EventObserverConfig>();
        if let Some(config) = res {
            if !config.is_authorization_required() {
                Outcome::Success(ApiKey(None))
            } else {
                let key = req.headers().get_one("x-api-key");
                if let Some(key) = key {
                    match config.is_authorized(key) {
                        true => Outcome::Success(ApiKey(Some(key.to_string()))),
                        false => Outcome::Failure((Status::BadRequest, ApiKeyError::Invalid)),
                    }
                } else {
                    Outcome::Failure((Status::BadRequest, ApiKeyError::Missing))
                }
            }
        } else {
            Outcome::Failure((Status::InternalServerError, ApiKeyError::InternalError))
        }
    }
}

#[cfg(test)]
mod tests;
