pub mod types;

use self::types::{
    BitcoinHookSpecification, BitcoinPredicate, BitcoinTxInBasedPredicate, HookAction,
    HookFormation, HookSpecification, MatchingRule, StacksHookSpecification,
};
use base58::FromBase58;
use bitcoincore_rpc::bitcoin::blockdata::opcodes;
use bitcoincore_rpc::bitcoin::blockdata::script::Builder as BitcoinScriptBuilder;
use bitcoincore_rpc::bitcoin::{Address, PubkeyHash, PublicKey, Script, TxIn};
use clarity_repl::clarity::util::hash::Hash160;
use orchestra_types::{
    BitcoinChainEvent, BitcoinTransactionData, BlockIdentifier, StacksChainEvent, StacksNetwork,
    StacksTransactionData,
};
use reqwest::{Client, Method};
use std::str::FromStr;

pub fn evaluate_stacks_hooks_on_chain_event<'a>(
    chain_event: &'a StacksChainEvent,
    active_hooks: Vec<&'a StacksHookSpecification>,
) -> Vec<(
    &'a StacksHookSpecification,
    &'a StacksTransactionData,
    &'a BlockIdentifier,
)> {
    let mut enabled = vec![];
    match chain_event {
        StacksChainEvent::ChainUpdatedWithBlock(update) => {
            for tx in update.new_block.transactions.iter() {
                for hook in active_hooks.iter() {
                    // enabled.push((hook, tx));
                }
            }
        }
        StacksChainEvent::ChainUpdatedWithMicroblock(update) => {}
        StacksChainEvent::ChainUpdatedWithMicroblockReorg(update) => {}
        StacksChainEvent::ChainUpdatedWithReorg(update) => {}
    }
    enabled
}

pub fn evaluate_bitcoin_hooks_on_chain_event<'a>(
    chain_event: &'a BitcoinChainEvent,
    active_hooks: Vec<&'a BitcoinHookSpecification>,
) -> Vec<(
    &'a BitcoinHookSpecification,
    &'a BitcoinTransactionData,
    &'a BlockIdentifier,
)> {
    let mut enabled = vec![];
    match chain_event {
        BitcoinChainEvent::ChainUpdatedWithBlock(block) => {
            for hook in active_hooks.into_iter() {
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
    hook: &'a BitcoinHookSpecification,
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
                })]
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
    hook: &'a StacksHookSpecification,
    tx: &'a StacksTransactionData,
) {
    match &hook.action {
        HookAction::HttpHook(http) => {
            let client = Client::builder().build().unwrap();
            let host = format!("{}", http.url);
            let method = Method::from_bytes(http.method.as_bytes()).unwrap();
            let body = serde_json::to_vec(&tx).unwrap();
            let _ = client
                .request(method, &host)
                .header("Content-Type", "application/json")
                .body(body)
                .send()
                .await;
        }
    }
}

impl BitcoinHookSpecification {
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
