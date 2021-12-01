use crate::indexer::AssetClassCache;
use crate::indexer::{IndexerConfig, StacksChainContext};
use crate::types::{
    AccountIdentifier, Amount, BlockIdentifier, Currency, CurrencyMetadata, CurrencyStandard,
    Operation, OperationIdentifier, OperationStatusKind, OperationType, StacksBlockData,
    StacksBlockMetadata, StacksTransactionData, StacksTransactionMetadata, TransactionIdentifier,
};
use crate::utils::stacks::{transactions, StacksRpc};
use clarity_repl::clarity::codec::transaction::TransactionPayload;
use clarity_repl::clarity::codec::{StacksMessageCodec, StacksTransaction};
use clarity_repl::clarity::types::Value as ClarityValue;
use clarity_repl::clarity::util::hash::hex_bytes;
use rocket::serde::json::Value as JsonValue;
use rocket::serde::Deserialize;
use std::collections::HashMap;
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
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct STXTransferEventData {
    pub sender: String,
    pub recipient: String,
    pub amount: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct STXMintEventData {
    pub recipient: String,
    pub amount: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct STXLockEventData {
    pub locked_amount: String,
    pub unlock_height: u64,
    pub locked_address: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct STXBurnEventData {
    pub sender: String,
    pub amount: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct NFTTransferEventData {
    #[serde(rename = "asset_identifier")]
    pub asset_class_identifier: String,
    #[serde(rename = "value")]
    pub asset_identifier: String,
    pub sender: String,
    pub recipient: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct NFTMintEventData {
    #[serde(rename = "asset_identifier")]
    pub asset_class_identifier: String,
    #[serde(rename = "value")]
    pub asset_identifier: String,
    pub recipient: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct NFTBurnEventData {
    #[serde(rename = "asset_identifier")]
    pub asset_class_identifier: String,
    #[serde(rename = "value")]
    pub asset_identifier: String,
    pub sender: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct FTTransferEventData {
    #[serde(rename = "asset_identifier")]
    pub asset_class_identifier: String,
    pub sender: String,
    pub recipient: String,
    pub amount: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct FTMintEventData {
    #[serde(rename = "asset_identifier")]
    pub asset_class_identifier: String,
    pub recipient: String,
    pub amount: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct FTBurnEventData {
    #[serde(rename = "asset_identifier")]
    pub asset_class_identifier: String,
    pub sender: String,
    pub amount: String,
}

pub fn get_stacks_currency() -> Currency {
    Currency {
        symbol: "STX".into(),
        decimals: 6,
        metadata: None,
    }
}

#[derive(Deserialize, Debug)]
struct ContractReadonlyCall {
    okay: bool,
    result: String,
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
            let description = get_tx_description(&t.raw_tx);
            StacksTransactionData {
                transaction_identifier: TransactionIdentifier {
                    hash: t.txid.clone(),
                },
                operations: get_standardized_stacks_operations(
                    t,
                    &mut events,
                    &mut ctx.asset_class_map,
                    &indexer_config.stacks_node_rpc_url,
                ),
                metadata: StacksTransactionMetadata {
                    success: t.status == "success",
                    result: get_value_description(&t.raw_result),
                    events: vec![],
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
            index: block.block_height,
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

pub fn get_standardized_fungible_currency_from_asset_class_id(
    asset_class_id: &str,
    asset_class_cache: &mut HashMap<String, AssetClassCache>,
    node_url: &str,
) -> Currency {
    match asset_class_cache.get(asset_class_id) {
        None => {
            let comps = asset_class_id.split("::").collect::<Vec<&str>>();
            let principal = comps[0].split(".").collect::<Vec<&str>>();

            let get_symbol_request_url = format!(
                "{}/v2/contracts/call-read/{}/{}/get-symbol",
                node_url, principal[0], principal[1],
            );

            println!("get_standardized_fungible_currency_from_asset_class_id");

            let symbol_res: ContractReadonlyCall = reqwest::blocking::get(&get_symbol_request_url)
                .expect("Unable to retrieve account")
                .json()
                .expect("Unable to parse contract");

            let raw_value = match symbol_res.result.strip_prefix("0x") {
                Some(raw_value) => raw_value,
                _ => panic!(),
            };
            let value_bytes = match hex_bytes(&raw_value) {
                Ok(bytes) => bytes,
                _ => panic!(),
            };

            let symbol = match ClarityValue::consensus_deserialize(&mut Cursor::new(&value_bytes)) {
                Ok(value) => value.expect_result_ok().expect_u128(),
                _ => panic!(),
            };

            let get_decimals_request_url = format!(
                "{}/v2/contracts/call-read/{}/{}/get-decimals",
                node_url, principal[0], principal[1],
            );

            let decimals_res: ContractReadonlyCall =
                reqwest::blocking::get(&get_decimals_request_url)
                    .expect("Unable to retrieve account")
                    .json()
                    .expect("Unable to parse contract");

            let raw_value = match decimals_res.result.strip_prefix("0x") {
                Some(raw_value) => raw_value,
                _ => panic!(),
            };
            let value_bytes = match hex_bytes(&raw_value) {
                Ok(bytes) => bytes,
                _ => panic!(),
            };

            let value = match ClarityValue::consensus_deserialize(&mut Cursor::new(&value_bytes)) {
                Ok(value) => value.expect_result_ok().expect_u128(),
                _ => panic!(),
            };

            let entry = AssetClassCache {
                symbol: format!("{}", symbol),
                decimals: value as u8,
            };

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
    asset_class_cache: &mut HashMap<String, AssetClassCache>,
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
    transaction: &NewTransaction,
    events: &mut Vec<NewEvent>,
    asset_class_cache: &mut HashMap<String, AssetClassCache>,
    node_url: &str,
) -> Vec<Operation> {
    let mut operations = vec![];
    let mut operation_id = 0;

    let mut i = 0;
    while i < events.len() {
        if events[i].txid == transaction.txid {
            let event = events.remove(i);
            if let Some(ref event_data) = event.stx_mint_event {
                let data: STXMintEventData = serde_json::from_value(event_data.clone())
                    .expect("Unable to decode event_data");
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
                let currency = get_standardized_non_fungible_currency_from_asset_class_id(
                    &data.asset_class_identifier,
                    &data.asset_identifier,
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
                let currency = get_standardized_non_fungible_currency_from_asset_class_id(
                    &data.asset_class_identifier,
                    &data.asset_identifier,
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
                let currency = get_standardized_non_fungible_currency_from_asset_class_id(
                    &data.asset_class_identifier,
                    &data.asset_identifier,
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
            }
        } else {
            i += 1;
        }
    }
    operations
}
