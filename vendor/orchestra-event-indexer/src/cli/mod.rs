use super::block;
use crate::{
    block::DigestingCommand,
    config::{Config, IndexerConfig, Topology, Bare},
};
use clap::Parser;
use ctrlc;
use orchestra_event_observer::{observer::{EventObserverConfig, ObserverEvent, DEFAULT_INGESTION_PORT, DEFAULT_CONTROL_PORT, start_event_observer, ObserverCommand}, utils::nestable_block_on};
use std::{
    process,
    sync::mpsc::channel,
    thread,
};
use std::collections::HashSet;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Name of the person to greet
    #[clap(short, long, value_parser)]
    events_logs_csv_path: String,
}

pub fn main() {
    let args = Args::parse();
    let (digestion_tx, digestion_rx) = channel();
    let (observer_event_tx, observer_event_rx) = channel();
    let (observer_command_tx, observer_command_rx) = channel();

    let terminate_digestion_tx = digestion_tx.clone();
    ctrlc::set_handler(move || {
        warn!("Manual interruption signal received");
        terminate_digestion_tx
            .send(DigestingCommand::Kill)
            .expect("Unable to terminate service");
    })
    .expect("Error setting Ctrl-C handler");

    let config = Config {
        redis_url: "redis://127.0.0.1/".into(),
        events_dump_url: "https://storage.googleapis.com/blockstack-publish/archiver-main/api/stacks-node-events-latest.tar.gz".into(),
        seed_tsv_path: args.events_logs_csv_path.clone(),
        topology: Topology::Bare(Bare {
            stacks_node_pool: vec!["http://0.0.0.0:20443".into()],
            bitcoin_node_pool: vec!["http://0.0.0.0:18443".into()],
        }),
        indexer_config: IndexerConfig {
            stacks_node_rpc_url: "http://0.0.0.0:20443".into(),
            bitcoin_node_rpc_url: "http://0.0.0.0:18443".into(),
            bitcoin_node_rpc_username: "devnet".into(),
            bitcoin_node_rpc_password: "devnet".into(),
        },
    };

    let ingestion_config = config.clone();
    let seed_digestion_tx = digestion_tx.clone();
    thread::spawn(move || {
        let res = block::ingestion::start(seed_digestion_tx.clone(), &ingestion_config);
        let (_stacks_chain_tip, _bitcoin_chain_tip) = match res {
            Ok(chain_tips) => chain_tips,
            Err(e) => {
                error!("{}", e);
                process::exit(1);
            }
        };
        seed_digestion_tx
            .send(DigestingCommand::Terminate)
            .expect("Unable to terminate service");
    });

    let digestion_config = config.clone();
    let terminate_observer_command_tx = observer_command_tx.clone();
    thread::spawn(move || {
        block::digestion::start(digestion_rx, &digestion_config);
        let _ = terminate_observer_command_tx.send(ObserverCommand::Terminate);
    });
    
    let event_observer_config = EventObserverConfig {
        normalization_enabled: true,
        grpc_server_enabled: false,
        hooks_enabled: true,
        bitcoin_rpc_proxy_enabled: true,
        event_handlers: vec![],
        initial_hook_formation: None,
        ingestion_port: DEFAULT_INGESTION_PORT,
        control_port: DEFAULT_CONTROL_PORT,
        bitcoin_node_username: "devnet".into(),
        bitcoin_node_password: "devnet".into(),
        bitcoin_node_rpc_host: "http://localhost".into(),
        bitcoin_node_rpc_port: 18443,
        stacks_node_rpc_host: "http://localhost".into(),
        stacks_node_rpc_port: 20443,
        operators: HashSet::new(),
        display_logs: false,
    };

    let _ = std::thread::spawn(move || {
        let future = start_event_observer(
            event_observer_config,
            observer_command_tx,
            observer_command_rx,
            Some(observer_event_tx),
        );
        let _ = nestable_block_on(future);
    });
    
    loop {
        let event = match observer_event_rx.recv() {
            Ok(cmd) => cmd,
            Err(_e) => std::process::exit(1)
        };
        match event {
            ObserverEvent::HookRegistered(chain_hook) => {
                // Do something
            }
            ObserverEvent::Terminate => {
                break;
            }
            _ => {}
        }
    }
}
