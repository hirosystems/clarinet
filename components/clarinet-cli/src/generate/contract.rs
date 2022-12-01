use super::changes::{Changes, FileCreation, TOMLEdition};
use clarinet_files::FileLocation;
use clarity_repl::repl::{
    ClarityCodeSource, ClarityContract, ContractDeployer, DEFAULT_CLARITY_VERSION, DEFAULT_EPOCH,
};
use std::{collections::HashMap, path::PathBuf, str::FromStr};

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

    pub fn run(&mut self, include_test: bool) -> Result<Vec<Changes>, String> {
        self.create_template_contract()?;
        if include_test {
            self.create_template_test()?;
        }
        self.index_contract_in_clarinet_toml();
        Ok(self.changes.clone())
    }

    fn create_template_contract(&mut self) -> Result<(), String> {
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
        let mut new_file = self.manifest_location.get_project_root_location().unwrap();
        new_file.append_path("contracts")?;
        new_file.append_path(&name)?;
        if new_file.exists() {
            return Err(format!("{} already exists", new_file.to_string()));
        }
        let change = FileCreation {
            comment: format!("{} contracts/{}", green!("Created file"), name),
            name,
            content,
            path: new_file.to_string(),
        };
        self.changes.push(Changes::AddFile(change));
        Ok(())
    }

    fn create_template_test(&mut self) -> Result<(), String> {
        let content = format!(
            r#"
import {{ Clarinet, Tx, Chain, Account, types }} from 'https://deno.land/x/clarinet@v{}/index.ts';
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
"#,
            env!("CARGO_PKG_VERSION")
        );

        let name = format!("{}_test.ts", self.contract_name);
        let mut new_file = self.manifest_location.get_project_root_location().unwrap();
        new_file.append_path("tests")?;
        new_file.append_path(&name)?;
        if new_file.exists() {
            return Err(format!("{} already exists", new_file.to_string()));
        }
        let change = FileCreation {
            comment: format!("{} tests/{}", green!("Created file"), name),
            name,
            content,
            path: new_file.to_string(),
        };
        self.changes.push(Changes::AddFile(change));
        Ok(())
    }

    fn index_contract_in_clarinet_toml(&mut self) {
        let contract_file_name = format!("{}.clar", self.contract_name);
        let manifest_location = self.manifest_location.clone();
        let contract_path = {
            let path = format!("contracts/{}", contract_file_name);
            PathBuf::from_str(&path).unwrap()
        };
        let contract_config = ClarityContract {
            code_source: ClarityCodeSource::ContractOnDisk(contract_path),
            deployer: ContractDeployer::DefaultDeployer,
            name: self.contract_name.clone(),
            clarity_version: DEFAULT_CLARITY_VERSION,
            epoch: DEFAULT_EPOCH,
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
