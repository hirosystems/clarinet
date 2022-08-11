use super::block;
use crate::{
    block::DigestingCommand,
    config::{Config, IndexerConfig},
};
use clap::Parser;
use ctrlc;
use std::{
    process,
    sync::mpsc::channel,
    thread::{self, sleep},
    time::Duration,
};

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
    println!("-> {}", args.events_logs_csv_path);

    let (digestion_tx, digestion_rx) = channel();
    let digestion_terminator_tx = digestion_tx.clone();
    ctrlc::set_handler(move || {
        println!("Ctrl+C intercepted");
        digestion_terminator_tx
            .send(DigestingCommand::Terminate)
            .expect("Unable to terminate service");
    })
    .expect("Error setting Ctrl-C handler");

    let config = Config {
        redis_url: "redis://127.0.0.1/".into(),
        seed_tsv_path: args.events_logs_csv_path.clone(),
        stacks_node_pool: vec![],
        bitcoin_node_pool: vec![],
        indexer_config: IndexerConfig {
            stacks_node_rpc_url: "http://0.0.0.0:20443".into(),
            bitcoin_node_rpc_url: "http://0.0.0.0:18443".into(),
            bitcoin_node_rpc_username: "devnet".into(),
            bitcoin_node_rpc_password: "devnet".into(),
        },
    };

    let digestion_config = config.clone();
    thread::spawn(move || {
        block::digestion::start(digestion_rx, &digestion_config);
    });

    let ingestion_config = config.clone();
    let seed_digestion_tx = digestion_tx.clone();
    // thread::spawn(move || {
    let res = block::ingestion::start(seed_digestion_tx, &ingestion_config);
    let (stacks_chain_tip, bitcoin_chain_tip) = match res {
        Ok(chain_tips) => chain_tips,
        Err(e) => {
            println!("{}", e);
            process::exit(1);
        }
    };
    // });

    sleep(Duration::from_secs(180));
    // sleep_ms(180_000)
}
