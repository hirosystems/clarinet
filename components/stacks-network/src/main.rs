use chainhook_sdk::chainhook_types::{BitcoinNetwork, StacksNetwork};
use clarinet_deployments::types::DeploymentSpecification;

use clap::Parser;
use clarinet_files::{DevnetConfigFile, FileLocation, ProjectManifest};
use hiro_system_kit::slog;

use std::path::PathBuf;
use std::sync::mpsc::channel;

use stacks_network::{do_run_chain_coordinator, load_chainhooks};
use stacks_network::{Context, DevnetOrchestrator};

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
        false,
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
    ctx.try_log(|logger| slog::info!(logger, "starting devnet coordinator"));

    let (orchestrator_terminated_tx, _) = channel();
    let res = hiro_system_kit::nestable_block_on(do_run_chain_coordinator(
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
