use std::str::FromStr;
use std::{path::PathBuf, fs::DirEntry};

pub mod types;
use self::types::{HookAction, BitcoinTxInBasedPredicate, BitcoinPredicate, MatchingRule};
use crate::types::{StacksTransactionData, BitcoinTransactionData, StacksNetwork, BitcoinChainEvent, StacksChainEvent, BlockIdentifier};
use bitcoincore_rpc::bitcoin::blockdata::script::{Builder as BitcoinScriptBuilder};
use bitcoincore_rpc::bitcoin::blockdata::opcodes;
use bitcoincore_rpc::bitcoin::{TxIn, Script, PubkeyHash, Address, PublicKey};
use clarity_repl::clarity::util::hash::Hash160;
use reqwest::{Client};
use reqwest::Method;
use base58::FromBase58;
use self::types::{HookSpecification, StacksHookSpecification, BitcoinHookSpecification, HookFormation};
use std::fs;
use clarity_repl::clarity::util::hash::bytes_to_hex;


pub fn load_hooks(manifest_path: &PathBuf, network: &StacksNetwork) -> Result<HookFormation, String> {
    let hook_files = get_hooks_files(manifest_path)?;
    let mut stacks_hooks = vec![];
    let mut bitcoin_hooks = vec![];
    for (path, relative_path) in hook_files.into_iter() {
        let hook = match HookSpecification::from_config_file(&path) {
            Ok(hook) => match hook {
                HookSpecification::Bitcoin(hook) => bitcoin_hooks.push(hook),
                HookSpecification::Stacks(hook) => stacks_hooks.push(hook),
            },
            Err(msg) => {
                return Err(format!("{} syntax incorrect: {}", relative_path, msg))
            }
        };
    }
    Ok(HookFormation {
        stacks_hooks,
        bitcoin_hooks,
    })
}

pub fn check_hooks(manifest_path: &PathBuf) -> Result<(), String> {
    let hook_files = get_hooks_files(manifest_path)?;
    for (path, relative_path) in hook_files.into_iter() {
        let _hook = match HookSpecification::from_config_file(&path) {
            Ok(hook) => hook,
            Err(msg) => {
                println!("{} {} syntax incorrect\n{}", red!("x"), relative_path, msg);
                continue;        
            }
        };
        println!("{} {} succesfully checked", green!("✔"), relative_path);    
    }
    Ok(())
}

fn get_hooks_files(manifest_path: &PathBuf) -> Result<Vec<(PathBuf, String)>, String> {
    let mut hooks_home = manifest_path.clone();
    hooks_home.pop();
    let suffix_len = hooks_home.to_str().unwrap().len() + 1;
    hooks_home.push("hooks");
    let paths = match fs::read_dir(&hooks_home) {
        Ok(paths) => paths,
        Err(_) => return Ok(vec![])
    };
    let mut hook_paths = vec![];
    for path in paths {
        let file = path.unwrap().path();
        let is_extension_valid = file.extension()
            .and_then(|ext| ext.to_str())
            .and_then(|ext| Some(ext == "yml" || ext == "yaml"));

        if let Some(true) = is_extension_valid {
            let relative_path = file.clone();
            let (_, relative_path) = relative_path.to_str().unwrap().split_at(suffix_len);
            hook_paths.push((file, relative_path.to_string()));
        }
    }

    Ok(hook_paths)
}


pub fn evaluate_stacks_hooks_on_chain_event<'a>(chain_event: &'a StacksChainEvent, active_hooks: &'a Vec<StacksHookSpecification>) -> Vec<(&'a StacksHookSpecification, &'a StacksTransactionData, &'a BlockIdentifier)> {
    let mut enabled = vec![];
    match chain_event {
        StacksChainEvent::ChainUpdatedWithBlock(update) => {
            for tx in update.new_block.transactions.iter() {
                for hook in active_hooks.iter() {
                    // enabled.push((hook, tx));
                }
            }
        },
        StacksChainEvent::ChainUpdatedWithMicroblock(update) => {

        },
        StacksChainEvent::ChainUpdatedWithMicroblockReorg(update) => {

        },
        StacksChainEvent::ChainUpdatedWithReorg(update) => {

        }
    }
    enabled
}

pub fn evaluate_bitcoin_hooks_on_chain_event<'a>(chain_event: &'a BitcoinChainEvent, active_hooks: &'a Vec<BitcoinHookSpecification>) -> Vec<(&'a BitcoinHookSpecification, &'a BitcoinTransactionData, &'a BlockIdentifier)> {
    let mut enabled = vec![];
    match chain_event {
        BitcoinChainEvent::ChainUpdatedWithBlock(block) => {
            for hook in active_hooks.iter() {
                for tx in block.transactions.iter() {
                    if hook.evaluate_predicate(&tx) {
                        enabled.push((hook, tx, &block.block_identifier));
                    }
                }
            }
        },
        BitcoinChainEvent::ChainUpdatedWithReorg(old_blocks, new_blocks) => {

        }
    }
    enabled
}

pub async fn handle_bitcoin_hook_action<'a>(hook: &'a BitcoinHookSpecification, tx: &'a BitcoinTransactionData, block_identifier: &'a BlockIdentifier, proof: Option<&String>) {
    match &hook.action {
        HookAction::HttpHook(http) => {
            let client = Client::builder().build().unwrap();
            let host = format!("{}", http.url);
            let method = Method::from_bytes(http.method.as_bytes()).unwrap();
            let payload = json!({
                "transaction": tx,
                "proof": proof,
                "block_identifier": block_identifier,
            });
            let body = serde_json::to_vec(&payload).unwrap();
            let res = client
                .request(method, &host)
                .header("Content-Type", "application/json")
                .body(body)
                .send()
                .await;
        }
    }
}

pub async fn handle_stacks_hook_action<'a>(hook: &'a StacksHookSpecification, tx: &'a StacksTransactionData) {
    match &hook.action {
        HookAction::HttpHook(http) => {
            let client = Client::builder().build().unwrap();
            let host = format!("{}", http.url);
            let method = Method::from_bytes(http.method.as_bytes()).unwrap();
            let body = serde_json::to_vec(&tx).unwrap();
            let res = client
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
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::Hex(MatchingRule::Equals(address))) => {
                false
            }
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::Hex(MatchingRule::StartsWith(address))) => {
                false
            }
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::Hex(MatchingRule::EndsWith(address))) => {
                false
            }
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::P2PKH(MatchingRule::Equals(address))) => {
                false
            }
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::P2PKH(MatchingRule::StartsWith(address))) => {
                false
            }
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::P2PKH(MatchingRule::EndsWith(address))) => {
                false
            }
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::P2SH(MatchingRule::Equals(address))) => {
                false
            }
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::P2SH(MatchingRule::StartsWith(address))) => {
                false
            }
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::P2SH(MatchingRule::EndsWith(address))) => {
                false
            }
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::P2WPKH(MatchingRule::Equals(address))) => {
                false
            }
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::P2WPKH(MatchingRule::StartsWith(address))) => {
                false
            }
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::P2WPKH(MatchingRule::EndsWith(address))) => {
                false
            }
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::P2WSH(MatchingRule::Equals(address))) => {
                false
            }
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::P2WSH(MatchingRule::StartsWith(address))) => {
                false
            }
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::P2WSH(MatchingRule::EndsWith(address))) => {
                false
            }
            types::BitcoinHookPredicate::TxIn(BitcoinPredicate::Script(template)) => {
                false
            }
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::Hex(MatchingRule::Equals(address))) => {
                false
            }
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::Hex(MatchingRule::StartsWith(address))) => {
                false
            }
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::Hex(MatchingRule::EndsWith(address))) => {
                false
            }
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::P2PKH(MatchingRule::Equals(address))) => {
                let pubkey_hash = address.from_base58().expect("Unable to get bytes from btc address");
                let script = BitcoinScriptBuilder::new()
                    .push_opcode(opcodes::all::OP_DUP)
                    .push_opcode(opcodes::all::OP_HASH160)
                    .push_slice(&pubkey_hash[1..21])
                    .push_opcode(opcodes::all::OP_EQUALVERIFY)
                    .push_opcode(opcodes::all::OP_CHECKSIG)
                    .into_script();
                
                for output in tx.metadata.outputs.iter() {
                    if output.script_pubkey == script {
                        return true
                    }
                }
                false
            }
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::P2PKH(MatchingRule::StartsWith(address))) => {
                false
            }
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::P2PKH(MatchingRule::EndsWith(address))) => {
                false
            }
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::P2SH(MatchingRule::Equals(address))) => {
                false
            }
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::P2SH(MatchingRule::StartsWith(address))) => {
                false
            }
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::P2SH(MatchingRule::EndsWith(address))) => {
                false
            }
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::P2WPKH(MatchingRule::Equals(address))) => {
                false
            }
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::P2WPKH(MatchingRule::StartsWith(address))) => {
                false
            }
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::P2WPKH(MatchingRule::EndsWith(address))) => {
                false
            }
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::P2WSH(MatchingRule::Equals(address))) => {
                false
            }
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::P2WSH(MatchingRule::StartsWith(address))) => {
                false
            }
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::P2WSH(MatchingRule::EndsWith(address))) => {
                false
            }
            types::BitcoinHookPredicate::TxOut(BitcoinPredicate::Script(template)) => {

                // let mut hex = vec![];


                false
            }
        }       
    }
}
