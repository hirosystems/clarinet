use clarinet_deployments::types::DeploymentGenerationArtifacts;
use clarinet_deployments::{
    initiate_session_from_deployment, update_session_with_contracts_executions,
    update_session_with_genesis_accounts,
};
use clarinet_files::{FileLocation, ProjectManifest};
use clarity_repl::analysis::coverage::TestCoverageReport;
use clarity_repl::clarity::vm::analysis::contract_interface_builder::{
    build_contract_interface, ContractInterface,
};
use clarity_repl::clarity::vm::ast::ContractAST;
use clarity_repl::clarity::vm::types::QualifiedContractIdentifier;
use clarity_repl::clarity::vm::EvaluationResult;
use clarity_repl::repl::{session::CostsReport, Session};
use deno_core::error::AnyError;
use stacks_network::chainhook_event_observer::chainhooks::types::StacksChainhookSpecification;
use std::collections::HashMap;
use std::path::PathBuf;

use clarinet_deployments::types::DeploymentSpecification;

mod api_v1;
mod costs;
mod deno;
mod vendor;

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
            None,
        );

        let mut contracts_artifacts = HashMap::new();
        for (contract_id, execution_result) in execution_results.into_iter() {
            let execution_result = match execution_result {
                Ok(execution_result) => execution_result,
                Err(diagnostics) => {
                    println!("Error found in contract {}", contract_id);
                    for d in diagnostics {
                        println!("{}", d);
                    }
                    std::process::exit(1);
                }
            };
            if let EvaluationResult::Contract(contract_result) = execution_result.result {
                contracts_artifacts.insert(
                    contract_id.clone(),
                    AnalysisArtifacts {
                        ast: contract_result.contract.ast,
                        interface: build_contract_interface(&contract_result.contract.analysis),
                        source: contract_result.contract.code,
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

pub enum ChainhookEvent {
    PerformRequest(reqwest::RequestBuilder),
    Exit,
}

pub fn run_scripts(
    include: Vec<String>,
    coverage_report: Option<PathBuf>,
    include_costs_report: bool,
    watch: bool,
    allow_wallets: bool,
    allow_disk_read: bool,
    allow_disk_write: bool,
    allow_run: Option<Vec<String>>,
    allow_env: Option<Vec<String>>,
    manifest: &ProjectManifest,
    cache: DeploymentCache,
    deployment_plan_path: Option<String>,
    fail_fast: Option<u16>,
    filter: Option<String>,
    import_map: Option<String>,
    allow_net: bool,
    cache_location: FileLocation,
    ts_config: Option<String>,
    stacks_chainhooks: Vec<StacksChainhookSpecification>,
    mine_block_delay: u16,
) -> Result<usize, (AnyError, usize)> {
    block_on(deno::do_run_scripts(
        include,
        coverage_report,
        include_costs_report,
        watch,
        allow_wallets,
        allow_disk_read,
        allow_disk_write,
        allow_run,
        allow_env,
        manifest,
        cache,
        deployment_plan_path,
        fail_fast,
        filter,
        import_map,
        allow_net,
        cache_location,
        ts_config,
        stacks_chainhooks,
        mine_block_delay,
    ))
}

pub fn block_on<F, R>(future: F) -> R
where
    F: std::future::Future<Output = R>,
{
    let rt = hiro_system_kit::create_basic_runtime();
    rt.block_on(future)
}

#[derive(Debug)]
pub struct SessionArtifacts {
    pub coverage_reports: Vec<TestCoverageReport>,
    pub costs_reports: Vec<CostsReport>,
}
