pub mod chains;

use orchestra_types::{
    BitcoinChainEvent, BlockIdentifier, ChainUpdatedWithBlockData, ChainUpdatedWithMicroblockData,
    StacksBlockData, StacksChainEvent, StacksMicroblocksTrail,
};
use stacks_rpc_client::PoxInfo;
use rocket::serde::json::Value as JsonValue;
use std::collections::{HashMap, VecDeque};

#[derive(Deserialize, Debug, Clone, Default)]
pub struct AssetClassCache {
    pub symbol: String,
    pub decimals: u8,
}

pub struct StacksChainContext {
    asset_class_map: HashMap<String, AssetClassCache>,
    pox_info: PoxInfo,
}

impl StacksChainContext {
    pub fn new() -> StacksChainContext {
        StacksChainContext {
            asset_class_map: HashMap::new(),
            pox_info: PoxInfo::default(),
        }
    }
}

pub struct IndexerConfig {
    pub stacks_node_rpc_url: String,
    pub bitcoin_node_rpc_url: String,
    pub bitcoin_node_rpc_username: String,
    pub bitcoin_node_rpc_password: String,
}

pub struct Indexer {
    config: IndexerConfig,
    current_microblock_trail: StacksMicroblocksTrail,
    stacks_last_7_blocks: VecDeque<(BlockIdentifier, StacksBlockData)>,
    bitcoin_last_7_blocks: VecDeque<BlockIdentifier>,
    pub stacks_context: StacksChainContext,
}

impl Indexer {
    pub fn new(config: IndexerConfig) -> Indexer {
        let stacks_last_7_blocks = VecDeque::new();
        let bitcoin_last_7_blocks = VecDeque::new();
        let current_microblock_trail = StacksMicroblocksTrail {
            microblocks: vec![],
        };
        let stacks_context = StacksChainContext::new();
        Indexer {
            config,
            stacks_last_7_blocks,
            bitcoin_last_7_blocks,
            stacks_context,
            current_microblock_trail,
        }
    }

    pub fn handle_bitcoin_block(&mut self, marshalled_block: JsonValue) -> BitcoinChainEvent {
        let block = chains::standardize_bitcoin_block(&self.config, marshalled_block);
        if let Some(tip) = self.bitcoin_last_7_blocks.back() {
            if block.block_identifier.index == tip.index + 1 {
                self.bitcoin_last_7_blocks
                    .push_back(block.block_identifier.clone());
                if self.bitcoin_last_7_blocks.len() > 7 {
                    self.bitcoin_last_7_blocks.pop_front();
                }
            } else if block.block_identifier.index > tip.index + 1 {
                // TODO(lgalabru): we received a block and we don't have the parent
            } else if block.block_identifier.index == tip.index {
                // TODO(lgalabru): 1 block reorg
            } else {
                // TODO(lgalabru): deeper reorg
            }
        } else {
            self.bitcoin_last_7_blocks
                .push_front(block.block_identifier.clone());
        }
        BitcoinChainEvent::ChainUpdatedWithBlock(block)
    }

    pub fn handle_stacks_block(&mut self, marshalled_block: JsonValue) -> StacksChainEvent {
        let block = chains::standardize_stacks_block(
            &self.config,
            marshalled_block,
            &mut self.stacks_context,
        );
        let mut anchored_trail = None;
        if let Some((tip, _)) = self.stacks_last_7_blocks.back() {
            if block.block_identifier.index == tip.index + 1 {
                self.stacks_last_7_blocks
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
            self.stacks_last_7_blocks
                .push_front((block.block_identifier.clone(), block.clone()));
            self.current_microblock_trail = StacksMicroblocksTrail {
                microblocks: vec![],
            };
        }
        let (_, confirmed_block) = self.stacks_last_7_blocks.front().unwrap().clone();
        if self.stacks_last_7_blocks.len() > 7 {
            self.stacks_last_7_blocks.pop_front();
        }

        let update = ChainUpdatedWithBlockData {
            new_block: block,
            anchored_trail,
            confirmed_block: (confirmed_block, None),
        };
        StacksChainEvent::ChainUpdatedWithBlock(update)
    }

    pub fn handle_stacks_microblock(
        &mut self,
        marshalled_microblock: JsonValue,
    ) -> StacksChainEvent {
        let (_, anchored_block) = self.stacks_last_7_blocks.back().unwrap();

        let microblock = chains::standardize_stacks_microblock(
            &self.config,
            marshalled_microblock,
            &anchored_block.block_identifier,
            &mut self.stacks_context,
        );
        self.current_microblock_trail.microblocks.push(microblock);

        let update = ChainUpdatedWithMicroblockData {
            anchored_block: anchored_block.clone(),
            current_trail: self.current_microblock_trail.clone(),
        };

        StacksChainEvent::ChainUpdatedWithMicroblock(update)
    }

    pub fn get_pox_info(&mut self) -> PoxInfo {
        self.stacks_context.pox_info.clone()
    }
}
