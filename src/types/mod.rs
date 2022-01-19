mod chain_config;
mod data;
pub mod events;
mod project_manifest;

pub use chain_config::{
    compute_addresses, AccountConfig, ChainConfig, ChainConfigFile, DevnetConfig, DevnetConfigFile,
    PoxStackingOrder, DEFAULT_DERIVATION_PATH,
};
pub use project_manifest::{
    ContractConfig, ProjectManifest, ProjectManifestFile, RequirementConfig,
};

pub use data::{
    AccountIdentifier, Amount, BitcoinBlockData, BitcoinBlockMetadata, BitcoinTransactionData,
    BitcoinTransactionMetadata, BlockIdentifier, Currency, CurrencyMetadata, CurrencyStandard,
    Operation, OperationIdentifier, OperationMetadata, OperationStatusKind, OperationType,
    StacksBlockData, StacksBlockMetadata, StacksTransactionData, StacksTransactionMetadata,
    StacksTransactionReceipt, TransactionIdentifier, StacksTransactionExecutionCost, StacksTransactionKind, StacksContractDeploymentData
};

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum BitcoinChainEvent {
    ChainUpdatedWithBlock(BitcoinBlockData),
    ChainUpdatedWithReorg(Vec<BitcoinBlockData>, Vec<BitcoinBlockData>),
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum StacksChainEvent {
    ChainUpdatedWithBlock(StacksBlockData),
    ChainUpdatedWithReorg(Vec<StacksBlockData>, Vec<StacksBlockData>),
}
