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
) -> Result<BitcoinBlockData, String> {
    let auth = Auth::UserPass(
        indexer_config.bitcoin_node_rpc_username.clone(),
        indexer_config.bitcoin_node_rpc_password.clone(),
    );
    let rpc = Client::new(&indexer_config.bitcoin_node_rpc_url, auth).map_err(|e| {
        format!(
            "unable for bitcoin rpc initialize client: {}",
            e.to_string()
        )
    })?;
    let partial_block: NewBitcoinBlock = serde_json::from_value(marshalled_block)
        .map_err(|e| format!("unable for parse bitcoin block: {}", e.to_string()))?;
    let block_hash = {
        let block_hash_str = partial_block.burn_block_hash.strip_prefix("0x").unwrap();
        let mut block_hash_bytes = hex_bytes(&block_hash_str).unwrap();
        block_hash_bytes.reverse();
        BlockHash::from_slice(&block_hash_bytes).unwrap()
    };
    let block = rpc
        .get_block(&block_hash)
        .map_err(|e| format!("unable for invoke rpc get_block: {}", e.to_string()))?;
    let block_height = partial_block.burn_block_height;
    Ok(build_block(block, block_height, indexer_config))
}

pub fn build_block(
    block: Block,
    block_height: u64,
    indexer_config: &IndexerConfig,
) -> BitcoinBlockData {
    let mut transactions = vec![];

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
                sequence: input.sequence.0,
                witness: input
                    .witness
                    .to_vec()
                    .iter()
                    .map(|w| format!("0x{}", to_hex(w)))
                    .collect::<Vec<_>>(),
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
                proof: None,
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
    if op_return_output.len() < 7 {
        return None;
    }
    if op_return_output[3] != expected_magic_bytes[0]
        || op_return_output[4] != expected_magic_bytes[1]
    {
        return None;
    }
    // Safely classifying the Stacks operation;
    let op_type: StacksOpcodes = match op_return_output[5].try_into() {
        Ok(op) => op,
        Err(_) => {
            debug!(
                "Stacks operation parsing - opcode unknown {}",
                op_return_output[5]
            );
            return None;
        }
    };
    let op = match op_type {
        StacksOpcodes::KeyRegister => {
            let res = try_parse_key_register_op(&op_return_output[6..])?;
            StacksBaseChainOperation::KeyRegistration(res)
        }
        StacksOpcodes::PreStx => {
            let _ = try_parse_pre_stx_op(&op_return_output[6..])?;
            return None;
        }
        StacksOpcodes::TransferStx => {
            let res = try_parse_transfer_stx_op(&op_return_output[6..])?;
            StacksBaseChainOperation::TransferSTX(res)
        }
        StacksOpcodes::StackStx => {
            let res = try_parse_stacks_stx_op(&op_return_output[6..])?;
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

fn try_parse_key_register_op(_bytes: &[u8]) -> Option<KeyRegistrationData> {
    Some(KeyRegistrationData {})
}

fn try_parse_pre_stx_op(_bytes: &[u8]) -> Option<()> {
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

// Test vectors
// 1) Devnet PoB
// 2022-10-26T03:06:17.376341Z  INFO chainhook_event_observer::indexer: BitcoinBlockData { block_identifier: BlockIdentifier { index: 104, hash: "0x210d0d095a75d88fc059cb97f453eee33b1833153fb1f81b9c3c031c26bb106b" }, parent_block_identifier: BlockIdentifier { index: 103, hash: "0x5d5a4b8113c35f20fb0b69b1fb1ae1b88461ea57e2a2e4c036f97fae70ca1abb" }, timestamp: 1666753576, transactions: [BitcoinTransactionData { transaction_identifier: TransactionIdentifier { hash: "0xfaaac1833dc4883e7ec28f61e35b41f896c395f8d288b1a177155de2abd6052f" }, operations: [], metadata: BitcoinTransactionMetadata { inputs: [TxIn { previous_output: OutPoint { txid: "0000000000000000000000000000000000000000000000000000000000000000", vout: 4294967295 }, script_sig: "01680101", sequence: 4294967295, witness: [] }], outputs: [TxOut { value: 5000017550, script_pubkey: "76a914ee9369fb719c0ba43ddf4d94638a970b84775f4788ac" }, TxOut { value: 0, script_pubkey: "6a24aa21a9ed4a190dfdc77e260409c2a693e6d3b8eca43afbc4bffb79ddcdcc9516df804d9b" }], stacks_operations: [] } }, BitcoinTransactionData { transaction_identifier: TransactionIdentifier { hash: "0x59193c24cb2325cd2271b89f790f958dcd4065088680ffbc201a0ebb2f3cbf25" }, operations: [], metadata: BitcoinTransactionMetadata { inputs: [TxIn { previous_output: OutPoint { txid: "9eebe848baaf8dd4810e4e4a91168e2e471c949439faf5d768750ca21d067689", vout: 3 }, script_sig: "483045022100a20f90e9e3c3bb7e558ad4fa65902d8cf6ce4bff1f5af0ac0a323b547385069c022021b9877abbc9d1eef175c7f712ac1b2d8f5ce566be542714effe42711e75b83801210239810ebf35e6f6c26062c99f3e183708d377720617c90a986859ec9c95d00be9", sequence: 4294967293, witness: [] }], outputs: [TxOut { value: 0, script_pubkey: "6a4c5069645b1681995f8e568287e0e4f5cbc1d6727dafb5e3a7822a77c69bd04208265aca9424d0337dac7d9e84371a2c91ece1891d67d3554bd9fdbe60afc6924d4b0773d90000006700010000006600012b" }, TxOut { value: 10000, script_pubkey: "76a914000000000000000000000000000000000000000088ac" }, TxOut { value: 10000, script_pubkey: "76a914000000000000000000000000000000000000000088ac" }, TxOut { value: 4999904850, script_pubkey: "76a914ee9369fb719c0ba43ddf4d94638a970b84775f4788ac" }], stacks_operations: [PobBlockCommitment(PobBlockCommitmentData { signers: [], stacks_block_hash: "0x5b1681995f8e568287e0e4f5cbc1d6727dafb5e3a7822a77c69bd04208265aca", amount: 10000 })] } }], metadata: BitcoinBlockMetadata }
// 2022-10-26T03:06:21.929157Z  INFO chainhook_event_observer::indexer: BitcoinBlockData { block_identifier: BlockIdentifier { index: 105, hash: "0x0302c4c6063eb7199d3a565351bceeea9df4cb4aa09293194dbab277e46c2979" }, parent_block_identifier: BlockIdentifier { index: 104, hash: "0x210d0d095a75d88fc059cb97f453eee33b1833153fb1f81b9c3c031c26bb106b" }, timestamp: 1666753581, transactions: [BitcoinTransactionData { transaction_identifier: TransactionIdentifier { hash: "0xe7de433aa89c1f946f89133b0463b6cfebb26ad73b0771a79fd66c6acbfe3fb9" }, operations: [], metadata: BitcoinTransactionMetadata { inputs: [TxIn { previous_output: OutPoint { txid: "0000000000000000000000000000000000000000000000000000000000000000", vout: 4294967295 }, script_sig: "01690101", sequence: 4294967295, witness: [] }], outputs: [TxOut { value: 5000017600, script_pubkey: "76a914ee9369fb719c0ba43ddf4d94638a970b84775f4788ac" }, TxOut { value: 0, script_pubkey: "6a24aa21a9ed98ac3bc4e0c9ed53e3418a3bf3aa511dcd76088cf0e1c4fc71fb9755840d7a08" }], stacks_operations: [] } }, BitcoinTransactionData { transaction_identifier: TransactionIdentifier { hash: "0xe654501805d80d59ef0d95b57ad7a924f3be4a4dc0db5a785dfebe1f70c4e23e" }, operations: [], metadata: BitcoinTransactionMetadata { inputs: [TxIn { previous_output: OutPoint { txid: "59193c24cb2325cd2271b89f790f958dcd4065088680ffbc201a0ebb2f3cbf25", vout: 3 }, script_sig: "483045022100b59d2d07f68ea3a4f27a49979080a07b2432cfad9fc90e1edd0241496f0fd83f02205ac233f4cb68ada487f16339abedb7093948b683ba7d76b3b4058b2c0181a68901210239810ebf35e6f6c26062c99f3e183708d377720617c90a986859ec9c95d00be9", sequence: 4294967293, witness: [] }], outputs: [TxOut { value: 0, script_pubkey: "6a4c5069645b351bb015ef4f7dcdce4c9d95cbf157f85a3714626252cfc9078f3f1591ccdb13c3c7e22b34c4ffc2f6064a41df6fcd7f1b759d4f28b2f7cb6b27f283c868406e0000006800010000006600012c" }, TxOut { value: 10000, script_pubkey: "76a914000000000000000000000000000000000000000088ac" }, TxOut { value: 10000, script_pubkey: "76a914000000000000000000000000000000000000000088ac" }, TxOut { value: 4999867250, script_pubkey: "76a914ee9369fb719c0ba43ddf4d94638a970b84775f4788ac" }], stacks_operations: [PobBlockCommitment(PobBlockCommitmentData { signers: [], stacks_block_hash: "0x5b351bb015ef4f7dcdce4c9d95cbf157f85a3714626252cfc9078f3f1591ccdb", amount: 10000 })] } }], metadata: BitcoinBlockMetadata }
// 2022-10-26T03:07:53.298531Z  INFO chainhook_event_observer::indexer: BitcoinBlockData { block_identifier: BlockIdentifier { index: 106, hash: "0x52eb2aa15aa99afc4b918a552cef13e8b6eed84b257be097ad954b4f37a7e98d" }, parent_block_identifier: BlockIdentifier { index: 105, hash: "0x0302c4c6063eb7199d3a565351bceeea9df4cb4aa09293194dbab277e46c2979" }, timestamp: 1666753672, transactions: [BitcoinTransactionData { transaction_identifier: TransactionIdentifier { hash: "0xd28d7f5411416f94b95e9f999d5ee8ded5543ba9daae9f612b80f01c5107862d" }, operations: [], metadata: BitcoinTransactionMetadata { inputs: [TxIn { previous_output: OutPoint { txid: "0000000000000000000000000000000000000000000000000000000000000000", vout: 4294967295 }, script_sig: "016a0101", sequence: 4294967295, witness: [] }], outputs: [TxOut { value: 5000017500, script_pubkey: "76a914ee9369fb719c0ba43ddf4d94638a970b84775f4788ac" }, TxOut { value: 0, script_pubkey: "6a24aa21a9ed71aaf7e5384879a1b112bf623ac8b46dd88b39c3d2c6f8a1d264fc4463e6356a" }], stacks_operations: [] } }, BitcoinTransactionData { transaction_identifier: TransactionIdentifier { hash: "0x72e8e43afc4362cf921ccc57fde3e07b4cb6fac5f306525c86d38234c18e21d1" }, operations: [], metadata: BitcoinTransactionMetadata { inputs: [TxIn { previous_output: OutPoint { txid: "e654501805d80d59ef0d95b57ad7a924f3be4a4dc0db5a785dfebe1f70c4e23e", vout: 3 }, script_sig: "4730440220798bb7d7fb14df35610db2ef04e5d5b6588440b7c429bf650a96f8570904052b02204a817e13e7296a24a8f6cc8737bddb55d1835e513ec2b9dcb03424e4536ae34c01210239810ebf35e6f6c26062c99f3e183708d377720617c90a986859ec9c95d00be9", sequence: 4294967293, witness: [] }], outputs: [TxOut { value: 0, script_pubkey: "6a4c5069645b504d310fc27c86a6b65d0b0e0297db1e185d3432fdab9fa96a1053407ed07b537b8b7d23c6309dfd24340e85b75cff11ad685f8b310c1d2098748a0fffb146ec00000069000100000066000128" }, TxOut { value: 20000, script_pubkey: "76a914000000000000000000000000000000000000000088ac" }, TxOut { value: 4999829750, script_pubkey: "76a914ee9369fb719c0ba43ddf4d94638a970b84775f4788ac" }], stacks_operations: [PobBlockCommitment(PobBlockCommitmentData { signers: [], stacks_block_hash: "0x5b504d310fc27c86a6b65d0b0e0297db1e185d3432fdab9fa96a1053407ed07b", amount: 20000 })] } }], metadata: BitcoinBlockMetadata }
