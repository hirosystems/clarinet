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

use clarity::types::chainstate::StacksAddress;

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub struct CheckCheckerSettings {
    // Strict mode sets all other options to false
    pub strict: bool,
    // After a filter on tx-sender, trust all inputs
    pub trusted_sender: bool,
    // After a filter on contract-caller, trust all inputs
    pub trusted_caller: bool,
    // Allow filters in callee to filter caller
    pub callee_filter: bool,
}

#[derive(Debug, Default, Clone, Copy, Serialize, Deserialize)]
pub struct CheckCheckerSettingsFile {
    // Strict mode sets all other options to false
    strict: Option<bool>,
    // After a filter on tx-sender, trust all inputs
    trusted_sender: Option<bool>,
    // After a filter on contract-caller, trust all inputs
    trusted_caller: Option<bool>,
    // Allow filters in callee to filter caller
    callee_filter: Option<bool>,
}

impl From<CheckCheckerSettingsFile> for CheckCheckerSettings {
    fn from(from_file: CheckCheckerSettingsFile) -> Self {
        if from_file.strict.unwrap_or(false) {
            CheckCheckerSettings {
                strict: true,
                trusted_sender: false,
                trusted_caller: false,
                callee_filter: false,
            }
        } else {
            CheckCheckerSettings {
                strict: false,
                trusted_sender: from_file.trusted_sender.unwrap_or(false),
                trusted_caller: from_file.trusted_caller.unwrap_or(false),
                callee_filter: from_file.callee_filter.unwrap_or(false),
            }
        }
    }
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pass {
    All,
    CheckChecker,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum OneOrList<T> {
    /// Allow `T` as shorthand for `[T]` in the TOML
    One(T),
    /// Allow more than one `T` in the TOML
    List(Vec<T>),
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct AnalysisSettingsFile {
    passes: Option<OneOrList<Pass>>,
    check_checker: Option<CheckCheckerSettingsFile>,
}

// Each new pass should be included in this list
static ALL_PASSES: [Pass; 1] = [Pass::CheckChecker];

impl From<AnalysisSettingsFile> for AnalysisSettings {
    fn from(from_file: AnalysisSettingsFile) -> Self {
        let passes = if let Some(file_passes) = from_file.passes {
            match file_passes {
                OneOrList::One(pass) => match pass {
                    Pass::All => ALL_PASSES.to_vec(),
                    pass => vec![pass],
                },
                OneOrList::List(passes) => {
                    if passes.contains(&Pass::All) {
                        ALL_PASSES.to_vec()
                    } else {
                        passes
                    }
                }
            }
        } else {
            vec![]
        };

        // Each pass that has its own settings should be included here.
        let checker_settings = if let Some(checker_settings) = from_file.check_checker {
            CheckCheckerSettings::from(checker_settings)
        } else {
            CheckCheckerSettings::default()
        };

        Self {
            passes,
            check_checker: checker_settings,
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct AnalysisSettings {
    pub passes: Vec<Pass>,
    pub check_checker: CheckCheckerSettings,
}

impl AnalysisSettings {
    pub fn enable_all_passes(&mut self) {
        self.passes = ALL_PASSES.to_vec();
    }

    pub fn set_passes(&mut self, passes: Vec<Pass>) {
        for pass in passes {
            match pass {
                Pass::All => {
                    self.passes = ALL_PASSES.to_vec();
                    return;
                }
                pass => self.passes.push(pass),
            };
        }
    }
}

#[derive(Clone, Debug)]
pub struct InitialContract {
    pub code: String,
    pub name: Option<String>,
    pub path: String,
    pub deployer: Option<String>,
}

impl InitialContract {
    pub fn get_contract_identifier(&self, is_mainnet: bool) -> Option<QualifiedContractIdentifier> {
        self.name.as_ref().map(|name| QualifiedContractIdentifier {
            issuer: self.get_deployer_principal(is_mainnet),
            name: name.to_string().try_into().unwrap(),
        })
    }

    pub fn get_deployer_principal(&self, is_mainnet: bool) -> StandardPrincipalData {
        let address = match self.deployer {
            Some(ref entry) => entry.clone(),
            None => format!("{}", StacksAddress::burn_address(is_mainnet)),
        };
        PrincipalData::parse_standard_principal(&address)
            .expect("Unable to parse deployer's address")
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Account {
    pub address: String,
    pub balance: u64,
    pub name: String,
}

#[derive(Clone, Debug, Default)]
pub struct SessionSettings {
    pub node: String,
    pub include_boot_contracts: Vec<String>,
    pub include_costs: bool,
    pub initial_contracts: Vec<InitialContract>,
    pub initial_accounts: Vec<Account>,
    pub initial_deployer: Option<Account>,
    pub scoping_contract: Option<String>,
    pub lazy_initial_contracts_interpretation: bool,
    pub disk_cache_enabled: bool,
    pub repl_settings: Settings,
    pub epoch_id: Option<StacksEpochId>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct Settings {
    pub analysis: AnalysisSettings,
    #[serde(skip_serializing, skip_deserializing)]
    pub clarity_wasm_mode: bool,
    #[serde(skip_serializing, skip_deserializing)]
    pub show_timings: bool,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct SettingsFile {
    pub analysis: Option<AnalysisSettingsFile>,
}

impl From<SettingsFile> for Settings {
    fn from(file: SettingsFile) -> Self {
        let analysis = if let Some(analysis) = file.analysis {
            AnalysisSettings::from(analysis)
        } else {
            AnalysisSettings::default()
        };
        Self {
            analysis,
            clarity_wasm_mode: false,
            show_timings: false,
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct ReplSettings {
    pub analysis: AnalysisSettings,
    #[serde(skip_serializing, skip_deserializing)]
    pub clarity_wasm_mode: bool,
    #[serde(skip_serializing, skip_deserializing)]
    pub show_timings: bool,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct ReplSettingsFile {
    pub analysis: Option<AnalysisSettingsFile>,
}

impl From<ReplSettingsFile> for ReplSettings {
    fn from(file: ReplSettingsFile) -> Self {
        let analysis = if let Some(analysis) = file.analysis {
            AnalysisSettings::from(analysis)
        } else {
            AnalysisSettings::default()
        };
        Self {
            analysis,
            clarity_wasm_mode: false,
            show_timings: false,
        }
    }
}
