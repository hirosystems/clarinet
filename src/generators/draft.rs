use super::changes::{Changes, DirectoryCreation, FileCreation, TOMLEdition};

pub struct GetChangesForNewDraft {
    project_path: String, 
    draft_name: String,
    changes: Vec<Changes>
}

impl GetChangesForNewDraft {

    pub fn new(project_path: String, draft_name: String) -> Self {
        Self {
            project_path,
            draft_name,
            changes: vec![],
        }
    }

    pub fn run(&self) -> Vec<Changes> {
        self.changes.clone()
    }
}