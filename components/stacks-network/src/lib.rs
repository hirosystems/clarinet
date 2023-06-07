extern crate serde;

#[macro_use]
extern crate serde_derive;

pub mod chains_coordinator;
mod orchestrator;

pub use chainhook_event_observer::{self, utils::Context};
pub use orchestrator::DevnetOrchestrator;
use orchestrator::ServicesMapHosts;

use std::{
    fmt,
    sync::{
        mpsc::{self, channel, Sender},
        Arc,
    },
    thread::sleep,
    time::Duration,
};

use chainhook_event_observer::chainhook_types::{BitcoinChainEvent, StacksChainEvent};
use chainhook_event_observer::{
    chainhooks::types::ChainhookConfig, observer::MempoolAdmissionData,
};
use chains_coordinator::{start_chains_coordinator, BitcoinMiningCommand};
use chrono::prelude::*;
use clarinet_deployments::types::DeploymentSpecification;
use hiro_system_kit::slog;
use hiro_system_kit::{self};
use std::sync::atomic::{AtomicBool, Ordering};
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

const BITCOIND_CHAIN_COORDINATOR_SERVICE: &str = "bitcoind-chain-coordinator-service";
const STACKS_NODE_SERVICE: &str = "stacks-node-service";
const STACKS_API_SERVICE: &str = "stacks-api-service";

const BITCOIND_RPC_PORT: &str = "18443";
const STACKS_NODE_RPC_PORT: &str = "20443";
const STACKS_API_PORT: &str = "3999";
const STACKS_API_POSTGRES_PORT: &str = "5432";

pub async fn do_run_devnet(
    mut devnet: DevnetOrchestrator,
    deployment: DeploymentSpecification,
    chainhooks: &mut Option<ChainhookConfig>,
    log_tx: Option<Sender<LogData>>,
    ctx: Context,
    orchestrator_terminated_tx: Sender<bool>,
    namespace: &str,
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

    let file_appender =
        tracing_appender::rolling::never(&devnet_config.working_dir, "networking.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_writer(non_blocking)
        .try_init();

    let ip_address_setup = {
        let hosts = ServicesMapHosts {
            bitcoin_node_host: format!(
                "{BITCOIND_CHAIN_COORDINATOR_SERVICE}.{namespace}.svc.cluster.local:{BITCOIND_RPC_PORT}"
            ),
            stacks_node_host: format!("{STACKS_NODE_SERVICE}.{namespace}.svc.cluster.local:{STACKS_NODE_RPC_PORT}"),
            postgres_host: format!("{STACKS_API_SERVICE}.{namespace}.svc.cluster.local:{STACKS_API_POSTGRES_PORT}"),
            stacks_api_host: format!("{STACKS_API_SERVICE}.{namespace}.svc.cluster.local:{STACKS_API_PORT}"),
            stacks_explorer_host: "localhost".into(), // todo (micaiah)
            bitcoin_explorer_host: "localhost".into(), // todo (micaiah)
            subnet_node_host: "localhost".into(), // todo (micaiah)
            subnet_api_host: "localhost".into(), // todo (micaiah)
        };
        devnet.set_services_map_hosts(hosts.clone());
        hosts
    };

    // The event observer should be able to send some events to the UI thread,
    // and should be able to be terminated
    let hooks = match chainhooks.take() {
        Some(hooks) => hooks,
        _ => ChainhookConfig::new(),
    };
    let config = DevnetEventObserverConfig::new(
        devnet_config.clone(),
        devnet.manifest.clone(),
        deployment,
        hooks,
        &ctx,
        ip_address_setup,
    );
    let chains_coordinator_tx = devnet_events_tx.clone();
    let (chains_coordinator_commands_tx, chains_coordinator_commands_rx) =
        crossbeam_channel::unbounded();
    let (orchestrator_terminator_tx, _) = channel();
    let (observer_command_tx, observer_command_rx) = channel();
    let moved_orchestrator_terminator_tx = orchestrator_terminator_tx.clone();
    let moved_chains_coordinator_commands_tx = chains_coordinator_commands_tx.clone();

    let ctx_moved = ctx.clone();
    let _ = hiro_system_kit::thread_named("Chains coordinator")
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
    let _ = {
        hiro_system_kit::thread_named("Initializing bitcoin node")
            .spawn(move || {
                let moved_orchestrator_event_tx = orchestrator_event_tx.clone();
                let future = devnet.initialize_bitcoin_node(&moved_orchestrator_event_tx);
                let rt = hiro_system_kit::create_basic_runtime();
                let res = rt.block_on(future);
                if let Err(ref e) = res {
                    let _ = orchestrator_event_tx.send(DevnetEvent::FatalError(e.clone()));
                    let _ = chains_coordinator_commands_tx_moved
                        .send(ChainsCoordinatorCommand::Terminate);
                }
                res
            })
            .expect("unable to retrieve join handle")
    };

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

#[allow(dead_code)]
#[derive(Debug)]
pub enum DevnetEvent {
    Log(LogData),
    KeyEvent(crossterm::event::KeyEvent),
    Tick,
    ServiceStatus(ServiceStatusData),
    ProtocolDeployingProgress(ProtocolDeployingData),
    BootCompleted(Sender<BitcoinMiningCommand>),
    StacksChainEvent(StacksChainEvent),
    BitcoinChainEvent(BitcoinChainEvent),
    MempoolAdmission(MempoolAdmissionData),
    FatalError(String),
    // Restart,
    // Terminate,
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

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match &self {
                LogLevel::Error => "erro",
                LogLevel::Warning => "warn",
                LogLevel::Info => "info",
                LogLevel::Success => "succ",
                LogLevel::Debug => "debg",
            }
        )
    }
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

impl fmt::Display for LogData {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} [{}] {}", self.occurred_at, self.level, self.message)
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
pub struct BootCompletedData {
    pub contracts_deployed: Vec<String>,
}

// pub struct MicroblockData {
//     pub seq: u32,
//     pub transactions: Vec<Transaction>
// }
