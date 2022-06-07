use std::str::FromStr;
use std::{fs::DirEntry, path::PathBuf};

pub mod types;
use crate::chainhooks::types::ChainhookSpecificationFile;
use base58::FromBase58;
use bitcoincore_rpc::bitcoin::blockdata::opcodes;
use bitcoincore_rpc::bitcoin::blockdata::script::Builder as BitcoinScriptBuilder;
use bitcoincore_rpc::bitcoin::{Address, PubkeyHash, PublicKey, Script, TxIn};
use clarity_repl::clarity::util::hash::bytes_to_hex;
use clarity_repl::clarity::util::hash::Hash160;
use orchestra_event_observer::chainhooks::types::{
    BitcoinChainhookSpecification, BitcoinHookPredicate, ChainhookSpecification, HookAction,
    HookFormation, MatchingRule, StacksChainhookSpecification,
};
use orchestra_types::{
    BitcoinChainEvent, BitcoinTransactionData, BlockIdentifier, StacksChainEvent,
    StacksTransactionData,
};
use orchestra_types::{BitcoinNetwork, StacksNetwork};
use reqwest::Client;
use reqwest::Method;
use std::fs;

pub fn load_chainhooks(
    manifest_path: &PathBuf,
    networks: &(BitcoinNetwork, StacksNetwork),
) -> Result<HookFormation, String> {
    let hook_files = get_chainhooks_files(manifest_path)?;
    let mut stacks_chainhooks = vec![];
    let mut bitcoin_chainhooks = vec![];
    for (path, relative_path) in hook_files.into_iter() {
        match ChainhookSpecificationFile::parse(&path, networks) {
            Ok(hook) => match hook {
                ChainhookSpecification::Bitcoin(hook) => bitcoin_chainhooks.push(hook),
                ChainhookSpecification::Stacks(hook) => stacks_chainhooks.push(hook),
            },
            Err(msg) => return Err(format!("{} syntax incorrect: {}", relative_path, msg)),
        };
    }
    Ok(HookFormation {
        stacks_chainhooks,
        bitcoin_chainhooks,
    })
}

pub fn check_chainhooks(manifest_path: &PathBuf, output_json: bool) -> Result<(), String> {
    let hook_files = get_chainhooks_files(manifest_path)?;
    for (path, relative_path) in hook_files.into_iter() {
        let _hook = match ChainhookSpecificationFile::parse(
            &path,
            &(BitcoinNetwork::Regtest, StacksNetwork::Devnet),
        ) {
            Ok(hook) => hook,
            Err(msg) => {
                println!("{} {} syntax incorrect\n{}", red!("x"), relative_path, msg);
                continue;
            }
        };
        println!("{} {} succesfully checked", green!("✔"), relative_path);
        if output_json {
            let body = serde_json::to_string_pretty(&_hook).unwrap();
            println!("{}", body);
        }
    }
    Ok(())
}

fn get_chainhooks_files(manifest_path: &PathBuf) -> Result<Vec<(PathBuf, String)>, String> {
    let mut hooks_home = manifest_path.clone();
    hooks_home.pop();
    let suffix_len = hooks_home.to_str().unwrap().len() + 1;
    hooks_home.push("chainhooks");
    let paths = match fs::read_dir(&hooks_home) {
        Ok(paths) => paths,
        Err(_) => return Ok(vec![]),
    };
    let mut hook_paths = vec![];
    for path in paths {
        let file = path.unwrap().path();
        let is_extension_valid = file
            .extension()
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
