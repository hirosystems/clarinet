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
    StacksBlockData, StacksBlockMetadata, StacksContractDeploymentData, StacksMicroblockData,
    StacksMicroblocksTrail, StacksTransactionData, StacksTransactionExecutionCost,
    StacksTransactionKind, StacksTransactionMetadata, StacksTransactionReceipt,
    TransactionIdentifier,
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
    ChainUpdatedWithBlock(ChainUpdatedWithBlockData),
    ChainUpdatedWithReorg(ChainUpdatedWithReorgData),
    ChainUpdatedWithMicroblock(ChainUpdatedWithMicroblockData),
    ChainUpdatedWithMicroblockReorg(ChainUpdatedWithMicroblockReorgData),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChainUpdatedWithBlockData {
    pub new_block: StacksBlockData,
    pub anchored_trail: Option<StacksMicroblocksTrail>,
    pub confirmed_block: (StacksBlockData, Option<StacksMicroblocksTrail>),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChainUpdatedWithReorgData {
    pub old_blocks: Vec<(Option<StacksMicroblocksTrail>, StacksBlockData)>,
    pub new_blocks: Vec<(Option<StacksMicroblocksTrail>, StacksBlockData)>,
    pub confirmed_block: (StacksBlockData, Option<StacksMicroblocksTrail>),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChainUpdatedWithMicroblockData {
    pub anchored_block: StacksBlockData,
    pub current_trail: StacksMicroblocksTrail,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChainUpdatedWithMicroblockReorgData {
    pub new_block: StacksBlockData,
    pub new_anchored_trail: Option<StacksMicroblocksTrail>,
    pub old_trail: Option<StacksMicroblocksTrail>,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub enum StacksNetwork {
    Devnet,
    Testnet,
    Mainnet,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum BitcoinNetwork {
    Regtest,
    Testnet,
    Mainnet,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum ChainsCoordinatorCommand {
    Terminate(bool), // Restart
    PublishInitialContracts,
    BitcoinOpSent,
    ProtocolDeployed,
    StartAutoMining,
    StopAutoMining,
    MineBitcoinBlock,
    InvalidateBitcoinChainTip,
    PublishPoxStackingOrders(BlockIdentifier),
}
