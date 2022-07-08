use crate::types::{CompletionItem, CompletionMaps};
use crate::utils;
use clarinet_deployments::{
    generate_default_deployment, initiate_session_from_deployment,
    update_session_with_contracts_executions,
};
use clarinet_files::ProjectManifest;
use clarinet_files::{FileAccessor, FileLocation};
use clarity_repl::analysis::ast_dependency_detector::DependencySet;
use clarity_repl::clarity::analysis::ContractAnalysis;
use clarity_repl::clarity::diagnostic::{Diagnostic as ClarityDiagnostic, Level as ClarityLevel};
use clarity_repl::clarity::types::QualifiedContractIdentifier;
use clarity_repl::repl::ast::ContractAST;
use lsp_types::MessageType;
use orchestra_types::StacksNetwork;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub struct ContractState {
    intellisense: CompletionMaps,
    errors: Vec<ClarityDiagnostic>,
    warnings: Vec<ClarityDiagnostic>,
    notes: Vec<ClarityDiagnostic>,
    contract_id: QualifiedContractIdentifier,
    analysis: Option<ContractAnalysis>,
    location: FileLocation,
}

impl ContractState {
    pub fn new(
        contract_id: QualifiedContractIdentifier,
        _ast: ContractAST,
        _deps: DependencySet,
        mut diags: Vec<ClarityDiagnostic>,
        analysis: Option<ContractAnalysis>,
        location: FileLocation,
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
            location,
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct EditorState {
    pub protocols: HashMap<FileLocation, ProtocolState>,
    pub contracts_lookup: HashMap<FileLocation, FileLocation>,
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

    pub fn index_protocol(&mut self, manifest_location: FileLocation, protocol: ProtocolState) {
        for (contract_uri, _) in protocol.contracts.iter() {
            self.contracts_lookup
                .insert(contract_uri.clone(), manifest_location.clone());
        }
        self.protocols.insert(manifest_location, protocol);
    }

    pub fn clear_protocol(&mut self, manifest_location: &FileLocation) {
        if let Some(protocol) = self.protocols.remove(manifest_location) {
            for (contract_location, _) in protocol.contracts.iter() {
                self.contracts_lookup.remove(contract_location);
            }
        }
    }

    pub fn clear_protocol_associated_with_contract(
        &mut self,
        contract_location: &FileLocation,
    ) -> Option<FileLocation> {
        match self.contracts_lookup.get(&contract_location) {
            Some(manifest_location) => {
                let manifest_location = manifest_location.clone();
                self.clear_protocol(&manifest_location);
                Some(manifest_location)
            }
            None => None,
        }
    }

    pub fn get_completion_items_for_contract(
        &self,
        contract_location: &FileLocation,
    ) -> Vec<CompletionItem> {
        let mut keywords = self.native_functions.clone();

        let mut user_defined_keywords = self
            .contracts_lookup
            .get(&contract_location)
            .and_then(|p| self.protocols.get(p))
            .and_then(|p| Some(p.get_completion_items_for_contract(contract_location)))
            .unwrap_or_default();

        keywords.append(&mut user_defined_keywords);
        keywords
    }

    pub fn get_aggregated_diagnostics(
        &self,
    ) -> (
        Vec<(FileLocation, Vec<ClarityDiagnostic>)>,
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
                    erroring_files.insert(state.location.to_string());
                    for error in state.errors.iter() {
                        diags.push(error.clone());
                    }
                }

                // Convert and collect warnings
                if !state.warnings.is_empty() {
                    warning_files.insert(state.location.to_string());
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
    contracts: HashMap<FileLocation, ContractState>,
}

impl ProtocolState {
    pub fn new() -> ProtocolState {
        ProtocolState {
            contracts: HashMap::new(),
        }
    }

    pub fn consolidate(
        &mut self,
        locations: &mut HashMap<QualifiedContractIdentifier, FileLocation>,
        asts: &mut HashMap<QualifiedContractIdentifier, ContractAST>,
        deps: &mut HashMap<QualifiedContractIdentifier, DependencySet>,
        diags: &mut HashMap<QualifiedContractIdentifier, Vec<ClarityDiagnostic>>,
        analyses: &mut HashMap<QualifiedContractIdentifier, Option<ContractAnalysis>>,
    ) {
        // Remove old paths
        // TODO(lgalabru)

        // Add / Replace new paths
        for (contract_id, contract_location) in locations.iter() {
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

            let contract_state = ContractState::new(
                contract_id,
                ast,
                deps,
                diags,
                analysis,
                contract_location.clone(),
            );
            self.contracts
                .insert(contract_location.clone(), contract_state);
        }
    }

    pub fn get_completion_items_for_contract(
        &self,
        contract_uri: &FileLocation,
    ) -> Vec<CompletionItem> {
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

pub async fn build_state<'a>(
    manifest_location: &FileLocation,
    protocol_state: &mut ProtocolState,
    file_accessor: Option<&Box<dyn FileAccessor>>,
) -> Result<(), String> {
    let mut locations = HashMap::new();
    let mut analyses = HashMap::new();

    // In the LSP use case, trying to load an existing deployment
    // might not be suitable, in an edition context, we should
    // expect contracts to be created, edited, removed.
    // A on-disk deployment could quickly lead to an outdated
    // view of the repo.
    let manifest = match file_accessor {
        None => ProjectManifest::from_location(manifest_location)?,
        Some(file_accessor) => {
            // We should create a new async function in `ProjectManifest` that can take a `file_accessor: &Box<dyn FileAccessor>`
            // as an argument, and that will use:
            // file_accessor.read_manifest_content()
            ProjectManifest::from_file_accessor(manifest_location, file_accessor).await?
        }
    };

    let (deployment, mut artifacts) =
        generate_default_deployment(&manifest, &StacksNetwork::Simnet, false, file_accessor)
            .await?;

    let mut session = initiate_session_from_deployment(&manifest);
    let results = update_session_with_contracts_executions(
        &mut session,
        &deployment,
        Some(&artifacts.asts),
        false,
    );
    for (contract_id, mut result) in results.into_iter() {
        let (_, contract_location) = match deployment.contracts.get(&contract_id) {
            Some(entry) => entry,
            None => continue,
        };
        locations.insert(contract_id.clone(), contract_location.clone());

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
        &mut locations,
        &mut artifacts.asts,
        &mut artifacts.deps,
        &mut artifacts.diags,
        &mut analyses,
    );
    Ok(())
}
