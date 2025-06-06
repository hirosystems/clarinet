//! Simple logger that can log to a file or discard logs

use std::fmt::{self, Display};
use std::fs::{File, OpenOptions};
use std::io::{self, Write as _};

#[derive(Default, PartialEq, PartialOrd)]
pub enum LogLevel {
    Debug,
    Info,
    #[default]
    Warn,
    Error,
}

impl LogLevel {
    const fn as_str(&self) -> &str {
        use LogLevel::*;
        match self {
            Debug => "DEBUG",
            Info => "INFO",
            Warn => "WARN",
            Error => "ERROR",
        }
    }
}

impl TryFrom<&str> for LogLevel {
    type Error = &'static str;

    fn try_from(s: &str) -> Result<Self, &'static str> {
        use LogLevel::*;

        match s.to_uppercase() {
            val if val == Debug.as_str() => Ok(Debug),
            val if val == Info.as_str() => Ok(Info),
            val if val == Warn.as_str() => Ok(Warn),
            val if val == Error.as_str() => Ok(Error),
            _ => Err("Not a valid log level"),
        }
    }
}

impl TryFrom<Option<&str>> for LogLevel {
    type Error = &'static str;

    fn try_from(s: Option<&str>) -> Result<Self, &'static str> {
        s.map(Self::try_from).unwrap_or(Err("No log level given"))
    }
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

pub struct FileLogger {
    file: Option<File>,
    filter: LogLevel,
}

impl FileLogger {
    pub fn new(path: Option<&str>, filter: Option<LogLevel>) -> Self {
        let filter = filter.unwrap_or_default();
        let file = path.and_then(|p| {
            OpenOptions::new()
                .read(false)
                .write(true)
                .create(true)
                .truncate(true)
                .open(p)
                .ok()
        });
        Self { file, filter }
    }

    pub fn write<D: Display>(&mut self, level: LogLevel, message: D) -> Result<bool, io::Error> {
        let Some(file) = self.file.as_mut() else {
            return Ok(false);
        };
        if level < self.filter {
            return Ok(false);
        }
        let time = chrono::Local::now();
        writeln!(file, "{level} {time} {message}").map(|()| true)
    }

    // Wrappers around `Self::write()` for convenience
    pub fn debug<D: Display>(&mut self, message: D) {
        _ = self.write(LogLevel::Debug, message);
    }
    pub fn info<D: Display>(&mut self, message: D) {
        _ = self.write(LogLevel::Info, message);
    }
    pub fn warn<D: Display>(&mut self, message: D) {
        _ = self.write(LogLevel::Warn, message);
    }
    pub fn error<D: Display>(&mut self, message: D) {
        _ = self.write(LogLevel::Error, message);
    }
}
