use crate::indexer::bitcoin::{
    build_http_client, download_and_parse_block_with_retry, NewBitcoinBlock,
};
use crate::indexer::{self, Indexer};
use crate::monitoring::PrometheusMonitoring;
use crate::utils::Context;
use crate::{try_error, try_info};
use hiro_system_kit::slog;
use rocket::http::Status;
use rocket::response::status::Custom;
use rocket::serde::json::{json, Json, Value as JsonValue};
use rocket::State;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, RwLock};

use super::{
    BitcoinConfig, BitcoinRPCRequest, MempoolAdmissionData, ObserverCommand,
    StacksChainMempoolEvent,
};

fn success_response() -> Result<Json<JsonValue>, Custom<Json<JsonValue>>> {
    Ok(Json(json!({
        "status": 200,
        "result": "Ok",
    })))
}

fn error_response(
    message: String,
    ctx: &State<Context>,
) -> Result<Json<JsonValue>, Custom<Json<JsonValue>>> {
    try_error!(ctx, "{message}");
    Err(Custom(
        Status::InternalServerError,
        Json(json!({
            "status": 500,
            "result": message,
        })),
    ))
}

#[rocket::get("/ping", format = "application/json")]
pub fn handle_ping(
    ctx: &State<Context>,
    prometheus_monitoring: &State<PrometheusMonitoring>,
) -> Json<JsonValue> {
    ctx.try_log(|logger| slog::debug!(logger, "GET /ping"));

    Json(json!({
        "status": 200,
        "result": prometheus_monitoring.get_metrics(),
    }))
}

#[post("/new_burn_block", format = "json", data = "<bitcoin_block>")]
pub async fn handle_new_bitcoin_block(
    indexer_rw_lock: &State<Arc<RwLock<Indexer>>>,
    bitcoin_config: &State<BitcoinConfig>,
    bitcoin_block: Json<NewBitcoinBlock>,
    background_job_tx: &State<Arc<Mutex<Sender<ObserverCommand>>>>,
    prometheus_monitoring: &State<PrometheusMonitoring>,
    ctx: &State<Context>,
) -> Result<Json<JsonValue>, Custom<Json<JsonValue>>> {
    if bitcoin_config
        .bitcoin_block_signaling
        .should_ignore_bitcoin_block_signaling_through_stacks()
    {
        return success_response();
    }

    try_info!(ctx, "POST /new_burn_block");
    // Standardize the structure of the block, and identify the
    // kind of update that this new block would imply, taking
    // into account the last 7 blocks.

    let http_client = build_http_client();
    let block_hash = bitcoin_block.burn_block_hash.strip_prefix("0x").unwrap();
    let block =
        match download_and_parse_block_with_retry(&http_client, block_hash, bitcoin_config, ctx)
            .await
        {
            Ok(block) => block,
            Err(e) => {
                return error_response(format!("unable to download_and_parse_block: {e}"), ctx)
            }
        };

    let header = block.get_block_header();
    let block_height = header.block_identifier.index;
    prometheus_monitoring.btc_metrics_block_received(block_height);
    if let Err(e) = background_job_tx.lock().map(|tx| {
        tx.send(ObserverCommand::ProcessBitcoinBlock(block))
            .map_err(|e| format!("Unable to send stacks chain event: {}", e))
    }) {
        return error_response(format!("unable to acquire background_job_tx: {e}"), ctx);
    }

    let chain_update = match indexer_rw_lock.inner().write() {
        Ok(mut indexer) => indexer.handle_bitcoin_header(header, ctx),
        Err(e) => {
            return error_response(format!("Unable to acquire indexer_rw_lock: {e}"), ctx);
        }
    };

    match chain_update {
        Ok(Some(chain_event)) => {
            prometheus_monitoring.btc_metrics_block_appended(block_height);
            if let Err(e) = background_job_tx.lock().map(|tx| {
                tx.send(ObserverCommand::PropagateBitcoinChainEvent(chain_event))
                    .map_err(|e| format!("Unable to send stacks chain event: {}", e))
            }) {
                return error_response(format!("unable to acquire background_job_tx: {e}"), ctx);
            }
        }
        Ok(None) => {
            try_info!(ctx, "No chain event was generated");
        }
        Err(e) => {
            return error_response(format!("Unable to handle bitcoin block: {e}"), ctx);
        }
    }

    success_response()
}

#[post("/new_block", format = "application/json", data = "<marshalled_block>")]
pub fn handle_new_stacks_block(
    indexer_rw_lock: &State<Arc<RwLock<Indexer>>>,
    marshalled_block: Json<JsonValue>,
    background_job_tx: &State<Arc<Mutex<Sender<ObserverCommand>>>>,
    prometheus_monitoring: &State<PrometheusMonitoring>,
    ctx: &State<Context>,
) -> Result<Json<JsonValue>, Custom<Json<JsonValue>>> {
    try_info!(ctx, "POST /new_block");
    // Standardize the structure of the block, and identify the
    // kind of update that this new block would imply, taking
    // into account the last 7 blocks.
    // TODO(lgalabru): use _pox_config
    let (_pox_config, chain_event, new_tip) = match indexer_rw_lock.inner().write() {
        Ok(mut indexer) => {
            let pox_config = indexer.get_pox_config();
            let block = match indexer
                .standardize_stacks_marshalled_block(marshalled_block.into_inner(), ctx)
            {
                Ok(block) => block,
                Err(e) => {
                    return error_response(format!("Unable to standardize stacks block {e}"), ctx);
                }
            };
            let new_tip = block.block_identifier.index;
            prometheus_monitoring.stx_metrics_block_received(new_tip);
            let chain_event = indexer.process_stacks_block(block, ctx);
            (pox_config, chain_event, new_tip)
        }
        Err(e) => {
            return error_response(format!("Unable to acquire indexer_rw_lock: {e}"), ctx);
        }
    };

    match chain_event {
        Ok(Some(chain_event)) => {
            prometheus_monitoring.stx_metrics_block_appeneded(new_tip);
            if let Err(e) = background_job_tx.lock().map(|tx| {
                tx.send(ObserverCommand::PropagateStacksChainEvent(chain_event))
                    .map_err(|e| format!("Unable to send stacks chain event: {}", e))
            }) {
                return error_response(format!("unable to acquire background_job_tx: {e}"), ctx);
            }
        }
        Ok(None) => {
            try_info!(ctx, "No chain event was generated");
        }
        Err(e) => {
            return error_response(format!("Chain event error: {e}"), ctx);
        }
    }

    success_response()
}

#[post("/stackerdb_chunks", format = "application/json", data = "<payload>")]
#[cfg(feature = "stacks-signers")]
pub fn handle_stackerdb_chunks(
    indexer_rw_lock: &State<Arc<RwLock<Indexer>>>,
    payload: Json<JsonValue>,
    background_job_tx: &State<Arc<Mutex<Sender<ObserverCommand>>>>,
    ctx: &State<Context>,
) -> Result<Json<JsonValue>, Custom<Json<JsonValue>>> {
    use std::time::{SystemTime, UNIX_EPOCH};

    try_info!(ctx, "POST /stackerdb_chunks");

    // Standardize the structure of the StackerDB chunk, and identify the kind of update that this new message would imply.
    let Ok(epoch) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return error_response("Unable to get system receipt_time".to_string(), ctx);
    };
    let chain_event = match indexer_rw_lock.inner().write() {
        Ok(mut indexer) => indexer
            .handle_stacks_marshalled_stackerdb_chunk(payload.into_inner(), epoch.as_millis(), ctx),
        Err(e) => {
            return error_response(format!("Unable to acquire background_job_tx: {e}"), ctx);
        }
    };

    match chain_event {
        Ok(Some(chain_event)) => {
            if let Err(e) = background_job_tx.lock().map(|tx| {
                tx.send(ObserverCommand::PropagateStacksChainEvent(chain_event))
                    .map_err(|e| format!("Unable to send stacks chain event: {}", e))
            }) {
                return error_response(format!("unable to acquire background_job_tx: {e}"), ctx);
            }
        }
        Ok(None) => {
            try_info!(ctx, "No chain event was generated");
        }
        Err(e) => {
            return error_response(format!("Chain event error: {e}"), ctx);
        }
    }

    success_response()
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
    ctx: &State<Context>,
) -> Result<Json<JsonValue>, Custom<Json<JsonValue>>> {
    try_info!(ctx, "POST /new_microblocks");
    // Standardize the structure of the microblock, and identify the
    // kind of update that this new microblock would imply
    let chain_event = match indexer_rw_lock.inner().write() {
        Ok(mut indexer) => indexer
            .handle_stacks_marshalled_microblock_trail(marshalled_microblock.into_inner(), ctx),
        Err(e) => {
            return error_response(format!("Unable to acquire background_job_tx: {e}"), ctx);
        }
    };

    match chain_event {
        Ok(Some(chain_event)) => {
            if let Err(e) = background_job_tx.lock().map(|tx| {
                tx.send(ObserverCommand::PropagateStacksChainEvent(chain_event))
                    .map_err(|e| format!("Unable to send stacks chain event: {}", e))
            }) {
                return error_response(format!("unable to acquire background_job_tx: {e}"), ctx);
            }
        }
        Ok(None) => {
            try_info!(ctx, "No chain event was generated");
        }
        Err(e) => {
            return error_response(format!("Chain event error: {e}"), ctx);
        }
    }

    success_response()
}

#[post("/new_mempool_tx", format = "application/json", data = "<raw_txs>")]
pub fn handle_new_mempool_tx(
    raw_txs: Json<Vec<String>>,
    background_job_tx: &State<Arc<Mutex<Sender<ObserverCommand>>>>,
    ctx: &State<Context>,
) -> Result<Json<JsonValue>, Custom<Json<JsonValue>>> {
    try_info!(ctx, "POST /new_mempool_tx");
    let transactions = match raw_txs
        .iter()
        .map(|tx_data| {
            indexer::stacks::get_tx_description(tx_data, &vec![])
                .map(|(tx_description, ..)| MempoolAdmissionData {
                    tx_data: tx_data.clone(),
                    tx_description,
                })
                .map_err(|e| e)
        })
        .collect::<Result<Vec<MempoolAdmissionData>, _>>()
    {
        Ok(transactions) => transactions,
        Err(e) => {
            return error_response(format!("Failed to parse mempool transactions: {e}"), ctx);
        }
    };

    if let Err(e) = background_job_tx.lock().map(|tx| {
        tx.send(ObserverCommand::PropagateStacksMempoolEvent(
            StacksChainMempoolEvent::TransactionsAdmitted(transactions),
        ))
        .map_err(|e| format!("Unable to send stacks chain event: {}", e))
    }) {
        return error_response(format!("unable to acquire background_job_tx: {e}"), ctx);
    }

    success_response()
}

#[post("/drop_mempool_tx", format = "application/json", data = "<payload>")]
pub fn handle_drop_mempool_tx(payload: Json<JsonValue>, ctx: &State<Context>) -> Json<JsonValue> {
    ctx.try_log(|logger| slog::debug!(logger, "POST /drop_mempool_tx {:?}", payload));
    // TODO(lgalabru): use propagate mempool events
    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}

#[post("/attachments/new", format = "application/json", data = "<payload>")]
pub fn handle_new_attachement(payload: Json<JsonValue>, ctx: &State<Context>) -> Json<JsonValue> {
    ctx.try_log(|logger| slog::debug!(logger, "POST /attachments/new {:?}", payload));
    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}

#[post("/mined_block", format = "application/json", data = "<payload>")]
pub fn handle_mined_block(payload: Json<JsonValue>, ctx: &State<Context>) -> Json<JsonValue> {
    ctx.try_log(|logger| slog::debug!(logger, "POST /mined_block {:?}", payload));
    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}

#[post("/mined_microblock", format = "application/json", data = "<payload>")]
pub fn handle_mined_microblock(payload: Json<JsonValue>, ctx: &State<Context>) -> Json<JsonValue> {
    ctx.try_log(|logger| slog::debug!(logger, "POST /mined_microblock {:?}", payload));
    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}

#[post("/wallet", format = "application/json", data = "<bitcoin_rpc_call>")]
pub async fn handle_bitcoin_wallet_rpc_call(
    bitcoin_config: &State<BitcoinConfig>,
    bitcoin_rpc_call: Json<BitcoinRPCRequest>,
    ctx: &State<Context>,
) -> Json<JsonValue> {
    ctx.try_log(|logger| slog::debug!(logger, "POST /wallet"));

    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::engine::Engine as _;
    use reqwest::Client;

    let bitcoin_rpc_call = bitcoin_rpc_call.into_inner().clone();

    let body = rocket::serde::json::serde_json::to_vec(&bitcoin_rpc_call).unwrap_or_default();

    let token = BASE64.encode(format!(
        "{}:{}",
        bitcoin_config.username, bitcoin_config.password
    ));

    let url = bitcoin_config.rpc_url.to_string();
    let client = Client::new();
    let builder = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Basic {}", token))
        .timeout(std::time::Duration::from_secs(5));

    match builder.body(body).send().await {
        Ok(res) => Json(res.json().await.unwrap()),
        Err(_) => Json(json!({
            "status": 500
        })),
    }
}

#[post("/", format = "application/json", data = "<bitcoin_rpc_call>")]
pub async fn handle_bitcoin_rpc_call(
    bitcoin_config: &State<BitcoinConfig>,
    bitcoin_rpc_call: Json<BitcoinRPCRequest>,
    background_job_tx: &State<Arc<Mutex<Sender<ObserverCommand>>>>,
    ctx: &State<Context>,
) -> Json<JsonValue> {
    ctx.try_log(|logger| slog::debug!(logger, "POST /"));

    use base64::engine::general_purpose::STANDARD as BASE64;
    use base64::engine::Engine as _;
    use reqwest::Client;

    let bitcoin_rpc_call = bitcoin_rpc_call.into_inner().clone();
    let method = bitcoin_rpc_call.method.clone();

    let body = rocket::serde::json::serde_json::to_vec(&bitcoin_rpc_call).unwrap_or_default();

    let token = BASE64.encode(format!(
        "{}:{}",
        bitcoin_config.username, bitcoin_config.password
    ));

    ctx.try_log(|logger| {
        slog::debug!(
            logger,
            "Forwarding {} request to {}",
            method,
            bitcoin_config.rpc_url
        )
    });

    let url = if method == "listunspent" {
        format!("{}/wallet/", bitcoin_config.rpc_url)
    } else {
        bitcoin_config.rpc_url.to_string()
    };

    let client = Client::new();
    let builder = client
        .post(&url)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Basic {}", token))
        .timeout(std::time::Duration::from_secs(5));

    if method == "sendrawtransaction" {
        let background_job_tx = background_job_tx.inner();
        if let Ok(tx) = background_job_tx.lock() {
            let _ = tx.send(ObserverCommand::NotifyBitcoinTransactionProxied);
        };
    }

    match builder.body(body).send().await {
        Ok(res) => {
            let payload = res.json().await.unwrap();
            ctx.try_log(|logger| slog::debug!(logger, "Responding with response {:?}", payload));
            Json(payload)
        }
        Err(_) => Json(json!({
            "status": 500
        })),
    }
}
