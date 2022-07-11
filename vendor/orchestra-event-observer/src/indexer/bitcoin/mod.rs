mod blocks_pool;

use crate::indexer::IndexerConfig;
use bitcoincore_rpc::bitcoin::hashes::Hash;
use bitcoincore_rpc::bitcoin::BlockHash;
use bitcoincore_rpc::{Auth, Client, RpcApi};
pub use blocks_pool::BitcoinBlockPool;
use clarity_repl::clarity::util::hash::{hex_bytes, to_hex};
use orchestra_types::bitcoin::{OutPoint, TxIn, TxOut};
use orchestra_types::{
    BitcoinBlockData, BitcoinBlockMetadata, BitcoinTransactionData, BitcoinTransactionMetadata,
    BlockIdentifier, TransactionIdentifier,
};
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

    for mut tx in block.txdata.into_iter() {
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
        for output in tx.output.drain(..) {
            outputs.push(TxOut {
                value: output.value,
                script_pubkey: to_hex(output.script_pubkey.as_bytes()),
            })
        }

        let tx = BitcoinTransactionData {
            transaction_identifier: TransactionIdentifier {
                hash: tx.txid().to_string(),
            },
            operations: vec![],
            metadata: BitcoinTransactionMetadata { inputs, outputs },
        };
        transactions.push(tx);
    }

    BitcoinBlockData {
        block_identifier: BlockIdentifier {
            hash: block.header.block_hash().to_string(),
            index: block_height,
        },
        parent_block_identifier: BlockIdentifier {
            hash: block.header.prev_blockhash.to_string(),
            index: block_height - 1,
        },
        timestamp: block.header.time,
        metadata: BitcoinBlockMetadata {},
        transactions,
    }
}

#[cfg(test)]
pub mod tests;
