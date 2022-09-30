use std::convert::TryInto;
use std::path::PathBuf;

use crate::repl::database::{
    ClarityDatabase, ClarityDeserializable, ClaritySerializable, HeadersDB,
};
use clarity::types::chainstate::{BlockHeaderHash, BurnchainHeaderHash, StacksBlockId, VRFSeed};
use clarity_repl::clarity::util::hash::{hex_bytes, to_hex, Sha512Trunc256Sum};
use clarity::vm::analysis::AnalysisDatabase;
use clarity::vm::errors::{
    CheckErrors, IncomparableError, InterpreterError, InterpreterResult as Result, RuntimeErrorType,
};
use clarity::vm::types::QualifiedContractIdentifier;

pub struct ContractCommitment {
    pub hash: Sha512Trunc256Sum,
    pub block_height: u32,
}

impl ClaritySerializable for ContractCommitment {
    fn serialize(&self) -> String {
        format!("{}{}", self.hash, to_hex(&self.block_height.to_be_bytes()))
    }
}

impl ClarityDeserializable<ContractCommitment> for ContractCommitment {
    fn deserialize(input: &str) -> ContractCommitment {
        assert_eq!(input.len(), 72);
        let hash = Sha512Trunc256Sum::from_hex(&input[0..64]).expect("Hex decode fail.");
        let height_bytes = hex_bytes(&input[64..72]).expect("Hex decode fail.");
        let block_height = u32::from_be_bytes(height_bytes.as_slice().try_into().unwrap());
        ContractCommitment { hash, block_height }
    }
}

