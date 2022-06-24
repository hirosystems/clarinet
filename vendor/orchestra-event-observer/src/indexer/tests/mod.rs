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

#[test]
fn test_vector_004() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_004());
}

#[test]
fn test_vector_005() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_005());
}

#[test]
fn test_vector_006() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_006());
}

#[test]
fn test_vector_007() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_007());
}

#[test]
fn test_vector_008() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_008());
}

#[test]
fn test_vector_009() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_009());
}

#[test]
fn test_vector_010() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_010());
}

#[test]
fn test_vector_011() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_011());
}

#[test]
fn test_vector_012() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_012());
}

#[test]
fn test_vector_013() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_013());
}

#[test]
fn test_vector_014() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_014());
}
