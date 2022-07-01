use orchestra_types::{StacksBlockData, StacksMicroblockData};

pub mod accounts;
#[allow(non_snake_case, unreachable_code)]
pub mod blocks;
pub mod microblocks;
pub mod shapes;
pub mod transactions;

pub enum BlockEvent {
    Block(StacksBlockData),
    Microblock(StacksMicroblockData),
}

impl BlockEvent {
    pub fn expect_block(self) -> StacksBlockData {
        match self {
            BlockEvent::Block(block_data) => block_data,
            _ => panic!("expected block"),
        }
    }
}
