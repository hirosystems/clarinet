use std::convert::TryInto;

use crate::analysis;
use crate::clarity::{
    coverage::CoverageReporter,
    types::{PrincipalData, QualifiedContractIdentifier, StandardPrincipalData},
    util::StacksAddress,
};

const DEFAULT_COSTS_VERSION: u32 = 2;
const DEFAULT_PARSER_VERSION: u32 = 2;

#[derive(Clone, Debug)]
pub struct InitialContract {
    pub code: String,
    pub name: Option<String>,
    pub path: String,
    pub deployer: Option<String>,
}

impl InitialContract {
    pub fn get_contract_identifier(&self, is_mainnet: bool) -> Option<QualifiedContractIdentifier> {
        match self.name {
            Some(ref name) => Some(QualifiedContractIdentifier {
                issuer: self.get_deployer_principal(is_mainnet).into(),
                name: name.to_string().try_into().unwrap(),
            }),
            _ => None,
        }
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
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Settings {
    pub analysis: analysis::Settings,
    pub costs_version: u32,
    pub parser_version: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            analysis: analysis::Settings::default(),
            costs_version: DEFAULT_COSTS_VERSION,
            parser_version: DEFAULT_PARSER_VERSION,
        }
    }
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct SettingsFile {
    pub analysis: Option<analysis::SettingsFile>,
    pub costs_version: Option<u32>,
    pub parser_version: Option<u32>,
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
            costs_version: file.costs_version.unwrap_or(DEFAULT_COSTS_VERSION),
            parser_version: file.parser_version.unwrap_or(DEFAULT_PARSER_VERSION),
        }
    }
}
