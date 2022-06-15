use super::types::{CompletionItem, CompletionMaps};
use super::utils;
use clarinet_deployments::{
    generate_default_deployment, initiate_session_from_deployment,
    update_session_with_contracts_executions,
};
use clarinet_types::ProjectManifest;
use clarity_repl::analysis::ast_dependency_detector::DependencySet;
use clarity_repl::clarity::analysis::ContractAnalysis;
use clarity_repl::clarity::diagnostic::{Diagnostic as ClarityDiagnostic, Level as ClarityLevel};
use clarity_repl::clarity::types::QualifiedContractIdentifier;
use clarity_repl::repl::ast::ContractAST;
use lsp_types::{MessageType, Url};
use orchestra_types::StacksNetwork;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq)]
pub struct ContractState {
    intellisense: CompletionMaps,
    errors: Vec<ClarityDiagnostic>,
    warnings: Vec<ClarityDiagnostic>,
    notes: Vec<ClarityDiagnostic>,
    contract_id: QualifiedContractIdentifier,
    analysis: Option<ContractAnalysis>,
    path: PathBuf,
}

impl ContractState {
    pub fn new(
        contract_id: QualifiedContractIdentifier,
        _ast: ContractAST,
        _deps: DependencySet,
        mut diags: Vec<ClarityDiagnostic>,
        analysis: Option<ContractAnalysis>,
        path: PathBuf,
    ) -> ContractState {
        let mut errors = vec![];
        let mut warnings = vec![];
        let mut notes = vec![];

        for diag in diags.drain(..) {
            match diag.level {
                ClarityLevel::Error => {
                    errors.push(diag);
                }
                ClarityLevel::Warning => {
                    warnings.push(diag);
                }
                ClarityLevel::Note => {
                    notes.push(diag);
                }
            }
        }

        let intellisense = match analysis {
            Some(ref analysis) => utils::build_intellisense(&analysis),
            None => CompletionMaps::default(),
        };

        ContractState {
            contract_id,
            intellisense,
            errors,
            warnings,
            notes,
            analysis,
            path,
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct EditorState {
    pub protocols: HashMap<PathBuf, ProtocolState>,
    pub contracts_lookup: HashMap<Url, PathBuf>,
    pub native_functions: Vec<CompletionItem>,
}

impl EditorState {
    pub fn new() -> EditorState {
        EditorState {
            protocols: HashMap::new(),
            contracts_lookup: HashMap::new(),
            native_functions: utils::build_default_native_keywords_list(),
        }
    }

    pub fn index_protocol(&mut self, manifest_path: PathBuf, protocol: ProtocolState) {
        for (contract_uri, _) in protocol.contracts.iter() {
            self.contracts_lookup
                .insert(contract_uri.clone(), manifest_path.clone());
        }
        self.protocols.insert(manifest_path, protocol);
    }

    pub fn clear_protocol(&mut self, manifest_path: &PathBuf) {
        if let Some(protocol) = self.protocols.remove(manifest_path) {
            for (contract_uri, _) in protocol.contracts.iter() {
                self.contracts_lookup.remove(contract_uri);
            }
        }
    }

    pub fn clear_protocol_associated_with_contract(
        &mut self,
        contract_url: &Url,
    ) -> Option<PathBuf> {
        match self.contracts_lookup.get(&contract_url) {
            Some(manifest_path) => {
                let manifest_path = manifest_path.clone();
                self.clear_protocol(&manifest_path);
                Some(manifest_path)
            }
            None => None,
        }
    }

    pub fn get_completion_items_for_contract(&self, contract_url: &Url) -> Vec<CompletionItem> {
        let mut keywords = self.native_functions.clone();

        let mut user_defined_keywords = self
            .contracts_lookup
            .get(&contract_url)
            .and_then(|p| self.protocols.get(p))
            .and_then(|p| Some(p.get_completion_items_for_contract(contract_url)))
            .unwrap_or_default();

        keywords.append(&mut user_defined_keywords);
        keywords
    }

    pub fn get_aggregated_diagnostics(
        &self,
    ) -> (
        Vec<(Url, Vec<ClarityDiagnostic>)>,
        Option<(MessageType, String)>,
    ) {
        let mut contracts = vec![];
        let mut erroring_files = HashSet::new();
        let mut warning_files = HashSet::new();

        for (_, protocol_state) in self.protocols.iter() {
            for (contract_url, state) in protocol_state.contracts.iter() {
                let mut diags = vec![];

                // Convert and collect errors
                if !state.errors.is_empty() {
                    if let Some(file_name) = state.path.file_name().and_then(|f| f.to_str()) {
                        erroring_files.insert(file_name);
                    }
                    for error in state.errors.iter() {
                        diags.push(error.clone());
                    }
                }

                // Convert and collect warnings
                if !state.warnings.is_empty() {
                    if let Some(file_name) = state.path.file_name().and_then(|f| f.to_str()) {
                        warning_files.insert(file_name);
                    }
                    for warning in state.warnings.iter() {
                        diags.push(warning.clone());
                    }
                }

                // Convert and collect notes
                for note in state.notes.iter() {
                    diags.push(note.clone());
                }
                contracts.push((contract_url.clone(), diags));
            }
        }

        let tldr = match (erroring_files.len(), warning_files.len()) {
            (0, 0) => None,
            (0, warnings) if warnings > 0 => Some((
                MessageType::WARNING,
                format!(
                    "Warning detected in following contracts: {}",
                    warning_files.into_iter().collect::<Vec<_>>().join(", ")
                ),
            )),
            (errors, 0) if errors > 0 => Some((
                MessageType::ERROR,
                format!(
                    "Errors detected in following contracts: {}",
                    erroring_files.into_iter().collect::<Vec<_>>().join(", ")
                ),
            )),
            (_errors, _warnings) => Some((
                MessageType::ERROR,
                format!(
                    "Errors and warnings detected in following contracts: {}",
                    erroring_files.into_iter().collect::<Vec<_>>().join(", ")
                ),
            )),
        };

        (contracts, tldr)
    }
}

#[derive(Clone, Default, Debug)]
pub struct ProtocolState {
    contracts: HashMap<Url, ContractState>,
}

impl ProtocolState {
    pub fn new() -> ProtocolState {
        ProtocolState {
            contracts: HashMap::new(),
        }
    }

    pub fn consolidate(
        &mut self,
        paths: &mut HashMap<QualifiedContractIdentifier, (Url, PathBuf)>,
        asts: &mut HashMap<QualifiedContractIdentifier, ContractAST>,
        deps: &mut HashMap<QualifiedContractIdentifier, DependencySet>,
        diags: &mut HashMap<QualifiedContractIdentifier, Vec<ClarityDiagnostic>>,
        analyses: &mut HashMap<QualifiedContractIdentifier, Option<ContractAnalysis>>,
    ) {
        // Remove old paths
        // TODO(lgalabru)

        // Add / Replace new paths
        for (contract_id, (url, path)) in paths.iter() {
            let (contract_id, ast) = match asts.remove_entry(&contract_id) {
                Some(ast) => ast,
                None => continue,
            };
            let deps = match deps.remove(&contract_id) {
                Some(deps) => deps,
                None => DependencySet::new(),
            };
            let diags = match diags.remove(&contract_id) {
                Some(diags) => diags,
                None => vec![],
            };
            let analysis = match analyses.remove(&contract_id) {
                Some(analysis) => analysis,
                None => None,
            };

            let contract_state =
                ContractState::new(contract_id, ast, deps, diags, analysis, path.clone());
            self.contracts.insert(url.clone(), contract_state);
        }
    }

    pub fn get_completion_items_for_contract(&self, contract_uri: &Url) -> Vec<CompletionItem> {
        let mut keywords = vec![];

        let (mut contract_keywords, mut contract_calls) = {
            let contract_keywords = match self.contracts.get(&contract_uri) {
                Some(entry) => entry.intellisense.intra_contract.clone(),
                _ => vec![],
            };
            let mut contract_calls = vec![];
            for (url, contract_state) in self.contracts.iter() {
                if !contract_uri.eq(url) {
                    contract_calls.append(&mut contract_state.intellisense.inter_contract.clone());
                }
            }
            (contract_keywords, contract_calls)
        };

        keywords.append(&mut contract_keywords);
        keywords.append(&mut contract_calls);
        keywords
    }
}

pub fn build_state(
    manifest_path: &PathBuf,
    protocol_state: &mut ProtocolState,
) -> Result<(), String> {
    let mut paths = HashMap::new();
    let mut analyses = HashMap::new();

    // In the LSP use case, trying to load an existing deployment
    // might not be suitable, in an edition context, we should
    // expect contracts to be created, edited, removed.
    // A on-disk deployment could quickly lead to an outdated
    // view of the repo.
    let manifest = ProjectManifest::from_path(manifest_path)?;

    let (deployment, mut artifacts) =
        generate_default_deployment(&manifest, &StacksNetwork::Simnet, false)?;

    let mut session = initiate_session_from_deployment(&manifest);
    let results = update_session_with_contracts_executions(
        &mut session,
        &deployment,
        Some(&artifacts.asts),
        false,
    );
    for (contract_id, mut result) in results.into_iter() {
        let (url, path) = {
            let (_, relative_path) = match deployment.contracts.get(&contract_id) {
                Some(entry) => entry,
                None => continue,
            };
            let relative_path = PathBuf::from_str(relative_path).expect(&format!(
                "Unable to build path for contract {}",
                contract_id
            ));
            let mut path = manifest_path.clone();
            path.pop();
            path.extend(&relative_path);
            let url = Url::from_file_path(&path).unwrap();
            (url, path)
        };
        paths.insert(contract_id.clone(), (url, path));

        let contract_analysis = match result {
            Ok(ref mut execution_result) => {
                if let Some(entry) = artifacts.diags.get_mut(&contract_id) {
                    entry.append(&mut execution_result.diagnostics);
                }
                execution_result.contract.take()
            }
            Err(ref mut diags) => {
                if let Some(entry) = artifacts.diags.get_mut(&contract_id) {
                    entry.append(diags);
                }
                continue;
            }
        };
        if let Some((_, _, _, _, contract_analysis)) = contract_analysis {
            analyses.insert(contract_id.clone(), Some(contract_analysis));
        }
    }

    protocol_state.consolidate(
        &mut paths,
        &mut artifacts.asts,
        &mut artifacts.deps,
        &mut artifacts.diags,
        &mut analyses,
    );
    Ok(())
}
