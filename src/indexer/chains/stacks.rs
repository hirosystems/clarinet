use crate::indexer::AssetClassCache;
use crate::indexer::{IndexerConfig, StacksChainContext};
use crate::types::events::*;
use crate::types::{
    AccountIdentifier, Amount, BlockIdentifier, Currency, CurrencyMetadata, CurrencyStandard,
    Operation, OperationIdentifier, OperationStatusKind, OperationType, StacksBlockData,
    StacksBlockMetadata, StacksContractDeploymentData, StacksMicroblockData, StacksTransactionData,
    StacksTransactionExecutionCost, StacksTransactionKind, StacksTransactionMetadata,
    StacksTransactionReceipt, TransactionIdentifier,
};
use clarity_repl::clarity::codec::transaction::{TransactionAuth, TransactionPayload};
use clarity_repl::clarity::codec::{StacksMessageCodec, StacksTransaction};
use clarity_repl::clarity::types::Value as ClarityValue;
use clarity_repl::clarity::util::hash::hex_bytes;
use rocket::serde::json::Value as JsonValue;
use rocket::serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::convert::TryInto;
use std::io::Cursor;
use std::str;

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct NewBlock {
    block_height: u64,
    block_hash: String,
    burn_block_height: u64,
    burn_block_hash: String,
    parent_block_hash: String,
    index_block_hash: String,
    parent_index_block_hash: String,
    transactions: Vec<NewTransaction>,
    events: Vec<NewEvent>,
    // reward_slot_holders: Vec<String>,
    // burn_amount: u32,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct NewMicroblock {
    transactions: Vec<NewTransaction>,
    events: Vec<NewEvent>,
}

#[derive(Deserialize)]
pub struct NewTransaction {
    pub txid: String,
    pub tx_index: u32,
    pub status: String,
    pub raw_result: String,
    pub raw_tx: String,
    pub execution_cost: Option<StacksTransactionExecutionCost>,
    pub microblock_sequence: Option<u32>,
    pub microblock_hash: Option<String>,
    pub microblock_parent_hash: Option<String>,
}

#[derive(Deserialize)]
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
    pub print_event: Option<JsonValue>,
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

pub fn standardize_stacks_block(
    indexer_config: &IndexerConfig,
    marshalled_block: JsonValue,
    ctx: &mut StacksChainContext,
) -> StacksBlockData {
    let mut block: NewBlock = serde_json::from_value(marshalled_block).unwrap();

    let pox_cycle_length: u64 =
        (ctx.pox_info.prepare_phase_block_length + ctx.pox_info.reward_phase_block_length).into();
    let current_len = block.burn_block_height - ctx.pox_info.first_burnchain_block_height;
    let pox_cycle_id: u32 = (current_len / pox_cycle_length).try_into().unwrap();

    let mut events = vec![];
    events.append(&mut block.events);
    let transactions = block
        .transactions
        .iter()
        .map(|t| {
            let (description, tx_type, fee, sender, sponsor) =
                get_tx_description(&t.raw_tx).expect("unable to parse transaction");
            let (operations, receipt) = get_standardized_stacks_operations(
                &t.txid,
                &mut events,
                &mut ctx.asset_class_map,
                &indexer_config.stacks_node_rpc_url,
            );
            StacksTransactionData {
                transaction_identifier: TransactionIdentifier {
                    hash: t.txid.clone(),
                },
                operations,
                metadata: StacksTransactionMetadata {
                    success: t.status == "success",
                    result: get_value_description(&t.raw_result),
                    raw_tx: t.raw_tx.clone(),
                    sender,
                    fee,
                    sponsor,
                    kind: tx_type,
                    execution_cost: t.execution_cost.clone(),
                    receipt,
                    description,
                },
            }
        })
        .collect();

    StacksBlockData {
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
        },
        transactions,
    }
}

pub fn standardize_stacks_microblock(
    indexer_config: &IndexerConfig,
    marshalled_microblock: JsonValue,
    anchored_block_identifier: &BlockIdentifier,
    ctx: &mut StacksChainContext,
) -> StacksMicroblockData {
    let mut microblock: NewMicroblock = serde_json::from_value(marshalled_microblock).unwrap();

    let mut events = vec![];
    events.append(&mut microblock.events);
    let transactions = microblock
        .transactions
        .iter()
        .map(|t| {
            let (description, tx_type, fee, sender, sponsor) =
                get_tx_description(&t.raw_tx).expect("unable to parse transaction");
            let (operations, receipt) = get_standardized_stacks_operations(
                &t.txid,
                &mut events,
                &mut ctx.asset_class_map,
                &indexer_config.stacks_node_rpc_url,
            );
            StacksTransactionData {
                transaction_identifier: TransactionIdentifier {
                    hash: t.txid.clone(),
                },
                operations,
                metadata: StacksTransactionMetadata {
                    success: t.status == "success",
                    result: get_value_description(&t.raw_result),
                    raw_tx: t.raw_tx.clone(),
                    sender,
                    fee,
                    sponsor,
                    kind: tx_type,
                    execution_cost: t.execution_cost.clone(),
                    receipt,
                    description,
                },
            }
        })
        .collect();

    StacksMicroblockData {
        block_identifier: BlockIdentifier {
            hash: "".into(),
            index: 0,
        },
        parent_block_identifier: anchored_block_identifier.clone(),
        transactions,
    }
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
            println!("{:?}", e);
            return raw_value.to_string();
        }
    };
    value
}

pub fn get_tx_description(
    raw_tx: &str,
) -> Result<
    (
        String, // Human readable transaction's description (contract-call, publish, ...)
        StacksTransactionKind, //
        u64,    // Transaction fee
        String, // Sender's address
        Option<String>, // Sponsor's address (optional)
    ),
    (),
> {
    let raw_tx = match raw_tx.strip_prefix("0x") {
        Some(raw_tx) => raw_tx,
        _ => return Err(()),
    };
    let tx_bytes = match hex_bytes(&raw_tx) {
        Ok(bytes) => bytes,
        _ => return Err(()),
    };
    let tx = match StacksTransaction::consensus_deserialize(&mut Cursor::new(&tx_bytes)) {
        Ok(bytes) => bytes,
        _ => return Err(()),
    };

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
                .collect::<Vec<String>>()
                .join(", ");
            (
                format!(
                    "invoked: {}.{}::{}({})",
                    contract_call.address,
                    contract_call.contract_name,
                    contract_call.function_name,
                    formatted_args
                ),
                StacksTransactionKind::ContractCall,
            )
        }
        TransactionPayload::SmartContract(ref smart_contract) => {
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
        TransactionPayload::Coinbase(_) => (format!("coinbase"), StacksTransactionKind::Coinbase),
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

pub fn get_standardized_stacks_operations(
    txid: &str,
    events: &mut Vec<NewEvent>,
    asset_class_cache: &mut HashMap<String, AssetClassCache>,
    node_url: &str,
) -> (Vec<Operation>, StacksTransactionReceipt) {
    let mut mutated_contracts_radius = HashSet::new();
    let mut mutated_assets_radius = HashSet::new();
    let mut marshalled_events = Vec::new();

    let mut operations = vec![];
    let mut operation_id = 0;

    let mut i = 0;
    while i < events.len() {
        if events[i].txid == txid {
            let event = events.remove(i);
            if let Some(ref event_data) = event.stx_mint_event {
                let data: STXMintEventData = serde_json::from_value(event_data.clone())
                    .expect("Unable to decode event_data");
                marshalled_events.push(StacksTransactionEvent::STXMintEvent(data.clone()));
                operations.push(Operation {
                    operation_identifier: OperationIdentifier {
                        index: operation_id,
                        network_index: None,
                    },
                    related_operations: None,
                    type_: OperationType::Credit,
                    status: Some(OperationStatusKind::Success),
                    account: AccountIdentifier {
                        address: data.recipient,
                        sub_account: None,
                    },
                    amount: Some(Amount {
                        value: data.amount.parse::<u64>().expect("Unable to parse u64"),
                        currency: get_stacks_currency(),
                    }),
                    metadata: None,
                });
                operation_id += 1;
            } else if let Some(ref event_data) = event.stx_lock_event {
                let data: STXLockEventData = serde_json::from_value(event_data.clone())
                    .expect("Unable to decode event_data");
                marshalled_events.push(StacksTransactionEvent::STXLockEvent(data.clone()));
                operations.push(Operation {
                    operation_identifier: OperationIdentifier {
                        index: operation_id,
                        network_index: None,
                    },
                    related_operations: None,
                    type_: OperationType::Lock,
                    status: Some(OperationStatusKind::Success),
                    account: AccountIdentifier {
                        address: data.locked_address,
                        sub_account: None,
                    },
                    amount: Some(Amount {
                        value: data
                            .locked_amount
                            .parse::<u64>()
                            .expect("Unable to parse u64"),
                        currency: get_stacks_currency(),
                    }),
                    metadata: None,
                });
                operation_id += 1;
            } else if let Some(ref event_data) = event.stx_burn_event {
                let data: STXBurnEventData = serde_json::from_value(event_data.clone())
                    .expect("Unable to decode event_data");
                marshalled_events.push(StacksTransactionEvent::STXBurnEvent(data.clone()));
                operations.push(Operation {
                    operation_identifier: OperationIdentifier {
                        index: operation_id,
                        network_index: None,
                    },
                    related_operations: None,
                    type_: OperationType::Debit,
                    status: Some(OperationStatusKind::Success),
                    account: AccountIdentifier {
                        address: data.sender,
                        sub_account: None,
                    },
                    amount: Some(Amount {
                        value: data.amount.parse::<u64>().expect("Unable to parse u64"),
                        currency: get_stacks_currency(),
                    }),
                    metadata: None,
                });
                operation_id += 1;
            } else if let Some(ref event_data) = event.stx_transfer_event {
                let data: STXTransferEventData = serde_json::from_value(event_data.clone())
                    .expect("Unable to decode event_data");
                marshalled_events.push(StacksTransactionEvent::STXTransferEvent(data.clone()));
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
                        address: data.sender,
                        sub_account: None,
                    },
                    amount: Some(Amount {
                        value: data.amount.parse::<u64>().expect("Unable to parse u64"),
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
                        address: data.recipient,
                        sub_account: None,
                    },
                    amount: Some(Amount {
                        value: data.amount.parse::<u64>().expect("Unable to parse u64"),
                        currency: get_stacks_currency(),
                    }),
                    metadata: None,
                });
                operation_id += 1;
            } else if let Some(ref event_data) = event.nft_mint_event {
                let data: NFTMintEventData = serde_json::from_value(event_data.clone())
                    .expect("Unable to decode event_data");
                marshalled_events.push(StacksTransactionEvent::NFTMintEvent(data.clone()));
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
                        address: data.recipient,
                        sub_account: None,
                    },
                    amount: Some(Amount { value: 1, currency }),
                    metadata: None,
                });
                operation_id += 1;
            } else if let Some(ref event_data) = event.nft_burn_event {
                let data: NFTBurnEventData = serde_json::from_value(event_data.clone())
                    .expect("Unable to decode event_data");
                marshalled_events.push(StacksTransactionEvent::NFTBurnEvent(data.clone()));
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
                        address: data.sender,
                        sub_account: None,
                    },
                    amount: Some(Amount { value: 1, currency }),
                    metadata: None,
                });
                operation_id += 1;
            } else if let Some(ref event_data) = event.nft_transfer_event {
                let data: NFTTransferEventData = serde_json::from_value(event_data.clone())
                    .expect("Unable to decode event_data");
                marshalled_events.push(StacksTransactionEvent::NFTTransferEvent(data.clone()));
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
                        address: data.sender,
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
                        address: data.recipient,
                        sub_account: None,
                    },
                    amount: Some(Amount { value: 1, currency }),
                    metadata: None,
                });
                operation_id += 1;
            } else if let Some(ref event_data) = event.ft_mint_event {
                let data: FTMintEventData = serde_json::from_value(event_data.clone())
                    .expect("Unable to decode event_data");
                marshalled_events.push(StacksTransactionEvent::FTMintEvent(data.clone()));
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
                    type_: OperationType::Credit,
                    status: Some(OperationStatusKind::Success),
                    account: AccountIdentifier {
                        address: data.recipient,
                        sub_account: None,
                    },
                    amount: Some(Amount {
                        value: data.amount.parse::<u64>().expect("Unable to parse u64"),
                        currency,
                    }),
                    metadata: None,
                });
                operation_id += 1;
            } else if let Some(ref event_data) = event.ft_burn_event {
                let data: FTBurnEventData = serde_json::from_value(event_data.clone())
                    .expect("Unable to decode event_data");
                marshalled_events.push(StacksTransactionEvent::FTBurnEvent(data.clone()));
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
                        address: data.sender,
                        sub_account: None,
                    },
                    amount: Some(Amount {
                        value: data.amount.parse::<u64>().expect("Unable to parse u64"),
                        currency,
                    }),
                    metadata: None,
                });
                operation_id += 1;
            } else if let Some(ref event_data) = event.ft_transfer_event {
                let data: FTTransferEventData = serde_json::from_value(event_data.clone())
                    .expect("Unable to decode event_data");
                marshalled_events.push(StacksTransactionEvent::FTTransferEvent(data.clone()));
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
                        address: data.sender,
                        sub_account: None,
                    },
                    amount: Some(Amount {
                        value: data.amount.parse::<u64>().expect("Unable to parse u64"),
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
                        address: data.recipient,
                        sub_account: None,
                    },
                    amount: Some(Amount {
                        value: data.amount.parse::<u64>().expect("Unable to parse u64"),
                        currency,
                    }),
                    metadata: None,
                });
                operation_id += 1;
            } else if let Some(ref event_data) = event.data_var_set_event {
                let data: DataVarSetEventData = serde_json::from_value(event_data.clone())
                    .expect("Unable to decode event_data");
                marshalled_events.push(StacksTransactionEvent::DataVarSetEvent(data.clone()));
                mutated_contracts_radius.insert(data.contract_identifier.clone());
            } else if let Some(ref event_data) = event.data_map_insert_event {
                let data: DataMapInsertEventData = serde_json::from_value(event_data.clone())
                    .expect("Unable to decode event_data");
                marshalled_events.push(StacksTransactionEvent::DataMapInsertEvent(data.clone()));
                mutated_contracts_radius.insert(data.contract_identifier.clone());
            } else if let Some(ref event_data) = event.data_map_update_event {
                let data: DataMapUpdateEventData = serde_json::from_value(event_data.clone())
                    .expect("Unable to decode event_data");
                marshalled_events.push(StacksTransactionEvent::DataMapUpdateEvent(data.clone()));
                mutated_contracts_radius.insert(data.contract_identifier.clone());
            } else if let Some(ref event_data) = event.data_map_delete_event {
                let data: DataMapDeleteEventData = serde_json::from_value(event_data.clone())
                    .expect("Unable to decode event_data");
                marshalled_events.push(StacksTransactionEvent::DataMapDeleteEvent(data.clone()));
                mutated_contracts_radius.insert(data.contract_identifier.clone());
            } else if let Some(ref event_data) = event.print_event {
                let data: SmartContractEventData = serde_json::from_value(event_data.clone())
                    .expect("Unable to decode event_data");
                marshalled_events.push(StacksTransactionEvent::SmartContractEvent(data.clone()));
                mutated_contracts_radius.insert(data.contract_identifier.clone());
            }
        } else {
            i += 1;
        }
    }
    let receipt = StacksTransactionReceipt::new(
        mutated_contracts_radius,
        mutated_assets_radius,
        marshalled_events,
    );
    (operations, receipt)
}

fn get_mutated_ids(asset_class_id: &str) -> (String, String) {
    let contract_id = asset_class_id.split("::").collect::<Vec<_>>()[0];
    (asset_class_id.into(), contract_id.into())
}
