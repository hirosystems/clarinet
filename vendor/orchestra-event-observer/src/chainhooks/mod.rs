pub mod types;

use self::types::{
    BitcoinChainhookSpecification, BitcoinHookPredicate, ChainhookSpecification, HookAction,
    HookFormation, MatchingRule, StacksChainhookSpecification, StacksHookPredicate,
};
use base58::FromBase58;
use bitcoincore_rpc::bitcoin::blockdata::opcodes;
use bitcoincore_rpc::bitcoin::blockdata::script::Builder as BitcoinScriptBuilder;
use bitcoincore_rpc::bitcoin::{Address, PubkeyHash, PublicKey, Script};
use clarity_repl::clarity::util::hash::Hash160;
use orchestra_types::{
    BitcoinChainEvent, BitcoinTransactionData, BlockIdentifier, StacksChainEvent, StacksNetwork,
    StacksTransactionData, StacksTransactionEvent, StacksTransactionKind,
};
use reqwest::{Client, Method};
use std::str::FromStr;

pub fn evaluate_stacks_chainhooks_on_chain_event<'a>(
    chain_event: &'a StacksChainEvent,
    active_chainhooks: Vec<&'a StacksChainhookSpecification>,
) -> Vec<(
    &'a StacksChainhookSpecification,
    &'a StacksTransactionData,
    &'a BlockIdentifier,
)> {
    let mut enabled = vec![];
    match chain_event {
        StacksChainEvent::ChainUpdatedWithBlock(update) => {
            for tx in update.new_block.transactions.iter() {
                for hook in active_chainhooks.iter() {
                    match (&tx.metadata.kind, &hook.predicate) {
                        (
                            StacksTransactionKind::ContractCall(actual_contract_call),
                            StacksHookPredicate::ContractCall(expected_contract_call),
                        ) => {
                            if actual_contract_call.contract_identifier
                                == expected_contract_call.contract_identifier
                                && actual_contract_call.method == expected_contract_call.method
                            {
                                enabled.push((
                                    hook.clone(),
                                    tx,
                                    &update.new_block.block_identifier,
                                ));
                                continue;
                            }
                        }
                        (StacksTransactionKind::ContractCall(_), _)
                        | (StacksTransactionKind::ContractDeployment(_), _) => {
                            // Look for emitted events
                            for event in tx.metadata.receipt.events.iter() {
                                match (event, &hook.predicate) {
                                    (
                                        StacksTransactionEvent::NFTMintEvent(actual),
                                        StacksHookPredicate::NftEvent(expected),
                                    ) => {
                                        if actual.asset_class_identifier
                                            == expected.asset_identifier
                                            && expected.actions.contains(&"mint".to_string())
                                        {
                                            enabled.push((
                                                hook.clone(),
                                                tx,
                                                &update.new_block.block_identifier,
                                            ));
                                            break;
                                        }
                                    }
                                    (
                                        StacksTransactionEvent::NFTTransferEvent(actual),
                                        StacksHookPredicate::NftEvent(expected),
                                    ) => {
                                        if actual.asset_class_identifier
                                            == expected.asset_identifier
                                            && expected.actions.contains(&"transfer".to_string())
                                        {
                                            enabled.push((
                                                hook.clone(),
                                                tx,
                                                &update.new_block.block_identifier,
                                            ));
                                            break;
                                        }
                                    }
                                    (
                                        StacksTransactionEvent::NFTBurnEvent(actual),
                                        StacksHookPredicate::NftEvent(expected),
                                    ) => {
                                        if actual.asset_class_identifier
                                            == expected.asset_identifier
                                            && expected.actions.contains(&"burn".to_string())
                                        {
                                            enabled.push((
                                                hook.clone(),
                                                tx,
                                                &update.new_block.block_identifier,
                                            ));
                                            break;
                                        }
                                    }
                                    (
                                        StacksTransactionEvent::FTMintEvent(actual),
                                        StacksHookPredicate::FtEvent(expected),
                                    ) => {
                                        if actual.asset_class_identifier
                                            == expected.asset_identifier
                                            && expected.actions.contains(&"mint".to_string())
                                        {
                                            enabled.push((
                                                hook.clone(),
                                                tx,
                                                &update.new_block.block_identifier,
                                            ));
                                            break;
                                        }
                                    }
                                    (
                                        StacksTransactionEvent::FTTransferEvent(actual),
                                        StacksHookPredicate::FtEvent(expected),
                                    ) => {
                                        if actual.asset_class_identifier
                                            == expected.asset_identifier
                                            && expected.actions.contains(&"transfer".to_string())
                                        {
                                            enabled.push((
                                                hook.clone(),
                                                tx,
                                                &update.new_block.block_identifier,
                                            ));
                                            break;
                                        }
                                    }
                                    (
                                        StacksTransactionEvent::FTBurnEvent(actual),
                                        StacksHookPredicate::FtEvent(expected),
                                    ) => {
                                        if actual.asset_class_identifier
                                            == expected.asset_identifier
                                            && expected.actions.contains(&"burn".to_string())
                                        {
                                            enabled.push((
                                                hook.clone(),
                                                tx,
                                                &update.new_block.block_identifier,
                                            ));
                                            break;
                                        }
                                    }
                                    (
                                        StacksTransactionEvent::STXMintEvent(_),
                                        StacksHookPredicate::StxEvent(expected),
                                    ) => {
                                        if expected.actions.contains(&"mint".to_string()) {
                                            enabled.push((
                                                hook.clone(),
                                                tx,
                                                &update.new_block.block_identifier,
                                            ));
                                            break;
                                        }
                                    }
                                    (
                                        StacksTransactionEvent::STXTransferEvent(_),
                                        StacksHookPredicate::StxEvent(expected),
                                    ) => {
                                        if expected.actions.contains(&"transfer".to_string()) {
                                            enabled.push((
                                                hook.clone(),
                                                tx,
                                                &update.new_block.block_identifier,
                                            ));
                                            break;
                                        }
                                    }
                                    (
                                        StacksTransactionEvent::STXLockEvent(_),
                                        StacksHookPredicate::StxEvent(expected),
                                    ) => {
                                        if expected.actions.contains(&"lock".to_string()) {
                                            enabled.push((
                                                hook.clone(),
                                                tx,
                                                &update.new_block.block_identifier,
                                            ));
                                            break;
                                        }
                                    }
                                    (
                                        StacksTransactionEvent::SmartContractEvent(actual),
                                        StacksHookPredicate::PrintEvent(expected),
                                    ) => {
                                        if actual.contract_identifier
                                            == expected.contract_identifier
                                        {
                                            enabled.push((
                                                hook.clone(),
                                                tx,
                                                &update.new_block.block_identifier,
                                            ));
                                            break;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        (
                            StacksTransactionKind::NativeTokenTransfer,
                            StacksHookPredicate::StxEvent(expected_stx_event),
                        ) => {}
                        _ => {}
                    }
                    if let StacksTransactionKind::ContractCall(actual_contract_call) =
                        &tx.metadata.kind
                    {
                        match &hook.predicate {
                            StacksHookPredicate::ContractCall(expected_contract_call) => {}
                            StacksHookPredicate::PrintEvent(expected_print_event) => {}
                            StacksHookPredicate::StxEvent(expected_stx_event) => unimplemented!(),
                            StacksHookPredicate::NftEvent(expected_nft_event) => unimplemented!(),
                            StacksHookPredicate::FtEvent(expected_ft_event) => {}
                        }
                    }
                }
            }
        }
        StacksChainEvent::ChainUpdatedWithMicroblock(update) => {}
        StacksChainEvent::ChainUpdatedWithMicroblockReorg(update) => {}
        StacksChainEvent::ChainUpdatedWithReorg(update) => {}
    }
    enabled
}

pub fn evaluate_bitcoin_chainhooks_on_chain_event<'a>(
    chain_event: &'a BitcoinChainEvent,
    active_chainhooks: Vec<&'a BitcoinChainhookSpecification>,
) -> Vec<(
    &'a BitcoinChainhookSpecification,
    &'a BitcoinTransactionData,
    &'a BlockIdentifier,
)> {
    let mut enabled = vec![];
    match chain_event {
        BitcoinChainEvent::ChainUpdatedWithBlock(block) => {
            for hook in active_chainhooks.into_iter() {
                for tx in block.transactions.iter() {
                    if hook.evaluate_predicate(&tx) {
                        enabled.push((hook, tx, &block.block_identifier));
                    }
                }
            }
        }
        BitcoinChainEvent::ChainUpdatedWithReorg(old_blocks, new_blocks) => {}
    }
    enabled
}

pub async fn handle_bitcoin_hook_action<'a>(
    hook: &'a BitcoinChainhookSpecification,
    tx: &'a BitcoinTransactionData,
    block_identifier: &'a BlockIdentifier,
    proof: Option<&String>,
) {
    match &hook.action {
        HookAction::Http(http) => {
            let client = Client::builder().build().unwrap();
            let host = format!("{}", http.url);
            let method = Method::from_bytes(http.method.as_bytes()).unwrap();
            let payload = json!({
                "apply": vec![json!({
                    "transaction": tx,
                    "proof": proof,
                    "block_identifier": block_identifier,
                    "confirmations": 1,
                })],
                "chainhook": {
                    "uuid": hook.uuid,
                    "predicate": hook.predicate,
                }
            });
            let body = serde_json::to_vec(&payload).unwrap();
            let _ = client
                .request(method, &host)
                .header("Content-Type", "application/json")
                .header("Authorization", http.authorization_header.clone())
                .body(body)
                .send()
                .await;
        }
    }
}

pub async fn handle_stacks_hook_action<'a>(
    hook: &'a StacksChainhookSpecification,
    tx: &'a StacksTransactionData,
    block_identifier: &'a BlockIdentifier,
    proof: Option<&String>,
) {
    match &hook.action {
        HookAction::Http(http) => {
            let client = Client::builder().build().unwrap();
            let host = format!("{}", http.url);
            let method = Method::from_bytes(http.method.as_bytes()).unwrap();
            let payload = json!({
                "apply": vec![json!({
                    "transaction": tx,
                    "proof": proof,
                    "block_identifier": block_identifier,
                    "confirmations": 1,
                })],
                "chainhook": {
                    "uuid": hook.uuid,
                    "predicate": hook.predicate,
                }
            });
            let body = serde_json::to_vec(&payload).unwrap();
            let _ = client
                .request(method, &host)
                .header("Content-Type", "application/json")
                .body(body)
                .send()
                .await;
        }
    }
}

impl BitcoinChainhookSpecification {
    pub fn evaluate_predicate(&self, tx: &BitcoinTransactionData) -> bool {
        match &self.predicate.kind {
            types::BitcoinPredicateType::Hex(MatchingRule::Equals(_address)) => false,
            types::BitcoinPredicateType::Hex(MatchingRule::StartsWith(_address)) => false,
            types::BitcoinPredicateType::Hex(MatchingRule::EndsWith(_address)) => false,
            types::BitcoinPredicateType::P2pkh(MatchingRule::Equals(address)) => {
                let pubkey_hash = address
                    .from_base58()
                    .expect("Unable to get bytes from btc address");
                let script = BitcoinScriptBuilder::new()
                    .push_opcode(opcodes::all::OP_DUP)
                    .push_opcode(opcodes::all::OP_HASH160)
                    .push_slice(&pubkey_hash[1..21])
                    .push_opcode(opcodes::all::OP_EQUALVERIFY)
                    .push_opcode(opcodes::all::OP_CHECKSIG)
                    .into_script();

                for output in tx.metadata.outputs.iter() {
                    if output.script_pubkey == script {
                        return true;
                    }
                }
                false
            }
            types::BitcoinPredicateType::P2pkh(MatchingRule::StartsWith(_address)) => false,
            types::BitcoinPredicateType::P2pkh(MatchingRule::EndsWith(_address)) => false,
            types::BitcoinPredicateType::P2sh(MatchingRule::Equals(_address)) => false,
            types::BitcoinPredicateType::P2sh(MatchingRule::StartsWith(_address)) => false,
            types::BitcoinPredicateType::P2sh(MatchingRule::EndsWith(_address)) => false,
            types::BitcoinPredicateType::P2wpkh(MatchingRule::Equals(_address)) => false,
            types::BitcoinPredicateType::P2wpkh(MatchingRule::StartsWith(_address)) => false,
            types::BitcoinPredicateType::P2wpkh(MatchingRule::EndsWith(_address)) => false,
            types::BitcoinPredicateType::P2wsh(MatchingRule::Equals(_address)) => false,
            types::BitcoinPredicateType::P2wsh(MatchingRule::StartsWith(_address)) => false,
            types::BitcoinPredicateType::P2wsh(MatchingRule::EndsWith(_address)) => false,
            types::BitcoinPredicateType::Script(template) => false,
        }
    }
}
