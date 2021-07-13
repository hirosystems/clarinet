use serde_json::Value;
use std::error::Error;
use std::path::PathBuf;
use rocket::config::{Config, Environment, LoggingLevel};
use rocket_contrib::json::Json;
use rocket::State;
use std::str;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use crate::integrate::{LogData, BlockData, Transaction};
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
}

pub async fn start_events_observer(events_config: EventObserverConfig, devnet_event_tx: Sender<DevnetEvent>, terminator_rx: Receiver<bool>) -> Result<(), Box<dyn Error>> {

    let port = events_config.devnet_config.orchestrator_port;

    let config = Config::build(Environment::Production)
        .address("127.0.0.1")
        .port(port)
        .log_level(LoggingLevel::Off)
        .finalize()?;

    rocket::custom(config)
        .manage(Arc::new(Mutex::new(events_config)))
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
pub fn handle_new_burn_block(config: State<Arc<Mutex<EventObserverConfig>>>, devnet_events_tx: State<Arc<Mutex<Sender<DevnetEvent>>>>, new_burn_block: Json<NewBurnBlock>) -> Json<Value> {
    let devnet_events_tx = devnet_events_tx.inner();

    match devnet_events_tx.lock() {
        Ok(tx) => {
            let _ = tx.send(DevnetEvent::debug(format!("Bitcoin block #{} received", new_burn_block.burn_block_height)));
        }
        _ => {} 
    };

    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}

#[post("/new_block", format = "application/json", data = "<new_block>")]
pub fn handle_new_block(config: State<Arc<Mutex<EventObserverConfig>>>, devnet_events_tx: State<Arc<Mutex<Sender<DevnetEvent>>>>, new_block: Json<NewBlock>) -> Json<Value> {
    let devnet_events_tx = devnet_events_tx.inner();

    if let Ok(tx) = devnet_events_tx.lock() {
        let _ = tx.send(DevnetEvent::info(format!("Block #{} successfully anchored in Bitcoin block #{}, includes {} transactions", 
            new_block.block_height, 
            new_block.burn_block_height,
            new_block.transactions.len(),
        )));
        let _ = tx.send(DevnetEvent::Block(BlockData {
            block_height: new_block.block_height,
            block_hash: new_block.block_hash.clone(),
            bitcoin_block_height: new_block.burn_block_height,
            bitcoin_block_hash: new_block.burn_block_hash.clone(),
            transactions: new_block.transactions.iter().map(|t| {
                Transaction {
                    txid: t.txid.clone(),
                    success: t.status == "success",
                    result: t.raw_result.clone(),
                    events: vec![],
                }
            }).collect(),
        }));

        if new_block.block_height == 1 {
            let config = config.inner();
            if let Ok(config) = config.lock() {
                let _ = tx.send(DevnetEvent::info(format!("Checking and publishing contracts..."))); 
                let logs = match publish_contracts(config.manifest_path.clone(), Network::Devnet) {
                    Ok(res) => res.iter().map(|l| DevnetEvent::success(l.into())).collect(),
                    Err(e) => vec![DevnetEvent::error(e.into())]
                };
                for log in logs.into_iter() {
                    let _ = tx.send(log);
                }
            };
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


#[post("/new_mempool_tx", format = "application/json")]
pub fn handle_new_mempool_tx() -> Json<Value> {
    println!("POST /new_mempool_tx");

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
