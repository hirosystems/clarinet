use std::collections::{hash_map::Entry, BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

use bitcoincore_rpc::bitcoin::Block;
use clarity_repl::clarity::util::hash::to_hex;
use orchestra_types::{
    BitcoinChainEvent, BlockIdentifier, Chain, ChainUpdatedWithBlocksData,
    ChainUpdatedWithMicroblocksData, ChainUpdatedWithMicroblocksReorgData,
    ChainUpdatedWithReorgData, StacksBlockData, StacksBlockUpdate, StacksChainEvent,
    StacksMicroblockData, StacksMicroblocksTrail,
};

trait AbstractBlock {
    fn get_identifier(&self) -> &BlockIdentifier;
    fn get_parent_identifier(&self) -> &BlockIdentifier;
}

impl AbstractBlock for StacksBlockData {
    fn get_identifier(&self) -> &BlockIdentifier {
        &self.block_identifier
    }

    fn get_parent_identifier(&self) -> &BlockIdentifier {
        &self.parent_block_identifier
    }
}

impl AbstractBlock for StacksMicroblockData {
    fn get_identifier(&self) -> &BlockIdentifier {
        &self.block_identifier
    }

    fn get_parent_identifier(&self) -> &BlockIdentifier {
        &self.parent_block_identifier
    }
}

pub struct UnconfirmedBlocksProcessor {
    canonical_fork_id: usize,
    orphans: BTreeSet<BlockIdentifier>,
    block_store: HashMap<BlockIdentifier, StacksBlockData>,
    forks: Vec<ChainSegment>,
    microblock_store: HashMap<(BlockIdentifier, BlockIdentifier), StacksMicroblockData>,
    micro_forks: HashMap<BlockIdentifier, Vec<ChainSegment>>,
    micro_orphans: BTreeSet<(BlockIdentifier, BlockIdentifier)>,
    canonical_micro_fork_id: HashMap<BlockIdentifier, usize>,
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
        if tip.index == block.get_parent_identifier().index {
            match tip.hash == block.get_parent_identifier().hash {
                true => return Ok(()),
                false => return Err(ChainSegmentIncompatibility::ParentBlockUnknown),
            }
        }
        println!(
            "Index: {}",
            self.get_relative_index(&block.get_identifier())
        );
        if let Some(colliding_block) = self.get_block_id(&block.get_identifier()) {
            match colliding_block.eq(&block.get_identifier()) {
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
    ) -> (bool, bool) {
        let mut mutated = false;
        loop {
            match self.block_ids.pop_front() {
                Some(tip) => {
                    println!("{} = {}?", tip, block_identifier);
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
                    println!("case 1: {} = {}", cursor_segment_2, cursor_segment_1);
                    break;
                }
                blocks_to_apply.push(cursor_segment_2.clone());
            }
            if common_root.is_some() {
                println!("common_root: {:?}", common_root);
                break;
            }
            blocks_to_rollback.push(cursor_segment_1.clone());
        }
        println!("TO ROLLBACK: {:?}", blocks_to_rollback);
        println!("TO APPLY: {:?}", blocks_to_apply);
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
        match self.can_append_block(block) {
            Ok(()) => {
                self.append_block_identifier(&block.get_identifier(), false);
                block_appended = true;
            }
            Err(incompatibility) => {
                match incompatibility {
                    ChainSegmentIncompatibility::BlockCollision => {
                        let mut new_fork = self.clone();
                        let (parent_found, _) = new_fork
                            .keep_blocks_from_oldest_to_block_identifier(
                                &block.get_parent_identifier(),
                            );
                        if parent_found {
                            new_fork.append_block_identifier(&block.get_identifier(), false);
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

impl UnconfirmedBlocksProcessor {
    pub fn new() -> UnconfirmedBlocksProcessor {
        UnconfirmedBlocksProcessor {
            canonical_fork_id: 0,
            block_store: HashMap::new(),
            orphans: BTreeSet::new(),
            forks: vec![ChainSegment::new()],
            microblock_store: HashMap::new(),
            micro_forks: HashMap::new(),
            micro_orphans: BTreeSet::new(),
            canonical_micro_fork_id: HashMap::new(),
        }
    }

    pub fn try_append_block_to_chain_segment(
        &mut self,
        chain_segment: &mut ChainSegment,
        fork_id: usize,
        new_forks: &mut Vec<ChainSegment>,
        fork_ids_to_prune: &mut Vec<usize>,
        block_appended_in_forks: &mut Vec<usize>,
        block: &StacksBlockData,
        prune: bool,
    ) -> bool {
        let mut block_appended = false;
        match chain_segment.can_append_block(block) {
            Ok(()) => {
                println!("Appending {} to {}", block.block_identifier, chain_segment);
                chain_segment.append_block_identifier(&block.block_identifier, prune);
                println!("-> {}", chain_segment);
                block_appended_in_forks.push(fork_id);
                block_appended = true;
            }
            Err(incompatibility) => {
                println!("{:?}", incompatibility);
                match incompatibility {
                    ChainSegmentIncompatibility::BlockCollision => {
                        let mut new_fork = chain_segment.clone();
                        let (parent_found, _) = new_fork
                            .keep_blocks_from_oldest_to_block_identifier(
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

    pub fn process_block(&mut self, block: StacksBlockData) -> Option<StacksChainEvent> {
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

        let mut fork_updated = None;
        for fork in self.forks.iter_mut() {
            let (block_appended, mut new_fork) = fork.try_append_block(&block);
            if block_appended {
                if let Some(new_fork) = new_fork.take() {
                    self.forks.push(new_fork);
                    fork_updated = self.forks.last_mut();
                } else {
                    fork_updated = Some(fork);
                }
                // A block can only be added to one segment
                break;
            }
        }

        let fork_updated = match fork_updated.take() {
            Some(fork) => fork,
            None => {
                self.orphans.insert(block.block_identifier.clone());
                return None;
            }
        };

        // Process former orphans
        let orphans = self.orphans.clone();
        let mut orphans_to_untrack = HashSet::new();

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
                let block = match self.block_store.get(orphan_block_identifier) {
                    Some(block) => block.clone(),
                    None => continue,
                };

                let (orphan_appended, new_fork) = fork_updated.try_append_block(&block);
                if orphan_appended {
                    applied.insert(orphan_block_identifier);
                    orphans_to_untrack.insert(orphan_block_identifier);
                }
                at_least_one_orphan_appended = at_least_one_orphan_appended || orphan_appended;
            }
        }

        // Update orphans
        for orphan in orphans_to_untrack.into_iter() {
            println!("Dequeuing orphan");
            self.orphans.remove(orphan);
        }

        // Collect confirmed blocks, remove from block store

        // Process prunable forks
        // fork_ids_to_prune.reverse();
        // for fork_id in fork_ids_to_prune {
        //     println!("Pruning fork {}", fork_id);
        //     self.forks.remove(fork_id);
        // }

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
        let canonical_fork = self.forks.get(canonical_fork_id).unwrap().clone();
        if canonical_fork.eq(&previous_canonical_fork) {
            return None;
        }

        let chain_event =
            Some(self.generate_block_chain_event(&canonical_fork, &previous_canonical_fork));

        for fork in self.forks.iter_mut() {
            fork.prune_confirmed_blocks();
        }

        chain_event
    }

    pub fn process_microblocks(
        &mut self,
        microblocks: Vec<StacksMicroblockData>,
    ) -> Option<StacksChainEvent> {
        println!("===============================\nprocess_microblocks");

        // Retrieve anchor block being updated
        let anchor_block_updated = match microblocks.first() {
            Some(microcblock) => microcblock.metadata.anchor_block_identifier.clone(),
            None => return None,
        };

        let mut previous_canonical_micro_fork = None;

        if let (Some(microforks), Some(micro_fork_id)) = (
            self.micro_forks.get(&anchor_block_updated),
            self.canonical_micro_fork_id.get(&anchor_block_updated),
        ) {
            println!(
                "Previous fork selected as canonical: {}",
                microforks[*micro_fork_id]
            );

            previous_canonical_micro_fork = Some(microforks[*micro_fork_id].clone());
        }

        println!(
            "previous_canonical_micro_fork: {:?}",
            previous_canonical_micro_fork
        );

        let mut micro_forks_updated = HashSet::new();

        println!("Microblocks: {:?}", microblocks);

        for mut microblock in microblocks.into_iter() {
            // Temporary patch: the event observer is not returning the block height of the anchor block,
            // we're using the local state to retrieve this missing piece of data.
            if let Some(block) = self
                .block_store
                .get(&microblock.metadata.anchor_block_identifier)
            {
                microblock.metadata.anchor_block_identifier = block.block_identifier.clone();
            }
            // Keep microblock data in memory
            self.microblock_store.insert(
                (
                    microblock.metadata.anchor_block_identifier.clone(),
                    microblock.block_identifier.clone(),
                ),
                microblock.clone(),
            );

            let mut micro_fork_updated = None;

            if microblock.block_identifier.index == 0 {
                println!("Initiating new micro fork {}", microblock.block_identifier);
                let mut microfork = ChainSegment::new();
                microfork.append_block_identifier(&&microblock.block_identifier, false);

                match self.micro_forks.entry(anchor_block_updated.clone()) {
                    Entry::Occupied(microforks) => microforks.into_mut().push(microfork),
                    Entry::Vacant(v) => {
                        v.insert(vec![microfork]);
                    }
                };
                micro_fork_updated = self
                    .micro_forks
                    .get_mut(&anchor_block_updated)
                    .and_then(|microfork| microfork.last_mut())
            } else {
                if let Some(microforks) = self.micro_forks.get_mut(&anchor_block_updated) {
                    for micro_fork in microforks.iter_mut() {
                        let (block_appended, mut new_micro_fork) =
                            micro_fork.try_append_block(&microblock);
                        if block_appended {
                            if let Some(new_micro_fork) = new_micro_fork.take() {
                                microforks.push(new_micro_fork);
                                micro_fork_updated = microforks.last_mut();
                            } else {
                                micro_fork_updated = Some(micro_fork);
                            }
                            // A block can only be added to one segment
                            break;
                        }
                    }
                }
            }

            println!("{:?}", micro_fork_updated);

            let micro_fork_updated = match micro_fork_updated.take() {
                Some(micro_fork) => micro_fork,
                None => {
                    self.micro_orphans.insert((
                        microblock.metadata.anchor_block_identifier.clone(),
                        microblock.block_identifier.clone(),
                    ));
                    continue;
                }
            };

            // Process former orphans
            let orphans = self.orphans.clone();
            let mut orphans_to_untrack = HashSet::new();

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
                    let block = match self.block_store.get(orphan_block_identifier) {
                        Some(block) => block.clone(),
                        None => continue,
                    };

                    let (orphan_appended, new_fork) = micro_fork_updated.try_append_block(&block);
                    if orphan_appended {
                        applied.insert(orphan_block_identifier);
                        orphans_to_untrack.insert(orphan_block_identifier);
                    }
                    at_least_one_orphan_appended = at_least_one_orphan_appended || orphan_appended;
                }
            }

            // Update orphans
            for orphan in orphans_to_untrack.into_iter() {
                println!("Dequeuing orphan");
                self.orphans.remove(orphan);
            }

            // Collect confirmed blocks, remove from block store

            // Process prunable forks
            // fork_ids_to_prune.reverse();
            // for fork_id in fork_ids_to_prune {
            //     println!("Pruning fork {}", fork_id);
            //     self.forks.remove(fork_id);
            // }

            micro_forks_updated.insert(microblock.metadata.anchor_block_identifier);
        }

        if micro_forks_updated.is_empty() {
            return None;
        }

        assert_eq!(micro_forks_updated.len(), 1);
        let microforks = {
            let microforks = self
                .micro_forks
                .get(&anchor_block_updated)
                .expect("unable to retrieve microforks");
            microforks
        };

        // Select canonical fork
        let mut canonical_micro_fork_id = 0;
        let mut highest_height = 0;
        for (fork_id, fork) in microforks.iter().enumerate() {
            println!("Fork Id: {} - {}", fork_id, fork);
            if fork.get_length() >= highest_height {
                highest_height = fork.get_length();
                canonical_micro_fork_id = fork_id;
            }
        }

        self.canonical_micro_fork_id
            .insert(anchor_block_updated.clone(), canonical_micro_fork_id);

        // Generate chain event from the previous and current canonical forks
        let canonical_micro_fork = microforks.get(canonical_micro_fork_id).unwrap();

        println!("Fork selected as canonical: {}", canonical_micro_fork);

        let chain_event = self.generate_microblock_chain_event(
            &anchor_block_updated,
            canonical_micro_fork,
            &previous_canonical_micro_fork,
        );

        println!("==>: {:?}", chain_event);

        for fork in self.forks.iter_mut() {
            fork.prune_confirmed_blocks();
        }

        chain_event
    }

    // We got the confirmed canonical microblock trail,
    // and we want to send a diff with whatever was sent
    // in the past.
    // The issue comes from the following case. If we
    // condider the 3 following forks
    //
    // 1) A1 - B1 - a1 - b1 - c1 - C1
    //
    // 2) A1 - B1 - a1 - b1 - C2
    //
    // 3) A1 - B1 - a1 - b1 - c1 - d1 - C3
    //
    // How can we always be sending back the right diff?
    // As is, if 1) 2) #), we will be sending:
    // - BlockUpdate(C1)
    // - BlockReorg(C2, rollback: [c1], apply: [])
    // - BlockReorg(C3, rollback: [], apply: [c1, d1])

    pub fn confirm_microblocks_for_block(
        &mut self,
        block: &StacksBlockData,
    ) -> Option<StacksChainEvent> {
        match (
            &block.metadata.confirm_microblock_identifier,
            self.micro_forks.get(&block.parent_block_identifier),
        ) {
            (Some(last_microblock), Some(microforks)) => {
                let previous_canonical_segment = match self
                    .canonical_micro_fork_id
                    .get(&block.parent_block_identifier)
                {
                    Some(id) => Some(microforks[*id].clone()),
                    None => None,
                };

                let mut new_canonical_segment = None;
                for (microfork_id, microfork) in microforks.iter().enumerate() {
                    self.canonical_micro_fork_id
                        .insert(block.parent_block_identifier.clone(), microfork_id);
                    if microfork.block_ids.contains(&last_microblock) {
                        let mut confirmed_microblocks = microfork.clone();
                        let (_, mutated) = confirmed_microblocks
                            .keep_blocks_from_oldest_to_block_identifier(&last_microblock);
                        new_canonical_segment = Some((
                            confirmed_microblocks,
                            if mutated {
                                microforks.len()
                            } else {
                                microfork_id
                            },
                        ));
                        break;
                    }
                }

                if let Some((new_canonical_segment, microfork_id)) = new_canonical_segment {
                    let chain_event = self.generate_microblock_chain_event(
                        &block.parent_block_identifier,
                        &new_canonical_segment,
                        &previous_canonical_segment,
                    );

                    // insert confirmed_microblocks in self.micro_forks
                    self.canonical_micro_fork_id
                        .insert(block.parent_block_identifier.clone(), microfork_id);

                    // update self.canonical_micro_fork_id
                    match self
                        .micro_forks
                        .entry(block.parent_block_identifier.clone())
                    {
                        Entry::Occupied(microforks) => {
                            microforks.into_mut().push(new_canonical_segment)
                        }
                        Entry::Vacant(v) => {
                            v.insert(vec![new_canonical_segment]);
                        }
                    };

                    return chain_event;
                }

                None
            }
            _ => None,
        }
    }

    pub fn generate_block_chain_event(
        &mut self,
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
                let block_update = match self.confirm_microblocks_for_block(&block) {
                    Some(ref mut chain_event) => {
                        let mut update = StacksBlockUpdate::new(block);
                        match chain_event {
                            StacksChainEvent::ChainUpdatedWithMicroblocks(data) => {
                                update
                                    .parents_microblocks_to_apply
                                    .append(&mut data.new_microblocks);
                            }
                            StacksChainEvent::ChainUpdatedWithMicroblocksReorg(data) => {
                                update
                                    .parents_microblocks_to_apply
                                    .append(&mut data.microblocks_to_apply);
                                update
                                    .parents_microblocks_to_rollback
                                    .append(&mut data.microblocks_to_rollback);
                            }
                            _ => unreachable!(),
                        };
                        update
                    }
                    None => StacksBlockUpdate::new(block),
                };
                new_blocks.push(block_update)
            }
            return StacksChainEvent::ChainUpdatedWithBlocks(ChainUpdatedWithBlocksData {
                new_blocks,
            });
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
                    let block_update = match self.confirm_microblocks_for_block(&block) {
                        Some(ref mut chain_event) => {
                            let mut update = StacksBlockUpdate::new(block);
                            match chain_event {
                                StacksChainEvent::ChainUpdatedWithMicroblocks(data) => {
                                    update
                                        .parents_microblocks_to_apply
                                        .append(&mut data.new_microblocks);
                                }
                                StacksChainEvent::ChainUpdatedWithMicroblocksReorg(data) => {
                                    update
                                        .parents_microblocks_to_apply
                                        .append(&mut data.microblocks_to_apply);
                                    update
                                        .parents_microblocks_to_rollback
                                        .append(&mut data.microblocks_to_rollback);
                                }
                                _ => unreachable!(),
                            };
                            update
                        }
                        None => StacksBlockUpdate::new(block),
                    };
                    new_blocks.push(block_update)
                }
                return StacksChainEvent::ChainUpdatedWithBlocks(ChainUpdatedWithBlocksData {
                    new_blocks,
                });
            } else {
                return StacksChainEvent::ChainUpdatedWithReorg(ChainUpdatedWithReorgData {
                    blocks_to_rollback: divergence
                        .blocks_to_rollback
                        .iter()
                        .map(|block_id| {
                            let block = match self.block_store.get(block_id) {
                                Some(block) => block.clone(),
                                None => panic!("unable to retrive block from block store"),
                            };
                            let block_update = match self.confirm_microblocks_for_block(&block) {
                                Some(ref mut chain_event) => {
                                    let mut update = StacksBlockUpdate::new(block);
                                    match chain_event {
                                        StacksChainEvent::ChainUpdatedWithMicroblocks(data) => {
                                            update
                                                .parents_microblocks_to_apply
                                                .append(&mut data.new_microblocks);
                                        }
                                        StacksChainEvent::ChainUpdatedWithMicroblocksReorg(
                                            data,
                                        ) => {
                                            update
                                                .parents_microblocks_to_apply
                                                .append(&mut data.microblocks_to_apply);
                                            update
                                                .parents_microblocks_to_rollback
                                                .append(&mut data.microblocks_to_rollback);
                                        }
                                        _ => unreachable!(),
                                    };
                                    update
                                }
                                None => StacksBlockUpdate::new(block),
                            };
                            block_update
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
                            let block_update = match self.confirm_microblocks_for_block(&block) {
                                Some(ref mut chain_event) => {
                                    let mut update = StacksBlockUpdate::new(block);
                                    match chain_event {
                                        StacksChainEvent::ChainUpdatedWithMicroblocks(data) => {
                                            update
                                                .parents_microblocks_to_apply
                                                .append(&mut data.new_microblocks);
                                        }
                                        StacksChainEvent::ChainUpdatedWithMicroblocksReorg(
                                            data,
                                        ) => {
                                            update
                                                .parents_microblocks_to_apply
                                                .append(&mut data.microblocks_to_apply);
                                            update
                                                .parents_microblocks_to_rollback
                                                .append(&mut data.microblocks_to_rollback);
                                        }
                                        _ => unreachable!(),
                                    };
                                    update
                                }
                                None => StacksBlockUpdate::new(block),
                            };
                            block_update
                        })
                        .collect::<Vec<_>>(),
                });
            }
        }
        panic!()
    }

    pub fn generate_microblock_chain_event(
        &self,
        anchor_block_identifier: &BlockIdentifier,
        new_canonical_segment: &ChainSegment,
        previous_canonical_segment: &Option<ChainSegment>,
    ) -> Option<StacksChainEvent> {
        let previous_canonical_segment = match previous_canonical_segment {
            Some(previous_canonical_segment) if !previous_canonical_segment.is_empty() => {
                previous_canonical_segment
            }
            _ => {
                let mut new_microblocks = vec![];
                for i in 0..new_canonical_segment.block_ids.len() {
                    let block_identifier = &new_canonical_segment.block_ids
                        [new_canonical_segment.block_ids.len() - 1 - i];
                    let microblock_identifier =
                        (anchor_block_identifier.clone(), block_identifier.clone());
                    let block = match self.microblock_store.get(&microblock_identifier) {
                        Some(block) => block.clone(),
                        None => panic!("unable to retrive microblock from microblock store"),
                    };
                    new_microblocks.push(block)
                }
                return Some(StacksChainEvent::ChainUpdatedWithMicroblocks(
                    ChainUpdatedWithMicroblocksData { new_microblocks },
                ));
            }
        };

        if new_canonical_segment.eq(&previous_canonical_segment) {
            return None;
        }

        println!("1: {}", previous_canonical_segment);
        println!("2: {}", new_canonical_segment);

        if let Ok(divergence) =
            new_canonical_segment.try_identify_divergence(previous_canonical_segment, true)
        {
            println!("{:?}", divergence);
            if divergence.blocks_to_rollback.is_empty() {
                let mut new_microblocks = vec![];
                for i in 0..divergence.blocks_to_apply.len() {
                    let block_identifier = &new_canonical_segment.block_ids[i];
                    println!("{} -> {}", i, block_identifier);

                    let microblock_identifier =
                        (anchor_block_identifier.clone(), block_identifier.clone());
                    let block = match self.microblock_store.get(&microblock_identifier) {
                        Some(block) => block.clone(),
                        None => {
                            panic!("unable to retrive microblock from microblock store")
                        }
                    };
                    new_microblocks.push(block);
                }
                return Some(StacksChainEvent::ChainUpdatedWithMicroblocks(
                    ChainUpdatedWithMicroblocksData { new_microblocks },
                ));
            } else {
                return Some(StacksChainEvent::ChainUpdatedWithMicroblocksReorg(
                    ChainUpdatedWithMicroblocksReorgData {
                        microblocks_to_rollback: divergence
                            .blocks_to_rollback
                            .iter()
                            .map(|microblock_identifier| {
                                let microblock_identifier = (
                                    anchor_block_identifier.clone(),
                                    microblock_identifier.clone(),
                                );
                                let block = match self.microblock_store.get(&microblock_identifier)
                                {
                                    Some(block) => block.clone(),
                                    None => {
                                        panic!("unable to retrive microblock from microblock store")
                                    }
                                };
                                block
                            })
                            .collect::<Vec<_>>(),
                        microblocks_to_apply: divergence
                            .blocks_to_apply
                            .iter()
                            .map(|microblock_identifier| {
                                let microblock_identifier = (
                                    anchor_block_identifier.clone(),
                                    microblock_identifier.clone(),
                                );
                                let block = match self.microblock_store.get(&microblock_identifier)
                                {
                                    Some(block) => block.clone(),
                                    None => {
                                        panic!("unable to retrive microblock from microblock store")
                                    }
                                };
                                block
                            })
                            .collect::<Vec<_>>(),
                    },
                ));
            }
        }
        None
    }
}
