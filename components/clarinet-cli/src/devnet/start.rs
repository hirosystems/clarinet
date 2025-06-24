use std::{
    fs,
    path::PathBuf,
    str::FromStr,
    sync::mpsc::{self, channel, Sender},
};

use clarinet_deployments::types::DeploymentSpecification;
use hiro_system_kit::Drain;
use hiro_system_kit::{slog, slog_async, slog_term};
use stacks_network::{
    chainhook_sdk::types::{BitcoinNetwork, StacksNetwork},
    chainhook_sdk::utils::Context,
    do_run_local_devnet, load_chainhooks, ChainsCoordinatorCommand, DevnetEvent,
    DevnetOrchestrator, LogData,
};
use std::fs::OpenOptions;

pub fn start(
    devnet: DevnetOrchestrator,
    deployment: DeploymentSpecification,
    log_tx: Option<Sender<LogData>>,
    display_dashboard: bool,
    default_settings: bool,
) -> Result<
    (
        Option<mpsc::Receiver<DevnetEvent>>,
        Option<mpsc::Sender<bool>>,
        Option<crossbeam_channel::Sender<ChainsCoordinatorCommand>>,
    ),
    String,
> {
    let mut hooks = if default_settings {
        None
    } else {
        Some(
            match load_chainhooks(
                &devnet.manifest.location,
                &(BitcoinNetwork::Regtest, StacksNetwork::Devnet),
            ) {
                Ok(hooks) => hooks,
                Err(e) => {
                    println!("{}", e);
                    std::process::exit(1);
                }
            },
        )
    };

    let (logger, working_dir) = if !default_settings {
        let working_dir = devnet
            .network_config
            .as_ref()
            .and_then(|c| c.devnet.as_ref())
            .map(|d| d.working_dir.to_string())
            .ok_or("unable to read settings/Devnet.toml")?;
        fs::create_dir_all(&working_dir)
            .map_err(|_| format!("unable to create dir {}", working_dir))?;
        let mut log_path = PathBuf::from_str(&working_dir)
            .map_err(|e| format!("unable to working_dir {}\n{}", working_dir, e))?;
        log_path.push("devnet.log");
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(log_path)
            .map_err(|e| format!("unable to create log file {}", e))?;

        let decorator = slog_term::PlainDecorator::new(file);
        let drain = slog_term::FullFormat::new(decorator).build().fuse();
        let drain = slog_async::Async::new(drain).build().fuse();
        (Some(slog::Logger::root(drain, o!())), Some(working_dir))
    } else {
        (None, None)
    };

    let ctx = Context {
        logger,
        tracer: false,
    };
    let (orchestrator_terminated_tx, orchestrator_terminated_rx) = channel();
    let res = hiro_system_kit::nestable_block_on(do_run_local_devnet(
        devnet,
        deployment,
        &mut hooks,
        log_tx,
        display_dashboard,
        ctx,
        orchestrator_terminated_tx,
        Some(orchestrator_terminated_rx),
    ));
    if !default_settings {
        println!(
            "{} logs and chainstate available at location {}",
            yellow!("\nterminating devnet network:"),
            working_dir.unwrap()
        );
    }

    res
}
