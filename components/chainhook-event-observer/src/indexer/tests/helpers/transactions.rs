use std::collections::HashSet;

use base58::FromBase58;
use bitcoincore_rpc::bitcoin::blockdata::opcodes;
use bitcoincore_rpc::bitcoin::blockdata::script::Builder as BitcoinScriptBuilder;
use chainhook_types::bitcoin::TxOut;
use chainhook_types::{
    BitcoinTransactionData, BitcoinTransactionMetadata, StacksContractCallData,
    StacksTransactionData, StacksTransactionKind, StacksTransactionMetadata,
    StacksTransactionReceipt, TransactionIdentifier,
};
use clarity_repl::clarity::util::hash::to_hex;

use super::accounts;

pub fn generate_test_tx_stacks_contract_call(
    txid: u64,
    sender: &str,
    contract_name: &str,
    method: &str,
    args: Vec<&str>,
) -> StacksTransactionData {
    let mut hash = vec![
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    hash.append(&mut txid.to_be_bytes().to_vec());

    let contract_identifier = format!("{}.{}", accounts::deployer_stx_address(), contract_name);

    // Preparing metadata
    let mut mutated_contracts_radius = HashSet::new();
    mutated_contracts_radius.insert(contract_identifier.to_string());

    let mutated_assets_radius = HashSet::new();

    let contract_calls_stack = HashSet::new();

    let events = vec![];

    StacksTransactionData {
        transaction_identifier: TransactionIdentifier {
            hash: to_hex(&hash[..]),
        },
        operations: vec![],
        metadata: StacksTransactionMetadata {
            success: true,
            raw_tx: format!("__raw_tx__"),
            result: format!("(ok true)"),
            sender: format!("{}", sender),
            fee: 0,
            kind: StacksTransactionKind::ContractCall(StacksContractCallData {
                contract_identifier: contract_identifier.to_string(),
                method: method.to_string(),
                args: args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>(),
            }),
            execution_cost: None,
            receipt: StacksTransactionReceipt {
                mutated_contracts_radius,
                mutated_assets_radius,
                contract_calls_stack,
                events,
            },
            description: format!("contract call {}::{}", contract_identifier, method),
            sponsor: None,
            position: chainhook_types::StacksTransactionPosition::Index(0),
            proof: None,
        },
    }
}

pub fn generate_test_tx_bitcoin_p2pkh_transfer(
    txid: u64,
    _sender: &str,
    recipient: &str,
    amount: u64,
) -> BitcoinTransactionData {
    let mut hash = vec![
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    ];
    hash.append(&mut txid.to_be_bytes().to_vec());

    // Preparing metadata
    let pubkey_hash = recipient
        .from_base58()
        .expect("Unable to get bytes from btc address");
    let script = BitcoinScriptBuilder::new()
        .push_opcode(opcodes::all::OP_DUP)
        .push_opcode(opcodes::all::OP_HASH160)
        .push_slice(&pubkey_hash[1..21])
        .push_opcode(opcodes::all::OP_EQUALVERIFY)
        .push_opcode(opcodes::all::OP_CHECKSIG)
        .into_script();
    let outputs = vec![TxOut {
        value: amount,
        script_pubkey: to_hex(script.as_bytes()),
    }];

    BitcoinTransactionData {
        transaction_identifier: TransactionIdentifier {
            hash: format!("0x{}", to_hex(&hash[..])),
        },
        operations: vec![],
        metadata: BitcoinTransactionMetadata {
            inputs: vec![],
            outputs,
            stacks_operations: vec![],
            proof: None,
        },
    }
}
