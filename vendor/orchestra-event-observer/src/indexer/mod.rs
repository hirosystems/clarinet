pub mod chains;
pub mod unconfirmed_blocks_handler;
use orchestra_types::{
    BitcoinChainEvent, BlockIdentifier, ChainUpdatedWithBlockData, ChainUpdatedWithMicroblockData,
    StacksBlockData, StacksChainEvent, StacksMicroblocksTrail,
};
use rocket::serde::json::Value as JsonValue;
use stacks_rpc_client::PoxInfo;
use std::collections::{HashMap, VecDeque};
pub use unconfirmed_blocks_handler::UnconfirmedBlocksProcessor;

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
    unconfirmed_stacks_blocks_processor: UnconfirmedBlocksProcessor,
    bitcoin_last_7_blocks: VecDeque<BlockIdentifier>,
    pub stacks_context: StacksChainContext,
}

impl Indexer {
    pub fn new(config: IndexerConfig) -> Indexer {
        let unconfirmed_stacks_blocks_processor = UnconfirmedBlocksProcessor::new();
        let bitcoin_last_7_blocks = VecDeque::new();
        let stacks_context = StacksChainContext::new();
        Indexer {
            config,
            unconfirmed_stacks_blocks_processor,
            bitcoin_last_7_blocks,
            stacks_context,
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

    pub fn handle_stacks_block(&mut self, marshalled_block: JsonValue) -> Option<StacksChainEvent> {
        let block = chains::standardize_stacks_block(
            &self.config,
            marshalled_block,
            &mut self.stacks_context,
        );
        self.unconfirmed_stacks_blocks_processor
            .process_block(&block)
    }

    pub fn handle_stacks_microblock(
        &mut self,
        _marshalled_microblock: JsonValue,
    ) -> Option<StacksChainEvent> {
        None
    }

    pub fn get_pox_info(&mut self) -> PoxInfo {
        self.stacks_context.pox_info.clone()
    }
}

#[cfg(test)]
mod tests;
