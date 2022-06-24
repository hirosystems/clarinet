pub mod helpers;

use orchestra_types::{StacksBlockData, StacksChainEvent};

type ChainEventExpectation = Box<dyn Fn(Option<StacksChainEvent>) -> ()>;

use super::UnconfirmedBlocksProcessor;

fn process_blocks_and_check_expectations(steps: Vec<(StacksBlockData, ChainEventExpectation)>) {
    let mut blocks_processor = UnconfirmedBlocksProcessor::new();
    for (block, check_chain_event_expectations) in steps {
        let chain_event = blocks_processor.process_block(&block);
        check_chain_event_expectations(chain_event);
    }
}

#[test]
fn test_vector_001() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_001());
}

#[test]
fn test_vector_002() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_002());
}

#[test]
fn test_vector_003() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_003());
}
