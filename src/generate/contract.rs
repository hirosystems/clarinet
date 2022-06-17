use super::changes::{Changes, FileCreation, TOMLEdition};
use clarinet_files::{ContractConfig, FileLocation};
use std::{collections::HashMap, path::PathBuf};

pub struct GetChangesForNewContract {
    manifest_location: FileLocation,
    contract_name: String,
    source: Option<String>,
    changes: Vec<Changes>,
}

impl GetChangesForNewContract {
    pub fn new(
        manifest_location: FileLocation,
        contract_name: String,
        source: Option<String>,
    ) -> Self {
        Self {
            manifest_location,
            contract_name,
            source,
            changes: vec![],
        }
    }

    pub fn run(&mut self, include_test: bool) -> Vec<Changes> {
        self.create_template_contract();
        if include_test {
            self.create_template_test();
        }
        self.index_contract_in_clarinet_toml();
        self.changes.clone()
    }

    fn create_template_contract(&mut self) {
        let content = if let Some(ref source) = self.source {
            source.to_string()
        } else {
            format!(
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
            )
        };
        let name = format!("{}.clar", self.contract_name);
        let mut contract_path = self.manifest_location.get_project_root_location().unwrap();
        contract_path.append_relative_path("contracts");
        contract_path.append_relative_path(&name);
        let change = FileCreation {
            comment: format!("{} contracts/{}", green!("Created file"), name),
            name,
            content,
            path: contract_path.to_string(),
        };
        self.changes.push(Changes::AddFile(change));
    }

    fn create_template_test(&mut self) {
        let content = format!(
            r#"
import {{ Clarinet, Tx, Chain, Account, types }} from 'https://deno.land/x/clarinet@v0.31.0/index.ts';
import {{ assertEquals }} from 'https://deno.land/std@0.90.0/testing/asserts.ts';

Clarinet.test({{
    name: "Ensure that <...>",
    async fn(chain: Chain, accounts: Map<string, Account>) {{
        let block = chain.mineBlock([
            /* 
             * Add transactions with: 
             * Tx.contractCall(...)
            */
        ]);
        assertEquals(block.receipts.length, 0);
        assertEquals(block.height, 2);

        block = chain.mineBlock([
            /* 
             * Add transactions with: 
             * Tx.contractCall(...)
            */
        ]);
        assertEquals(block.receipts.length, 0);
        assertEquals(block.height, 3);
    }},
}});
"#
        );

        let name = format!("{}_test.ts", self.contract_name);
        let mut contract_path = self.manifest_location.get_project_root_location().unwrap();
        contract_path.append_relative_path("tests");
        contract_path.append_relative_path(&name);
        let change = FileCreation {
            comment: format!("{} tests/{}", green!("Created file"), name),
            name,
            content,
            path: contract_path.to_string(),
        };
        self.changes.push(Changes::AddFile(change));
    }

    fn index_contract_in_clarinet_toml(&mut self) {
        let contract_file_name = format!("{}.clar", self.contract_name);
        let manifest_location = self.manifest_location.clone();

        let contract_config = ContractConfig {
            path: format!("contracts/{}", contract_file_name),
            deployer: None,
        };
        let mut contracts_to_add = HashMap::new();
        contracts_to_add.insert(self.contract_name.clone(), contract_config);

        let change = TOMLEdition {
            comment: format!(
                "{} with contract {}",
                yellow!("Updated Clarinet.toml"),
                self.contract_name
            ),
            manifest_location,
            contracts_to_add,
            requirements_to_add: vec![],
        };
        self.changes.push(Changes::EditTOML(change));
    }
}
