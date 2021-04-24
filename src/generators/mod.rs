pub mod changes;
mod contract;
mod notebook;
mod project;
mod project_fork;

pub use changes::{Changes, DirectoryCreation, FileCreation, TOMLEdition};
use contract::GetChangesForNewContract;
use notebook::GetChangesForNewNotebook;
use project::GetChangesForNewProject;
use project_fork::GetChangesForForkedProject;

pub fn get_changes_for_new_project(project_path: String, project_name: String) -> Vec<Changes> {
    let mut command = GetChangesForNewProject::new(project_path, project_name);
    command.run()
}

pub fn get_changes_for_new_contract(project_path: String, contract_name: String) -> Vec<Changes> {
    let mut command = GetChangesForNewContract::new(project_path, contract_name);
    command.run()
}

pub fn get_changes_for_new_notebook(project_path: String, notebook_name: String) -> Vec<Changes> {
    let command = GetChangesForNewNotebook::new(project_path, notebook_name);
    command.run()
}

pub fn get_changes_for_forked_contract(project_path: String, project_name: String, contract_id: String) -> Vec<Changes> {
    let mut command = GetChangesForForkedProject::new(project_path, project_name, contract_id);
    command.run()
}