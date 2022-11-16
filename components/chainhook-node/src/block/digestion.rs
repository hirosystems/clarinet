use super::DigestingCommand;
use crate::config::Config;
use chainhook_event_observer::indexer;
use chainhook_event_observer::indexer::Indexer;
use redis::Commands;
use std::cmp::Ordering;
use std::{collections::BinaryHeap, process, sync::mpsc::Receiver};

const JOB_TERMINATION_HIGH_PRIORITY: u64 = 100_000_000;
const JOB_DIGEST_BLOCK_SEED_PRIORITY: u64 = 100_000;
const JOB_LOW_PRIORITY: u64 = 10;
const JOB_TERMINATION_LOW_PRIORITY: u64 = 1;

#[derive(Debug, Clone, Eq, PartialEq)]
struct Job {
    pub command: DigestingCommand,
    pub priority: u64,
}

pub fn start(command_rx: Receiver<DigestingCommand>, config: &Config) -> Result<(), String> {
    // let mut bit_vector
    let mut job_queue: BinaryHeap<Job> = BinaryHeap::new();
    let redis_config = config.expected_redis_config();
    let client = redis::Client::open(redis_config.uri.clone()).unwrap();
    let mut indexer = Indexer::new(config.network.clone());

    let mut con = match client.get_connection() {
        Ok(con) => con,
        Err(message) => {
            return Err(format!("Redis: {}", message.to_string()));
        }
    };
    let mut block_digested = 0;
    loop {
        while let Some(job) = job_queue.pop() {
            match &job.command {
                DigestingCommand::DigestSeedBlock(block_identifier) => {
                    let key = format!("stx:{}", block_identifier.index);
                    let payload: String = con
                        .hget(&key, "blob")
                        .expect("unable to retrieve tip height");
                    let block_data = match indexer::stacks::standardize_stacks_serialized_block(
                        &indexer.config,
                        &payload,
                        &mut indexer.stacks_context,
                    ) {
                        Ok(block) => block,
                        Err(e) => {
                            error!("{e}");
                            continue;
                        }
                    };
                    let _: Result<(), redis::RedisError> = con.hset_multiple(
                        &key,
                        &[
                            ("transactions", json!(block_data.transactions).to_string()),
                            ("metadata", json!(block_data.metadata).to_string()),
                            ("timestamp", json!(block_data.timestamp).to_string()),
                        ],
                    );
                    if block_digested > 0 && job_queue.is_empty() {
                        info!("Seeding completed - {} block processed", block_digested + 1);
                    }
                    block_digested += 1;
                }
                DigestingCommand::GarbageCollect => {
                    let keys_to_prune: Vec<String> = con
                        .scan_match("stx:*:*")
                        .expect("unable to retrieve prunable entries")
                        .into_iter()
                        .collect();
                    let _: Result<(), redis::RedisError> = con.del(&keys_to_prune);
                    debug!(
                        "{} Stacks orphaned blocks removed from storage",
                        keys_to_prune.len()
                    );
                    info!("Initial ingestion succesfully performed")
                }
                DigestingCommand::Terminate | DigestingCommand::Kill => {
                    info!("Terminating");
                    return Ok(());
                }
            }
            while let Ok(new_command) = command_rx.try_recv() {
                job_queue.push(new_job(new_command));
            }
        }
        let command = match command_rx.recv() {
            Ok(command) => command,
            Err(e) => {
                error!("block digestion channel broken {:?}", e);
                process::exit(1);
            }
        };
        job_queue.push(new_job(command));
    }
}

fn new_job(command: DigestingCommand) -> Job {
    match command {
        DigestingCommand::DigestSeedBlock(block_identifier) => Job {
            priority: JOB_DIGEST_BLOCK_SEED_PRIORITY + block_identifier.index,
            command: DigestingCommand::DigestSeedBlock(block_identifier),
        },
        DigestingCommand::GarbageCollect => Job {
            priority: JOB_LOW_PRIORITY,
            command: DigestingCommand::GarbageCollect,
        },
        DigestingCommand::Terminate => Job {
            priority: JOB_TERMINATION_LOW_PRIORITY,
            command: DigestingCommand::Terminate,
        },
        DigestingCommand::Kill => Job {
            priority: JOB_TERMINATION_HIGH_PRIORITY,
            command: DigestingCommand::Kill,
        },
    }
}

impl Ord for Job {
    fn cmp(&self, other: &Job) -> Ordering {
        self.priority.cmp(&other.priority)
    }
}

impl PartialOrd for Job {
    fn partial_cmp(&self, other: &Job) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
