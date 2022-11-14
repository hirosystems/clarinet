use crate::indexer::{ChainSegment, ChainSegmentIncompatibility};
use chainhook_types::{
    BitcoinBlockData, BitcoinChainEvent, BitcoinChainUpdatedWithBlocksData,
    BitcoinChainUpdatedWithReorgData, BlockIdentifier,
};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

pub struct BitcoinBlockPool {
    canonical_fork_id: usize,
    orphans: BTreeSet<BlockIdentifier>,
    block_store: HashMap<BlockIdentifier, BitcoinBlockData>,
    forks: BTreeMap<usize, ChainSegment>,
}

impl BitcoinBlockPool {
    pub fn new() -> BitcoinBlockPool {
        let mut forks = BTreeMap::new();
        forks.insert(0, ChainSegment::new());
        BitcoinBlockPool {
            canonical_fork_id: 0,
            block_store: HashMap::new(),
            orphans: BTreeSet::new(),
            forks,
        }
    }

    pub fn process_block(
        &mut self,
        block: BitcoinBlockData,
    ) -> Result<Option<BitcoinChainEvent>, String> {
        info!("Start processing Bitcoin {}", block.block_identifier);

        // Keep block data in memory
        let existing_entry = self
            .block_store
            .insert(block.block_identifier.clone(), block.clone());
        if existing_entry.is_some() {
            warn!(
                "Bitcoin {} has already been processed",
                block.block_identifier
            );
            return Ok(None);
        }

        for (i, fork) in self.forks.iter() {
            info!("Active fork {}: {}", i, fork);
        }
        // Retrieve previous canonical fork
        let previous_canonical_fork_id = self.canonical_fork_id;
        let previous_canonical_fork = match self.forks.get(&previous_canonical_fork_id) {
            Some(fork) => fork.clone(),
            None => {
                error!("unable to retrieve previous bitcoin fork");
                return Ok(None);
            }
        };

        let mut fork_updated = None;
        for (_, fork) in self.forks.iter_mut() {
            let (block_appended, mut new_fork) = fork.try_append_block(&block);
            if block_appended {
                if let Some(new_fork) = new_fork.take() {
                    let fork_id = self.forks.len();
                    self.forks.insert(fork_id, new_fork);
                    fork_updated = self.forks.get_mut(&fork_id);
                } else {
                    fork_updated = Some(fork);
                }
                // A block can only be added to one segment
                break;
            }
        }

        let fork_updated = match fork_updated.take() {
            Some(fork) => {
                info!(
                    "Bitcoin {} successfully appended to {}",
                    block.block_identifier, fork
                );
                fork
            }
            None => {
                info!(
                    "Unable to process Bitcoin {} - inboxed for later",
                    block.block_identifier
                );
                self.orphans.insert(block.block_identifier.clone());
                return Ok(None);
            }
        };

        // Process former orphans
        let orphans = self.orphans.clone();
        let mut orphans_to_untrack = HashSet::new();

        let mut at_least_one_orphan_appended = true;
        // As long as we are successful appending blocks that were previously unprocessable,
        // Keep looping on this backlog
        let mut applied = HashSet::new();
        let mut forks_created = vec![];
        while at_least_one_orphan_appended {
            at_least_one_orphan_appended = false;
            for orphan_block_identifier in orphans.iter() {
                if applied.contains(orphan_block_identifier) {
                    continue;
                }
                let block = match self.block_store.get(orphan_block_identifier) {
                    Some(block) => block.clone(),
                    None => continue,
                };

                let (orphan_appended, mut new_fork) = fork_updated.try_append_block(&block);
                if orphan_appended {
                    applied.insert(orphan_block_identifier);
                    orphans_to_untrack.insert(orphan_block_identifier);
                    if let Some(new_fork) = new_fork.take() {
                        forks_created.push(new_fork);
                    }
                }
                at_least_one_orphan_appended = at_least_one_orphan_appended || orphan_appended;
            }
        }

        // Update orphans
        for orphan in orphans_to_untrack.into_iter() {
            info!("Dequeuing orphan {}", orphan);
            self.orphans.remove(orphan);
        }

        // Select canonical fork
        let mut canonical_fork_id = 0;
        let mut highest_height = 0;
        for (fork_id, fork) in self.forks.iter() {
            info!("Active fork: {} - {}", fork_id, fork);
            if fork.get_length() >= highest_height {
                highest_height = fork.get_length();
                canonical_fork_id = *fork_id;
            }
        }
        info!("Active fork selected as canonical: {}", canonical_fork_id);

        self.canonical_fork_id = canonical_fork_id;
        // Generate chain event from the previous and current canonical forks
        let canonical_fork = self.forks.get(&canonical_fork_id).unwrap().clone();
        if canonical_fork.eq(&previous_canonical_fork) {
            info!("Canonical fork unchanged");
            return Ok(None);
        }

        let res = self.generate_block_chain_event(&canonical_fork, &previous_canonical_fork);
        let mut chain_event = match res {
            Ok(chain_event) => chain_event,
            Err(ChainSegmentIncompatibility::ParentBlockUnknown) => {
                self.canonical_fork_id = previous_canonical_fork_id;
                return Ok(None);
            }
            _ => return Ok(None),
        };

        self.collect_and_prune_confirmed_blocks(&mut chain_event);

        Ok(Some(chain_event))
    }

    pub fn collect_and_prune_confirmed_blocks(&mut self, chain_event: &mut BitcoinChainEvent) {
        let (tip, confirmed_blocks) = match chain_event {
            BitcoinChainEvent::ChainUpdatedWithBlocks(ref mut event) => {
                match event.new_blocks.last() {
                    Some(tip) => (tip.block_identifier.clone(), &mut event.confirmed_blocks),
                    None => return,
                }
            }
            BitcoinChainEvent::ChainUpdatedWithReorg(ref mut event) => {
                match event.blocks_to_apply.last() {
                    Some(tip) => (tip.block_identifier.clone(), &mut event.confirmed_blocks),
                    None => return,
                }
            }
        };

        let mut forks_to_prune = vec![];
        let mut ancestor_identifier = &tip;

        // Retrieve the whole canonical segment present in memory, ascending order
        // [1] ... [6] [7]
        let canonical_segment = {
            let mut segment = vec![];
            while let Some(ancestor) = self.block_store.get(&ancestor_identifier) {
                ancestor_identifier = &ancestor.parent_block_identifier;
                segment.push(ancestor.block_identifier.clone());
            }
            segment
        };
        if canonical_segment.len() < 7 {
            return;
        }
        // Any block beyond 6th ancestor is considered as confirmed and can be pruned
        let cut_off = &canonical_segment[5];

        // Prune forks using the confirmed block
        let mut blocks_to_prune = vec![];
        for (fork_id, fork) in self.forks.iter_mut() {
            let mut res = fork.prune_confirmed_blocks(&cut_off);
            blocks_to_prune.append(&mut res);
            if fork.block_ids.is_empty() {
                forks_to_prune.push(*fork_id);
            }
        }

        // Prune orphans using the confirmed block
        let iter = self.orphans.clone().into_iter();
        for orphan in iter {
            if orphan.index < cut_off.index {
                self.orphans.remove(&orphan);
                blocks_to_prune.push(orphan);
            }
        }

        for confirmed_block in canonical_segment[6..].into_iter() {
            let block = match self.block_store.remove(confirmed_block) {
                None => {
                    error!("unable to retrieve data for {}", confirmed_block);
                    return;
                }
                Some(block) => block,
            };
            confirmed_blocks.push(block);
        }

        // Prune data
        for block_to_prune in blocks_to_prune {
            self.block_store.remove(&block_to_prune);
        }
        for fork_id in forks_to_prune {
            self.forks.remove(&fork_id);
        }
        confirmed_blocks.reverse();
    }

    pub fn generate_block_chain_event(
        &mut self,
        canonical_segment: &ChainSegment,
        other_segment: &ChainSegment,
    ) -> Result<BitcoinChainEvent, ChainSegmentIncompatibility> {
        if other_segment.is_empty() {
            let mut new_blocks = vec![];
            for i in 0..canonical_segment.block_ids.len() {
                let block_identifier =
                    &canonical_segment.block_ids[canonical_segment.block_ids.len() - 1 - i];
                let block = match self.block_store.get(block_identifier) {
                    Some(block) => block.clone(),
                    None => {
                        error!(
                            "unable to retrive Bitcoin {} from block store",
                            block_identifier
                        );
                        return Err(ChainSegmentIncompatibility::Unknown);
                    }
                };
                new_blocks.push(block)
            }
            return Ok(BitcoinChainEvent::ChainUpdatedWithBlocks(
                BitcoinChainUpdatedWithBlocksData {
                    new_blocks,
                    confirmed_blocks: vec![],
                },
            ));
        }
        if let Ok(divergence) = canonical_segment.try_identify_divergence(other_segment, false) {
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
                return Ok(BitcoinChainEvent::ChainUpdatedWithBlocks(
                    BitcoinChainUpdatedWithBlocksData {
                        new_blocks,
                        confirmed_blocks: vec![],
                    },
                ));
            } else {
                return Ok(BitcoinChainEvent::ChainUpdatedWithReorg(
                    BitcoinChainUpdatedWithReorgData {
                        blocks_to_rollback: divergence
                            .blocks_to_rollback
                            .iter()
                            .map(|block_id| {
                                let block = match self.block_store.get(block_id) {
                                    Some(block) => block.clone(),
                                    None => panic!("unable to retrive block from block store"),
                                };
                                block
                            })
                            .collect::<Vec<_>>(),
                        blocks_to_apply: divergence
                            .blocks_to_apply
                            .iter()
                            .map(|block_id| {
                                let block = match self.block_store.get(block_id) {
                                    Some(block) => block.clone(),
                                    None => panic!("unable to retrive block from block store"),
                                };
                                block
                            })
                            .collect::<Vec<_>>(),
                        confirmed_blocks: vec![],
                    },
                ));
            }
        }
        info!(
            "Unable to infer chain event out of {} and {}",
            canonical_segment, other_segment
        );
        Err(ChainSegmentIncompatibility::ParentBlockUnknown)
    }
}
