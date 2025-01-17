use std::convert::TryInto;
use std::fmt;
use std::str::FromStr;

use clarity::types::chainstate::StacksAddress;
use clarity::types::StacksEpochId;
use clarity::vm::types::{PrincipalData, QualifiedContractIdentifier, StandardPrincipalData};

use crate::analysis;

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
    // pub node: String,
    pub include_boot_contracts: Vec<String>,
    pub include_costs: bool,
    pub initial_accounts: Vec<Account>,
    pub initial_deployer: Option<Account>,
    pub disk_cache_enabled: bool,
    pub repl_settings: Settings,
    pub epoch_id: Option<StacksEpochId>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct ApiUrl(pub String);
impl Default for ApiUrl {
    fn default() -> Self {
        ApiUrl("http://api.hiro.so".to_string())
    }
}

impl FromStr for ApiUrl {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(ApiUrl(s.to_string()))
    }
}

impl fmt::Display for ApiUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct Settings {
    pub analysis: analysis::Settings,
    pub remote_data: RemoteDataSettings,
    #[serde(skip_serializing, skip_deserializing)]
    pub clarity_wasm_mode: bool,
    #[serde(skip_serializing, skip_deserializing)]
    pub show_timings: bool,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct SettingsFile {
    analysis: Option<analysis::SettingsFile>,
    remote_data: Option<RemoteDataSettingsFile>,
}

impl From<SettingsFile> for Settings {
    fn from(file: SettingsFile) -> Self {
        let analysis = file
            .analysis
            .map(analysis::Settings::from)
            .unwrap_or_default();

        let remote_data = file
            .remote_data
            .map(RemoteDataSettings::from)
            .unwrap_or_default();

        Self {
            analysis,
            remote_data,
            clarity_wasm_mode: false,
            show_timings: false,
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct RemoteDataSettingsFile {
    enabled: Option<bool>,
    api_url: Option<ApiUrl>,
    initial_height: Option<u32>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct RemoteDataSettings {
    pub enabled: bool,
    pub api_url: ApiUrl,
    pub initial_height: Option<u32>,
}

impl From<RemoteDataSettingsFile> for RemoteDataSettings {
    fn from(file: RemoteDataSettingsFile) -> Self {
        Self {
            enabled: file.enabled.unwrap_or_default(),
            api_url: file.api_url.unwrap_or_default(),
            initial_height: file.initial_height,
        }
    }
}
