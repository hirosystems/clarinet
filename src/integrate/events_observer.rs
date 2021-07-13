use serde_json::Value;
use std::error::Error;
use std::path::PathBuf;
use rocket::config::{Config, Environment, LoggingLevel};
use rocket_contrib::json::Json;
use rocket::State;
use std::str;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};
use crate::integrate::{BlockData, LogData, MempoolAdmissionData, ServiceStatusData, Status, Transaction};
use crate::types::{ChainConfig, DevnetConfig};
use crate::publish::{publish_contracts, Network};
use super::DevnetEvent;

// decode request data
#[derive(Deserialize)]
pub struct NewBurnBlock {
    burn_block_hash: String,
    burn_block_height: u32,
    reward_slot_holders: Vec<String>,
    burn_amount: u32,
}

#[derive(Deserialize)]
pub struct NewBlock {
    block_height: u32,
    block_hash: String,
    burn_block_height: u32,
    burn_block_hash: String,
    transactions: Vec<NewTransaction>,
    // reward_slot_holders: Vec<String>,
    // burn_amount: u32,
}

#[derive(Deserialize)]
pub struct NewTransaction {
    txid: String,
    status: String,
    raw_result: String,
    // tx_index: u32,
}

#[derive(Clone, Debug)]
pub struct EventObserverConfig {
    pub devnet_config: DevnetConfig,
    pub manifest_path: PathBuf,
    pub pox_info: PoxInfo,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct PoxInfo {
    pox_activation_threshold_ustx: u64,
    first_burnchain_block_height: u32,
    prepare_phase_block_length: u32,
    reward_phase_block_length: u32,
    reward_slots: u32,
}

pub async fn start_events_observer(events_config: EventObserverConfig, devnet_event_tx: Sender<DevnetEvent>, terminator_rx: Receiver<bool>) -> Result<(), Box<dyn Error>> {

    let port = events_config.devnet_config.orchestrator_port;

    let config = Config::build(Environment::Production)
        .address("127.0.0.1")
        .port(port)
        .log_level(LoggingLevel::Off)
        .finalize()?;

    rocket::custom(config)
        .manage(RwLock::new(events_config))
        .manage(Arc::new(Mutex::new(devnet_event_tx.clone())))
        .mount("/", routes![handle_new_burn_block, handle_new_block, handle_new_mempool_tx, handle_drop_mempool_tx])
        .launch();

    match terminator_rx.recv() {
        Ok(true) => {
            devnet_event_tx.send(DevnetEvent::info("Terminating event observer".into()))
                .expect("Unable to terminate event observer");
        },
        _ => {}
    }
    Ok(())
}

#[post("/new_burn_block", format = "application/json", data = "<new_burn_block>")]
pub fn handle_new_burn_block(config: State<RwLock<EventObserverConfig>>, devnet_events_tx: State<Arc<Mutex<Sender<DevnetEvent>>>>, new_burn_block: Json<NewBurnBlock>) -> Json<Value> {
    let devnet_events_tx = devnet_events_tx.inner();

    match devnet_events_tx.lock() {
        Ok(tx) => {
            let _ = tx.send(DevnetEvent::debug(format!("Bitcoin block #{} received", new_burn_block.burn_block_height)));
            let _ = tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
                order: 0,
                status: Status::Green,
                name: "bitcoin-node".into(),
                comment: format!("mining blocks (chaintip = #{})", new_burn_block.burn_block_height),
            }));
        }
        _ => {} 
    };

    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}

#[post("/new_block", format = "application/json", data = "<new_block>")]
pub fn handle_new_block(config: State<RwLock<EventObserverConfig>>, devnet_events_tx: State<Arc<Mutex<Sender<DevnetEvent>>>>, new_block: Json<NewBlock>) -> Json<Value> {
    let devnet_events_tx = devnet_events_tx.inner();
    let config = config.inner();

    if let Ok(tx) = devnet_events_tx.lock() {
        let _ = tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
            order: 1,
            status: Status::Green,
            name: "stacks-node".into(),
            comment: format!("mining blocks (chaintip = #{})", new_block.block_height),
        }));
        let _ = tx.send(DevnetEvent::info(format!("Block #{} anchored in Bitcoin block #{} includes {} transactions", 
            new_block.block_height, 
            new_block.burn_block_height,
            new_block.transactions.len(),
        )));

        if new_block.block_height == 1 {
            // We just received the Stacks Genesis block.
            // With that, we will be:
            // - Fetching the pox constants
            // - Publishing the contracts
            let updated_config = if let Ok(config_reader) = config.read() {
                let mut updated_config = config_reader.clone();
                let url = format!("http://0.0.0.0:{}/v2/pox", updated_config.devnet_config.stacks_node_rpc_port);
                updated_config.pox_info = match reqwest::blocking::get(url) {
                    Ok(reponse) => {
                        let _ = tx.send(DevnetEvent::debug(format!("{:?}", reponse)));
                        let pox_info: PoxInfo = reponse.json().unwrap();
                        let _ = tx.send(DevnetEvent::debug(format!("{:?}", pox_info)));
                        pox_info
                    },
                    Err(_) => PoxInfo::default()
                };
                Some(updated_config)
            } else {
                None
            };

            if let Some(updated_config) = updated_config {
                let _ = tx.send(DevnetEvent::debug(format!("Updated config: {:?} ",  updated_config.pox_info)));

                let logs = match publish_contracts(updated_config.manifest_path.clone(), Network::Devnet) {
                    Ok(res) => res.iter().map(|l| DevnetEvent::success(l.into())).collect(),
                    Err(e) => vec![DevnetEvent::error(e.into())]
                };
                for log in logs.into_iter() {
                    let _ = tx.send(log);
                }

                if let Ok(mut config_writer) = config.write() {
                    *config_writer = updated_config;
                }
            }
        }

        if let Ok(config_reader) = config.read() {
            let pox_cycle_length = config_reader.pox_info.prepare_phase_block_length + config_reader.pox_info.reward_phase_block_length;
            let pox_cycle_id = (new_block.burn_block_height - config_reader.pox_info.first_burnchain_block_height) / pox_cycle_length;
            let _ = tx.send(DevnetEvent::Block(BlockData {
                block_height: new_block.block_height,
                block_hash: new_block.block_hash.clone(),
                bitcoin_block_height: new_block.burn_block_height,
                bitcoin_block_hash: new_block.burn_block_hash.clone(),
                first_burnchain_block_height: config_reader.pox_info.first_burnchain_block_height,
                pox_cycle_length,
                pox_cycle_id,        
                transactions: new_block.transactions.iter().map(|t| {
                    Transaction {
                        txid: t.txid.clone(),
                        success: t.status == "success",
                        result: t.raw_result.clone(),
                        events: vec![],
                    }
                }).collect(),
            }));
        }
    };

    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}

#[post("/new_microblocks", format = "application/json")]
pub fn handle_new_microblocks(devnet_events_tx: State<Arc<Mutex<Sender<DevnetEvent>>>>) -> Json<Value> {
    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}

#[post("/new_mempool_tx", format = "application/json", data = "<raw_txs>")]
pub fn handle_new_mempool_tx(config: State<RwLock<EventObserverConfig>>, devnet_events_tx: State<Arc<Mutex<Sender<DevnetEvent>>>>, raw_txs: Json<Vec<String>>) -> Json<Value> {

    if let Ok(tx) = devnet_events_tx.lock() {
        for raw_tx in raw_txs.iter() {
            let _ = tx.send(DevnetEvent::MempoolAdmission(MempoolAdmissionData {
                txid: raw_tx.to_string()
            }));
        }
    }

    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}

#[post("/drop_mempool_tx", format = "application/json")]
pub fn handle_drop_mempool_tx() -> Json<Value> {
    println!("POST /drop_mempool_tx");

    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}
