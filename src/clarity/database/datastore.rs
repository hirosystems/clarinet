use std::collections::HashMap;
use super::ClarityBackingStore;
use crate::clarity::StacksBlockId;
use crate::clarity::util::hash::Sha512Trunc256Sum;
use crate::clarity::types::QualifiedContractIdentifier;
use crate::clarity::errors::{InterpreterError, CheckErrors, InterpreterResult as Result, IncomparableError, RuntimeErrorType};

pub struct Datastore {
    store: HashMap<String, String>,
    chain_tip: Option<StacksBlockId>,
}

impl Datastore {

    pub fn new() -> Datastore {
        Datastore {
            store: HashMap::new(),
            chain_tip: None
        }
    }
}

impl ClarityBackingStore for Datastore {

    fn put_all(&mut self, items: Vec<(String, String)>) {
    }

    /// fetch K-V out of the committed datastore
    fn get(&mut self, key: &str) -> Option<String> {
        None
    }

    fn has_entry(&mut self, key: &str) -> bool {
        self.get(key).is_some()
    }

    /// change the current MARF context to service reads from a different chain_tip
    ///   used to implement time-shifted evaluation.
    /// returns the previous block header hash on success
    fn set_block_hash(&mut self, bhh: StacksBlockId) -> Result<StacksBlockId> {
        Ok(bhh)
    }

    fn get_block_at_height(&mut self, height: u32) -> Option<StacksBlockId> {
        self.chain_tip
    }

    /// this function returns the current block height, as viewed by this marfed-kv structure,
    ///  i.e., it changes on time-shifted evaluation. the open_chain_tip functions always
    ///   return data about the chain tip that is currently open for writing.
    fn get_current_block_height(&mut self) -> u32 {
        0
    }

    fn get_open_chain_tip_height(&mut self) -> u32 {
        0
    }

    fn get_open_chain_tip(&mut self) -> StacksBlockId {
        self.chain_tip.unwrap()
    }

    fn get_side_store(&mut self) -> &mut SqliteConnection;

    /// The contract commitment is the hash of the contract, plus the block height in
    ///   which the contract was initialized.
    fn make_contract_commitment(&mut self, contract_hash: Sha512Trunc256Sum) -> String {
        "".to_string()
    }

    /// This function is used to obtain a committed contract hash, and the block header hash of the block
    ///   in which the contract was initialized. This data is used to store contract metadata in the side
    ///   store.
    fn get_contract_hash(&mut self, contract: &QualifiedContractIdentifier) -> Result<(StacksBlockId, Sha512Trunc256Sum)> {
        let key = MarfedKV::make_contract_hash_key(contract);
        let contract_commitment = self.get(&key).map(|x| ContractCommitment::deserialize(&x))
            .ok_or_else(|| { CheckErrors::NoSuchContract(contract.to_string()) })?;
        let ContractCommitment { block_height, hash: contract_hash } = contract_commitment;
        let bhh = self.get_block_at_height(block_height)
            .expect("Should always be able to map from height to block hash when looking up contract information.");
        Ok((bhh, contract_hash))
    }

    fn insert_metadata(&mut self, contract: &QualifiedContractIdentifier, key: &str, value: &str) {
        let bhh = self.get_open_chain_tip();
        self.get_side_store().insert_metadata(&bhh, &contract.to_string(), key, value)
    }

    fn get_metadata(&mut self, contract: &QualifiedContractIdentifier, key: &str) -> Result<Option<String>> {
        let (bhh, _) = self.get_contract_hash(contract)?;
        Ok(self.get_side_store().get_metadata(&bhh, &contract.to_string(), key))
    }

    fn put_all_metadata(&mut self, mut items: Vec<((QualifiedContractIdentifier, String), String)>) {
        for ((contract, key), value) in items.drain(..) {
            self.insert_metadata(&contract, &key, &value);
        }
    }}