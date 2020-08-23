use super::changes::{Changes, DirectoryCreation, FileCreation, TOMLEdition};

pub struct GetChangesForNewContract {
    project_path: String, 
    contract_name: String,
    changes: Vec<Changes>
}

impl GetChangesForNewContract {

    pub fn new(project_path: String, contract_name: String) -> Self {
        Self {
            project_path,
            contract_name,
            changes: vec![]
        }
    }

    pub fn run(&mut self) -> Vec<Changes> {
        self.create_template_contract();
        self.index_contract_in_paper_toml();
        self.changes.clone()
    }

    fn create_template_contract(&mut self) {
        let content = format!(r#"
;; {}
;; <Add a description here>

;; Constants
;;

;; Data maps and vars
;;

;; Private functions
;;

;; Public functions
;;

"#, self.contract_name); // todo(ludo): Capitalize contract_name
        let name = format!("{}.clar", self.contract_name);
        let path = format!("{}/contracts/{}", self.project_path, name);
        let change = FileCreation {
            comment: format!("Creating file contracts/{}", name),
            name,
            content,
            path,
        };
        self.changes.push(Changes::AddFile(change));
    }

    fn index_contract_in_paper_toml(&mut self) {
        let contract_file_name = format!("{}.clar", self.contract_name);
        let contract_file_path = format!("{}/contracts/{}", self.project_path, contract_file_name);
        let path = format!("{}/Paper.toml", self.project_path);

        let change = TOMLEdition {
            comment: format!("Indexing contract {} in ./Paper.toml", self.contract_name),
            path,
            section: "contracts".to_string(),
            content: format!("{} = {{ path = \"{}\" }}", self.contract_name, contract_file_path),
            index: -1, // Append to the end
        };
        self.changes.push(Changes::EditTOML(change));
    }

}