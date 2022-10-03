use chainhook_types::{
    BitcoinBlockData, BlockIdentifier, StacksBlockData, StacksMicroblockData, StacksTransactionData,
};
use std::future::Future;
use tokio;

pub fn nestable_block_on<F: Future>(future: F) -> F::Output {
    let (handle, _rt) = match tokio::runtime::Handle::try_current() {
        Ok(h) => (h, None),
        Err(_) => {
            let rt = tokio::runtime::Runtime::new().unwrap();
            (rt.handle().clone(), Some(rt))
        }
    };
    let response = handle.block_on(async { future.await });
    response
}

pub trait AbstractStacksBlock {
    fn get_identifier(&self) -> &BlockIdentifier;
    fn get_parent_identifier(&self) -> &BlockIdentifier;
    fn get_transactions(&self) -> &Vec<StacksTransactionData>;
}

impl AbstractStacksBlock for StacksBlockData {
    fn get_identifier(&self) -> &BlockIdentifier {
        &self.block_identifier
    }

    fn get_parent_identifier(&self) -> &BlockIdentifier {
        &self.parent_block_identifier
    }

    fn get_transactions(&self) -> &Vec<StacksTransactionData> {
        &self.transactions
    }
}

impl AbstractStacksBlock for StacksMicroblockData {
    fn get_identifier(&self) -> &BlockIdentifier {
        &self.block_identifier
    }

    fn get_parent_identifier(&self) -> &BlockIdentifier {
        &self.parent_block_identifier
    }

    fn get_transactions(&self) -> &Vec<StacksTransactionData> {
        &self.transactions
    }
}

pub trait AbstractBlock {
    fn get_identifier(&self) -> &BlockIdentifier;
    fn get_parent_identifier(&self) -> &BlockIdentifier;
}

impl AbstractBlock for StacksBlockData {
    fn get_identifier(&self) -> &BlockIdentifier {
        &self.block_identifier
    }

    fn get_parent_identifier(&self) -> &BlockIdentifier {
        &self.parent_block_identifier
    }
}

impl AbstractBlock for StacksMicroblockData {
    fn get_identifier(&self) -> &BlockIdentifier {
        &self.block_identifier
    }

    fn get_parent_identifier(&self) -> &BlockIdentifier {
        &self.parent_block_identifier
    }
}

impl AbstractBlock for BitcoinBlockData {
    fn get_identifier(&self) -> &BlockIdentifier {
        &self.block_identifier
    }

    fn get_parent_identifier(&self) -> &BlockIdentifier {
        &self.parent_block_identifier
    }
}
