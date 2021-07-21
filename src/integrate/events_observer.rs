
use super::DevnetEvent;
use crate::integrate::{BlockData, MempoolAdmissionData, ServiceStatusData, Status, Transaction};
use crate::poke::load_session;
use crate::publish::{publish_contract, Network};
use crate::types::{self, AccountConfig, DevnetConfig};
use crate::utils::stacks::{transactions, StacksRpc};
use clarity_repl::clarity::representations::ClarityName;
use clarity_repl::clarity::types::{BuffData, SequenceData, TupleData, Value as ClarityValue};
use clarity_repl::clarity::util::address::AddressHashMode;
use clarity_repl::clarity::util::hash::{hex_bytes, Hash160};
use clarity_repl::repl::SessionSettings;
use clarity_repl::repl::settings::InitialContract;
use rocket::config::{Config, Environment, LoggingLevel};
use rocket::State;
use rocket_contrib::json::Json;
use serde_json::Value;
use std::collections::{BTreeMap, VecDeque};
use std::convert::{TryFrom, TryInto};
use std::error::Error;
use std::path::PathBuf;
use std::str;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::iter::FromIterator;
use base58::FromBase58;

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
pub struct NewTransaction {
    txid: String,
    status: String,
    raw_result: String,
    // tx_index: u32,
}

#[derive(Clone, Debug)]
pub struct EventObserverConfig {
    pub devnet_config: DevnetConfig,
    pub accounts: BTreeMap<String, AccountConfig>,
    pub contracts_to_deploy: VecDeque<InitialContract>,
    pub manifest_path: PathBuf,
    pub pox_info: PoxInfo,
    pub session_settings: SessionSettings,
}

impl EventObserverConfig {
    pub fn new(devnet_config: DevnetConfig, manifest_path: PathBuf, accounts: BTreeMap<String, AccountConfig>) -> Self {
        println!("Checking contracts...");
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
            contracts_to_deploy: VecDeque::from_iter(session_settings.initial_contracts.iter().map(|c| c.clone())),
            session_settings,
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

    pub fn pox_cycle_len(&self) -> u32 {
        self.reward_phase_block_length + self.prepare_phase_block_length
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
) -> Result<(), Box<dyn Error>> {
    let port = events_config.devnet_config.orchestrator_port;

    let config = Config::build(Environment::Production)
        .address("127.0.0.1")
        .port(port)
        .log_level(LoggingLevel::Off)
        .finalize()?;

    rocket::custom(config)
        .manage(RwLock::new(events_config))
        .manage(Arc::new(Mutex::new(devnet_event_tx.clone())))
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

    match terminator_rx.recv() {
        Ok(true) => {
            devnet_event_tx
                .send(DevnetEvent::info("Terminating event observer".into()))
                .expect("Unable to terminate event observer");
        }
        _ => {}
    }
    Ok(())
}

#[post(
    "/new_burn_block",
    format = "application/json",
    data = "<new_burn_block>"
)]
pub fn handle_new_burn_block(
    config_state: State<RwLock<EventObserverConfig>>,
    devnet_events_tx: State<Arc<Mutex<Sender<DevnetEvent>>>>,
    new_burn_block: Json<NewBurnBlock>,
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

    let config = config_state.inner();

    let (pox_cycle_len, pox_url) = match config.read() {
        Ok(config_reader) => {
            let pox_cycle_len: u64 = config_reader.pox_info.pox_cycle_len().into();
            let pox_url = format!(
                "http://localhost:{}/v2/pox",
                config_reader.devnet_config.stacks_node_rpc_port
            );
            (pox_cycle_len, pox_url)
        }
        Err(_) => {
            return Json(json!({
                "status": 200,
                "result": "Ok",
            }))        
        }
    };

    if new_burn_block.burn_block_height % pox_cycle_len == 1 {
        if let Ok(reponse) = reqwest::blocking::get(pox_url) {
            if let Ok(pox_info) = reponse.json() {
                if let Ok(mut config_writer) = config.write() {
                    config_writer.pox_info = pox_info;
                }
            }
        }
    }

    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}

#[post("/new_block", format = "application/json", data = "<new_block>")]
pub fn handle_new_block(
    config: State<RwLock<EventObserverConfig>>,
    devnet_events_tx: State<Arc<Mutex<Sender<DevnetEvent>>>>,
    new_block: Json<NewBlock>,
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

        let updated_config = if let Ok(config_reader) = config.read() {
            let mut updated_config = config_reader.clone();
            let node = format!(
                "http://localhost:{}",
                config_reader.devnet_config.stacks_node_rpc_port
            );

            if updated_config.contracts_to_deploy.len() > 0 {

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

                let mut deployers_lookup = BTreeMap::new();
                for account in updated_config.session_settings.initial_accounts.iter() {
                    if account.name == "deployer" {
                        deployers_lookup.insert("*".into(), account.clone());
                    }
                }

                let tx_clone = tx.clone();
                let node_clone = node.clone();

                // Move the transactions submission to another thread, the clock on that thread is ticking,
                // and blocking our stacks-node
                std::thread::spawn(move || {
                    let mut deployers_nonces = BTreeMap::new();
    
                    for contract in contracts_to_deploy.into_iter() {
                        match publish_contract(&contract, &deployers_lookup, &mut deployers_nonces, &node_clone) {
                            Ok((txid, nonce)) => {
                                let _ = tx_clone.send(DevnetEvent::success(format!(
                                    "Contract {} broadcasted in mempool (txid: {}, nonce: {})",
                                    contract.name.unwrap(), txid, nonce
                                )));
                            }
                            Err(err) => {
                                let _ = tx_clone.send(DevnetEvent::error(err.to_string()));
                                break;
                            }
                        }    
                    }
                });
            }

            Some(updated_config)
        } else {
            None
        };

        if let Some(updated_config) = updated_config {
            if let Ok(mut config_writer) = config.write() {
                *config_writer = updated_config;
            }
        }

        if let Ok(config_reader) = config.read() {
            let pox_cycle_length: u64 = (config_reader.pox_info.prepare_phase_block_length
                + config_reader.pox_info.reward_phase_block_length).into();
            let current_len = new_block.burn_block_height - config_reader.pox_info.first_burnchain_block_height;
            let pox_cycle_id: u32 = (current_len / pox_cycle_length).try_into().unwrap();
            let _ = tx.send(DevnetEvent::Block(BlockData {
                block_height: new_block.block_height,
                block_hash: new_block.block_hash.clone(),
                bitcoin_block_height: new_block.burn_block_height,
                bitcoin_block_hash: new_block.burn_block_hash.clone(),
                first_burnchain_block_height: config_reader.pox_info.first_burnchain_block_height,
                pox_cycle_length: pox_cycle_length.try_into().unwrap(),
                pox_cycle_id,
                transactions: new_block
                    .transactions
                    .iter()
                    .map(|t| Transaction {
                        txid: t.txid.clone(),
                        success: t.status == "success",
                        result: t.raw_result.clone(),
                        events: vec![],
                    })
                    .collect(),
            }));

            // Every penultimate block, we check if some stacking orders should be submitted before the next
            // cycle starts.
            if new_block.burn_block_height % pox_cycle_length == (pox_cycle_length - 2) {

                // let tx_clone = tx.clone();
                let node = format!(
                    "http://localhost:{}",
                    config_reader.devnet_config.stacks_node_rpc_port
                );
                let accounts = config_reader.accounts.clone();
                let pox_info = config_reader.pox_info.clone();

                let pox_stacking_orders = config_reader.devnet_config.pox_stacking_orders.clone();
                std::thread::spawn(move || {
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
    
                            let stx_amount = pox_info.next_cycle.min_threshold_ustx
                                * pox_stacking_order.slots;
                            let (_, _, account_secret_keu) = types::compute_addresses(
                                &account.mnemonic,
                                &account.derivation,
                                account.is_mainnet,
                            );
                            let addr_bytes = pox_stacking_order.btc_address.from_base58()
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
    };

    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}

#[post("/new_microblocks", format = "application/json")]
pub fn handle_new_microblocks(
    _devnet_events_tx: State<Arc<Mutex<Sender<DevnetEvent>>>>,
) -> Json<Value> {
    Json(json!({
        "status": 200,
        "result": "Ok",
    }))
}

#[post("/new_mempool_tx", format = "application/json", data = "<raw_txs>")]
pub fn handle_new_mempool_tx(
    _config: State<RwLock<EventObserverConfig>>,
    devnet_events_tx: State<Arc<Mutex<Sender<DevnetEvent>>>>,
    raw_txs: Json<Vec<String>>,
) -> Json<Value> {
    if let Ok(tx) = devnet_events_tx.lock() {
        for raw_tx in raw_txs.iter() {
            let _ = tx.send(DevnetEvent::MempoolAdmission(MempoolAdmissionData {
                txid: raw_tx.to_string(),
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

/*
export interface CoreNodeTxMessage {
    raw_tx: string;
    result: NonStandardClarityValue;
    status: CoreNodeTxStatus;
    raw_result: string;
    txid: string;
    tx_index: number;
    contract_abi: ClarityAbi | null;
  }

  export interface CoreNodeBlockMessage {
    block_hash: string;
    block_height: number;
    burn_block_time: number;
    burn_block_hash: string;
    burn_block_height: number;
    miner_txid: string;
    index_block_hash: string;
    parent_index_block_hash: string;
    parent_block_hash: string;
    parent_microblock: string;
    events: CoreNodeEvent[];
    transactions: CoreNodeTxMessage[];
    matured_miner_rewards: {
      from_index_consensus_hash: string;
      from_stacks_block_hash: string;
      /** STX principal */
      recipient: string;
      /** String quoted micro-STX amount. */
      coinbase_amount: string;
      /** String quoted micro-STX amount. */
      tx_fees_anchored: string;
      /** String quoted micro-STX amount. */
      tx_fees_streamed_confirmed: string;
      /** String quoted micro-STX amount. */
      tx_fees_streamed_produced: string;
    }[];
  }

  export interface CoreNodeMessageParsed extends CoreNodeBlockMessage {
    parsed_transactions: CoreNodeParsedTxMessage[];
  }

  export interface CoreNodeParsedTxMessage {
    core_tx: CoreNodeTxMessage;
    parsed_tx: Transaction;
    raw_tx: Buffer;
    nonce: number;
    sender_address: string;
    sponsor_address?: string;
    block_hash: string;
    index_block_hash: string;
    block_height: number;
    burn_block_time: number;
  }

  export interface CoreNodeBurnBlockMessage {
    burn_block_hash: string;
    burn_block_height: number;
    /** Amount in BTC satoshis. */
    burn_amount: number;
    reward_recipients: [
      {
        /** Bitcoin address (b58 encoded). */
        recipient: string;
        /** Amount in BTC satoshis. */
        amt: number;
      }
    ];
    /**
     * Array of the Bitcoin addresses that would validly receive PoX commitments during this block.
     * These addresses may not actually receive rewards during this block if the block is faster
     * than miners have an opportunity to commit.
     */
    reward_slot_holders: string[];
  }

  export type CoreNodeDropMempoolTxReasonType =
    | 'ReplaceByFee'
    | 'ReplaceAcrossFork'
    | 'TooExpensive'
    | 'StaleGarbageCollect';

  export interface CoreNodeDropMempoolTxMessage {
    dropped_txids: string[];
    reason: CoreNodeDropMempoolTxReasonType;
  }

  export interface CoreNodeAttachmentMessage {
    attachment_index: number;
    index_block_hash: string;
    block_height: string; // string quoted integer?
    content_hash: string;
    contract_id: string;
    /** Hex serialized Clarity value */
    metadata: string;
    tx_id: string;
    /* Hex encoded attachment content bytes */
    content: string;
  }
  */

// let join_handle = std::thread::spawn(move || {
//     let mut i = 0;
//     loop {
//         std::thread::sleep(std::time::Duration::from_secs(1));
//         event_tx_simulator.send(DevnetEvent::Log(LogData {
//             level: LogLevel::Info,
//             message: "Hello world".into(),
//             occurred_at: 0
//         })).unwrap();
//         event_tx_simulator.send(DevnetEvent::Block(BlockData {
//             block_height: i,
//             bitcoin_block_height: i,
//             block_hash: format!("{}", i),
//             bitcoin_block_hash: format!("{}", i),
//             transactions: vec![
//                 Transaction {
//                     txid: "".to_string(),
//                     success: i % 2 == 0,
//                     result: format!("(ok u1)"),
//                     events: vec![],
//                 },
//                 Transaction {
//                     txid: "".to_string(),
//                     success: (i + 1) % 2 == 0,
//                     result: format!("(err u3)"),
//                     events: vec![],
//                 },
//                 Transaction {
//                     txid: "".to_string(),
//                     success: (i + 2) % 2 == 0,
//                     result: format!("(ok err)"),
//                     events: vec![],
//                 },
//             ]
//         })).unwrap();
//         i += 1;
//     }
// });
