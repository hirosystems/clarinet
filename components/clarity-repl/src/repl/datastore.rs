use std::collections::HashMap;

use clarity::types::chainstate::BlockHeaderHash;
use clarity::types::chainstate::BurnchainHeaderHash;
use clarity::types::chainstate::ConsensusHash;
use clarity::types::chainstate::SortitionId;
use clarity::types::chainstate::StacksAddress;
use clarity::types::chainstate::StacksBlockId;
use clarity::types::chainstate::VRFSeed;
use clarity::types::StacksEpochId;
use clarity::util::hash::Sha512Trunc256Sum;
use clarity::vm::analysis::AnalysisDatabase;
use clarity::vm::database::BurnStateDB;
use clarity::vm::database::{ClarityBackingStore, HeadersDB};
use clarity::vm::errors::InterpreterResult as Result;
use clarity::vm::types::QualifiedContractIdentifier;
use clarity::vm::types::TupleData;
use clarity::vm::StacksEpoch;
use pox_locking::handle_contract_call_special_cases;
use sha2::{Digest, Sha512_256};

use super::interpreter::BLOCK_LIMIT_MAINNET;

fn epoch_to_peer_version(epoch: StacksEpochId) -> u8 {
    use clarity::consts::*;
    match epoch {
        StacksEpochId::Epoch10 => PEER_VERSION_EPOCH_1_0,
        StacksEpochId::Epoch20 => PEER_VERSION_EPOCH_2_0,
        StacksEpochId::Epoch2_05 => PEER_VERSION_EPOCH_2_05,
        StacksEpochId::Epoch21 => PEER_VERSION_EPOCH_2_1,
        StacksEpochId::Epoch22 => PEER_VERSION_EPOCH_2_2,
        StacksEpochId::Epoch23 => PEER_VERSION_EPOCH_2_3,
        StacksEpochId::Epoch24 => PEER_VERSION_EPOCH_2_4,
        StacksEpochId::Epoch25 => PEER_VERSION_EPOCH_2_5,
        StacksEpochId::Epoch30 => PEER_VERSION_EPOCH_3_0,
    }
}

#[derive(Clone, Debug)]
pub struct Datastore {
    store: HashMap<StacksBlockId, HashMap<String, String>>,
    block_id_lookup: HashMap<StacksBlockId, StacksBlockId>,
    metadata: HashMap<(String, String), String>,
    open_chain_tip: StacksBlockId,
    current_chain_tip: StacksBlockId,
    chain_height: u32,
    height_at_chain_tip: HashMap<StacksBlockId, u32>,
}

#[derive(Clone, Debug)]
pub struct BlockInfo {
    block_header_hash: BlockHeaderHash,
    burn_block_header_hash: BurnchainHeaderHash,
    consensus_hash: ConsensusHash,
    vrf_seed: VRFSeed,
    burn_block_time: u64,
    burn_block_height: u32,
    miner: StacksAddress,
    burnchain_tokens_spent_for_block: u128,
    get_burnchain_tokens_spent_for_winning_block: u128,
    tokens_earned_for_block: u128,
    pox_payout_addrs: (Vec<TupleData>, u128),
}

#[derive(Clone, Debug)]
pub struct StacksConstants {
    pub burn_start_height: u32,
    pub pox_prepare_length: u32,
    pub pox_reward_cycle_length: u32,
    pub pox_rejection_fraction: u64,
}

#[derive(Clone, Debug)]
pub struct BurnDatastore {
    store: HashMap<StacksBlockId, BlockInfo>,
    sortition_lookup: HashMap<SortitionId, StacksBlockId>,
    consensus_hash_lookup: HashMap<ConsensusHash, SortitionId>,
    block_id_lookup: HashMap<StacksBlockId, StacksBlockId>,
    open_chain_tip: StacksBlockId,
    current_chain_tip: StacksBlockId,
    chain_height: u32,
    height_at_chain_tip: HashMap<StacksBlockId, u32>,
    current_epoch: StacksEpochId,
    current_epoch_start_height: u32,
    constants: StacksConstants,
    genesis_time: u64,
}

fn height_to_hashed_bytes(height: u32) -> [u8; 32] {
    let input_bytes = height.to_be_bytes();
    let mut hasher = Sha512_256::new();
    hasher.update(input_bytes);
    let hash = Sha512Trunc256Sum::from_hasher(hasher);
    hash.0
}

fn height_to_id(height: u32) -> StacksBlockId {
    StacksBlockId(height_to_hashed_bytes(height))
}

fn height_to_block(height: u32, genesis_time: Option<u64>) -> BlockInfo {
    let bytes = height_to_hashed_bytes(height);
    let genesis_time = genesis_time.unwrap_or(0);

    let block_header_hash = {
        let mut buffer = bytes;
        buffer[0] = 1;
        BlockHeaderHash(buffer)
    };
    let burn_block_header_hash = {
        let mut buffer = bytes;
        buffer[0] = 2;
        BurnchainHeaderHash(buffer)
    };
    let consensus_hash = {
        let mut buffer = bytes;
        buffer[0] = 3;
        ConsensusHash::from_bytes(&buffer[0..20]).unwrap()
    };
    let vrf_seed = {
        let mut buffer = bytes;
        buffer[0] = 4;
        VRFSeed(buffer)
    };
    let time_since_genesis: u64 = (height * 1800).into();
    let burn_block_time: u64 = genesis_time + time_since_genesis;
    let burn_block_height = height;
    let miner = StacksAddress::burn_address(true);
    let burnchain_tokens_spent_for_block = 2000;
    let get_burnchain_tokens_spent_for_winning_block = 2000;
    let tokens_earned_for_block = 5000;
    let pox_payout_addrs = (vec![], 0_u128);

    BlockInfo {
        block_header_hash,
        burn_block_header_hash,
        consensus_hash,
        vrf_seed,
        burn_block_time,
        burn_block_height,
        miner,
        burnchain_tokens_spent_for_block,
        get_burnchain_tokens_spent_for_winning_block,
        tokens_earned_for_block,
        pox_payout_addrs,
    }
}

impl Default for Datastore {
    fn default() -> Self {
        Self::new()
    }
}

impl Datastore {
    pub fn new() -> Self {
        let id = height_to_id(0);

        let mut store = HashMap::new();
        store.insert(id, HashMap::new());

        let mut block_id_lookup = HashMap::new();
        block_id_lookup.insert(id, id);

        let mut id_height_map = HashMap::new();
        id_height_map.insert(id, 0);

        Self {
            store,
            block_id_lookup,
            metadata: HashMap::new(),
            open_chain_tip: id,
            current_chain_tip: id,
            chain_height: 0,
            height_at_chain_tip: id_height_map,
        }
    }

    pub fn advance_chain_tip(&mut self, count: u32) -> u32 {
        let cur_height = self.chain_height;
        let current_lookup_id = *self
            .block_id_lookup
            .get(&self.open_chain_tip)
            .expect("Open chain tip missing in block id lookup table");

        for i in 1..=count {
            let height = cur_height + i;
            let id = height_to_id(height);

            self.block_id_lookup.insert(id, current_lookup_id);
            self.height_at_chain_tip.insert(id, height);
        }

        self.chain_height += count;
        self.open_chain_tip = height_to_id(self.chain_height);
        self.current_chain_tip = self.open_chain_tip;
        self.chain_height
    }
}

impl ClarityBackingStore for Datastore {
    fn put_all_data(&mut self, items: Vec<(String, String)>) -> Result<()> {
        for (key, value) in items {
            self.put(&key, &value);
        }
        Ok(())
    }

    /// fetch K-V out of the committed datastore
    fn get_data(&mut self, key: &str) -> Result<Option<String>> {
        let lookup_id = self
            .block_id_lookup
            .get(&self.current_chain_tip)
            .expect("Could not find current chain tip in block_id_lookup map");

        if let Some(map) = self.store.get(lookup_id) {
            Ok(map.get(key).cloned())
        } else {
            panic!("Block does not exist for current chain tip");
        }
    }

    fn has_entry(&mut self, key: &str) -> Result<bool> {
        Ok(self.get_data(key)?.is_some())
    }

    /// change the current MARF context to service reads from a different chain_tip
    ///   used to implement time-shifted evaluation.
    /// returns the previous block header hash on success
    fn set_block_hash(&mut self, bhh: StacksBlockId) -> Result<StacksBlockId> {
        let prior_tip = self.open_chain_tip;
        self.current_chain_tip = bhh;
        Ok(prior_tip)
    }

    fn get_block_at_height(&mut self, height: u32) -> Option<StacksBlockId> {
        Some(height_to_id(height))
    }

    /// this function returns the current block height, as viewed by this marfed-kv structure,
    ///  i.e., it changes on time-shifted evaluation. the open_chain_tip functions always
    ///   return data about the chain tip that is currently open for writing.
    fn get_current_block_height(&mut self) -> u32 {
        *self
            .height_at_chain_tip
            .get(self.get_chain_tip())
            .unwrap_or(&u32::MAX)
    }

    fn get_open_chain_tip_height(&mut self) -> u32 {
        self.chain_height
    }

    fn get_open_chain_tip(&mut self) -> StacksBlockId {
        self.open_chain_tip
    }

    /// The contract commitment is the hash of the contract, plus the block height in
    ///   which the contract was initialized.
    fn make_contract_commitment(&mut self, _contract_hash: Sha512Trunc256Sum) -> String {
        "".to_string()
    }

    fn insert_metadata(
        &mut self,
        contract: &QualifiedContractIdentifier,
        key: &str,
        value: &str,
    ) -> Result<()> {
        // let bhh = self.get_open_chain_tip();
        // self.get_side_store().insert_metadata(&bhh, &contract.to_string(), key, value)
        self.metadata
            .insert((contract.to_string(), key.to_string()), value.to_string());
        Ok(())
    }

    fn get_metadata(
        &mut self,
        contract: &QualifiedContractIdentifier,
        key: &str,
    ) -> Result<Option<String>> {
        // let (bhh, _) = self.get_contract_hash(contract)?;
        // Ok(self.get_side_store().get_metadata(&bhh, &contract.to_string(), key))
        let key = &(contract.to_string(), key.to_string());

        match self.metadata.get(key) {
            Some(result) => Ok(Some(result.to_string())),
            None => Ok(None),
        }
    }

    fn get_data_with_proof(&mut self, _key: &str) -> Result<Option<(String, Vec<u8>)>> {
        Ok(None)
    }

    fn get_contract_hash(
        &mut self,
        _contract: &QualifiedContractIdentifier,
    ) -> Result<(StacksBlockId, Sha512Trunc256Sum)> {
        panic!("Datastore cannot get_contract_hash")
    }

    fn get_metadata_manual(
        &mut self,
        _at_height: u32,
        _contract: &QualifiedContractIdentifier,
        _key: &str,
    ) -> Result<Option<String>> {
        panic!("Datastore cannot get_metadata_manual")
    }

    fn get_cc_special_cases_handler(&self) -> Option<clarity::vm::database::SpecialCaseHandler> {
        Some(&handle_contract_call_special_cases)
    }

    #[cfg(any(feature = "cli", feature = "cli"))]
    fn get_side_store(&mut self) -> &::clarity::rusqlite::Connection {
        panic!("Datastore cannot get_side_store")
    }
}

impl BurnDatastore {
    pub fn new(constants: StacksConstants) -> Self {
        let bytes = height_to_hashed_bytes(0);
        let id = StacksBlockId(bytes);
        let sortition_id = SortitionId(bytes);
        let genesis_time = chrono::Utc::now().timestamp() as u64;

        let genesis_block = BlockInfo {
            block_header_hash: BlockHeaderHash([0x00; 32]),
            burn_block_header_hash: BurnchainHeaderHash([0x00; 32]),
            consensus_hash: ConsensusHash([0x00; 20]),
            vrf_seed: VRFSeed([0x00; 32]),
            burn_block_time: genesis_time,
            burn_block_height: 0,
            miner: StacksAddress::burn_address(false),
            burnchain_tokens_spent_for_block: 0,
            get_burnchain_tokens_spent_for_winning_block: 0,
            tokens_earned_for_block: 0,
            pox_payout_addrs: (vec![], 0),
        };

        let mut height_at_chain_tip = HashMap::new();
        height_at_chain_tip.insert(id, 0);

        let mut sortition_lookup = HashMap::new();
        sortition_lookup.insert(sortition_id, id);

        let mut consensus_hash_lookup = HashMap::new();
        consensus_hash_lookup.insert(genesis_block.consensus_hash, sortition_id);

        let mut store = HashMap::new();
        store.insert(id, genesis_block);

        let mut block_id_lookup = HashMap::new();
        block_id_lookup.insert(id, id);

        let mut id_height_map = HashMap::new();
        id_height_map.insert(id, 0);

        BurnDatastore {
            store,
            sortition_lookup,
            consensus_hash_lookup,
            block_id_lookup,
            open_chain_tip: id,
            current_chain_tip: id,
            chain_height: 0,
            height_at_chain_tip,
            current_epoch: StacksEpochId::Epoch2_05,
            current_epoch_start_height: 0,
            constants,
            genesis_time,
        }
    }

    pub fn get_current_epoch(&self) -> StacksEpochId {
        self.current_epoch
    }

    pub fn get_current_block_height(&self) -> u32 {
        self.chain_height
    }
    pub fn advance_chain_tip(&mut self, count: u32) -> u32 {
        let cur_height = self.chain_height;
        let current_lookup_id = *self
            .block_id_lookup
            .get(&self.open_chain_tip)
            .expect("Open chain tip missing in block id lookup table");
        let genesis_time = self.genesis_time;

        for i in 1..=count {
            let height = cur_height + i;
            let bytes = height_to_hashed_bytes(height);
            let id = StacksBlockId(bytes);
            let sortition_id = SortitionId(bytes);
            let block_info = height_to_block(height, Some(genesis_time));
            self.block_id_lookup.insert(id, current_lookup_id);
            self.height_at_chain_tip.insert(id, height);
            self.sortition_lookup.insert(sortition_id, id);
            self.consensus_hash_lookup
                .insert(block_info.consensus_hash, sortition_id);
            self.store.insert(id, block_info);
        }

        self.chain_height += count;
        self.open_chain_tip = height_to_id(self.chain_height);
        self.current_chain_tip = self.open_chain_tip;
        self.chain_height
    }

    pub fn set_current_epoch(&mut self, epoch: StacksEpochId) {
        self.current_epoch = epoch;
        self.current_epoch_start_height = self.chain_height;
    }
}

impl HeadersDB for BurnDatastore {
    // fn get(&mut self, key: &str) -> Option<String> {
    //     let lookup_id = self
    //         .block_id_lookup
    //         .get(&self.current_chain_tip)
    //         .expect("Could not find current chain tip in block_id_lookup map");

    //     if let Some(map) = self.store.get(lookup_id) {
    //         map.get(key).map(|v| v.clone())
    //     } else {
    //         panic!("Block does not exist for current chain tip");
    //     }
    // }

    fn get_stacks_block_header_hash_for_block(
        &self,
        id_bhh: &StacksBlockId,
        _epoch_id: &StacksEpochId,
    ) -> Option<BlockHeaderHash> {
        self.store.get(id_bhh).map(|id| id.block_header_hash)
    }

    fn get_burn_header_hash_for_block(
        &self,
        id_bhh: &StacksBlockId,
    ) -> Option<BurnchainHeaderHash> {
        self.store.get(id_bhh).map(|id| id.burn_block_header_hash)
    }

    fn get_consensus_hash_for_block(
        &self,
        id_bhh: &StacksBlockId,
        _epoch_id: &StacksEpochId,
    ) -> Option<ConsensusHash> {
        self.store.get(id_bhh).map(|id| id.consensus_hash)
    }
    fn get_vrf_seed_for_block(
        &self,
        id_bhh: &StacksBlockId,
        _epoch_id: &StacksEpochId,
    ) -> Option<VRFSeed> {
        self.store.get(id_bhh).map(|id| id.vrf_seed)
    }
    fn get_stacks_block_time_for_block(&self, id_bhh: &StacksBlockId) -> Option<u64> {
        self.store.get(id_bhh).map(|id| id.burn_block_time)
    }
    fn get_burn_block_time_for_block(
        &self,
        id_bhh: &StacksBlockId,
        _epoch_id: Option<&StacksEpochId>,
    ) -> Option<u64> {
        self.store.get(id_bhh).map(|id| id.burn_block_time)
    }
    fn get_burn_block_height_for_block(&self, id_bhh: &StacksBlockId) -> Option<u32> {
        self.store.get(id_bhh).map(|id| id.burn_block_height)
    }
    fn get_miner_address(
        &self,
        id_bhh: &StacksBlockId,
        _epoch_id: &StacksEpochId,
    ) -> Option<StacksAddress> {
        self.store.get(id_bhh).map(|id| id.miner)
    }
    fn get_burnchain_tokens_spent_for_block(
        &self,
        id_bhh: &StacksBlockId,
        _epoch_id: &StacksEpochId,
    ) -> Option<u128> {
        self.store
            .get(id_bhh)
            .map(|id| id.burnchain_tokens_spent_for_block)
    }
    fn get_burnchain_tokens_spent_for_winning_block(
        &self,
        id_bhh: &StacksBlockId,
        _epoch_id: &StacksEpochId,
    ) -> Option<u128> {
        self.store
            .get(id_bhh)
            .map(|id| id.get_burnchain_tokens_spent_for_winning_block)
    }
    fn get_tokens_earned_for_block(
        &self,
        id_bhh: &StacksBlockId,
        _epoch_id: &StacksEpochId,
    ) -> Option<u128> {
        self.store.get(id_bhh).map(|id| id.tokens_earned_for_block)
    }
}

impl BurnStateDB for BurnDatastore {
    fn get_v1_unlock_height(&self) -> u32 {
        0
    }

    fn get_v2_unlock_height(&self) -> u32 {
        0
    }

    fn get_v3_unlock_height(&self) -> u32 {
        0
    }

    fn get_pox_3_activation_height(&self) -> u32 {
        0
    }

    fn get_pox_4_activation_height(&self) -> u32 {
        0
    }

    fn get_tip_burn_block_height(&self) -> Option<u32> {
        Some(self.chain_height)
    }

    fn get_tip_sortition_id(&self) -> Option<SortitionId> {
        let bytes = height_to_hashed_bytes(self.chain_height);
        let sortition_id = SortitionId(bytes);
        Some(sortition_id)
    }

    /// Returns the *burnchain block height* for the `sortition_id` is associated with.
    fn get_burn_block_height(&self, sortition_id: &SortitionId) -> Option<u32> {
        self.sortition_lookup
            .get(sortition_id)
            .and_then(|id| self.store.get(id))
            .map(|block_info| block_info.burn_block_height)
    }

    /// Returns the height of the burnchain when the Stacks chain started running.
    fn get_burn_start_height(&self) -> u32 {
        self.constants.burn_start_height
    }

    fn get_pox_prepare_length(&self) -> u32 {
        self.constants.pox_prepare_length
    }

    fn get_pox_reward_cycle_length(&self) -> u32 {
        self.constants.pox_reward_cycle_length
    }

    fn get_pox_rejection_fraction(&self) -> u64 {
        self.constants.pox_rejection_fraction
    }

    /// Returns the burnchain header hash for the given burn block height, as queried from the given SortitionId.
    ///
    /// Returns Some if `self.get_burn_start_height() <= height < self.get_burn_block_height(sorition_id)`, and None otherwise.
    fn get_burn_header_hash(
        &self,
        _height: u32,
        sortition_id: &SortitionId,
    ) -> Option<BurnchainHeaderHash> {
        self.sortition_lookup
            .get(sortition_id)
            .and_then(|id| self.store.get(id))
            .map(|block_info| block_info.burn_block_header_hash)
    }

    /// Lookup a `SortitionId` keyed to a `ConsensusHash`.
    ///
    /// Returns None if no block found.
    fn get_sortition_id_from_consensus_hash(
        &self,
        consensus_hash: &ConsensusHash,
    ) -> Option<SortitionId> {
        self.consensus_hash_lookup.get(consensus_hash).copied()
    }

    /// The epoch is defined as by a start and end height. This returns
    /// the epoch enclosing `height`.
    fn get_stacks_epoch(&self, _height: u32) -> Option<StacksEpoch> {
        Some(StacksEpoch {
            epoch_id: self.current_epoch,
            start_height: self.current_epoch_start_height.into(),
            end_height: u64::MAX,
            block_limit: BLOCK_LIMIT_MAINNET,
            network_epoch: epoch_to_peer_version(self.current_epoch),
        })
    }

    fn get_stacks_epoch_by_epoch_id(&self, _epoch_id: &StacksEpochId) -> Option<StacksEpoch> {
        self.get_stacks_epoch(0)
    }

    /// Get the PoX payout addresses for a given burnchain block
    fn get_pox_payout_addrs(
        &self,
        _height: u32,
        sortition_id: &SortitionId,
    ) -> Option<(Vec<TupleData>, u128)> {
        self.sortition_lookup
            .get(sortition_id)
            .and_then(|id| self.store.get(id))
            .map(|block_info| block_info.pox_payout_addrs.clone())
    }

    fn get_ast_rules(&self, _height: u32) -> clarity::vm::ast::ASTRules {
        clarity::vm::ast::ASTRules::PrecheckSize
    }
}

impl Datastore {
    pub fn open(_path_str: &str, _miner_tip: Option<&StacksBlockId>) -> Result<Datastore> {
        Ok(Datastore::new())
    }

    pub fn as_analysis_db(&mut self) -> AnalysisDatabase<'_> {
        AnalysisDatabase::new(self)
    }

    /// begin, commit, rollback a save point identified by key
    ///    this is used to clean up any data from aborted blocks
    ///     (NOT aborted transactions that is handled by the clarity vm directly).
    /// The block header hash is used for identifying savepoints.
    ///     this _cannot_ be used to rollback to arbitrary prior block hash, because that
    ///     blockhash would already have committed and no longer exist in the save point stack.
    /// this is a "lower-level" rollback than the roll backs performed in
    ///   ClarityDatabase or AnalysisDatabase -- this is done at the backing store level.

    pub fn begin(&mut self, _current: &StacksBlockId, _next: &StacksBlockId) {
        // self.marf.begin(current, next)
        //     .expect(&format!("ERROR: Failed to begin new MARF block {} - {})", current, next));
        // self.chain_tip = self.marf.get_open_chain_tip()
        //     .expect("ERROR: Failed to get open MARF")
        //     .clone();
        // self.side_store.begin(&self.chain_tip);
    }
    pub fn rollback(&mut self) {
        // self.marf.drop_current();
        // self.side_store.rollback(&self.chain_tip);
        // self.chain_tip = StacksBlockId::sentinel();
    }
    // This is used by miners
    //   so that the block validation and processing logic doesn't
    //   reprocess the same data as if it were already loaded
    pub fn commit_mined_block(&mut self, _will_move_to: &StacksBlockId) {
        // rollback the side_store
        //    the side_store shouldn't commit data for blocks that won't be
        //    included in the processed chainstate (like a block constructed during mining)
        //    _if_ for some reason, we do want to be able to access that mined chain state in the future,
        //    we should probably commit the data to a different table which does not have uniqueness constraints.
        // self.side_store.rollback(&self.chain_tip);
        // self.marf.commit_mined(will_move_to)
        //     .expect("ERROR: Failed to commit MARF block");
    }
    pub fn commit_to(&mut self, _final_bhh: &StacksBlockId) {
        // println!("commit_to({})", final_bhh);
        // self.side_store.commit_metadata_to(&self.chain_tip, final_bhh);
        // self.side_store.commit(&self.chain_tip);
        // self.marf.commit_to(final_bhh)
        //     .expect("ERROR: Failed to commit MARF block");
    }
    pub fn get_chain_tip(&self) -> &StacksBlockId {
        &self.current_chain_tip
    }

    pub fn set_chain_tip(&mut self, bhh: &StacksBlockId) {
        self.current_chain_tip = *bhh;
    }

    pub fn put(&mut self, key: &str, value: &str) {
        let lookup_id = self
            .block_id_lookup
            .get(&self.open_chain_tip)
            .expect("Could not find current chain tip in block_id_lookup map");

        // if there isn't a store for the open chain_tip, make one and update the
        // entry for the block id in the lookup table
        if *lookup_id != self.open_chain_tip {
            self.store.insert(
                self.open_chain_tip,
                self.store
                    .get(lookup_id)
                    .unwrap_or_else(|| panic!("Block with ID {:?} does not exist", lookup_id))
                    .clone(),
            );

            self.block_id_lookup
                .insert(self.open_chain_tip, self.current_chain_tip);
        }

        if let Some(map) = self.store.get_mut(&self.open_chain_tip) {
            map.insert(key.to_string(), value.to_string());
        } else {
            panic!("Block does not exist for current chain tip");
        }
    }

    pub fn make_contract_hash_key(contract: &QualifiedContractIdentifier) -> String {
        format!("clarity-contract::{}", contract)
    }
}

#[cfg(test)]
mod tests {
    use clarity::types::StacksEpoch;

    use super::*;

    fn get_burn_datastore() -> BurnDatastore {
        let constants = StacksConstants {
            burn_start_height: 0,
            pox_prepare_length: 50,
            pox_reward_cycle_length: 1050,
            pox_rejection_fraction: 0,
        };
        BurnDatastore::new(constants)
    }

    #[test]
    fn test_advance_chain_tip() {
        let mut datastore = get_burn_datastore();
        datastore.advance_chain_tip(5);
        assert_eq!(datastore.chain_height, 5);
    }

    #[test]
    fn test_set_current_epoch() {
        let mut datastore = get_burn_datastore();
        let epoch_id = StacksEpochId::Epoch25;
        datastore.set_current_epoch(epoch_id);
        assert_eq!(datastore.current_epoch, epoch_id);
    }

    #[test]
    fn test_get_v1_unlock_height() {
        let datastore = get_burn_datastore();
        assert_eq!(datastore.get_v1_unlock_height(), 0);
    }

    #[test]
    fn test_get_v2_unlock_height() {
        let datastore = get_burn_datastore();
        assert_eq!(datastore.get_v2_unlock_height(), 0);
    }

    #[test]
    fn test_get_v3_unlock_height() {
        let datastore = get_burn_datastore();
        assert_eq!(datastore.get_v3_unlock_height(), 0);
    }

    #[test]
    fn test_get_pox_3_activation_height() {
        let datastore = get_burn_datastore();
        assert_eq!(datastore.get_pox_3_activation_height(), 0);
    }

    #[test]
    fn test_get_pox_4_activation_height() {
        let datastore = get_burn_datastore();
        assert_eq!(datastore.get_pox_4_activation_height(), 0);
    }

    #[test]
    fn test_get_tip_burn_block_height() {
        let mut datastore = get_burn_datastore();
        let chain_height = 10;
        datastore.chain_height = chain_height;
        let tip_burn_block_height = datastore.get_tip_burn_block_height();
        assert_eq!(tip_burn_block_height, Some(chain_height));
    }

    #[test]
    fn test_get_burn_start_height() {
        let datastore = get_burn_datastore();
        assert_eq!(datastore.get_burn_start_height(), 0);
    }

    #[test]
    fn test_get_pox_prepare_length() {
        let datastore = get_burn_datastore();
        assert_eq!(datastore.get_pox_prepare_length(), 50);
    }

    #[test]
    fn test_get_pox_reward_cycle_length() {
        let datastore = get_burn_datastore();
        assert_eq!(datastore.get_pox_reward_cycle_length(), 1050);
    }

    #[test]
    fn test_get_pox_rejection_fraction() {
        let datastore = get_burn_datastore();
        assert_eq!(datastore.get_pox_rejection_fraction(), 0);
    }

    #[test]
    fn test_get_stacks_epoch() {
        let datastore = get_burn_datastore();
        let height = 10;
        let epoch = datastore.get_stacks_epoch(height);
        assert_eq!(
            epoch,
            Some(StacksEpoch {
                epoch_id: StacksEpochId::Epoch2_05,
                start_height: 0,
                end_height: u64::MAX,
                block_limit: BLOCK_LIMIT_MAINNET,
                network_epoch: clarity::consts::PEER_VERSION_EPOCH_2_05,
            })
        );
    }

    #[test]
    fn test_get_stacks_epoch_by_epoch_id() {
        let datastore = get_burn_datastore();
        let epoch_id = StacksEpochId::Epoch2_05;
        let epoch = datastore.get_stacks_epoch_by_epoch_id(&epoch_id);
        assert_eq!(
            epoch,
            Some(StacksEpoch {
                epoch_id: StacksEpochId::Epoch2_05,
                start_height: 0,
                end_height: u64::MAX,
                block_limit: BLOCK_LIMIT_MAINNET,
                network_epoch: clarity::consts::PEER_VERSION_EPOCH_2_05,
            })
        );
    }
}
