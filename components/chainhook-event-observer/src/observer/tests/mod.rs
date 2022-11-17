use crate::chainhooks::types::{
    BitcoinChainhookSpecification, BitcoinPredicateType, BitcoinTransactionFilterPredicate,
    ChainhookSpecification, ExactMatchingRule, HookAction, HookFormation, Scope,
    StacksChainhookSpecification, StacksContractCallBasedPredicate,
    StacksTransactionFilterPredicate,
};
use crate::indexer::tests::helpers::transactions::generate_test_tx_bitcoin_p2pkh_transfer;
use crate::indexer::tests::helpers::{
    accounts, bitcoin_blocks, stacks_blocks, transactions::generate_test_tx_stacks_contract_call,
};
use crate::observer::{
    start_observer_commands_handler, ApiKey, ChainhookStore, EventObserverConfig, ObserverCommand,
};
use chainhook_types::{
    BitcoinChainEvent, BitcoinChainUpdatedWithBlocksData, BitcoinNetwork, StacksBlockUpdate,
    StacksChainEvent, StacksChainUpdatedWithBlocksData, StacksNetwork,
};
use hiro_system_kit;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, RwLock};

use super::ObserverEvent;

fn generate_test_config() -> (EventObserverConfig, ChainhookStore) {
    let operators = HashSet::new();
    let config = EventObserverConfig {
        normalization_enabled: true,
        grpc_server_enabled: false,
        hooks_enabled: true,
        initial_hook_formation: Some(HookFormation::new()),
        bitcoin_rpc_proxy_enabled: false,
        event_handlers: vec![],
        ingestion_port: 0,
        control_port: 0,
        bitcoin_node_username: "user".into(),
        bitcoin_node_password: "user".into(),
        bitcoin_node_rpc_url: "http://localhost:20443".into(),
        stacks_node_rpc_url: "http://localhost:18443".into(),
        operators,
        display_logs: false,
    };
    let mut entries = HashMap::new();
    entries.insert(ApiKey(None), HookFormation::new());
    let chainhook_store = ChainhookStore { entries };
    (config, chainhook_store)
}

fn stacks_chainhook_contract_call(
    id: u8,
    contract_identifier: &str,
    method: &str,
) -> StacksChainhookSpecification {
    let spec = StacksChainhookSpecification {
        uuid: format!("{}", id),
        name: format!("Chainhook {}", id),
        network: StacksNetwork::Devnet,
        version: 1,
        start_block: None,
        end_block: None,
        expire_after_occurrence: None,
        transaction_predicate: StacksTransactionFilterPredicate::ContractCall(
            StacksContractCallBasedPredicate {
                contract_identifier: contract_identifier.to_string(),
                method: method.to_string(),
            },
        ),
        block_predicate: None,
        action: HookAction::Noop,
        capture_all_events: None,
        decode_clarity_values: Some(true),
    };
    spec
}

fn bitcoin_chainhook_p2pkh(
    id: u8,
    address: &str,
    expire_after_occurrence: Option<u64>,
) -> BitcoinChainhookSpecification {
    let spec = BitcoinChainhookSpecification {
        uuid: format!("{}", id),
        name: format!("Chainhook {}", id),
        network: BitcoinNetwork::Regtest,
        version: 1,
        start_block: None,
        end_block: None,
        expire_after_occurrence,
        predicate: BitcoinTransactionFilterPredicate {
            scope: Scope::Outputs,
            kind: BitcoinPredicateType::P2pkh(ExactMatchingRule::Equals(address.to_string())),
        },
        action: HookAction::Noop,
    };
    spec
}

fn generate_and_register_new_stacks_chainhook(
    observer_commands_tx: &Sender<ObserverCommand>,
    observer_events_rx: &Receiver<ObserverEvent>,
    id: u8,
    contract_name: &str,
    method: &str,
) -> StacksChainhookSpecification {
    let contract_identifier = format!("{}.{}", accounts::deployer_stx_address(), contract_name);
    let chainhook = stacks_chainhook_contract_call(id, &contract_identifier, method);
    let _ = observer_commands_tx.send(ObserverCommand::RegisterHook(
        ChainhookSpecification::Stacks(chainhook.clone()),
        ApiKey(None),
    ));
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::HookRegistered(registered_chainhook)) => {
            assert_eq!(
                ChainhookSpecification::Stacks(chainhook.clone()),
                registered_chainhook
            );
            true
        }
        _ => false,
    });
    chainhook
}

fn generate_and_register_new_bitcoin_chainhook(
    observer_commands_tx: &Sender<ObserverCommand>,
    observer_events_rx: &Receiver<ObserverEvent>,
    id: u8,
    p2pkh_address: &str,
    expire_after_occurrence: Option<u64>,
) -> BitcoinChainhookSpecification {
    let chainhook = bitcoin_chainhook_p2pkh(id, &p2pkh_address, expire_after_occurrence);
    let _ = observer_commands_tx.send(ObserverCommand::RegisterHook(
        ChainhookSpecification::Bitcoin(chainhook.clone()),
        ApiKey(None),
    ));
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::HookRegistered(registered_chainhook)) => {
            assert_eq!(
                ChainhookSpecification::Bitcoin(chainhook.clone()),
                registered_chainhook
            );
            true
        }
        _ => false,
    });
    chainhook
}

#[test]
fn test_stacks_chainhook_register_deregister() {
    let (observer_commands_tx, observer_commands_rx) = channel();
    let (observer_events_tx, observer_events_rx) = channel();

    let handle = std::thread::spawn(move || {
        let (config, chainhook_store) = generate_test_config();
        let _ = hiro_system_kit::nestable_block_on(start_observer_commands_handler(
            config,
            Arc::new(RwLock::new(chainhook_store)),
            observer_commands_rx,
            Some(observer_events_tx),
        ));
    });

    // Create and register a new chainhook
    let chainhook = generate_and_register_new_stacks_chainhook(
        &observer_commands_tx,
        &observer_events_rx,
        1,
        "counter",
        "increment",
    );

    // Simulate a block that does not include a trigger
    let transactions = vec![generate_test_tx_stacks_contract_call(
        0,
        &accounts::wallet_1_stx_address(),
        "counter",
        "decrement",
        vec!["u1"],
    )];
    let chain_event = StacksChainEvent::ChainUpdatedWithBlocks(StacksChainUpdatedWithBlocksData {
        new_blocks: vec![StacksBlockUpdate::new(
            stacks_blocks::generate_test_stacks_block(0, 1, transactions, None).expect_block(),
        )],
        confirmed_blocks: vec![],
    });
    let _ = observer_commands_tx.send(ObserverCommand::PropagateStacksChainEvent(chain_event));
    // Should signal that no hook were triggered
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::HooksTriggered(len)) => {
            assert_eq!(len, 0);
            true
        }
        _ => false,
    });
    // Should propagate block
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::StacksChainEvent(_)) => {
            true
        }
        _ => false,
    });

    // Simulate a block that does include a trigger
    let transactions = vec![generate_test_tx_stacks_contract_call(
        1,
        &accounts::wallet_1_stx_address(),
        "counter",
        "increment",
        vec!["u1"],
    )];
    let chain_event = StacksChainEvent::ChainUpdatedWithBlocks(StacksChainUpdatedWithBlocksData {
        new_blocks: vec![StacksBlockUpdate::new(
            stacks_blocks::generate_test_stacks_block(0, 2, transactions, None).expect_block(),
        )],
        confirmed_blocks: vec![],
    });
    let _ = observer_commands_tx.send(ObserverCommand::PropagateStacksChainEvent(chain_event));
    // Should signal that no hook were triggered
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::HooksTriggered(len)) => {
            assert_eq!(len, 1);
            true
        }
        _ => false,
    });

    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::StacksChainhookTriggered(payload)) => {
            assert_eq!(payload.apply.len(), 1);
            assert_eq!(payload.apply[0].transactions.len(), 1);
            true
        }
        _ => false,
    });

    // Should propagate block
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::StacksChainEvent(_)) => {
            true
        }
        _ => false,
    });

    // Simulate a block that does include 2 trigger
    let transactions = vec![
        generate_test_tx_stacks_contract_call(
            1,
            &accounts::wallet_1_stx_address(),
            "counter",
            "increment",
            vec!["u1"],
        ),
        generate_test_tx_stacks_contract_call(
            2,
            &accounts::wallet_2_stx_address(),
            "counter",
            "increment",
            vec!["u2"],
        ),
        generate_test_tx_stacks_contract_call(
            3,
            &accounts::wallet_3_stx_address(),
            "counter",
            "decrement",
            vec!["u2"],
        ),
    ];
    let chain_event = StacksChainEvent::ChainUpdatedWithBlocks(StacksChainUpdatedWithBlocksData {
        new_blocks: vec![StacksBlockUpdate::new(
            stacks_blocks::generate_test_stacks_block(0, 2, transactions, None).expect_block(),
        )],
        confirmed_blocks: vec![],
    });
    let _ = observer_commands_tx.send(ObserverCommand::PropagateStacksChainEvent(chain_event));
    // Should signal that no hook were triggered
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::HooksTriggered(len)) => {
            assert_eq!(len, 1);
            true
        }
        _ => false,
    });

    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::StacksChainhookTriggered(payload)) => {
            assert_eq!(payload.apply.len(), 1);
            assert_eq!(payload.apply[0].transactions.len(), 2);
            true
        }
        _ => false,
    });

    // Should propagate block
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::StacksChainEvent(_)) => {
            true
        }
        _ => false,
    });

    // Deregister the hook
    let _ = observer_commands_tx.send(ObserverCommand::DeregisterStacksHook(
        chainhook.uuid.clone(),
        ApiKey(None),
    ));
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::HookDeregistered(deregistered_chainhook)) => {
            assert_eq!(
                ChainhookSpecification::Stacks(chainhook),
                deregistered_chainhook
            );
            true
        }
        _ => false,
    });

    // Simulate a block that does not include a trigger
    let transactions = vec![generate_test_tx_stacks_contract_call(
        2,
        &accounts::wallet_1_stx_address(),
        "counter",
        "decrement",
        vec!["u1"],
    )];
    let chain_event = StacksChainEvent::ChainUpdatedWithBlocks(StacksChainUpdatedWithBlocksData {
        new_blocks: vec![StacksBlockUpdate::new(
            stacks_blocks::generate_test_stacks_block(0, 2, transactions, None).expect_block(),
        )],
        confirmed_blocks: vec![],
    });
    let _ = observer_commands_tx.send(ObserverCommand::PropagateStacksChainEvent(chain_event));
    // Should signal that no hook were triggered
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::HooksTriggered(len)) => {
            assert_eq!(len, 0);
            true
        }
        _ => false,
    });
    // Should propagate block
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::StacksChainEvent(_)) => {
            true
        }
        _ => false,
    });

    // Simulate a block that does include a trigger
    let transactions = vec![generate_test_tx_stacks_contract_call(
        3,
        &accounts::wallet_1_stx_address(),
        "counter",
        "increment",
        vec!["u1"],
    )];
    let chain_event = StacksChainEvent::ChainUpdatedWithBlocks(StacksChainUpdatedWithBlocksData {
        new_blocks: vec![StacksBlockUpdate::new(
            stacks_blocks::generate_test_stacks_block(0, 3, transactions, None).expect_block(),
        )],
        confirmed_blocks: vec![],
    });
    let _ = observer_commands_tx.send(ObserverCommand::PropagateStacksChainEvent(chain_event));
    // Should signal that no hook were triggered
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::HooksTriggered(len)) => {
            assert_eq!(len, 0);
            true
        }
        _ => false,
    });
    // Should propagate block
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::StacksChainEvent(_)) => {
            true
        }
        _ => false,
    });

    let _ = observer_commands_tx.send(ObserverCommand::Terminate);
    handle.join().expect("unable to terminate thread");
}

#[test]
fn test_stacks_chainhook_auto_deregister() {
    let (observer_commands_tx, observer_commands_rx) = channel();
    let (observer_events_tx, observer_events_rx) = channel();

    let handle = std::thread::spawn(move || {
        let (config, chainhook_store) = generate_test_config();
        let _ = hiro_system_kit::nestable_block_on(start_observer_commands_handler(
            config,
            Arc::new(RwLock::new(chainhook_store)),
            observer_commands_rx,
            Some(observer_events_tx),
        ));
    });

    // Create and register a new chainhook
    let contract_identifier = format!("{}.{}", accounts::deployer_stx_address(), "counter");
    let mut chainhook = stacks_chainhook_contract_call(0, &contract_identifier, "increment");
    chainhook.expire_after_occurrence = Some(1);

    let _ = observer_commands_tx.send(ObserverCommand::RegisterHook(
        ChainhookSpecification::Stacks(chainhook.clone()),
        ApiKey(None),
    ));
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::HookRegistered(registered_chainhook)) => {
            assert_eq!(
                ChainhookSpecification::Stacks(chainhook.clone()),
                registered_chainhook
            );
            true
        }
        _ => false,
    });

    // Simulate a block that does not include a trigger
    let transactions = vec![generate_test_tx_stacks_contract_call(
        0,
        &accounts::wallet_1_stx_address(),
        "counter",
        "decrement",
        vec!["u1"],
    )];
    let chain_event = StacksChainEvent::ChainUpdatedWithBlocks(StacksChainUpdatedWithBlocksData {
        new_blocks: vec![StacksBlockUpdate::new(
            stacks_blocks::generate_test_stacks_block(0, 1, transactions, None).expect_block(),
        )],
        confirmed_blocks: vec![],
    });
    let _ = observer_commands_tx.send(ObserverCommand::PropagateStacksChainEvent(chain_event));
    // Should signal that no hook were triggered
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::HooksTriggered(len)) => {
            assert_eq!(len, 0);
            true
        }
        _ => false,
    });
    // Should propagate block
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::StacksChainEvent(_)) => {
            true
        }
        _ => false,
    });

    // Simulate a block that does include a trigger
    let transactions = vec![generate_test_tx_stacks_contract_call(
        1,
        &accounts::wallet_1_stx_address(),
        "counter",
        "increment",
        vec!["u1"],
    )];
    let chain_event = StacksChainEvent::ChainUpdatedWithBlocks(StacksChainUpdatedWithBlocksData {
        new_blocks: vec![StacksBlockUpdate::new(
            stacks_blocks::generate_test_stacks_block(0, 2, transactions, None).expect_block(),
        )],
        confirmed_blocks: vec![],
    });
    let _ = observer_commands_tx.send(ObserverCommand::PropagateStacksChainEvent(chain_event));
    // Should signal that no hook were triggered
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::HooksTriggered(len)) => {
            assert_eq!(len, 1);
            true
        }
        _ => false,
    });

    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::StacksChainhookTriggered(_)) => {
            true
        }
        _ => false,
    });

    // Should propagate block
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::StacksChainEvent(_)) => {
            true
        }
        _ => false,
    });

    // Simulate another block that does include a trigger
    let transactions = vec![generate_test_tx_stacks_contract_call(
        3,
        &accounts::wallet_1_stx_address(),
        "counter",
        "increment",
        vec!["u1"],
    )];
    let chain_event = StacksChainEvent::ChainUpdatedWithBlocks(StacksChainUpdatedWithBlocksData {
        new_blocks: vec![StacksBlockUpdate::new(
            stacks_blocks::generate_test_stacks_block(0, 3, transactions, None).expect_block(),
        )],
        confirmed_blocks: vec![],
    });
    let _ = observer_commands_tx.send(ObserverCommand::PropagateStacksChainEvent(chain_event));
    // Should signal that no hook were triggered
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::HooksTriggered(len)) => {
            assert_eq!(len, 0);
            true
        }
        _ => false,
    });
    // Should signal that a hook was deregistered
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::HookDeregistered(deregistered_hook)) => {
            assert_eq!(deregistered_hook.uuid(), chainhook.uuid);
            true
        }
        _ => false,
    });

    // Should propagate block
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::StacksChainEvent(_)) => {
            true
        }
        Ok(event) => {
            println!("Unexpected event: {:?}", event);
            false
        }
        Err(e) => {
            println!("Error: {:?}", e);
            false
        }
    });

    let _ = observer_commands_tx.send(ObserverCommand::Terminate);
    handle.join().expect("unable to terminate thread");
}

#[test]
fn test_bitcoin_chainhook_register_deregister() {
    let (observer_commands_tx, observer_commands_rx) = channel();
    let (observer_events_tx, observer_events_rx) = channel();

    let handle = std::thread::spawn(move || {
        let (config, chainhook_store) = generate_test_config();
        let _ = hiro_system_kit::nestable_block_on(start_observer_commands_handler(
            config,
            Arc::new(RwLock::new(chainhook_store)),
            observer_commands_rx,
            Some(observer_events_tx),
        ));
    });

    // Create and register a new chainhook (wallet_2 received some sats)
    let chainhook = generate_and_register_new_bitcoin_chainhook(
        &observer_commands_tx,
        &observer_events_rx,
        1,
        &accounts::wallet_2_btc_address(),
        None,
    );

    // Simulate a block that does not include a trigger (wallet_1 to wallet_3)
    let transactions = vec![generate_test_tx_bitcoin_p2pkh_transfer(
        0,
        &accounts::wallet_1_btc_address(),
        &accounts::wallet_3_btc_address(),
        3,
    )];
    let chain_event =
        BitcoinChainEvent::ChainUpdatedWithBlocks(BitcoinChainUpdatedWithBlocksData {
            new_blocks: vec![bitcoin_blocks::generate_test_bitcoin_block(
                0,
                1,
                transactions,
                None,
            )],
            confirmed_blocks: vec![],
        });
    let _ = observer_commands_tx.send(ObserverCommand::PropagateBitcoinChainEvent(chain_event));
    // Should signal that no hook were triggered
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::HooksTriggered(len)) => {
            assert_eq!(len, 0);
            true
        }
        Ok(event) => {
            println!("Unexpected event: {:?}", event);
            false
        }
        Err(e) => {
            println!("Error: {:?}", e);
            false
        }
    });

    // Should propagate block
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::BitcoinChainEvent(_)) => {
            true
        }
        _ => false,
    });

    // Simulate a block that does include a trigger (wallet_1 to wallet_2)
    let transactions = vec![generate_test_tx_bitcoin_p2pkh_transfer(
        0,
        &accounts::wallet_1_btc_address(),
        &accounts::wallet_2_btc_address(),
        3,
    )];
    let chain_event =
        BitcoinChainEvent::ChainUpdatedWithBlocks(BitcoinChainUpdatedWithBlocksData {
            new_blocks: vec![bitcoin_blocks::generate_test_bitcoin_block(
                0,
                2,
                transactions,
                None,
            )],
            confirmed_blocks: vec![],
        });
    let _ = observer_commands_tx.send(ObserverCommand::PropagateBitcoinChainEvent(chain_event));
    // Should signal that no hook were triggered
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::HooksTriggered(len)) => {
            assert_eq!(len, 1);
            true
        }
        _ => false,
    });

    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::BitcoinChainhookTriggered(payload)) => {
            assert_eq!(payload.apply.len(), 1);
            assert_eq!(payload.apply[0].block.transactions.len(), 1);
            true
        }
        _ => false,
    });

    // Should propagate block
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::BitcoinChainEvent(_)) => {
            true
        }
        _ => false,
    });

    // Simulate a block that does include a trigger (wallet_1 to wallet_2)
    let transactions = vec![
        generate_test_tx_bitcoin_p2pkh_transfer(
            0,
            &accounts::wallet_1_btc_address(),
            &accounts::wallet_2_btc_address(),
            3,
        ),
        generate_test_tx_bitcoin_p2pkh_transfer(
            1,
            &accounts::wallet_3_btc_address(),
            &accounts::wallet_2_btc_address(),
            5,
        ),
        generate_test_tx_bitcoin_p2pkh_transfer(
            1,
            &accounts::wallet_3_btc_address(),
            &accounts::wallet_1_btc_address(),
            5,
        ),
    ];
    let chain_event =
        BitcoinChainEvent::ChainUpdatedWithBlocks(BitcoinChainUpdatedWithBlocksData {
            new_blocks: vec![bitcoin_blocks::generate_test_bitcoin_block(
                0,
                2,
                transactions,
                None,
            )],
            confirmed_blocks: vec![],
        });
    let _ = observer_commands_tx.send(ObserverCommand::PropagateBitcoinChainEvent(chain_event));
    // Should signal that no hook were triggered
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::HooksTriggered(len)) => {
            assert_eq!(len, 1);
            true
        }
        _ => false,
    });

    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::BitcoinChainhookTriggered(payload)) => {
            assert_eq!(payload.apply.len(), 1);
            assert_eq!(payload.apply[0].block.transactions.len(), 2);
            true
        }
        _ => false,
    });

    // Should propagate block
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::BitcoinChainEvent(_)) => {
            true
        }
        _ => false,
    });

    // Deregister the hook
    let _ = observer_commands_tx.send(ObserverCommand::DeregisterBitcoinHook(
        chainhook.uuid.clone(),
        ApiKey(None),
    ));
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::HookDeregistered(deregistered_chainhook)) => {
            assert_eq!(
                ChainhookSpecification::Bitcoin(chainhook),
                deregistered_chainhook
            );
            true
        }
        _ => false,
    });

    // Simulate a block that does not include a trigger
    let transactions = vec![generate_test_tx_bitcoin_p2pkh_transfer(
        2,
        &accounts::wallet_1_btc_address(),
        &accounts::wallet_3_btc_address(),
        1,
    )];
    let chain_event =
        BitcoinChainEvent::ChainUpdatedWithBlocks(BitcoinChainUpdatedWithBlocksData {
            new_blocks: vec![bitcoin_blocks::generate_test_bitcoin_block(
                0,
                2,
                transactions,
                None,
            )],
            confirmed_blocks: vec![],
        });
    let _ = observer_commands_tx.send(ObserverCommand::PropagateBitcoinChainEvent(chain_event));
    // Should signal that no hook were triggered
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::HooksTriggered(len)) => {
            assert_eq!(len, 0);
            true
        }
        _ => false,
    });
    // Should propagate block
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::BitcoinChainEvent(_)) => {
            true
        }
        _ => false,
    });

    // Simulate a block that does include a trigger
    let transactions = vec![generate_test_tx_bitcoin_p2pkh_transfer(
        3,
        &accounts::wallet_1_btc_address(),
        &accounts::wallet_2_btc_address(),
        1,
    )];
    let chain_event =
        BitcoinChainEvent::ChainUpdatedWithBlocks(BitcoinChainUpdatedWithBlocksData {
            new_blocks: vec![bitcoin_blocks::generate_test_bitcoin_block(
                0,
                3,
                transactions,
                None,
            )],
            confirmed_blocks: vec![],
        });
    let _ = observer_commands_tx.send(ObserverCommand::PropagateBitcoinChainEvent(chain_event));
    // Should signal that no hook were triggered
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::HooksTriggered(len)) => {
            assert_eq!(len, 0);
            true
        }
        _ => false,
    });
    // Should propagate block
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::BitcoinChainEvent(_)) => {
            true
        }
        _ => false,
    });

    let _ = observer_commands_tx.send(ObserverCommand::Terminate);
    handle.join().expect("unable to terminate thread");
}

#[test]
fn test_bitcoin_chainhook_auto_deregister() {
    let (observer_commands_tx, observer_commands_rx) = channel();
    let (observer_events_tx, observer_events_rx) = channel();

    let handle = std::thread::spawn(move || {
        let (config, chainhook_store) = generate_test_config();
        let _ = hiro_system_kit::nestable_block_on(start_observer_commands_handler(
            config,
            Arc::new(RwLock::new(chainhook_store)),
            observer_commands_rx,
            Some(observer_events_tx),
        ));
    });

    // Create and register a new chainhook (wallet_2 received some sats)
    let chainhook = generate_and_register_new_bitcoin_chainhook(
        &observer_commands_tx,
        &observer_events_rx,
        1,
        &accounts::wallet_2_btc_address(),
        Some(1),
    );

    // Simulate a block that does not include a trigger (wallet_1 to wallet_3)
    let transactions = vec![generate_test_tx_bitcoin_p2pkh_transfer(
        0,
        &accounts::wallet_1_btc_address(),
        &accounts::wallet_3_btc_address(),
        3,
    )];
    let chain_event =
        BitcoinChainEvent::ChainUpdatedWithBlocks(BitcoinChainUpdatedWithBlocksData {
            new_blocks: vec![bitcoin_blocks::generate_test_bitcoin_block(
                0,
                1,
                transactions,
                None,
            )],
            confirmed_blocks: vec![],
        });
    let _ = observer_commands_tx.send(ObserverCommand::PropagateBitcoinChainEvent(chain_event));
    // Should signal that no hook were triggered
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::HooksTriggered(len)) => {
            assert_eq!(len, 0);
            true
        }
        _ => false,
    });
    // Should propagate block
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::BitcoinChainEvent(_)) => {
            true
        }
        _ => false,
    });

    // Simulate a block that does include a trigger (wallet_1 to wallet_2)
    let transactions = vec![generate_test_tx_bitcoin_p2pkh_transfer(
        0,
        &accounts::wallet_1_btc_address(),
        &accounts::wallet_2_btc_address(),
        3,
    )];
    let chain_event =
        BitcoinChainEvent::ChainUpdatedWithBlocks(BitcoinChainUpdatedWithBlocksData {
            new_blocks: vec![bitcoin_blocks::generate_test_bitcoin_block(
                0,
                2,
                transactions,
                None,
            )],
            confirmed_blocks: vec![],
        });
    let _ = observer_commands_tx.send(ObserverCommand::PropagateBitcoinChainEvent(chain_event));
    // Should signal that no hook were triggered
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::HooksTriggered(len)) => {
            assert_eq!(len, 1);
            true
        }
        _ => false,
    });

    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::BitcoinChainhookTriggered(_)) => {
            true
        }
        _ => false,
    });

    // Should propagate block
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::BitcoinChainEvent(_)) => {
            true
        }
        _ => false,
    });

    // Simulate a block that does not include a trigger
    let transactions = vec![generate_test_tx_bitcoin_p2pkh_transfer(
        2,
        &accounts::wallet_1_btc_address(),
        &accounts::wallet_3_btc_address(),
        1,
    )];
    let chain_event =
        BitcoinChainEvent::ChainUpdatedWithBlocks(BitcoinChainUpdatedWithBlocksData {
            new_blocks: vec![bitcoin_blocks::generate_test_bitcoin_block(
                0,
                2,
                transactions,
                None,
            )],
            confirmed_blocks: vec![],
        });
    let _ = observer_commands_tx.send(ObserverCommand::PropagateBitcoinChainEvent(chain_event));
    // Should signal that no hook were triggered
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::HooksTriggered(len)) => {
            assert_eq!(len, 0);
            true
        }
        _ => false,
    });
    // Should propagate block
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::BitcoinChainEvent(_)) => {
            true
        }
        _ => false,
    });

    // Simulate a block that does include a trigger
    let transactions = vec![generate_test_tx_bitcoin_p2pkh_transfer(
        3,
        &accounts::wallet_1_btc_address(),
        &accounts::wallet_2_btc_address(),
        1,
    )];
    let chain_event =
        BitcoinChainEvent::ChainUpdatedWithBlocks(BitcoinChainUpdatedWithBlocksData {
            new_blocks: vec![bitcoin_blocks::generate_test_bitcoin_block(
                0,
                3,
                transactions,
                None,
            )],
            confirmed_blocks: vec![],
        });
    let _ = observer_commands_tx.send(ObserverCommand::PropagateBitcoinChainEvent(chain_event));
    // Should signal that no hook were triggered
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::HooksTriggered(len)) => {
            assert_eq!(len, 0);
            true
        }
        _ => false,
    });
    // Should signal that a hook was deregistered
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::HookDeregistered(deregistered_hook)) => {
            assert_eq!(deregistered_hook.uuid(), chainhook.uuid);
            true
        }
        _ => false,
    });

    // Should propagate block
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::BitcoinChainEvent(_)) => {
            true
        }
        _ => false,
    });

    let _ = observer_commands_tx.send(ObserverCommand::Terminate);
    handle.join().expect("unable to terminate thread");
}
