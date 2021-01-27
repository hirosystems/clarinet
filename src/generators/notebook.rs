use super::changes::{Changes, DirectoryCreation, FileCreation, TOMLEdition};

pub struct GetChangesForNewNotebook {
    project_path: String, 
    notebook_name: String,
    changes: Vec<Changes>
}

impl GetChangesForNewNotebook {

    pub fn new(project_path: String, notebook_name: String) -> Self {
        Self {
            project_path,
            notebook_name,
            changes: vec![],
        }
    }

    pub fn run(&self) -> Vec<Changes> {
        self.changes.clone()
    }
}