mod chainhook;
pub mod changes;
mod contract;
mod project;

use chainhook::GetChangesForNewChainhook;
pub use changes::{Changes, DirectoryCreation, FileCreation, TOMLEdition};
use clarinet_files::FileLocation;
use contract::GetChangesForNewContract;
use orchestra_types::Chain;
use project::GetChangesForNewProject;

use clarinet_files::ProjectManifest;

pub fn get_changes_for_new_project(
    project_path: String,
    project_name: String,
    telemetry_enabled: bool,
) -> Vec<Changes> {
    let mut command = GetChangesForNewProject::new(project_path, project_name, telemetry_enabled);
    command.run()
}

pub fn get_changes_for_new_contract(
    manifest_location: &FileLocation,
    contract_name: String,
    source: Option<String>,
    include_test: bool,
) -> Vec<Changes> {
    let mut command =
        GetChangesForNewContract::new(manifest_location.clone(), contract_name, source);
    command.run(include_test)
}

pub fn get_changes_for_new_chainhook(
    manifest: &ProjectManifest,
    chainhook_name: String,
    chain: Chain,
) -> Vec<Changes> {
    let mut command = GetChangesForNewChainhook::new(manifest, chainhook_name, chain);
    command.run()
}
