pub mod changes;
mod contract;
mod notebook;
mod project;

pub use changes::{Changes, DirectoryCreation, FileCreation, TOMLEdition};
use contract::GetChangesForNewContract;
use notebook::GetChangesForNewNotebook;
use project::GetChangesForNewProject;
use std::collections::HashMap;
use crate::types::RequirementConfig;

pub fn get_changes_for_new_project(project_path: String, project_name: String) -> Vec<Changes> {
    let mut command = GetChangesForNewProject::new(project_path, project_name);
    command.run()
}

pub fn get_changes_for_new_contract(project_path: String, contract_name: String, source: Option<String>, include_test: bool, deps: Vec<String>) -> Vec<Changes> {
    let mut command = GetChangesForNewContract::new(project_path, contract_name, source);
    command.run(include_test, deps)
}

pub fn get_changes_for_new_link(project_path: String, contract_id: String, _source: Option<String>) -> Vec<Changes> {
    let change = TOMLEdition {
        comment: format!("Adding {} as a requirement in Clarinet.toml", contract_id),
        path: project_path,
        contracts_to_add: HashMap::new(),
        requirements_to_add: vec![RequirementConfig {
            contract_id: contract_id.clone(),
        }],
    };
    vec![Changes::EditTOML(change)]
}

#[allow(dead_code)]
pub fn get_changes_for_new_notebook(project_path: String, notebook_name: String) -> Vec<Changes> {
    let command = GetChangesForNewNotebook::new(project_path, notebook_name);
    command.run()
}
