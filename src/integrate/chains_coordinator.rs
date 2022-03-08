use super::DevnetEvent;
use crate::indexer::{chains, Indexer, IndexerConfig};
use crate::integrate::{MempoolAdmissionData, ServiceStatusData, Status};
use crate::poke::load_session;
use crate::publish::{publish_all_contracts, Network};
use crate::types::{self, BlockIdentifier, DevnetConfig};
use crate::types::{BitcoinChainEvent, StacksChainEvent};
use crate::utils;
use crate::utils::stacks::{transactions, PoxInfo, StacksRpc};
use base58::FromBase58;
use clarity_repl::clarity::representations::ClarityName;
use clarity_repl::clarity::types::{BuffData, SequenceData, TupleData, Value as ClarityValue};
use clarity_repl::clarity::util::address::AddressHashMode;
use clarity_repl::clarity::util::hash::{hex_bytes, Hash160};
use clarity_repl::repl::settings::{Account, InitialContract};
use clarity_repl::repl::Session;
use rocket::config::{Config, LogLevel};
use rocket::serde::json::{json, Json, Value as JsonValue};
use rocket::serde::Deserialize;
use rocket::State;
use std::collections::VecDeque;
use std::convert::TryFrom;
use std::error::Error;
use std::iter::FromIterator;
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::str;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};
use tracing::info;

#[cfg(feature = "cli")]
use crate::runnner::deno;

#[derive(Deserialize)]
pub struct NewTransaction {
    pub txid: String,
    pub status: String,
    pub raw_result: String,
    pub raw_tx: String,
}

#[derive(Clone, Debug)]
pub struct StacksEventObserverConfig {
    pub devnet_config: DevnetConfig,
    pub accounts: Vec<Account>,
    pub contracts_to_deploy: VecDeque<InitialContract>,
    pub manifest_path: PathBuf,
    pub session: Session,
    pub deployment_fee_rate: u64,
}

#[derive(Clone, Debug)]
pub struct DevnetInitializationStatus {
    pub should_deploy_protocol: bool,
}

#[derive(Deserialize, Debug)]
struct ContractReadonlyCall {
    okay: bool,
    result: String,
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

impl StacksEventObserverConfig {
    pub fn new(devnet_config: DevnetConfig, manifest_path: PathBuf) -> Self {
        info!("Checking contracts...");
        let (session, config) = match load_session(&manifest_path, false, &Network::Devnet) {
            Ok((session, config, _, _)) => (session, config),
            Err((_, e)) => {
                println!("{}", e);
                std::process::exit(1);
            }
        };

        StacksEventObserverConfig {
            devnet_config,
            accounts: session.settings.initial_accounts.clone(),
            manifest_path,
            contracts_to_deploy: VecDeque::from_iter(
                session.settings.initial_contracts.iter().map(|c| c.clone()),
            ),
            session,
            deployment_fee_rate: config.network.deployment_fee_rate,
        }
    }

    pub async fn execute_scripts(&self) {
        if self.devnet_config.execute_script.len() > 0 {
            for _cmd in self.devnet_config.execute_script.iter() {
                #[cfg(feature = "cli")]
                let _ = deno::do_run_scripts(
                    vec![_cmd.script.clone()],
                    false,
                    false,
                    false,
                    _cmd.allow_wallets,
                    _cmd.allow_write,
                    self.manifest_path.clone(),
                    Some(self.session.clone()),
                )
                .await;
            }
        }
    }
}

pub enum StacksEventsObserverCommand {
    Terminate(bool), // Restart
    PublishInitialContracts,
    BitcoinOpSent,
    ProtocolDeployed,
    PublishPoxStackingOrders(BlockIdentifier),
}

pub async fn start_chains_coordinator(
    config: StacksEventObserverConfig,
    devnet_event_tx: Sender<DevnetEvent>,
    chains_coordinator_commands_rx: Receiver<StacksEventsObserverCommand>,
    chains_coordinator_commands_tx: Sender<StacksEventsObserverCommand>,
) -> Result<(), Box<dyn Error>> {
    let _ = config.execute_scripts().await;

    let indexer = Indexer::new(IndexerConfig {
        stacks_node_rpc_url: format!(
            "http://localhost:{}",
            config.devnet_config.stacks_node_rpc_port
        ),
        bitcoin_node_rpc_url: format!(
            "http://localhost:{}",
            config.devnet_config.bitcoin_node_rpc_port
        ),
        bitcoin_node_rpc_username: config.devnet_config.bitcoin_node_username.clone(),
        bitcoin_node_rpc_password: config.devnet_config.bitcoin_node_password.clone(),
    });

    let init_status = DevnetInitializationStatus {
        should_deploy_protocol: true,
    };

    let port = config.devnet_config.orchestrator_port;
    let manifest_path = config.manifest_path.clone();

    let config_mutex = Arc::new(Mutex::new(config.clone()));
    let init_status_rw_lock = Arc::new(RwLock::new(init_status));
    let indexer_rw_lock = Arc::new(RwLock::new(indexer));

    let devnet_event_tx_mutex = Arc::new(Mutex::new(devnet_event_tx.clone()));
    let background_job_tx_mutex = Arc::new(Mutex::new(chains_coordinator_commands_tx.clone()));

    let moved_init_status_rw_lock = init_status_rw_lock.clone();

    let rocket_config = Config {
        port: port,
        workers: 4,
        address: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
        keep_alive: 5,
        temp_dir: std::env::temp_dir(),
        log_level: LogLevel::Debug,
        ..Config::default()
    };

    let _ = std::thread::spawn(move || {
        let future = rocket::custom(rocket_config)
            .manage(indexer_rw_lock)
            .manage(devnet_event_tx_mutex)
            .manage(config_mutex)
            .manage(moved_init_status_rw_lock)
            .manage(background_job_tx_mutex)
            .mount(
                "/",
                routes![
                    handle_ping,
                    handle_new_burn_block,
                    handle_new_block,
                    handle_new_microblocks,
                    handle_new_mempool_tx,
                    handle_drop_mempool_tx,
                    handle_bitcoin_rpc_call,
                ],
            )
            .launch();
        let rt = utils::create_basic_runtime();
        rt.block_on(future).expect("Unable to spawn event observer");
    });

    // This loop is used for handling background jobs, emitted by HTTP calls.
    let mut should_deploy_protocol = true;
    let mut protocol_deployed = false;

    loop {
        match chains_coordinator_commands_rx.recv() {
            Ok(StacksEventsObserverCommand::Terminate(true)) => {
                devnet_event_tx
                    .send(DevnetEvent::info("Terminating event observer".into()))
                    .expect("Unable to terminate event observer");
                break;
            }
            Ok(StacksEventsObserverCommand::Terminate(false)) => {
                // Restart
                devnet_event_tx
                    .send(DevnetEvent::info("Reloading contracts".into()))
                    .expect("Unable to terminate event observer");

                let session = match load_session(&manifest_path, false, &Network::Devnet) {
                    Ok((session, _, _, _)) => session,
                    Err((_, e)) => {
                        devnet_event_tx
                            .send(DevnetEvent::error(format!("Contracts invalid: {}", e)))
                            .expect("Unable to terminate event observer");
                        continue;
                    }
                };
                let contracts_to_deploy = VecDeque::from_iter(
                    session.settings.initial_contracts.iter().map(|c| c.clone()),
                );
                devnet_event_tx
                    .send(DevnetEvent::success(format!(
                        "{} contracts to deploy",
                        contracts_to_deploy.len()
                    )))
                    .expect("Unable to terminate event observer");

                should_deploy_protocol = true;
                protocol_deployed = false;
                if let Ok(mut init_status) = init_status_rw_lock.write() {
                    init_status.should_deploy_protocol = true;
                }
            }
            Ok(StacksEventsObserverCommand::PublishInitialContracts) => {
                if should_deploy_protocol {
                    should_deploy_protocol = false;
                    if let Ok(mut init_status) = init_status_rw_lock.write() {
                        init_status.should_deploy_protocol = false;
                    }
                    publish_initial_contracts(&config.manifest_path, &devnet_event_tx);
                }
            }
            Ok(StacksEventsObserverCommand::ProtocolDeployed) => {
                should_deploy_protocol = false;
                protocol_deployed = true;
            }
            Ok(StacksEventsObserverCommand::BitcoinOpSent) => {
                if !protocol_deployed {
                    use bitcoincore_rpc::bitcoin::Address;
                    use bitcoincore_rpc::{Auth, Client, RpcApi};
                    use std::str::FromStr;

                    std::thread::sleep(std::time::Duration::from_secs(1));
                    let rpc = Client::new(
                        &format!(
                            "http://0.0.0.0:{}",
                            config.devnet_config.bitcoin_node_rpc_port
                        ),
                        Auth::UserPass(
                            config.devnet_config.bitcoin_node_username.to_string(),
                            config.devnet_config.bitcoin_node_password.to_string(),
                        ),
                    )
                    .unwrap();
                    let miner_address =
                        Address::from_str(&config.devnet_config.miner_btc_address).unwrap();
                    let _ = rpc.generate_to_address(1, &miner_address);
                }
            }
            Ok(StacksEventsObserverCommand::PublishPoxStackingOrders(block_identifier)) => {
                let bitcoin_block_height = block_identifier.index;
                let res = publish_stacking_orders(
                    &config.devnet_config,
                    &config.accounts,
                    config.deployment_fee_rate,
                    bitcoin_block_height as u32,
                )
                .await;
                if let Some(tx_count) = res {
                    let _ = devnet_event_tx.send(DevnetEvent::success(format!(
                        "Will broadcast {} stacking orders",
                        tx_count
                    )));
                }
            }
            Err(_) => {
                break;
            }
        }
    }
    Ok(())
}

#[post("/new_burn_block", format = "json", data = "<marshalled_block>")]
pub fn handle_new_burn_block(
    indexer_rw_lock: &State<Arc<RwLock<Indexer>>>,
    devnet_events_tx: &State<Arc<Mutex<Sender<DevnetEvent>>>>,
    marshalled_block: Json<JsonValue>,
) -> Json<JsonValue> {
    let devnet_events_tx = devnet_events_tx.inner();

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

    // Contextual shortcut: Devnet is an environment under control,
    // with 1 miner. As such we will ignore Reorgs handling.
    let (log, status) = match &chain_update {
        BitcoinChainEvent::ChainUpdatedWithBlock(block) => {
            let log = format!("Bitcoin block #{} received", block.block_identifier.index);
            let status = format!(
                "mining blocks (chaintip = #{})",
                block.block_identifier.index
            );
            (log, status)
        }
        BitcoinChainEvent::ChainUpdatedWithReorg(_old_blocks, new_blocks) => {
            let tip = new_blocks.last().unwrap();
            let log = format!(
                "Bitcoin reorg received (new height: {})",
                tip.block_identifier.index
            );
            let status = format!("mining blocks (chaintip = #{})", tip.block_identifier.index);
            (log, status)
        }
    };

    match devnet_events_tx.lock() {
        Ok(tx) => {
            let _ = tx.send(DevnetEvent::debug(log));

            let _ = tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
                order: 0,
                status: Status::Green,
                name: "bitcoin-node".into(),
                comment: status,
            }));
            let _ = tx.send(DevnetEvent::BitcoinChainEvent(chain_update));
        }
        _ => {}
    };

    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}

#[post("/new_block", format = "application/json", data = "<marshalled_block>")]
pub fn handle_new_block(
    indexer_rw_lock: &State<Arc<RwLock<Indexer>>>,
    devnet_events_tx: &State<Arc<Mutex<Sender<DevnetEvent>>>>,
    init_status: &State<Arc<RwLock<DevnetInitializationStatus>>>,
    background_job_tx_mutex: &State<Arc<Mutex<Sender<StacksEventsObserverCommand>>>>,
    marshalled_block: Json<JsonValue>,
) -> Json<JsonValue> {
    // A few things need to be done:
    // - Contracts deployment orchestration during the first blocks (max: 25 / block)
    // - PoX stacking order orchestration: enable PoX with some "stack-stx" transactions
    // defined in the devnet file config.
    if let Ok(init_status_writer) = init_status.inner().read() {
        if init_status_writer.should_deploy_protocol {
            if let Ok(background_job_tx) = background_job_tx_mutex.lock() {
                let _ =
                    background_job_tx.send(StacksEventsObserverCommand::PublishInitialContracts);
            }
        }
    }

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

    // Contextual: Devnet is an environment under control,
    // with 1 miner. As such we will ignore Reorgs handling.
    let update = match &chain_event {
        StacksChainEvent::ChainUpdatedWithBlock(block) => block.clone(),
        StacksChainEvent::ChainUpdatedWithMicroblock(_) => {
            unreachable!() // TODO(lgalabru): good enough for now
        }
        StacksChainEvent::ChainUpdatedWithMicroblockReorg(_) => {
            unreachable!() // TODO(lgalabru): good enough for now
        }
        StacksChainEvent::ChainUpdatedWithReorg(_) => {
            unreachable!() // TODO(lgalabru): good enough for now
        }
    };

    // Partially update the UI. With current approach a full update
    // would requires either cloning the block, or passing ownership.
    let devnet_events_tx = devnet_events_tx.inner();
    if let Ok(tx) = devnet_events_tx.lock() {
        let _ = tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
            order: 1,
            status: Status::Green,
            name: "stacks-node".into(),
            comment: format!(
                "mining blocks (chaintip = #{})",
                update.new_block.block_identifier.index
            ),
        }));
        let _ = tx.send(DevnetEvent::info(format!(
            "Block #{} anchored in Bitcoin block #{} includes {} transactions",
            update.new_block.block_identifier.index,
            update
                .new_block
                .metadata
                .bitcoin_anchor_block_identifier
                .index,
            update.new_block.transactions.len(),
        )));
    }

    // Every penultimate block, we check if some stacking orders should be submitted before the next
    // cycle starts.
    let pox_cycle_length: u32 =
        pox_info.prepare_phase_block_length + pox_info.reward_phase_block_length;
    let should_submit_pox_orders =
        update.new_block.metadata.pox_cycle_position == (pox_cycle_length - 2);
    if should_submit_pox_orders {
        if let Ok(background_job_tx) = background_job_tx_mutex.lock() {
            let _ = background_job_tx.send(StacksEventsObserverCommand::PublishPoxStackingOrders(
                update
                    .new_block
                    .metadata
                    .bitcoin_anchor_block_identifier
                    .clone(),
            ));
        }
    }

    if let Ok(tx) = devnet_events_tx.lock() {
        let _ = tx.send(DevnetEvent::StacksChainEvent(chain_event));
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
    _config: &State<Arc<Mutex<StacksEventObserverConfig>>>,
    indexer_rw_lock: &State<Arc<RwLock<Indexer>>>,
    devnet_events_tx: &State<Arc<Mutex<Sender<DevnetEvent>>>>,
    marshalled_microblock: Json<JsonValue>,
) -> Json<JsonValue> {
    let devnet_events_tx = devnet_events_tx.inner();

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

    if let Ok(tx) = devnet_events_tx.lock() {
        if let StacksChainEvent::ChainUpdatedWithMicroblock(ref update) = chain_event {
            if let Some(microblock) = update.current_trail.microblocks.last() {
                let _ = tx.send(DevnetEvent::info(format!(
                    "Microblock received including {} transactions",
                    microblock.transactions.len(),
                )));
            }
        }
        let _ = tx.send(DevnetEvent::StacksChainEvent(chain_event));
    }

    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}

#[post("/new_mempool_tx", format = "application/json", data = "<raw_txs>")]
pub fn handle_new_mempool_tx(
    devnet_events_tx: &State<Arc<Mutex<Sender<DevnetEvent>>>>,
    raw_txs: Json<Vec<String>>,
) -> Json<JsonValue> {
    let decoded_transactions = raw_txs
        .iter()
        .map(|t| {
            let (txid, ..) =
                chains::stacks::get_tx_description(t).expect("unable to parse transaction");
            txid
        })
        .collect::<Vec<String>>();

    if let Ok(tx_sender) = devnet_events_tx.lock() {
        for tx in decoded_transactions.into_iter() {
            let _ = tx_sender.send(DevnetEvent::MempoolAdmission(MempoolAdmissionData { tx }));
        }
    }

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

#[get("/ping", format = "application/json")]
pub fn handle_ping() -> Json<JsonValue> {
    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}

#[post("/", format = "application/json", data = "<bitcoin_rpc_call>")]
pub async fn handle_bitcoin_rpc_call(
    config: &State<Arc<Mutex<StacksEventObserverConfig>>>,
    _devnet_events_tx: &State<Arc<Mutex<Sender<DevnetEvent>>>>,
    background_job_tx_mutex: &State<Arc<Mutex<Sender<StacksEventsObserverCommand>>>>,
    bitcoin_rpc_call: Json<BitcoinRPCRequest>,
) -> Json<JsonValue> {
    use base64::encode;
    use reqwest::Client;

    // if let Ok(tx_sender) = devnet_events_tx.lock() {
    //     let _ = tx_sender.send(DevnetEvent::debug(format!(
    //         "Forwarding request {:?}",
    //         bitcoin_rpc_call.method
    //     )));
    // }

    let bitcoin_rpc_call = bitcoin_rpc_call.into_inner().clone();
    let method = bitcoin_rpc_call.method.clone();
    let body = rocket::serde::json::serde_json::to_vec(&bitcoin_rpc_call).unwrap();

    let builder = match config.inner().lock() {
        Ok(config) => {
            let token = encode(format!(
                "{}:{}",
                config.devnet_config.bitcoin_node_username,
                config.devnet_config.bitcoin_node_password
            ));

            let client = Client::new();
            client
                .post(format!(
                    "http://0.0.0.0:{}/",
                    config.devnet_config.bitcoin_node_rpc_port
                ))
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Basic {}", token))
        }
        _ => unreachable!(),
    };

    if method == "sendrawtransaction" {
        if let Ok(background_job_tx) = background_job_tx_mutex.lock() {
            let _ = background_job_tx.send(StacksEventsObserverCommand::BitcoinOpSent);
        }
    }

    let res = builder.body(body).send().await.unwrap();

    Json(res.json().await.unwrap())
}

pub fn publish_initial_contracts(manifest_path: &PathBuf, devnet_event_tx: &Sender<DevnetEvent>) {
    let moved_manifest_path = manifest_path.clone();
    let moved_devnet_event_tx = devnet_event_tx.clone();
    std::thread::spawn(move || {
        let _ = publish_all_contracts(
            &moved_manifest_path,
            &Network::Devnet,
            false,
            1,
            Some(&moved_devnet_event_tx),
        );
    });
}

pub async fn publish_stacking_orders(
    devnet_config: &DevnetConfig,
    accounts: &Vec<Account>,
    fee_rate: u64,
    bitcoin_block_height: u32,
) -> Option<usize> {
    if devnet_config.pox_stacking_orders.len() == 0 {
        return None;
    }

    let stacks_node_rpc_url = format!("http://localhost:{}", devnet_config.stacks_node_rpc_port);

    let mut transactions = 0;
    let pox_info: PoxInfo = reqwest::get(format!("{}/v2/pox", stacks_node_rpc_url))
        .await
        .expect("Unable to retrieve pox info")
        .json()
        .await
        .expect("Unable to parse contract");

    for pox_stacking_order in devnet_config.pox_stacking_orders.iter() {
        if pox_stacking_order.start_at_cycle == (pox_info.reward_cycle_id + 1) {
            let mut account = None;
            let mut accounts_iter = accounts.iter();
            while let Some(e) = accounts_iter.next() {
                if e.name == pox_stacking_order.wallet {
                    account = Some(e.clone());
                    break;
                }
            }
            let account = match account {
                Some(account) => account,
                _ => continue,
            };

            transactions += 1;

            let stx_amount = pox_info.next_cycle.min_threshold_ustx * pox_stacking_order.slots;
            let addr_bytes = pox_stacking_order
                .btc_address
                .from_base58()
                .expect("Unable to get bytes from btc address");
            let duration = pox_stacking_order.duration.into();
            let node_url = stacks_node_rpc_url.clone();
            let pox_contract_id = pox_info.contract_id.clone();

            std::thread::spawn(move || {
                let default_fee = fee_rate * 1000;
                let stacks_rpc = StacksRpc::new(&node_url);
                let nonce = stacks_rpc
                    .get_nonce(&account.address)
                    .expect("Unable to retrieve nonce");

                let (_, _, account_secret_key) =
                    types::compute_addresses(&account.mnemonic, &account.derivation, false);

                let addr_bytes = Hash160::from_bytes(&addr_bytes[1..21]).unwrap();
                let addr_version = AddressHashMode::SerializeP2PKH;
                let stack_stx_tx = transactions::build_contrat_call_transaction(
                    pox_contract_id,
                    "stack-stx".into(),
                    vec![
                        ClarityValue::UInt(stx_amount.into()),
                        ClarityValue::Tuple(
                            TupleData::from_data(vec![
                                (
                                    ClarityName::try_from("version".to_owned()).unwrap(),
                                    ClarityValue::buff_from_byte(addr_version as u8),
                                ),
                                (
                                    ClarityName::try_from("hashbytes".to_owned()).unwrap(),
                                    ClarityValue::Sequence(SequenceData::Buffer(BuffData {
                                        data: addr_bytes.as_bytes().to_vec(),
                                    })),
                                ),
                            ])
                            .unwrap(),
                        ),
                        ClarityValue::UInt((bitcoin_block_height - 1).into()),
                        ClarityValue::UInt(duration),
                    ],
                    nonce,
                    default_fee,
                    &hex_bytes(&account_secret_key).unwrap(),
                );
                let _ = stacks_rpc
                    .post_transaction(stack_stx_tx)
                    .expect("Unable to broadcast transaction");
            });
        }
    }
    if transactions > 0 {
        Some(transactions)
    } else {
        None
    }
}
