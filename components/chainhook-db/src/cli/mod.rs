use super::block;
use crate::{
    block::DigestingCommand,
    config::{Bare, Config, IndexerConfig, Topology},
};
use chainhook_event_observer::{
    chainhooks::{
        evaluate_stacks_chainhook_on_transaction, handle_stacks_hook_action,
        types::ChainhookSpecification, StacksChainhookOccurrence, StacksTriggerChainhook,
    },
    observer::{
        start_event_observer, EventObserverConfig, ObserverCommand, ObserverEvent,
        DEFAULT_CONTROL_PORT, DEFAULT_INGESTION_PORT,
    },
};
use chainhook_types::{BlockIdentifier, StacksBlockData, StacksTransactionData};
use clap::Parser;
use ctrlc;
use hiro_system_kit;
use std::collections::HashSet;
use std::{collections::HashMap, process, sync::mpsc::channel, thread};

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

    info!(
        "Listening for chainhooks events on port {}",
        DEFAULT_CONTROL_PORT
    );
    let _ = std::thread::spawn(move || {
        let future = start_event_observer(
            event_observer_config,
            observer_command_tx,
            observer_command_rx,
            Some(observer_event_tx),
        );
        let _ = hiro_system_kit::nestable_block_on(future);
    });

    loop {
        let event = match observer_event_rx.recv() {
            Ok(cmd) => cmd,
            Err(_e) => std::process::exit(1),
        };
        match event {
            ObserverEvent::HookRegistered(chain_hook) => {
                // If start block specified, use it.
                // I no start block specified, depending on the nature the hook, we'd like to retrieve:
                // - contract-id

                match chain_hook {
                    ChainhookSpecification::Stacks(stacks_hook) => {
                        info!("Received chainhook {:?}", stacks_hook);

                        use redis::Commands;

                        let client = redis::Client::open(config.redis_url.clone()).unwrap();
                        let mut con = client.get_connection().unwrap();

                        // Retrieve highest block height stored
                        let tip_height: u64 = con
                            .get(&format!("stx:tip"))
                            .expect("unable to retrieve tip height");

                        let start_block = stacks_hook.start_block.unwrap_or(2); // TODO(lgalabru): handle STX hooks and genesis block :s
                        let end_block = stacks_hook.end_block.unwrap_or(tip_height); // TODO(lgalabru): handle STX hooks and genesis block :s

                        // for cursor in 60000..=65000 {
                        for cursor in start_block..=end_block {
                            info!("Checking block {}", cursor);
                            let (block_identifier, transactions) = {
                                let payload: Vec<String> = con
                                    .hget(
                                        &format!("stx:{}", cursor),
                                        &["block_identifier", "transactions"],
                                    )
                                    .expect("unable to retrieve tip height");
                                if payload.len() != 2 {
                                    warn!("Chain still being processed, please retry in a few minutes");
                                    continue;
                                }
                                (
                                    serde_json::from_str::<BlockIdentifier>(&payload[0]).unwrap(),
                                    serde_json::from_str::<Vec<StacksTransactionData>>(&payload[1])
                                        .unwrap(),
                                )
                            };
                            let mut apply = vec![];
                            for tx in transactions.iter() {
                                if evaluate_stacks_chainhook_on_transaction(&tx, &stacks_hook) {
                                    info!("Predicate is true for transaction {}", cursor);
                                    apply.push((tx, &block_identifier));
                                }
                            }

                            if apply.len() > 0 {
                                let trigger = StacksTriggerChainhook {
                                    chainhook: &stacks_hook,
                                    apply,
                                    rollback: vec![],
                                };

                                let proofs = HashMap::new();
                                if let Some(result) = handle_stacks_hook_action(trigger, &proofs) {
                                    if let StacksChainhookOccurrence::Http(request) = result {
                                        hiro_system_kit::nestable_block_on(request.send()).unwrap();
                                    }
                                }
                            }
                        }
                    }
                    ChainhookSpecification::Bitcoin(bitcoin_hook) => {}
                }
            }
            ObserverEvent::Terminate => {
                break;
            }
            _ => {}
        }
    }
}
