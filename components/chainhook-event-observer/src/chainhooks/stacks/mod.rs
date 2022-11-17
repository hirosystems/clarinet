use crate::utils::AbstractStacksBlock;

use super::types::{
    HookAction, StacksChainhookSpecification, StacksContractDeploymentPredicate,
    StacksTransactionFilterPredicate,
};
use chainhook_types::{
    BlockIdentifier, StacksChainEvent, StacksTransactionData, StacksTransactionEvent,
    StacksTransactionKind, TransactionIdentifier,
};
use clarity_repl::clarity::codec::StacksMessageCodec;
use clarity_repl::clarity::util::hash::hex_bytes;
use clarity_repl::clarity::vm::types::{CharType, SequenceData, Value as ClarityValue};
use reqwest::{Client, Method};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::io::Cursor;

use reqwest::RequestBuilder;

pub struct StacksTriggerChainhook<'a> {
    pub chainhook: &'a StacksChainhookSpecification,
    pub apply: Vec<(Vec<&'a StacksTransactionData>, &'a dyn AbstractStacksBlock)>,
    pub rollback: Vec<(Vec<&'a StacksTransactionData>, &'a dyn AbstractStacksBlock)>,
}

#[derive(Clone, Debug)]
pub struct StacksApplyTransactionPayload {
    pub block_identifier: BlockIdentifier,
    pub transactions: Vec<StacksTransactionData>,
}

#[derive(Clone, Debug)]
pub struct StacksRollbackTransactionPayload {
    pub block_identifier: BlockIdentifier,
    pub transactions: Vec<StacksTransactionData>,
}

#[derive(Clone, Debug)]
pub struct StacksChainhookPayload {
    pub uuid: String,
}

#[derive(Clone, Debug)]
pub struct StacksChainhookOccurrencePayload {
    pub apply: Vec<StacksApplyTransactionPayload>,
    pub rollback: Vec<StacksRollbackTransactionPayload>,
    pub chainhook: StacksChainhookPayload,
}
pub enum StacksChainhookOccurrence {
    Http(RequestBuilder),
    File(String, Vec<u8>),
    Data(StacksChainhookOccurrencePayload),
}

impl<'a> StacksTriggerChainhook<'a> {
    pub fn should_decode_clarity_value(&self) -> bool {
        self.chainhook.decode_clarity_values.unwrap_or(false)
    }
}

pub fn evaluate_stacks_chainhooks_on_chain_event<'a>(
    chain_event: &'a StacksChainEvent,
    active_chainhooks: Vec<&'a StacksChainhookSpecification>,
) -> Vec<StacksTriggerChainhook<'a>> {
    let mut triggered_chainhooks = vec![];
    match chain_event {
        StacksChainEvent::ChainUpdatedWithBlocks(update) => {
            for chainhook in active_chainhooks.iter() {
                let mut apply = vec![];
                let mut rollback = vec![];
                for block_update in update.new_blocks.iter() {
                    for parents_microblock_to_apply in
                        block_update.parent_microblocks_to_apply.iter()
                    {
                        apply.append(&mut evaluate_stacks_chainhook_on_blocks(
                            vec![parents_microblock_to_apply],
                            chainhook,
                        ));
                    }
                    for parents_microblock_to_rolllback in
                        block_update.parent_microblocks_to_rollback.iter()
                    {
                        rollback.append(&mut evaluate_stacks_chainhook_on_blocks(
                            vec![parents_microblock_to_rolllback],
                            chainhook,
                        ));
                    }
                    apply.append(&mut evaluate_stacks_chainhook_on_blocks(
                        vec![&block_update.block],
                        chainhook,
                    ));
                }
                if !apply.is_empty() || !rollback.is_empty() {
                    triggered_chainhooks.push(StacksTriggerChainhook {
                        chainhook,
                        apply,
                        rollback,
                    })
                }
            }
        }
        StacksChainEvent::ChainUpdatedWithMicroblocks(update) => {
            for chainhook in active_chainhooks.iter() {
                let mut apply = vec![];
                let rollback = vec![];

                for microblock_to_apply in update.new_microblocks.iter() {
                    apply.append(&mut evaluate_stacks_chainhook_on_blocks(
                        vec![microblock_to_apply],
                        chainhook,
                    ));
                }
                if !apply.is_empty() || !rollback.is_empty() {
                    triggered_chainhooks.push(StacksTriggerChainhook {
                        chainhook,
                        apply,
                        rollback,
                    })
                }
            }
        }
        StacksChainEvent::ChainUpdatedWithMicroblocksReorg(update) => {
            for chainhook in active_chainhooks.iter() {
                let mut apply = vec![];
                let mut rollback = vec![];

                for microblock_to_apply in update.microblocks_to_apply.iter() {
                    apply.append(&mut evaluate_stacks_chainhook_on_blocks(
                        vec![microblock_to_apply],
                        chainhook,
                    ));
                }
                for microblock_to_rollback in update.microblocks_to_rollback.iter() {
                    rollback.append(&mut evaluate_stacks_chainhook_on_blocks(
                        vec![microblock_to_rollback],
                        chainhook,
                    ));
                }
                if !apply.is_empty() || !rollback.is_empty() {
                    triggered_chainhooks.push(StacksTriggerChainhook {
                        chainhook,
                        apply,
                        rollback,
                    })
                }
            }
        }
        StacksChainEvent::ChainUpdatedWithReorg(update) => {
            for chainhook in active_chainhooks.iter() {
                let mut apply = vec![];
                let mut rollback = vec![];

                for block_update in update.blocks_to_apply.iter() {
                    for parents_microblock_to_apply in
                        block_update.parent_microblocks_to_apply.iter()
                    {
                        apply.append(&mut evaluate_stacks_chainhook_on_blocks(
                            vec![parents_microblock_to_apply],
                            chainhook,
                        ));
                    }
                    apply.append(&mut evaluate_stacks_chainhook_on_blocks(
                        vec![&block_update.block],
                        chainhook,
                    ));
                }
                for block_update in update.blocks_to_rollback.iter() {
                    for parents_microblock_to_rollback in
                        block_update.parent_microblocks_to_rollback.iter()
                    {
                        rollback.append(&mut evaluate_stacks_chainhook_on_blocks(
                            vec![parents_microblock_to_rollback],
                            chainhook,
                        ));
                    }
                    rollback.append(&mut evaluate_stacks_chainhook_on_blocks(
                        vec![&block_update.block],
                        chainhook,
                    ));
                }
                if !apply.is_empty() || !rollback.is_empty() {
                    triggered_chainhooks.push(StacksTriggerChainhook {
                        chainhook,
                        apply,
                        rollback,
                    })
                }
            }
        }
    }
    triggered_chainhooks
}

fn evaluate_stacks_chainhook_on_blocks<'a>(
    blocks: Vec<&'a dyn AbstractStacksBlock>,
    chainhook: &'a StacksChainhookSpecification,
) -> Vec<(Vec<&'a StacksTransactionData>, &'a dyn AbstractStacksBlock)> {
    let mut occurrences = vec![];
    for block in blocks {
        let mut hits = vec![];
        for tx in block.get_transactions().iter() {
            if evaluate_stacks_transaction_predicate_on_transaction(tx, chainhook) {
                hits.push(tx);
            }
        }
        if hits.len() > 0 {
            occurrences.push((hits, block));
        }
    }
    occurrences
}

pub fn evaluate_stacks_transaction_predicate_on_transaction<'a>(
    transaction: &'a StacksTransactionData,
    chainhook: &'a StacksChainhookSpecification,
) -> bool {
    match &chainhook.transaction_predicate {
        StacksTransactionFilterPredicate::ContractDeployment(
            StacksContractDeploymentPredicate::Principal(expected_deployer),
        ) => match &transaction.metadata.kind {
            StacksTransactionKind::ContractDeployment(actual_deployment) => actual_deployment
                .contract_identifier
                .starts_with(expected_deployer),
            _ => false,
        },
        StacksTransactionFilterPredicate::ContractDeployment(
            StacksContractDeploymentPredicate::Trait(_expected_trait),
        ) => match &transaction.metadata.kind {
            StacksTransactionKind::ContractDeployment(_actual_deployment) => {
                warn!("StacksContractDeploymentPredicate::Trait uninmplemented");
                false
            }
            _ => false,
        },
        StacksTransactionFilterPredicate::ContractCall(expected_contract_call) => {
            match &transaction.metadata.kind {
                StacksTransactionKind::ContractCall(actual_contract_call) => {
                    actual_contract_call
                        .contract_identifier
                        .eq(&expected_contract_call.contract_identifier)
                        && actual_contract_call
                            .method
                            .eq(&expected_contract_call.method)
                }
                _ => false,
            }
        }
        StacksTransactionFilterPredicate::FtEvent(expected_event) => {
            let expecting_mint = expected_event.actions.contains(&"mint".to_string());
            let expecting_transfer = expected_event.actions.contains(&"transfer".to_string());
            let expecting_burn = expected_event.actions.contains(&"burn".to_string());

            for event in transaction.metadata.receipt.events.iter() {
                match (event, expecting_mint, expecting_transfer, expecting_burn) {
                    (StacksTransactionEvent::FTMintEvent(_), true, _, _) => return true,
                    (StacksTransactionEvent::FTTransferEvent(_), _, true, _) => return true,
                    (StacksTransactionEvent::FTBurnEvent(_), _, _, true) => return true,
                    _ => continue,
                }
            }
            false
        }
        StacksTransactionFilterPredicate::NftEvent(expected_event) => {
            let expecting_mint = expected_event.actions.contains(&"mint".to_string());
            let expecting_transfer = expected_event.actions.contains(&"transfer".to_string());
            let expecting_burn = expected_event.actions.contains(&"burn".to_string());

            for event in transaction.metadata.receipt.events.iter() {
                match (event, expecting_mint, expecting_transfer, expecting_burn) {
                    (StacksTransactionEvent::NFTMintEvent(_), true, _, _) => return true,
                    (StacksTransactionEvent::NFTTransferEvent(_), _, true, _) => return true,
                    (StacksTransactionEvent::NFTBurnEvent(_), _, _, true) => return true,
                    _ => continue,
                }
            }
            false
        }
        StacksTransactionFilterPredicate::StxEvent(expected_event) => {
            let expecting_mint = expected_event.actions.contains(&"mint".to_string());
            let expecting_transfer = expected_event.actions.contains(&"transfer".to_string());
            let expecting_lock = expected_event.actions.contains(&"lock".to_string());

            for event in transaction.metadata.receipt.events.iter() {
                match (event, expecting_mint, expecting_transfer, expecting_lock) {
                    (StacksTransactionEvent::STXMintEvent(_), true, _, _) => return true,
                    (StacksTransactionEvent::STXTransferEvent(_), _, true, _) => return true,
                    (StacksTransactionEvent::STXLockEvent(_), _, _, true) => return true,
                    _ => continue,
                }
            }
            false
        }
        StacksTransactionFilterPredicate::PrintEvent(expected_event) => {
            for event in transaction.metadata.receipt.events.iter() {
                match event {
                    StacksTransactionEvent::SmartContractEvent(actual) => {
                        if actual.contract_identifier == expected_event.contract_identifier {
                            let value =
                                format!("{}", expect_decoded_clarity_value(&actual.hex_value));
                            if value.contains(&expected_event.contains) {
                                return true;
                            }
                        }
                    }
                    _ => {}
                }
            }
            false
        }
        StacksTransactionFilterPredicate::TransactionIdentifierHash(txid) => {
            txid.eq(&transaction.transaction_identifier.hash)
        }
    }
}

fn encode_transaction_including_with_clarity_decoding(
    transaction: &StacksTransactionData,
) -> serde_json::Value {
    json!({
        "transaction_identifier": transaction.transaction_identifier,
        "operations": transaction.operations,
        "metadata": {
            "success": transaction.metadata.success,
            "raw_tx": transaction.metadata.raw_tx,
            "result": serialized_decoded_clarity_value(&transaction.metadata.result),
            "sender": transaction.metadata.sender,
            "fee": transaction.metadata.fee,
            "kind": transaction.metadata.kind,
            "receipt": {
                "mutated_contracts_radius": transaction.metadata.receipt.mutated_contracts_radius,
                "mutated_assets_radius": transaction.metadata.receipt.mutated_assets_radius,
                "contract_calls_stack": transaction.metadata.receipt.contract_calls_stack,
                "events": transaction.metadata.receipt.events.iter().map(|event| {
                    serialized_event_with_decoded_clarity_value(event)
                }).collect::<Vec<serde_json::Value>>(),
            },
            "description": transaction.metadata.description,
            "sponsor": transaction.metadata.sponsor,
            "execution_cost": transaction.metadata.execution_cost,
            "position": transaction.metadata.position,
        },
    })
}

pub fn serialized_event_with_decoded_clarity_value(
    event: &StacksTransactionEvent,
) -> serde_json::Value {
    match event {
        StacksTransactionEvent::STXTransferEvent(payload) => {
            json!({
                "type": "STXTransferEvent",
                "data": payload
            })
        }
        StacksTransactionEvent::STXMintEvent(payload) => {
            json!({
                "type": "STXMintEvent",
                "data": payload
            })
        }
        StacksTransactionEvent::STXLockEvent(payload) => {
            json!({
                "type": "STXLockEvent",
                "data": payload
            })
        }
        StacksTransactionEvent::STXBurnEvent(payload) => {
            json!({
                "type": "STXBurnEvent",
                "data": payload
            })
        }
        StacksTransactionEvent::NFTTransferEvent(payload) => {
            json!({
                "type": "NFTTransferEvent",
                "data": {
                    "asset_class_identifier": payload.asset_class_identifier,
                    "asset_identifier": serialized_decoded_clarity_value(&payload.hex_asset_identifier),
                    "sender": payload.sender,
                    "recipient": payload.recipient,
                }
            })
        }
        StacksTransactionEvent::NFTMintEvent(payload) => {
            json!({
                "type": "NFTMintEvent",
                "data": {
                    "asset_class_identifier": payload.asset_class_identifier,
                    "asset_identifier": serialized_decoded_clarity_value(&payload.hex_asset_identifier),
                    "recipient": payload.recipient,
                }
            })
        }
        StacksTransactionEvent::NFTBurnEvent(payload) => {
            json!({
                "type": "NFTBurnEvent",
                "data": {
                    "asset_class_identifier": payload.asset_class_identifier,
                    "asset_identifier": serialized_decoded_clarity_value(&payload.hex_asset_identifier),
                    "sender": payload.sender,
                }
            })
        }
        StacksTransactionEvent::FTTransferEvent(payload) => {
            json!({
                "type": "FTTransferEvent",
                "data": payload
            })
        }
        StacksTransactionEvent::FTMintEvent(payload) => {
            json!({
                "type": "FTMintEvent",
                "data": payload
            })
        }
        StacksTransactionEvent::FTBurnEvent(payload) => {
            json!({
                "type": "FTBurnEvent",
                "data": payload
            })
        }
        StacksTransactionEvent::DataVarSetEvent(payload) => {
            json!({
                "type": "DataVarSetEvent",
                "data": {
                    "contract_identifier": payload.contract_identifier,
                    "var": payload.var,
                    "new_value": serialized_decoded_clarity_value(&payload.hex_new_value),
                }
            })
        }
        StacksTransactionEvent::DataMapInsertEvent(payload) => {
            json!({
                "type": "DataMapInsertEvent",
                "data": {
                    "contract_identifier": payload.contract_identifier,
                    "map": payload.map,
                    "inserted_key": serialized_decoded_clarity_value(&payload.hex_inserted_key),
                    "inserted_value": serialized_decoded_clarity_value(&payload.hex_inserted_value),
                }
            })
        }
        StacksTransactionEvent::DataMapUpdateEvent(payload) => {
            json!({
                "type": "DataMapUpdateEvent",
                "data": {
                    "contract_identifier": payload.contract_identifier,
                    "map": payload.map,
                    "key": serialized_decoded_clarity_value(&payload.hex_key),
                    "new_value": serialized_decoded_clarity_value(&payload.hex_new_value),
                }
            })
        }
        StacksTransactionEvent::DataMapDeleteEvent(payload) => {
            json!({
                "type": "DataMapDeleteEvent",
                "data": {
                    "contract_identifier": payload.contract_identifier,
                    "map": payload.map,
                    "deleted_key": serialized_decoded_clarity_value(&payload.hex_deleted_key),
                }
            })
        }
        StacksTransactionEvent::SmartContractEvent(payload) => {
            json!({
                "type": "SmartContractEvent",
                "data": {
                    "contract_identifier": payload.contract_identifier,
                    "topic": payload.topic,
                    "value": serialized_decoded_clarity_value(&payload.hex_value),
                }
            })
        }
    }
}

pub fn expect_decoded_clarity_value(hex_value: &str) -> ClarityValue {
    let hex_value = hex_value
        .strip_prefix("0x")
        .expect("unable to decode clarity value emitted by stacks-node");
    let value_bytes =
        hex_bytes(&hex_value).expect("unable to decode clarity value emitted by stacks-node");
    ClarityValue::consensus_deserialize(&mut Cursor::new(&value_bytes))
        .expect("unable to decode clarity value emitted by stacks-node")
}

pub fn serialized_decoded_clarity_value(hex_value: &str) -> serde_json::Value {
    let hex_value = match hex_value.strip_prefix("0x") {
        Some(hex_value) => hex_value,
        _ => return json!(hex_value.to_string()),
    };
    let value_bytes = match hex_bytes(&hex_value) {
        Ok(bytes) => bytes,
        _ => return json!(hex_value.to_string()),
    };
    let value = match ClarityValue::consensus_deserialize(&mut Cursor::new(&value_bytes)) {
        Ok(value) => serialize_to_json(&value),
        Err(e) => {
            error!("unable to deserialize clarity value {:?}", e);
            return json!(hex_value.to_string());
        }
    };
    value
}

pub fn serialize_to_json(value: &ClarityValue) -> serde_json::Value {
    match value {
        ClarityValue::Int(int) => json!(int),
        ClarityValue::UInt(int) => json!(int),
        ClarityValue::Bool(boolean) => json!(boolean),
        ClarityValue::Principal(principal_data) => json!(format!("{}", principal_data)),
        ClarityValue::Sequence(SequenceData::Buffer(vec_bytes)) => {
            json!(format!("0x{}", &vec_bytes))
        }
        ClarityValue::Sequence(SequenceData::String(CharType::ASCII(string))) => {
            json!(String::from_utf8(string.data.clone()).unwrap())
        }
        ClarityValue::Sequence(SequenceData::String(CharType::UTF8(string))) => {
            let mut result = String::new();
            for c in string.data.iter() {
                if c.len() > 1 {
                    result.push_str(&String::from_utf8(c.to_vec()).unwrap());
                } else {
                    result.push(c[0] as char)
                }
            }
            json!(result)
        }
        ClarityValue::Optional(opt_data) => match &opt_data.data {
            None => serde_json::Value::Null,
            Some(value) => serialize_to_json(&*value),
        },
        ClarityValue::Response(res_data) => {
            json!({
                "result": {
                    "success": res_data.committed,
                    "value": serialize_to_json(&*res_data.data),
                }
            })
        }
        ClarityValue::Tuple(data) => {
            let mut map = serde_json::Map::new();
            for (name, value) in data.data_map.iter() {
                map.insert(name.to_string(), serialize_to_json(value));
            }
            json!(map)
        }
        ClarityValue::Sequence(SequenceData::List(list_data)) => {
            let mut list = vec![];
            for value in list_data.data.iter() {
                list.push(serialize_to_json(value));
            }
            json!(list)
        }
    }
}

pub fn serialize_stacks_payload_to_json<'a>(
    trigger: StacksTriggerChainhook<'a>,
    _proofs: &HashMap<&'a TransactionIdentifier, String>,
) -> JsonValue {
    let decode_clarity_values = trigger.should_decode_clarity_value();
    json!({
        "apply": trigger.apply.into_iter().map(|(transactions, block)| {
            json!({
                "block_identifier": block.get_identifier(),
                "parent_block_identifier": block.get_parent_identifier(),
                "timestamp": block.get_timestamp(),
                "transactions": transactions.iter().map(|transaction| {
                    if decode_clarity_values {
                        encode_transaction_including_with_clarity_decoding(transaction)
                    } else {
                        json!(transaction)
                    }
                }).collect::<Vec<_>>(),
                "metadata": block.get_serialized_metadata(),
            })
        }).collect::<Vec<_>>(),
        "rollback": trigger.rollback.into_iter().map(|(transactions, block)| {
            json!({
                "block_identifier": block.get_identifier(),
                "parent_block_identifier": block.get_parent_identifier(),
                "timestamp": block.get_timestamp(),
                "transactions": transactions.iter().map(|transaction| {
                    if decode_clarity_values {
                        encode_transaction_including_with_clarity_decoding(transaction)
                    } else {
                        json!(transaction)
                    }
                }).collect::<Vec<_>>(),
                "metadata": block.get_serialized_metadata(),
                // "proof": proofs.get(&transaction.transaction_identifier),
            })
        }).collect::<Vec<_>>(),
        "chainhook": {
            "uuid": trigger.chainhook.uuid,
            "transaction_predicate": trigger.chainhook.transaction_predicate,
            "block_predicate": trigger.chainhook.transaction_predicate,
        }
    })
}

pub fn handle_stacks_hook_action<'a>(
    trigger: StacksTriggerChainhook<'a>,
    proofs: &HashMap<&'a TransactionIdentifier, String>,
) -> Option<StacksChainhookOccurrence> {
    match &trigger.chainhook.action {
        HookAction::Http(http) => {
            let client = Client::builder().build().unwrap();
            let host = format!("{}", http.url);
            let method = Method::from_bytes(http.method.as_bytes()).unwrap();
            let body =
                serde_json::to_vec(&serialize_stacks_payload_to_json(trigger, proofs)).unwrap();
            Some(StacksChainhookOccurrence::Http(
                client
                    .request(method, &host)
                    .header("Content-Type", "application/json")
                    .body(body),
            ))
        }
        HookAction::File(disk) => {
            let bytes =
                serde_json::to_vec(&serialize_stacks_payload_to_json(trigger, proofs)).unwrap();
            Some(StacksChainhookOccurrence::File(
                disk.path.to_string(),
                bytes,
            ))
        }
        HookAction::Noop => Some(StacksChainhookOccurrence::Data(
            StacksChainhookOccurrencePayload {
                apply: trigger
                    .apply
                    .into_iter()
                    .map(|(transactions, block)| {
                        let transactions = transactions
                            .into_iter()
                            .map(|t| t.clone())
                            .collect::<Vec<_>>();
                        StacksApplyTransactionPayload {
                            block_identifier: block.get_identifier().clone(),
                            transactions,
                        }
                    })
                    .collect::<Vec<_>>(),
                rollback: trigger
                    .rollback
                    .into_iter()
                    .map(|(transactions, block)| {
                        let transactions = transactions
                            .into_iter()
                            .map(|t| t.clone())
                            .collect::<Vec<_>>();
                        StacksRollbackTransactionPayload {
                            block_identifier: block.get_identifier().clone(),
                            transactions,
                        }
                    })
                    .collect::<Vec<_>>(),
                chainhook: StacksChainhookPayload {
                    uuid: trigger.chainhook.uuid.clone(),
                },
            },
        )),
    }
}
