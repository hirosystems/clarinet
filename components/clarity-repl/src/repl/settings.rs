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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ApiUrl(String);

impl Default for ApiUrl {
    fn default() -> Self {
        ApiUrl("http://api.hiro.so".to_string())
    }
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct Settings {
    pub analysis: analysis::Settings,
    pub network_simulation: NetworkSimulationSettings,
    #[serde(skip_serializing, skip_deserializing)]
    pub clarity_wasm_mode: bool,
    #[serde(skip_serializing, skip_deserializing)]
    pub show_timings: bool,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct SettingsFile {
    analysis: Option<analysis::SettingsFile>,
    network_simulation: Option<NetworkSimulationSettingsFile>,
}

impl From<SettingsFile> for Settings {
    fn from(file: SettingsFile) -> Self {
        let analysis = file
            .analysis
            .map(analysis::Settings::from)
            .unwrap_or_default();

        let network_simulation = file
            .network_simulation
            .map(NetworkSimulationSettings::from)
            .unwrap_or_default();

        Self {
            analysis,
            network_simulation,
            clarity_wasm_mode: false,
            show_timings: false,
        }
    }
}

// #[derive(Debug, Default, Clone, Deserialize, Serialize)]
// pub struct SimnetSettingsFile {
//     network_simulation: NetworkSimulationSettingsFile,
// }

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct NetworkSimulationSettingsFile {
    enabled: Option<bool>,
    api_url: Option<ApiUrl>,
}

// #[derive(Debug, Default, Clone, Deserialize, Serialize)]
// pub struct SimnetSettings {
//     pub network_simulation: NetworkSimulationSettings,
// }

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct NetworkSimulationSettings {
    pub enabled: bool,
    pub api_url: ApiUrl,
}

impl From<NetworkSimulationSettingsFile> for NetworkSimulationSettings {
    fn from(file: NetworkSimulationSettingsFile) -> Self {
        Self {
            enabled: file.enabled.unwrap_or_default(),
            api_url: file.api_url.unwrap_or_default(),
        }
    }
}
