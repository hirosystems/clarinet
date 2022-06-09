pub mod connection;
pub mod control_file;
pub mod core;
pub mod install;
pub mod jupyter_message;

use std;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use failure::Error;

pub struct EvalContextOutputs {
    pub stdout: mpsc::Receiver<String>,
    pub stderr: mpsc::Receiver<String>,
}

#[derive(Default, Debug)]
pub struct EvalOutputs {
    pub content_by_mime_type: HashMap<String, String>,
    pub timing: Option<Duration>,
}

pub struct CommandContext {
}

impl CommandContext {
    pub fn new() -> Result<(CommandContext, EvalContextOutputs), Error> {

        let (stdout_sender, stdout_receiver) = mpsc::channel();
        let (stderr_sender, stderr_receiver) = mpsc::channel();
        let outputs = EvalContextOutputs {
            stdout: stdout_receiver,
            stderr: stderr_receiver,
        };

        let command_context = CommandContext {};

        Ok((command_context, outputs))
    }

    pub fn execute(&mut self, to_run: &str) -> Result<EvalOutputs, Error> {
        unimplemented!();
    }

    pub fn set_opt_level(&mut self, level: &str) -> Result<(), Error> {
        unimplemented!();
    }

    fn load_config(&mut self) -> Result<EvalOutputs, Error> {
        unimplemented!();
    }

    fn process_command(&mut self, command: &str, args: Option<&str>) -> Result<EvalOutputs, Error> {
        unimplemented!();
    }

    fn vars_as_text(&self) -> String {
        unimplemented!();
    }

    fn vars_as_html(&self) -> String {
        unimplemented!();
    }
}
