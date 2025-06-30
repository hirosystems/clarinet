use std::sync::mpsc::Sender;

use chainhook_sdk::{
    observer::MempoolAdmissionData,
    types::{BitcoinChainEvent, StacksChainEvent},
};
use hiro_system_kit::slog;

use crate::{
    chains_coordinator::BitcoinMiningCommand,
    log::{LogData, LogLevel},
};

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
    Terminate,
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

pub fn send_status_update(
    event_tx: &Sender<DevnetEvent>,
    with_subnets: bool,
    logger: &Option<slog::Logger>,
    name: &str,
    status: Status,
    comment: &str,
) {
    let subnet_services = if with_subnets { 2 } else { 0 };

    let order = match name {
        "bitcoin-node" => 0,
        "stacks-node" => 1,
        "stacks-signers" => 2,
        "stacks-api" => 3,
        "subnet-node" => 4,
        "subnet-api" => 5,
        "stacks-explorer" => subnet_services + 4,
        "bitcoin-explorer" => subnet_services + 5,
        _ => return,
    };

    match logger {
        None => {
            let _ = event_tx.send(DevnetEvent::ServiceStatus(ServiceStatusData {
                order,
                status,
                name: name.into(),
                comment: comment.into(),
            }));
        }
        Some(logger) => {
            let msg = format!("{name} - {comment}");
            match status {
                Status::Green => slog::info!(logger, "{}", &msg),
                Status::Yellow => slog::warn!(logger, "{}", &msg),
                Status::Red => slog::error!(logger, "{}", &msg),
            }
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
