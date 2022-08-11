use crate::config::Config;
use orchestra_event_observer::indexer::stacks::NewBlockHeader;
use orchestra_event_observer::indexer::{self, Indexer};
use orchestra_types::{
    BlockIdentifier, StacksBlockData, StacksChainEvent, StacksChainUpdatedWithBlocksData,
    StacksChainUpdatedWithMicroblocksData, StacksMicroblocksTrail,
};
use redis;
use redis::Commands;
use serde::Deserialize;
use std::sync::mpsc::Sender;
use std::{sync::mpsc::channel, thread};

use super::DigestingCommand;

#[derive(Debug, Deserialize)]
pub struct Record {
    pub id: u64,
    pub created_at: String,
    pub kind: RecordKind,
    pub raw_log: String,
}

#[derive(Debug, Deserialize)]
pub enum RecordKind {
    #[serde(rename = "/new_block")]
    StacksBlockReceived,
    #[serde(rename = "/new_microblocks")]
    StacksMicroblockReceived,
    #[serde(rename = "/new_burn_block")]
    BitcoinBlockReceived,
    #[serde(rename = "/new_mempool_tx")]
    TransactionAdmitted,
    #[serde(rename = "/drop_mempool_tx")]
    TransactionDropped,
    #[serde(rename = "/attachments/new")]
    AttachmentReceived,
}

pub fn start(
    digestion_tx: Sender<DigestingCommand>,
    config: &Config,
) -> Result<(BlockIdentifier, BlockIdentifier), String> {
    let (stacks_record_tx, stacks_record_rx) = channel();
    let (bitcoin_record_tx, bitcoin_record_rx) = channel();

    let seed_tsv_path = config.seed_tsv_path.clone();
    let parsing_handle = thread::spawn(move || {
        let mut reader_builder = csv::ReaderBuilder::default()
            .has_headers(false)
            .delimiter(b'\t')
            .buffer_capacity(8 * (1 << 10))
            .from_path(&seed_tsv_path)
            .expect("unable to create csv reader");

        for result in reader_builder.deserialize() {
            // Notice that we need to provide a type hint for automatic
            // deserialization.
            let record: Record = result.unwrap();
            match &record.kind {
                RecordKind::BitcoinBlockReceived => {
                    let _ = bitcoin_record_tx.send(Some(record));
                }
                RecordKind::StacksBlockReceived => {
                    let _ = stacks_record_tx.send(Some(record));
                }
                // RecordKind::StacksMicroblockReceived => {
                //     let _ = stacks_record_tx.send(Some(record));
                // },
                _ => {}
            };
        }
        let _ = stacks_record_tx.send(None);
        let _ = bitcoin_record_tx.send(None);
    });

    let stacks_thread_config = config.clone();

    let stacks_processing_handle = thread::spawn(move || {
        let client = redis::Client::open(stacks_thread_config.redis_url.clone()).unwrap();
        let mut con = client.get_connection().unwrap();
        let mut indexer = Indexer::new(stacks_thread_config.indexer_config.clone());
        let mut tip = 0;

        println!("Initial seeding");
        while let Ok(Some(record)) = stacks_record_rx.recv() {
            let (block_identifer, parent_block_identifier) = match &record.kind {
                RecordKind::StacksBlockReceived => {
                    indexer::stacks::standardize_stacks_serialized_block_header(&record.raw_log)
                }
                _ => return Err(()),
            };

            let _: Result<(), redis::RedisError> = con.hset_multiple(
                &format!("stx:{}:{}", block_identifer.index, block_identifer.hash),
                &[
                    ("block_identifer", json!(block_identifer).to_string()),
                    (
                        "parent_block_identifier",
                        json!(parent_block_identifier).to_string(),
                    ),
                    ("blob", record.raw_log),
                ],
            );
            if block_identifer.index > tip {
                tip = block_identifer.index;
                let _: Result<(), redis::RedisError> = con.set(&format!("stx:tip"), tip);
            }
        }

        // Retrieve highest block height stored
        let tip_height: u64 = con
            .get(&format!("stx:tip"))
            .expect("unable to retrieve tip height");
        let chain_tips: Vec<String> = con
            .scan_match(&format!("stx:{}:*", tip_height))
            .expect("unable to retrieve tip height")
            .into_iter()
            .collect();

        println!("Retrieve chain tip");
        // Retrieve all the headers stored at this height (SCAN - expensive)
        let mut selected_tip = BlockIdentifier::default();
        for key in chain_tips.into_iter() {
            let payload: String = con
                .hget(&key, "parent_block_identifier")
                .expect("unable to retrieve tip height");
            selected_tip = serde_json::from_str(&payload).unwrap();
            break;
        }

        println!("Reverse traversal");
        let mut cursor = selected_tip.clone();
        while cursor.index > 1 {
            let key = format!("stx:{}:{}", cursor.index, cursor.hash);
            let parent_block_identifier: BlockIdentifier = {
                let payload: String = con
                    .hget(&key, "parent_block_identifier")
                    .expect("unable to retrieve tip height");
                serde_json::from_str(&payload).unwrap()
            };
            let _: Result<(), redis::RedisError> = con.rename(key, format!("stx:{}", cursor.index));
            let _ = digestion_tx.send(DigestingCommand::DigestSeedBlock(cursor.clone()));
            cursor = parent_block_identifier.clone();
        }

        // Garbage collect
        println!("Garbage collecting");
        let keys_to_prune: Vec<String> = con
            .scan_match("stx:*:*")
            .expect("unable to retrieve prunable entries")
            .into_iter()
            .collect();
        let _: Result<(), redis::RedisError> = con.del(&keys_to_prune);
        Ok(selected_tip)
    });

    let bitcoin_indexer_config = config.clone();

    let bitcoin_processing_handle = thread::spawn(move || {
        let client = redis::Client::open(bitcoin_indexer_config.redis_url.clone()).unwrap();
        let mut con = client.get_connection().unwrap();
        while let Ok(Some(record)) = bitcoin_record_rx.recv() {
            let _: () = match con.set(&format!("btc:{}", record.id), record.raw_log.as_str()) {
                Ok(()) => (),
                Err(_) => return Err(()),
            };
        }
        Ok(BlockIdentifier::default())
    });

    let _ = parsing_handle.join();
    let stacks_chain_tip = match stacks_processing_handle.join().unwrap() {
        Ok(chain_tip) => chain_tip,
        Err(e) => panic!(),
    };
    let bitcoin_chain_tip = match bitcoin_processing_handle.join().unwrap() {
        Ok(chain_tip) => chain_tip,
        Err(e) => panic!(),
    };

    Ok((stacks_chain_tip, bitcoin_chain_tip))
}
