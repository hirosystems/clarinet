pub mod helpers;
use self::helpers::BlockEvent;
use super::{BitcoinBlockPool, StacksBlockPool};
use chainhook_types::{BitcoinBlockData, BitcoinChainEvent, StacksChainEvent};

pub type StacksChainEventExpectation = Box<dyn Fn(Option<StacksChainEvent>) -> ()>;

pub fn process_stacks_blocks_and_check_expectations(
    steps: Vec<(BlockEvent, StacksChainEventExpectation)>,
) {
    let mut blocks_processor = StacksBlockPool::new();
    for (block_event, check_chain_event_expectations) in steps.into_iter() {
        match block_event {
            BlockEvent::Block(block) => {
                let chain_event = blocks_processor.process_block(block).unwrap();
                check_chain_event_expectations(chain_event);
            }
            BlockEvent::Microblock(microblock) => {
                let chain_event = blocks_processor
                    .process_microblocks(vec![microblock])
                    .unwrap();
                check_chain_event_expectations(chain_event);
            }
        }
    }
}

pub type BitcoinChainEventExpectation = Box<dyn Fn(Option<BitcoinChainEvent>) -> ()>;

pub fn process_bitcoin_blocks_and_check_expectations(
    steps: Vec<(BitcoinBlockData, BitcoinChainEventExpectation)>,
) {
    let mut blocks_processor = BitcoinBlockPool::new();
    for (block, check_chain_event_expectations) in steps.into_iter() {
        let chain_event = blocks_processor.process_block(block).unwrap();
        check_chain_event_expectations(chain_event);
    }
}
