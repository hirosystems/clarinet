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

#[test]
fn test_vector_015() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_015());
}

#[test]
fn test_vector_016() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_016());
}

#[test]
fn test_vector_017() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_017());
}

#[test]
fn test_vector_018() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_018());
}

#[test]
fn test_vector_019() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_019());
}

#[test]
fn test_vector_020() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_020());
}

#[test]
fn test_vector_021() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_021());
}

#[test]
fn test_vector_022() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_022());
}

#[test]
fn test_vector_023() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_023());
}

#[test]
fn test_vector_024() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_024());
}

#[test]
fn test_vector_025() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_025());
}

#[test]
fn test_vector_026() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_026());
}

#[test]
fn test_vector_027() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_027());
}

#[test]
fn test_vector_028() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_028());
}

#[test]
fn test_vector_029() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_029());
}

#[test]
fn test_vector_030() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_030());
}

#[test]
fn test_vector_031() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_031());
}

#[test]
fn test_vector_032() {
    process_blocks_and_check_expectations(helpers::shapes::get_vector_031());
}
