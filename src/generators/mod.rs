pub mod changes;
mod contract;
mod draft;
mod project;

pub use changes::{Changes, DirectoryCreation, FileCreation, TOMLEdition};
use project::GetChangesForNewProject;
use contract::GetChangesForNewContract;
use draft::GetChangesForNewDraft;

pub fn get_changes_for_new_project(project_path: String, project_name: String) -> Vec<Changes> {
    let mut command = GetChangesForNewProject::new(project_path, project_name);
    command.run()
}

pub fn get_changes_for_new_contract(project_path: String, contract_name: String) -> Vec<Changes> {
    let mut command = GetChangesForNewContract::new(project_path, contract_name);
    command.run()
}

pub fn get_changes_for_new_draft(project_path: String, draft_name: String) -> Vec<Changes> {
    let mut command = GetChangesForNewDraft::new(project_path, draft_name);
    command.run()
}
