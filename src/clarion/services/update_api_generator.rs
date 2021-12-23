
use crate::indexer::{BitcoinChainEvent, StacksChainEvent};
use crate::clarion::

pub fn stacks_chain_event_handler(datastore: Datastore, contract_id: QualifiedContractIdentifier, chain_event: StacksChainEvent) {


    match chain_event {
        ChainUpdatedWithBlock(StacksBlockData) => {

        }
        ChainUpdatedWithReorg(Vec<StacksBlockData>) => {
            
        }
    } 
}