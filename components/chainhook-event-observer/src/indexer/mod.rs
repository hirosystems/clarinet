pub mod bitcoin;
pub mod stacks;

use crate::utils::AbstractBlock;
use chainhook_types::{
    BitcoinChainEvent, BitcoinNetwork, BlockIdentifier, StacksChainEvent, StacksNetwork,
};
use rocket::serde::json::Value as JsonValue;
use stacks::StacksBlockPool;
use stacks_rpc_client::PoxInfo;
use std::collections::{HashMap, VecDeque};

use self::bitcoin::BitcoinBlockPool;

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

#[derive(Debug, Clone)]
pub struct IndexerConfig {
    pub bitcoin_network: BitcoinNetwork,
    pub stacks_network: StacksNetwork,
    pub stacks_node_rpc_url: String,
    pub bitcoin_node_rpc_url: String,
    pub bitcoin_node_rpc_username: String,
    pub bitcoin_node_rpc_password: String,
}

pub struct Indexer {
    pub config: IndexerConfig,
    stacks_blocks_pool: StacksBlockPool,
    bitcoin_blocks_pool: BitcoinBlockPool,
    pub stacks_context: StacksChainContext,
}

impl Indexer {
    pub fn new(config: IndexerConfig) -> Indexer {
        let stacks_blocks_pool = StacksBlockPool::new();
        let bitcoin_blocks_pool = BitcoinBlockPool::new();
        let stacks_context = StacksChainContext::new();
        Indexer {
            config,
            stacks_blocks_pool,
            bitcoin_blocks_pool,
            stacks_context,
        }
    }

    pub fn handle_bitcoin_block(
        &mut self,
        marshalled_block: JsonValue,
    ) -> Result<Option<BitcoinChainEvent>, String> {
        let block = bitcoin::standardize_bitcoin_block(&self.config, marshalled_block)?;
        let event = self.bitcoin_blocks_pool.process_block(block);
        event
    }

    pub fn handle_stacks_serialized_block(
        &mut self,
        serialized_block: &str,
    ) -> Result<Option<StacksChainEvent>, String> {
        let block = stacks::standardize_stacks_serialized_block(
            &self.config,
            serialized_block,
            &mut self.stacks_context,
        )?;
        self.stacks_blocks_pool.process_block(block)
    }

    pub fn handle_stacks_marshalled_block(
        &mut self,
        marshalled_block: JsonValue,
    ) -> Result<Option<StacksChainEvent>, String> {
        let block = stacks::standardize_stacks_marshalled_block(
            &self.config,
            marshalled_block,
            &mut self.stacks_context,
        )?;
        self.stacks_blocks_pool.process_block(block)
    }

    pub fn handle_stacks_serialized_microblock_trail(
        &mut self,
        serialized_microblock_trail: &str,
    ) -> Result<Option<StacksChainEvent>, String> {
        let microblocks = stacks::standardize_stacks_serialized_microblock_trail(
            &self.config,
            serialized_microblock_trail,
            &mut self.stacks_context,
        )?;
        self.stacks_blocks_pool.process_microblocks(microblocks)
    }

    pub fn handle_stacks_marshalled_microblock_trail(
        &mut self,
        marshalled_microblock_trail: JsonValue,
    ) -> Result<Option<StacksChainEvent>, String> {
        let microblocks = stacks::standardize_stacks_marshalled_microblock_trail(
            &self.config,
            marshalled_microblock_trail,
            &mut self.stacks_context,
        )?;
        self.stacks_blocks_pool.process_microblocks(microblocks)
    }

    pub fn get_pox_info(&mut self) -> PoxInfo {
        self.stacks_context.pox_info.clone()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChainSegment {
    pub amount_of_btc_spent: u64,
    pub most_recent_confirmed_block_height: u64,
    pub block_ids: VecDeque<BlockIdentifier>,
    confirmed_blocks_inbox: Vec<BlockIdentifier>,
}

#[derive(Clone, Debug)]
pub enum ChainSegmentIncompatibility {
    OutdatedBlock,
    OutdatedSegment,
    BlockCollision,
    ParentBlockUnknown,
    AlreadyPresent,
    Unknown,
}

#[derive(Debug)]
pub struct ChainSegmentDivergence {
    blocks_to_apply: Vec<BlockIdentifier>,
    blocks_to_rollback: Vec<BlockIdentifier>,
}

impl ChainSegment {
    pub fn new() -> ChainSegment {
        let block_ids = VecDeque::new();
        ChainSegment {
            block_ids,
            most_recent_confirmed_block_height: 0,
            confirmed_blocks_inbox: vec![],
            amount_of_btc_spent: 0,
        }
    }

    fn is_empty(&self) -> bool {
        self.block_ids.is_empty()
    }

    fn is_block_id_older_than_segment(&self, block_identifier: &BlockIdentifier) -> bool {
        block_identifier.index < self.most_recent_confirmed_block_height
    }

    fn is_block_id_newer_than_segment(&self, block_identifier: &BlockIdentifier) -> bool {
        if let Some(tip) = self.block_ids.front() {
            return block_identifier.index > (tip.index + 1);
        }
        return false;
    }

    fn get_relative_index(&self, block_identifier: &BlockIdentifier) -> usize {
        if let Some(tip) = self.block_ids.front() {
            let segment_index = tip.index.saturating_sub(block_identifier.index);
            return segment_index.try_into().unwrap();
        }
        return 0;
    }

    fn can_append_block(
        &self,
        block: &dyn AbstractBlock,
    ) -> Result<(), ChainSegmentIncompatibility> {
        if self.is_block_id_older_than_segment(&block.get_identifier()) {
            // Could be facing a deep fork...
            return Err(ChainSegmentIncompatibility::OutdatedBlock);
        }
        if self.is_block_id_newer_than_segment(&block.get_identifier()) {
            // Chain segment looks outdated, we should just prune it?
            return Err(ChainSegmentIncompatibility::OutdatedSegment);
        }
        let tip = match self.block_ids.front() {
            Some(tip) => tip,
            None => return Ok(()),
        };
        info!("Comparing {} with {}", tip, block.get_identifier());
        if tip.index == block.get_parent_identifier().index {
            match tip.hash == block.get_parent_identifier().hash {
                true => return Ok(()),
                false => return Err(ChainSegmentIncompatibility::ParentBlockUnknown),
            }
        }
        if let Some(colliding_block) = self.get_block_id(&block.get_identifier()) {
            match colliding_block.eq(&block.get_identifier()) {
                true => return Err(ChainSegmentIncompatibility::AlreadyPresent),
                false => return Err(ChainSegmentIncompatibility::BlockCollision),
            }
        }
        Err(ChainSegmentIncompatibility::Unknown)
    }

    fn get_block_id(&self, block_id: &BlockIdentifier) -> Option<&BlockIdentifier> {
        info!("=> {}", self.get_relative_index(block_id));
        match self.block_ids.get(self.get_relative_index(block_id)) {
            Some(res) => Some(res),
            None => None,
        }
    }

    pub fn append_block_identifier(&mut self, block_identifier: &BlockIdentifier) {
        self.block_ids.push_front(block_identifier.clone());
    }

    pub fn prune_confirmed_blocks(&mut self, cut_off: &BlockIdentifier) -> Vec<BlockIdentifier> {
        let mut keep = vec![];
        let mut prune = vec![];

        for block_id in self.block_ids.drain(..) {
            if block_id.index >= cut_off.index {
                keep.push(block_id);
            } else {
                prune.push(block_id);
            }
        }
        for block_id in keep.into_iter() {
            self.block_ids.push_back(block_id);
        }
        prune
    }

    pub fn get_tip(&self) -> &BlockIdentifier {
        self.block_ids.front().unwrap()
    }

    pub fn get_length(&self) -> u64 {
        let len: u64 = self.block_ids.len().try_into().unwrap();
        self.most_recent_confirmed_block_height + len
    }

    pub fn keep_blocks_from_oldest_to_block_identifier(
        &mut self,
        block_identifier: &BlockIdentifier,
    ) -> (bool, bool) {
        let mut mutated = false;
        loop {
            match self.block_ids.pop_front() {
                Some(tip) => {
                    if tip.eq(&block_identifier) {
                        self.block_ids.push_front(tip);
                        break (true, mutated);
                    }
                }
                _ => break (false, mutated),
            }
            mutated = true;
        }
    }

    fn try_identify_divergence(
        &self,
        other_segment: &ChainSegment,
        allow_reset: bool,
    ) -> Result<ChainSegmentDivergence, ChainSegmentIncompatibility> {
        let mut common_root = None;
        let mut blocks_to_rollback = vec![];
        let mut blocks_to_apply = vec![];
        for cursor_segment_1 in other_segment.block_ids.iter() {
            blocks_to_apply.clear();
            for cursor_segment_2 in self.block_ids.iter() {
                if cursor_segment_2.eq(cursor_segment_1) {
                    common_root = Some(cursor_segment_2.clone());
                    break;
                }
                blocks_to_apply.push(cursor_segment_2.clone());
            }
            if common_root.is_some() {
                break;
            }
            blocks_to_rollback.push(cursor_segment_1.clone());
        }
        debug!("Blocks to rollback: {:?}", blocks_to_rollback);
        debug!("Blocks to apply: {:?}", blocks_to_apply);
        blocks_to_rollback.reverse();
        blocks_to_apply.reverse();
        match common_root.take() {
            Some(_common_root) => Ok(ChainSegmentDivergence {
                blocks_to_rollback,
                blocks_to_apply,
            }),
            None if allow_reset => Ok(ChainSegmentDivergence {
                blocks_to_rollback,
                blocks_to_apply,
            }),
            None => Err(ChainSegmentIncompatibility::Unknown),
        }
    }

    fn try_append_block(&mut self, block: &dyn AbstractBlock) -> (bool, Option<ChainSegment>) {
        let mut block_appended = false;
        let mut fork = None;
        info!("Trying to append {} to {}", block.get_identifier(), self);
        match self.can_append_block(block) {
            Ok(()) => {
                self.append_block_identifier(&block.get_identifier());
                block_appended = true;
            }
            Err(incompatibility) => {
                info!("Will have to fork: {:?}", incompatibility);
                match incompatibility {
                    ChainSegmentIncompatibility::BlockCollision => {
                        let mut new_fork = self.clone();
                        let (parent_found, _) = new_fork
                            .keep_blocks_from_oldest_to_block_identifier(
                                &block.get_parent_identifier(),
                            );
                        if parent_found {
                            info!("Success");
                            new_fork.append_block_identifier(&block.get_identifier());
                            fork = Some(new_fork);
                            block_appended = true;
                        }
                    }
                    ChainSegmentIncompatibility::OutdatedSegment => {
                        // TODO(lgalabru): test depth
                        // fork_ids_to_prune.push(fork_id);
                    }
                    ChainSegmentIncompatibility::ParentBlockUnknown => {}
                    ChainSegmentIncompatibility::OutdatedBlock => {}
                    ChainSegmentIncompatibility::Unknown => {}
                    ChainSegmentIncompatibility::AlreadyPresent => {}
                }
            }
        }
        (block_appended, fork)
    }
}

impl std::fmt::Display for ChainSegment {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Fork [{}], height = {}",
            self.block_ids
                .iter()
                .map(|b| format!("{}", b))
                .collect::<Vec<_>>()
                .join(", "),
            self.get_length()
        )
    }
}

#[cfg(test)]
pub mod tests;
