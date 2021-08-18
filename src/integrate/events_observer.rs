use super::{DevnetEvent, NodeObserverEvent};
use crate::integrate::{BlockData, MempoolAdmissionData, ServiceStatusData, Status, Transaction};
use crate::poke::load_session;
use crate::publish::{publish_contract, Network};
use crate::types::{self, AccountConfig, DevnetConfig};
use crate::utils;
use crate::utils::stacks::{transactions, StacksRpc};
use base58::FromBase58;
use clarity_repl::clarity::codec::transaction::TransactionPayload;
use clarity_repl::clarity::codec::{StacksMessageCodec, StacksTransaction};
use clarity_repl::clarity::representations::ClarityName;
use clarity_repl::clarity::types::{BuffData, SequenceData, TupleData, Value as ClarityValue};
use clarity_repl::clarity::util::address::AddressHashMode;
use clarity_repl::clarity::util::hash::{hex_bytes, Hash160};
use clarity_repl::repl::settings::InitialContract;
use clarity_repl::repl::SessionSettings;
use rocket::config::{Config, LogLevel};
use rocket::serde::json::{json, Json, Value};
use rocket::serde::Deserialize;
use rocket::State;
use std::collections::{BTreeMap, VecDeque};
use std::convert::{TryFrom, TryInto};
use std::error::Error;
use std::io::Cursor;
use std::iter::FromIterator;
use std::net::{IpAddr, Ipv4Addr};
use std::path::PathBuf;
use std::str;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};
use tracing::info;

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct NewBurnBlock {
    burn_block_hash: String,
    burn_block_height: u64,
    reward_slot_holders: Vec<String>,
    burn_amount: u64,
}

#[derive(Deserialize)]
pub struct NewBlock {
    block_height: u64,
    block_hash: String,
    burn_block_height: u64,
    burn_block_hash: String,
    transactions: Vec<NewTransaction>,
    // reward_slot_holders: Vec<String>,
    // burn_amount: u32,
}

#[derive(Deserialize)]
pub struct NewMicroBlock {
    transactions: Vec<NewTransaction>,
}

#[derive(Deserialize)]
pub struct NewTransaction {
    txid: String,
    status: String,
    raw_result: String,
    raw_tx: String,
}

#[derive(Clone, Debug)]
pub struct EventObserverConfig {
    pub devnet_config: DevnetConfig,
    pub accounts: BTreeMap<String, AccountConfig>,
    pub contracts_to_deploy: VecDeque<InitialContract>,
    pub manifest_path: PathBuf,
    pub pox_info: PoxInfo,
    pub session_settings: SessionSettings,
    pub deployer_nonce: u64,
}

impl EventObserverConfig {
    pub fn new(
        devnet_config: DevnetConfig,
        manifest_path: PathBuf,
        accounts: BTreeMap<String, AccountConfig>,
    ) -> Self {
        info!("Checking contracts...");
        let session_settings = match load_session(manifest_path.clone(), false, Network::Devnet) {
            Ok(settings) => settings,
            Err(e) => {
                println!("{}", e);
                std::process::exit(1);
            }
        };
        EventObserverConfig {
            devnet_config,
            accounts,
            manifest_path,
            pox_info: PoxInfo::default(),
            contracts_to_deploy: VecDeque::from_iter(
                session_settings.initial_contracts.iter().map(|c| c.clone()),
            ),
            session_settings,
            deployer_nonce: 0,
        }
    }
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct PoxInfo {
    contract_id: String,
    pox_activation_threshold_ustx: u64,
    first_burnchain_block_height: u64,
    prepare_phase_block_length: u32,
    reward_phase_block_length: u32,
    reward_slots: u32,
    total_liquid_supply_ustx: u64,
    next_cycle: PoxCycle,
}

impl PoxInfo {
    pub fn default() -> PoxInfo {
        PoxInfo {
            contract_id: "ST000000000000000000002AMW42H.pox".into(),
            pox_activation_threshold_ustx: 0,
            first_burnchain_block_height: 100,
            prepare_phase_block_length: 1,
            reward_phase_block_length: 4,
            reward_slots: 8,
            total_liquid_supply_ustx: 1000000000000000,
            ..Default::default()
        }
    }
}

#[derive(Deserialize, Debug, Clone, Default)]
pub struct PoxCycle {
    min_threshold_ustx: u64,
}

pub async fn start_events_observer(
    events_config: EventObserverConfig,
    devnet_event_tx: Sender<DevnetEvent>,
    terminator_rx: Receiver<bool>,
    event_tx: Option<Sender<NodeObserverEvent>>,
) -> Result<(), Box<dyn Error>> {
    let port = events_config.devnet_config.orchestrator_port;
    let manifest_path = events_config.manifest_path.clone();
    let rw_lock = Arc::new(RwLock::new(events_config));

    let moved_rw_lock = rw_lock.clone();
    let moved_tx = Arc::new(Mutex::new(devnet_event_tx.clone()));
    let moved_node_tx = Arc::new(Mutex::new(event_tx.clone()));

    let config = Config {
        port: port,
        workers: 4,
        address: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
        keep_alive: 5,
        temp_dir: std::env::temp_dir(),
        log_level: LogLevel::Off,
        ..Config::default()
    };

    let _ = std::thread::spawn(move || {
        let future = rocket::custom(config)
            .manage(moved_rw_lock)
            .manage(moved_tx)
            .manage(moved_node_tx)
            .mount(
                "/",
                routes![
                    handle_new_burn_block,
                    handle_new_block,
                    handle_new_microblocks,
                    handle_new_mempool_tx,
                    handle_drop_mempool_tx
                ],
            )
            .launch();
        let rt = utils::create_basic_runtime();
        rt.block_on(future).expect("Unable to spawn event observer");
    });

    loop {
        match terminator_rx.recv() {
            Ok(true) => {
                devnet_event_tx
                    .send(DevnetEvent::info("Terminating event observer".into()))
                    .expect("Unable to terminate event observer");
                break;
            }
            Ok(false) => {
                // Restart
                devnet_event_tx
                    .send(DevnetEvent::info("Reloading contracts".into()))
                    .expect("Unable to terminate event observer");

                let session_settings =
                    match load_session(manifest_path.clone(), false, Network::Devnet) {
                        Ok(settings) => settings,
                        Err(e) => {
                            devnet_event_tx
                                .send(DevnetEvent::error(format!("Contracts invalid: {}", e)))
                                .expect("Unable to terminate event observer");
                            continue;
                        }
                    };
                let contracts_to_deploy = VecDeque::from_iter(
                    session_settings.initial_contracts.iter().map(|c| c.clone()),
                );
                devnet_event_tx
                    .send(DevnetEvent::success(format!(
                        "{} contracts to deploy",
                        contracts_to_deploy.len()
                    )))
                    .expect("Unable to terminate event observer");

                if let Ok(mut config_writer) = rw_lock.write() {
                    config_writer.contracts_to_deploy = contracts_to_deploy;
                    config_writer.session_settings = session_settings;
                    config_writer.deployer_nonce = 0;
                }
            }
            Err(_) => {
                break;
            }
        }
    }
    Ok(())
}

#[post("/new_burn_block", format = "json", data = "<new_burn_block>")]
pub fn handle_new_burn_block(
    devnet_events_tx: &State<Arc<Mutex<Sender<DevnetEvent>>>>,
    new_burn_block: Json<NewBurnBlock>,
    _node_event_tx: &State<Arc<Mutex<Option<Sender<NodeObserverEvent>>>>>,
) -> Json<Value> {
    let devnet_events_tx = devnet_events_tx.inner();

    match devnet_events_tx.lock() {
        Ok(tx) => {
            let _ = tx.send(DevnetEvent::debug(format!(
                "Bitcoin block #{} received",
                new_burn_block.burn_block_height
            )));
            let _ = tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
                order: 0,
                status: Status::Green,
                name: "bitcoin-node".into(),
                comment: format!(
                    "mining blocks (chaintip = #{})",
                    new_burn_block.burn_block_height
                ),
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
pub fn handle_new_block(
    config: &State<Arc<RwLock<EventObserverConfig>>>,
    devnet_events_tx: &State<Arc<Mutex<Sender<DevnetEvent>>>>,
    new_block: Json<NewBlock>,
    _node_event_tx: &State<Arc<Mutex<Option<Sender<NodeObserverEvent>>>>>,
) -> Json<Value> {
    let devnet_events_tx = devnet_events_tx.inner();
    let config = config.inner();

    if let Ok(tx) = devnet_events_tx.lock() {
        let _ = tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
            order: 1,
            status: Status::Green,
            name: "stacks-node".into(),
            comment: format!("mining blocks (chaintip = #{})", new_block.block_height),
        }));
        let _ = tx.send(DevnetEvent::info(format!(
            "Block #{} anchored in Bitcoin block #{} includes {} transactions",
            new_block.block_height,
            new_block.burn_block_height,
            new_block.transactions.len(),
        )));
    }

    let (
        updated_config,
        first_burnchain_block_height,
        prepare_phase_block_length,
        reward_phase_block_length,
        node,
    ) = if let Ok(config_reader) = config.read() {
        let node = format!(
            "http://localhost:{}",
            config_reader.devnet_config.stacks_node_rpc_port
        );

        if config_reader.contracts_to_deploy.len() > 0 {
            let mut updated_config = config_reader.clone();

            // How many contracts left?
            let contracts_left = updated_config.contracts_to_deploy.len();
            let tx_chaining_limit = 25;
            let blocks_required = 1 + (contracts_left / tx_chaining_limit);
            let contracts_to_deploy_in_blocks = if blocks_required == 1 {
                contracts_left
            } else {
                contracts_left / blocks_required
            };

            let mut contracts_to_deploy = vec![];

            for _ in 0..contracts_to_deploy_in_blocks {
                let contract = updated_config.contracts_to_deploy.pop_front().unwrap();
                contracts_to_deploy.push(contract);
            }

            let node_clone = node.clone();

            let mut deployers_lookup = BTreeMap::new();
            for account in updated_config.session_settings.initial_accounts.iter() {
                if account.name == "deployer" {
                    deployers_lookup.insert("*".into(), account.clone());
                }
            }
            // TODO(ludo): one day, we will get rid of this shortcut
            let mut deployers_nonces = BTreeMap::new();
            deployers_nonces.insert("deployer".to_string(), config_reader.deployer_nonce);
            updated_config.deployer_nonce += contracts_to_deploy.len() as u64;

            if let Ok(tx) = devnet_events_tx.lock() {
                let _ = tx.send(DevnetEvent::success(format!(
                    "Will broadcast {} transactions",
                    contracts_to_deploy.len()
                )));
            }

            // Move the transactions submission to another thread, the clock on that thread is ticking,
            // and blocking our stacks-node
            std::thread::spawn(move || {
                for contract in contracts_to_deploy.into_iter() {
                    match publish_contract(
                        &contract,
                        &deployers_lookup,
                        &mut deployers_nonces,
                        &node_clone,
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
            (
                Some(updated_config),
                config_reader.pox_info.first_burnchain_block_height,
                config_reader.pox_info.prepare_phase_block_length,
                config_reader.pox_info.reward_phase_block_length,
                node,
            )
        } else {
            (
                None,
                config_reader.pox_info.first_burnchain_block_height,
                config_reader.pox_info.prepare_phase_block_length,
                config_reader.pox_info.reward_phase_block_length,
                node,
            )
        }
    } else {
        (None, 0, 0, 0, "".into())
    };

    if let Some(updated_config) = updated_config {
        if let Ok(mut config_writer) = config.write() {
            *config_writer = updated_config;
        }
    }

    let pox_cycle_length: u64 = (prepare_phase_block_length + reward_phase_block_length).into();
    let current_len = new_block.burn_block_height - first_burnchain_block_height;
    let pox_cycle_id: u32 = (current_len / pox_cycle_length).try_into().unwrap();
    let transactions = new_block
        .transactions
        .iter()
        .map(|t| {
            let description = get_tx_description(&t.raw_tx);
            Transaction {
                txid: t.txid.clone(),
                success: t.status == "success",
                result: get_value_description(&t.raw_result),
                events: vec![],
                description,
            }
        })
        .collect();

    if let Ok(tx) = devnet_events_tx.lock() {
        let _ = tx.send(DevnetEvent::Block(BlockData {
            block_height: new_block.block_height,
            block_hash: new_block.block_hash.clone(),
            bitcoin_block_height: new_block.burn_block_height,
            bitcoin_block_hash: new_block.burn_block_hash.clone(),
            first_burnchain_block_height: first_burnchain_block_height,
            pox_cycle_length: pox_cycle_length.try_into().unwrap(),
            pox_cycle_id,
            transactions,
        }));
    }

    // Every penultimate block, we check if some stacking orders should be submitted before the next
    // cycle starts.
    if new_block.burn_block_height % pox_cycle_length == (pox_cycle_length - 2) {
        if let Ok(config_reader) = config.read() {
            // let tx_clone = tx.clone();

            let accounts = config_reader.accounts.clone();
            let mut pox_info = config_reader.pox_info.clone();

            let pox_stacking_orders = config_reader.devnet_config.pox_stacking_orders.clone();
            std::thread::spawn(move || {
                let pox_url = format!("{}/v2/pox", node);

                if let Ok(reponse) = reqwest::blocking::get(pox_url) {
                    if let Ok(update) = reponse.json() {
                        pox_info = update
                    }
                }

                for pox_stacking_order in pox_stacking_orders.into_iter() {
                    if pox_stacking_order.start_at_cycle == (pox_cycle_id + 1) {
                        let account = match accounts.get(&pox_stacking_order.wallet) {
                            None => continue,
                            Some(account) => account,
                        };
                        let stacks_rpc = StacksRpc::new(node.clone());
                        let default_fee = 1000;
                        let nonce = stacks_rpc
                            .get_nonce(account.address.to_string())
                            .expect("Unable to retrieve nonce");

                        let stx_amount =
                            pox_info.next_cycle.min_threshold_ustx * pox_stacking_order.slots;
                        let (_, _, account_secret_keu) = types::compute_addresses(
                            &account.mnemonic,
                            &account.derivation,
                            account.is_mainnet,
                        );
                        let addr_bytes = pox_stacking_order
                            .btc_address
                            .from_base58()
                            .expect("Unable to get bytes from btc address");

                        let addr_bytes = Hash160::from_bytes(&addr_bytes[1..21]).unwrap();
                        let addr_version = AddressHashMode::SerializeP2PKH;
                        let stack_stx_tx = transactions::build_contrat_call_transaction(
                            pox_info.contract_id.clone(),
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
                                            ClarityValue::Sequence(SequenceData::Buffer(
                                                BuffData {
                                                    data: addr_bytes.as_bytes().to_vec(),
                                                },
                                            )),
                                        ),
                                    ])
                                    .unwrap(),
                                ),
                                ClarityValue::UInt((new_block.burn_block_height - 1).into()),
                                ClarityValue::UInt(pox_stacking_order.duration.into()),
                            ],
                            nonce,
                            default_fee,
                            &hex_bytes(&account_secret_keu).unwrap(),
                        );
                        let _ = stacks_rpc
                            .post_transaction(stack_stx_tx)
                            .expect("Unable to broadcast transaction");
                    }
                }
            });
        }
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
    _config: &State<Arc<RwLock<EventObserverConfig>>>,
    devnet_events_tx: &State<Arc<Mutex<Sender<DevnetEvent>>>>,
    new_microblock: Json<NewMicroBlock>,
    _node_event_tx: &State<Arc<Mutex<Option<Sender<NodeObserverEvent>>>>>,
) -> Json<Value> {
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
    //         Transaction {
    //             txid: t.txid.clone(),
    //             success: t.status == "success",
    //             result: get_value_description(&t.raw_result),
    //             events: vec![],
    //             description,
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
    _node_event_tx: &State<Arc<Mutex<Option<Sender<NodeObserverEvent>>>>>,
) -> Json<Value> {
    let decoded_transactions = raw_txs
        .iter()
        .map(|t| get_tx_description(t))
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
pub fn handle_drop_mempool_tx() -> Json<Value> {
    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}

fn get_value_description(raw_value: &str) -> String {
    let raw_value = match raw_value.strip_prefix("0x") {
        Some(raw_value) => raw_value,
        _ => return raw_value.to_string(),
    };
    let value_bytes = match hex_bytes(&raw_value) {
        Ok(bytes) => bytes,
        _ => return raw_value.to_string(),
    };

    let value = match ClarityValue::consensus_deserialize(&mut Cursor::new(&value_bytes)) {
        Ok(value) => format!("{}", value),
        Err(e) => {
            println!("{:?}", e);
            return raw_value.to_string();
        }
    };
    value
}

pub fn get_tx_description(raw_tx: &str) -> String {
    let raw_tx = match raw_tx.strip_prefix("0x") {
        Some(raw_tx) => raw_tx,
        _ => return raw_tx.to_string(),
    };
    let tx_bytes = match hex_bytes(&raw_tx) {
        Ok(bytes) => bytes,
        _ => return raw_tx.to_string(),
    };
    let tx = match StacksTransaction::consensus_deserialize(&mut Cursor::new(&tx_bytes)) {
        Ok(bytes) => bytes,
        Err(e) => {
            println!("{:?}", e);
            return raw_tx.to_string();
        }
    };
    let description = match tx.payload {
        TransactionPayload::TokenTransfer(ref addr, ref amount, ref _memo) => {
            format!(
                "transfered: {} µSTX from {} to {}",
                amount,
                tx.origin_address(),
                addr
            )
        }
        TransactionPayload::ContractCall(ref contract_call) => {
            let formatted_args = contract_call
                .function_args
                .iter()
                .map(|v| format!("{}", v))
                .collect::<Vec<String>>()
                .join(", ");
            format!(
                "invoked: {}.{}::{}({})",
                contract_call.address,
                contract_call.contract_name,
                contract_call.function_name,
                formatted_args
            )
        }
        TransactionPayload::SmartContract(ref smart_contract) => {
            format!("deployed: {}.{}", tx.origin_address(), smart_contract.name)
        }
        _ => {
            format!("coinbase")
        }
    };
    description
}
