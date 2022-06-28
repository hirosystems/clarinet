use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

use bitcoincore_rpc::bitcoin::Block;
use clarity_repl::clarity::util::hash::to_hex;
use orchestra_types::{
    BitcoinChainEvent, BlockIdentifier, ChainUpdatedWithBlocksData,
    ChainUpdatedWithMicroblocksData, ChainUpdatedWithReorgData, StacksBlockData, StacksChainEvent,
    StacksMicroblocksTrail,
};

pub struct UnconfirmedBlocksProcessor {
    canonical_fork_id: usize,
    orphans: BTreeSet<BlockIdentifier>,
    block_store: HashMap<BlockIdentifier, StacksBlockData>,
    forks: Vec<ChainSegment>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChainSegment {
    pub amount_of_btc_spent: u64,
    pub most_recent_confirmed_block_height: u64,
    block_ids: VecDeque<BlockIdentifier>,
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
        let segment_index =
            (block_identifier.index - self.most_recent_confirmed_block_height).saturating_sub(1);
        segment_index.try_into().unwrap()
    }

    pub fn can_append_block(
        &self,
        block: &StacksBlockData,
    ) -> Result<(), ChainSegmentIncompatibility> {
        if self.is_block_id_older_than_segment(&block.block_identifier) {
            // Could be facing a deep fork...
            return Err(ChainSegmentIncompatibility::OutdatedBlock);
        }
        if self.is_block_id_newer_than_segment(&block.block_identifier) {
            // Chain segment looks outdated, we should just prune it?
            return Err(ChainSegmentIncompatibility::OutdatedSegment);
        }
        let tip = match self.block_ids.front() {
            Some(tip) => tip,
            None => return Ok(()),
        };
        if tip.index == block.parent_block_identifier.index {
            match tip.hash == block.parent_block_identifier.hash {
                true => return Ok(()),
                false => return Err(ChainSegmentIncompatibility::ParentBlockUnknown),
            }
        }
        println!(
            "Index: {}",
            self.get_relative_index(&block.block_identifier)
        );
        if let Some(colliding_block) = self.get_block_id(&block.block_identifier) {
            match colliding_block.eq(&block.block_identifier) {
                true => return Err(ChainSegmentIncompatibility::AlreadyPresent),
                false => return Err(ChainSegmentIncompatibility::BlockCollision),
            }
        }
        Err(ChainSegmentIncompatibility::Unknown)
    }

    fn get_block_id(&self, block_id: &BlockIdentifier) -> Option<&BlockIdentifier> {
        match self.block_ids.get(self.get_relative_index(block_id)) {
            Some(res) => Some(res),
            None => None,
        }
    }

    pub fn append_block_identifier(&mut self, block_identifier: &BlockIdentifier, prune: bool) {
        self.block_ids.push_front(block_identifier.clone());
        if prune {
            self.prune_confirmed_blocks()
        }
    }

    pub fn prune_confirmed_blocks(&mut self) {
        while self.block_ids.len() > 7 {
            let confirmed_block_id = self.block_ids.pop_back().unwrap();
            self.most_recent_confirmed_block_height = confirmed_block_id.index;
            self.confirmed_blocks_inbox.push(confirmed_block_id);
        }
    }

    pub fn get_length(&self) -> u64 {
        let len: u64 = self.block_ids.len().try_into().unwrap();
        self.most_recent_confirmed_block_height + len
    }

    pub fn keep_blocks_from_oldest_to_block_identifier(
        &mut self,
        block_identifier: &BlockIdentifier,
    ) -> bool {
        loop {
            match self.block_ids.pop_front() {
                Some(tip) => {
                    println!("{} = {}?", tip, block_identifier);
                    if tip.eq(&block_identifier) {
                        self.block_ids.push_front(tip);
                        break true;
                    }
                }
                _ => break false,
            }
        }
    }

    fn try_identify_divergence(
        &self,
        other_segment: &ChainSegment,
    ) -> Result<ChainSegmentDivergence, ChainSegmentIncompatibility> {
        let mut common_root = None;
        let mut blocks_to_rollback = vec![];
        let mut blocks_to_apply = vec![];
        for cursor_segment_1 in other_segment.block_ids.iter() {
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
            blocks_to_apply.clear();
        }
        blocks_to_rollback.reverse();
        blocks_to_apply.reverse();
        match common_root.take() {
            Some(_common_root) => Ok(ChainSegmentDivergence {
                blocks_to_rollback,
                blocks_to_apply,
            }),
            None => Err(ChainSegmentIncompatibility::Unknown),
        }
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

impl UnconfirmedBlocksProcessor {
    pub fn new() -> UnconfirmedBlocksProcessor {
        UnconfirmedBlocksProcessor {
            canonical_fork_id: 0,
            block_store: HashMap::new(),
            orphans: BTreeSet::new(),
            forks: vec![ChainSegment::new()],
        }
    }

    pub fn try_append_block_to_fork(
        &mut self,
        fork_id: usize,
        new_forks: &mut Vec<ChainSegment>,
        fork_ids_to_prune: &mut Vec<usize>,
        block_appended_in_forks: &mut Vec<usize>,
        block: &StacksBlockData,
        prune: bool,
    ) -> bool {
        let fork = match self.forks.get_mut(fork_id) {
            Some(fork) => fork,
            None => return false,
        };
        let mut block_appended = false;
        match fork.can_append_block(block) {
            Ok(()) => {
                println!("Appending {} to {}", block.block_identifier, fork);
                fork.append_block_identifier(&block.block_identifier, prune);
                println!("-> {}", fork);
                block_appended_in_forks.push(fork_id);
                block_appended = true;
            }
            Err(incompatibility) => {
                println!("{:?}", incompatibility);
                match incompatibility {
                    ChainSegmentIncompatibility::BlockCollision => {
                        let mut new_fork = fork.clone();
                        let parent_found = new_fork.keep_blocks_from_oldest_to_block_identifier(
                            &block.parent_block_identifier,
                        );
                        println!("Parent found: {}", parent_found);
                        if parent_found {
                            new_fork.append_block_identifier(&block.block_identifier, prune);
                            new_forks.push(new_fork);
                            let fork_id = self.forks.len() + new_forks.len() - 1;
                            block_appended_in_forks.push(fork_id);
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
        block_appended
    }

    pub fn process_block(&mut self, block: &StacksBlockData) -> Option<StacksChainEvent> {
        println!("Processing {}", block.block_identifier);
        for fork in self.forks.iter() {
            println!("{}", fork);
        }
        // Retrieve previous canonical fork
        let previous_canonical_fork = match self.forks.get(self.canonical_fork_id) {
            Some(fork) => fork.clone(),
            None => return None,
        };

        // Keep block data in memory
        self.block_store
            .insert(block.block_identifier.clone(), block.clone());

        let mut new_forks = vec![];
        let mut fork_ids_to_prune = vec![];
        let mut block_appended_in_forks = vec![];
        let fork_ids = self.forks.len();
        let mut block_appended = false;
        for fork_id in 0..fork_ids {
            block_appended = self.try_append_block_to_fork(
                fork_id,
                &mut new_forks,
                &mut fork_ids_to_prune,
                &mut block_appended_in_forks,
                block,
                true,
            );
            if !block_appended {
                self.orphans.insert(block.block_identifier.clone());
            }
        }

        // Start tracking new forks
        self.forks.append(&mut new_forks);

        // Process former orphans
        let orphans = self.orphans.clone();
        let mut orphans_to_untrack = HashSet::new();
        // For each fork that were modified with the block being processed
        for fork_id in block_appended_in_forks.into_iter() {
            let mut at_least_one_orphan_appended = true;
            // As long as we are successful appending blocks that were previously unprocessable,
            // Keep looping on this backlog
            let mut applied = HashSet::new();
            while at_least_one_orphan_appended {
                at_least_one_orphan_appended = false;
                for orphan_block_identifier in orphans.iter() {
                    if applied.contains(orphan_block_identifier) {
                        continue;
                    }
                    println!("Considering orphaned {}", orphan_block_identifier);
                    let block = match self.block_store.get(orphan_block_identifier) {
                        Some(block) => block.clone(),
                        None => continue,
                    };
                    let orphan_appended = self.try_append_block_to_fork(
                        fork_id,
                        &mut vec![],
                        &mut vec![],
                        &mut vec![],
                        &block,
                        false,
                    );
                    if orphan_appended {
                        applied.insert(orphan_block_identifier);
                    }
                    block_appended = block_appended || orphan_appended;
                    at_least_one_orphan_appended = at_least_one_orphan_appended || orphan_appended;
                    println!("{} / {}", block_appended, at_least_one_orphan_appended);
                    if orphan_appended {
                        orphans_to_untrack.insert(orphan_block_identifier);
                    }
                }
            }
        }

        // Update orphans
        for orphan in orphans_to_untrack.into_iter() {
            println!("Dequeuing orphan");
            self.orphans.remove(orphan);
        }

        // Collect confirmed blocks, remove from block store

        // Process prunable forks
        fork_ids_to_prune.reverse();
        for fork_id in fork_ids_to_prune {
            println!("Pruning fork {}", fork_id);
            self.forks.remove(fork_id);
        }

        if !block_appended {
            return None;
        }

        // Select canonical fork
        let mut canonical_fork_id = 0;
        let mut highest_height = 0;
        let mut highest_btc_spent = 0;
        for (fork_id, fork) in self.forks.iter().enumerate() {
            println!("Fork Id: {} - {}", fork_id, fork);
            if fork.get_length() >= highest_height {
                highest_height = fork.get_length();
                if fork.amount_of_btc_spent > highest_btc_spent
                    || (fork.amount_of_btc_spent == highest_btc_spent
                        && fork_id > canonical_fork_id)
                {
                    highest_btc_spent = fork.amount_of_btc_spent;
                    canonical_fork_id = fork_id;
                }
            }
        }
        println!("Fork Id selected: {}", canonical_fork_id);

        self.canonical_fork_id = canonical_fork_id;
        // Generate chain event from the previous and current canonical forks
        let canonical_fork = self.forks.get(canonical_fork_id).unwrap();
        if canonical_fork.eq(&previous_canonical_fork) {
            return None;
        }

        let chain_event = Some(self.generate_chain_event(canonical_fork, &previous_canonical_fork));

        for fork in self.forks.iter_mut() {
            fork.prune_confirmed_blocks();
        }

        chain_event
    }

    pub fn generate_chain_event(
        &self,
        canonical_segment: &ChainSegment,
        other_segment: &ChainSegment,
    ) -> StacksChainEvent {
        println!("1: {}", other_segment);
        println!("2: {}", canonical_segment);
        if other_segment.is_empty() {
            let mut new_blocks = vec![];
            for i in 0..canonical_segment.block_ids.len() {
                let block_identifier =
                    &canonical_segment.block_ids[canonical_segment.block_ids.len() - 1 - i];
                let block = match self.block_store.get(block_identifier) {
                    Some(block) => block.clone(),
                    None => panic!("unable to retrive block from block store"),
                };
                new_blocks.push(block)
            }
            return StacksChainEvent::ChainUpdatedWithBlocks(ChainUpdatedWithBlocksData {
                new_blocks,
                anchored_trail: None,
            });
        }
        if let Ok(divergence) = canonical_segment.try_identify_divergence(other_segment) {
            if divergence.blocks_to_rollback.is_empty() {
                let mut new_blocks = vec![];
                for i in 0..divergence.blocks_to_apply.len() {
                    let block_identifier = &divergence.blocks_to_apply[i];
                    let block = match self.block_store.get(block_identifier) {
                        Some(block) => block.clone(),
                        None => panic!("unable to retrive block from block store"),
                    };
                    new_blocks.push(block)
                }
                return StacksChainEvent::ChainUpdatedWithBlocks(ChainUpdatedWithBlocksData {
                    new_blocks,
                    anchored_trail: None,
                });
            } else {
                return StacksChainEvent::ChainUpdatedWithReorg(ChainUpdatedWithReorgData {
                    blocks_to_rollback: divergence
                        .blocks_to_rollback
                        .iter()
                        .map(|block_id| {
                            let block = match self.block_store.get(block_id) {
                                Some(block) => block.clone(),
                                None => panic!(),
                            };
                            (None, block)
                        })
                        .collect::<Vec<_>>(),
                    blocks_to_apply: divergence
                        .blocks_to_apply
                        .iter()
                        .map(|block_id| {
                            let block = match self.block_store.get(block_id) {
                                Some(block) => block.clone(),
                                None => panic!(),
                            };
                            (None, block)
                        })
                        .collect::<Vec<_>>(),
                });
            }
        }
        panic!()
    }
}
