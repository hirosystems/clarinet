use super::DigestingCommand;
use crate::config::Config;
use orchestra_event_observer::indexer;
use orchestra_event_observer::indexer::Indexer;
use redis;
use redis::Commands;
use std::cmp::Ordering;
use std::{collections::BinaryHeap, process, sync::mpsc::Receiver};

const JOB_DIGEST_BLOCK_SEED_PRIORITY: u64 = 100_000;
const JOB_TERMINATION_PRIORITY: u64 = 100_000_000;

#[derive(Debug, Clone, Eq, PartialEq)]
struct Job {
    pub command: DigestingCommand,
    pub priority: u64,
}

pub fn start(command_rx: Receiver<DigestingCommand>, config: &Config) {
    // let mut bit_vector
    let mut job_queue: BinaryHeap<Job> = BinaryHeap::new();
    let client = redis::Client::open(config.redis_url.clone()).unwrap();
    let mut indexer = Indexer::new(config.indexer_config.clone());

    let mut con = client.get_connection().unwrap();
    loop {
        while let Some(job) = job_queue.pop() {
            match &job.command {
                DigestingCommand::DigestSeedBlock(block_identifier) => {
                    let key = format!("stx:{}", block_identifier.index);
                    let payload: String = con
                        .hget(&key, "blob")
                        .expect("unable to retrieve tip height");
                    let block_data = indexer::stacks::standardize_stacks_serialized_block(
                        &indexer.config,
                        &payload,
                        &mut indexer.stacks_context,
                    );
                    let _: Result<(), redis::RedisError> = con.hset_multiple(
                        &key,
                        &[
                            ("transactions", json!(block_data.transactions).to_string()),
                            ("metadata", json!(block_data.metadata).to_string()),
                        ],
                    );
                }
                DigestingCommand::Terminate => {
                    println!("Terminating");
                    return;
                }
            }

            if let Ok(new_command) = command_rx.try_recv() {
                job_queue.push(new_job(new_command));
            };
        }
        let command = match command_rx.recv() {
            Ok(command) => command,
            Err(e) => {
                println!("block digestion halted.");
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
        DigestingCommand::Terminate => {
            println!("Inserting Terminate");
            Job {
                priority: JOB_TERMINATION_PRIORITY,
                command: DigestingCommand::Terminate,
            }
        }
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
