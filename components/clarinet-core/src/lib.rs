use std::convert::TryInto;
use std::fmt::Display;
use std::path::PathBuf;

use clarity::types::StacksEpochId;
use clarity::vm::types::{PrincipalData, QualifiedContractIdentifier, StandardPrincipalData};
use clarity::vm::ClarityVersion;
use hmac::Hmac;
use pbkdf2::pbkdf2;
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};
use sha2::Sha512;

pub const DEFAULT_CLARITY_VERSION: ClarityVersion = ClarityVersion::Clarity2;
pub const DEFAULT_EPOCH: StacksEpochId = StacksEpochId::Epoch25;

#[derive(Deserialize, Debug, Clone)]
pub struct ClarityContract {
    pub code_source: ClarityCodeSource,
    pub name: String,
    pub deployer: ContractDeployer,
    pub clarity_version: ClarityVersion,
    pub epoch: StacksEpochId,
}

impl Serialize for ClarityContract {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        match self.code_source {
            ClarityCodeSource::ContractOnDisk(ref path) => {
                map.serialize_entry("path", &format!("{}", path.display()))?;
            }
            _ => unreachable!(),
        }
        match self.deployer {
            ContractDeployer::LabeledDeployer(ref label) => {
                map.serialize_entry("deployer", &label)?;
            }
            ContractDeployer::DefaultDeployer => {}
            _ => unreachable!(),
        }
        match self.clarity_version {
            ClarityVersion::Clarity1 => {
                map.serialize_entry("clarity_version", &1)?;
            }
            ClarityVersion::Clarity2 => {
                map.serialize_entry("clarity_version", &2)?;
            }
            ClarityVersion::Clarity3 => {
                map.serialize_entry("clarity_version", &3)?;
            }
        }
        match self.epoch {
            StacksEpochId::Epoch10 => {
                map.serialize_entry("epoch", &1.0)?;
            }
            StacksEpochId::Epoch20 => {
                map.serialize_entry("epoch", &2.0)?;
            }
            StacksEpochId::Epoch2_05 => {
                map.serialize_entry("epoch", &2.05)?;
            }
            StacksEpochId::Epoch21 => {
                map.serialize_entry("epoch", &2.1)?;
            }
            StacksEpochId::Epoch22 => {
                map.serialize_entry("epoch", &2.2)?;
            }
            StacksEpochId::Epoch23 => {
                map.serialize_entry("epoch", &2.3)?;
            }
            StacksEpochId::Epoch24 => {
                map.serialize_entry("epoch", &2.4)?;
            }
            StacksEpochId::Epoch25 => {
                map.serialize_entry("epoch", &2.5)?;
            }
            StacksEpochId::Epoch30 => {
                map.serialize_entry("epoch", &3.0)?;
            }
        }
        map.end()
    }
}

pub mod test_fixtures;

impl Display for ClarityContract {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "<Contract contract_id={}, clarity_version={}, epoch={}>",
            self.expect_resolved_contract_identifier(None),
            self.clarity_version,
            self.epoch
        )
    }
}

impl ClarityContract {
    pub fn expect_in_memory_code_source(&self) -> &str {
        match self.code_source {
            ClarityCodeSource::ContractInMemory(ref code_source) => code_source.as_str(),
            _ => panic!("source code expected to be in memory"),
        }
    }

    pub fn expect_contract_path_as_str(&self) -> &str {
        match self.code_source {
            ClarityCodeSource::ContractOnDisk(ref path) => path.to_str().unwrap(),
            _ => panic!("source code expected to be in memory"),
        }
    }

    pub fn expect_resolved_contract_identifier(
        &self,
        default_deployer: Option<&StandardPrincipalData>,
    ) -> QualifiedContractIdentifier {
        let deployer = match &self.deployer {
            ContractDeployer::ContractIdentifier(contract_identifier) => {
                return contract_identifier.clone()
            }
            ContractDeployer::Transient => StandardPrincipalData::transient(),
            ContractDeployer::Address(address) => {
                PrincipalData::parse_standard_principal(address).expect("unable to parse address")
            }
            ContractDeployer::DefaultDeployer => default_deployer
                .expect("default provider should have been provided")
                .clone(),
            _ => panic!("deployer expected to be resolved"),
        };
        let contract_name = self
            .name
            .clone()
            .try_into()
            .expect("unable to parse contract name");
        QualifiedContractIdentifier::new(deployer, contract_name)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ContractDeployer {
    Transient,
    DefaultDeployer,
    LabeledDeployer(String),
    Address(String),
    ContractIdentifier(QualifiedContractIdentifier),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClarityCodeSource {
    ContractInMemory(String),
    ContractOnDisk(PathBuf),
    Empty,
}

pub fn get_bip39_seed_from_mnemonic(mnemonic: &str, password: &str) -> Result<Vec<u8>, String> {
    const PBKDF2_ROUNDS: u32 = 2048;
    const PBKDF2_BYTES: usize = 64;
    let salt = format!("mnemonic{}", password);
    let mut seed = vec![0u8; PBKDF2_BYTES];

    pbkdf2::<Hmac<Sha512>>(
        mnemonic.as_bytes(),
        salt.as_bytes(),
        PBKDF2_ROUNDS,
        &mut seed,
    )
    .map_err(|e| e.to_string())?;
    Ok(seed)
}
