mod chainhook;
pub mod changes;
mod contract;
mod project;

use chainhook::GetChangesForNewChainhook;
pub use changes::Changes;
use clarinet_files::chainhook_types::Chain;
use clarinet_files::FileLocation;
use contract::GetChangesForNewContract;
use project::GetChangesForNewProject;

use clarinet_files::ProjectManifest;

use self::contract::GetChangesForRmContract;

pub fn get_changes_for_new_project(
    project_path: String,
    project_name: String,
    telemetry_enabled: bool,
) -> Result<Vec<Changes>, String> {
    let mut command = GetChangesForNewProject::new(project_path, project_name, telemetry_enabled);
    command.run()
}

pub fn get_changes_for_new_contract(
    manifest_location: &FileLocation,
    contract_name: String,
    source: Option<String>,
    include_test: bool,
) -> Result<Vec<Changes>, String> {
    let mut command =
        GetChangesForNewContract::new(manifest_location.clone(), contract_name, source);
    command.run(include_test)
}

pub fn get_changes_for_rm_contract(
    manifest_location: &FileLocation,
    contract_name: String,
) -> Result<Vec<Changes>, String> {
    let mut command = GetChangesForRmContract::new(manifest_location.clone(), contract_name);
    command.run()
}

pub fn get_changes_for_new_chainhook(
    manifest: &ProjectManifest,
    chainhook_name: String,
    chain: Chain,
) -> Result<Vec<Changes>, String> {
    let mut command = GetChangesForNewChainhook::new(manifest, chainhook_name, chain);
    command.run()
}
