use super::DevnetEvent;
use crate::indexer::{chains, BitcoinChainEvent, Indexer, IndexerConfig, StacksChainEvent};
use crate::integrate::{MempoolAdmissionData, ServiceStatusData, Status};
use crate::poke::load_session;
use crate::publish::{publish_contract, Network};
use crate::types::{self, BlockIdentifier, DevnetConfig};
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
use std::collections::{BTreeMap, VecDeque};
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
pub struct NewMicroBlock {
    transactions: Vec<NewTransaction>,
}

#[derive(Deserialize)]
pub struct NewTransaction {
    pub txid: String,
    pub status: String,
    pub raw_result: String,
    pub raw_tx: String,
}

#[derive(Clone, Debug)]
pub struct EventObserverConfig {
    pub devnet_config: DevnetConfig,
    pub accounts: Vec<Account>,
    pub contracts_to_deploy: VecDeque<InitialContract>,
    pub manifest_path: PathBuf,
    pub session: Session,
    pub deployment_fee_rate: u64,
}

#[derive(Clone, Debug)]
pub struct DevnetInitializationStatus {
    pub contracts_left_to_deploy: VecDeque<InitialContract>,
    pub deployer_nonce: u64,
}

#[derive(Deserialize, Debug)]
struct ContractReadonlyCall {
    okay: bool,
    result: String,
}

impl EventObserverConfig {
    pub fn new(devnet_config: DevnetConfig, manifest_path: PathBuf) -> Self {
        info!("Checking contracts...");
        let (session, config) = match load_session(manifest_path.clone(), false, &Network::Devnet) {
            Ok((session, config)) => (session, config),
            Err(e) => {
                println!("{}", e);
                std::process::exit(1);
            }
        };

        EventObserverConfig {
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

pub enum EventsObserverCommand {
    Terminate(bool), // Restart
    UpdatePoxInfo,
    PublishInitialContracts,
    PublishPoxStackingOrders(BlockIdentifier),
}

pub async fn start_events_observer(
    config: EventObserverConfig,
    devnet_event_tx: Sender<DevnetEvent>,
    events_observer_commands_rx: Receiver<EventsObserverCommand>,
    events_observer_commands_tx: Sender<EventsObserverCommand>,
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
        contracts_left_to_deploy: config.contracts_to_deploy.clone(),
        deployer_nonce: 0,
    };

    let port = config.devnet_config.orchestrator_port;
    let manifest_path = config.manifest_path.clone();

    let config_mutex = Arc::new(Mutex::new(config.clone()));
    let init_status_rw_lock = Arc::new(RwLock::new(init_status));
    let indexer_rw_lock = Arc::new(RwLock::new(indexer));

    let devnet_event_tx_mutex = Arc::new(Mutex::new(devnet_event_tx.clone()));
    let background_job_tx_mutex = Arc::new(Mutex::new(events_observer_commands_tx.clone()));

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
                    handle_new_burn_block,
                    handle_new_block,
                    handle_new_microblocks,
                    handle_new_mempool_tx,
                    handle_drop_mempool_tx,
                ],
            )
            .launch();
        let rt = utils::create_basic_runtime();
        rt.block_on(future).expect("Unable to spawn event observer");
    });

    // This loop is used for handling background jobs, emitted by HTTP calls.
    loop {
        match events_observer_commands_rx.recv() {
            Ok(EventsObserverCommand::Terminate(true)) => {
                devnet_event_tx
                    .send(DevnetEvent::info("Terminating event observer".into()))
                    .expect("Unable to terminate event observer");
                break;
            }
            Ok(EventsObserverCommand::Terminate(false)) => {
                // Restart
                devnet_event_tx
                    .send(DevnetEvent::info("Reloading contracts".into()))
                    .expect("Unable to terminate event observer");

                let session = match load_session(manifest_path.clone(), false, &Network::Devnet) {
                    Ok((session, _)) => session,
                    Err(e) => {
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

                if let Ok(mut init_status) = init_status_rw_lock.write() {
                    init_status.contracts_left_to_deploy = contracts_to_deploy;
                    init_status.deployer_nonce = 0;
                }
            }
            Ok(EventsObserverCommand::UpdatePoxInfo) => {}
            Ok(EventsObserverCommand::PublishInitialContracts) => {
                if let Ok(mut init_status_writer) = init_status_rw_lock.write() {
                    let res = publish_initial_contracts(
                        &config.devnet_config,
                        &config.accounts,
                        config.deployment_fee_rate,
                        &mut init_status_writer,
                    );
                    if let Some(tx_count) = res {
                        let _ = devnet_event_tx.send(DevnetEvent::success(format!(
                            "Will publish {} contracts",
                            tx_count
                        )));
                    }
                }
            }
            Ok(EventsObserverCommand::PublishPoxStackingOrders(block_identifier)) => {
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
    let block = match chain_update {
        BitcoinChainEvent::ChainUpdatedWithBlock(block) => block,
        _ => {
            return Json(json!({
                "status": 200,
                "result": "Ok",
            }))
        }
    };

    match devnet_events_tx.lock() {
        Ok(tx) => {
            let _ = tx.send(DevnetEvent::debug(format!(
                "Bitcoin block #{} received",
                block.block_identifier.index
            )));

            let _ = tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
                order: 0,
                status: Status::Green,
                name: "bitcoin-node".into(),
                comment: format!(
                    "mining blocks (chaintip = #{})",
                    block.block_identifier.index
                ),
            }));
            let _ = tx.send(DevnetEvent::BitcoinBlock(block));
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
    init_status: &State<Arc<RwLock<DevnetInitializationStatus>>>,
    indexer_rw_lock: &State<Arc<RwLock<Indexer>>>,
    devnet_events_tx: &State<Arc<Mutex<Sender<DevnetEvent>>>>,
    background_job_tx_mutex: &State<Arc<Mutex<Sender<EventsObserverCommand>>>>,
    marshalled_block: Json<JsonValue>,
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

    // Contextual: Devnet is an environment under control,
    // with 1 miner. As such we will ignore Reorgs handling.
    let block = match chain_event {
        StacksChainEvent::ChainUpdatedWithBlock(block) => block,
        _ => {
            return Json(json!({
                "status": 200,
                "result": "Ok",
            }))
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
                block.block_identifier.index
            ),
        }));
        let _ = tx.send(DevnetEvent::info(format!(
            "Block #{} anchored in Bitcoin block #{} includes {} transactions",
            block.block_identifier.index,
            block.metadata.bitcoin_anchor_block_identifier.index,
            block.transactions.len(),
        )));
    }

    // A few things need to be done:
    // - Contracts deployment orchestration during the first blocks (max: 25 / block)
    // - PoX stacking order orchestration: enable PoX with some "stack-stx" transactions
    // defined in the devnet file config.
    if let Ok(init_status_writer) = init_status.inner().read() {
        if init_status_writer.contracts_left_to_deploy.len() > 0 {
            if let Ok(background_job_tx) = background_job_tx_mutex.lock() {
                let _ = background_job_tx.send(EventsObserverCommand::PublishInitialContracts);
            }
        }
    }

    // Every penultimate block, we check if some stacking orders should be submitted before the next
    // cycle starts.
    let pox_cycle_length: u32 =
        pox_info.prepare_phase_block_length + pox_info.reward_phase_block_length;
    let should_submit_pox_orders = block.metadata.pox_cycle_position == (pox_cycle_length - 2);
    if should_submit_pox_orders {
        if let Ok(background_job_tx) = background_job_tx_mutex.lock() {
            let _ = background_job_tx.send(EventsObserverCommand::PublishPoxStackingOrders(
                block.metadata.bitcoin_anchor_block_identifier.clone(),
            ));
        }
    }

    if let Ok(tx) = devnet_events_tx.lock() {
        let _ = tx.send(DevnetEvent::StacksBlock(block));
    }

    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}

#[post(
    "/new_microblocks",
    format = "application/json",
    data = "<new_microblock>"
)]
pub fn handle_new_microblocks(
    _config: &State<Arc<Mutex<EventObserverConfig>>>,
    _indexer: &State<Arc<RwLock<Indexer>>>,
    devnet_events_tx: &State<Arc<Mutex<Sender<DevnetEvent>>>>,
    new_microblock: Json<NewMicroBlock>,
) -> Json<JsonValue> {
    let devnet_events_tx = devnet_events_tx.inner();

    if let Ok(tx) = devnet_events_tx.lock() {
        let _ = tx.send(DevnetEvent::info(format!(
            "Microblock received including {} transactions",
            new_microblock.transactions.len(),
        )));
    }

    // let transactions = new_block
    //     .transactions
    //     .iter()
    //     .map(|t| {
    //         let description = get_tx_description(&t.raw_tx);
    //         StacksTransactionData {
    //             transaction_identifier: TransactionIdentifier {
    //                 hash: t.txid.clone(),
    //             },
    //             metadata: {
    //                 success: t.status == "success",
    //                 result: get_value_description(&t.raw_result),
    //                 events: vec![],
    //                 description,
    //             }
    //         }
    //     })
    //     .collect();

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
        .map(|t| chains::stacks::get_tx_description(t))
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

pub fn publish_initial_contracts(
    devnet_config: &DevnetConfig,
    accounts: &Vec<Account>,
    deployment_fee_rate: u64,
    init_status: &mut DevnetInitializationStatus,
) -> Option<usize> {
    let contracts_left = init_status.contracts_left_to_deploy.len();
    if contracts_left == 0 {
        return None;
    }

    let node_url = format!("http://localhost:{}", devnet_config.stacks_node_rpc_port);
    let tx_chaining_limit = 25;
    let blocks_required = 1 + (contracts_left / tx_chaining_limit);
    let contracts_to_deploy_in_blocks = if blocks_required == 1 {
        contracts_left
    } else {
        contracts_left / blocks_required
    };

    let mut contracts_to_deploy = vec![];

    for _ in 0..contracts_to_deploy_in_blocks {
        let contract = init_status.contracts_left_to_deploy.pop_front().unwrap();
        contracts_to_deploy.push(contract);
    }

    let mut deployers_lookup = BTreeMap::new();
    for account in accounts.iter() {
        if account.name == "deployer" {
            deployers_lookup.insert("*".into(), account.clone());
        }
    }

    let mut deployers_nonces = BTreeMap::new();
    deployers_nonces.insert("deployer".to_string(), init_status.deployer_nonce);
    let contract_to_deploy_len = contracts_to_deploy.len();
    init_status.deployer_nonce += contract_to_deploy_len as u64;

    std::thread::spawn(move || {
        for contract in contracts_to_deploy.into_iter() {
            match publish_contract(
                &contract,
                &deployers_lookup,
                &mut deployers_nonces,
                &node_url,
                deployment_fee_rate,
                &Network::Devnet,
            ) {
                Ok((_txid, _nonce)) => {
                    // let _ = tx_clone.send(DevnetEvent::success(format!(
                    //     "Contract {} broadcasted in mempool (txid: {}, nonce: {})",
                    //     contract.name.unwrap(), txid, nonce
                    // )));
                }
                Err(_err) => {
                    // let _ = tx_clone.send(DevnetEvent::error(err.to_string()));
                    break;
                }
            }
        }
    });

    Some(contract_to_deploy_len)
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
                let default_fee = fee_rate * 10;
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
