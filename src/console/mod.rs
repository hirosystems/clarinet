use std::fs;
use std::env;
use crate::types::{MainConfig, ChainConfig};
use clarity_repl::{repl, Terminal};


pub fn load_session(start_repl: bool) -> Result<(), String> {
    let mut settings = repl::SessionSettings::default();

    let root_path = env::current_dir().unwrap();
    let mut project_config_path = root_path.clone();
    project_config_path.push("Clarinet.toml");

    let mut chain_config_path = root_path.clone();
    chain_config_path.push("settings");
    chain_config_path.push("Development.toml");

    let project_config = MainConfig::from_path(&project_config_path);
    let chain_config = ChainConfig::from_path(&chain_config_path);

    let mut deployer = None;
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
        if name == "deployer" {
            deployer = Some(account.address.clone());
        }
    }

    for (name, config) in project_config.ordered_contracts().iter() {
        let mut contract_path = root_path.clone();
        contract_path.push(&config.path);

        let code = match fs::read_to_string(&contract_path) {
            Ok(code) => code,
            Err(err) => {
                return Err(format!("Error: unable to read {:?}: {}", contract_path, err))
            }
        };

        settings
            .initial_contracts
            .push(repl::settings::InitialContract {
                code: code,
                name: Some(name.clone()),
                deployer: deployer.clone(),
            });
    }

    if start_repl {
        let mut terminal = Terminal::new(settings);
        terminal.start();
    } else {
        let mut session = repl::Session::new(settings);
        session.check()?;
    }
    Ok(())
}
