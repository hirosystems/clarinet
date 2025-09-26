use std::collections::BTreeMap;

use super::{BitcoinBlockData, BitcoinTransactionData, StacksBlockData, StacksTransactionData};
use serde_json::Value as JsonValue;

pub struct ProcessedStacksTransaction {
    pub tx: StacksTransactionData,
    pub metadata: BTreeMap<String, JsonValue>,
}

pub struct ProcessedStacksBlock {
    pub tx: StacksBlockData,
    pub metadata: BTreeMap<String, JsonValue>,
}

pub struct ProcessedBitcoinTransaction {
    pub tx: BitcoinTransactionData,
    pub metadata: BTreeMap<String, JsonValue>,
}

pub struct ProcessedBitcoinBlock {
    pub tx: BitcoinBlockData,
    pub metadata: BTreeMap<String, JsonValue>,
}

pub enum ProcessingContext {
    Scanning,
    Streaming,
}

pub trait BitcoinProtocolProcessor {
    fn register(&mut self);
    fn process_block(
        &mut self,
        block: &mut ProcessedBitcoinBlock,
        processing_context: ProcessingContext,
    );
    fn process_transaction(
        &mut self,
        transaction: &mut ProcessedBitcoinTransaction,
        processing_context: ProcessingContext,
    );
}

pub fn run_processor<P>(mut p: P)
where
    P: BitcoinProtocolProcessor,
{
    p.register();
}
