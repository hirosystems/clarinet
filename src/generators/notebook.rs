use super::changes::{Changes};

#[allow(dead_code)]
pub struct GetChangesForNewNotebook {
    pub project_path: String,
    pub notebook_name: String,
    changes: Vec<Changes>,
}

impl GetChangesForNewNotebook {

    #[allow(dead_code)]
    pub fn new(project_path: String, notebook_name: String) -> Self {
        Self {
            project_path,
            notebook_name,
            changes: vec![],
        }
    }

    #[allow(dead_code)]
    pub fn run(&self) -> Vec<Changes> {
        self.changes.clone()
    }
}
