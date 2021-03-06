use super::changes::{Changes, DirectoryCreation, FileCreation};

pub struct GetChangesForNewProject {
    project_path: String,
    project_name: String,
    changes: Vec<Changes>,
}

impl GetChangesForNewProject {
    pub fn new(project_path: String, project_name: String) -> Self {
        Self {
            project_path,
            project_name,
            changes: vec![],
        }
    }

    pub fn run(&mut self) -> Vec<Changes> {
        self.create_root_directory();
        self.create_clients_directory();
        self.create_contracts_directory();
        self.create_notebooks_directory();
        self.create_scripts_directory();
        self.create_environments_directory();
        self.create_tests_directory();

        self.create_clarinette_toml();
        self.create_environment_mainnet_toml();
        self.create_environment_testnet_toml();
        self.create_environment_local_toml();
        self.changes.clone()
    }

    fn create_root_directory(&mut self) {
        let dir = format!("{}/{}", self.project_path, self.project_name);
        let change = DirectoryCreation {
            comment: format!("Creating directory {}", self.project_name),
            name: self.project_name.clone(),
            path: dir,
        };
        self.changes.push(Changes::AddDirectory(change));
    }

    fn create_clients_directory(&mut self) {
        self.changes
            .push(self.get_changes_for_new_root_dir(format!("clients")));
    }

    fn create_contracts_directory(&mut self) {
        self.changes
            .push(self.get_changes_for_new_root_dir(format!("contracts")));
    }

    fn create_notebooks_directory(&mut self) {
        self.changes
            .push(self.get_changes_for_new_root_dir(format!("notebooks")));
    }

    fn create_scripts_directory(&mut self) {
        self.changes
            .push(self.get_changes_for_new_root_dir(format!("scripts")));
    }

    fn create_environments_directory(&mut self) {
        self.changes
            .push(self.get_changes_for_new_root_dir(format!("environments")));
    }

    fn create_tests_directory(&mut self) {
        self.changes
            .push(self.get_changes_for_new_root_dir(format!("tests")));
    }

    fn create_clarinette_toml(&mut self) {
        let content = format!(
            r#"
[project]
name = "{}"

[contracts]

[notebooks]
"#,
            self.project_name
        );
        let name = format!("Clarinette.toml");
        let path = format!("{}/{}/{}", self.project_path, self.project_name, name);
        let change = FileCreation {
            comment: format!("Creating file {}/{}", self.project_name, name),
            name,
            content,
            path,
        };
        self.changes.push(Changes::AddFile(change));
    }

    fn create_environment_mainnet_toml(&mut self) {
        let content = format!(
            r#"[network]
name = "mainnet"
node_rpc_address = "http://stacks-node-api.blockstack.org:20443"
"#
        );
        let name = format!("Mainnet.toml");
        let path = format!(
            "{}/{}/environments/{}",
            self.project_path, self.project_name, name
        );
        let change = FileCreation {
            comment: format!("Creating file {}/environments/{}", self.project_name, name),
            name,
            content,
            path,
        };
        self.changes.push(Changes::AddFile(change));
    }

    fn create_environment_testnet_toml(&mut self) {
        let content = format!(
            r#"[network]
name = "testnet"
node_rpc_address = "http://xenon.blockstack.org:20443"
"#
        );
        let name = format!("Testnet.toml");
        let path = format!(
            "{}/{}/environments/{}",
            self.project_path, self.project_name, name
        );
        let change = FileCreation {
            comment: format!("Creating file {}/environments/{}", self.project_name, name),
            name,
            content,
            path,
        };
        self.changes.push(Changes::AddFile(change));
    }

    fn create_environment_local_toml(&mut self) {
        let content = format!(
            r#"[network]
name = "local"
node_rpc_address = "http://127.0.0.1:20443"
"#
        );
        let name = format!("Local.toml");
        let path = format!(
            "{}/{}/environments/{}",
            self.project_path, self.project_name, name
        );
        let change = FileCreation {
            comment: format!("Creating file {}/environments/{}", self.project_name, name),
            name,
            content,
            path,
        };
        self.changes.push(Changes::AddFile(change));
    }

    fn get_changes_for_new_root_dir(&self, name: String) -> Changes {
        let dir = format!("{}/{}", self.project_name, name);
        let change = DirectoryCreation {
            comment: format!("Creating directory {}/{}", self.project_name, name),
            name,
            path: dir,
        };
        Changes::AddDirectory(change)
    }
}
