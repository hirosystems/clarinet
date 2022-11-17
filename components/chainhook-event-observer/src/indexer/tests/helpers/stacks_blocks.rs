use super::BlockEvent;
use chainhook_types::{
    BlockIdentifier, StacksBlockData, StacksBlockMetadata, StacksTransactionData,
};
use clarity_repl::clarity::util::hash::to_hex;

pub fn generate_test_stacks_block(
    fork_id: u8,
    block_height: u64,
    transactions: Vec<StacksTransactionData>,
    parent: Option<BlockEvent>,
) -> BlockEvent {
    let mut hash = vec![
        fork_id, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    let (parent_block_identifier, confirm_microblock_identifier) = match parent {
        Some(BlockEvent::Block(parent)) => {
            assert_eq!(parent.block_identifier.index, block_height - 1);
            (parent.block_identifier.clone(), None)
        }
        Some(BlockEvent::Microblock(microblock_parent)) => {
            assert_eq!(
                microblock_parent.metadata.anchor_block_identifier.index,
                block_height - 1
            );
            (
                microblock_parent.metadata.anchor_block_identifier.clone(),
                Some(microblock_parent.block_identifier.clone()),
            )
        }
        _ => {
            let mut parent_hash = if (block_height - 1) == 1 {
                vec![
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                ]
            } else {
                vec![
                    fork_id, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                ]
            };
            parent_hash.append(&mut (block_height - 1).to_be_bytes().to_vec());
            (
                BlockIdentifier {
                    index: block_height - 1,
                    hash: to_hex(&parent_hash[..]),
                },
                None,
            )
        }
    };
    hash.append(&mut block_height.to_be_bytes().to_vec());
    BlockEvent::Block(StacksBlockData {
        block_identifier: BlockIdentifier {
            index: block_height,
            hash: to_hex(&hash[..]),
        },
        parent_block_identifier,
        timestamp: 0,
        transactions,
        metadata: StacksBlockMetadata {
            bitcoin_anchor_block_identifier: BlockIdentifier {
                index: block_height - 1,
                hash: format!(""),
            },
            pox_cycle_index: 1,
            pox_cycle_position: block_height.try_into().unwrap(),
            pox_cycle_length: 100,
            confirm_microblock_identifier,
        },
    })
}

pub fn A1(parent: Option<BlockEvent>) -> BlockEvent {
    generate_test_stacks_block(0, 1, vec![], parent)
}

pub fn B1(parent: Option<BlockEvent>) -> BlockEvent {
    generate_test_stacks_block(1, 2, vec![], parent)
}

pub fn B2(parent: Option<BlockEvent>) -> BlockEvent {
    generate_test_stacks_block(2, 2, vec![], parent)
}

pub fn C1(parent: Option<BlockEvent>) -> BlockEvent {
    generate_test_stacks_block(1, 3, vec![], parent)
}

pub fn C2(parent: Option<BlockEvent>) -> BlockEvent {
    generate_test_stacks_block(2, 3, vec![], parent)
}

pub fn D1(parent: Option<BlockEvent>) -> BlockEvent {
    generate_test_stacks_block(1, 4, vec![], parent)
}

pub fn D2(parent: Option<BlockEvent>) -> BlockEvent {
    generate_test_stacks_block(2, 4, vec![], parent)
}

pub fn E1(parent: Option<BlockEvent>) -> BlockEvent {
    generate_test_stacks_block(1, 5, vec![], parent)
}

pub fn E2(parent: Option<BlockEvent>) -> BlockEvent {
    generate_test_stacks_block(2, 5, vec![], parent)
}

pub fn B3(parent: Option<BlockEvent>) -> BlockEvent {
    generate_test_stacks_block(3, 2, vec![], parent)
}

pub fn C3(parent: Option<BlockEvent>) -> BlockEvent {
    generate_test_stacks_block(3, 3, vec![], parent)
}

pub fn D3(parent: Option<BlockEvent>) -> BlockEvent {
    generate_test_stacks_block(3, 4, vec![], parent)
}

pub fn E3(parent: Option<BlockEvent>) -> BlockEvent {
    generate_test_stacks_block(3, 5, vec![], parent)
}

pub fn F1(parent: Option<BlockEvent>) -> BlockEvent {
    generate_test_stacks_block(1, 6, vec![], parent)
}

pub fn F2(parent: Option<BlockEvent>) -> BlockEvent {
    generate_test_stacks_block(2, 6, vec![], parent)
}

pub fn F3(parent: Option<BlockEvent>) -> BlockEvent {
    generate_test_stacks_block(3, 6, vec![], parent)
}

pub fn G1(parent: Option<BlockEvent>) -> BlockEvent {
    generate_test_stacks_block(1, 7, vec![], parent)
}

pub fn G2(parent: Option<BlockEvent>) -> BlockEvent {
    generate_test_stacks_block(2, 7, vec![], parent)
}

pub fn G3(parent: Option<BlockEvent>) -> BlockEvent {
    generate_test_stacks_block(3, 7, vec![], parent)
}

pub fn H1(parent: Option<BlockEvent>) -> BlockEvent {
    generate_test_stacks_block(1, 8, vec![], parent)
}

pub fn H3(parent: Option<BlockEvent>) -> BlockEvent {
    generate_test_stacks_block(3, 8, vec![], parent)
}

pub fn I1(parent: Option<BlockEvent>) -> BlockEvent {
    generate_test_stacks_block(1, 9, vec![], parent)
}

pub fn I3(parent: Option<BlockEvent>) -> BlockEvent {
    generate_test_stacks_block(3, 9, vec![], parent)
}

pub fn J1(parent: Option<BlockEvent>) -> BlockEvent {
    generate_test_stacks_block(1, 10, vec![], parent)
}

pub fn K1(parent: Option<BlockEvent>) -> BlockEvent {
    generate_test_stacks_block(1, 11, vec![], parent)
}

pub fn L1(parent: Option<BlockEvent>) -> BlockEvent {
    generate_test_stacks_block(1, 12, vec![], parent)
}

pub fn M1(parent: Option<BlockEvent>) -> BlockEvent {
    generate_test_stacks_block(1, 13, vec![], parent)
}

pub fn N1(parent: Option<BlockEvent>) -> BlockEvent {
    generate_test_stacks_block(1, 14, vec![], parent)
}

pub fn O1(parent: Option<BlockEvent>) -> BlockEvent {
    generate_test_stacks_block(1, 15, vec![], parent)
}

pub fn P1(parent: Option<BlockEvent>) -> BlockEvent {
    generate_test_stacks_block(1, 16, vec![], parent)
}
