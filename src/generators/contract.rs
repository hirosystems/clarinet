use super::changes::{Changes, FileCreation, TOMLEdition};
use std::collections::HashMap;
use crate::types::ContractConfig;

pub struct GetChangesForNewContract {
    project_path: String,
    contract_name: String,
    changes: Vec<Changes>,
}

impl GetChangesForNewContract {
    pub fn new(project_path: String, contract_name: String) -> Self {
        Self {
            project_path,
            contract_name,
            changes: vec![],
        }
    }

    pub fn run(&mut self) -> Vec<Changes> {
        self.create_template_contract();
        self.create_template_test();
        self.index_contract_in_clarinet_toml();
        self.changes.clone()
    }

    fn create_template_contract(&mut self) {
        let content = format!(
            r#"
;; {}
;; <add a description here>

;; constants
;;

;; data maps and vars
;;

;; private functions
;;

;; public functions
;;

"#,
            self.contract_name
        );
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

    fn create_template_test(&mut self) {
        let content = format!(
            r#"
import {{ Clarinet, Block, Chain, Account }} from 'https://deno.land/x/clarinet@v0.1.2/index.ts';

Clarinet.test({{
    name: "Ensure that test 1 are being executed",
    async fn(chain: Chain, accounts: Array<Account>) {{
        console.log(`Test initialized with Chain::${{chain.sessionId}} and accounts ${{JSON.stringify(accounts)}}`);
    }},
}});
"#
        );
        let name = format!("{}_test.ts", self.contract_name);
        let path = format!("{}/tests/{}", self.project_path, name);
        let change = FileCreation {
            comment: format!("Creating file tests/{}", name),
            name,
            content,
            path,
        };
        self.changes.push(Changes::AddFile(change));
    }

    fn index_contract_in_clarinet_toml(&mut self) {
        let contract_file_name = format!("{}.clar", self.contract_name);
        let contract_file_path = format!("{}/contracts/{}", self.project_path, contract_file_name);
        let path = format!("{}/Clarinet.toml", self.project_path);

        let contract_config = ContractConfig {
            version: 1,
            path: format!("contracts/{}", contract_file_name),
        };
        let mut contracts_to_add = HashMap::new();
        contracts_to_add.insert(self.contract_name.clone(), contract_config);

        let change = TOMLEdition {
            comment: format!("Adding contract {} to Clarinet.toml", self.contract_name),
            path,
            contracts_to_add,
            notebooks_to_add: HashMap::new(),
        };
        self.changes.push(Changes::EditTOML(change));
    }
}
