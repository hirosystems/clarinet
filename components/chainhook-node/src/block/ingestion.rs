use crate::config::Config;
use chainhook_event_observer::indexer::{self, Indexer};
use chainhook_types::BlockIdentifier;
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

    let seed_tsv_path = config.expected_local_tsv_file().clone();
    info!("Initialize storage with events {}", seed_tsv_path.display());
    let parsing_handle = thread::spawn(move || {
        let mut reader_builder = csv::ReaderBuilder::default()
            .has_headers(false)
            .delimiter(b'\t')
            .buffer_capacity(8 * (1 << 10))
            .from_path(&seed_tsv_path)
            .expect("unable to create csv reader");

        // TODO
        // let mut record = csv::StringRecord::new();
        // let mut rdr = Reader::from_reader(data.as_bytes());
        // let mut record = StringRecord::new();
        // if rdr.read_record(&mut record)? {
        //     assert_eq!(record, vec!["Boston", "United States", "4628910"]);
        //     Ok(())
        // } else {
        //     Err(From::from("expected at least one record but got none"))
        // }

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
        let redis_config = stacks_thread_config.expected_redis_config();

        let client = redis::Client::open(redis_config.uri.clone()).unwrap();
        let mut con = match client.get_connection() {
            Ok(con) => con,
            Err(message) => {
                return Err(format!("Redis: {}", message.to_string()));
            }
        };
        let _indexer = Indexer::new(stacks_thread_config.network.clone());

        // Retrieve the former highest block height stored
        let former_tip_height: u64 = con.get(&format!("stx:tip")).unwrap_or(0);

        let mut tip = 0;

        while let Ok(Some(record)) = stacks_record_rx.recv() {
            let (block_identifier, parent_block_identifier) = match &record.kind {
                RecordKind::StacksBlockReceived => {
                    match indexer::stacks::standardize_stacks_serialized_block_header(
                        &record.raw_log,
                    ) {
                        Ok(data) => data,
                        Err(e) => {
                            error!("{e}");
                            continue;
                        }
                    }
                }
                _ => unreachable!(),
            };

            let _: Result<(), redis::RedisError> = con.hset_multiple(
                &format!("stx:{}:{}", block_identifier.index, block_identifier.hash),
                &[
                    ("block_identifier", json!(block_identifier).to_string()),
                    (
                        "parent_block_identifier",
                        json!(parent_block_identifier).to_string(),
                    ),
                    ("blob", record.raw_log),
                ],
            );
            if block_identifier.index > tip {
                tip = block_identifier.index;
                let _: Result<(), redis::RedisError> = con.set(&format!("stx:tip"), tip);
            }
        }

        // Retrieve highest block height stored
        let tip_height: u64 = con
            .get(&format!("stx:tip"))
            .expect("unable to retrieve tip height");

        if former_tip_height == tip_height {
            // No new block to ingest, we will make sure that we have all the blocks,
            // and succesfully terminate this routine.
            let _ = digestion_tx.send(DigestingCommand::GarbageCollect);
            // Retrieve block identifier
            let key = format!("stx:{}", tip_height);
            let block_identifier: BlockIdentifier = {
                let payload: String = con
                    .hget(&key, "block_identifier")
                    .expect("unable to retrieve tip height");
                serde_json::from_str(&payload).unwrap()
            };
            info!("Local storage seeded, no new block to process");
            return Ok(block_identifier);
        }

        let chain_tips: Vec<String> = con
            .scan_match(&format!("stx:{}:*", tip_height))
            .expect("unable to retrieve tip height")
            .into_iter()
            .collect();

        info!(
            "Start processing canonical Stacks blocks from chain tip #{}",
            tip_height
        );
        // Retrieve all the headers stored at this height (SCAN - expensive)
        let mut selected_tip = BlockIdentifier::default();
        for key in chain_tips.into_iter() {
            let payload: String = con
                .hget(&key, "block_identifier")
                .expect("unable to retrieve tip height");
            selected_tip = serde_json::from_str(&payload).unwrap();
            break;
        }

        let mut cursor = selected_tip.clone();
        while cursor.index > 0 {
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
        info!("{} Stacks blocks queued for processing", tip_height);

        let _ = digestion_tx.send(DigestingCommand::GarbageCollect);
        Ok(selected_tip)
    });

    let bitcoin_indexer_config = config.clone();

    let bitcoin_processing_handle = thread::spawn(move || {
        let redis_config = bitcoin_indexer_config.expected_redis_config();

        let client = redis::Client::open(redis_config.uri.clone()).unwrap();
        let mut con = match client.get_connection() {
            Ok(con) => con,
            Err(message) => {
                return Err(format!("Redis: {}", message.to_string()));
            }
        };
        while let Ok(Some(record)) = bitcoin_record_rx.recv() {
            let _: () = match con.set(&format!("btc:{}", record.id), record.raw_log.as_str()) {
                Ok(()) => (),
                Err(e) => return Err(e.to_string()),
            };
        }
        Ok(BlockIdentifier::default())
    });

    let _ = parsing_handle.join();
    let stacks_chain_tip = stacks_processing_handle.join().unwrap()?;
    let bitcoin_chain_tip = bitcoin_processing_handle.join().unwrap()?;

    Ok((stacks_chain_tip, bitcoin_chain_tip))
}
