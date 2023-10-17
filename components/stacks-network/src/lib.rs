extern crate serde;

#[macro_use]
extern crate serde_derive;

mod chainhooks;
pub mod chains_coordinator;
mod event;
mod log;
mod orchestrator;
mod ui;

pub use chainhook_sdk::observer::MempoolAdmissionData;
pub use chainhook_sdk::{self, utils::Context};
pub use chainhooks::{check_chainhooks, load_chainhooks, parse_chainhook_full_specification};
use chains_coordinator::BitcoinMiningCommand;
use clarinet_files::NetworkManifest;
pub use event::DevnetEvent;
pub use log::{LogData, LogLevel};
pub use orchestrator::DevnetOrchestrator;
use orchestrator::ServicesMapHosts;

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, channel, Receiver, Sender},
        Arc,
    },
    thread::sleep,
    time::Duration,
};

use chainhook_sdk::chainhooks::types::ChainhookConfig;
use chains_coordinator::start_chains_coordinator;
use clarinet_deployments::types::DeploymentSpecification;
use hiro_system_kit;
use hiro_system_kit::slog;
use tracing_appender;

use self::chains_coordinator::DevnetEventObserverConfig;
#[allow(dead_code)]
#[derive(Debug)]
pub enum ChainsCoordinatorCommand {
    Terminate,
}

pub fn block_on<F, R>(future: F) -> R
where
    F: std::future::Future<Output = R>,
{
    let rt = hiro_system_kit::create_basic_runtime();
    rt.block_on(future)
}

async fn do_run_devnet(
    mut devnet: DevnetOrchestrator,
    deployment: DeploymentSpecification,
    chainhooks: &mut Option<ChainhookConfig>,
    log_tx: Option<Sender<LogData>>,
    display_dashboard: bool,
    ctx: Context,
    orchestrator_terminated_tx: Sender<bool>,
    orchestrator_terminated_rx: Option<Receiver<bool>>,
    ip_address_setup: ServicesMapHosts,
    start_local_devnet_services: bool,
    network_manifest: Option<NetworkManifest>,
) -> Result<
    (
        Option<mpsc::Receiver<DevnetEvent>>,
        Option<mpsc::Sender<bool>>,
        Option<crossbeam_channel::Sender<ChainsCoordinatorCommand>>,
    ),
    String,
> {
    let (devnet_events_tx, devnet_events_rx) = channel();

    devnet.termination_success_tx = Some(orchestrator_terminated_tx);

    let devnet_config = match devnet.network_config {
        Some(ref network_config) => match &network_config.devnet {
            Some(devnet_config) => Ok(devnet_config.clone()),
            _ => Err("Unable to retrieve config"),
        },
        _ => Err("Unable to retrieve config"),
    }?;
    // if we're starting all services, all trace logs go to networking.log
    if start_local_devnet_services {
        let file_appender =
            tracing_appender::rolling::never(&devnet_config.working_dir, "networking.log");
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .with_writer(non_blocking)
            .try_init();
    } else {
        // for the devnet, we can't write to a file, so we log everything to stdout, but we still want to set
        // the max trace level so we don't get too much information in the logs
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .try_init();
    }

    // The event observer should be able to send some events to the UI thread,
    // and should be able to be terminated
    let hooks = match chainhooks.take() {
        Some(hooks) => hooks,
        _ => ChainhookConfig::new(),
    };
    let devnet_path = devnet_config.working_dir.clone();
    let config = DevnetEventObserverConfig::new(
        devnet_config.clone(),
        devnet.manifest.clone(),
        network_manifest,
        deployment,
        hooks,
        &ctx,
        ip_address_setup,
    );

    let chains_coordinator_tx = devnet_events_tx.clone();
    let (chains_coordinator_commands_tx, chains_coordinator_commands_rx) =
        crossbeam_channel::unbounded();
    let (orchestrator_terminator_tx, terminator_rx) = channel();
    let (observer_command_tx, observer_command_rx) = channel();
    let moved_orchestrator_terminator_tx = orchestrator_terminator_tx.clone();
    let moved_chains_coordinator_commands_tx = chains_coordinator_commands_tx.clone();

    let ctx_moved = ctx.clone();
    let chains_coordinator_handle = hiro_system_kit::thread_named("Chains coordinator")
        .spawn(move || {
            let future = start_chains_coordinator(
                config,
                chains_coordinator_tx,
                chains_coordinator_commands_rx,
                moved_chains_coordinator_commands_tx,
                moved_orchestrator_terminator_tx,
                observer_command_tx,
                observer_command_rx,
                ctx_moved,
            );
            let rt = hiro_system_kit::create_basic_runtime();
            rt.block_on(future)
        })
        .expect("unable to retrieve join handle");

    // Let's start the orchestration

    // The devnet orchestrator should be able to send some events to the UI thread,
    // and should be able to be restarted/terminated
    let orchestrator_event_tx = devnet_events_tx.clone();
    let chains_coordinator_commands_tx_moved = chains_coordinator_commands_tx.clone();
    let ctx_moved = ctx.clone();
    let orchestrator_handle = {
        hiro_system_kit::thread_named("Initializing bitcoin node")
            .spawn(move || {
                let moved_orchestrator_event_tx = orchestrator_event_tx.clone();
                let res = if start_local_devnet_services {
                    let future =
                        devnet.start(moved_orchestrator_event_tx, terminator_rx, &ctx_moved);
                    let rt = hiro_system_kit::create_basic_runtime();
                    rt.block_on(future)
                } else {
                    let future = devnet.initialize_bitcoin_node(&moved_orchestrator_event_tx);
                    let rt = hiro_system_kit::create_basic_runtime();
                    rt.block_on(future)
                };
                if let Err(ref e) = res {
                    let _ = orchestrator_event_tx.send(DevnetEvent::FatalError(e.clone()));
                    let _ = chains_coordinator_commands_tx_moved
                        .send(ChainsCoordinatorCommand::Terminate);
                }
                res
            })
            .expect("unable to retrieve join handle")
    };

    if display_dashboard {
        ctx.try_log(|logger| slog::info!(logger, "Starting Devnet"));
        let moved_chains_coordinator_commands_tx = chains_coordinator_commands_tx.clone();
        let _ = ui::start_ui(
            devnet_events_tx,
            devnet_events_rx,
            moved_chains_coordinator_commands_tx,
            orchestrator_terminated_rx.expect(
                "orchestrator_terminated_rx should be provided when display_dashboard set to true",
            ),
            &devnet_path,
            devnet_config.enable_subnet_node,
            !devnet_config.bitcoin_controller_automining_disabled,
            &ctx,
        )?;

        if let Err(e) = chains_coordinator_handle.join() {
            if let Ok(message) = e.downcast::<String>() {
                return Err(*message);
            }
        }

        if let Err(e) = orchestrator_handle.join() {
            if let Ok(message) = e.downcast::<String>() {
                return Err(*message);
            }
        }
    } else {
        let termination_reader = Arc::new(AtomicBool::new(false));
        let termination_writer = termination_reader.clone();
        let moved_orchestrator_terminator_tx = orchestrator_terminator_tx.clone();
        let moved_events_observer_commands_tx = chains_coordinator_commands_tx.clone();
        let _ = ctrlc::set_handler(move || {
            let _ = moved_events_observer_commands_tx.send(ChainsCoordinatorCommand::Terminate);
            let _ = moved_orchestrator_terminator_tx.send(true);
            termination_writer.store(true, Ordering::SeqCst);
        });

        if log_tx.is_none() {
            loop {
                match devnet_events_rx.recv() {
                    Ok(DevnetEvent::Log(log)) => {
                        if let Some(ref log_tx) = log_tx {
                            let _ = log_tx.send(log.clone());
                        } else {
                            match log.level {
                                LogLevel::Debug => {
                                    ctx.try_log(|logger| slog::debug!(logger, "{}", log.message))
                                }
                                LogLevel::Info | LogLevel::Success => {
                                    ctx.try_log(|logger| slog::info!(logger, "{}", log.message))
                                }
                                LogLevel::Warning => {
                                    ctx.try_log(|logger| slog::warn!(logger, "{}", log.message))
                                }
                                LogLevel::Error => {
                                    ctx.try_log(|logger| slog::error!(logger, "{}", log.message))
                                }
                            }
                        }
                    }
                    Ok(DevnetEvent::BootCompleted(bitcoin_mining_tx)) => {
                        if !devnet_config.bitcoin_controller_automining_disabled {
                            let _ = bitcoin_mining_tx.send(BitcoinMiningCommand::Start);
                        }
                    }
                    _ => {}
                }
                if termination_reader.load(Ordering::SeqCst) {
                    sleep(Duration::from_secs(3));
                    std::process::exit(0);
                }
            }
        } else {
            return Ok((
                Some(devnet_events_rx),
                Some(orchestrator_terminator_tx),
                Some(chains_coordinator_commands_tx),
            ));
        }
    }

    Ok((None, None, Some(chains_coordinator_commands_tx)))
}

pub async fn do_run_chain_coordinator(
    mut devnet: DevnetOrchestrator,
    deployment: DeploymentSpecification,
    chainhooks: &mut Option<ChainhookConfig>,
    log_tx: Option<Sender<LogData>>,
    ctx: Context,
    orchestrator_terminated_tx: Sender<bool>,
    namespace: &str,
    network_manifest: NetworkManifest,
) -> Result<
    (
        Option<mpsc::Receiver<DevnetEvent>>,
        Option<mpsc::Sender<bool>>,
        Option<crossbeam_channel::Sender<ChainsCoordinatorCommand>>,
    ),
    String,
> {
    let ip_address_setup = devnet.prepare_network_k8s_coordinator(namespace)?;
    do_run_devnet(
        devnet,
        deployment,
        chainhooks,
        log_tx,
        false,
        ctx,
        orchestrator_terminated_tx,
        None,
        ip_address_setup,
        false,
        Some(network_manifest),
    )
    .await
}

pub async fn do_run_local_devnet(
    mut devnet: DevnetOrchestrator,
    deployment: DeploymentSpecification,
    chainhooks: &mut Option<ChainhookConfig>,
    log_tx: Option<Sender<LogData>>,
    display_dashboard: bool,
    ctx: Context,
    orchestrator_terminated_tx: Sender<bool>,
    orchestrator_terminated_rx: Option<Receiver<bool>>,
) -> Result<
    (
        Option<mpsc::Receiver<DevnetEvent>>,
        Option<mpsc::Sender<bool>>,
        Option<crossbeam_channel::Sender<ChainsCoordinatorCommand>>,
    ),
    String,
> {
    let ip_address_setup = devnet.prepare_local_network().await?;
    do_run_devnet(
        devnet,
        deployment,
        chainhooks,
        log_tx,
        display_dashboard,
        ctx,
        orchestrator_terminated_tx,
        orchestrator_terminated_rx,
        ip_address_setup,
        true,
        None,
    )
    .await
}
