use super::changes::{Changes, DirectoryCreation, FileCreation};
use chainhook_types::Chain;
use clarinet_files::ProjectManifest;

pub struct GetChangesForNewChainhook<'a> {
    manifest: &'a ProjectManifest,
    chainhook_name: String,
    changes: Vec<Changes>,
    chain: Chain,
}

impl<'a> GetChangesForNewChainhook<'a> {
    pub fn new(manifest: &'a ProjectManifest, chainhook_name: String, chain: Chain) -> Self {
        Self {
            manifest,
            chainhook_name,
            changes: vec![],
            chain,
        }
    }

    pub fn run(&mut self) -> Result<Vec<Changes>, String> {
        let mut project_path = self.manifest.location.get_project_root_location().unwrap();
        project_path.append_path("chainhooks")?;
        if !project_path.exists() {
            let change = DirectoryCreation {
                comment: format!("{} chainhooks/", green!("Created directory"),),
                name: "chainhooks".to_string(),
                path: project_path.to_string(),
            };
            self.changes.push(Changes::AddDirectory(change))
        }
        match &self.chain {
            Chain::Bitcoin => self.create_template_bitcoin_chainhook()?,
            Chain::Stacks => self.create_template_stacks_chainhook()?,
        };
        Ok(self.changes.clone())
    }

    fn create_template_bitcoin_chainhook(&mut self) -> Result<(), String> {
        let content = format!(
            r#"
---
name: "Bitcoin hook"
version: 1
chain: bitcoin
networks:
    regtest:
        predicate:
            confirmations: 1                                    # 1 to 7. 1 = optimistic and better UX except when the chain is forking
            tx-out:                                             # support tx-in, tx-out.
                p2pkh:                                          # support hex, p2pkh, p2sh, p2wpkh, p2wsh.
                    equals: muYdXKmX9bByAueDe6KFfHd5Ff1gdN9ErG  # support equals, starts-with, ends-with
        action:
            http: 
                url: http://localhost:3000/api/v1/<path>
                method: POST
                authorization-header: "Bearer cn389ncoiwuencr"
"#
        );

        let name = format!("{}.chainhook.yaml", self.chainhook_name);
        let mut new_file = self
            .manifest
            .location
            .get_project_root_location()
            .expect("unable to retrieve project root");
        new_file.append_path(&format!("chainhooks/{}", name))?;
        if new_file.exists() {
            return Err(format!("{} already exists", new_file.to_string()));
        }
        let change = FileCreation {
            comment: format!("{} chainhooks/{}", green!("Created file"), name),
            name,
            content,
            path: new_file.to_string(),
        };
        self.changes.push(Changes::AddFile(change));
        Ok(())
    }

    fn create_template_stacks_chainhook(&mut self) -> Result<(), String> {
        let content = format!(
            r#"
---
name: "Stacks hook"
version: 1
chain: stacks
networks:
    devnet:
        predicate:
            print-event:
                contract-identifier: ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.cbtc-token
                contains: vault
            # Also supports the following predicates:
            # nft-event:
            #     asset-identifier: ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.cbtc-token::cbtc
            #     actions: [mint, transfer, burn]
            # ft-event:
            #     asset-identifier: ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.cbtc-token::cbtc
            #     actions: [mint, transfer, burn]
            # stx-event:
            #     asset-identifier: ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.cbtc-token::cbtc
            #     actions: [mint, transfer, lock]
            # contract-call:
            #     contract-identifier: ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.cbtc-token
            #     method: mint
        action:
            http: 
                url: http://localhost:3000/api/v1/<path>
                method: POST
                authorization-header: "Bearer cn389ncoiwuencr"
"#
        );

        let name = format!("{}.chainhook.yaml", self.chainhook_name);
        let mut project_path = self
            .manifest
            .location
            .get_project_root_location()
            .expect("unable to retrieve project root");
        project_path.append_path(&format!("chainhooks/{}", name))?;
        let change = FileCreation {
            comment: format!("{} chainhooks/{}", green!("Created file"), name),
            name,
            content,
            path: project_path.to_string(),
        };
        self.changes.push(Changes::AddFile(change));
        Ok(())
    }

    // TODO(lgalabru): should we index chainhooks in project manifests?
    // fn index_chainhook_in_clarinet_toml(&mut self) {
    //     let contract_file_name = format!("{}.chainhook.yaml", self.chainhook_name);
    //     let project_path = self.manifest.get_project_root_url();

    //     let contract_config = ContractConfig {
    //         path: format!("chainhooks/{}", contract_file_name),
    //         deployer: None,
    //     };
    //     let mut contracts_to_add = HashMap::new();
    //     contracts_to_add.insert(self.chainhook_name.clone(), contract_config);

    //     let change = TOMLEdition {
    //         comment: format!(
    //             "{} with chainhook {}",
    //             yellow!("Updated Clarinet.toml"),
    //             self.chainhook_name
    //         ),
    //         manifest_location,
    //         contracts_to_add,
    //         requirements_to_add: vec![],
    //     };
    //     self.changes.push(Changes::EditTOML(change));
    // }
}
