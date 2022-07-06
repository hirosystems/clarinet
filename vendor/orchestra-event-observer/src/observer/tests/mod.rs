use crate::chainhooks::types::{
    ChainhookSpecification, HookAction, HookFormation, StacksChainhookSpecification,
    StacksContractCallBasedPredicate, StacksHookPredicate,
};
use crate::indexer::tests::helpers::{
    accounts, blocks, transactions::generate_test_tx_contract_call,
};
use crate::observer::{
    self, start_observer_commands_handler, ApiKey, EventHandler, EventObserverConfig,
    ObserverCommand,
};
use crate::utils;
use clarity_repl::clarity::types::QualifiedContractIdentifier;
use orchestra_types::{
    ChainUpdatedWithBlocksData, StacksBlockData, StacksBlockUpdate, StacksChainEvent, StacksNetwork,
};
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};

use super::ObserverEvent;

fn generate_test_config() -> EventObserverConfig {
    let operators = HashMap::new();
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
        bitcoin_node_rpc_host: "http://localhost".into(),
        bitcoin_node_rpc_port: 0,
        stacks_node_rpc_host: "http://localhost".into(),
        stacks_node_rpc_port: 0,
        operators,
    };
    config
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
        predicate: StacksHookPredicate::ContractCall(StacksContractCallBasedPredicate {
            contract_identifier: contract_identifier.to_string(),
            method: method.to_string(),
        }),
        action: HookAction::Noop,
    };
    spec
}

fn generate_and_register_new_chainhook(
    observer_commands_tx: &Sender<ObserverCommand>,
    observer_events_rx: &Receiver<ObserverEvent>,
    id: u8,
    contract_name: &str,
    method: &str,
) -> StacksChainhookSpecification {
    let contract_identifier = format!("{}.{}", accounts::deployer(), contract_name);
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

#[test]
fn test_chainhook_register_deregister() {
    let (observer_commands_tx, observer_commands_rx) = channel();
    let (observer_events_tx, observer_events_rx) = channel();

    let handle = std::thread::spawn(move || {
        let mut config = generate_test_config();
        let _ = crate::utils::nestable_block_on(start_observer_commands_handler(
            &mut config,
            observer_commands_rx,
            Some(observer_events_tx),
        ));
    });

    // Create and register a new chainhook
    let chainhook = generate_and_register_new_chainhook(
        &observer_commands_tx,
        &observer_events_rx,
        1,
        "counter",
        "increment",
    );

    // Simulate a block that does not include a trigger
    let transactions = vec![generate_test_tx_contract_call(
        0,
        &accounts::wallet_1(),
        "counter",
        "decrement",
        vec!["u1"],
    )];
    let chain_event = StacksChainEvent::ChainUpdatedWithBlocks(ChainUpdatedWithBlocksData {
        new_blocks: vec![StacksBlockUpdate::new(
            blocks::generate_test_block(0, 1, transactions, None).expect_block(),
        )],
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
    // Should propage block
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::StacksChainEvent(_)) => {
            true
        }
        _ => false,
    });

    // Simulate a block that does include a trigger
    let transactions = vec![generate_test_tx_contract_call(
        1,
        &accounts::wallet_1(),
        "counter",
        "increment",
        vec!["u1"],
    )];
    let chain_event = StacksChainEvent::ChainUpdatedWithBlocks(ChainUpdatedWithBlocksData {
        new_blocks: vec![StacksBlockUpdate::new(
            blocks::generate_test_block(0, 2, transactions, None).expect_block(),
        )],
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
    // Should propage block
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
    let transactions = vec![generate_test_tx_contract_call(
        2,
        &accounts::wallet_1(),
        "counter",
        "decrement",
        vec!["u1"],
    )];
    let chain_event = StacksChainEvent::ChainUpdatedWithBlocks(ChainUpdatedWithBlocksData {
        new_blocks: vec![StacksBlockUpdate::new(
            blocks::generate_test_block(0, 2, transactions, None).expect_block(),
        )],
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
    // Should propage block
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::StacksChainEvent(_)) => {
            true
        }
        _ => false,
    });

    // Simulate a block that does include a trigger
    let transactions = vec![generate_test_tx_contract_call(
        3,
        &accounts::wallet_1(),
        "counter",
        "increment",
        vec!["u1"],
    )];
    let chain_event = StacksChainEvent::ChainUpdatedWithBlocks(ChainUpdatedWithBlocksData {
        new_blocks: vec![StacksBlockUpdate::new(
            blocks::generate_test_block(0, 3, transactions, None).expect_block(),
        )],
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
    // Should propage block
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
fn test_chainhook_auto_deregister() {
    let (observer_commands_tx, observer_commands_rx) = channel();
    let (observer_events_tx, observer_events_rx) = channel();

    let handle = std::thread::spawn(move || {
        let mut config = generate_test_config();
        let _ = crate::utils::nestable_block_on(start_observer_commands_handler(
            &mut config,
            observer_commands_rx,
            Some(observer_events_tx),
        ));
    });

    // Create and register a new chainhook
    let contract_identifier = format!("{}.{}", accounts::deployer(), "counter");
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
    let transactions = vec![generate_test_tx_contract_call(
        0,
        &accounts::wallet_1(),
        "counter",
        "decrement",
        vec!["u1"],
    )];
    let chain_event = StacksChainEvent::ChainUpdatedWithBlocks(ChainUpdatedWithBlocksData {
        new_blocks: vec![StacksBlockUpdate::new(
            blocks::generate_test_block(0, 1, transactions, None).expect_block(),
        )],
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
    // Should propage block
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::StacksChainEvent(_)) => {
            true
        }
        _ => false,
    });

    // Simulate a block that does include a trigger
    let transactions = vec![generate_test_tx_contract_call(
        1,
        &accounts::wallet_1(),
        "counter",
        "increment",
        vec!["u1"],
    )];
    let chain_event = StacksChainEvent::ChainUpdatedWithBlocks(ChainUpdatedWithBlocksData {
        new_blocks: vec![StacksBlockUpdate::new(
            blocks::generate_test_block(0, 2, transactions, None).expect_block(),
        )],
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
    // Should propage block
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::StacksChainEvent(_)) => {
            true
        }
        _ => false,
    });

    // Simulate another block that does include a trigger
    let transactions = vec![generate_test_tx_contract_call(
        3,
        &accounts::wallet_1(),
        "counter",
        "increment",
        vec!["u1"],
    )];
    let chain_event = StacksChainEvent::ChainUpdatedWithBlocks(ChainUpdatedWithBlocksData {
        new_blocks: vec![StacksBlockUpdate::new(
            blocks::generate_test_block(0, 3, transactions, None).expect_block(),
        )],
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
    // Should propage block
    assert!(match observer_events_rx.recv() {
        Ok(ObserverEvent::StacksChainEvent(_)) => {
            true
        }
        _ => false,
    });

    let _ = observer_commands_tx.send(ObserverCommand::Terminate);
    handle.join().expect("unable to terminate thread");
}
