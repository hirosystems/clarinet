use clarinet_files::FileLocation;
use std::path::PathBuf;

pub mod types;
use crate::chainhooks::types::ChainhookSpecificationFile;

use chainhook_event_observer::chainhooks::types::{ChainhookSpecification, HookFormation};

use chainhook_types::{BitcoinNetwork, StacksNetwork};

use std::fs;

pub fn load_chainhooks(
    manifest_location: &FileLocation,
    networks: &(BitcoinNetwork, StacksNetwork),
) -> Result<HookFormation, String> {
    let hook_files = get_chainhooks_files(manifest_location)?;
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

pub fn check_chainhooks(manifest_location: &FileLocation, output_json: bool) -> Result<(), String> {
    let hook_files = get_chainhooks_files(manifest_location)?;
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

fn get_chainhooks_files(
    manifest_location: &FileLocation,
) -> Result<Vec<(PathBuf, String)>, String> {
    let mut chainhooks_dir = manifest_location.get_project_root_location()?;
    chainhooks_dir.append_path("chainhooks")?;
    let prefix_len = chainhooks_dir.to_string().len() + 1;
    let paths = match fs::read_dir(&chainhooks_dir.to_string()) {
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
            let (_, relative_path) = relative_path.to_str().unwrap().split_at(prefix_len);
            hook_paths.push((file, relative_path.to_string()));
        }
    }

    Ok(hook_paths)
}
