use crate::poke::load_session;
use crate::types::Network;
use clarity_repl::clarity::debug::dap::DAPDebugger;
use std::path::PathBuf;

pub fn run_dap() -> Result<(), String> {
    let mut dap = DAPDebugger::new();
    match dap.init() {
        Ok((manifest, expression)) => {
            let manifest_path = PathBuf::from(manifest);
            let mut session = match load_session(&manifest_path, false, &Network::Devnet) {
                Ok((session, _, _, _)) => session,
                Err((_, e)) => {
                    println!("{}: unable to load session: {}", red!("error"), e);
                    std::process::exit(1);
                }
            };

            for contract in &session.settings.initial_contracts {
                dap.path_to_contract_id.insert(
                    contract.path.clone(),
                    contract.get_contract_identifier(false).unwrap(),
                );
                dap.contract_id_to_path.insert(
                    contract.get_contract_identifier(false).unwrap(),
                    contract.path.clone(),
                );
            }

            // Begin execution of the expression in debug mode
            match session.interpret(
                expression.clone(),
                None,
                Some(vec![Box::new(dap)]),
                false,
                None,
            ) {
                Ok(result) => Ok(()),
                Err(diagnostics) => Err("unable to interpret expression".to_string()),
            }
        }
        Err(e) => Err(format!("dap_init: {}", e)),
    }
}
