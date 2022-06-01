pub mod types;

use self::types::{
    BitcoinChainhookSpecification, BitcoinPredicate, BitcoinTxInBasedPredicate,
    ChainhookSpecification, HookAction, HookFormation, MatchingRule, StacksChainhookSpecification,
    StacksHookPredicate,
};
use base58::FromBase58;
use bitcoincore_rpc::bitcoin::blockdata::opcodes;
use bitcoincore_rpc::bitcoin::blockdata::script::Builder as BitcoinScriptBuilder;
use bitcoincore_rpc::bitcoin::{Address, PubkeyHash, PublicKey, Script, TxIn};
use clarity_repl::clarity::util::hash::Hash160;
use orchestra_types::{
    BitcoinChainEvent, BitcoinTransactionData, BlockIdentifier, StacksChainEvent, StacksNetwork,
    StacksTransactionData, StacksTransactionKind,
};
use reqwest::{Client, Method};
use std::str::FromStr;

pub fn evaluate_stacks_chainhooks_on_chain_event<'a>(
    chain_event: &'a StacksChainEvent,
    active_chainhooks: Vec<&'a StacksChainhookSpecification>,
) -> Vec<(
    &'a StacksChainhookSpecification,
    &'a StacksTransactionData,
    &'a BlockIdentifier,
)> {
    let mut enabled = vec![];
    match chain_event {
        StacksChainEvent::ChainUpdatedWithBlock(update) => {
            for tx in update.new_block.transactions.iter() {
                for hook in active_chainhooks.iter() {
                    if let StacksTransactionKind::ContractCall(actual_contract_call) =
                        &tx.metadata.kind
                    {
                        match &hook.predicate {
                            StacksHookPredicate::ContractCall(expected_contract_call) => {
                                if actual_contract_call.contract_identifier
                                    == expected_contract_call.contract_identifier
                                    && actual_contract_call.method == expected_contract_call.method
                                {
                                    enabled.push((
                                        hook.clone(),
                                        tx,
                                        &update.new_block.block_identifier,
                                    ));
                                    continue;
                                }
                            }
                            StacksHookPredicate::PrintEvent(event) => unimplemented!(),
                            StacksHookPredicate::StxEvent(event) => unimplemented!(),
                            StacksHookPredicate::NftEvent(event) => unimplemented!(),
                            StacksHookPredicate::FtEvent(event) => unimplemented!(),
                        }
                    }
                }
            }
        }
        StacksChainEvent::ChainUpdatedWithMicroblock(update) => {}
        StacksChainEvent::ChainUpdatedWithMicroblockReorg(update) => {}
        StacksChainEvent::ChainUpdatedWithReorg(update) => {}
    }
    enabled
}

pub fn evaluate_bitcoin_chainhooks_on_chain_event<'a>(
    chain_event: &'a BitcoinChainEvent,
    active_chainhooks: Vec<&'a BitcoinChainhookSpecification>,
) -> Vec<(
    &'a BitcoinChainhookSpecification,
    &'a BitcoinTransactionData,
    &'a BlockIdentifier,
)> {
    let mut enabled = vec![];
    match chain_event {
        BitcoinChainEvent::ChainUpdatedWithBlock(block) => {
            for hook in active_chainhooks.into_iter() {
                for tx in block.transactions.iter() {
                    if hook.evaluate_predicate(&tx) {
                        enabled.push((hook, tx, &block.block_identifier));
                    }
                }
            }
        }
        BitcoinChainEvent::ChainUpdatedWithReorg(old_blocks, new_blocks) => {}
    }
    enabled
}

pub async fn handle_bitcoin_hook_action<'a>(
    hook: &'a BitcoinChainhookSpecification,
    tx: &'a BitcoinTransactionData,
    block_identifier: &'a BlockIdentifier,
    proof: Option<&String>,
) {
    match &hook.action {
        HookAction::HttpHook(http) => {
            let client = Client::builder().build().unwrap();
            let host = format!("{}", http.url);
            let method = Method::from_bytes(http.method.as_bytes()).unwrap();
            let payload = json!({
                "apply": vec![json!({
                    "transaction": tx,
                    "proof": proof,
                    "block_identifier": block_identifier,
                    "confirmations": 1,
                })],
                "hook_uuid": hook.uuid,
            });
            let body = serde_json::to_vec(&payload).unwrap();
            let _ = client
                .request(method, &host)
                .header("Content-Type", "application/json")
                .header("Authorization", http.authorization_header.clone())
                .body(body)
                .send()
                .await;
        }
    }
}

pub async fn handle_stacks_hook_action<'a>(
    hook: &'a StacksChainhookSpecification,
    tx: &'a StacksTransactionData,
    block_identifier: &'a BlockIdentifier,
    proof: Option<&String>,
) {
    match &hook.action {
        HookAction::HttpHook(http) => {
            let client = Client::builder().build().unwrap();
            let host = format!("{}", http.url);
            let method = Method::from_bytes(http.method.as_bytes()).unwrap();
            let payload = json!({
                "apply": vec![json!({
                    "transaction": tx,
                    "proof": proof,
                    "block_identifier": block_identifier,
                    "confirmations": 1,
                })],
                "hook_uuid": hook.uuid,
            });
            let body = serde_json::to_vec(&payload).unwrap();
            let _ = client
                .request(method, &host)
                .header("Content-Type", "application/json")
                .body(body)
                .send()
                .await;
        }
    }
}

impl BitcoinChainhookSpecification {
    pub fn evaluate_predicate(&self, tx: &BitcoinTransactionData) -> bool {
        match &self.predicate {
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::Hex(MatchingRule::Equals(
                _address,
            ))) => false,
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::Hex(MatchingRule::StartsWith(
                _address,
            ))) => false,
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::Hex(MatchingRule::EndsWith(
                _address,
            ))) => false,
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::P2pkh(MatchingRule::Equals(
                _address,
            ))) => false,
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::P2pkh(
                MatchingRule::StartsWith(_address),
            )) => false,
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::P2pkh(MatchingRule::EndsWith(
                _address,
            ))) => false,
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::P2sh(MatchingRule::Equals(
                _address,
            ))) => false,
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::P2sh(
                MatchingRule::StartsWith(_address),
            )) => false,
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::P2sh(MatchingRule::EndsWith(
                _address,
            ))) => false,
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::P2wpkh(MatchingRule::Equals(
                _address,
            ))) => false,
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::P2wpkh(
                MatchingRule::StartsWith(_address),
            )) => false,
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::P2wpkh(
                MatchingRule::EndsWith(_address),
            )) => false,
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::P2wsh(MatchingRule::Equals(
                _address,
            ))) => false,
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::P2wsh(
                MatchingRule::StartsWith(_address),
            )) => false,
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::P2wsh(MatchingRule::EndsWith(
                _address,
            ))) => false,
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::Script(template)) => false,
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::Hex(MatchingRule::Equals(
                _address,
            ))) => false,
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::Hex(
                MatchingRule::StartsWith(_address),
            )) => false,
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::Hex(MatchingRule::EndsWith(
                _address,
            ))) => false,
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::P2pkh(MatchingRule::Equals(
                address,
            ))) => {
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
                    if output.script_pubkey == script {
                        return true;
                    }
                }
                false
            }
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::P2pkh(
                MatchingRule::StartsWith(_address),
            )) => false,
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::P2pkh(
                MatchingRule::EndsWith(_address),
            )) => false,
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::P2sh(MatchingRule::Equals(
                _address,
            ))) => false,
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::P2sh(
                MatchingRule::StartsWith(_address),
            )) => false,
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::P2sh(MatchingRule::EndsWith(
                _address,
            ))) => false,
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::P2wpkh(MatchingRule::Equals(
                _address,
            ))) => false,
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::P2wpkh(
                MatchingRule::StartsWith(_address),
            )) => false,
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::P2wpkh(
                MatchingRule::EndsWith(_address),
            )) => false,
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::P2wsh(MatchingRule::Equals(
                _address,
            ))) => false,
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::P2wsh(
                MatchingRule::StartsWith(_address),
            )) => false,
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::P2wsh(
                MatchingRule::EndsWith(_address),
            )) => false,
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::Script(template)) => {
                // let mut hex = vec![];

                false
            }
        }
    }
}
