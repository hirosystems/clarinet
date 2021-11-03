mod events_observer;
mod orchestrator;
mod ui;

use std::sync::mpsc::{channel, Sender};

use chrono::prelude::*;
use tracing::{self, debug, error, info, warn};
use tracing_appender;

use crate::utils;
use events_observer::start_events_observer;
pub use orchestrator::DevnetOrchestrator;

use self::events_observer::EventObserverConfig;

pub enum NodeObserverEvent {
    NewStacksBlock,
    NewBitcoinBlock,
}

pub fn run_devnet(
    devnet: DevnetOrchestrator,
    event_tx: Option<Sender<NodeObserverEvent>>,
    log_tx: Option<Sender<LogData>>,
    display_dashboard: bool,
) {
    match block_on(do_run_devnet(devnet, event_tx, log_tx, display_dashboard)) {
        Err(_e) => std::process::exit(1),
        _ => {}
    };
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
    event_tx: Option<Sender<NodeObserverEvent>>,
    log_tx: Option<Sender<LogData>>,
    display_dashboard: bool,
) -> Result<bool, String> {
    let (devnet_events_tx, devnet_events_rx) = channel();
    let (termination_success_tx, orchestrator_terminated_rx) = channel();

    devnet.termination_success_tx = Some(termination_success_tx);

    let (devnet_config, accounts) = match devnet.network_config {
        Some(ref network_config) => match &network_config.devnet {
            Some(devnet_config) => Ok((devnet_config.clone(), network_config.accounts.clone())),
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
    let config = EventObserverConfig::new(devnet_config, devnet.manifest_path.clone(), accounts);
    let contracts_to_deploy_len = config.contracts_to_deploy.len();
    let events_observer_tx = devnet_events_tx.clone();
    let (events_observer_terminator_tx, terminator_rx) = channel();
    let events_observer_handle = std::thread::spawn(move || {
        let future = start_events_observer(config, events_observer_tx, terminator_rx, event_tx);
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
        if log_tx.is_some() {
            println!("Starting Devnet...");
        }

        ctrlc::set_handler(move || {
            events_observer_terminator_tx
                .send(true)
                .expect("Unable to terminate devnet");
            orchestrator_terminator_tx
                .send(true)
                .expect("Unable to terminate devnet");
        })
        .expect("Error setting Ctrl-C handler");

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
    }

    events_observer_handle.join().unwrap();
    orchestrator_handle.join().unwrap();

    Ok(true)
}

pub enum DevnetEvent {
    Log(LogData),
    KeyEvent(crossterm::event::KeyEvent),
    Tick,
    ServiceStatus(ServiceStatusData),
    Block(BlockData),
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

#[derive(Clone)]
pub enum LogLevel {
    Error,
    Warning,
    Info,
    Success,
    Debug,
}

#[derive(Clone)]
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

pub enum Status {
    Red,
    Yellow,
    Green,
}

pub struct ServiceStatusData {
    pub order: usize,
    pub status: Status,
    pub name: String,
    pub comment: String,
}

#[derive(Clone)]
pub struct Transaction {
    pub txid: String,
    pub success: bool,
    pub result: String,
    pub events: Vec<String>,
    pub description: String,
}

#[derive(Clone)]
pub struct BlockData {
    pub block_height: u64,
    pub block_hash: String,
    pub bitcoin_block_height: u64,
    pub bitcoin_block_hash: String,
    pub first_burnchain_block_height: u64,
    pub pox_cycle_length: u32,
    pub pox_cycle_id: u32,
    pub transactions: Vec<Transaction>,
}

// pub struct MicroblockData {
//     pub seq: u32,
//     pub transactions: Vec<Transaction>
// }

pub struct MempoolAdmissionData {
    pub tx: String,
}
