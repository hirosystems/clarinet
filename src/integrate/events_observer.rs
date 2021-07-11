use serde_json::Value;
use std::error::Error;
use rocket::config::{Config, Environment, LoggingLevel};
use rocket_contrib::json::Json;
use rocket::State;
use std::thread;
use std::str;

// decode request data
#[derive(Deserialize)]
pub struct NewBurnBlock {
    burn_block_hash: String,
    burn_block_height: u32,
    reward_slot_holders: Vec<String>,
    burn_amount: u32,
}

pub async fn start_events_observer(port: u16) -> Result<(), Box<dyn Error>> {
    println!("Start event observer on port {}", port);
    let config = Config::build(Environment::Production)
        .address("127.0.0.1")
        .port(port)
        .log_level(LoggingLevel::Off)
        .finalize()?;

    rocket::custom(config)
        .mount("/", routes![handle_new_burn_block, handle_new_block, handle_new_mempool_tx, handle_drop_mempool_tx])
        .launch();

    println!("returning");
    Ok(())
}

#[post("/new_burn_block", format = "application/json")]
pub fn handle_new_burn_block() -> Json<Value> {
    println!("POST /new_burn_block");

    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}

#[post("/new_block", format = "application/json")]
pub fn handle_new_block() -> Json<Value> {
    println!("POST /new_block");

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
