use std::collections::HashSet;

use clarity_repl::clarity::util::hash::{hex_bytes, to_hex};
use orchestra_types::{
    StacksContractCallData, StacksTransactionData, StacksTransactionKind,
    StacksTransactionMetadata, StacksTransactionReceipt, TransactionIdentifier,
};

use super::accounts;

pub fn generate_test_tx_contract_call(
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

    let contract_identifier = format!("{}.{}", accounts::deployer(), contract_name);

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
            position: orchestra_types::StacksTransactionPosition::Index(0),
        },
    }
}
