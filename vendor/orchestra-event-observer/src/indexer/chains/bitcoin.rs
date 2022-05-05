use crate::indexer::IndexerConfig;
use orchestra_types::{BitcoinBlockData, BitcoinBlockMetadata, BlockIdentifier};
use bitcoincore_rpc::bitcoin::hashes::Hash;
use bitcoincore_rpc::bitcoin::BlockHash;
use bitcoincore_rpc::{Auth, Client, RpcApi};
use clarity_repl::clarity::util::hash::hex_bytes;
use rocket::serde::json::Value as JsonValue;

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct NewBurnBlock {
    burn_block_hash: String,
    burn_block_height: u64,
    reward_slot_holders: Vec<String>,
    burn_amount: u64,
}

pub fn standardize_bitcoin_block(
    indexer_config: &IndexerConfig,
    marshalled_block: JsonValue,
) -> BitcoinBlockData {
    let transactions = vec![];

    let auth = Auth::UserPass(
        indexer_config.bitcoin_node_rpc_username.clone(),
        indexer_config.bitcoin_node_rpc_password.clone(),
    );

    let rpc = Client::new(&indexer_config.bitcoin_node_rpc_url, auth).unwrap();

    let partial_block: NewBurnBlock = serde_json::from_value(marshalled_block).unwrap();
    let block_height = partial_block.burn_block_height;
    let block_hash = {
        let block_hash_str = partial_block.burn_block_hash.strip_prefix("0x").unwrap();
        let mut block_hash_bytes = hex_bytes(&block_hash_str).unwrap();
        block_hash_bytes.reverse();
        BlockHash::from_slice(&block_hash_bytes).unwrap()
    };
    let block = rpc.get_block(&block_hash).unwrap();

    for _txdata in block.txdata.iter() {
        // TODO(lgalabru): retrieve stacks transactions
        // let _ = tx.send(DevnetEvent::debug(format!(
        //     "Tx.out: {:?}", txdata.output
        // )));
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
        metadata: BitcoinBlockMetadata {
        },
        transactions,
    }
}
