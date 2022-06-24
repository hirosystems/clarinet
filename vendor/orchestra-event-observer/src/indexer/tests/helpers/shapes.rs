use super::super::ChainEventExpectation;
use super::blocks;
use orchestra_types::{StacksBlockData, StacksChainEvent};

pub fn expect_no_chain_update() -> ChainEventExpectation {
    Box::new(move |chain_event_to_check: Option<StacksChainEvent>| {
        assert!(
            match chain_event_to_check {
                None => true,
                _ => false,
            },
            "expected no Chain update, got {:?}",
            chain_event_to_check
        );
    })
}

pub fn expect_chain_updated_with_block(expected_block: StacksBlockData) -> ChainEventExpectation {
    Box::new(move |chain_event_to_check: Option<StacksChainEvent>| {
        assert!(
            match chain_event_to_check {
                Some(StacksChainEvent::ChainUpdatedWithBlock(ref event)) => {
                    assert!(
                        event
                            .new_block
                            .block_identifier
                            .eq(&expected_block.block_identifier),
                        "{} ≠ {}",
                        event.new_block.block_identifier,
                        expected_block.block_identifier
                    );
                    true
                }
                _ => false,
            },
            "expected ChainUpdatedWithBlock, got {:?}",
            chain_event_to_check
        );
    })
}

pub fn expect_chain_updated_with_reorg(
    blocks_to_rollback: Vec<StacksBlockData>,
    blocks_to_apply: Vec<StacksBlockData>,
) -> ChainEventExpectation {
    Box::new(move |chain_event_to_check: Option<StacksChainEvent>| {
        assert!(
            match chain_event_to_check {
                Some(StacksChainEvent::ChainUpdatedWithReorg(ref event)) => {
                    assert_eq!(blocks_to_rollback.len(), event.blocks_to_rollback.len());
                    assert_eq!(blocks_to_apply.len(), event.blocks_to_apply.len());
                    for (expected, (_microblock_trail, block)) in
                        blocks_to_rollback.iter().zip(&event.blocks_to_rollback)
                    {
                        assert!(
                            expected.block_identifier.eq(&block.block_identifier),
                            "{} ≠ {}",
                            expected.block_identifier,
                            block.block_identifier
                        );
                    }
                    for (expected, (_microblock_trail, block)) in
                        blocks_to_apply.iter().zip(&event.blocks_to_apply)
                    {
                        assert!(
                            expected.block_identifier.eq(&block.block_identifier),
                            "{} ≠ {}",
                            expected.block_identifier,
                            block.block_identifier
                        );
                    }
                    true
                }
                _ => false,
            },
            "expected ChainUpdatedWithReorg, got {:?}",
            chain_event_to_check
        );
    })
}

// Test vectors:
// 001 to 020: Stacks anchored blocks received in order
// 021 to 040: Stacks anchored blocks received out of order
// 041 to 060: Stacks anchored blocks received in order + microblocks received in order
// 061 to 080: Stacks anchored blocks received in order + microblocks received out of order
// 081 to 100: Stacks anchored blocks received out of order + microblocks received out of order

/// Vector 001: Generate the following blocks
///
/// A1(1)  -  B1(2)  -  C1(3)
///
pub fn get_vector_001() -> Vec<(StacksBlockData, ChainEventExpectation)> {
    vec![
        (
            blocks::A1(None),
            expect_chain_updated_with_block(blocks::A1(None)),
        ),
        (
            blocks::B1(None),
            expect_chain_updated_with_block(blocks::B1(None)),
        ),
        (
            blocks::C1(None),
            expect_chain_updated_with_block(blocks::C1(None)),
        ),
    ]
}

/// Vector 002: Generate the following blocks
///  
/// A1(1)  -  B1(2)  -  C1(4)
///        \  B2(3)  -  C2(5)
///
pub fn get_vector_002() -> Vec<(StacksBlockData, ChainEventExpectation)> {
    vec![
        (
            blocks::A1(None),
            expect_chain_updated_with_block(blocks::A1(None)),
        ),
        (
            blocks::B1(None),
            expect_chain_updated_with_block(blocks::B1(None)),
        ),
        (
            blocks::B2(None),
            expect_chain_updated_with_reorg(vec![blocks::B1(None)], vec![blocks::B2(None)]),
        ),
        (
            blocks::C1(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B2(None)],
                vec![blocks::B1(None), blocks::C1(None)],
            ),
        ),
        (
            blocks::C2(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B1(None), blocks::C1(None)],
                vec![blocks::B2(None), blocks::C2(None)],
            ),
        ),
    ]
}

/// Vector 003: Generate the following blocks
///  
/// A1(1)  -  B1(2)  -  C1(3)
///        \  B2(4)  -  C2(5)
///
pub fn get_vector_003() -> Vec<(StacksBlockData, ChainEventExpectation)> {
    vec![
        (
            blocks::A1(None),
            expect_chain_updated_with_block(blocks::A1(None)),
        ),
        (
            blocks::B1(None),
            expect_chain_updated_with_block(blocks::B1(None)),
        ),
        (
            blocks::C1(None),
            expect_chain_updated_with_block(blocks::C1(None)),
        ),
        (blocks::B2(None), expect_no_chain_update()),
        (
            blocks::C2(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B1(None), blocks::C1(None)],
                vec![blocks::B2(None), blocks::C2(None)],
            ),
        ),
    ]
}

/// Vector 004: Generate the following blocks
///  
/// A1(1)  -  B1(2)  -  C1(3)  -  D1(6)
///        \  B2(4)  -  C2(5)
///
pub fn get_vector_004() -> Vec<(StacksBlockData, ChainEventExpectation)> {
    vec![
        (
            blocks::A1(None),
            expect_chain_updated_with_block(blocks::A1(None)),
        ),
        (
            blocks::B1(None),
            expect_chain_updated_with_block(blocks::B1(None)),
        ),
        (
            blocks::C1(None),
            expect_chain_updated_with_block(blocks::C1(None)),
        ),
        (blocks::B2(None), expect_no_chain_update()),
        (
            blocks::C2(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B1(None), blocks::C1(None)],
                vec![blocks::B2(None), blocks::C2(None)],
            ),
        ),
        (
            blocks::D1(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B2(None), blocks::C2(None)],
                vec![blocks::B1(None), blocks::C1(None), blocks::D1(None)],
            ),
        ),
    ]
}

/// Vector 005: Generate the following blocks
///  
/// A1(1)  -  B1(2)  -  C1(3)  -  D1(6)  -  E1(7)
///        \  B2(4)  -  C2(5)
///
pub fn get_vector_005() -> Vec<(StacksBlockData, ChainEventExpectation)> {
    vec![
        (
            blocks::A1(None),
            expect_chain_updated_with_block(blocks::A1(None)),
        ),
        (
            blocks::B1(None),
            expect_chain_updated_with_block(blocks::B1(None)),
        ),
        (
            blocks::C1(None),
            expect_chain_updated_with_block(blocks::C1(None)),
        ),
        (blocks::B2(None), expect_no_chain_update()),
        (
            blocks::C2(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B1(None), blocks::C1(None)],
                vec![blocks::B2(None), blocks::C2(None)],
            ),
        ),
        (
            blocks::D1(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B2(None), blocks::C2(None)],
                vec![blocks::B1(None), blocks::C1(None), blocks::D1(None)],
            ),
        ),
        (
            blocks::E1(None),
            expect_chain_updated_with_block(blocks::E1(None)),
        ),
    ]
}

/// Vector 006: Generate the following blocks
///  
/// A1(1)  -  B1(2)  -  C1(3)  -  D1(6)  -  E1(7)  -  F1(8)
///        \  B2(4)  -  C2(5)
///
pub fn get_vector_006() -> Vec<(StacksBlockData, ChainEventExpectation)> {
    vec![
        (
            blocks::A1(None),
            expect_chain_updated_with_block(blocks::A1(None)),
        ),
        (
            blocks::B1(None),
            expect_chain_updated_with_block(blocks::B1(None)),
        ),
        (
            blocks::C1(None),
            expect_chain_updated_with_block(blocks::C1(None)),
        ),
        (blocks::B2(None), expect_no_chain_update()),
        (
            blocks::C2(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B1(None), blocks::C1(None)],
                vec![blocks::B2(None), blocks::C2(None)],
            ),
        ),
        (
            blocks::D1(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B2(None), blocks::C2(None)],
                vec![blocks::B1(None), blocks::C1(None), blocks::D1(None)],
            ),
        ),
        (
            blocks::E1(None),
            expect_chain_updated_with_block(blocks::E1(None)),
        ),
        (
            blocks::F1(None),
            expect_chain_updated_with_block(blocks::F1(None)),
        ),
    ]
}

/// Vector 007: Generate the following blocks
///  
/// A1(1)  -  B1(2)  -  C1(3)  -  D1(6)  -  E1(7)  -  F1(8)  -  G1(9)
///        \  B2(4)  -  C2(5)
///
pub fn get_vector_007() -> Vec<(StacksBlockData, ChainEventExpectation)> {
    vec![
        (
            blocks::A1(None),
            expect_chain_updated_with_block(blocks::A1(None)),
        ),
        (
            blocks::B1(None),
            expect_chain_updated_with_block(blocks::B1(None)),
        ),
        (
            blocks::C1(None),
            expect_chain_updated_with_block(blocks::C1(None)),
        ),
        (blocks::B2(None), expect_no_chain_update()),
        (
            blocks::C2(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B1(None), blocks::C1(None)],
                vec![blocks::B2(None), blocks::C2(None)],
            ),
        ),
        (
            blocks::D1(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B2(None), blocks::C2(None)],
                vec![blocks::B1(None), blocks::C1(None), blocks::D1(None)],
            ),
        ),
        (
            blocks::E1(None),
            expect_chain_updated_with_block(blocks::E1(None)),
        ),
        (
            blocks::F1(None),
            expect_chain_updated_with_block(blocks::F1(None)),
        ),
        (
            blocks::G1(None),
            expect_chain_updated_with_block(blocks::G1(None)),
        ),
    ]
}

/// Vector 008: Generate the following blocks
///  
/// A1(1)  -  B1(2)  -  C1(3)  -  D1(6)  -  E1(7)  -  F1(8)  -  G1(9)  -  H1(10)
///        \  B2(4)  -  C2(5)
///
pub fn get_vector_008() -> Vec<(StacksBlockData, ChainEventExpectation)> {
    vec![
        (
            blocks::A1(None),
            expect_chain_updated_with_block(blocks::A1(None)),
        ),
        (
            blocks::B1(None),
            expect_chain_updated_with_block(blocks::B1(None)),
        ),
        (
            blocks::C1(None),
            expect_chain_updated_with_block(blocks::C1(None)),
        ),
        (blocks::B2(None), expect_no_chain_update()),
        (
            blocks::C2(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B1(None), blocks::C1(None)],
                vec![blocks::B2(None), blocks::C2(None)],
            ),
        ),
        (
            blocks::D1(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B2(None), blocks::C2(None)],
                vec![blocks::B1(None), blocks::C1(None), blocks::D1(None)],
            ),
        ),
        (
            blocks::E1(None),
            expect_chain_updated_with_block(blocks::E1(None)),
        ),
        (
            blocks::F1(None),
            expect_chain_updated_with_block(blocks::F1(None)),
        ),
        (
            blocks::G1(None),
            expect_chain_updated_with_block(blocks::G1(None)),
        ),
        (
            blocks::H1(None),
            expect_chain_updated_with_block(blocks::H1(None)),
        ),
    ]
}

/// Vector 009: Generate the following blocks
///  
/// A1(1)  -  B1(2)  -  C1(3)  -  D1(6)  -  E1(7)  -  F1(8)  -  G1(9)  -  H1(10)  -  I1(11)
///        \  B2(4)  -  C2(5)
///
pub fn get_vector_009() -> Vec<(StacksBlockData, ChainEventExpectation)> {
    vec![
        (
            blocks::A1(None),
            expect_chain_updated_with_block(blocks::A1(None)),
        ),
        (
            blocks::B1(None),
            expect_chain_updated_with_block(blocks::B1(None)),
        ),
        (
            blocks::C1(None),
            expect_chain_updated_with_block(blocks::C1(None)),
        ),
        (blocks::B2(None), expect_no_chain_update()),
        (
            blocks::C2(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B1(None), blocks::C1(None)],
                vec![blocks::B2(None), blocks::C2(None)],
            ),
        ),
        (
            blocks::D1(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B2(None), blocks::C2(None)],
                vec![blocks::B1(None), blocks::C1(None), blocks::D1(None)],
            ),
        ),
        (
            blocks::E1(None),
            expect_chain_updated_with_block(blocks::E1(None)),
        ),
        (
            blocks::F1(None),
            expect_chain_updated_with_block(blocks::F1(None)),
        ),
        (
            blocks::G1(None),
            expect_chain_updated_with_block(blocks::G1(None)),
        ),
        (
            blocks::H1(None),
            expect_chain_updated_with_block(blocks::H1(None)),
        ),
        (
            blocks::I1(None),
            expect_chain_updated_with_block(blocks::I1(None)),
        ),
    ]
}

/// Vector 010: Generate the following blocks
///  
/// A1(1)  -  B1(2)  -  C1(3)  -  D1(6)  -  E1(7)  -  F1(8)  -  G1(9)  -  H1(10) -  I1(11)
///        \  B2(4)  -  C2(5)  -  D2(12)
///
pub fn get_vector_010() -> Vec<(StacksBlockData, ChainEventExpectation)> {
    vec![
        (
            blocks::A1(None),
            expect_chain_updated_with_block(blocks::A1(None)),
        ),
        (
            blocks::B1(None),
            expect_chain_updated_with_block(blocks::B1(None)),
        ),
        (
            blocks::C1(None),
            expect_chain_updated_with_block(blocks::C1(None)),
        ),
        (blocks::B2(None), expect_no_chain_update()),
        (
            blocks::C2(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B1(None), blocks::C1(None)],
                vec![blocks::B2(None), blocks::C2(None)],
            ),
        ),
        (
            blocks::D1(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B2(None), blocks::C2(None)],
                vec![blocks::B1(None), blocks::C1(None), blocks::D1(None)],
            ),
        ),
        (
            blocks::E1(None),
            expect_chain_updated_with_block(blocks::E1(None)),
        ),
        (
            blocks::F1(None),
            expect_chain_updated_with_block(blocks::F1(None)),
        ),
        (
            blocks::G1(None),
            expect_chain_updated_with_block(blocks::G1(None)),
        ),
        (
            blocks::H1(None),
            expect_chain_updated_with_block(blocks::H1(None)),
        ),
        (
            blocks::I1(None),
            expect_chain_updated_with_block(blocks::I1(None)),
        ),
    ]
}

/// Vector 011: Generate the following blocks
///  
/// A1(1)  -  B1(2)  -  C1(3)  -  D1(6)  -  E1(7)  -  F1(8)  -  G1(10)  -  H1(11)  -  I1(12)
///       \                               \ E3(9)
///        \  B2(4)  -  C2(5)
///
pub fn get_vector_011() -> Vec<(StacksBlockData, ChainEventExpectation)> {
    vec![
        (
            blocks::A1(None),
            expect_chain_updated_with_block(blocks::A1(None)),
        ),
        (
            blocks::B1(None),
            expect_chain_updated_with_block(blocks::B1(None)),
        ),
        (
            blocks::C1(None),
            expect_chain_updated_with_block(blocks::C1(None)),
        ),
        (blocks::B2(None), expect_no_chain_update()),
        (
            blocks::C2(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B1(None), blocks::C1(None)],
                vec![blocks::B2(None), blocks::C2(None)],
            ),
        ),
        (
            blocks::D1(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B2(None), blocks::C2(None)],
                vec![blocks::B1(None), blocks::C1(None), blocks::D1(None)],
            ),
        ),
        (
            blocks::E1(None),
            expect_chain_updated_with_block(blocks::E1(None)),
        ),
        (
            blocks::F1(None),
            expect_chain_updated_with_block(blocks::F1(None)),
        ),
        (blocks::E3(None), expect_no_chain_update()),
        (
            blocks::G1(None),
            expect_chain_updated_with_block(blocks::G1(None)),
        ),
        (
            blocks::H1(None),
            expect_chain_updated_with_block(blocks::H1(None)),
        ),
        (
            blocks::I1(None),
            expect_chain_updated_with_block(blocks::I1(None)),
        ),
    ]
}

/// Vector 012: Generate the following blocks
///  
/// A1(1)  -  B1(2)  -  C1(3)  -  D1(6)  -  E1(7)  -  F1(8)  -  G1(10)  -  H1(12)  -  I1(13)
///       \                               \ E3(9)  -  F3(11)
///        \  B2(4)  -  C2(5)
///
pub fn get_vector_012() -> Vec<(StacksBlockData, ChainEventExpectation)> {
    vec![
        (
            blocks::A1(None),
            expect_chain_updated_with_block(blocks::A1(None)),
        ),
        (
            blocks::B1(None),
            expect_chain_updated_with_block(blocks::B1(None)),
        ),
        (
            blocks::C1(None),
            expect_chain_updated_with_block(blocks::C1(None)),
        ),
        (blocks::B2(None), expect_no_chain_update()),
        (
            blocks::C2(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B1(None), blocks::C1(None)],
                vec![blocks::B2(None), blocks::C2(None)],
            ),
        ),
        (
            blocks::D1(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B2(None), blocks::C2(None)],
                vec![blocks::B1(None), blocks::C1(None), blocks::D1(None)],
            ),
        ),
        (
            blocks::E1(None),
            expect_chain_updated_with_block(blocks::E1(None)),
        ),
        (
            blocks::F1(None),
            expect_chain_updated_with_block(blocks::F1(None)),
        ),
        (blocks::E3(None), expect_no_chain_update()),
        (
            blocks::G1(None),
            expect_chain_updated_with_block(blocks::G1(None)),
        ),
        (blocks::F3(None), expect_no_chain_update()),
        (
            blocks::H1(None),
            expect_chain_updated_with_block(blocks::H1(None)),
        ),
        (
            blocks::I1(None),
            expect_chain_updated_with_block(blocks::I1(None)),
        ),
        (blocks::D2(None), expect_no_chain_update()),
    ]
}

/// Vector 013: Generate the following blocks
///  
/// A1(1)  -  B1(2)  -  C1(3)  -  D1(6)  -  E1(7)  -  F1(8)  -  G1(10)  -  H1(12)  -  I1(14)
///       \                               \ E3(9)  -  F3(11) -  G3(13)
///        \  B2(4)  -  C2(5)
///
pub fn get_vector_013() -> Vec<(StacksBlockData, ChainEventExpectation)> {
    vec![
        (
            blocks::A1(None),
            expect_chain_updated_with_block(blocks::A1(None)),
        ),
        (
            blocks::B1(None),
            expect_chain_updated_with_block(blocks::B1(None)),
        ),
        (
            blocks::C1(None),
            expect_chain_updated_with_block(blocks::C1(None)),
        ),
        (blocks::B2(None), expect_no_chain_update()),
        (
            blocks::C2(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B1(None), blocks::C1(None)],
                vec![blocks::B2(None), blocks::C2(None)],
            ),
        ),
        (
            blocks::D1(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B2(None), blocks::C2(None)],
                vec![blocks::B1(None), blocks::C1(None), blocks::D1(None)],
            ),
        ),
        (
            blocks::E1(None),
            expect_chain_updated_with_block(blocks::E1(None)),
        ),
        (
            blocks::F1(None),
            expect_chain_updated_with_block(blocks::F1(None)),
        ),
        (blocks::E3(None), expect_no_chain_update()),
        (
            blocks::G1(None),
            expect_chain_updated_with_block(blocks::G1(None)),
        ),
        (blocks::F3(None), expect_no_chain_update()),
        (
            blocks::H1(None),
            expect_chain_updated_with_block(blocks::H1(None)),
        ),
        (blocks::G3(None), expect_no_chain_update()),
        (
            blocks::I1(None),
            expect_chain_updated_with_block(blocks::I1(None)),
        ),
    ]
}

/// Vector 014: Generate the following blocks
///  
/// A1(1)  -  B1(2)  -  C1(3)  -  D1(6)  -  E1(7)  -  F1(8)  -  G1(10)  -  H1(12)  -  I1(14)
///       \                               \ E3(9)  -  F3(11) -  G3(13)  -  H3(15)
///        \  B2(4)  -  C2(5)
///
pub fn get_vector_014() -> Vec<(StacksBlockData, ChainEventExpectation)> {
    vec![
        (
            blocks::A1(None),
            expect_chain_updated_with_block(blocks::A1(None)),
        ),
        (
            blocks::B1(None),
            expect_chain_updated_with_block(blocks::B1(None)),
        ),
        (
            blocks::C1(None),
            expect_chain_updated_with_block(blocks::C1(None)),
        ),
        (blocks::B2(None), expect_no_chain_update()),
        (
            blocks::C2(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B1(None), blocks::C1(None)],
                vec![blocks::B2(None), blocks::C2(None)],
            ),
        ),
        (
            blocks::D1(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B2(None), blocks::C2(None)],
                vec![blocks::B1(None), blocks::C1(None), blocks::D1(None)],
            ),
        ),
        (
            blocks::E1(None),
            expect_chain_updated_with_block(blocks::E1(None)),
        ),
        (
            blocks::F1(None),
            expect_chain_updated_with_block(blocks::F1(None)),
        ),
        (blocks::E3(None), expect_no_chain_update()),
        (
            blocks::G1(None),
            expect_chain_updated_with_block(blocks::G1(None)),
        ),
        (blocks::F3(None), expect_no_chain_update()),
        (
            blocks::H1(None),
            expect_chain_updated_with_block(blocks::H1(None)),
        ),
        (blocks::G3(None), expect_no_chain_update()),
        (
            blocks::I1(None),
            expect_chain_updated_with_block(blocks::I1(None)),
        ),
        (blocks::H3(None), expect_no_chain_update()),
    ]
}

/// Vector 015: Generate the following blocks
///  
/// A1(1)  -  B1(2)  -  C1(3)  -  D1(6)  -  E1(7)  -  F1(8)  -  G1(10)  -  H1(12)  -  I1(14)
///       \                               \ E3(9)  -  F3(11) -  G3(13)  -  H3(15)  -  I3(16)
///        \  B2(4)  -  C2(5)
///
pub fn get_vector_015() -> Vec<(StacksBlockData, ChainEventExpectation)> {
    vec![
        (
            blocks::A1(None),
            expect_chain_updated_with_block(blocks::A1(None)),
        ),
        (
            blocks::B1(None),
            expect_chain_updated_with_block(blocks::B1(None)),
        ),
        (
            blocks::C1(None),
            expect_chain_updated_with_block(blocks::C1(None)),
        ),
        (blocks::B2(None), expect_no_chain_update()),
        (
            blocks::C2(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B1(None), blocks::C1(None)],
                vec![blocks::B2(None), blocks::C2(None)],
            ),
        ),
        (
            blocks::D1(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B2(None), blocks::C2(None)],
                vec![blocks::B1(None), blocks::C1(None), blocks::D1(None)],
            ),
        ),
        (
            blocks::E1(None),
            expect_chain_updated_with_block(blocks::E1(None)),
        ),
        (
            blocks::F1(None),
            expect_chain_updated_with_block(blocks::F1(None)),
        ),
        (blocks::E3(Some(blocks::D1(None))), expect_no_chain_update()),
        (
            blocks::G1(None),
            expect_chain_updated_with_block(blocks::G1(None)),
        ),
        (blocks::F3(None), expect_no_chain_update()),
        (
            blocks::H1(None),
            expect_chain_updated_with_block(blocks::H1(None)),
        ),
        (blocks::G3(None), expect_no_chain_update()),
        (
            blocks::I1(None),
            expect_chain_updated_with_block(blocks::I1(None)),
        ),
        (blocks::H3(None), expect_no_chain_update()),
        (
            blocks::I3(None),
            expect_chain_updated_with_reorg(
                vec![
                    blocks::E1(None),
                    blocks::F1(None),
                    blocks::G1(None),
                    blocks::H1(None),
                    blocks::I1(None),
                ],
                vec![
                    blocks::E3(Some(blocks::D1(None))),
                    blocks::F3(None),
                    blocks::G3(None),
                    blocks::H3(None),
                    blocks::I3(None),
                ],
            ),
        ),
    ]
}

/// Vector 016: Generate the following blocks
///  
/// A1(1)  -  B1(2)  -  C1(3)  -  D1(6)  -  E1(7)  -  F1(8)  -  G1(10)  -  H1(12)  -  I1(14)
///       \                               \ E3(9)  -  F3(11) -  G3(13)  -  H3(15)  -  I3(16)
///        \  B2(4)  -  C2(5)  -  D2(17)
///
pub fn get_vector_016() -> Vec<(StacksBlockData, ChainEventExpectation)> {
    vec![
        (
            blocks::A1(None),
            expect_chain_updated_with_block(blocks::A1(None)),
        ),
        (
            blocks::B1(None),
            expect_chain_updated_with_block(blocks::B1(None)),
        ),
        (
            blocks::C1(None),
            expect_chain_updated_with_block(blocks::C1(None)),
        ),
        (blocks::B2(None), expect_no_chain_update()),
        (
            blocks::C2(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B1(None), blocks::C1(None)],
                vec![blocks::B2(None), blocks::C2(None)],
            ),
        ),
        (
            blocks::D1(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B2(None), blocks::C2(None)],
                vec![blocks::B1(None), blocks::C1(None), blocks::D1(None)],
            ),
        ),
        (
            blocks::E1(None),
            expect_chain_updated_with_block(blocks::E1(None)),
        ),
        (
            blocks::F1(None),
            expect_chain_updated_with_block(blocks::F1(None)),
        ),
        (blocks::E3(Some(blocks::D1(None))), expect_no_chain_update()),
        (
            blocks::G1(None),
            expect_chain_updated_with_block(blocks::G1(None)),
        ),
        (blocks::F3(None), expect_no_chain_update()),
        (
            blocks::H1(None),
            expect_chain_updated_with_block(blocks::H1(None)),
        ),
        (blocks::G3(None), expect_no_chain_update()),
        (
            blocks::I1(None),
            expect_chain_updated_with_block(blocks::I1(None)),
        ),
        (blocks::H3(None), expect_no_chain_update()),
        (
            blocks::I3(None),
            expect_chain_updated_with_reorg(
                vec![
                    blocks::E1(None),
                    blocks::F1(None),
                    blocks::G1(None),
                    blocks::H1(None),
                    blocks::I1(None),
                ],
                vec![
                    blocks::E3(Some(blocks::D1(None))),
                    blocks::F3(None),
                    blocks::G3(None),
                    blocks::H3(None),
                    blocks::I3(None),
                ],
            ),
        ),
        (blocks::D2(None), expect_no_chain_update()),
    ]
}

/// Vector 017: Generate the following blocks
///  
/// A1(1)  -  B1(2)  -  C1(3)  -  D1(6)  -  E1(7)  -  F1(8)  -  G1(10)
///       \                               \ E3(9)  -  F3(11) -  G3(12)
///        \  B2(4)  -  C2(5)  -  D2(13) -  E2(14)  - F2(15)  - G2(16)
///
pub fn get_vector_017() -> Vec<(StacksBlockData, ChainEventExpectation)> {
    vec![
        (
            blocks::A1(None),
            expect_chain_updated_with_block(blocks::A1(None)),
        ),
        (
            blocks::B1(None),
            expect_chain_updated_with_block(blocks::B1(None)),
        ),
        (
            blocks::C1(None),
            expect_chain_updated_with_block(blocks::C1(None)),
        ),
        (blocks::B2(None), expect_no_chain_update()),
        (
            blocks::C2(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B1(None), blocks::C1(None)],
                vec![blocks::B2(None), blocks::C2(None)],
            ),
        ),
        (
            blocks::D1(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B2(None), blocks::C2(None)],
                vec![blocks::B1(None), blocks::C1(None), blocks::D1(None)],
            ),
        ),
        (
            blocks::E1(None),
            expect_chain_updated_with_block(blocks::E1(None)),
        ),
        (
            blocks::F1(None),
            expect_chain_updated_with_block(blocks::F1(None)),
        ),
        (blocks::E3(Some(blocks::D1(None))), expect_no_chain_update()),
        (
            blocks::G1(None),
            expect_chain_updated_with_block(blocks::G1(None)),
        ),
        (blocks::F3(None), expect_no_chain_update()),
        (
            blocks::G3(None),
            expect_chain_updated_with_reorg(
                vec![
                    blocks::E1(None),
                    blocks::F1(None),
                    blocks::G1(None),
                ],
                vec![
                    blocks::E3(Some(blocks::D1(None))),
                    blocks::F3(None),
                    blocks::G3(None),
                ],
            ),
        ),
        (blocks::D2(None), expect_no_chain_update()),
        (blocks::E2(None), expect_no_chain_update()),
        (blocks::F2(None), expect_no_chain_update()),
        (
            blocks::G2(None),
            expect_no_chain_update(),
        ),
    ]
}



/// Vector 018: Generate the following blocks
///  
/// A1(1)  -  B1(2)  -  C1(3)  -  D1(6)  -  E1(7)  -  F1(8)
///       \                               \ E3(9)  -  F3(10)
///        \  B2(4)  -  C2(5)  -  D2(11) -  E2(12) -  F2(13)  - G2(14)
///
pub fn get_vector_018() -> Vec<(StacksBlockData, ChainEventExpectation)> {
    vec![
        (
            blocks::A1(None),
            expect_chain_updated_with_block(blocks::A1(None)),
        ),
        (
            blocks::B1(None),
            expect_chain_updated_with_block(blocks::B1(None)),
        ),
        (
            blocks::C1(None),
            expect_chain_updated_with_block(blocks::C1(None)),
        ),
        (blocks::B2(None), expect_no_chain_update()),
        (
            blocks::C2(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B1(None), blocks::C1(None)],
                vec![blocks::B2(None), blocks::C2(None)],
            ),
        ),
        (
            blocks::D1(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B2(None), blocks::C2(None)],
                vec![blocks::B1(None), blocks::C1(None), blocks::D1(None)],
            ),
        ),
        (
            blocks::E1(None),
            expect_chain_updated_with_block(blocks::E1(None)),
        ),
        (
            blocks::F1(None),
            expect_chain_updated_with_block(blocks::F1(None)),
        ),
        (blocks::E3(Some(blocks::D1(None))), expect_no_chain_update()),
        (blocks::F3(None), expect_chain_updated_with_reorg(
                vec![
                    blocks::E1(None),
                    blocks::F1(None),
                ],
                vec![
                    blocks::E3(Some(blocks::D1(None))),
                    blocks::F3(None),
                ],
            ),
        ),
        (blocks::D2(None), expect_no_chain_update()),
        (blocks::E2(None), expect_no_chain_update()),
        (blocks::F2(None), expect_no_chain_update()),
        (blocks::G2(None), expect_chain_updated_with_reorg(
            vec![
                blocks::B1(None),
                blocks::C1(None),
                blocks::D1(None),
                blocks::E3(Some(blocks::D1(None))),
                blocks::F3(None),
            ],
            vec![
                blocks::B2(None),
                blocks::C2(None),
                blocks::D2(None),
                blocks::E2(None),
                blocks::F2(None),
                blocks::G2(None),
            ],
        )),        
    ]
}

/// Vector 019: Generate the following blocks
///  
/// A1(1)  -  B1(2)  -  C1(3)  -  D1(6)  -  E1(7)  -  F1(8)  - G1(15)
///       \                               \ E3(9)  -  F3(10)
///        \  B2(4)  -  C2(5)  -  D2(11) -  E2(12) -  F2(13) - G2(14)
///
pub fn get_vector_019() -> Vec<(StacksBlockData, ChainEventExpectation)> {
    vec![
        (
            blocks::A1(None),
            expect_chain_updated_with_block(blocks::A1(None)),
        ),
        (
            blocks::B1(None),
            expect_chain_updated_with_block(blocks::B1(None)),
        ),
        (
            blocks::C1(None),
            expect_chain_updated_with_block(blocks::C1(None)),
        ),
        (blocks::B2(None), expect_no_chain_update()),
        (
            blocks::C2(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B1(None), blocks::C1(None)],
                vec![blocks::B2(None), blocks::C2(None)],
            ),
        ),
        (
            blocks::D1(None),
            expect_chain_updated_with_reorg(
                vec![blocks::B2(None), blocks::C2(None)],
                vec![blocks::B1(None), blocks::C1(None), blocks::D1(None)],
            ),
        ),
        (
            blocks::E1(None),
            expect_chain_updated_with_block(blocks::E1(None)),
        ),
        (
            blocks::F1(None),
            expect_chain_updated_with_block(blocks::F1(None)),
        ),
        (blocks::E3(Some(blocks::D1(None))), expect_no_chain_update()),
        (blocks::F3(None), expect_chain_updated_with_reorg(
                vec![
                    blocks::E1(None),
                    blocks::F1(None),
                ],
                vec![
                    blocks::E3(Some(blocks::D1(None))),
                    blocks::F3(None),
                ],
            ),
        ),
        (blocks::D2(None), expect_no_chain_update()),
        (blocks::E2(None), expect_no_chain_update()),
        (blocks::F2(None), expect_no_chain_update()),
        (blocks::G2(None), expect_chain_updated_with_reorg(
            vec![
                blocks::B1(None),
                blocks::C1(None),
                blocks::D1(None),
                blocks::E3(Some(blocks::D1(None))),
                blocks::F3(None),
            ],
            vec![
                blocks::B2(None),
                blocks::C2(None),
                blocks::D2(None),
                blocks::E2(None),
                blocks::F2(None),
                blocks::G2(None),
            ],
        )),     
        (blocks::G1(None), expect_no_chain_update()),
    ]
}

/// Vector 020: Generate the following blocks
///  
/// A1(1)  -  B1(2)  -  C1(3)  -  D1(6)  -  E1(7)  -  F1(8)  -  G1(10)  -  H1(12)  -  I1(14) - J1(24)
///       \                               \ E3(9)  -  F3(11) -  G3(13)  -  H3(15)  -  I3(16) - J3(25)
///        \  B2(4)  -  C2(5)  -  D2(17) -  E2(18) -  F2(19)  - G2(20)  -  H2(21)  -  I2(22) - J2(23)
///
pub fn get_vector_020() -> Vec<StacksBlockData> {
    vec![
        blocks::A1(None),
        blocks::B1(None),
        blocks::C1(None),
        blocks::B2(None),
        blocks::C2(None),
        blocks::D1(None),
        blocks::E1(None),
        blocks::F1(None),
        blocks::E3(None),
        blocks::G1(None),
        blocks::F3(None),
        blocks::H1(None),
        blocks::G3(None),
        blocks::I1(None),
        blocks::H3(None),
        blocks::I3(None),
        blocks::D2(None),
        blocks::E2(None),
        blocks::F2(None),
        blocks::G2(None),
        blocks::H2(None),
        blocks::I2(None),
        blocks::J2(None),
        blocks::J3(None),
    ]
}

/// Vector 021: Generate the following blocks
///  
/// A1(1)  -  B1(2)  -  C1(3)  -  D1(6)  -  E1(7)  -  F1(8)  -  G1(10)  -  H1(12)  -  I1(14) - J1(24)
///       \                               \ E3(9)  -  F3(11) -  G3(13)  -  H3(15)  -  I3(16) - J3(25)
///        \  B2(4)  -  C2(5)  -  D2(17) -  E2(18)  - F2(19)  - G2(20)  -  H2(21)  -  I2(22) - J2(23) - K2(26)
///
pub fn get_vector_021() -> Vec<StacksBlockData> {
    vec![
        blocks::A1(None),
        blocks::B1(None),
        blocks::C1(None),
        blocks::B2(None),
        blocks::C2(None),
        blocks::D1(None),
        blocks::E1(None),
        blocks::F1(None),
        blocks::E3(None),
        blocks::G1(None),
        blocks::F3(None),
        blocks::H1(None),
        blocks::G3(None),
        blocks::I1(None),
        blocks::H3(None),
        blocks::I3(None),
        blocks::D2(None),
        blocks::E2(None),
        blocks::F2(None),
        blocks::G2(None),
        blocks::H2(None),
        blocks::I2(None),
        blocks::J2(None),
        blocks::K2(None),
    ]
}

/// Vector 022: Generate the following blocks
///
/// A1(1)  -  B1(3)  -  C1(2)
///
pub fn get_vector_022() -> Vec<StacksBlockData> {
    vec![]
}

/// Vector 023: Generate the following blocks
///  
/// A1(1)  -  B1(2)  -  C1(3)
///        \  B2(5)  -  C2(4)
///
pub fn get_vector_023() -> Vec<StacksBlockData> {
    vec![]
}

/// Vector 024: Generate the following blocks
///  
/// A1(1)  -  B1(5)  -  C1(3)
///        \  B2(2)  -  C2(4)
///
pub fn get_vector_024() -> Vec<StacksBlockData> {
    vec![]
}

/// Vector 025: Generate the following blocks
///  
/// A1(1)  -  B1(5)  -  C1(4)  -  D1(6)
///        \  B2(2)  -  C2(3)
///
pub fn get_vector_025() -> Vec<StacksBlockData> {
    vec![]
}

/// Vector 026: Generate the following blocks
///  
/// A1(1)  -  B1(2)  -  C1(4)  -  D1(5)  -  E1(6)
///        \  B2(3)  -  C2(7)
///
pub fn get_vector_026() -> Vec<StacksBlockData> {
    vec![]
}

/// Vector 027: Generate the following blocks
///  
/// A1(1)  -  B1(2)  -  C1(3)  -  D1(8)  -  E1(7)  -  F1(6)
///        \  B2(4)  -  C2(5)
///
pub fn get_vector_027() -> Vec<StacksBlockData> {
    vec![]
}

/// Vector 028: Generate the following blocks
///  
/// A1(1)  -  B1(2)  -  C1(4)  -  D1(9)  -  E1(8)  -  F1(7)  -  G1(6)
///        \  B2(5)  -  C2(5)
///
pub fn get_vector_028() -> Vec<StacksBlockData> {
    vec![]
}

/// Vector 029: Generate the following blocks
///  
/// A1(1)  -  B1(9)  -  C1(7)  -  D1(3)  -  E1(7)  -  F1(2)  -  G1(5)  -  H1(4)
///        \  B2(8)  -  C2(10)
///
pub fn get_vector_029() -> Vec<StacksBlockData> {
    vec![]
}

/// Vector 030: Generate the following blocks
///  
/// A1(1)  -  B1(7)  -  C1(6)  -  D1(9)  -  E1(10)  -  F1(2)  -  G1(3)  -  H1(4)  -  I1(11)
///        \  B2(8)  -  C2(5)
///
pub fn get_vector_030() -> Vec<StacksBlockData> {
    vec![]
}

/// Vector 031: Generate the following blocks
///  
/// A1(1)  -  B1(9)  -  C1(8)  -  D1(7)  -  E1(6)  -  F1(5)  -  G1(4)  -  H1(3)  -  I1(2)
///        \  B2(11)  -  C2(10)
///
pub fn get_vector_031() -> Vec<StacksBlockData> {
    vec![]
}

/// Vector 032: Generate the following blocks
///  
/// A1(1)  -  B1(8)  -  C1(7)  -  D1(5)  -  E1(4)  -  F1(6)  -  G1(10)  -  H1(11)  -  I1(9)
///       \                               \ E3(2)
///        \  B2(3)  -  C2(5)
///
pub fn get_vector_032() -> Vec<StacksBlockData> {
    vec![]
}

/// Vector 033: Generate the following blocks
///  
/// A1(1)  -  B1(3)  -  C1(5)  -  D1(7)  -  E1(8)  -  F1(10)  -  G1(13)  -  H1(12)  -  I1(11)
///       \                               \ E3(2)  -  F3(9)
///        \  B2(4)  -  C2(6)
///
pub fn get_vector_033() -> Vec<StacksBlockData> {
    vec![]
}

/// Vector 034: Generate the following blocks
///  
/// A1(1)  -  B1(12)  -  C1(13)  -  D1(6)  -  E1(9)  -  F1(6)  -  G1(5)  -  H1(4)  -  I1(2)
///       \                                 \ E3(10) -  F3(7)  -  G3(3)
///        \  B2(11)  -  C2(8)
///
pub fn get_vector_034() -> Vec<StacksBlockData> {
    vec![]
}

/// Vector 035: Generate the following blocks
///  
/// A1(1)  -  B1(12)  -  C1(14)  -  D1(7)  -  E1(2)  -  F1(4)  -  G1(6)  -  H1(9)  -  I1(13)
///       \                                 \ E3(5)  -  F3(3)  -  G3(8)  -  H3(15)
///        \  B2(10)  -  C2(11)
///
pub fn get_vector_035() -> Vec<StacksBlockData> {
    vec![]
}

/// Vector 036: Generate the following blocks
///  
/// A1(1)  -  B1(2)  -  C1(3)  -  D1(6)  -  E1(7)  -  F1(8)  -  G1(10)  -  H1(12)  -  I1(14)
///       \                               \ E3(9)  -  F3(11) -  G3(13)  -  H3(15)  -  I3(16)
///        \  B2(4)  -  C2(5)
///
pub fn get_vector_036() -> Vec<StacksBlockData> {
    vec![]
}

/// Vector 037: Generate the following blocks
///  
/// A1(1)  -  B1(13)  -  C1(10)  -  D1(16) -  E1(4)  -  F1(6)  -  G1(15)  -  H1(12)  -  I1(3)
///       \                                \  E3(5)  -  F3(17) -  G3(14)  -  H3(9)   -  I3(8)
///        \  B2(11)  -  C2(7)  -  D2(2)
///
pub fn get_vector_037() -> Vec<StacksBlockData> {
    vec![]
}

/// Vector 038: Generate the following blocks
///  
/// A1(1)  -  B1(7)  -  C1(6)  -  D1(9)  -  E1(4)  -  F1(3)  -  G1(16)  -  H1(14)  -  I1(10)
///       \                               \ E3(2)  -  F3(17) -  G3(19)  -  H3(13)  -  I3(11)
///        \  B2(8)  -  C2(5)  -  D2(22) -  E2(21)  - F2(20)  - G2(18)  -  H2(15)  -  I2(12)
///
pub fn get_vector_038() -> Vec<StacksBlockData> {
    vec![]
}

/// Vector 039: Generate the following blocks
///  
/// A1(1)  -  B1(9)  -  C1(10)  -  D1(11)  -  E1(15)  -  F1(14) -  G1(10)  -  H1(12)  -  I1(2)
///       \                                \  E3(8)   -  F3(7)  -  G3(6)   -  H3(5)   -  I3(16)
///        \  B2(12)  -  C2(13)  -  D2(17) -  E2(18)  - F2(19)  -  G2(20)  -  H2(21)  -  I2(4) - J2(3)
///
pub fn get_vector_039() -> Vec<StacksBlockData> {
    vec![]
}

/// Vector 040: Generate the following blocks
///  
/// A1(1)  -  B1(21)  -  C1(19) -  D1(20) - E1(8)  -  F1(5)  -  G1(23)  -  H1(22) -  I1(15) - J1(2)
///       \                               \ E3(3)  -  F3(11) -  G3(16)  -  H3(14) -  I3(12)
///        \  B2(4)  -  C2(17) -  D2(18) -  E2(7)  -  F2(6)  -  G2(24)  -  H2(9)  -  I2(10) - J2(13)
///
pub fn get_vector_040() -> Vec<StacksBlockData> {
    vec![]
}

/// Vector 041: Generate the following blocks
///  
/// A1(1)  -  B1(24)  -  C1(22)  -  D1(20)  -  E1(17) -  F1(14)  -  G1(11)  -  H1(6)  -  I1(3) - J1(2)
///       \                                 \  E3(18) -  F3(15)  -  G3(12)  -  H3(9)  -  I3(5) - J3(4)
///        \  B2(25)  -  C2(23)  -  D2(21)  -  E2(19) -  F2(16)  -  G2(13)  -  H2(10)  - I2(8) - J2(7)
///
pub fn get_vector_041() -> Vec<StacksBlockData> {
    vec![]
}
