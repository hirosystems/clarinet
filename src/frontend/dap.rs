use crate::deployment::{generate_default_deployment, setup_session_with_deployment};
use clarinet_lib::types::ProjectManifest;
use clarity_repl::clarity::debug::dap::DAPDebugger;
use std::path::PathBuf;

#[cfg(feature = "telemetry")]
use super::telemetry::{telemetry_report_event, DeveloperUsageDigest, DeveloperUsageEvent};

pub fn run_dap() -> Result<(), String> {
    let mut dap = DAPDebugger::new();
    match dap.init() {
        Ok((manifest_path_str, expression)) => {
            let manifest_path = PathBuf::from(manifest_path_str);
            let (deployment, _) = generate_default_deployment(&manifest_path, &None)?;
            let (mut session, _) = setup_session_with_deployment(&manifest_path, &deployment, None);
            let project_manifest = ProjectManifest::from_path(&manifest_path);

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
                dap.path_to_contract_id
                    .insert(relative_path.clone(), contract_id.clone());
                dap.contract_id_to_path
                    .insert(contract_id.clone(), relative_path.clone());
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
