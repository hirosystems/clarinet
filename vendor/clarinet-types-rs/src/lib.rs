extern crate serde;

#[macro_use]
extern crate serde_derive;

mod chain_config;
mod project_manifest;

pub use chain_config::{
    compute_addresses, AccountConfig, ChainConfig, ChainConfigFile, DevnetConfig, DevnetConfigFile,
    PoxStackingOrder, DEFAULT_DERIVATION_PATH,
};
pub use project_manifest::{
    ContractConfig, ProjectManifest, ProjectManifestFile, RequirementConfig,
};

pub const DEFAULT_DEVNET_BALANCE: u64 = 100_000_000_000_000;
