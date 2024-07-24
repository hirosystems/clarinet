use std::convert::TryInto;

use clarinet_core::ReplSettings;
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
    pub repl_settings: ReplSettings,
    pub epoch_id: Option<StacksEpochId>,
}
