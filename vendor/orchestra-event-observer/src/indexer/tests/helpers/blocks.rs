use clarity_repl::clarity::util::hash::{hex_bytes, to_hex};
use orchestra_types::{
    BlockIdentifier, StacksBlockData, StacksBlockMetadata, StacksTransactionData,
    StacksTransactionMetadata,
};

fn generate_test_block(
    fork_id: u8,
    block_height: u64,
    transactions: Vec<StacksTransactionData>,
) -> StacksBlockData {
    let mut hash = vec![
        fork_id, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    let mut parent_hash = if (block_height - 1) == 1 {
        vec![
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]
    } else {
        vec![
            fork_id, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ]
    };
    hash.append(&mut block_height.to_be_bytes().to_vec());
    parent_hash.append(&mut (block_height - 1).to_be_bytes().to_vec());
    StacksBlockData {
        block_identifier: BlockIdentifier {
            index: block_height,
            hash: to_hex(&hash[..]),
        },
        parent_block_identifier: BlockIdentifier {
            index: block_height - 1,
            hash: to_hex(&parent_hash[..]),
        },
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
        },
    }
}

pub fn A1() -> StacksBlockData {
    generate_test_block(0, 1, vec![])
}

pub fn B1() -> StacksBlockData {
    generate_test_block(1, 2, vec![])
}

pub fn B2() -> StacksBlockData {
    generate_test_block(2, 2, vec![])
}

pub fn C1() -> StacksBlockData {
    generate_test_block(1, 3, vec![])
}

pub fn C2() -> StacksBlockData {
    generate_test_block(2, 3, vec![])
}

pub fn D1() -> StacksBlockData {
    generate_test_block(1, 4, vec![])
}

pub fn D2() -> StacksBlockData {
    generate_test_block(2, 4, vec![])
}

pub fn E1() -> StacksBlockData {
    generate_test_block(1, 5, vec![])
}

pub fn E2() -> StacksBlockData {
    generate_test_block(2, 5, vec![])
}

pub fn E3() -> StacksBlockData {
    generate_test_block(3, 5, vec![])
}

pub fn F1() -> StacksBlockData {
    generate_test_block(1, 6, vec![])
}

pub fn F2() -> StacksBlockData {
    generate_test_block(2, 6, vec![])
}

pub fn F3() -> StacksBlockData {
    generate_test_block(3, 6, vec![])
}

pub fn G1() -> StacksBlockData {
    generate_test_block(1, 7, vec![])
}

pub fn G2() -> StacksBlockData {
    generate_test_block(2, 7, vec![])
}

pub fn G3() -> StacksBlockData {
    generate_test_block(3, 7, vec![])
}

pub fn H1() -> StacksBlockData {
    generate_test_block(1, 8, vec![])
}

pub fn H2() -> StacksBlockData {
    generate_test_block(2, 8, vec![])
}

pub fn H3() -> StacksBlockData {
    generate_test_block(3, 8, vec![])
}

pub fn I1() -> StacksBlockData {
    generate_test_block(1, 9, vec![])
}

pub fn I2() -> StacksBlockData {
    generate_test_block(2, 9, vec![])
}

pub fn I3() -> StacksBlockData {
    generate_test_block(3, 9, vec![])
}

pub fn J1() -> StacksBlockData {
    generate_test_block(1, 10, vec![])
}

pub fn J2() -> StacksBlockData {
    generate_test_block(2, 10, vec![])
}

pub fn J3() -> StacksBlockData {
    generate_test_block(3, 10, vec![])
}

pub fn K2() -> StacksBlockData {
    generate_test_block(2, 11, vec![])
}
