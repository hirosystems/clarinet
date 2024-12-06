use std::collections::HashMap;

use clarity::types::chainstate::{
    BlockHeaderHash, BurnchainHeaderHash, ConsensusHash, SortitionId, StacksAddress, StacksBlockId,
    TrieHash, VRFSeed,
};
use clarity::types::StacksEpochId;
use clarity::util::hash::Sha512Trunc256Sum;
use clarity::vm::analysis::AnalysisDatabase;
use clarity::vm::database::BurnStateDB;
use clarity::vm::database::{ClarityBackingStore, HeadersDB};
use clarity::vm::errors::InterpreterResult as Result;
use clarity::vm::types::{QualifiedContractIdentifier, TupleData};
use clarity::vm::StacksEpoch;
use pox_locking::handle_contract_call_special_cases;
use sha2::{Digest, Sha512_256};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsValue;
#[cfg(target_arch = "wasm32")]
use web_sys::js_sys::{Function as JsFunction, JsString, Uint8Array};

use super::interpreter::BLOCK_LIMIT_MAINNET;
use super::settings::RemoteDataSettings;

const SECONDS_BETWEEN_BURN_BLOCKS: u64 = 600;
const SECONDS_BETWEEN_STACKS_BLOCKS: u64 = 10;

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
        StacksEpochId::Epoch31 => PEER_VERSION_EPOCH_3_1,
    }
}

#[derive(Clone, Debug)]
struct StoreEntry(StacksBlockId, String);

#[derive(Clone, Debug)]
pub struct ClarityDatastore {
    open_chain_tip: StacksBlockId,
    current_chain_tip: StacksBlockId,
    store: HashMap<String, Vec<StoreEntry>>,
    metadata: HashMap<(String, String), String>,
    block_id_lookup: HashMap<StacksBlockId, StacksBlockId>,
    height_at_chain_tip: HashMap<StacksBlockId, u32>,

    remote_data_settings: RemoteDataSettings,
    remote_chaintip_cache: HashMap<u32, String>,

    #[cfg(target_arch = "wasm32")]
    http_client: Option<JsFunction>,
}

#[derive(Deserialize)]
struct ClarityDataResponse {
    pub data: String,
}

#[derive(Deserialize)]
struct BlockInfoResponse {
    pub index_block_hash: String,
}

#[derive(Clone, Debug)]
struct BurnBlockInfo {
    burn_block_time: u64,
    burn_block_height: u32,
}

#[derive(Clone, Debug)]
pub struct StacksBlockInfo {
    block_header_hash: BlockHeaderHash,
    burn_block_header_hash: BurnchainHeaderHash,
    consensus_hash: ConsensusHash,
    vrf_seed: VRFSeed,
    stacks_block_time: u64,
}

#[derive(Clone, Debug)]
pub struct StacksConstants {
    pub burn_start_height: u32,
    pub pox_prepare_length: u32,
    pub pox_reward_cycle_length: u32,
    pub pox_rejection_fraction: u64,
}

impl Default for StacksConstants {
    fn default() -> Self {
        StacksConstants {
            burn_start_height: 0,
            pox_prepare_length: 50,
            pox_reward_cycle_length: 1050,
            pox_rejection_fraction: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Datastore {
    genesis_id: StacksBlockId,
    burn_chain_height: u32,
    burn_blocks: HashMap<BurnchainHeaderHash, BurnBlockInfo>,
    stacks_chain_height: u32,
    stacks_blocks: HashMap<StacksBlockId, StacksBlockInfo>,
    sortition_lookup: HashMap<SortitionId, StacksBlockId>,
    tenure_blocks_height: HashMap<u32, u32>,
    consensus_hash_lookup: HashMap<ConsensusHash, SortitionId>,
    current_epoch: StacksEpochId,
    current_epoch_start_height: u32,
    constants: StacksConstants,
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

fn height_to_burn_block_header_hash(height: u32) -> BurnchainHeaderHash {
    let mut bytes = height_to_hashed_bytes(height);
    bytes[0] = 2;
    BurnchainHeaderHash(bytes)
}

impl Default for ClarityDatastore {
    fn default() -> Self {
        Self::new(RemoteDataSettings::default())
    }
}

impl ClarityDatastore {
    pub fn new(remote_data_settings: RemoteDataSettings) -> Self {
        let block_height = if remote_data_settings.enabled {
            remote_data_settings.initial_height.unwrap_or(32)
        } else {
            0
        };
        let id = height_to_id(block_height);
        Self {
            open_chain_tip: id,
            current_chain_tip: id,
            store: HashMap::new(),
            metadata: HashMap::new(),
            block_id_lookup: HashMap::from([(id, id)]),
            height_at_chain_tip: HashMap::from([(id, block_height)]),

            remote_data_settings,
            remote_chaintip_cache: HashMap::new(),
            #[cfg(target_arch = "wasm32")]
            http_client: None,
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn set_http_client(&mut self, http_client: web_sys::js_sys::Function) {
        self.http_client = Some(http_client);
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

    fn put(&mut self, key: &str, value: &str) {
        if let Some(entries) = self.store.get_mut(key) {
            entries.push(StoreEntry(self.open_chain_tip, value.to_string()));
        } else {
            self.store.insert(
                key.to_string(),
                vec![StoreEntry(self.open_chain_tip, value.to_string())],
            );
        }
    }

    fn get_latest_data(&self, data: &[StoreEntry]) -> Option<String> {
        let StoreEntry(tip, value) = data.last()?;

        if self.height_at_chain_tip.get(tip)?
            <= self.height_at_chain_tip.get(&self.current_chain_tip)?
        {
            Some(value.clone())
        } else {
            self.get_latest_data(&data[..data.len() - 1])
        }
    }

    pub fn make_contract_hash_key(contract: &QualifiedContractIdentifier) -> String {
        format!("clarity-contract::{}", contract)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn fetch_data<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<Option<T>> {
        let url = format!("{}{}", self.remote_data_settings.api_url, path);
        println!("fetching: {}", url);
        let response =
            reqwest::blocking::get(url).unwrap_or_else(|e| panic!("unable to fetch data: {}", e));

        match response.json::<T>() {
            Ok(data) => Ok(Some(data)),
            _ => Ok(None),
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn fetch_data<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<Option<T>> {
        let url = JsString::from(format!("{}{}", self.remote_data_settings.api_url, path));

        match self.http_client {
            Some(ref client) => {
                let response = client.call2(&JsValue::NULL, &JsString::from("GET"), &url);
                if let Ok(response) = response {
                    let bytes = Uint8Array::from(response).to_vec();
                    let raw_result = std::str::from_utf8(bytes.as_slice()).unwrap();
                    match serde_json::from_str::<T>(raw_result) {
                        Ok(data) => Ok(Some(data)),
                        _ => Ok(None),
                    }
                } else {
                    panic!("unable to fetch data: {:?}", response);
                }
            }
            None => panic!("http client not set"),
        }
    }

    fn get_remote_chaintip(&mut self, height: u32) -> String {
        if let Some(cached) = self.remote_chaintip_cache.get(&height) {
            println!("using cached chaintip for height: {}", height);
            return cached.to_string();
        }
        println!("fetching remote chaintip for height: {}", height);

        let url = format!("/extended/v2/blocks/{}", height);

        let data = self
            .fetch_data::<BlockInfoResponse>(&url)
            .unwrap_or_else(|e| {
                panic!("unable to parse json, error: {}", e);
            })
            .unwrap_or_else(|| {
                panic!("unable to get remote chaintip");
            });

        let block_hash = data.index_block_hash.replacen("0x", "", 1);
        self.remote_chaintip_cache
            .insert(height, block_hash.clone());

        block_hash
    }

    fn fetch_remote_data(&mut self, path: &str) -> Result<Option<String>> {
        let data = self
            .fetch_data::<ClarityDataResponse>(path)
            .unwrap_or_else(|e| {
                panic!("unable to parse json, error: {}", e);
            });

        match data {
            Some(data) => {
                let value = if data.data.starts_with("0x") {
                    data.data[2..].to_string()
                } else {
                    data.data
                };
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    fn fetch_clarity_marf_value(&mut self, key: &str) -> Result<Option<String>> {
        let key_hash = TrieHash::from_key(key);
        // the block height should be the min value between current block height and initial height
        // making sure we don't ever try reading data on remote network with a higher block height than the initial one
        let block_height = self.get_current_block_height();
        let remote_chaintip = self.get_remote_chaintip(block_height);
        let path = format!(
            "/v2/clarity/marf/{}?tip={}&proof=false",
            key_hash, remote_chaintip
        );
        self.fetch_remote_data(&path)
    }

    fn fetch_clarity_metadata(
        &mut self,
        contract: &QualifiedContractIdentifier,
        key: &str,
    ) -> Result<Option<String>> {
        let addr = contract.issuer.to_string();
        let contract = contract.name.to_string();
        // the block height should be the min value between current block height and initial height
        // making sure we don't ever try reading data on remote network with a higher block height than the initial one
        let block_height = self.get_current_block_height();
        let remote_chaintip = self.get_remote_chaintip(block_height);
        uprint!(
            "fetching METADATA from network, {}/{}/{}",
            addr,
            contract,
            key
        );

        let url = format!(
            "/v2/clarity/metadata/{}/{}/{}?tip={}",
            addr, contract, key, remote_chaintip
        );
        self.fetch_remote_data(&url)
    }
}

impl ClarityBackingStore for ClarityDatastore {
    fn put_all_data(&mut self, items: Vec<(String, String)>) -> Result<()> {
        for (key, value) in items {
            self.put(&key, &value);
        }
        Ok(())
    }

    /// fetch K-V out of the committed datastore
    fn get_data(&mut self, key: &str) -> Result<Option<String>> {
        println!("get_data({})", key);

        match self.store.get(key) {
            Some(data) => Ok(self.get_latest_data(data)),
            None => {
                if self.remote_data_settings.enabled {
                    let data = self.fetch_clarity_marf_value(key);
                    if let Ok(Some(value)) = &data {
                        self.put(key, value);
                    }
                    return data;
                }
                Ok(None)
            }
        }
    }

    fn get_data_from_path(&mut self, _hash: &TrieHash) -> Result<Option<String>> {
        unreachable!()
    }

    fn get_data_with_proof(&mut self, _key: &str) -> Result<Option<(String, Vec<u8>)>> {
        Ok(None)
    }

    fn get_data_with_proof_from_path(
        &mut self,
        _hash: &TrieHash,
    ) -> Result<Option<(String, Vec<u8>)>> {
        unreachable!()
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
            .get(&self.current_chain_tip)
            .unwrap_or(&u32::MAX)
    }

    fn get_open_chain_tip_height(&mut self) -> u32 {
        self.height_at_chain_tip
            .get(&self.open_chain_tip)
            .copied()
            .unwrap_or(u32::MAX)
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
        self.metadata
            .insert((contract.to_string(), key.to_string()), value.to_string());
        Ok(())
    }

    fn get_metadata(
        &mut self,
        contract: &QualifiedContractIdentifier,
        key: &str,
    ) -> Result<Option<String>> {
        println!("get_metadata({}, {})", contract, key);
        let metadata = self.metadata.get(&(contract.to_string(), key.to_string()));
        if metadata.is_some() {
            return Ok(metadata.cloned());
        }
        if self.remote_data_settings.enabled {
            let data = self.fetch_clarity_metadata(contract, key);
            if let Ok(Some(value)) = &data {
                let _ = self.insert_metadata(contract, key, value);
            }
            return data;
        }
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

    #[cfg(feature = "sdk")]
    fn get_side_store(&mut self) -> &::clarity::rusqlite::Connection {
        panic!("Datastore cannot get_side_store")
    }
}

impl Default for Datastore {
    fn default() -> Self {
        Self::new(RemoteDataSettings::default(), StacksConstants::default())
    }
}

impl Datastore {
    pub fn new(remote_data_settings: RemoteDataSettings, constants: StacksConstants) -> Self {
        let block_height = if remote_data_settings.enabled {
            remote_data_settings.initial_height.unwrap_or(32)
        } else {
            0
        };
        let burn_block_height = if remote_data_settings.enabled { 145 } else { 0 };
        let bytes = height_to_hashed_bytes(block_height);
        let id = StacksBlockId(bytes);
        let sortition_id = SortitionId(bytes);
        let genesis_time = chrono::Utc::now().timestamp() as u64;

        let first_burn_block_header_hash = height_to_burn_block_header_hash(burn_block_height);

        let genesis_burn_block = BurnBlockInfo {
            burn_block_time: genesis_time,
            burn_block_height,
        };

        let genesis_block = StacksBlockInfo {
            block_header_hash: BlockHeaderHash(bytes),
            burn_block_header_hash: first_burn_block_header_hash,
            consensus_hash: ConsensusHash::from_bytes(&bytes[0..20]).unwrap(),
            vrf_seed: VRFSeed(bytes),
            stacks_block_time: genesis_time + SECONDS_BETWEEN_STACKS_BLOCKS,
        };

        let sortition_lookup = HashMap::from([(sortition_id, id)]);
        let consensus_hash_lookup = HashMap::from([(genesis_block.consensus_hash, sortition_id)]);
        let tenure_blocks_height = HashMap::from([(1, 1)]);
        let burn_blocks = HashMap::from([(first_burn_block_header_hash, genesis_burn_block)]);
        let stacks_blocks = HashMap::from([(id, genesis_block)]);

        Datastore {
            genesis_id: id,
            burn_chain_height: burn_block_height,
            burn_blocks,
            stacks_chain_height: block_height,
            stacks_blocks,
            sortition_lookup,
            consensus_hash_lookup,
            tenure_blocks_height,
            current_epoch: StacksEpochId::Epoch2_05,
            current_epoch_start_height: 1,
            constants,
        }
    }

    pub fn get_current_epoch(&self) -> StacksEpochId {
        self.current_epoch
    }

    pub fn get_current_stacks_block_height(&self) -> u32 {
        self.stacks_chain_height
    }

    pub fn get_current_burn_block_height(&self) -> u32 {
        self.burn_chain_height
    }

    fn build_next_stacks_block(&self, clarity_datastore: &ClarityDatastore) -> StacksBlockInfo {
        let burn_chain_height = self.burn_chain_height;
        let stacks_block_height = self.stacks_chain_height;

        let last_stacks_block = self
            .stacks_blocks
            .get(&clarity_datastore.current_chain_tip)
            .expect("current chain tip missing in stacks block table");
        let last_burn_block = self
            .burn_blocks
            .get(&height_to_burn_block_header_hash(burn_chain_height))
            .expect("burn block missing in burn block table");

        let last_block_time = std::cmp::max(
            last_stacks_block.stacks_block_time,
            last_burn_block.burn_block_time,
        );

        let bytes = height_to_hashed_bytes(stacks_block_height);

        let block_header_hash = {
            let mut buffer = bytes;
            buffer[0] = 1;
            BlockHeaderHash(buffer)
        };
        let burn_block_header_hash = height_to_burn_block_header_hash(burn_chain_height);
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
        let stacks_block_time: u64 = last_block_time + SECONDS_BETWEEN_STACKS_BLOCKS;

        StacksBlockInfo {
            block_header_hash,
            burn_block_header_hash,
            consensus_hash,
            vrf_seed,
            stacks_block_time,
        }
    }

    pub fn advance_burn_chain_tip(
        &mut self,
        clarity_datastore: &mut ClarityDatastore,
        count: u32,
    ) -> u32 {
        for _ in 1..=count {
            let last_stacks_block = self
                .stacks_blocks
                .get(&clarity_datastore.current_chain_tip)
                .unwrap();
            let last_burn_block = self
                .burn_blocks
                .get(&last_stacks_block.burn_block_header_hash)
                .unwrap();

            let mut next_burn_block_time =
                last_burn_block.burn_block_time + SECONDS_BETWEEN_BURN_BLOCKS;
            if last_stacks_block.stacks_block_time > next_burn_block_time {
                next_burn_block_time =
                    last_stacks_block.stacks_block_time + SECONDS_BETWEEN_STACKS_BLOCKS;
            }

            let height = self.burn_chain_height + 1;
            let hash = height_to_burn_block_header_hash(height);
            let burn_block_info = BurnBlockInfo {
                burn_block_time: next_burn_block_time,
                burn_block_height: height,
            };

            self.burn_blocks.insert(hash, burn_block_info);
            self.burn_chain_height = height;
            self.advance_stacks_chain_tip(clarity_datastore, 1);

            self.tenure_blocks_height
                .insert(self.burn_chain_height, self.stacks_chain_height);
        }

        self.burn_chain_height
    }

    pub fn advance_stacks_chain_tip(
        &mut self,
        clarity_datastore: &mut ClarityDatastore,
        count: u32,
    ) -> u32 {
        let current_lookup_id = *clarity_datastore
            .block_id_lookup
            .get(&clarity_datastore.open_chain_tip)
            .expect("Open chain tip missing in block id lookup table");

        for _ in 1..=count {
            self.stacks_chain_height += 1;

            let bytes = height_to_hashed_bytes(self.stacks_chain_height);
            let id = StacksBlockId(bytes);
            let sortition_id = SortitionId(bytes);
            let block_info = self.build_next_stacks_block(clarity_datastore);

            self.sortition_lookup.insert(sortition_id, id);
            self.consensus_hash_lookup
                .insert(block_info.consensus_hash, sortition_id);
            self.stacks_blocks.insert(id, block_info);

            clarity_datastore
                .block_id_lookup
                .entry(id)
                .or_insert(current_lookup_id);
            clarity_datastore
                .height_at_chain_tip
                .entry(id)
                .or_insert(self.stacks_chain_height);
            clarity_datastore.open_chain_tip = height_to_id(self.stacks_chain_height);
            clarity_datastore.current_chain_tip = clarity_datastore.open_chain_tip;
        }

        self.stacks_chain_height
    }

    pub fn set_current_epoch(
        &mut self,
        clarity_datastore: &mut ClarityDatastore,
        epoch: StacksEpochId,
    ) {
        if epoch == self.current_epoch {
            return;
        }
        self.current_epoch = epoch;
        self.current_epoch_start_height = self.stacks_chain_height;
        if epoch >= StacksEpochId::Epoch30 {
            // ideally the burn chain tip should be advanced for each new epoch
            // but this would introduce breaking changes to existing 2.x tests
            self.advance_burn_chain_tip(clarity_datastore, 1);
        }
    }
}

impl HeadersDB for Datastore {
    fn get_stacks_block_header_hash_for_block(
        &self,
        id_bhh: &StacksBlockId,
        _epoch_id: &StacksEpochId,
    ) -> Option<BlockHeaderHash> {
        self.stacks_blocks
            .get(id_bhh)
            .map(|id| id.block_header_hash)
    }

    fn get_burn_header_hash_for_block(
        &self,
        id_bhh: &StacksBlockId,
    ) -> Option<BurnchainHeaderHash> {
        self.stacks_blocks
            .get(id_bhh)
            .map(|block| block.burn_block_header_hash)
    }

    fn get_consensus_hash_for_block(
        &self,
        id_bhh: &StacksBlockId,
        _epoch_id: &StacksEpochId,
    ) -> Option<ConsensusHash> {
        self.stacks_blocks.get(id_bhh).map(|id| id.consensus_hash)
    }

    fn get_vrf_seed_for_block(
        &self,
        id_bhh: &StacksBlockId,
        _epoch_id: &StacksEpochId,
    ) -> Option<VRFSeed> {
        self.stacks_blocks.get(id_bhh).map(|id| id.vrf_seed)
    }

    fn get_stacks_block_time_for_block(&self, id_bhh: &StacksBlockId) -> Option<u64> {
        self.stacks_blocks
            .get(id_bhh)
            .map(|id| id.stacks_block_time)
    }

    fn get_burn_block_time_for_block(
        &self,
        id_bhh: &StacksBlockId,
        _epoch_id: Option<&StacksEpochId>,
    ) -> Option<u64> {
        self.get_burn_header_hash_for_block(id_bhh)
            .and_then(|hash| self.burn_blocks.get(&hash))
            .map(|b| b.burn_block_time)
    }

    fn get_burn_block_height_for_block(&self, id_bhh: &StacksBlockId) -> Option<u32> {
        self.get_burn_header_hash_for_block(id_bhh)
            .and_then(|hash| self.burn_blocks.get(&hash))
            .map(|b| b.burn_block_height)
    }

    fn get_stacks_height_for_tenure_height(
        &self,
        _id_bhh: &StacksBlockId,
        tenure_height: u32,
    ) -> Option<u32> {
        self.tenure_blocks_height.get(&tenure_height).copied()
    }

    fn get_miner_address(
        &self,
        id_bhh: &StacksBlockId,
        _epoch_id: &StacksEpochId,
    ) -> Option<StacksAddress> {
        if self.get_burn_block_height_for_block(id_bhh).is_some() {
            return StacksAddress::burn_address(id_bhh != &self.genesis_id).into();
        }
        None
    }

    fn get_burnchain_tokens_spent_for_block(
        &self,
        id_bhh: &StacksBlockId,
        _epoch_id: &StacksEpochId,
    ) -> Option<u128> {
        if id_bhh == &self.genesis_id {
            return Some(0);
        };
        if self.get_burn_block_height_for_block(id_bhh).is_some() {
            return Some(2000);
        };
        None
    }

    fn get_burnchain_tokens_spent_for_winning_block(
        &self,
        id_bhh: &StacksBlockId,
        _epoch_id: &StacksEpochId,
    ) -> Option<u128> {
        if id_bhh == &self.genesis_id {
            return Some(0);
        };
        None
    }

    fn get_tokens_earned_for_block(
        &self,
        id_bhh: &StacksBlockId,
        _epoch_id: &StacksEpochId,
    ) -> Option<u128> {
        if id_bhh == &self.genesis_id {
            return Some(0);
        };
        if self.get_burn_block_height_for_block(id_bhh).is_some() {
            return Some(5000);
        }
        None
    }
}

impl BurnStateDB for Datastore {
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
        Some(self.burn_chain_height)
    }

    fn get_tip_sortition_id(&self) -> Option<SortitionId> {
        let bytes = height_to_hashed_bytes(self.stacks_chain_height);
        let sortition_id = SortitionId(bytes);
        Some(sortition_id)
    }

    /// Returns the *burnchain block height* for the `sortition_id` is associated with.
    fn get_burn_block_height(&self, sortition_id: &SortitionId) -> Option<u32> {
        self.sortition_lookup
            .get(sortition_id)
            .and_then(|id| self.stacks_blocks.get(id))
            .map(|stacks_block_info| stacks_block_info.burn_block_header_hash)
            .and_then(|hash| self.burn_blocks.get(&hash))
            .map(|burn_block_info| burn_block_info.burn_block_height)
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
            .and_then(|id| self.stacks_blocks.get(id))
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
        height: u32,
        _sortition_id: &SortitionId,
    ) -> Option<(Vec<TupleData>, u128)> {
        if height <= self.burn_chain_height {
            Some((vec![], 0))
        } else {
            None
        }
    }

    fn get_ast_rules(&self, _height: u32) -> clarity::vm::ast::ASTRules {
        clarity::vm::ast::ASTRules::PrecheckSize
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use clarity::types::StacksEpoch;

    use super::*;

    #[test]
    fn test_advance_chain_tip() {
        let mut datastore = Datastore::default();
        let mut clarity_datastore = ClarityDatastore::new(RemoteDataSettings::default());
        datastore.advance_burn_chain_tip(&mut clarity_datastore, 5);
        assert_eq!(datastore.stacks_chain_height, 5);
    }

    #[test]
    fn test_set_current_epoch() {
        let mut datastore = Datastore::default();
        let mut clarity_datastore = ClarityDatastore::new(RemoteDataSettings::default());
        let epoch_id = StacksEpochId::Epoch25;
        datastore.set_current_epoch(&mut clarity_datastore, epoch_id);
        assert_eq!(datastore.current_epoch, epoch_id);
    }

    #[test]
    fn test_get_v1_unlock_height() {
        let datastore = Datastore::default();
        assert_eq!(datastore.get_v1_unlock_height(), 0);
    }

    #[test]
    fn test_get_v2_unlock_height() {
        let datastore = Datastore::default();
        assert_eq!(datastore.get_v2_unlock_height(), 0);
    }

    #[test]
    fn test_get_v3_unlock_height() {
        let datastore = Datastore::default();
        assert_eq!(datastore.get_v3_unlock_height(), 0);
    }

    #[test]
    fn test_get_pox_3_activation_height() {
        let datastore = Datastore::default();
        assert_eq!(datastore.get_pox_3_activation_height(), 0);
    }

    #[test]
    fn test_get_pox_4_activation_height() {
        let datastore = Datastore::default();
        assert_eq!(datastore.get_pox_4_activation_height(), 0);
    }

    #[test]
    fn test_get_tip_burn_block_height() {
        let mut datastore = Datastore::default();
        let mut clarity_datastore = ClarityDatastore::new(RemoteDataSettings::default());
        let chain_height = 10;
        datastore.advance_burn_chain_tip(&mut clarity_datastore, 10);
        let tip_burn_block_height = datastore.get_tip_burn_block_height();
        assert_eq!(tip_burn_block_height, Some(chain_height));
    }

    #[test]
    fn test_get_burn_start_height() {
        let datastore = Datastore::default();
        assert_eq!(datastore.get_burn_start_height(), 0);
    }

    #[test]
    fn test_get_pox_prepare_length() {
        let datastore = Datastore::default();
        assert_eq!(datastore.get_pox_prepare_length(), 50);
    }

    #[test]
    fn test_get_pox_reward_cycle_length() {
        let datastore = Datastore::default();
        assert_eq!(datastore.get_pox_reward_cycle_length(), 1050);
    }

    #[test]
    fn test_get_pox_rejection_fraction() {
        let datastore = Datastore::default();
        assert_eq!(datastore.get_pox_rejection_fraction(), 0);
    }

    #[test]
    fn test_get_stacks_epoch() {
        let datastore = Datastore::default();
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
        let datastore = Datastore::default();
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
