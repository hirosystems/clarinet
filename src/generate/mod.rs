pub mod changes;
mod contract;
mod notebook;
mod project;

pub use changes::{Changes, DirectoryCreation, FileCreation, TOMLEdition};
use contract::GetChangesForNewContract;
use notebook::GetChangesForNewNotebook;
use project::GetChangesForNewProject;
use std::path::PathBuf;

pub fn get_changes_for_new_project(
    project_path: String,
    project_name: String,
    telemetry_enabled: bool,
) -> Vec<Changes> {
    let mut command = GetChangesForNewProject::new(project_path, project_name, telemetry_enabled);
    command.run()
}

pub fn get_changes_for_new_contract(
    manifest_path: &PathBuf,
    contract_name: String,
    source: Option<String>,
    include_test: bool,
) -> Vec<Changes> {
    let mut command = GetChangesForNewContract::new(manifest_path.clone(), contract_name, source);
    command.run(include_test)
}

#[allow(dead_code)]
pub fn get_changes_for_new_notebook(manifest_path: PathBuf, notebook_name: String) -> Vec<Changes> {
    let command = GetChangesForNewNotebook::new(manifest_path, notebook_name);
    command.run()
}
