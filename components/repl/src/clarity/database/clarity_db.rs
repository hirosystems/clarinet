use rand_pcg::Pcg64;
use rand_seeder::rand_core::RngCore;
use rand_seeder::Seeder;
use std::collections::{HashMap, VecDeque};
use std::convert::TryFrom;

use crate::clarity::analysis::{AnalysisDatabase, ContractAnalysis};
use crate::clarity::contracts::Contract;
use crate::clarity::errors::{
    CheckErrors, Error, IncomparableError, InterpreterError, InterpreterResult as Result,
    RuntimeErrorType,
};
use crate::clarity::types::{
    OptionalData, PrincipalData, QualifiedContractIdentifier, StandardPrincipalData,
    TupleTypeSignature, TypeSignature, Value, NONE,
};

use crate::clarity::costs::CostOverflowingMath;
use crate::clarity::database::structures::{
    ClarityDeserializable, ClaritySerializable, ContractMetadata, DataMapMetadata,
    DataVariableMetadata, FungibleTokenMetadata, NonFungibleTokenMetadata, STXBalance,
    STXBalanceSnapshot, SimmedBlock,
};
use crate::clarity::database::ClarityBackingStore;
use crate::clarity::database::RollbackWrapper;
use crate::clarity::util::hash::{Sha256Sum, Sha512Trunc256Sum};
use crate::clarity::util::StacksAddress;
use crate::clarity::{BlockHeaderHash, BurnchainHeaderHash, StacksBlockId, VRFSeed};

const SIMMED_BLOCK_TIME: u64 = 10 * 60; // 10 min

pub const STORE_CONTRACT_SRC_INTERFACE: bool = true;

#[repr(u8)]
pub enum StoreType {
    DataMap = 0x00,
    Variable = 0x01,
    FungibleToken = 0x02,
    CirculatingSupply = 0x03,
    NonFungibleToken = 0x04,
    DataMapMeta = 0x05,
    VariableMeta = 0x06,
    FungibleTokenMeta = 0x07,
    NonFungibleTokenMeta = 0x08,
    Contract = 0x09,
    SimmedBlock = 0x10,
    SimmedBlockHeight = 0x11,
    Nonce = 0x12,
    STXBalance = 0x13,
    PoxSTXLockup = 0x14,
    PoxUnlockHeight = 0x15,
}

pub struct ClarityDatabase<'a> {
    pub store: RollbackWrapper<'a>,
    headers_db: &'a dyn HeadersDB,
}

pub trait HeadersDB {
    fn get_stacks_block_header_hash_for_block(
        &self,
        id_bhh: &StacksBlockId,
    ) -> Option<BlockHeaderHash>;
    fn get_burn_header_hash_for_block(&self, id_bhh: &StacksBlockId)
        -> Option<BurnchainHeaderHash>;
    fn get_vrf_seed_for_block(&self, id_bhh: &StacksBlockId) -> Option<VRFSeed>;
    fn get_burn_block_time_for_block(&self, id_bhh: &StacksBlockId) -> Option<u64>;
    fn get_burn_block_height_for_block(&self, id_bhh: &StacksBlockId) -> Option<u32>;
    fn get_miner_address(&self, id_bhh: &StacksBlockId) -> Option<StacksAddress>;
    fn get_total_liquid_ustx(&self, id_bhh: &StacksBlockId) -> u128;
}

pub struct NullHeadersDB {}

pub const NULL_HEADER_DB: NullHeadersDB = NullHeadersDB {};

impl HeadersDB for NullHeadersDB {
    fn get_burn_header_hash_for_block(&self, bhh: &StacksBlockId) -> Option<BurnchainHeaderHash> {
        let burn_header_hash = BurnchainHeaderHash(bhh.0.clone());
        Some(burn_header_hash)
    }

    fn get_vrf_seed_for_block(&self, bhh: &StacksBlockId) -> Option<VRFSeed> {
        let mut rng: Pcg64 = Seeder::from(bhh).make_rng();
        let mut buf = [0u8; 32];
        rng.fill_bytes(&mut buf);
        Some(VRFSeed(buf))
    }

    fn get_stacks_block_header_hash_for_block(
        &self,
        id_bhh: &StacksBlockId,
    ) -> Option<BlockHeaderHash> {
        let header_hash = BlockHeaderHash(id_bhh.0.clone());
        Some(header_hash)
    }

    fn get_burn_block_time_for_block(&self, id_bhh: &StacksBlockId) -> Option<u64> {
        Some(0)
    }

    fn get_total_liquid_ustx(&self, _id_bhh: &StacksBlockId) -> u128 {
        0
    }
    fn get_burn_block_height_for_block(&self, _id_bhh: &StacksBlockId) -> Option<u32> {
        None
    }
    fn get_miner_address(&self, _id_bhh: &StacksBlockId) -> Option<StacksAddress> {
        None
    }
}

impl<'a> ClarityDatabase<'a> {
    pub fn new(
        store: &'a mut dyn ClarityBackingStore,
        headers_db: &'a dyn HeadersDB,
    ) -> ClarityDatabase<'a> {
        ClarityDatabase {
            store: RollbackWrapper::new(store),
            headers_db,
        }
    }

    pub fn new_with_rollback_wrapper(
        store: RollbackWrapper<'a>,
        headers_db: &'a dyn HeadersDB,
    ) -> ClarityDatabase<'a> {
        ClarityDatabase { store, headers_db }
    }

    pub fn initialize(&mut self) {}

    pub fn begin(&mut self) {
        self.store.nest();
    }

    pub fn commit(&mut self) {
        self.store.commit();
    }

    pub fn roll_back(&mut self) {
        self.store.rollback();
    }

    pub fn set_block_hash(
        &mut self,
        bhh: StacksBlockId,
        query_pending_data: bool,
    ) -> Result<StacksBlockId> {
        self.store.set_block_hash(bhh, query_pending_data)
    }

    pub fn put<T: ClaritySerializable>(&mut self, key: &str, value: &T) {
        self.store.put(&key, &value.serialize());
    }

    pub fn get<T>(&mut self, key: &str) -> Option<T>
    where
        T: ClarityDeserializable<T>,
    {
        self.store.get::<T>(key)
    }

    pub fn get_value(&mut self, key: &str, expected: &TypeSignature) -> Option<Value> {
        self.store.get_value(key, expected)
    }

    pub fn make_key_for_trip(
        contract_identifier: &QualifiedContractIdentifier,
        data: StoreType,
        var_name: &str,
    ) -> String {
        format!("vm::{}::{}::{}", contract_identifier, data as u8, var_name)
    }

    pub fn make_metadata_key(data: StoreType, var_name: &str) -> String {
        format!("vm-metadata::{}::{}", data as u8, var_name)
    }

    pub fn make_key_for_quad(
        contract_identifier: &QualifiedContractIdentifier,
        data: StoreType,
        var_name: &str,
        key_value: String,
    ) -> String {
        format!(
            "vm::{}::{}::{}::{}",
            contract_identifier, data as u8, var_name, key_value
        )
    }

    pub fn insert_contract_hash(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        contract_content: &str,
    ) -> Result<()> {
        let hash = Sha512Trunc256Sum::from_data(contract_content.as_bytes());
        self.store
            .prepare_for_contract_metadata(contract_identifier, hash);
        // insert contract-size
        let key = ClarityDatabase::make_metadata_key(StoreType::Contract, "contract-size");
        self.insert_metadata(contract_identifier, &key, &(contract_content.len() as u64));

        // insert contract-src
        if STORE_CONTRACT_SRC_INTERFACE {
            let key = ClarityDatabase::make_metadata_key(StoreType::Contract, "contract-src");
            self.insert_metadata(contract_identifier, &key, &contract_content.to_string());
        }
        Ok(())
    }

    pub fn get_contract_src(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
    ) -> Option<String> {
        let key = ClarityDatabase::make_metadata_key(StoreType::Contract, "contract-src");
        self.fetch_metadata(contract_identifier, &key)
            .ok()
            .flatten()
    }

    pub fn set_metadata(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        key: &str,
        data: &str,
    ) {
        self.store.insert_metadata(contract_identifier, key, data);
    }

    fn insert_metadata<T: ClaritySerializable>(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        key: &str,
        data: &T,
    ) {
        if self.store.has_metadata_entry(contract_identifier, key) {
            panic!(
                "Metadata entry '{}' already exists for contract: {}",
                key, contract_identifier
            );
        } else {
            self.store
                .insert_metadata(contract_identifier, key, &data.serialize());
        }
    }

    fn fetch_metadata<T>(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        key: &str,
    ) -> Result<Option<T>>
    where
        T: ClarityDeserializable<T>,
    {
        self.store
            .get_metadata(contract_identifier, key)
            .map(|x_opt| x_opt.map(|x| T::deserialize(&x)))
    }

    // load contract analysis stored by an analysis_db instance.
    //   in unit testing, where the interpreter is invoked without
    //   an analysis pass, this function will fail to find contract
    //   analysis data
    pub fn load_contract_analysis(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
    ) -> Option<ContractAnalysis> {
        self.store
            .get_metadata(contract_identifier, AnalysisDatabase::storage_key())
            // treat NoSuchContract error thrown by get_metadata as an Option::None --
            //    the analysis will propagate that as a CheckError anyways.
            .ok()?
            .map(|x| ContractAnalysis::deserialize(&x))
    }

    pub fn get_contract_size(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
    ) -> Result<u64> {
        let key = ClarityDatabase::make_metadata_key(StoreType::Contract, "contract-size");
        let contract_size: u64 = self.fetch_metadata(contract_identifier, &key)?.expect(
            "Failed to read non-consensus contract metadata, even though contract exists in MARF.",
        );
        let key = ClarityDatabase::make_metadata_key(StoreType::Contract, "contract-data-size");
        let data_size: u64 = self.fetch_metadata(contract_identifier, &key)?.expect(
            "Failed to read non-consensus contract metadata, even though contract exists in MARF.",
        );

        // u64 overflow is _checked_ on insert into contract-data-size
        Ok(data_size + contract_size)
    }

    /// used for adding the memory usage of `define-constant` variables.
    pub fn set_contract_data_size(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        data_size: u64,
    ) -> Result<()> {
        let key = ClarityDatabase::make_metadata_key(StoreType::Contract, "contract-size");
        let contract_size: u64 = self.fetch_metadata(contract_identifier, &key)?.expect(
            "Failed to read non-consensus contract metadata, even though contract exists in MARF.",
        );
        contract_size.cost_overflow_add(data_size)?;

        let key = ClarityDatabase::make_metadata_key(StoreType::Contract, "contract-data-size");
        self.insert_metadata(contract_identifier, &key, &data_size);
        Ok(())
    }

    pub fn insert_contract(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        contract: Contract,
    ) {
        let key = ClarityDatabase::make_metadata_key(StoreType::Contract, "contract");
        self.insert_metadata(contract_identifier, &key, &contract);
    }

    pub fn has_contract(&mut self, contract_identifier: &QualifiedContractIdentifier) -> bool {
        let key = ClarityDatabase::make_metadata_key(StoreType::Contract, "contract");
        self.store.has_metadata_entry(contract_identifier, &key)
    }

    pub fn get_contract(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
    ) -> Result<Contract> {
        let key = ClarityDatabase::make_metadata_key(StoreType::Contract, "contract");
        let data = self.fetch_metadata(contract_identifier, &key)?.expect(
            "Failed to read non-consensus contract metadata, even though contract exists in MARF.",
        );
        Ok(data)
    }

    pub fn ustx_liquid_supply_key() -> &'static str {
        "_stx-data::ustx_liquid_supply"
    }

    /// Returns the _current_ total liquid ustx
    pub fn get_total_liquid_ustx(&mut self) -> u128 {
        self.get_value(
            ClarityDatabase::ustx_liquid_supply_key(),
            &TypeSignature::UIntType,
        )
        .map(|v| v.expect_u128())
        .unwrap_or(0)
    }

    fn set_ustx_liquid_supply(&mut self, set_to: u128) {
        self.put(
            ClarityDatabase::ustx_liquid_supply_key(),
            &Value::UInt(set_to),
        )
    }

    pub fn increment_ustx_liquid_supply(&mut self, incr_by: u128) -> Result<()> {
        let current = self.get_total_liquid_ustx();
        let next = current
            .checked_add(incr_by)
            .ok_or_else(|| RuntimeErrorType::ArithmeticOverflow)?;
        self.set_ustx_liquid_supply(next);
        Ok(())
    }

    pub fn decrement_ustx_liquid_supply(&mut self, decr_by: u128) -> Result<()> {
        let current = self.get_total_liquid_ustx();
        let next = current
            .checked_sub(decr_by)
            .ok_or_else(|| RuntimeErrorType::ArithmeticUnderflow)?;
        self.set_ustx_liquid_supply(next);
        Ok(())
    }

    pub fn destroy(self) -> RollbackWrapper<'a> {
        self.store
    }
}

// Get block information

impl<'a> ClarityDatabase<'a> {
    pub fn get_index_block_header_hash(&mut self, block_height: u32) -> StacksBlockId {
        self.store
            .get_block_header_hash(block_height)
            // the caller is responsible for ensuring that the block_height given
            //  is < current_block_height, so this should _always_ return a value.
            .expect("Block header hash must return for provided block height")
    }

    pub fn get_current_block_height(&mut self) -> u32 {
        self.store.get_current_block_height()
    }

    pub fn get_current_burnchain_block_height(&mut self) -> u32 {
        let cur_stacks_height = self.store.get_current_block_height();
        let cur_id_bhh = self.get_index_block_header_hash(cur_stacks_height);
        self.get_burnchain_block_height(&cur_id_bhh)
            .unwrap_or(cur_stacks_height)
    }

    pub fn get_block_header_hash(&mut self, block_height: u32) -> BlockHeaderHash {
        let id_bhh = self.get_index_block_header_hash(block_height);
        self.headers_db
            .get_stacks_block_header_hash_for_block(&id_bhh)
            .expect("Failed to get block data.")
    }

    pub fn get_block_time(&mut self, block_height: u32) -> u64 {
        block_height as u64 * 600
    }

    pub fn get_burnchain_block_header_hash(&mut self, block_height: u32) -> BurnchainHeaderHash {
        let id_bhh = self.get_index_block_header_hash(block_height);
        self.headers_db
            .get_burn_header_hash_for_block(&id_bhh)
            .expect("Failed to get block data.")
    }

    pub fn get_burnchain_block_height(&mut self, id_bhh: &StacksBlockId) -> Option<u32> {
        self.headers_db.get_burn_block_height_for_block(id_bhh)
    }

    pub fn get_block_vrf_seed(&mut self, block_height: u32) -> VRFSeed {
        let id_bhh = self.get_index_block_header_hash(block_height);
        self.headers_db
            .get_vrf_seed_for_block(&id_bhh)
            .expect("Failed to get block data.")
    }

    pub fn get_miner_address(&mut self, block_height: u32) -> StandardPrincipalData {
        StandardPrincipalData::transient()
    }

    pub fn get_stx_btc_ops_processed(&mut self) -> u64 {
        self.get("vm_pox::stx_btc_ops::processed_blocks")
            .unwrap_or(0)
    }

    pub fn set_stx_btc_ops_processed(&mut self, processed: u64) {
        // let id_bhh = self.get_index_block_header_hash(block_height);
        // self.headers_db.get_miner_address(&id_bhh)
        //     .expect("Failed to get block data.")
        //     .into()
    }
}

// this is used so that things like load_map, load_var, load_nft, etc.
//   will throw NoSuchFoo errors instead of NoSuchContract errors.
fn map_no_contract_as_none<T>(res: Result<Option<T>>) -> Result<Option<T>> {
    res.or_else(|e| match e {
        Error::Unchecked(CheckErrors::NoSuchContract(_)) => Ok(None),
        x => Err(x),
    })
}

// Variable Functions...
impl<'a> ClarityDatabase<'a> {
    pub fn create_variable(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        variable_name: &str,
        value_type: TypeSignature,
    ) -> DataVariableMetadata {
        let variable_data = DataVariableMetadata { value_type };
        let key = ClarityDatabase::make_metadata_key(StoreType::VariableMeta, variable_name);

        self.insert_metadata(contract_identifier, &key, &variable_data);
        variable_data
    }

    pub fn load_variable(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        variable_name: &str,
    ) -> Result<DataVariableMetadata> {
        let key = ClarityDatabase::make_metadata_key(StoreType::VariableMeta, variable_name);

        map_no_contract_as_none(self.fetch_metadata(contract_identifier, &key))?
            .ok_or(CheckErrors::NoSuchDataVariable(variable_name.to_string()).into())
    }

    pub fn set_variable_unknown_descriptor(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        variable_name: &str,
        value: Value,
    ) -> Result<Value> {
        let descriptor = self.load_variable(contract_identifier, variable_name)?;
        self.set_variable(contract_identifier, variable_name, value, &descriptor)
    }

    pub fn set_variable(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        variable_name: &str,
        value: Value,
        variable_descriptor: &DataVariableMetadata,
    ) -> Result<Value> {
        if !variable_descriptor.value_type.admits(&value) {
            return Err(
                CheckErrors::TypeValueError(variable_descriptor.value_type.clone(), value).into(),
            );
        }

        let key = ClarityDatabase::make_key_for_trip(
            contract_identifier,
            StoreType::Variable,
            variable_name,
        );

        self.put(&key, &value);

        return Ok(Value::Bool(true));
    }

    pub fn lookup_variable_unknown_descriptor(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        variable_name: &str,
    ) -> Result<Value> {
        let descriptor = self.load_variable(contract_identifier, variable_name)?;
        self.lookup_variable(contract_identifier, variable_name, &descriptor)
    }

    pub fn lookup_variable(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        variable_name: &str,
        variable_descriptor: &DataVariableMetadata,
    ) -> Result<Value> {
        let key = ClarityDatabase::make_key_for_trip(
            contract_identifier,
            StoreType::Variable,
            variable_name,
        );

        let result = self.get_value(&key, &variable_descriptor.value_type);

        match result {
            None => Ok(Value::none()),
            Some(data) => Ok(data),
        }
    }
}

// Data Map Functions
impl<'a> ClarityDatabase<'a> {
    pub fn create_map(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        map_name: &str,
        key_type: TypeSignature,
        value_type: TypeSignature,
    ) -> DataMapMetadata {
        let data = DataMapMetadata {
            key_type,
            value_type,
        };

        let key = ClarityDatabase::make_metadata_key(StoreType::DataMapMeta, map_name);
        self.insert_metadata(contract_identifier, &key, &data);

        data
    }

    pub fn load_map(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        map_name: &str,
    ) -> Result<DataMapMetadata> {
        let key = ClarityDatabase::make_metadata_key(StoreType::DataMapMeta, map_name);

        map_no_contract_as_none(self.fetch_metadata(contract_identifier, &key))?
            .ok_or(CheckErrors::NoSuchMap(map_name.to_string()).into())
    }

    pub fn make_key_for_data_map_entry(
        contract_identifier: &QualifiedContractIdentifier,
        map_name: &str,
        key_value: &Value,
    ) -> String {
        ClarityDatabase::make_key_for_quad(
            contract_identifier,
            StoreType::DataMap,
            map_name,
            key_value.serialize(),
        )
    }

    pub fn fetch_entry_unknown_descriptor(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        map_name: &str,
        key_value: &Value,
    ) -> Result<Value> {
        let descriptor = self.load_map(contract_identifier, map_name)?;
        self.fetch_entry(contract_identifier, map_name, key_value, &descriptor)
    }

    pub fn fetch_entry(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        map_name: &str,
        key_value: &Value,
        map_descriptor: &DataMapMetadata,
    ) -> Result<Value> {
        if !map_descriptor.key_type.admits(key_value) {
            return Err(CheckErrors::TypeValueError(
                map_descriptor.key_type.clone(),
                (*key_value).clone(),
            )
            .into());
        }

        let key =
            ClarityDatabase::make_key_for_data_map_entry(contract_identifier, map_name, key_value);

        let stored_type = TypeSignature::new_option(map_descriptor.value_type.clone())?;
        let result = self.get_value(&key, &stored_type);

        match result {
            None => Ok(Value::none()),
            Some(data) => Ok(data),
        }
    }

    pub fn set_entry(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        map_name: &str,
        key: Value,
        value: Value,
        map_descriptor: &DataMapMetadata,
    ) -> Result<Value> {
        self.inner_set_entry(
            contract_identifier,
            map_name,
            key,
            value,
            false,
            map_descriptor,
        )
    }

    pub fn set_entry_unknown_descriptor(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        map_name: &str,
        key: Value,
        value: Value,
    ) -> Result<Value> {
        let descriptor = self.load_map(contract_identifier, map_name)?;
        self.set_entry(contract_identifier, map_name, key, value, &descriptor)
    }

    pub fn insert_entry_unknown_descriptor(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        map_name: &str,
        key: Value,
        value: Value,
    ) -> Result<Value> {
        let descriptor = self.load_map(contract_identifier, map_name)?;
        self.insert_entry(contract_identifier, map_name, key, value, &descriptor)
    }

    pub fn insert_entry(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        map_name: &str,
        key: Value,
        value: Value,
        map_descriptor: &DataMapMetadata,
    ) -> Result<Value> {
        self.inner_set_entry(
            contract_identifier,
            map_name,
            key,
            value,
            true,
            map_descriptor,
        )
    }

    fn data_map_entry_exists(&mut self, key: &str, expected_value: &TypeSignature) -> Result<bool> {
        match self.get_value(key, expected_value) {
            None => Ok(false),
            Some(value) => Ok(value != Value::none()),
        }
    }

    fn inner_set_entry(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        map_name: &str,
        key_value: Value,
        value: Value,
        return_if_exists: bool,
        map_descriptor: &DataMapMetadata,
    ) -> Result<Value> {
        if !map_descriptor.key_type.admits(&key_value) {
            return Err(
                CheckErrors::TypeValueError(map_descriptor.key_type.clone(), key_value).into(),
            );
        }
        if !map_descriptor.value_type.admits(&value) {
            return Err(
                CheckErrors::TypeValueError(map_descriptor.value_type.clone(), value).into(),
            );
        }

        let key = ClarityDatabase::make_key_for_quad(
            contract_identifier,
            StoreType::DataMap,
            map_name,
            key_value.serialize(),
        );
        let stored_type = TypeSignature::new_option(map_descriptor.value_type.clone())?;

        if return_if_exists && self.data_map_entry_exists(&key, &stored_type)? {
            return Ok(Value::Bool(false));
        }

        let placed_value = Value::some(value)?;
        self.put(&key, &placed_value);

        return Ok(Value::Bool(true));
    }

    pub fn delete_entry(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        map_name: &str,
        key_value: &Value,
        map_descriptor: &DataMapMetadata,
    ) -> Result<Value> {
        if !map_descriptor.key_type.admits(key_value) {
            return Err(CheckErrors::TypeValueError(
                map_descriptor.key_type.clone(),
                (*key_value).clone(),
            )
            .into());
        }

        let key = ClarityDatabase::make_key_for_quad(
            contract_identifier,
            StoreType::DataMap,
            map_name,
            key_value.serialize(),
        );
        let stored_type = TypeSignature::new_option(map_descriptor.value_type.clone())?;
        if !self.data_map_entry_exists(&key, &stored_type)? {
            return Ok(Value::Bool(false));
        }

        self.put(&key, &(Value::none()));

        return Ok(Value::Bool(true));
    }
}

// Asset Functions

impl<'a> ClarityDatabase<'a> {
    pub fn create_fungible_token(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        token_name: &str,
        total_supply: &Option<u128>,
    ) -> FungibleTokenMetadata {
        let data = FungibleTokenMetadata {
            total_supply: total_supply.clone(),
        };

        let key = ClarityDatabase::make_metadata_key(StoreType::FungibleTokenMeta, token_name);
        self.insert_metadata(contract_identifier, &key, &data);

        // total supply _is_ included in the consensus hash
        let supply_key = ClarityDatabase::make_key_for_trip(
            contract_identifier,
            StoreType::CirculatingSupply,
            token_name,
        );
        self.put(&supply_key, &(0 as u128));

        data
    }

    pub fn load_ft(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        token_name: &str,
    ) -> Result<FungibleTokenMetadata> {
        let key = ClarityDatabase::make_metadata_key(StoreType::FungibleTokenMeta, token_name);

        map_no_contract_as_none(self.fetch_metadata(contract_identifier, &key))?
            .ok_or(CheckErrors::NoSuchFT(token_name.to_string()).into())
    }

    pub fn create_non_fungible_token(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        token_name: &str,
        key_type: &TypeSignature,
    ) -> NonFungibleTokenMetadata {
        let data = NonFungibleTokenMetadata {
            key_type: key_type.clone(),
        };
        let key = ClarityDatabase::make_metadata_key(StoreType::NonFungibleTokenMeta, token_name);
        self.insert_metadata(contract_identifier, &key, &data);

        data
    }

    fn load_nft(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        token_name: &str,
    ) -> Result<NonFungibleTokenMetadata> {
        let key = ClarityDatabase::make_metadata_key(StoreType::NonFungibleTokenMeta, token_name);

        map_no_contract_as_none(self.fetch_metadata(contract_identifier, &key))?
            .ok_or(CheckErrors::NoSuchNFT(token_name.to_string()).into())
    }

    pub fn checked_increase_token_supply(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        token_name: &str,
        amount: u128,
        descriptor: &FungibleTokenMetadata,
    ) -> Result<()> {
        let key = ClarityDatabase::make_key_for_trip(
            contract_identifier,
            StoreType::CirculatingSupply,
            token_name,
        );
        let current_supply: u128 = self
            .get(&key)
            .expect("ERROR: Clarity VM failed to track token supply.");

        let new_supply = current_supply
            .checked_add(amount)
            .ok_or(RuntimeErrorType::ArithmeticOverflow)?;

        if let Some(total_supply) = descriptor.total_supply {
            if new_supply > total_supply {
                return Err(RuntimeErrorType::SupplyOverflow(new_supply, total_supply).into());
            }
        }

        self.put(&key, &new_supply);
        Ok(())
    }

    pub fn checked_decrease_token_supply(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        token_name: &str,
        amount: u128,
    ) -> Result<()> {
        let key = ClarityDatabase::make_key_for_trip(
            contract_identifier,
            StoreType::CirculatingSupply,
            token_name,
        );
        let current_supply: u128 = self
            .get(&key)
            .expect("ERROR: Clarity VM failed to track token supply.");

        if amount > current_supply {
            return Err(RuntimeErrorType::SupplyUnderflow(current_supply, amount).into());
        }

        let new_supply = current_supply - amount;

        self.put(&key, &new_supply);
        Ok(())
    }

    pub fn get_ft_balance(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        token_name: &str,
        principal: &PrincipalData,
        descriptor: Option<&FungibleTokenMetadata>,
    ) -> Result<u128> {
        if descriptor.is_none() {
            self.load_ft(contract_identifier, token_name)?;
        }

        let key = ClarityDatabase::make_key_for_quad(
            contract_identifier,
            StoreType::FungibleToken,
            token_name,
            principal.serialize(),
        );

        let result = self.get(&key);
        match result {
            None => Ok(0),
            Some(balance) => Ok(balance),
        }
    }

    pub fn set_ft_balance(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        token_name: &str,
        principal: &PrincipalData,
        balance: u128,
    ) -> Result<()> {
        let key = ClarityDatabase::make_key_for_quad(
            contract_identifier,
            StoreType::FungibleToken,
            token_name,
            principal.serialize(),
        );
        self.put(&key, &balance);

        Ok(())
    }

    pub fn get_ft_supply(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        token_name: &str,
    ) -> Result<u128> {
        let key = ClarityDatabase::make_key_for_trip(
            contract_identifier,
            StoreType::CirculatingSupply,
            token_name,
        );
        let supply = self
            .get(&key)
            .expect("ERROR: Clarity VM failed to track token supply.");
        Ok(supply)
    }

    pub fn get_nft_owner(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        asset_name: &str,
        asset: &Value,
        key_type: &TypeSignature,
    ) -> Result<PrincipalData> {
        if !key_type.admits(asset) {
            return Err(CheckErrors::TypeValueError(key_type.clone(), (*asset).clone()).into());
        }

        let key = ClarityDatabase::make_key_for_quad(
            contract_identifier,
            StoreType::NonFungibleToken,
            asset_name,
            asset.serialize(),
        );

        let value: Option<Value> = self.get(&key);
        let owner = match value {
            Some(owner) => owner.expect_optional(),
            None => return Err(RuntimeErrorType::NoSuchToken.into()),
        };

        let principal = match owner {
            Some(value) => value.expect_principal(),
            None => return Err(RuntimeErrorType::NoSuchToken.into()),
        };

        Ok(principal)
    }

    pub fn get_nft_key_type(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        asset_name: &str,
    ) -> Result<TypeSignature> {
        let descriptor = self.load_nft(contract_identifier, asset_name)?;
        Ok(descriptor.key_type)
    }

    pub fn set_nft_owner(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        asset_name: &str,
        asset: &Value,
        principal: &PrincipalData,
        key_type: &TypeSignature,
    ) -> Result<()> {
        if !key_type.admits(asset) {
            return Err(CheckErrors::TypeValueError(key_type.clone(), (*asset).clone()).into());
        }

        let key = ClarityDatabase::make_key_for_quad(
            contract_identifier,
            StoreType::NonFungibleToken,
            asset_name,
            asset.serialize(),
        );

        let value = Value::some(Value::Principal(principal.clone()))?;
        self.put(&key, &value);

        Ok(())
    }

    pub fn burn_nft(
        &mut self,
        contract_identifier: &QualifiedContractIdentifier,
        asset_name: &str,
        asset: &Value,
        key_type: &TypeSignature,
    ) -> Result<()> {
        if !key_type.admits(asset) {
            return Err(CheckErrors::TypeValueError(key_type.clone(), (*asset).clone()).into());
        }

        let key = ClarityDatabase::make_key_for_quad(
            contract_identifier,
            StoreType::NonFungibleToken,
            asset_name,
            asset.serialize(),
        );

        self.put(&key, &(Value::none()));
        Ok(())
    }
}

// load/store STX token state and account nonces
impl<'a> ClarityDatabase<'a> {
    fn make_key_for_account(principal: &PrincipalData, data: StoreType) -> String {
        format!("vm-account::{}::{}", principal, data as u8)
    }

    pub fn make_key_for_account_balance(principal: &PrincipalData) -> String {
        ClarityDatabase::make_key_for_account(principal, StoreType::STXBalance)
    }

    pub fn make_key_for_account_nonce(principal: &PrincipalData) -> String {
        ClarityDatabase::make_key_for_account(principal, StoreType::Nonce)
    }

    pub fn make_key_for_account_stx_locked(principal: &PrincipalData) -> String {
        ClarityDatabase::make_key_for_account(principal, StoreType::PoxSTXLockup)
    }

    pub fn make_key_for_account_unlock_height(principal: &PrincipalData) -> String {
        ClarityDatabase::make_key_for_account(principal, StoreType::PoxUnlockHeight)
    }

    pub fn get_stx_balance_snapshot<'conn>(
        &'conn mut self,
        principal: &PrincipalData,
    ) -> STXBalanceSnapshot<'a, 'conn> {
        let stx_balance = self.get_account_stx_balance(principal);
        let cur_burn_height = self.get_current_burnchain_block_height() as u64;

        STXBalanceSnapshot::new(principal, stx_balance, cur_burn_height, self)
    }

    pub fn get_stx_balance_snapshot_genesis<'conn>(
        &'conn mut self,
        principal: &PrincipalData,
    ) -> STXBalanceSnapshot<'a, 'conn> {
        let stx_balance = self.get_account_stx_balance(principal);
        let cur_burn_height = 0;

        STXBalanceSnapshot::new(principal, stx_balance, cur_burn_height, self)
    }

    pub fn get_account_stx_balance(&mut self, principal: &PrincipalData) -> STXBalance {
        let key = ClarityDatabase::make_key_for_account_balance(principal);
        let result = self.get(&key);
        match result {
            None => STXBalance::zero(),
            Some(balance) => balance,
        }
    }

    pub fn get_account_nonce(&mut self, principal: &PrincipalData) -> u64 {
        let key = ClarityDatabase::make_key_for_account_nonce(principal);
        let result = self.get(&key);
        match result {
            None => 0,
            Some(nonce) => nonce,
        }
    }

    pub fn set_account_nonce(&mut self, principal: &PrincipalData, nonce: u64) {
        let key = ClarityDatabase::make_key_for_account_nonce(principal);
        self.put(&key, &nonce);
    }
}
