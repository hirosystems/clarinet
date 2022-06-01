use super::changes::{Changes, DirectoryCreation, FileCreation, TOMLEdition};
use crate::types::{ContractConfig, ProjectManifest};
use orchestra_types::Chain;
use std::{collections::HashMap, path::PathBuf};

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

    pub fn run(&mut self) -> Vec<Changes> {
        let mut project_path = self.manifest.get_project_root_dir();
        project_path.push("chainhooks");
        if !project_path.exists() {
            let change = DirectoryCreation {
                comment: format!("{} chainhooks/", green!("Created directory"),),
                name: "chainhooks".to_string(),
                path: format!("{}", project_path.display()),
            };
            self.changes.push(Changes::AddDirectory(change))
        }
        match &self.chain {
            Chain::Bitcoin => self.create_template_bitcoin_chainhook(),
            Chain::Stacks => self.create_template_stacks_chainhook(),
        };
        self.changes.clone()
    }

    fn create_template_bitcoin_chainhook(&mut self) {
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
            http-hook: 
                url: http://localhost:9000/chain-events/
                method: POST
                authorization-header: "Bearer cn389ncoiwuencr"
"#
        );

        let name = format!("{}.chainhook.yaml", self.chainhook_name);
        let project_path = self.manifest.get_project_root_dir();
        let path = format!("{}/chainhooks/{}", project_path.display(), name);
        let change = FileCreation {
            comment: format!("{} chainhooks/{}", green!("Created file"), name),
            name,
            content,
            path,
        };
        self.changes.push(Changes::AddFile(change));
    }

    fn create_template_stacks_chainhook(&mut self) {
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
                contract-id: ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.cbtc-token
                contains: vault
            # Also supports the following predicates:
            # nft-event:
            #     asset-id: ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.cbtc-token.cbtc
            #     actions: [mint, transfer, burn]
            # ft-event:
            #     asset-id: ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.cbtc-token.cbtc
            #     actions: [mint, transfer, burn]
            # stx-event:
            #     asset-id: ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.cbtc-token.cbtc
            #     actions: [mint, transfer, burn]
            # contract-call:
            #     contract-identifier: ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM.cbtc-token
            #     method: mint
        action:
            http-hook: 
                url: http://localhost:9000/chain-events/
                method: POST
                authorization-header: "Bearer cn389ncoiwuencr"
"#
        );

        let name = format!("{}.chainhook.yaml", self.chainhook_name);
        let project_path = self.manifest.get_project_root_dir();
        let path = format!("{}/chainhooks/{}", project_path.display(), name);
        let change = FileCreation {
            comment: format!("{} chainhooks/{}", green!("Created file"), name),
            name,
            content,
            path,
        };
        self.changes.push(Changes::AddFile(change));
    }

    // TODO(lgalabru): should we index chainhooks in project manifests?
    // fn index_chainhook_in_clarinet_toml(&mut self) {
    //     let contract_file_name = format!("{}.chainhook.yaml", self.chainhook_name);
    //     let project_path = self.manifest.get_project_root_dir();

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
    //         manifest_path,
    //         contracts_to_add,
    //         requirements_to_add: vec![],
    //     };
    //     self.changes.push(Changes::EditTOML(change));
    // }
}
