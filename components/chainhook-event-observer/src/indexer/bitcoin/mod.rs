mod blocks_pool;
use crate::chainhooks::types::{
    get_canonical_magic_bytes, get_canonical_pox_config, PoxConfig, StacksOpcodes,
};
use crate::indexer::IndexerConfig;
use bitcoincore_rpc::bitcoin::hashes::Hash;
use bitcoincore_rpc::bitcoin::{Block, BlockHash};
use bitcoincore_rpc::{Auth, Client, RpcApi};
pub use blocks_pool::BitcoinBlockPool;
use chainhook_types::bitcoin::{OutPoint, TxIn, TxOut};
use chainhook_types::{
    BitcoinBlockData, BitcoinBlockMetadata, BitcoinTransactionData, BitcoinTransactionMetadata,
    BlockCommitmentData, BlockIdentifier, KeyRegistrationData, LockSTXData, PobBlockCommitmentData,
    PoxBlockCommitmentData, PoxReward, StacksBaseChainOperation, TransactionIdentifier,
    TransferSTXData,
};
use clarity_repl::clarity::deps_common::bitcoin::blockdata::script::Script;
use clarity_repl::clarity::util::hash::{hex_bytes, to_hex};
use rocket::serde::json::Value as JsonValue;

#[derive(Deserialize)]
pub struct NewBitcoinBlock {
    pub burn_block_hash: String,
    pub burn_block_height: u64,
    pub reward_slot_holders: Vec<String>,
    pub reward_recipients: Vec<RewardParticipant>,
    pub burn_amount: u64,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct RewardParticipant {
    recipient: String,
    amt: u64,
}

pub fn standardize_bitcoin_block(
    indexer_config: &IndexerConfig,
    marshalled_block: JsonValue,
) -> BitcoinBlockData {
    let mut transactions = vec![];

    let auth = Auth::UserPass(
        indexer_config.bitcoin_node_rpc_username.clone(),
        indexer_config.bitcoin_node_rpc_password.clone(),
    );

    let rpc = Client::new(&indexer_config.bitcoin_node_rpc_url, auth).unwrap();

    let partial_block: NewBitcoinBlock = serde_json::from_value(marshalled_block).unwrap();
    let block_height = partial_block.burn_block_height;
    let block_hash = {
        let block_hash_str = partial_block.burn_block_hash.strip_prefix("0x").unwrap();
        let mut block_hash_bytes = hex_bytes(&block_hash_str).unwrap();
        block_hash_bytes.reverse();
        BlockHash::from_slice(&block_hash_bytes).unwrap()
    };
    let block = rpc.get_block(&block_hash).unwrap();
    let block_height = partial_block.burn_block_height;
    let expected_magic_bytes = get_canonical_magic_bytes(&indexer_config.bitcoin_network);
    let pox_config = get_canonical_pox_config(&indexer_config.bitcoin_network);

    for mut tx in block.txdata.into_iter() {
        let txid = tx.txid().to_string();
        let mut inputs = vec![];
        for input in tx.input.drain(..) {
            inputs.push(TxIn {
                previous_output: OutPoint {
                    txid: input.previous_output.txid.to_string(),
                    vout: input.previous_output.vout,
                },
                script_sig: to_hex(input.script_sig.as_bytes()),
                sequence: input.sequence,
                witness: input.witness,
            })
        }

        let mut outputs = vec![];
        let mut stacks_operations = vec![];

        if let Some(op) =
            try_parse_stacks_operation(&tx.output, &pox_config, &expected_magic_bytes, block_height)
        {
            stacks_operations.push(op);
        }

        for output in tx.output.drain(..) {
            outputs.push(TxOut {
                value: output.value,
                script_pubkey: to_hex(output.script_pubkey.as_bytes()),
            });
        }

        let tx = BitcoinTransactionData {
            transaction_identifier: TransactionIdentifier {
                hash: format!("0x{}", txid),
            },
            operations: vec![],
            metadata: BitcoinTransactionMetadata {
                inputs,
                outputs,
                stacks_operations,
            },
        };
        transactions.push(tx);
    }

    BitcoinBlockData {
        block_identifier: BlockIdentifier {
            hash: format!("0x{}", block.header.block_hash().to_string()),
            index: block_height,
        },
        parent_block_identifier: BlockIdentifier {
            hash: format!("0x{}", block.header.prev_blockhash.to_string()),
            index: block_height - 1,
        },
        timestamp: block.header.time,
        metadata: BitcoinBlockMetadata {},
        transactions,
    }
}

fn try_parse_stacks_operation(
    outputs: &Vec<bitcoincore_rpc::bitcoin::TxOut>,
    pox_config: &PoxConfig,
    expected_magic_bytes: &[u8; 2],
    block_height: u64,
) -> Option<StacksBaseChainOperation> {
    if outputs.is_empty() {
        return None;
    }

    if !outputs[0].script_pubkey.is_op_return() {
        return None;
    }

    // Safely parsing the first 2 bytes (following OP_RETURN + PUSH_DATA)
    let op_return_output = outputs[0].script_pubkey.as_bytes();
    if op_return_output.len() < 6 {
        return None;
    }
    if op_return_output[2] != expected_magic_bytes[0]
        || op_return_output[3] != expected_magic_bytes[1]
    {
        return None;
    }
    // Safely classifying the Stacks operation;
    let op_type: StacksOpcodes = match op_return_output[4].try_into() {
        Ok(op) => op,
        Err(_) => {
            debug!(
                "Stacks operation parsing - opcode unknown {}",
                op_return_output[4]
            );
            return None;
        }
    };
    let op = match op_type {
        StacksOpcodes::KeyRegister => {
            let res = try_parse_key_register_op(&op_return_output[5..])?;
            StacksBaseChainOperation::KeyRegistration(res)
        }
        StacksOpcodes::PreStx => {
            let res = try_parse_pre_stx_op(&op_return_output[5..])?;
            return None;
        }
        StacksOpcodes::TransferStx => {
            let res = try_parse_transfer_stx_op(&op_return_output[5..])?;
            StacksBaseChainOperation::TransferSTX(res)
        }
        StacksOpcodes::StackStx => {
            let res = try_parse_stacks_stx_op(&op_return_output[5..])?;
            StacksBaseChainOperation::LockSTX(res)
        }
        StacksOpcodes::BlockCommit => {
            let res = try_parse_block_commit_op(&op_return_output[5..])?;
            // We need to determine wether the transaction was a PoB or a Pox commitment
            if pox_config.is_consensus_rewarding_participants_at_block_height(block_height) {
                if outputs.len() < 1 + pox_config.rewarded_addresses_per_block {
                    return None;
                }
                let mut rewards = vec![];
                for output in outputs[1..pox_config.rewarded_addresses_per_block].into_iter() {
                    rewards.push(PoxReward {
                        recipient: output.script_pubkey.to_string(),
                        amount: output.value,
                    });
                }
                StacksBaseChainOperation::PoxBlockCommitment(PoxBlockCommitmentData {
                    signers: vec![], // todo(lgalabru)
                    stacks_block_hash: res.stacks_block_hash.clone(),
                    rewards,
                })
            } else {
                if outputs.len() < 2 {
                    return None;
                }
                let amount = outputs[1].value;
                StacksBaseChainOperation::PobBlockCommitment(PobBlockCommitmentData {
                    signers: vec![], // todo(lgalabru)
                    stacks_block_hash: res.stacks_block_hash.clone(),
                    amount,
                })
            }
        }
    };

    Some(op)
}

fn try_parse_block_commit_op(bytes: &[u8]) -> Option<BlockCommitmentData> {
    if bytes.len() < 32 {
        return None;
    }

    Some(BlockCommitmentData {
        stacks_block_hash: format!("0x{}", to_hex(&bytes[0..32])),
    })
}

fn try_parse_key_register_op(bytes: &[u8]) -> Option<KeyRegistrationData> {
    Some(KeyRegistrationData {})
}

fn try_parse_pre_stx_op(bytes: &[u8]) -> Option<()> {
    None
}

fn try_parse_transfer_stx_op(bytes: &[u8]) -> Option<TransferSTXData> {
    if bytes.len() < 16 {
        return None;
    }

    // todo(lgalabru)
    Some(TransferSTXData {
        sender: "".into(),
        recipient: "".into(),
        amount: "".into(),
    })
}

fn try_parse_stacks_stx_op(bytes: &[u8]) -> Option<LockSTXData> {
    if bytes.len() < 16 {
        return None;
    }

    // todo(lgalabru)
    Some(LockSTXData {
        sender: "".into(),
        amount: "".into(),
        duration: 1,
    })
}

#[cfg(test)]
pub mod tests;
