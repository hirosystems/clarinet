use crate::deployments::generate_default_deployment;
use clarinet_deployments::setup_session_with_deployment;
use clarinet_files::chainhook_types::StacksNetwork;
use clarinet_files::{FileLocation, ProjectManifest};
use clarity_repl::repl::debug::dap::DAPDebugger;
use std::path::PathBuf;

#[cfg(feature = "telemetry")]
use super::telemetry::{telemetry_report_event, DeveloperUsageDigest, DeveloperUsageEvent};

pub fn run_dap() -> Result<(), String> {
    let mut dap = DAPDebugger::new();
    match dap.init() {
        Ok((manifest_location_str, expression)) => {
            let manifest_location = FileLocation::from_path_string(&manifest_location_str)?;
            let project_manifest = ProjectManifest::from_location(&manifest_location)?;
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

            for (contract_id, (_, location)) in deployment.contracts.iter() {
                dap.path_to_contract_id
                    .insert(PathBuf::from(location.to_string()), contract_id.clone());
                dap.contract_id_to_path
                    .insert(contract_id.clone(), PathBuf::from(location.to_string()));
            }

            // Begin execution of the expression in debug mode
            match session.eval_with_hooks(expression.clone(), Some(vec![&mut dap]), false) {
                Ok(_result) => Ok(()),
                Err(_diagnostics) => Err("unable to interpret expression".to_string()),
            }
        }
        Err(e) => Err(format!("dap_init: {}", e)),
    }
}
