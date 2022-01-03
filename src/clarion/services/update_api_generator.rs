use crate::clarion::Datastore;
use crate::indexer::{BitcoinChainEvent, StacksChainEvent};
use clarity_repl::clarity::types::QualifiedContractIdentifier;

pub fn stacks_chain_event_handler(
    datastore: &dyn Datastore,
    contract_id: QualifiedContractIdentifier,
    chain_event: StacksChainEvent,
) {
    match chain_event {
        StacksChainEvent::ChainUpdatedWithBlock(block) => {}
        StacksChainEvent::ChainUpdatedWithReorg(old_segment, new_segment) => {}
    }
}
