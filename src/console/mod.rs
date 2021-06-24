use std::fs;
use std::path::PathBuf;
use crate::types::{MainConfig, ChainConfig};
use clarity_repl::{repl, Terminal};


pub fn load_session(manifest_path: PathBuf, start_repl: bool, env: String) -> Result<repl::SessionSettings, String> {
    let mut settings = repl::SessionSettings::default();

    let mut project_path = manifest_path.clone();
    project_path.pop();

    let mut chain_config_path = project_path.clone();
    // chain_config_path.pop();
    chain_config_path.push("settings");

    chain_config_path.push(if env == "mocknet" {
        "Mocknet.toml"
    } else if env == "testnet" {
        "Testnet.toml"
    } else {
        "Development.toml"
    });

    let mut project_config = MainConfig::from_path(&manifest_path);
    let chain_config = ChainConfig::from_path(&chain_config_path);

    let mut deployer_address = None;
    let mut initial_deployer = None;

    for (name, account) in chain_config.accounts.iter() {
        let account = repl::settings::Account {
            name: name.clone(),
            balance: account.balance,
            address: account.address.clone(),
            mnemonic: account.mnemonic.clone(),
            derivation: account.derivation.clone(),
        };
        if name == "deployer" {
            initial_deployer = Some(account.clone());
            deployer_address = Some(account.address.clone());
        }
        settings
            .initial_accounts
            .push(account);
    }

    for (name, config) in project_config.ordered_contracts().iter() {
        let mut contract_path = project_path.clone();
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
                path: contract_path.to_str().unwrap().into(),
                name: Some(name.clone()),
                deployer: deployer_address.clone(),
            });
    }

    let links = match project_config.project.requirements.take() {
        Some(links) => links,
        None => vec![],
    };

    for link_config in links.iter() {
        settings
            .initial_links
            .push(repl::settings::InitialLink {
                contract_id: link_config.contract_id.clone(),
                stacks_node_addr: None,
                cache: None,
        });
    }

    settings.include_boot_contracts = vec!["pox".to_string(), "costs".to_string(), "bns".to_string()];
    settings.initial_deployer = initial_deployer;

    if start_repl {
        let mut terminal = Terminal::new(settings.clone());
        terminal.start();
    } else {
        let mut session = repl::Session::new(settings.clone());
        match session.check() {
            Err(message) => {
                println!("Error: {}", message);
                std::process::exit(1);
            }
            _ => {}
        };
    }
    Ok(settings)
}
