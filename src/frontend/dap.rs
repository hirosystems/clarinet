use crate::deployment::{generate_default_deployment, setup_session_with_deployment};
use crate::types::ProjectManifest;
use clarity_repl::clarity::debug::dap::DAPDebugger;
use orchestra_types::StacksNetwork;
use std::path::PathBuf;
use std::str::FromStr;

#[cfg(feature = "telemetry")]
use super::telemetry::{telemetry_report_event, DeveloperUsageDigest, DeveloperUsageEvent};

pub fn run_dap() -> Result<(), String> {
    let mut dap = DAPDebugger::new();
    match dap.init() {
        Ok((manifest_path_str, expression)) => {
            let manifest_path = PathBuf::from(manifest_path_str);
            let project_manifest = ProjectManifest::from_path(&manifest_path)?;
            let (deployment, artifacts) =
                generate_default_deployment(&project_manifest, &StacksNetwork::Simnet, false)?;
            let mut session = setup_session_with_deployment(
                &project_manifest,
                &deployment,
                Some(&artifacts.asts),
            )
            .session;

            if project_manifest.project.telemetry {
                #[cfg(feature = "telemetry")]
                telemetry_report_event(DeveloperUsageEvent::DAPDebugStarted(
                    DeveloperUsageDigest::new(
                        &project_manifest.project.name,
                        &project_manifest.project.authors,
                    ),
                ));
            }

            for (contract_id, (_, relative_path)) in deployment.contracts.iter() {
                let mut absolute_path = project_manifest.get_project_root_dir();
                absolute_path.extend(&PathBuf::from_str(relative_path).unwrap());
                dap.path_to_contract_id
                    .insert(absolute_path.clone(), contract_id.clone());
                dap.contract_id_to_path
                    .insert(contract_id.clone(), absolute_path);
            }

            // Begin execution of the expression in debug mode
            match session.interpret(
                expression.clone(),
                None,
                Some(vec![Box::new(dap)]),
                false,
                None,
                None,
            ) {
                Ok(_result) => Ok(()),
                Err(_diagnostics) => Err("unable to interpret expression".to_string()),
            }
        }
        Err(e) => Err(format!("dap_init: {}", e)),
    }
}
