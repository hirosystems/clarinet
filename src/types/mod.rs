mod chain_config;
mod project_manifest;

pub use chain_config::{
    compute_addresses, AccountConfig, ChainConfig, ChainConfigFile, DevnetConfig, DevnetConfigFile,
    PoxStackingOrder, DEFAULT_DERIVATION_PATH,
};
pub use project_manifest::{
    ContractConfig, ProjectManifest, ProjectManifestFile, RequirementConfig,
};

#[derive(Debug)]
pub enum DeploymentEvent {
    ContractCallBroadcasted,
    ContractPublishBroadcasted,
    ProtocolDeployed,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum ChainsCoordinatorCommand {
    Terminate,
    ProtocolDeployed,
}
