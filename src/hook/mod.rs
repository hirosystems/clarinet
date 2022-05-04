use std::str::FromStr;
use std::{path::PathBuf, fs::DirEntry};

pub mod types;
use crate::hook::types::HookSpecificationFile;
use orchestra_event_observer::hooks::types::{HookAction, BitcoinPredicate, MatchingRule, HookSpecification, StacksHookSpecification, BitcoinHookSpecification, HookFormation};
use crate::types::{StacksTransactionData, BitcoinTransactionData, StacksNetwork, BitcoinChainEvent, StacksChainEvent, BlockIdentifier};
use bitcoincore_rpc::bitcoin::blockdata::script::{Builder as BitcoinScriptBuilder};
use bitcoincore_rpc::bitcoin::blockdata::opcodes;
use bitcoincore_rpc::bitcoin::{TxIn, Script, PubkeyHash, Address, PublicKey};
use clarity_repl::clarity::util::hash::Hash160;
use reqwest::{Client};
use reqwest::Method;
use base58::FromBase58;
use std::fs;
use clarity_repl::clarity::util::hash::bytes_to_hex;


pub fn load_hooks(manifest_path: &PathBuf, network: &StacksNetwork) -> Result<HookFormation, String> {
    let hook_files = get_hooks_files(manifest_path)?;
    let mut stacks_hooks = vec![];
    let mut bitcoin_hooks = vec![];
    for (path, relative_path) in hook_files.into_iter() {
        let hook = match HookSpecificationFile::parse(&path) {
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
        let _hook = match HookSpecificationFile::parse(&path) {
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
