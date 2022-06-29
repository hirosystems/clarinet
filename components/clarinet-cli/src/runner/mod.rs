#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_must_use)]

use crate::deployments::{
    apply_on_chain_deployment, check_deployments, get_absolute_deployment_path,
    get_default_deployment_path, load_deployment, write_deployment,
};
use clarinet_deployments::types::DeploymentGenerationArtifacts;
use clarinet_deployments::{
    generate_default_deployment, initiate_session_from_deployment, setup_session_with_deployment,
    update_session_with_contracts_executions, update_session_with_genesis_accounts,
};
use clarinet_files::ProjectManifest;
use clarity::vm::analysis::contract_interface_builder::{
    build_contract_interface, ContractInterface,
};
use clarity::vm::ast::ContractAST;
use clarity::vm::types::QualifiedContractIdentifier;
use clarity_repl::repl::Session;

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use clarinet_deployments::types::DeploymentSpecification;

pub mod api_v1;
mod costs;
pub mod deno;
mod utils;

#[derive(Clone)]
pub struct DeploymentCache {
    pub session: Session,
    pub session_accounts_only: Session,
    pub deployment_path: Option<String>,
    pub deployment: DeploymentSpecification,
    pub contracts_artifacts: HashMap<QualifiedContractIdentifier, AnalysisArtifacts>,
}

impl DeploymentCache {
    pub fn new(
        manifest: &ProjectManifest,
        deployment: DeploymentSpecification,
        deployment_path: &Option<String>,
        artifacts: DeploymentGenerationArtifacts,
    ) -> DeploymentCache {
        let mut session_accounts_only = initiate_session_from_deployment(&manifest);
        update_session_with_genesis_accounts(&mut session_accounts_only, &deployment);
        let mut session = session_accounts_only.clone();

        let execution_results = update_session_with_contracts_executions(
            &mut session,
            &deployment,
            Some(&artifacts.asts),
            true,
        );

        let mut contracts_artifacts = HashMap::new();
        for (contract_id, execution_result) in execution_results.into_iter() {
            let mut execution_result = match execution_result {
                Ok(execution_result) => execution_result,
                Err(diagnostics) => {
                    println!("Error found in contract {}", contract_id);
                    for d in diagnostics {
                        println!("{}", d);
                    }
                    std::process::exit(1);
                }
            };
            if let Some((_, source, functions, ast, analysis)) = execution_result.contract.take() {
                contracts_artifacts.insert(
                    contract_id.clone(),
                    AnalysisArtifacts {
                        ast,
                        interface: build_contract_interface(&analysis),
                        source,
                        dependencies: vec![],
                    },
                );
            }
        }

        DeploymentCache {
            session,
            session_accounts_only,
            deployment_path: deployment_path.clone(),
            contracts_artifacts,
            deployment,
        }
    }
}

#[derive(Clone)]
pub struct AnalysisArtifacts {
    pub ast: ContractAST,
    pub interface: ContractInterface,
    pub dependencies: Vec<String>,
    pub source: String,
}

pub fn run_scripts(
    files: Vec<String>,
    include_coverage: bool,
    include_costs_report: bool,
    watch: bool,
    allow_wallets: bool,
    allow_disk_write: bool,
    manifest: &ProjectManifest,
    cache: DeploymentCache,
    deployment_plan_path: Option<String>,
) -> Result<u32, (String, u32)> {
    match block_on(deno::do_run_scripts(
        files,
        include_coverage,
        include_costs_report,
        watch,
        allow_wallets,
        allow_disk_write,
        manifest,
        cache,
        deployment_plan_path,
    )) {
        Err(e) => Err((format!("{:?}", e), 0)),
        Ok(res) => Ok(res),
    }
}

pub fn block_on<F, R>(future: F) -> R
where
    F: std::future::Future<Output = R>,
{
    let rt = crate::utils::create_basic_runtime();
    rt.block_on(future)
}
