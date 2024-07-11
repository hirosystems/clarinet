use chainhook_sdk::chainhooks::types::{ChainhookSpecificationNetworkMap, ChainhookStore};
use chainhook_sdk::types::{BitcoinNetwork, StacksNetwork};
use clarinet_files::FileLocation;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use std::fs;

pub fn parse_chainhook_full_specification(
    path: &PathBuf,
) -> Result<ChainhookSpecificationNetworkMap, String> {
    let path = match File::open(path) {
        Ok(path) => path,
        Err(_e) => {
            return Err(format!("unable to locate {}", path.display()));
        }
    };

    let mut hook_spec_file_reader = BufReader::new(path);
    let specification: ChainhookSpecificationNetworkMap =
        serde_json::from_reader(&mut hook_spec_file_reader)
            .map_err(|e| format!("unable to parse chainhook spec: {}", e))?;

    Ok(specification)
}

pub fn load_chainhooks(
    manifest_location: &FileLocation,
    networks: &(BitcoinNetwork, StacksNetwork),
) -> Result<ChainhookStore, String> {
    let hook_files = get_chainhooks_files(manifest_location)?;
    let mut stacks_chainhooks = vec![];
    let mut bitcoin_chainhooks = vec![];
    for (path, relative_path) in hook_files.into_iter() {
        match parse_chainhook_full_specification(&path) {
            Ok(hook) => match hook {
                ChainhookSpecificationNetworkMap::Bitcoin(predicate) => {
                    let mut spec = predicate.into_specification_for_network(&networks.0)?;
                    spec.enabled = true;
                    bitcoin_chainhooks.push(spec)
                }
                ChainhookSpecificationNetworkMap::Stacks(predicate) => {
                    let mut spec = predicate.into_specification_for_network(&networks.1)?;
                    spec.enabled = true;
                    stacks_chainhooks.push(spec)
                }
            },
            Err(msg) => return Err(format!("{} syntax incorrect: {}", relative_path, msg)),
        };
    }
    Ok(ChainhookStore {
        stacks_chainhooks,
        bitcoin_chainhooks,
    })
}

fn get_chainhooks_files(
    manifest_location: &FileLocation,
) -> Result<Vec<(PathBuf, String)>, String> {
    let mut chainhooks_dir = manifest_location.get_project_root_location()?;
    chainhooks_dir.append_path("chainhooks")?;
    let prefix_len = chainhooks_dir.to_string().len() + 1;
    let paths = match fs::read_dir(chainhooks_dir.to_string()) {
        Ok(paths) => paths,
        Err(_) => return Ok(vec![]),
    };
    let mut hook_paths = vec![];
    for path in paths {
        let file = path.unwrap().path();
        let is_extension_valid = file
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext == "json");

        if let Some(true) = is_extension_valid {
            let relative_path = file.clone();
            let (_, relative_path) = relative_path.to_str().unwrap().split_at(prefix_len);
            hook_paths.push((file, relative_path.to_string()));
        }
    }

    Ok(hook_paths)
}
