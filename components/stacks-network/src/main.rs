use std::fs::{self};
use std::io::BufReader;
use std::{fs::File, path::PathBuf};

use chainhook_event_observer::chainhook_types::{BitcoinNetwork, StacksNetwork};
use chainhook_event_observer::chainhooks::types::{ChainhookConfig, ChainhookFullSpecification};
use chainhook_event_observer::{self, utils::Context};
use clarinet_deployments::types::DeploymentSpecification;

use clap::Parser;
use clarinet_files::{DevnetConfigFile, FileLocation, ProjectManifest};
use hiro_system_kit::slog;

use std::sync::mpsc::channel;

use stacks_network::{do_run_devnet, DevnetOrchestrator};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Devnet namespace
    #[clap(short, long)]
    namespace: String,
    /// Path of the project manifest to load
    #[clap(short, long)]
    manifest_path: Option<String>,
    /// Path of the deployment plan
    #[clap(short, long)]
    deployment_plan_path: Option<String>,
    /// Path of the project's root
    #[clap(short, long)]
    project_root_path: Option<String>,
}

fn main() {
    let args = Args::parse();
    let manifest_location = get_config_location_from_path_or_exit(&args.manifest_path);
    let deployment_location = get_config_location_from_path_or_exit(&args.deployment_plan_path);
    let project_location = get_config_location_from_path_or_exit(&args.project_root_path);

    let manifest = ProjectManifest::from_location(&manifest_location).unwrap();
    let orchestrator = DevnetOrchestrator::new(
        manifest,
        Some(DevnetConfigFile {
            working_dir: Some("./".into()),
            ..Default::default()
        }),
    )
    .unwrap();

    let deployment =
        DeploymentSpecification::from_config_file(&deployment_location, &project_location).unwrap();

    let chainhooks = match load_chainhooks(
        &manifest_location,
        &(BitcoinNetwork::Regtest, StacksNetwork::Devnet),
    ) {
        Ok(hooks) => hooks,
        Err(e) => {
            panic!("failed to load chainhooks {}", e);
        }
    };

    let logger = hiro_system_kit::log::setup_logger();
    let _guard = hiro_system_kit::log::setup_global_logger(logger.clone());
    let ctx = Context {
        logger: Some(logger),
        tracer: false,
    };
    ctx.try_log(|logger| slog::info!(logger, "startin devnet coordinator"));

    let (orchestrator_terminated_tx, _) = channel();
    let res = hiro_system_kit::nestable_block_on(do_run_devnet(
        orchestrator,
        deployment,
        &mut Some(chainhooks),
        None,
        ctx,
        orchestrator_terminated_tx,
        &args.namespace,
    ));
    println!("{:?}", res.unwrap());
}

fn get_config_location_from_path_or_exit(path: &Option<String>) -> FileLocation {
    if let Some(path) = path {
        let path_buf = PathBuf::from(path);
        if !path_buf.exists() {
            std::process::exit(1);
        }
        FileLocation::from_path(path_buf)
    } else {
        std::process::exit(1);
    }
}

pub fn load_chainhooks(
    manifest_location: &FileLocation,
    networks: &(BitcoinNetwork, StacksNetwork),
) -> Result<ChainhookConfig, String> {
    let hook_files = get_chainhooks_files(manifest_location)?;
    let mut stacks_chainhooks = vec![];
    let mut bitcoin_chainhooks = vec![];
    for (path, relative_path) in hook_files.into_iter() {
        match parse_chainhook_full_specification(&path) {
            Ok(hook) => {
                match hook {
                    ChainhookFullSpecification::Bitcoin(hook) => bitcoin_chainhooks
                        .push(hook.into_selected_network_specification(&networks.0)?),
                    ChainhookFullSpecification::Stacks(hook) => stacks_chainhooks
                        .push(hook.into_selected_network_specification(&networks.1)?),
                }
            }
            Err(msg) => return Err(format!("{} syntax incorrect: {}", relative_path, msg)),
        };
    }
    Ok(ChainhookConfig {
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
            .and_then(|ext| Some(ext == "json"));

        if let Some(true) = is_extension_valid {
            let relative_path = file.clone();
            let (_, relative_path) = relative_path.to_str().unwrap().split_at(prefix_len);
            hook_paths.push((file, relative_path.to_string()));
        }
    }

    Ok(hook_paths)
}

pub fn parse_chainhook_full_specification(
    path: &PathBuf,
) -> Result<ChainhookFullSpecification, String> {
    let path = match File::open(path) {
        Ok(path) => path,
        Err(_e) => {
            return Err(format!("unable to locate {}", path.display()));
        }
    };

    let mut hook_spec_file_reader = BufReader::new(path);
    let specification: ChainhookFullSpecification =
        serde_json::from_reader(&mut hook_spec_file_reader)
            .map_err(|e| format!("unable to parse chainhook spec: {}", e.to_string()))?;

    Ok(specification)
}
