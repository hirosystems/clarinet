#![allow(unused_imports)]

extern crate serde;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate serde_json;

#[macro_use]
extern crate rocket;

mod chainhooks;
mod indexer;
mod observer;
mod utils;

use crate::chainhooks::types::HookFormation;
use clap::Parser;
use ctrlc;
use observer::{EventHandler, EventObserverConfig, ObserverCommand};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use toml::value::Value;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path of the config to load
    #[clap(short, long)]
    config_path: Option<String>,
}

#[rocket::main]
async fn main() {
    let args = Args::parse();
    let config_path = get_config_path_or_exit(&args.config_path);
    let config = EventObserverConfig::from_path(&config_path);
    let (command_tx, command_rx) = channel();
    let tx_terminator = command_tx.clone();

    ctrlc::set_handler(move || {
        tx_terminator
            .send(ObserverCommand::Terminate)
            .expect("Could not send signal on channel.")
    })
    .expect("Error setting Ctrl-C handler");

    let config = EventObserverConfig {
        normalization_enabled: true,
        bitcoin_rpc_proxy_enabled: false,
        initial_hook_formation: None,
        hooks_enabled: true,
        event_handlers: vec![EventHandler::WebHook("http://0.0.0.0:19999".into())],
        control_port: 9998,
        ingestion_port: 9999,
        bitcoin_node_username: "devnet".into(),
        bitcoin_node_password: "devnet".into(),
        bitcoin_node_rpc_host: "0.0.0.0".into(),
        bitcoin_node_rpc_port: 18443,
        stacks_node_rpc_host: "0.0.0.0".into(),
        stacks_node_rpc_port: 20443,
        grpc_server_enabled: true,
        operators: HashMap::new(),
    };
    observer::start_event_observer(config, command_tx, command_rx, None).await;
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct EventObserverConfigFile {
    pub normalization_enabled: Option<bool>,
    pub grpc_server_enabled: Option<bool>,
    pub hooks_enabled: Option<bool>,
    pub bitcoin_rpc_proxy_enabled: Option<bool>,
    pub webhooks: Option<Vec<String>>,
    pub ingestion_port: Option<u16>,
    pub control_port: Option<u16>,
    pub bitcoin_node_username: String,
    pub bitcoin_node_password: String,
    pub bitcoin_node_rpc_host: String,
    pub bitcoin_node_rpc_port: u16,
    pub stacks_node_rpc_host: String,
    pub stacks_node_rpc_port: u16,
    pub operators: Option<Vec<String>>,
}

impl EventObserverConfig {
    pub fn from_path(path: &PathBuf) -> EventObserverConfig {
        let path = match File::open(path) {
            Ok(path) => path,
            Err(_e) => {
                println!("Error: unable to locate Clarinet.toml in current directory");
                std::process::exit(1);
            }
        };
        let mut file_reader = BufReader::new(path);
        let mut file_buffer = vec![];
        file_reader.read_to_end(&mut file_buffer).unwrap();

        let file: EventObserverConfigFile = match toml::from_slice(&file_buffer[..]) {
            Ok(s) => s,
            Err(e) => {
                println!("Unable to read config {}", e);
                std::process::exit(1);
            }
        };

        EventObserverConfig::from_config_file(file)
    }

    pub fn from_config_file(mut config_file: EventObserverConfigFile) -> EventObserverConfig {
        let event_handlers = match config_file.webhooks.take() {
            Some(webhooks) => webhooks
                .into_iter()
                .map(|h| EventHandler::WebHook(h))
                .collect::<Vec<_>>(),
            None => vec![],
        };
        let mut operators = HashMap::new();
        if let Some(operator_keys) = config_file.operators.take() {
            for operator_key in operator_keys.into_iter() {
                operators.insert(Some(operator_key), HookFormation::new());
            }
        }

        let config = EventObserverConfig {
            normalization_enabled: config_file.normalization_enabled.unwrap_or(true),
            grpc_server_enabled: config_file.grpc_server_enabled.unwrap_or(false),
            hooks_enabled: config_file.hooks_enabled.unwrap_or(false),
            initial_hook_formation: Some(HookFormation::new()),
            bitcoin_rpc_proxy_enabled: config_file.bitcoin_rpc_proxy_enabled.unwrap_or(false),
            event_handlers: event_handlers,
            ingestion_port: config_file
                .ingestion_port
                .unwrap_or(observer::DEFAULT_INGESTION_PORT),
            control_port: config_file
                .control_port
                .unwrap_or(observer::DEFAULT_CONTROL_PORT),
            bitcoin_node_username: config_file.bitcoin_node_username.clone(),
            bitcoin_node_password: config_file.bitcoin_node_password.clone(),
            bitcoin_node_rpc_host: config_file.bitcoin_node_rpc_host.clone(),
            bitcoin_node_rpc_port: config_file.bitcoin_node_rpc_port.clone(),
            stacks_node_rpc_host: config_file.stacks_node_rpc_host.clone(),
            stacks_node_rpc_port: config_file.stacks_node_rpc_port.clone(),
            operators,
        };
        config
    }
}

fn get_config_path_or_exit(path: &Option<String>) -> PathBuf {
    if let Some(path) = path {
        let manifest_path = PathBuf::from(path);
        if !manifest_path.exists() {
            println!("Could not find Observer.toml");
            std::process::exit(1);
        }
        manifest_path
    } else {
        let mut current_dir = std::env::current_dir().unwrap();
        loop {
            current_dir.push("Observer.toml");

            if current_dir.exists() {
                break current_dir;
            }
            current_dir.pop();

            if !current_dir.pop() {
                println!("Could not find Observer.toml");
                std::process::exit(1);
            }
        }
    }
}
