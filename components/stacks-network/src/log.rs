use std::fmt;

use chrono::{DateTime, Utc};

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
