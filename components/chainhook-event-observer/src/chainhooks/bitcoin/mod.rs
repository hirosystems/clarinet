use super::types::{
    BitcoinChainhookSpecification, BitcoinPredicateType, ExactMatchingRule, HookAction,
    KeyRegistrationPredicate, LockSTXPredicate, MatchingRule, PobPredicate, PoxPredicate,
    TransferSTXPredicate,
};
use base58::FromBase58;
use bitcoincore_rpc::bitcoin::blockdata::opcodes;
use bitcoincore_rpc::bitcoin::blockdata::script::Builder as BitcoinScriptBuilder;
use bitcoincore_rpc::bitcoin::util::address::Payload;
use bitcoincore_rpc::bitcoin::Address;
use chainhook_types::{
    BitcoinBlockData, BitcoinChainEvent, BitcoinTransactionData, StacksBaseChainOperation,
    TransactionIdentifier,
};
use clarity_repl::clarity::util::hash::to_hex;
use reqwest::{Client, Method};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::str::FromStr;

use reqwest::RequestBuilder;

pub struct BitcoinTriggerChainhook<'a> {
    pub chainhook: &'a BitcoinChainhookSpecification,
    pub apply: Vec<(Vec<&'a BitcoinTransactionData>, &'a BitcoinBlockData)>,
    pub rollback: Vec<(Vec<&'a BitcoinTransactionData>, &'a BitcoinBlockData)>,
}

#[derive(Clone, Debug)]
pub struct BitcoinApplyTransactionPayload {
    pub block: BitcoinBlockData,
}

#[derive(Clone, Debug)]
pub struct BitcoinRollbackTransactionPayload {
    pub block: BitcoinBlockData,
}

#[derive(Clone, Debug)]
pub struct BitcoinChainhookPayload {
    pub uuid: String,
}

#[derive(Clone, Debug)]
pub struct BitcoinChainhookOccurrencePayload {
    pub apply: Vec<BitcoinApplyTransactionPayload>,
    pub rollback: Vec<BitcoinRollbackTransactionPayload>,
    pub chainhook: BitcoinChainhookPayload,
}

pub enum BitcoinChainhookOccurrence {
    Http(RequestBuilder),
    File(String, Vec<u8>),
    Data(BitcoinChainhookOccurrencePayload),
}

pub fn evaluate_bitcoin_chainhooks_on_chain_event<'a>(
    chain_event: &'a BitcoinChainEvent,
    active_chainhooks: Vec<&'a BitcoinChainhookSpecification>,
) -> Vec<BitcoinTriggerChainhook<'a>> {
    let mut triggered_chainhooks = vec![];
    match chain_event {
        BitcoinChainEvent::ChainUpdatedWithBlocks(event) => {
            for chainhook in active_chainhooks.iter() {
                let mut apply = vec![];
                let rollback = vec![];

                for block in event.new_blocks.iter() {
                    let mut hits = vec![];
                    for tx in block.transactions.iter() {
                        if chainhook.evaluate_transaction_predicate(&tx) {
                            hits.push(tx);
                        }
                    }
                    if hits.len() > 0 {
                        apply.push((hits, block));
                    }
                }

                if !apply.is_empty() {
                    triggered_chainhooks.push(BitcoinTriggerChainhook {
                        chainhook,
                        apply,
                        rollback,
                    })
                }
            }
        }
        BitcoinChainEvent::ChainUpdatedWithReorg(event) => {
            for chainhook in active_chainhooks.iter() {
                let mut apply = vec![];
                let mut rollback = vec![];

                for block in event.blocks_to_apply.iter() {
                    let mut hits = vec![];
                    for tx in block.transactions.iter() {
                        if chainhook.evaluate_transaction_predicate(&tx) {
                            hits.push(tx);
                        }
                    }
                    if hits.len() > 0 {
                        apply.push((hits, block));
                    }
                }
                for block in event.blocks_to_rollback.iter() {
                    let mut hits = vec![];
                    for tx in block.transactions.iter() {
                        if chainhook.evaluate_transaction_predicate(&tx) {
                            hits.push(tx);
                        }
                    }
                    if hits.len() > 0 {
                        rollback.push((hits, block));
                    }
                }
                if !apply.is_empty() || !rollback.is_empty() {
                    triggered_chainhooks.push(BitcoinTriggerChainhook {
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

pub fn serialize_bitcoin_payload_to_json<'a>(
    trigger: BitcoinTriggerChainhook<'a>,
    proofs: &HashMap<&'a TransactionIdentifier, String>,
) -> JsonValue {
    json!({
        "apply": trigger.apply.into_iter().map(|(transactions, block)| {
            json!({
                "block_identifier": block.block_identifier,
                "parent_block_identifier": block.parent_block_identifier,
                "timestamp": block.timestamp,
                "transactions": transactions.into_iter().map(|transaction| {
                    json!({
                        "transaction_identifier": transaction.transaction_identifier,
                        "operations": transaction.operations,
                        "metadata": json!({
                            "inputs": transaction.metadata.inputs,
                            "outputs": transaction.metadata.outputs,
                            "stacks_operations": transaction.metadata.stacks_operations,
                            "proof": proofs.get(&transaction.transaction_identifier),
                        }),
                    })
                }).collect::<Vec<_>>(),
                "metadata": block.metadata,
            })
        }).collect::<Vec<_>>(),
        "rollback": trigger.rollback.into_iter().map(|(transactions, block)| {
            json!({
                "block_identifier": block.block_identifier,
                "parent_block_identifier": block.parent_block_identifier,
                "timestamp": block.timestamp,
                "transactions": transactions,
                "metadata": block.metadata,
            })
        }).collect::<Vec<_>>(),
        "chainhook": {
            "uuid": trigger.chainhook.uuid,
            "predicate": trigger.chainhook.predicate,
        }
    })
}

pub fn handle_bitcoin_hook_action<'a>(
    trigger: BitcoinTriggerChainhook<'a>,
    proofs: &HashMap<&'a TransactionIdentifier, String>,
) -> Option<BitcoinChainhookOccurrence> {
    match &trigger.chainhook.action {
        HookAction::Http(http) => {
            let client = Client::builder().build().unwrap();
            let host = format!("{}", http.url);
            let method = Method::from_bytes(http.method.as_bytes()).unwrap();
            let body =
                serde_json::to_vec(&serialize_bitcoin_payload_to_json(trigger, proofs)).unwrap();
            Some(BitcoinChainhookOccurrence::Http(
                client
                    .request(method, &host)
                    .header("Content-Type", "application/json")
                    .header("Authorization", http.authorization_header.clone())
                    .body(body),
            ))
        }
        HookAction::File(disk) => {
            let bytes =
                serde_json::to_vec(&serialize_bitcoin_payload_to_json(trigger, proofs)).unwrap();
            Some(BitcoinChainhookOccurrence::File(
                disk.path.to_string(),
                bytes,
            ))
        }
        HookAction::Noop => Some(BitcoinChainhookOccurrence::Data(
            BitcoinChainhookOccurrencePayload {
                apply: trigger
                    .apply
                    .into_iter()
                    .map(|(transactions, block)| {
                        let mut block = block.clone();
                        block.transactions = transactions
                            .into_iter()
                            .map(|t| t.clone())
                            .collect::<Vec<_>>();
                        BitcoinApplyTransactionPayload { block }
                    })
                    .collect::<Vec<_>>(),
                rollback: trigger
                    .rollback
                    .into_iter()
                    .map(|(transactions, block)| {
                        let mut block = block.clone();
                        block.transactions = transactions
                            .into_iter()
                            .map(|t| t.clone())
                            .collect::<Vec<_>>();
                        BitcoinRollbackTransactionPayload { block }
                    })
                    .collect::<Vec<_>>(),
                chainhook: BitcoinChainhookPayload {
                    uuid: trigger.chainhook.uuid.clone(),
                },
            },
        )),
    }
}

impl BitcoinChainhookSpecification {
    pub fn evaluate_transaction_predicate(&self, tx: &BitcoinTransactionData) -> bool {
        // TODO(lgalabru): follow-up on this implementation
        match &self.predicate.kind {
            BitcoinPredicateType::TransactionIdentifierHash(ExactMatchingRule::Equals(txid)) => {
                tx.transaction_identifier.hash.eq(txid)
            }
            BitcoinPredicateType::OpReturn(MatchingRule::Equals(hex_bytes)) => {
                for output in tx.metadata.outputs.iter() {
                    if output.script_pubkey.eq(hex_bytes) {
                        return true;
                    }
                }
                false
            }
            BitcoinPredicateType::OpReturn(MatchingRule::StartsWith(hex_bytes)) => {
                for output in tx.metadata.outputs.iter() {
                    if output.script_pubkey.starts_with(hex_bytes) {
                        return true;
                    }
                }
                false
            }
            BitcoinPredicateType::OpReturn(MatchingRule::EndsWith(hex_bytes)) => {
                for output in tx.metadata.outputs.iter() {
                    if output.script_pubkey.ends_with(hex_bytes) {
                        return true;
                    }
                }
                false
            }
            BitcoinPredicateType::P2pkh(ExactMatchingRule::Equals(address)) => {
                let pubkey_hash = address
                    .from_base58()
                    .expect("Unable to get bytes from btc address");
                let script = BitcoinScriptBuilder::new()
                    .push_opcode(opcodes::all::OP_DUP)
                    .push_opcode(opcodes::all::OP_HASH160)
                    .push_slice(&pubkey_hash[1..21])
                    .push_opcode(opcodes::all::OP_EQUALVERIFY)
                    .push_opcode(opcodes::all::OP_CHECKSIG)
                    .into_script();

                for output in tx.metadata.outputs.iter() {
                    if output.script_pubkey == to_hex(script.as_bytes()) {
                        return true;
                    }
                }
                false
            }
            BitcoinPredicateType::P2sh(ExactMatchingRule::Equals(address)) => {
                let script_hash = address
                    .from_base58()
                    .expect("Unable to get bytes from btc address");
                let script = BitcoinScriptBuilder::new()
                    .push_opcode(opcodes::all::OP_HASH160)
                    .push_slice(&script_hash[1..21])
                    .push_opcode(opcodes::all::OP_EQUAL)
                    .into_script();

                for output in tx.metadata.outputs.iter() {
                    if output.script_pubkey == to_hex(script.as_bytes()) {
                        return true;
                    }
                }
                false
            }
            BitcoinPredicateType::P2wpkh(ExactMatchingRule::Equals(encoded_address))
            | BitcoinPredicateType::P2wsh(ExactMatchingRule::Equals(encoded_address)) => {
                let address = match Address::from_str(encoded_address) {
                    Ok(address) => match address.payload {
                        Payload::WitnessProgram {
                            version: _,
                            program: _,
                        } => address,
                        _ => return false,
                    },
                    Err(_) => return false,
                };
                let script_bytes = to_hex(address.script_pubkey().as_bytes());
                for output in tx.metadata.outputs.iter() {
                    if output.script_pubkey == script_bytes {
                        return true;
                    }
                }
                false
            }
            BitcoinPredicateType::Pob(PobPredicate::Any) => {
                for op in tx.metadata.stacks_operations.iter() {
                    if let StacksBaseChainOperation::PobBlockCommitment(_) = op {
                        return true;
                    }
                }
                false
            }
            BitcoinPredicateType::Pox(PoxPredicate::Any) => {
                for op in tx.metadata.stacks_operations.iter() {
                    if let StacksBaseChainOperation::PoxBlockCommitment(_) = op {
                        return true;
                    }
                }
                false
            }
            BitcoinPredicateType::Pox(PoxPredicate::Recipient(MatchingRule::Equals(address))) => {
                for op in tx.metadata.stacks_operations.iter() {
                    if let StacksBaseChainOperation::PoxBlockCommitment(commitment) = op {
                        for reward in commitment.rewards.iter() {
                            if reward.recipient.eq(address) {
                                return true;
                            }
                        }
                    }
                }
                false
            }
            BitcoinPredicateType::Pox(PoxPredicate::Recipient(MatchingRule::StartsWith(
                prefix,
            ))) => {
                for op in tx.metadata.stacks_operations.iter() {
                    if let StacksBaseChainOperation::PoxBlockCommitment(commitment) = op {
                        for reward in commitment.rewards.iter() {
                            if reward.recipient.starts_with(prefix) {
                                return true;
                            }
                        }
                    }
                }
                false
            }
            BitcoinPredicateType::Pox(PoxPredicate::Recipient(MatchingRule::EndsWith(suffix))) => {
                for op in tx.metadata.stacks_operations.iter() {
                    if let StacksBaseChainOperation::PoxBlockCommitment(commitment) = op {
                        for reward in commitment.rewards.iter() {
                            if reward.recipient.ends_with(suffix) {
                                return true;
                            }
                        }
                    }
                }
                false
            }
            BitcoinPredicateType::KeyRegistration(KeyRegistrationPredicate::Any) => {
                for op in tx.metadata.stacks_operations.iter() {
                    if let StacksBaseChainOperation::KeyRegistration(_) = op {
                        return true;
                    }
                }
                false
            }
            BitcoinPredicateType::TransferSTX(TransferSTXPredicate::Any) => {
                for op in tx.metadata.stacks_operations.iter() {
                    if let StacksBaseChainOperation::TransferSTX(_) = op {
                        return true;
                    }
                }
                false
            }
            BitcoinPredicateType::LockSTX(LockSTXPredicate::Any) => {
                for op in tx.metadata.stacks_operations.iter() {
                    if let StacksBaseChainOperation::LockSTX(_) = op {
                        return true;
                    }
                }
                false
            }
        }
    }
}
