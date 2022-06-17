use std::collections::{VecDeque, BTreeMap, HashMap};

use bitcoincore_rpc::bitcoin::Block;
use clarity_repl::clarity::util::hash::to_hex;
use orchestra_types::{
    BitcoinChainEvent, BlockIdentifier, ChainUpdatedWithBlockData, ChainUpdatedWithMicroblockData,
    StacksBlockData, StacksChainEvent, StacksMicroblocksTrail,
};

pub struct UnconfirmedBlocksProcessor {
    confirmed_block_identifier: BlockIdentifier,
    canonical_unconfirmed_block_identifiers: VecDeque<BlockIdentifier>, 
    block_store: Vec<StacksBlockData>,
}

impl UnconfirmedBlocksProcessor {
    pub fn new() -> UnconfirmedBlocksProcessor {
        UnconfirmedBlocksProcessor {
            canonical_unconfirmed_block_identifiers: VecDeque::new(),
            confirmed_block_identifier: BlockIdentifier {
                index: 0,
                hash: to_hex(&[0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0])
            },
            block_store: HashMap::new(),
        }
    }

    pub fn process_block(&mut self, block: &StacksBlockData) -> Option<StacksChainEvent> {
        if let Some(tip_identifier) = self.canonical_unconfirmed_block_identifiers.back() {
            if block.parent_block_identifier.hash == tip_identifier.hash && block.parent_block_identifier.index == tip_identifier.index {
                self.canonical_unconfirmed_block_identifiers.push_back(block.parent_block_identifier.clone());
                self.block_store.insert(block.parent_block_identifier.clone(), block.clone());
            } else {

            }
        } else {
            // todo(lgalabru): revisit bootstrap phase
        }

        let mut anchored_trail = None;
        if let Some((tip, _)) = self.last_7_blocks.back() {
            if block.block_identifier.index == tip.index + 1 {
                self.last_7_blocks
                    .push_back((block.block_identifier.clone(), block.clone()));
                anchored_trail = Some(self.current_microblock_trail.clone());
                self.current_microblock_trail = StacksMicroblocksTrail {
                    microblocks: vec![],
                };
            } else if block.block_identifier.index > tip.index + 1 {
                // TODO(lgalabru): we received a block and we don't have the parent
            } else if block.block_identifier.index == tip.index {
                // TODO(lgalabru): 1 block reorg
            } else {
                // TODO(lgalabru): deeper reorg
            }
        } else {
            self.last_7_blocks
                .push_front((block.block_identifier.clone(), block.clone()));
            self.current_microblock_trail = StacksMicroblocksTrail {
                microblocks: vec![],
            };
        }
        let (_, confirmed_block) = self.last_7_blocks.front().unwrap().clone();
        if self.last_7_blocks.len() > 7 {
            self.last_7_blocks.pop_front();
        }

        let update = ChainUpdatedWithBlockData {
            new_block: block,
            anchored_trail,
            confirmed_block: (confirmed_block, None),
        };
        StacksChainEvent::ChainUpdatedWithBlock(update)
        None
    }
}