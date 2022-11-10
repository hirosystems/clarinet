extern crate serde;

#[macro_use]
extern crate serde_derive;

pub mod chains_coordinator;
mod orchestrator;
mod ui;

use std::{
    sync::{
        mpsc::{self, channel, Sender},
        Arc,
    },
    thread::sleep,
    time::Duration,
};

use chainhook_event_observer::{chainhooks::types::HookFormation, observer::MempoolAdmissionData};
use chrono::prelude::*;
use tracing::{self, debug, error, info, warn};
use tracing_appender;

use chainhook_types::{BitcoinChainEvent, StacksChainEvent};
use chains_coordinator::start_chains_coordinator;
use clarinet_deployments::types::DeploymentSpecification;
use hiro_system_kit;
pub use orchestrator::DevnetOrchestrator;
use std::sync::atomic::{AtomicBool, Ordering};

use self::chains_coordinator::DevnetEventObserverConfig;

#[allow(dead_code)]
#[derive(Debug)]
pub enum ChainsCoordinatorCommand {
    Terminate,
    ProtocolDeployed,
}

pub fn block_on<F, R>(future: F) -> R
where
    F: std::future::Future<Output = R>,
{
    let rt = hiro_system_kit::create_basic_runtime();
    rt.block_on(future)
}

pub async fn do_run_devnet(
    mut devnet: DevnetOrchestrator,
    deployment: DeploymentSpecification,
    chainhooks: &mut Option<HookFormation>,
    log_tx: Option<Sender<LogData>>,
    display_dashboard: bool,
) -> Result<
    (
        Option<mpsc::Receiver<DevnetEvent>>,
        Option<mpsc::Sender<bool>>,
        Option<mpsc::Sender<ChainsCoordinatorCommand>>,
    ),
    String,
> {
    let (devnet_events_tx, devnet_events_rx) = channel();
    let (termination_success_tx, orchestrator_terminated_rx) = channel();

    devnet.termination_success_tx = Some(termination_success_tx);

    let devnet_config = match devnet.network_config {
        Some(ref network_config) => match &network_config.devnet {
            Some(devnet_config) => Ok(devnet_config.clone()),
            _ => Err("Unable to retrieve config"),
        },
        _ => Err("Unable to retrieve config"),
    }?;

    let file_appender = tracing_appender::rolling::never(&devnet_config.working_dir, "devnet.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_writer(non_blocking)
        .init();

    // The event observer should be able to send some events to the UI thread,
    // and should be able to be terminated
    let hooks = match chainhooks.take() {
        Some(hooks) => hooks,
        _ => HookFormation::new(),
    };
    let devnet_path = devnet_config.working_dir.clone();
    let config = DevnetEventObserverConfig::new(
        devnet_config.clone(),
        devnet.manifest.clone(),
        deployment,
        hooks,
    );
    let chains_coordinator_tx = devnet_events_tx.clone();
    let (chains_coordinator_commands_tx, chains_coordinator_commands_rx) = channel();
    let (orchestrator_terminator_tx, terminator_rx) = channel();
    let (observer_command_tx, observer_command_rx) = channel();
    let moved_orchestrator_terminator_tx = orchestrator_terminator_tx.clone();
    let moved_chains_coordinator_commands_tx = chains_coordinator_commands_tx.clone();
    let moved_observer_command_tx = observer_command_tx.clone();

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
            );
            let rt = hiro_system_kit::create_basic_runtime();
            rt.block_on(future)
        })
        .expect("unable to retrieve join handle");

    // Let's start the orchestration

    // The devnet orchestrator should be able to send some events to the UI thread,
    // and should be able to be restarted/terminated
    let orchestrator_event_tx = devnet_events_tx.clone();
    let orchestrator_handle = hiro_system_kit::thread_named("Devnet orchestrator")
        .spawn(move || {
            let future = devnet.start(orchestrator_event_tx, terminator_rx);
            let rt = hiro_system_kit::create_basic_runtime();
            rt.block_on(future)
        })
        .expect("unable to retrieve join handle");

    if display_dashboard {
        info!("Starting Devnet");
        let moved_chains_coordinator_commands_tx = chains_coordinator_commands_tx.clone();
        let _ = ui::start_ui(
            devnet_events_tx,
            devnet_events_rx,
            moved_chains_coordinator_commands_tx,
            moved_observer_command_tx,
            orchestrator_terminated_rx,
            &devnet_path,
            devnet_config.enable_subnet_node,
        );

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
        ctrlc::set_handler(move || {
            moved_events_observer_commands_tx
                .send(ChainsCoordinatorCommand::Terminate)
                .expect("Unable to terminate devnet");
            moved_orchestrator_terminator_tx
                .send(true)
                .expect("Unable to terminate devnet");
            termination_writer.store(true, Ordering::SeqCst);
        })
        .expect("Error setting Ctrl-C handler");

        if log_tx.is_none() {
            loop {
                match devnet_events_rx.recv() {
                    Ok(DevnetEvent::Log(log)) => {
                        if let Some(ref log_tx) = log_tx {
                            let _ = log_tx.send(log.clone());
                        } else {
                            println!("{}", log.message);
                            match log.level {
                                LogLevel::Debug => debug!("{}", log.message),
                                LogLevel::Info | LogLevel::Success => info!("{}", log.message),
                                LogLevel::Warning => warn!("{}", log.message),
                                LogLevel::Error => error!("{}", log.message),
                            }
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

#[allow(dead_code)]
#[derive(Debug)]
pub enum DevnetEvent {
    Log(LogData),
    KeyEvent(crossterm::event::KeyEvent),
    Tick,
    ServiceStatus(ServiceStatusData),
    ProtocolDeployingProgress(ProtocolDeployingData),
    ProtocolDeployed,
    StacksChainEvent(StacksChainEvent),
    BitcoinChainEvent(BitcoinChainEvent),
    MempoolAdmission(MempoolAdmissionData),
    FatalError(String),
    // Restart,
    // Terminate,
    // Microblock(MicroblockData),
}

#[allow(dead_code)]
impl DevnetEvent {
    pub fn error(message: String) -> DevnetEvent {
        DevnetEvent::Log(Self::log_error(message))
    }

    #[allow(dead_code)]
    pub fn warning(message: String) -> DevnetEvent {
        DevnetEvent::Log(Self::log_warning(message))
    }

    pub fn info(message: String) -> DevnetEvent {
        DevnetEvent::Log(Self::log_info(message))
    }

    pub fn success(message: String) -> DevnetEvent {
        DevnetEvent::Log(Self::log_success(message))
    }

    pub fn debug(message: String) -> DevnetEvent {
        DevnetEvent::Log(Self::log_debug(message))
    }

    pub fn log_error(message: String) -> LogData {
        LogData::new(LogLevel::Error, message)
    }

    pub fn log_warning(message: String) -> LogData {
        LogData::new(LogLevel::Warning, message)
    }

    pub fn log_info(message: String) -> LogData {
        LogData::new(LogLevel::Info, message)
    }

    pub fn log_success(message: String) -> LogData {
        LogData::new(LogLevel::Success, message)
    }

    pub fn log_debug(message: String) -> LogData {
        LogData::new(LogLevel::Debug, message)
    }
}

#[derive(Clone, Debug)]
pub enum LogLevel {
    Error,
    Warning,
    Info,
    Success,
    Debug,
}

#[derive(Clone, Debug)]
pub struct LogData {
    pub occurred_at: String,
    pub message: String,
    pub level: LogLevel,
}

impl LogData {
    pub fn new(level: LogLevel, message: String) -> LogData {
        let now: DateTime<Utc> = Utc::now();
        LogData {
            level,
            message,
            occurred_at: now.format("%b %e %T%.6f").to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Status {
    Red,
    Yellow,
    Green,
}

#[derive(Clone, Debug)]
pub struct ServiceStatusData {
    pub order: usize,
    pub status: Status,
    pub name: String,
    pub comment: String,
}

#[derive(Clone, Debug)]
pub struct ProtocolDeployingData {
    pub new_contracts_deployed: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ProtocolDeployedData {
    pub contracts_deployed: Vec<String>,
}

// pub struct MicroblockData {
//     pub seq: u32,
//     pub transactions: Vec<Transaction>
// }
