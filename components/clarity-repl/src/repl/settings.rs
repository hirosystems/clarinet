use std::convert::TryInto;

use crate::analysis;
use clarity::types::chainstate::StacksAddress;
use clarity::types::StacksEpochId;
use clarity::vm::types::{PrincipalData, QualifiedContractIdentifier, StandardPrincipalData};

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
    pub analysis: analysis::Settings,
    #[serde(skip_serializing, skip_deserializing)]
    pub clarity_wasm_mode: bool,
    #[serde(skip_serializing, skip_deserializing)]
    pub show_timings: bool,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct SettingsFile {
    pub analysis: Option<analysis::SettingsFile>,
}

impl From<SettingsFile> for Settings {
    fn from(file: SettingsFile) -> Self {
        let analysis = if let Some(analysis) = file.analysis {
            analysis::Settings::from(analysis)
        } else {
            analysis::Settings::default()
        };
        Self {
            analysis,
            clarity_wasm_mode: false,
            show_timings: false,
        }
    }
}
