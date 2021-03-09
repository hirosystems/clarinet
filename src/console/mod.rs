use std::fs;
use std::env;
use crate::types::{MainConfig, ChainConfig};
use clarity_repl::{repl, Terminal};


pub fn run_console() {
    let mut settings = repl::SessionSettings::default();

    let root_path = env::current_dir().unwrap();
    let mut project_config_path = root_path.clone();
    project_config_path.push("Clarinet.toml");

    let mut chain_config_path = root_path.clone();
    chain_config_path.push("settings");
    chain_config_path.push("Local.toml");

    let project_config = MainConfig::from_path(&project_config_path);
    let chain_config = ChainConfig::from_path(&chain_config_path);

    for (name, config) in project_config.contracts.iter() {
        let mut contract_path = root_path.clone();
        contract_path.push(&config.path);

        let code = fs::read_to_string(&contract_path).unwrap();

        settings
            .initial_contracts
            .push(repl::settings::InitialContract {
                code: code,
                name: Some(name.clone()),
                deployer: Some("ST1D0XTBR7WVNSYBJ7M26XSJAXMDJGJQKNEXAM6JH".to_string()),
            });
    }

    for (name, account) in chain_config.accounts.iter() {
        settings
            .initial_accounts
            .push(repl::settings::Account {
                name: name.clone(),
                balance: account.balance,
                address: account.address.clone(),
                mnemonic: account.mnemonic.clone(),
                derivation: account.derivation.clone(),
            });
    }

    let mut session = Terminal::new(settings);
    let res = session.start();
}
