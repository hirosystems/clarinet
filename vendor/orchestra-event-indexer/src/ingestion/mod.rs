use std::{sync::mpsc::channel, thread};
use serde::Deserialize;
use redis;
use redis::Commands;

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

pub fn start_ingesting(events_logs_csv_path: String) {
    let (stacks_record_tx, stacks_record_rx) = channel();
    let (bitcoin_record_tx, bitcoin_record_rx) = channel();

    let parsing_handle = thread::spawn(move || {
        let mut reader_builder = csv::ReaderBuilder::default()
            .has_headers(false)
            .delimiter(b'\t')
            .buffer_capacity(8 * (1 << 10))
            .from_path(events_logs_csv_path)
            .expect("unable to create csv reader");
        
        for result in reader_builder.deserialize() {
            // Notice that we need to provide a type hint for automatic
            // deserialization.
            let record: Record = result.unwrap();
            match &record.kind {
                RecordKind::BitcoinBlockReceived => {
                    let _ = stacks_record_tx.send(Some(record));
                },
                RecordKind::StacksBlockReceived | RecordKind::StacksMicroblockReceived => {
                    let _ = bitcoin_record_tx.send(Some(record));
                },
                _ => {}
            };
        }
        let _ = stacks_record_tx.send(None);
        let _ = bitcoin_record_tx.send(None);
    });

    let stacks_processing_handle = thread::spawn(move || {
        let client = redis::Client::open("redis://127.0.0.1/").unwrap();
        let mut con = client.get_connection().unwrap();
        while let Ok(Some(record)) = stacks_record_rx.recv() {
            let _: () = con.set(&format!("stacks::{}", record.id), record.raw_log.as_str()).unwrap();
        }
    });

    let bitcoin_processing_handle = thread::spawn(move || {
        let client = redis::Client::open("redis://127.0.0.1/").unwrap();
        let mut con = client.get_connection().unwrap();
        while let Ok(Some(record)) = bitcoin_record_rx.recv() {
            let _: () = con.set(&format!("bitcoin::{}", record.id), record.raw_log.as_str()).unwrap();
        }
    });

    let _ = parsing_handle.join();
    let _ = stacks_processing_handle.join();
    let _ = bitcoin_processing_handle.join();

    println!("File processed");
}