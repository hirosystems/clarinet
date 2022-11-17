use super::block;
use crate::archive;
use crate::block::DigestingCommand;
use crate::config::Config;

use chainhook_event_observer::chainhooks::bitcoin::{
    handle_bitcoin_hook_action, BitcoinChainhookOccurrence, BitcoinTriggerChainhook,
};
use chainhook_event_observer::indexer::bitcoin::build_block;
use chainhook_event_observer::observer::{
    start_event_observer, EventObserverConfig, ObserverCommand, ObserverEvent,
};
use chainhook_event_observer::{
    chainhooks::stacks::{
        evaluate_stacks_transaction_predicate_on_transaction, handle_stacks_hook_action,
        StacksChainhookOccurrence, StacksTriggerChainhook,
    },
    chainhooks::types::ChainhookSpecification,
};

use bitcoincore_rpc::{Auth, Client, RpcApi};
use chainhook_types::{
    BitcoinBlockData, BitcoinBlockMetadata, BitcoinTransactionData, BlockIdentifier,
    StacksBlockData, StacksBlockMetadata, StacksChainEvent, StacksNetwork, StacksTransactionData,
};
use clap::{Parser, Subcommand};
use ctrlc;
use hiro_system_kit;
use redis::{Commands, Connection};
use reqwest::Url;
use std::collections::HashSet;
use std::{collections::HashMap, process, sync::mpsc::channel, thread};

pub const DEFAULT_INGESTION_PORT: u16 = 20455;
pub const DEFAULT_CONTROL_PORT: u16 = 20456;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Opts {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, PartialEq, Clone, Debug)]
enum Command {
    /// Start chainhook-node
    #[clap(name = "start", bin_name = "start")]
    Start(StartNode),
    /// Start chainhook-node in replay mode
    #[clap(name = "replay", bin_name = "replay")]
    Replay(ReplayConfig),
}

#[derive(Parser, PartialEq, Clone, Debug)]
struct StartNode {
    /// Target Devnet network
    #[clap(
        long = "devnet",
        conflicts_with = "testnet",
        conflicts_with = "mainnet"
    )]
    pub devnet: bool,
    /// Target Testnet network
    #[clap(
        long = "testnet",
        conflicts_with = "devnet",
        conflicts_with = "mainnet"
    )]
    pub testnet: bool,
    /// Target Mainnet network
    #[clap(
        long = "mainnet",
        conflicts_with = "testnet",
        conflicts_with = "devnet"
    )]
    pub mainnet: bool,
    /// Load config file path
    #[clap(
        long = "config-path",
        conflicts_with = "mainnet",
        conflicts_with = "testnet",
        conflicts_with = "devnet"
    )]
    pub config_path: Option<String>,
}

#[derive(Parser, PartialEq, Clone, Debug)]
struct ReplayConfig {
    pub devnet: bool,
    /// Target Testnet network
    #[clap(
        long = "testnet",
        conflicts_with = "devnet",
        conflicts_with = "mainnet"
    )]
    pub testnet: bool,
    /// Target Mainnet network
    #[clap(
        long = "mainnet",
        conflicts_with = "testnet",
        conflicts_with = "devnet"
    )]
    pub mainnet: bool,
    /// Apply chainhook action (false by default)
    #[clap(long = "apply-trigger")]
    pub apply_trigger: bool,
    /// Bitcoind node url
    #[clap(long = "bitcoind-rpc-url")]
    pub bitcoind_rpc_url: String,
}

pub fn main() {
    let opts: Opts = match Opts::try_parse() {
        Ok(opts) => opts,
        Err(e) => {
            println!("{}", e);
            process::exit(1);
        }
    };

    match opts.command {
        Command::Start(cmd) => {
            let config = match (cmd.devnet, cmd.testnet, cmd.mainnet, cmd.config_path) {
                (true, false, false, _) => Config::devnet_default(),
                (false, true, false, _) => Config::testnet_default(),
                (false, false, true, _) => Config::mainnet_default(),
                (false, false, false, Some(config_path)) => {
                    match Config::from_file_path(&config_path) {
                        Ok(config) => config,
                        Err(e) => {
                            println!("{e}");
                            process::exit(1);
                        }
                    }
                }
                _ => {
                    println!("network flag required (devnet, testnet, mainnet)");
                    process::exit(1);
                }
            };
            start_node(config);
        }
        Command::Replay(cmd) => {
            let network = match (cmd.testnet, cmd.mainnet) {
                (true, false) => StacksNetwork::Testnet,
                (false, true) => StacksNetwork::Mainnet,
                _ => {
                    println!(
                        "{}",
                        format_err!("network flag required (support --testnet, --mainnet)")
                    );
                    process::exit(1);
                }
            };
            let bitcoind_rpc_url = if cmd.bitcoind_rpc_url == "" {
                Url::parse("http://devnet:devnet@localhost:18443").unwrap()
            } else {
                match Url::parse(&cmd.bitcoind_rpc_url) {
                    Ok(url) => url,
                    Err(e) => {
                        println!(
                            "{} ({})",
                            format_err!("unable to parse bitcoin url"),
                            e.to_string()
                        );
                        process::exit(1);
                    }
                }
            };
            start_replay_flow(&network, bitcoind_rpc_url, cmd.apply_trigger);
        }
    }
}

pub fn start_replay_flow(network: &StacksNetwork, bitcoind_rpc_url: Url, apply: bool) {
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

    let mut config = match network {
        StacksNetwork::Devnet => Config::devnet_default(),
        StacksNetwork::Testnet => Config::testnet_default(),
        StacksNetwork::Mainnet => Config::mainnet_default(),
        _ => unreachable!(),
    };
    let bitcoin_host = bitcoind_rpc_url
        .host()
        .expect("unable to retrieve host from bitcoin_url")
        .to_string();
    let bitcoin_port = bitcoind_rpc_url
        .port()
        .expect("unable to retrieve port from bitcoin_url");
    config.network.bitcoin_node_rpc_url = format!("http://{}:{}", bitcoin_host, bitcoin_port);
    config.network.bitcoin_node_rpc_username = bitcoind_rpc_url.username().to_string();
    config.network.bitcoin_node_rpc_password = bitcoind_rpc_url
        .password()
        .expect("unable to retrieve password from bitcoin_url")
        .to_string();

    if config.is_initial_ingestion_required() {
        // Download default tsv.
        if config.rely_on_remote_tsv() && config.should_download_remote_tsv() {
            let url = config.expected_remote_tsv_url();
            let mut destination_path = config.expected_cache_path();
            destination_path.push("stacks-node-events.tsv");
            // Download archive if not already present in cache
            if !destination_path.exists() {
                info!("Downloading {}", url);
                match hiro_system_kit::nestable_block_on(archive::download_tsv_file(&config)) {
                    Ok(_) => {}
                    Err(e) => {
                        error!("{}", e);
                        process::exit(1);
                    }
                }
                let mut destination_path = config.expected_cache_path();
                destination_path.push("stacks-node-events.tsv");
            }
            config.add_local_tsv_source(&destination_path);

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
            });
        }
    } else {
        info!(
            "Streaming blocks from stacks-node {}",
            config.expected_stacks_node_event_source()
        );
    }

    let digestion_config = config.clone();
    let terminate_observer_command_tx = observer_command_tx.clone();
    thread::spawn(move || {
        let res = block::digestion::start(digestion_rx, &digestion_config);
        if let Err(e) = res {
            crit!("{}", e);
        }
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
        bitcoin_node_username: config.network.bitcoin_node_rpc_username.clone(),
        bitcoin_node_password: config.network.bitcoin_node_rpc_password.clone(),
        bitcoin_node_rpc_url: config.network.bitcoin_node_rpc_url.clone(),
        stacks_node_rpc_url: config.network.stacks_node_rpc_url.clone(),
        operators: HashSet::new(),
        display_logs: false,
    };
    info!(
        "Listening for new blockchain events on port {}",
        DEFAULT_INGESTION_PORT
    );
    info!(
        "Listening for chainhook predicate registrations on port {}",
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

    let redis_config = config.expected_redis_config();
    let client = redis::Client::open(redis_config.uri.clone()).unwrap();
    let mut redis_con = match client.get_connection() {
        Ok(con) => con,
        Err(message) => {
            crit!("Redis: {}", message.to_string());
            panic!();
        }
    };

    let auth = Auth::UserPass(
        config.network.bitcoin_node_rpc_username.clone(),
        config.network.bitcoin_node_rpc_password.clone(),
    );

    let bitcoin_rpc = match Client::new(&config.network.bitcoin_node_rpc_url, auth) {
        Ok(con) => con,
        Err(message) => {
            crit!("Bitcoin RPC: {}", message.to_string());
            panic!();
        }
    };

    loop {
        let event = match observer_event_rx.recv() {
            Ok(cmd) => cmd,
            Err(e) => {
                crit!("Error: broken channel {}", e.to_string());
                break;
            }
        };
        match event {
            ObserverEvent::HookRegistered(chain_hook) => {
                // If start block specified, use it.
                // I no start block specified, depending on the nature the hook, we'd like to retrieve:
                // - contract-id

                match chain_hook {
                    ChainhookSpecification::Stacks(stacks_hook) => {
                        // Retrieve highest block height stored
                        let tip_height: u64 = redis_con
                            .get(&format!("stx:tip"))
                            .expect("unable to retrieve tip height");

                        let start_block = stacks_hook.start_block.unwrap_or(2); // TODO(lgalabru): handle STX hooks and genesis block :s
                        let end_block = stacks_hook.end_block.unwrap_or(tip_height); // TODO(lgalabru): handle STX hooks and genesis block :s

                        info!(
                            "Processing Stacks chainhook {}, will scan blocks [{}; {}]  (apply = {})",
                            stacks_hook.uuid, start_block, end_block, apply
                        );
                        let mut total_hits = vec![];
                        for cursor in start_block..=end_block {
                            debug!(
                                "Evaluating predicate #{} on block #{}",
                                stacks_hook.uuid, cursor
                            );
                            let (
                                block_identifier,
                                parent_block_identifier,
                                timestamp,
                                transactions,
                                metadata,
                            ) = {
                                let payload: Vec<String> = redis_con
                                    .hget(
                                        &format!("stx:{}", cursor),
                                        &[
                                            "block_identifier",
                                            "parent_block_identifier",
                                            "timestamp",
                                            "transactions",
                                            "metadata",
                                        ],
                                    )
                                    .expect("unable to retrieve tip height");
                                if payload.len() != 5 {
                                    warn!("Chain still being processed, please retry in a few minutes");
                                    continue;
                                }
                                (
                                    serde_json::from_str::<BlockIdentifier>(&payload[0]).unwrap(),
                                    serde_json::from_str::<BlockIdentifier>(&payload[1]).unwrap(),
                                    serde_json::from_str::<i64>(&payload[2]).unwrap(),
                                    serde_json::from_str::<Vec<StacksTransactionData>>(&payload[3])
                                        .unwrap(),
                                    serde_json::from_str::<StacksBlockMetadata>(&payload[4])
                                        .unwrap(),
                                )
                            };
                            let mut hits = vec![];
                            for tx in transactions.iter() {
                                if evaluate_stacks_transaction_predicate_on_transaction(
                                    &tx,
                                    &stacks_hook,
                                ) {
                                    debug!(
                                        "Action #{} triggered by transaction {} (block #{})",
                                        stacks_hook.uuid, tx.transaction_identifier.hash, cursor
                                    );
                                    hits.push(tx);
                                    total_hits.push(tx.transaction_identifier.hash.to_string());
                                }
                            }

                            if hits.len() > 0 {
                                let block = StacksBlockData {
                                    block_identifier,
                                    parent_block_identifier,
                                    timestamp,
                                    transactions: vec![],
                                    metadata,
                                };
                                let trigger = StacksTriggerChainhook {
                                    chainhook: &stacks_hook,
                                    apply: vec![(hits, &block)],
                                    rollback: vec![],
                                };

                                let proofs = HashMap::new();
                                if apply {
                                    if let Some(result) =
                                        handle_stacks_hook_action(trigger, &proofs)
                                    {
                                        if let StacksChainhookOccurrence::Http(request) = result {
                                            hiro_system_kit::nestable_block_on(request.send())
                                                .unwrap();
                                        }
                                    }
                                }
                            }
                        }

                        info!("Stacks chainhook {} scan completed and triggered by {} transactions {}", stacks_hook.uuid, total_hits.len(), total_hits.join(","))
                    }
                    ChainhookSpecification::Bitcoin(bitcoin_hook) => {
                        let start_block = match bitcoin_hook.start_block {
                            Some(start_block) => start_block,
                            None => {
                                warn!("Bitcoin chainhook specification must include a field start_block in replay mode");
                                continue;
                            }
                        };
                        let tip_height = match bitcoin_rpc.get_blockchain_info() {
                            Ok(result) => result.blocks,
                            Err(e) => {
                                warn!("unable to retrieve Bitcoin chain tip ({})", e.to_string());
                                continue;
                            }
                        };
                        let end_block = bitcoin_hook.end_block.unwrap_or(tip_height);

                        info!(
                            "Processing Bitcoin chainhook {}, will scan blocks [{}; {}] (apply = {})",
                            bitcoin_hook.uuid, start_block, end_block, apply
                        );

                        let mut total_hits = vec![];
                        for cursor in start_block..=end_block {
                            debug!(
                                "Evaluating predicate #{} on block #{}",
                                bitcoin_hook.uuid, cursor
                            );

                            // Try to retrieve block from cache

                            let cached_block = {
                                let payload: Vec<String> = redis_con
                                    .hget(
                                        &format!("btc:{}", cursor),
                                        &[
                                            "block_identifier",
                                            "parent_block_identifier",
                                            "timestamp",
                                            "transactions",
                                            "metadata",
                                        ],
                                    )
                                    .expect("unable to retrieve tip height");
                                if payload.len() != 5 {
                                    None
                                } else {
                                    let block = BitcoinBlockData {
                                        block_identifier: serde_json::from_str::<BlockIdentifier>(
                                            &payload[0],
                                        )
                                        .unwrap(),
                                        parent_block_identifier: serde_json::from_str::<
                                            BlockIdentifier,
                                        >(
                                            &payload[1]
                                        )
                                        .unwrap(),
                                        timestamp: serde_json::from_str::<u32>(&payload[2])
                                            .unwrap(),
                                        transactions: serde_json::from_str::<
                                            Vec<BitcoinTransactionData>,
                                        >(
                                            &payload[3]
                                        )
                                        .unwrap(),
                                        metadata: serde_json::from_str::<BitcoinBlockMetadata>(
                                            &payload[4],
                                        )
                                        .unwrap(),
                                    };
                                    debug!(
                                        "Bitcoin block #{} retrieved from cache",
                                        block.block_identifier.index
                                    );
                                    Some(block)
                                }
                            };

                            let block = match cached_block {
                                Some(block) => block,
                                None => {
                                    let block_hash = match bitcoin_rpc.get_block_hash(cursor) {
                                        Ok(block_hash) => block_hash,
                                        Err(e) => {
                                            error!(
                                                "unable to retrieve block hash {}: {}",
                                                cursor,
                                                e.to_string()
                                            );
                                            continue;
                                        }
                                    };

                                    let block = match bitcoin_rpc.get_block(&block_hash) {
                                        Ok(block) => build_block(block, cursor, &config.network),
                                        Err(e) => {
                                            error!(
                                                "unable to retrieve block {}: {}",
                                                cursor,
                                                e.to_string()
                                            );
                                            continue;
                                        }
                                    };

                                    let key = format!("btc:{}", block.block_identifier.index);
                                    match redis_con.hset_multiple(
                                        &key,
                                        &[
                                            (
                                                "block_identifier",
                                                json!(block.block_identifier).to_string(),
                                            ),
                                            (
                                                "parent_block_identifier",
                                                json!(block.parent_block_identifier).to_string(),
                                            ),
                                            ("transactions", json!(block.transactions).to_string()),
                                            ("metadata", json!(block.metadata).to_string()),
                                            ("timestamp", json!(block.timestamp).to_string()),
                                        ],
                                    ) {
                                        Ok(()) => {
                                            debug!(
                                                "Bitcoin block #{} saved to cache",
                                                block.block_identifier.index
                                            );
                                        }
                                        Err(e) => {
                                            warn!(
                                                "unable to keep block {key} in cache: {}",
                                                e.to_string()
                                            );
                                        }
                                    };

                                    block
                                }
                            };

                            let mut hits = vec![];
                            for tx in block.transactions.iter() {
                                if bitcoin_hook.evaluate_transaction_predicate(&tx) {
                                    debug!(
                                        "Action #{} triggered by transaction {} (block #{})",
                                        bitcoin_hook.uuid, tx.transaction_identifier.hash, cursor
                                    );
                                    hits.push(tx);
                                    total_hits.push(tx.transaction_identifier.hash.to_string());
                                }
                            }

                            if hits.len() > 0 {
                                let trigger = BitcoinTriggerChainhook {
                                    chainhook: &bitcoin_hook,
                                    apply: vec![(hits, &block)],
                                    rollback: vec![],
                                };

                                let proofs = HashMap::new();
                                if apply {
                                    if let Some(result) =
                                        handle_bitcoin_hook_action(trigger, &proofs)
                                    {
                                        if let BitcoinChainhookOccurrence::Http(request) = result {
                                            hiro_system_kit::nestable_block_on(request.send())
                                                .unwrap();
                                        }
                                    }
                                }
                            }
                        }
                        info!("Bitcoin chainhook {} scan completed and triggered by {} transactions {}", bitcoin_hook.uuid, total_hits.len(), total_hits.join(","))
                    }
                }
            }
            ObserverEvent::BitcoinChainEvent(_chain_update) => {
                debug!("Bitcoin update not stored");
            }
            ObserverEvent::StacksChainEvent(chain_event) => {
                match &chain_event {
                    StacksChainEvent::ChainUpdatedWithBlocks(data) => {
                        update_storage_with_confirmed_stacks_blocks(
                            &mut redis_con,
                            &data.confirmed_blocks,
                        );
                    }
                    StacksChainEvent::ChainUpdatedWithReorg(data) => {
                        update_storage_with_confirmed_stacks_blocks(
                            &mut redis_con,
                            &data.confirmed_blocks,
                        );
                    }
                    StacksChainEvent::ChainUpdatedWithMicroblocks(_)
                    | StacksChainEvent::ChainUpdatedWithMicroblocksReorg(_) => {}
                };
            }
            ObserverEvent::Terminate => {
                break;
            }
            _ => {}
        }
    }
}

pub fn start_node(mut config: Config) {
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

    if config.is_initial_ingestion_required() {
        // Download default tsv.
        if config.rely_on_remote_tsv() && config.should_download_remote_tsv() {
            let url = config.expected_remote_tsv_url();
            let mut destination_path = config.expected_cache_path();
            destination_path.push("stacks-node-events.tsv");
            // Download archive if not already present in cache
            if !destination_path.exists() {
                info!("Downloading {}", url);
                match hiro_system_kit::nestable_block_on(archive::download_tsv_file(&config)) {
                    Ok(_) => {}
                    Err(e) => {
                        error!("{}", e);
                        process::exit(1);
                    }
                }
                let mut destination_path = config.expected_cache_path();
                destination_path.push("stacks-node-events.tsv");
            }
            config.add_local_tsv_source(&destination_path);

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
            });
        }
    } else {
        info!(
            "Streaming blocks from stacks-node {}",
            config.expected_stacks_node_event_source()
        );
    }

    let digestion_config = config.clone();
    let terminate_observer_command_tx = observer_command_tx.clone();
    thread::spawn(move || {
        let res = block::digestion::start(digestion_rx, &digestion_config);
        if let Err(e) = res {
            error!("{}", e);
        }
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
        bitcoin_node_username: config.network.bitcoin_node_rpc_username.clone(),
        bitcoin_node_password: config.network.bitcoin_node_rpc_password.clone(),
        bitcoin_node_rpc_url: config.network.bitcoin_node_rpc_url.clone(),
        stacks_node_rpc_url: config.network.stacks_node_rpc_url.clone(),
        operators: HashSet::new(),
        display_logs: false,
    };
    info!(
        "Listening for new blockchain events on port {}",
        DEFAULT_INGESTION_PORT
    );
    info!(
        "Listening for chainhook predicate registrations on port {}",
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
            Err(e) => {
                error!("Error: broken channel {}", e.to_string());
                break;
            }
        };
        let redis_config = config.expected_redis_config();
        let client = redis::Client::open(redis_config.uri.clone()).unwrap();
        let mut redis_con = match client.get_connection() {
            Ok(con) => con,
            Err(message) => {
                error!("Redis: {}", message.to_string());
                panic!();
            }
        };
        match event {
            ObserverEvent::HookRegistered(chain_hook) => {
                // If start block specified, use it.
                // I no start block specified, depending on the nature the hook, we'd like to retrieve:
                // - contract-id

                match chain_hook {
                    ChainhookSpecification::Stacks(stacks_hook) => {
                        // Retrieve highest block height stored
                        let tip_height: u64 = redis_con
                            .get(&format!("stx:tip"))
                            .expect("unable to retrieve tip height");

                        let start_block = stacks_hook.start_block.unwrap_or(2); // TODO(lgalabru): handle STX hooks and genesis block :s
                        let end_block = stacks_hook.end_block.unwrap_or(tip_height); // TODO(lgalabru): handle STX hooks and genesis block :s

                        info!(
                            "Processing Stacks chainhook {}, will scan blocks [{}; {}]",
                            stacks_hook.uuid, start_block, end_block
                        );
                        let mut total_hits = 0;
                        for cursor in start_block..=end_block {
                            debug!(
                                "Evaluating predicate #{} on block #{}",
                                stacks_hook.uuid, cursor
                            );
                            let (
                                block_identifier,
                                parent_block_identifier,
                                timestamp,
                                transactions,
                                metadata,
                            ) = {
                                let payload: Vec<String> = redis_con
                                    .hget(
                                        &format!("stx:{}", cursor),
                                        &[
                                            "block_identifier",
                                            "parent_block_identifier",
                                            "timestamp",
                                            "transactions",
                                            "metadata",
                                        ],
                                    )
                                    .expect("unable to retrieve tip height");
                                if payload.len() != 5 {
                                    warn!("Chain still being processed, please retry in a few minutes");
                                    continue;
                                }
                                (
                                    serde_json::from_str::<BlockIdentifier>(&payload[0]).unwrap(),
                                    serde_json::from_str::<BlockIdentifier>(&payload[1]).unwrap(),
                                    serde_json::from_str::<i64>(&payload[2]).unwrap(),
                                    serde_json::from_str::<Vec<StacksTransactionData>>(&payload[3])
                                        .unwrap(),
                                    serde_json::from_str::<StacksBlockMetadata>(&payload[4])
                                        .unwrap(),
                                )
                            };
                            let mut hits = vec![];
                            for tx in transactions.iter() {
                                if evaluate_stacks_transaction_predicate_on_transaction(
                                    &tx,
                                    &stacks_hook,
                                ) {
                                    debug!(
                                        "Action #{} triggered by transaction {} (block #{})",
                                        stacks_hook.uuid, tx.transaction_identifier.hash, cursor
                                    );
                                    hits.push(tx);
                                    total_hits += 1;
                                }
                            }

                            if hits.len() > 0 {
                                let block = StacksBlockData {
                                    block_identifier,
                                    parent_block_identifier,
                                    timestamp,
                                    transactions: vec![],
                                    metadata,
                                };
                                let trigger = StacksTriggerChainhook {
                                    chainhook: &stacks_hook,
                                    apply: vec![(hits, &block)],
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
                        info!("Stacks chainhook {} scan completed: action triggered by {} transactions", stacks_hook.uuid, total_hits);
                    }
                    ChainhookSpecification::Bitcoin(_bitcoin_hook) => {
                        warn!("Bitcoin chainhook evaluation unavailable for historical data");
                    }
                }
            }
            ObserverEvent::BitcoinChainEvent(_chain_update) => {
                debug!("Bitcoin update not stored");
            }
            ObserverEvent::StacksChainEvent(chain_event) => {
                match &chain_event {
                    StacksChainEvent::ChainUpdatedWithBlocks(data) => {
                        update_storage_with_confirmed_stacks_blocks(
                            &mut redis_con,
                            &data.confirmed_blocks,
                        );
                    }
                    StacksChainEvent::ChainUpdatedWithReorg(data) => {
                        update_storage_with_confirmed_stacks_blocks(
                            &mut redis_con,
                            &data.confirmed_blocks,
                        );
                    }
                    StacksChainEvent::ChainUpdatedWithMicroblocks(_)
                    | StacksChainEvent::ChainUpdatedWithMicroblocksReorg(_) => {}
                };
            }
            ObserverEvent::Terminate => {
                break;
            }
            _ => {}
        }
    }
}

fn update_storage_with_confirmed_stacks_blocks(
    redis_con: &mut Connection,
    blocks: &Vec<StacksBlockData>,
) {
    let current_tip_height: u64 = redis_con.get(&format!("stx:tip")).unwrap_or(0);

    let mut new_tip = None;

    for block in blocks.iter() {
        let res: Result<(), redis::RedisError> = redis_con.hset_multiple(
            &format!("stx:{}", block.block_identifier.index),
            &[
                (
                    "block_identifier",
                    json!(block.block_identifier).to_string(),
                ),
                (
                    "parent_block_identifier",
                    json!(block.parent_block_identifier).to_string(),
                ),
                ("transactions", json!(block.transactions).to_string()),
                ("metadata", json!(block.metadata).to_string()),
            ],
        );
        if let Err(error) = res {
            crit!(
                "unable to archive block {}: {}",
                block.block_identifier,
                error.to_string()
            );
        }
        if block.block_identifier.index >= current_tip_height {
            new_tip = Some(block);
        }
    }

    if let Some(block) = new_tip {
        info!(
            "Archiving confirmed Stacks chain block {}",
            block.block_identifier
        );
        let _: Result<(), redis::RedisError> =
            redis_con.set(&format!("stx:tip"), block.block_identifier.index);
    }
}
