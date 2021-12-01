pub mod events_observer;
mod orchestrator;
mod ui;

use std::sync::mpsc::{self, channel, Sender};

use chrono::prelude::*;
use tracing::{self, debug, error, info, warn};
use tracing_appender;

use crate::types::{BitcoinBlockData, StacksBlockData};
use crate::utils;
use events_observer::start_events_observer;
pub use orchestrator::DevnetOrchestrator;

use self::events_observer::EventObserverConfig;

pub fn run_devnet(
    devnet: DevnetOrchestrator,
    log_tx: Option<Sender<LogData>>,
    display_dashboard: bool,
) -> Result<
    (
        Option<mpsc::Receiver<DevnetEvent>>,
        Option<mpsc::Sender<bool>>,
    ),
    String,
> {
    match block_on(do_run_devnet(devnet, log_tx, display_dashboard)) {
        Err(_e) => std::process::exit(1),
        Ok(res) => Ok(res),
    }
}

pub fn block_on<F, R>(future: F) -> R
where
    F: std::future::Future<Output = R>,
{
    let rt = utils::create_basic_runtime();
    rt.block_on(future)
}

pub async fn do_run_devnet(
    mut devnet: DevnetOrchestrator,
    log_tx: Option<Sender<LogData>>,
    display_dashboard: bool,
) -> Result<
    (
        Option<mpsc::Receiver<DevnetEvent>>,
        Option<mpsc::Sender<bool>>,
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
    let devnet_path = devnet_config.working_dir.clone();
    let config = EventObserverConfig::new(devnet_config, devnet.manifest_path.clone());
    let contracts_to_deploy_len = config.contracts_to_deploy.len();
    let events_observer_tx = devnet_events_tx.clone();
    let (events_observer_terminator_tx, terminator_rx) = channel();
    let events_observer_handle = std::thread::spawn(move || {
        let future = start_events_observer(config, events_observer_tx, terminator_rx);
        let rt = utils::create_basic_runtime();
        let _ = rt.block_on(future);
    });

    // Let's start the orchestration

    // The devnet orchestrator should be able to send some events to the UI thread,
    // and should be able to be restarted/terminated
    let orchestrator_event_tx = devnet_events_tx.clone();
    let (orchestrator_terminator_tx, terminator_rx) = channel();
    let orchestrator_handle = std::thread::spawn(move || {
        let future = devnet.start(
            orchestrator_event_tx,
            terminator_rx,
            contracts_to_deploy_len,
        );
        let rt = utils::create_basic_runtime();
        rt.block_on(future);
    });

    if display_dashboard {
        info!("Starting Devnet...");
        let _ = ui::start_ui(
            devnet_events_tx,
            devnet_events_rx,
            events_observer_terminator_tx,
            orchestrator_terminator_tx,
            orchestrator_terminated_rx,
            &devnet_path,
        );
    } else {
        let moved_orchestrator_terminator_tx = orchestrator_terminator_tx.clone();
        let moved_events_observer_terminator_tx = events_observer_terminator_tx.clone();
        ctrlc::set_handler(move || {
            moved_events_observer_terminator_tx
                .send(true)
                .expect("Unable to terminate devnet");
            moved_orchestrator_terminator_tx
                .send(true)
                .expect("Unable to terminate devnet");
        })
        .expect("Error setting Ctrl-C handler");

        if log_tx.is_none() {
            println!("Starting Devnet...");

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
            }
        } else {
            return Ok((Some(devnet_events_rx), Some(orchestrator_terminator_tx)));
        }
    }

    events_observer_handle.join().unwrap();
    orchestrator_handle.join().unwrap();

    Ok((None, None))
}

#[derive(Debug)]
pub enum DevnetEvent {
    Log(LogData),
    KeyEvent(crossterm::event::KeyEvent),
    Tick,
    ServiceStatus(ServiceStatusData),
    StacksBlock(StacksBlockData),
    BitcoinBlock(BitcoinBlockData),
    MempoolAdmission(MempoolAdmissionData),
    // Restart,
    // Terminate,
    // Microblock(MicroblockData),
}

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

// pub struct MicroblockData {
//     pub seq: u32,
//     pub transactions: Vec<Transaction>
// }

#[derive(Clone, Debug)]
pub struct MempoolAdmissionData {
    pub tx: String,
}
