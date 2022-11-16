mod blocks_pool;

pub use blocks_pool::StacksBlockPool;

use crate::indexer::AssetClassCache;
use crate::indexer::{IndexerConfig, StacksChainContext};
use chainhook_types::*;
use clarity_repl::clarity::codec::StacksMessageCodec;
use clarity_repl::clarity::util::hash::hex_bytes;
use clarity_repl::clarity::vm::types::Value as ClarityValue;
use clarity_repl::codec::{StacksTransaction, TransactionAuth, TransactionPayload};
use rocket::serde::json::Value as JsonValue;
use rocket::serde::Deserialize;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::convert::TryInto;
use std::io::Cursor;
use std::str;

#[derive(Deserialize)]
pub struct NewBlock {
    pub block_height: u64,
    pub block_hash: String,
    pub index_block_hash: String,
    pub burn_block_height: u64,
    pub burn_block_hash: String,
    pub parent_block_hash: String,
    pub parent_index_block_hash: String,
    pub parent_microblock: String,
    pub parent_microblock_sequence: u64,
    pub parent_burn_block_hash: String,
    pub parent_burn_block_height: u64,
    pub parent_burn_block_timestamp: u64,
    pub transactions: Vec<NewTransaction>,
    pub events: Vec<NewEvent>,
    pub matured_miner_rewards: Vec<MaturedMinerReward>,
}

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct NewBlockHeader {
    pub block_height: u64,
    pub index_block_hash: Option<String>,
    pub parent_index_block_hash: Option<String>,
}

#[derive(Deserialize)]
pub struct MaturedMinerReward {
    pub from_index_consensus_hash: String,
    pub from_stacks_block_hash: String,
    pub recipient: String,
    pub coinbase_amount: String,
    /// micro-STX amount
    pub tx_fees_anchored: String,
    /// micro-STX amount
    pub tx_fees_streamed_confirmed: String,
    /// micro-STX amount
    pub tx_fees_streamed_produced: String,
}

#[derive(Deserialize, Debug)]
pub struct NewMicroblockTrail {
    pub parent_index_block_hash: String,
    pub burn_block_hash: String,
    pub burn_block_height: u64,
    pub burn_block_timestamp: u64,
    pub transactions: Vec<NewMicroblockTransaction>,
    pub events: Vec<NewEvent>,
}

#[derive(Deserialize, Debug)]
pub struct NewTransaction {
    pub txid: String,
    pub tx_index: usize,
    pub status: String,
    pub raw_result: String,
    pub raw_tx: String,
    pub execution_cost: Option<StacksTransactionExecutionCost>,
}

#[derive(Deserialize, Debug)]
pub struct NewMicroblockTransaction {
    pub txid: String,
    pub tx_index: usize,
    pub status: String,
    pub raw_result: String,
    pub raw_tx: String,
    pub execution_cost: Option<StacksTransactionExecutionCost>,
    pub microblock_sequence: usize,
    pub microblock_hash: String,
    pub microblock_parent_hash: String,
}

#[derive(Debug, Deserialize)]
pub struct NewEvent {
    pub txid: String,
    pub committed: bool,
    pub event_index: u32,
    #[serde(rename = "type")]
    pub event_type: String,
    pub stx_transfer_event: Option<JsonValue>,
    pub stx_mint_event: Option<JsonValue>,
    pub stx_burn_event: Option<JsonValue>,
    pub stx_lock_event: Option<JsonValue>,
    pub nft_transfer_event: Option<JsonValue>,
    pub nft_mint_event: Option<JsonValue>,
    pub nft_burn_event: Option<JsonValue>,
    pub ft_transfer_event: Option<JsonValue>,
    pub ft_mint_event: Option<JsonValue>,
    pub ft_burn_event: Option<JsonValue>,
    pub data_var_set_event: Option<JsonValue>,
    pub data_map_insert_event: Option<JsonValue>,
    pub data_map_update_event: Option<JsonValue>,
    pub data_map_delete_event: Option<JsonValue>,
    pub contract_event: Option<JsonValue>,
}

impl NewEvent {
    pub fn into_chainhook_event(&self) -> Result<StacksTransactionEvent, String> {
        if let Some(ref event_data) = self.stx_mint_event {
            let data: STXMintEventData =
                serde_json::from_value(event_data.clone()).expect("Unable to decode event_data");
            return Ok(StacksTransactionEvent::STXMintEvent(data.clone()));
        } else if let Some(ref event_data) = self.stx_lock_event {
            let data: STXLockEventData =
                serde_json::from_value(event_data.clone()).expect("Unable to decode event_data");
            return Ok(StacksTransactionEvent::STXLockEvent(data.clone()));
        } else if let Some(ref event_data) = self.stx_burn_event {
            let data: STXBurnEventData =
                serde_json::from_value(event_data.clone()).expect("Unable to decode event_data");
            return Ok(StacksTransactionEvent::STXBurnEvent(data.clone()));
        } else if let Some(ref event_data) = self.stx_transfer_event {
            let data: STXTransferEventData =
                serde_json::from_value(event_data.clone()).expect("Unable to decode event_data");
            return Ok(StacksTransactionEvent::STXTransferEvent(data.clone()));
        } else if let Some(ref event_data) = self.nft_mint_event {
            let data: NFTMintEventData =
                serde_json::from_value(event_data.clone()).expect("Unable to decode event_data");
            return Ok(StacksTransactionEvent::NFTMintEvent(data.clone()));
        } else if let Some(ref event_data) = self.nft_burn_event {
            let data: NFTBurnEventData =
                serde_json::from_value(event_data.clone()).expect("Unable to decode event_data");
            return Ok(StacksTransactionEvent::NFTBurnEvent(data.clone()));
        } else if let Some(ref event_data) = self.nft_transfer_event {
            let data: NFTTransferEventData =
                serde_json::from_value(event_data.clone()).expect("Unable to decode event_data");
            return Ok(StacksTransactionEvent::NFTTransferEvent(data.clone()));
        } else if let Some(ref event_data) = self.ft_mint_event {
            let data: FTMintEventData =
                serde_json::from_value(event_data.clone()).expect("Unable to decode event_data");
            return Ok(StacksTransactionEvent::FTMintEvent(data.clone()));
        } else if let Some(ref event_data) = self.ft_burn_event {
            let data: FTBurnEventData =
                serde_json::from_value(event_data.clone()).expect("Unable to decode event_data");
            return Ok(StacksTransactionEvent::FTBurnEvent(data.clone()));
        } else if let Some(ref event_data) = self.ft_transfer_event {
            let data: FTTransferEventData =
                serde_json::from_value(event_data.clone()).expect("Unable to decode event_data");
            return Ok(StacksTransactionEvent::FTTransferEvent(data.clone()));
        } else if let Some(ref event_data) = self.data_var_set_event {
            let data: DataVarSetEventData =
                serde_json::from_value(event_data.clone()).expect("Unable to decode event_data");
            return Ok(StacksTransactionEvent::DataVarSetEvent(data.clone()));
        } else if let Some(ref event_data) = self.data_map_insert_event {
            let data: DataMapInsertEventData =
                serde_json::from_value(event_data.clone()).expect("Unable to decode event_data");
            return Ok(StacksTransactionEvent::DataMapInsertEvent(data.clone()));
        } else if let Some(ref event_data) = self.data_map_update_event {
            let data: DataMapUpdateEventData =
                serde_json::from_value(event_data.clone()).expect("Unable to decode event_data");
            return Ok(StacksTransactionEvent::DataMapUpdateEvent(data.clone()));
        } else if let Some(ref event_data) = self.data_map_delete_event {
            let data: DataMapDeleteEventData =
                serde_json::from_value(event_data.clone()).expect("Unable to decode event_data");
            return Ok(StacksTransactionEvent::DataMapDeleteEvent(data.clone()));
        } else if let Some(ref event_data) = self.contract_event {
            let data: SmartContractEventData =
                serde_json::from_value(event_data.clone()).expect("Unable to decode event_data");
            return Ok(StacksTransactionEvent::SmartContractEvent(data.clone()));
        }
        return Err(format!("unable to support event type"));
    }
}

pub fn get_stacks_currency() -> Currency {
    Currency {
        symbol: "STX".into(),
        decimals: 6,
        metadata: None,
    }
}

#[derive(Deserialize, Debug)]
pub struct ContractReadonlyCall {
    pub okay: bool,
    pub result: String,
}

pub fn standardize_stacks_serialized_block_header(
    serialized_block: &str,
) -> Result<(BlockIdentifier, BlockIdentifier), String> {
    let mut block_header: NewBlockHeader = serde_json::from_str(serialized_block)
        .map_err(|e| format!("unable to parse stacks block_header {}", e.to_string()))?;
    let hash = block_header
        .index_block_hash
        .take()
        .ok_or(format!("unable to retrieve index_block_hash"))?;
    let block_identifier = BlockIdentifier {
        hash,
        index: block_header.block_height,
    };
    let parent_hash = block_header
        .parent_index_block_hash
        .take()
        .ok_or(format!("unable to retrieve parent_index_block_hash"))?;
    let parent_block_identifier = BlockIdentifier {
        hash: parent_hash,
        index: block_identifier.index - 1,
    };
    Ok((block_identifier, parent_block_identifier))
}

pub fn standardize_stacks_serialized_block(
    indexer_config: &IndexerConfig,
    serialized_block: &str,
    ctx: &mut StacksChainContext,
) -> Result<StacksBlockData, String> {
    let mut block: NewBlock = serde_json::from_str(serialized_block)
        .map_err(|e| format!("unable to parse stacks block_header {}", e.to_string()))?;
    standardize_stacks_block(indexer_config, &mut block, ctx)
}

pub fn standardize_stacks_marshalled_block(
    indexer_config: &IndexerConfig,
    marshalled_block: JsonValue,
    ctx: &mut StacksChainContext,
) -> Result<StacksBlockData, String> {
    let mut block: NewBlock = serde_json::from_value(marshalled_block)
        .map_err(|e| format!("unable to parse stacks block {}", e.to_string()))?;
    standardize_stacks_block(indexer_config, &mut block, ctx)
}

pub fn standardize_stacks_block(
    indexer_config: &IndexerConfig,
    block: &mut NewBlock,
    ctx: &mut StacksChainContext,
) -> Result<StacksBlockData, String> {
    let pox_cycle_length: u64 =
        (ctx.pox_info.prepare_phase_block_length + ctx.pox_info.reward_phase_block_length).into();
    let current_len = block.burn_block_height - ctx.pox_info.first_burnchain_block_height;
    let mut pox_cycle_id: u32 = (current_len / pox_cycle_length).try_into().unwrap();
    pox_cycle_id += 1; // Pox cycles are 1-indexed
    let mut events: HashMap<&String, Vec<&NewEvent>> = HashMap::new();
    for event in block.events.iter() {
        events
            .entry(&event.txid)
            .and_modify(|events| events.push(&event))
            .or_insert(vec![&event]);
    }

    let mut transactions = vec![];
    for tx in block.transactions.iter() {
        let tx_events = events.remove(&tx.txid).unwrap_or(vec![]);
        let (description, tx_type, fee, sender, sponsor) =
            match get_tx_description(&tx.raw_tx, &tx_events) {
                Ok(desc) => desc,
                Err(e) => {
                    return Err(format!("unable to standardize block ({})", e.to_string()));
                }
            };
        let events = tx_events
            .iter()
            .map(|e| e.into_chainhook_event())
            .collect::<Result<Vec<StacksTransactionEvent>, String>>()?;
        let (receipt, operations) = get_standardized_stacks_receipt(
            &tx.txid,
            events,
            &mut ctx.asset_class_map,
            &indexer_config.stacks_node_rpc_url,
            true,
        );

        transactions.push(StacksTransactionData {
            transaction_identifier: TransactionIdentifier {
                hash: tx.txid.clone(),
            },
            operations,
            metadata: StacksTransactionMetadata {
                success: tx.status == "success",
                result: get_value_description(&tx.raw_result),
                raw_tx: tx.raw_tx.clone(),
                sender,
                fee,
                sponsor,
                kind: tx_type,
                execution_cost: tx.execution_cost.clone(),
                receipt,
                description,
                position: StacksTransactionPosition::Index(tx.tx_index),
                proof: None,
            },
        });
    }

    let confirm_microblock_identifier = if block.parent_microblock
        == "0x0000000000000000000000000000000000000000000000000000000000000000"
    {
        None
    } else {
        Some(BlockIdentifier {
            index: block
                .parent_microblock_sequence
                .try_into()
                .expect("unable to get microblock sequence"),
            hash: block.parent_microblock.clone(),
        })
    };

    let block = StacksBlockData {
        block_identifier: BlockIdentifier {
            hash: block.index_block_hash.clone(),
            index: block.block_height,
        },
        parent_block_identifier: BlockIdentifier {
            hash: block.parent_index_block_hash.clone(),
            index: block.block_height - 1,
        },
        timestamp: 0,
        metadata: StacksBlockMetadata {
            bitcoin_anchor_block_identifier: BlockIdentifier {
                hash: block.burn_block_hash.clone(),
                index: block.burn_block_height,
            },
            pox_cycle_index: pox_cycle_id,
            pox_cycle_position: (current_len % pox_cycle_length) as u32,
            pox_cycle_length: pox_cycle_length.try_into().unwrap(),
            confirm_microblock_identifier,
        },
        transactions,
    };
    Ok(block)
}

pub fn standardize_stacks_serialized_microblock_trail(
    indexer_config: &IndexerConfig,
    serialized_microblock_trail: &str,
    ctx: &mut StacksChainContext,
) -> Result<Vec<StacksMicroblockData>, String> {
    let mut microblock_trail: NewMicroblockTrail =
        serde_json::from_str(serialized_microblock_trail)
            .map_err(|e| format!("unable to parse microblock trail {}", e.to_string()))?;
    standardize_stacks_microblock_trail(indexer_config, &mut microblock_trail, ctx)
}

pub fn standardize_stacks_marshalled_microblock_trail(
    indexer_config: &IndexerConfig,
    marshalled_microblock_trail: JsonValue,
    ctx: &mut StacksChainContext,
) -> Result<Vec<StacksMicroblockData>, String> {
    let mut microblock_trail: NewMicroblockTrail =
        serde_json::from_value(marshalled_microblock_trail)
            .map_err(|e| format!("unable to parse microblock trail {}", e.to_string()))?;
    standardize_stacks_microblock_trail(indexer_config, &mut microblock_trail, ctx)
}

pub fn standardize_stacks_microblock_trail(
    indexer_config: &IndexerConfig,
    microblock_trail: &mut NewMicroblockTrail,
    ctx: &mut StacksChainContext,
) -> Result<Vec<StacksMicroblockData>, String> {
    let mut events: HashMap<&String, Vec<&NewEvent>> = HashMap::new();
    for event in microblock_trail.events.iter() {
        events
            .entry(&event.txid)
            .and_modify(|events| events.push(&event))
            .or_insert(vec![&event]);
    }
    let mut microblocks_set: BTreeMap<
        (BlockIdentifier, BlockIdentifier),
        Vec<StacksTransactionData>,
    > = BTreeMap::new();
    for tx in microblock_trail.transactions.iter() {
        let tx_events = events.remove(&tx.txid).unwrap_or(vec![]);
        let (description, tx_type, fee, sender, sponsor) =
            get_tx_description(&tx.raw_tx, &tx_events).expect("unable to parse transaction");

        let events = tx_events
            .iter()
            .map(|e| e.into_chainhook_event())
            .collect::<Result<Vec<StacksTransactionEvent>, String>>()?;
        let (receipt, operations) = get_standardized_stacks_receipt(
            &tx.txid,
            events,
            &mut ctx.asset_class_map,
            &indexer_config.stacks_node_rpc_url,
            true,
        );

        let microblock_identifier = BlockIdentifier {
            hash: tx.microblock_hash.clone(),
            index: u64::try_from(tx.microblock_sequence).unwrap(),
        };

        let parent_microblock_identifier = if tx.microblock_sequence > 0 {
            BlockIdentifier {
                hash: tx.microblock_parent_hash.clone(),
                index: microblock_identifier.index.saturating_sub(1),
            }
        } else {
            microblock_identifier.clone()
        };

        let transaction = StacksTransactionData {
            transaction_identifier: TransactionIdentifier {
                hash: tx.txid.clone(),
            },
            operations,
            metadata: StacksTransactionMetadata {
                success: tx.status == "success",
                result: get_value_description(&tx.raw_result),
                raw_tx: tx.raw_tx.clone(),
                sender,
                fee,
                sponsor,
                kind: tx_type,
                execution_cost: tx.execution_cost.clone(),
                receipt,
                description,
                position: StacksTransactionPosition::Microblock(
                    microblock_identifier.clone(),
                    tx.tx_index,
                ),
                proof: None,
            },
        };

        microblocks_set
            .entry((microblock_identifier, parent_microblock_identifier))
            .and_modify(|transactions| transactions.push(transaction.clone()))
            .or_insert(vec![transaction]);
    }

    let mut microblocks = vec![];
    for ((block_identifier, parent_block_identifier), transactions) in microblocks_set.into_iter() {
        microblocks.push(StacksMicroblockData {
            block_identifier,
            parent_block_identifier,
            timestamp: 0,
            transactions,
            metadata: StacksMicroblockMetadata {
                anchor_block_identifier: BlockIdentifier {
                    hash: microblock_trail.parent_index_block_hash.clone(),
                    index: 0,
                },
            },
        })
    }
    microblocks.sort_by(|a, b| a.block_identifier.cmp(&b.block_identifier));

    Ok(microblocks)
}

pub fn get_value_description(raw_value: &str) -> String {
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
            error!("unable to deserialize clarity value {:?}", e);
            return raw_value.to_string();
        }
    };
    value
}

pub fn get_tx_description(
    raw_tx: &str,
    tx_events: &Vec<&NewEvent>,
) -> Result<
    (
        String, // Human readable transaction's description (contract-call, publish, ...)
        StacksTransactionKind, //
        u64,    // Transaction fee
        String, // Sender's address
        Option<String>, // Sponsor's address (optional)
    ),
    String,
> {
    let raw_tx = match raw_tx.strip_prefix("0x") {
        Some(raw_tx) => raw_tx,
        _ => return Err("unable to read txid".into()),
    };
    let tx_bytes = match hex_bytes(&raw_tx) {
        Ok(bytes) => bytes,
        Err(e) => return Err(format!("unable to read txid {}", e.to_string())),
    };

    // Handle Stacks transitions operated through Bitcoin transactions
    if tx_bytes.eq(&[0]) {
        let event = match tx_events.first() {
            Some(event) => event,
            None => {
                return Err(format!(
                    "received block with transaction '0x00' and no events"
                ));
            }
        };
        if let Some(ref event_data) = event.stx_transfer_event {
            let data: STXTransferEventData = serde_json::from_value(event_data.clone())
                .map_err(|e| format!("unable to decode event_data {}", e.to_string()))?;
            let description = format!(
                "transfered: {} µSTX from {} to {} through Bitcoin transaction",
                data.amount, data.sender, data.recipient
            );
            let tx_type = StacksTransactionKind::NativeTokenTransfer;
            return Ok((description, tx_type, 0, data.sender, None));
        } else if let Some(ref event_data) = event.stx_lock_event {
            let data: STXLockEventData = serde_json::from_value(event_data.clone())
                .map_err(|e| format!("unable to decode event_data {}", e.to_string()))?;
            let description = format!(
                "stacked: {} µSTX by {} through Bitcoin transaction",
                data.locked_amount, data.locked_address,
            );
            let tx_type = StacksTransactionKind::Other;
            return Ok((description, tx_type, 0, data.locked_address, None));
        }
        return Err(format!("unable to parse transaction {raw_tx}"));
    }

    let tx = StacksTransaction::consensus_deserialize(&mut Cursor::new(&tx_bytes))
        .map_err(|e| format!("unable to consensus decode transaction {}", e.to_string()))?;

    let (fee, sender, sponsor) = match tx.auth {
        TransactionAuth::Standard(ref conditions) => (
            conditions.tx_fee(),
            if tx.is_mainnet() {
                conditions.address_mainnet().to_string()
            } else {
                conditions.address_testnet().to_string()
            },
            None,
        ),
        TransactionAuth::Sponsored(ref sender_conditions, ref sponsor_conditions) => (
            sponsor_conditions.tx_fee(),
            if tx.is_mainnet() {
                sender_conditions.address_mainnet().to_string()
            } else {
                sender_conditions.address_testnet().to_string()
            },
            Some(if tx.is_mainnet() {
                sponsor_conditions.address_mainnet().to_string()
            } else {
                sponsor_conditions.address_testnet().to_string()
            }),
        ),
    };

    let (description, tx_type) = match tx.payload {
        TransactionPayload::TokenTransfer(ref addr, ref amount, ref _memo) => (
            format!(
                "transfered: {} µSTX from {} to {}",
                amount,
                tx.origin_address(),
                addr
            ),
            StacksTransactionKind::NativeTokenTransfer,
        ),
        TransactionPayload::ContractCall(ref contract_call) => {
            let formatted_args = contract_call
                .function_args
                .iter()
                .map(|v| format!("{}", v))
                .collect::<Vec<String>>();
            (
                format!(
                    "invoked: {}.{}::{}({})",
                    contract_call.address,
                    contract_call.contract_name,
                    contract_call.function_name,
                    formatted_args.join(", ")
                ),
                StacksTransactionKind::ContractCall(StacksContractCallData {
                    contract_identifier: format!(
                        "{}.{}",
                        contract_call.address, contract_call.contract_name
                    ),
                    method: contract_call.function_name.to_string(),
                    args: formatted_args,
                }),
            )
        }
        TransactionPayload::SmartContract(ref smart_contract, ref _clarity_version) => {
            let contract_identifier = format!("{}.{}", tx.origin_address(), smart_contract.name);
            let data = StacksContractDeploymentData {
                contract_identifier: contract_identifier.clone(),
                code: smart_contract.code_body.to_string(),
            };
            (
                format!("deployed: {}", contract_identifier),
                StacksTransactionKind::ContractDeployment(data),
            )
        }
        TransactionPayload::Coinbase(_, _) => {
            (format!("coinbase"), StacksTransactionKind::Coinbase)
        }
        _ => (format!("other"), StacksTransactionKind::Other),
    };
    Ok((description, tx_type, fee, sender, sponsor))
}

pub fn get_standardized_fungible_currency_from_asset_class_id(
    asset_class_id: &str,
    asset_class_cache: &mut HashMap<String, AssetClassCache>,
    _node_url: &str,
) -> Currency {
    match asset_class_cache.get(asset_class_id) {
        None => {
            // TODO(lgalabru): re-approach this, with an adequate runtime strategy.
            // let comps = asset_class_id.split("::").collect::<Vec<&str>>();
            // let principal = comps[0].split(".").collect::<Vec<&str>>();
            // let contract_address = principal[0];
            // let contract_name = principal[1];
            // let stacks_rpc = StacksRpc::new(&node_url);
            // let value = stacks_rpc
            //     .call_read_only_fn(
            //         &contract_address,
            //         &contract_name,
            //         "get-symbol",
            //         vec![],
            //         contract_address,
            //     )
            //     .expect("Unable to retrieve symbol");
            let symbol = "TOKEN".into(); //value.expect_result_ok().expect_ascii();

            // let value = stacks_rpc
            //     .call_read_only_fn(
            //         &contract_address,
            //         &contract_name,
            //         "get-decimals",
            //         vec![],
            //         &contract_address,
            //     )
            //     .expect("Unable to retrieve decimals");
            let decimals = 6; // value.expect_result_ok().expect_u128() as u8;

            let entry = AssetClassCache { symbol, decimals };

            let currency = Currency {
                symbol: entry.symbol.clone(),
                decimals: entry.decimals.into(),
                metadata: Some(CurrencyMetadata {
                    asset_class_identifier: asset_class_id.into(),
                    asset_identifier: None,
                    standard: CurrencyStandard::Sip10,
                }),
            };

            asset_class_cache.insert(asset_class_id.into(), entry);

            currency
        }
        Some(entry) => Currency {
            symbol: entry.symbol.clone(),
            decimals: entry.decimals.into(),
            metadata: Some(CurrencyMetadata {
                asset_class_identifier: asset_class_id.into(),
                asset_identifier: None,
                standard: CurrencyStandard::Sip10,
            }),
        },
    }
}

pub fn get_standardized_non_fungible_currency_from_asset_class_id(
    asset_class_id: &str,
    asset_id: &str,
    _asset_class_cache: &mut HashMap<String, AssetClassCache>,
) -> Currency {
    Currency {
        symbol: asset_class_id.into(),
        decimals: 0,
        metadata: Some(CurrencyMetadata {
            asset_class_identifier: asset_class_id.into(),
            asset_identifier: Some(asset_id.into()),
            standard: CurrencyStandard::Sip09,
        }),
    }
}

pub fn get_standardized_stacks_receipt(
    _txid: &str,
    events: Vec<StacksTransactionEvent>,
    asset_class_cache: &mut HashMap<String, AssetClassCache>,
    node_url: &str,
    include_operations: bool,
) -> (StacksTransactionReceipt, Vec<Operation>) {
    let mut mutated_contracts_radius = HashSet::new();
    let mut mutated_assets_radius = HashSet::new();
    let mut operations = vec![];

    if include_operations {
        let mut operation_id = 0;
        for event in events.iter() {
            match event {
                StacksTransactionEvent::STXMintEvent(data) => {
                    operations.push(Operation {
                        operation_identifier: OperationIdentifier {
                            index: operation_id,
                            network_index: None,
                        },
                        related_operations: None,
                        type_: OperationType::Credit,
                        status: Some(OperationStatusKind::Success),
                        account: AccountIdentifier {
                            address: data.recipient.clone(),
                            sub_account: None,
                        },
                        amount: Some(Amount {
                            value: data.amount.parse::<u128>().expect("Unable to parse u64"),
                            currency: get_stacks_currency(),
                        }),
                        metadata: None,
                    });
                    operation_id += 1;
                }
                StacksTransactionEvent::STXLockEvent(data) => {
                    operations.push(Operation {
                        operation_identifier: OperationIdentifier {
                            index: operation_id,
                            network_index: None,
                        },
                        related_operations: None,
                        type_: OperationType::Lock,
                        status: Some(OperationStatusKind::Success),
                        account: AccountIdentifier {
                            address: data.locked_address.clone(),
                            sub_account: None,
                        },
                        amount: Some(Amount {
                            value: data
                                .locked_amount
                                .parse::<u128>()
                                .expect("Unable to parse u64"),
                            currency: get_stacks_currency(),
                        }),
                        metadata: None,
                    });
                    operation_id += 1;
                }
                StacksTransactionEvent::STXBurnEvent(data) => {
                    operations.push(Operation {
                        operation_identifier: OperationIdentifier {
                            index: operation_id,
                            network_index: None,
                        },
                        related_operations: None,
                        type_: OperationType::Debit,
                        status: Some(OperationStatusKind::Success),
                        account: AccountIdentifier {
                            address: data.sender.clone(),
                            sub_account: None,
                        },
                        amount: Some(Amount {
                            value: data.amount.parse::<u128>().expect("Unable to parse u64"),
                            currency: get_stacks_currency(),
                        }),
                        metadata: None,
                    });
                    operation_id += 1;
                }
                StacksTransactionEvent::STXTransferEvent(data) => {
                    operations.push(Operation {
                        operation_identifier: OperationIdentifier {
                            index: operation_id,
                            network_index: None,
                        },
                        related_operations: Some(vec![OperationIdentifier {
                            index: operation_id + 1,
                            network_index: None,
                        }]),
                        type_: OperationType::Debit,
                        status: Some(OperationStatusKind::Success),
                        account: AccountIdentifier {
                            address: data.sender.clone(),
                            sub_account: None,
                        },
                        amount: Some(Amount {
                            value: data.amount.parse::<u128>().expect("Unable to parse u64"),
                            currency: get_stacks_currency(),
                        }),
                        metadata: None,
                    });
                    operation_id += 1;
                    operations.push(Operation {
                        operation_identifier: OperationIdentifier {
                            index: operation_id,
                            network_index: None,
                        },
                        related_operations: Some(vec![OperationIdentifier {
                            index: operation_id - 1,
                            network_index: None,
                        }]),
                        type_: OperationType::Credit,
                        status: Some(OperationStatusKind::Success),
                        account: AccountIdentifier {
                            address: data.recipient.clone(),
                            sub_account: None,
                        },
                        amount: Some(Amount {
                            value: data.amount.parse::<u128>().expect("Unable to parse u64"),
                            currency: get_stacks_currency(),
                        }),
                        metadata: None,
                    });
                    operation_id += 1;
                }
                StacksTransactionEvent::NFTMintEvent(data) => {
                    let (asset_class_identifier, contract_identifier) =
                        get_mutated_ids(&data.asset_class_identifier);
                    mutated_assets_radius.insert(asset_class_identifier);
                    mutated_contracts_radius.insert(contract_identifier);

                    let currency = get_standardized_non_fungible_currency_from_asset_class_id(
                        &data.asset_class_identifier,
                        &data.hex_asset_identifier,
                        asset_class_cache,
                    );
                    operations.push(Operation {
                        operation_identifier: OperationIdentifier {
                            index: operation_id,
                            network_index: None,
                        },
                        related_operations: None,
                        type_: OperationType::Credit,
                        status: Some(OperationStatusKind::Success),
                        account: AccountIdentifier {
                            address: data.recipient.clone(),
                            sub_account: None,
                        },
                        amount: Some(Amount { value: 1, currency }),
                        metadata: None,
                    });
                    operation_id += 1;
                }
                StacksTransactionEvent::NFTBurnEvent(data) => {
                    let (asset_class_identifier, contract_identifier) =
                        get_mutated_ids(&data.asset_class_identifier);
                    mutated_assets_radius.insert(asset_class_identifier);
                    mutated_contracts_radius.insert(contract_identifier);

                    let currency = get_standardized_non_fungible_currency_from_asset_class_id(
                        &data.asset_class_identifier,
                        &data.hex_asset_identifier,
                        asset_class_cache,
                    );
                    operations.push(Operation {
                        operation_identifier: OperationIdentifier {
                            index: operation_id,
                            network_index: None,
                        },
                        related_operations: None,
                        type_: OperationType::Debit,
                        status: Some(OperationStatusKind::Success),
                        account: AccountIdentifier {
                            address: data.sender.clone(),
                            sub_account: None,
                        },
                        amount: Some(Amount { value: 1, currency }),
                        metadata: None,
                    });
                    operation_id += 1;
                }
                StacksTransactionEvent::NFTTransferEvent(data) => {
                    let (asset_class_identifier, contract_identifier) =
                        get_mutated_ids(&data.asset_class_identifier);
                    mutated_assets_radius.insert(asset_class_identifier);
                    mutated_contracts_radius.insert(contract_identifier);

                    let currency = get_standardized_non_fungible_currency_from_asset_class_id(
                        &data.asset_class_identifier,
                        &data.hex_asset_identifier,
                        asset_class_cache,
                    );
                    operations.push(Operation {
                        operation_identifier: OperationIdentifier {
                            index: operation_id,
                            network_index: None,
                        },
                        related_operations: Some(vec![OperationIdentifier {
                            index: operation_id + 1,
                            network_index: None,
                        }]),
                        type_: OperationType::Debit,
                        status: Some(OperationStatusKind::Success),
                        account: AccountIdentifier {
                            address: data.sender.clone(),
                            sub_account: None,
                        },
                        amount: Some(Amount {
                            value: 1,
                            currency: currency.clone(),
                        }),
                        metadata: None,
                    });
                    operation_id += 1;
                    operations.push(Operation {
                        operation_identifier: OperationIdentifier {
                            index: operation_id,
                            network_index: None,
                        },
                        related_operations: Some(vec![OperationIdentifier {
                            index: operation_id - 1,
                            network_index: None,
                        }]),
                        type_: OperationType::Credit,
                        status: Some(OperationStatusKind::Success),
                        account: AccountIdentifier {
                            address: data.recipient.clone(),
                            sub_account: None,
                        },
                        amount: Some(Amount { value: 1, currency }),
                        metadata: None,
                    });
                    operation_id += 1;
                }
                StacksTransactionEvent::FTMintEvent(data) => {
                    let (asset_class_identifier, contract_identifier) =
                        get_mutated_ids(&data.asset_class_identifier);
                    mutated_assets_radius.insert(asset_class_identifier);
                    mutated_contracts_radius.insert(contract_identifier);

                    let currency = get_standardized_fungible_currency_from_asset_class_id(
                        &data.asset_class_identifier,
                        asset_class_cache,
                        node_url,
                    );

                    let value = match data.amount.parse::<u128>() {
                        Ok(value) => value,
                        Err(e) => {
                            panic!("unable to parse u64 {:?}: {:?}", data, e);
                        }
                    };

                    operations.push(Operation {
                        operation_identifier: OperationIdentifier {
                            index: operation_id,
                            network_index: None,
                        },
                        related_operations: None,
                        type_: OperationType::Credit,
                        status: Some(OperationStatusKind::Success),
                        account: AccountIdentifier {
                            address: data.recipient.clone(),
                            sub_account: None,
                        },
                        amount: Some(Amount { value, currency }),
                        metadata: None,
                    });
                    operation_id += 1;
                }
                StacksTransactionEvent::FTBurnEvent(data) => {
                    let (asset_class_identifier, contract_identifier) =
                        get_mutated_ids(&data.asset_class_identifier);
                    mutated_assets_radius.insert(asset_class_identifier);
                    mutated_contracts_radius.insert(contract_identifier);

                    let currency = get_standardized_fungible_currency_from_asset_class_id(
                        &data.asset_class_identifier,
                        asset_class_cache,
                        node_url,
                    );
                    operations.push(Operation {
                        operation_identifier: OperationIdentifier {
                            index: operation_id,
                            network_index: None,
                        },
                        related_operations: None,
                        type_: OperationType::Debit,
                        status: Some(OperationStatusKind::Success),
                        account: AccountIdentifier {
                            address: data.sender.clone(),
                            sub_account: None,
                        },
                        amount: Some(Amount {
                            value: data.amount.parse::<u128>().expect("Unable to parse u64"),
                            currency,
                        }),
                        metadata: None,
                    });
                    operation_id += 1;
                }
                StacksTransactionEvent::FTTransferEvent(data) => {
                    let (asset_class_identifier, contract_identifier) =
                        get_mutated_ids(&data.asset_class_identifier);
                    mutated_assets_radius.insert(asset_class_identifier);
                    mutated_contracts_radius.insert(contract_identifier);

                    let currency = get_standardized_fungible_currency_from_asset_class_id(
                        &data.asset_class_identifier,
                        asset_class_cache,
                        node_url,
                    );
                    operations.push(Operation {
                        operation_identifier: OperationIdentifier {
                            index: operation_id,
                            network_index: None,
                        },
                        related_operations: Some(vec![OperationIdentifier {
                            index: operation_id + 1,
                            network_index: None,
                        }]),
                        type_: OperationType::Debit,
                        status: Some(OperationStatusKind::Success),
                        account: AccountIdentifier {
                            address: data.sender.clone(),
                            sub_account: None,
                        },
                        amount: Some(Amount {
                            value: data.amount.parse::<u128>().expect("Unable to parse u64"),
                            currency: currency.clone(),
                        }),
                        metadata: None,
                    });
                    operation_id += 1;
                    operations.push(Operation {
                        operation_identifier: OperationIdentifier {
                            index: operation_id,
                            network_index: None,
                        },
                        related_operations: Some(vec![OperationIdentifier {
                            index: operation_id - 1,
                            network_index: None,
                        }]),
                        type_: OperationType::Credit,
                        status: Some(OperationStatusKind::Success),
                        account: AccountIdentifier {
                            address: data.recipient.clone(),
                            sub_account: None,
                        },
                        amount: Some(Amount {
                            value: data.amount.parse::<u128>().expect("Unable to parse u64"),
                            currency,
                        }),
                        metadata: None,
                    });
                    operation_id += 1;
                }
                StacksTransactionEvent::DataVarSetEvent(_data) => {}
                StacksTransactionEvent::DataMapInsertEvent(_data) => {}
                StacksTransactionEvent::DataMapUpdateEvent(_data) => {}
                StacksTransactionEvent::DataMapDeleteEvent(_data) => {}
                StacksTransactionEvent::SmartContractEvent(data) => {
                    mutated_contracts_radius.insert(data.contract_identifier.clone());
                }
            }
        }
    }

    let receipt =
        StacksTransactionReceipt::new(mutated_contracts_radius, mutated_assets_radius, events);
    (receipt, operations)
}

fn get_mutated_ids(asset_class_id: &str) -> (String, String) {
    let contract_id = asset_class_id.split("::").collect::<Vec<_>>()[0];
    (asset_class_id.into(), contract_id.into())
}

#[cfg(test)]
pub mod tests;
