mod clarity_language_backend;
mod utils;
use crate::deployment::{
    generate_default_deployment, initiate_session_from_deployment,
    update_session_with_contracts_analyses,
};
use crate::types::{ProjectManifest, StacksNetwork};
use clarity_language_backend::ClarityLanguageBackend;
use clarity_repl::analysis::ast_dependency_detector::DependencySet;
use clarity_repl::clarity::analysis::ContractAnalysis;
use clarity_repl::clarity::diagnostic::{Diagnostic as ClarityDiagnostic, Level as ClarityLevel};
use clarity_repl::clarity::types::QualifiedContractIdentifier;
use clarity_repl::repl::ast::ContractAST;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::mpsc::{self, Receiver, Sender};
use tokio;
use tower_lsp::lsp_types::*;
use tower_lsp::{LspService, Server};

pub fn run_lsp() {
    match block_on(do_run_lsp()) {
        Err(_e) => std::process::exit(1),
        _ => {}
    };
}

pub fn block_on<F, R>(future: F) -> R
where
    F: std::future::Future<Output = R>,
{
    let rt = crate::utils::create_basic_runtime();
    rt.block_on(future)
}

async fn do_run_lsp() -> Result<(), String> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        start_server(rx);
    });

    let (service, messages) = LspService::new(|client| ClarityLanguageBackend::new(client, tx));
    Server::new(stdin, stdout)
        .interleave(messages)
        .serve(service)
        .await;
    Ok(())
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
enum Symbol {
    PublicFunction,
    ReadonlyFunction,
    PrivateFunction,
    ImportedTrait,
    LocalVariable,
    Constant,
    DataMap,
    DataVar,
    FungibleToken,
    NonFungibleToken,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CompletionMaps {
    pub inter_contract: Vec<CompletionItem>,
    pub intra_contract: Vec<CompletionItem>,
    pub data_fields: Vec<CompletionItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContractState {
    intellisense: CompletionMaps,
    errors: Vec<Diagnostic>,
    warnings: Vec<Diagnostic>,
    notes: Vec<Diagnostic>,
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
                    errors.push(utils::convert_clarity_diagnotic_to_lsp_diagnostic(&diag));
                }
                ClarityLevel::Warning => {
                    warnings.push(utils::convert_clarity_diagnotic_to_lsp_diagnostic(&diag));
                }
                ClarityLevel::Note => {
                    notes.push(utils::convert_clarity_diagnotic_to_lsp_diagnostic(&diag));
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
    protocols: HashMap<PathBuf, ProtocolState>,
    contracts_lookup: HashMap<Url, PathBuf>,
    native_functions: Vec<CompletionItem>,
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
    ) -> (Vec<(Url, Vec<Diagnostic>)>, Option<(MessageType, String)>) {
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
                MessageType::Warning,
                format!(
                    "Warning detected in following contracts: {}",
                    warning_files.into_iter().collect::<Vec<_>>().join(", ")
                ),
            )),
            (errors, 0) if errors > 0 => Some((
                MessageType::Error,
                format!(
                    "Errors detected in following contracts: {}",
                    erroring_files.into_iter().collect::<Vec<_>>().join(", ")
                ),
            )),
            (_errors, _warnings) => Some((
                MessageType::Error,
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

pub enum LspRequest {
    ManifestOpened(PathBuf, Sender<Response>),
    ManifestChanged(PathBuf, Sender<Response>),
    ContractOpened(Url, Sender<Response>),
    ContractChanged(Url, Sender<Response>),
    GetIntellisense(Url, Sender<Response>),
}

#[derive(Default, Debug, PartialEq)]
pub struct Response {
    aggregated_diagnostics: Vec<(Url, Vec<Diagnostic>)>,
    notification: Option<(MessageType, String)>,
    completion_items: Vec<CompletionItem>,
}

impl Response {
    pub fn error(message: &str) -> Response {
        Response {
            aggregated_diagnostics: vec![],
            completion_items: vec![],
            notification: Some((MessageType::Error, format!("Internal error: {}", message))),
        }
    }
}

fn start_server(command_rx: Receiver<LspRequest>) {
    let mut editor_state = EditorState::new();

    loop {
        let command = match command_rx.recv() {
            Ok(command) => command,
            Err(_e) => {
                break;
            }
        };
        match command {
            LspRequest::GetIntellisense(contract_url, response_tx) => {
                let mut completion_items =
                    editor_state.get_completion_items_for_contract(&contract_url);

                // Little big detail: should we wrap the inserted_text with braces?
                let should_wrap = {
                    // let line = params.text_document_position.position.line;
                    // let char = params.text_document_position.position.character;
                    // let doc = params.text_document_position.text_document.uri;
                    //
                    // TODO(lgalabru): from there, we'd need to get the prior char
                    // and see if a parenthesis was opened. If not, we need to wrap.
                    // The LSP would need to update its local document cache, via
                    // the did_change method.
                    true
                };
                if should_wrap {
                    for item in completion_items.iter_mut() {
                        match item.kind {
                            Some(CompletionItemKind::Event)
                            | Some(CompletionItemKind::Function)
                            | Some(CompletionItemKind::Module)
                            | Some(CompletionItemKind::Class)
                            | Some(CompletionItemKind::Method) => {
                                item.insert_text =
                                    Some(format!("({})", item.insert_text.take().unwrap()));
                            }
                            _ => {}
                        }
                    }
                }

                let _ = response_tx.send(Response {
                    aggregated_diagnostics: vec![],
                    notification: None,
                    completion_items,
                });
            }
            LspRequest::ManifestOpened(opened_manifest_path, response_tx) => {
                // The only reason why we're waiting for this kind of events, is building our initial state
                // if the system is initialized, move on.
                if editor_state.protocols.contains_key(&opened_manifest_path) {
                    let _ = response_tx.send(Response::default());
                    continue;
                }

                // With this manifest_path, let's initialize our state.
                let mut protocol_state = ProtocolState::new();
                match build_state(&opened_manifest_path, &mut protocol_state) {
                    Ok(_) => {
                        editor_state.index_protocol(opened_manifest_path, protocol_state);
                        let (aggregated_diagnostics, notification) =
                            editor_state.get_aggregated_diagnostics();
                        let _ = response_tx.send(Response {
                            aggregated_diagnostics,
                            notification,
                            completion_items: vec![],
                        });
                    }
                    Err(e) => {
                        let _ = response_tx.send(Response::error(&e));
                    }
                };
            }
            LspRequest::ContractOpened(contract_url, response_tx) => {
                // The only reason why we're waiting for this kind of events, is building our initial state
                // if the system is initialized, move on.
                let manifest_path = match utils::get_manifest_path_from_contract_url(&contract_url)
                {
                    Some(manifest_path) => manifest_path,
                    None => {
                        let _ = response_tx.send(Response::default());
                        continue;
                    }
                };

                if editor_state.protocols.contains_key(&manifest_path) {
                    let _ = response_tx.send(Response::default());
                    continue;
                }

                // With this manifest_path, let's initialize our state.
                let mut protocol_state = ProtocolState::new();
                match build_state(&manifest_path, &mut protocol_state) {
                    Ok(_) => {
                        editor_state.index_protocol(manifest_path, protocol_state);
                        let (aggregated_diagnostics, notification) =
                            editor_state.get_aggregated_diagnostics();
                        let _ = response_tx.send(Response {
                            aggregated_diagnostics,
                            notification,
                            completion_items: vec![],
                        });
                    }
                    Err(e) => {
                        let _ = response_tx.send(Response::error(&e));
                    }
                };
            }
            LspRequest::ManifestChanged(manifest_path, response_tx) => {
                editor_state.clear_protocol(&manifest_path);

                // We will rebuild the entire state, without to try any optimizations for now
                let mut protocol_state = ProtocolState::new();
                match build_state(&manifest_path, &mut protocol_state) {
                    Ok(_) => {
                        editor_state.index_protocol(manifest_path, protocol_state);
                        let (aggregated_diagnostics, notification) =
                            editor_state.get_aggregated_diagnostics();
                        let _ = response_tx.send(Response {
                            aggregated_diagnostics,
                            notification,
                            completion_items: vec![],
                        });
                    }
                    Err(e) => {
                        let _ = response_tx.send(Response::error(&e));
                    }
                };
            }
            LspRequest::ContractChanged(contract_url, response_tx) => {
                let manifest_path =
                    match editor_state.clear_protocol_associated_with_contract(&contract_url) {
                        Some(manifest_path) => manifest_path,
                        None => match utils::get_manifest_path_from_contract_url(&contract_url) {
                            Some(manifest_path) => manifest_path,
                            None => {
                                let _ = response_tx.send(Response::default());
                                continue;
                            }
                        },
                    };
                // TODO(lgalabru): introduce partial analysis
                // https://github.com/hirosystems/clarity-lsp/issues/98
                // We will rebuild the entire state, without trying any optimizations for now
                let mut protocol_state = ProtocolState::new();
                match build_state(&manifest_path, &mut protocol_state) {
                    Ok(_contracts_updates) => {
                        editor_state.index_protocol(manifest_path, protocol_state);
                        let (aggregated_diagnostics, notification) =
                            editor_state.get_aggregated_diagnostics();
                        let _ = response_tx.send(Response {
                            aggregated_diagnostics,
                            notification,
                            completion_items: vec![],
                        });
                    }
                    Err(e) => {
                        let _ = response_tx.send(Response::error(&e));
                    }
                };
            }
        }
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
        generate_default_deployment(&manifest, &StacksNetwork::Simnet)?;

    let mut session = initiate_session_from_deployment(&manifest);
    let results =
        update_session_with_contracts_analyses(&mut session, &deployment, &artifacts.asts);
    for (contract_id, result) in results.into_iter() {
        let (url, path) = {
            let (_, relative_path) = deployment.contracts.get(&contract_id).unwrap();
            let relative_path = PathBuf::from_str(relative_path).unwrap();
            let mut path = manifest_path.clone();
            path.pop();
            path.extend(&relative_path);
            let url = Url::from_file_path(&path).unwrap();
            (url, path)
        };
        paths.insert(contract_id.clone(), (url, path));

        let (contract_analysis, mut analysis_diags) = match result {
            Ok((contract_analysis, diags)) => (Some(contract_analysis), diags),
            Err(diags) => (None, diags),
        };
        if let Some(entry) = artifacts.diags.get_mut(&contract_id) {
            entry.append(&mut analysis_diags);
        }
        analyses.insert(contract_id.clone(), contract_analysis);
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

#[test]
fn test_opening_counter_contract_should_return_fresh_analysis() {
    use std::sync::mpsc::channel;

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        start_server(rx);
    });

    let mut counter_path = std::env::current_dir().expect("Unable to get current dir");
    counter_path.push("examples");
    counter_path.push("counter");
    counter_path.push("contracts");
    counter_path.push("counter.clar");
    let counter_url = Url::from_file_path(counter_path).unwrap();

    let (response_tx, response_rx) = channel();
    let _ = tx.send(LspRequest::ContractOpened(
        counter_url.clone(),
        response_tx.clone(),
    ));
    let response = response_rx.recv().expect("Unable to get response");

    // the counter project should emit 2 warnings and 2 notes coming from counter.clar
    assert_eq!(response.aggregated_diagnostics.len(), 1);
    let (_url, diags) = &response.aggregated_diagnostics[0];
    assert_eq!(diags.len(), 4);

    // re-opening this contract should not trigger a full analysis
    let _ = tx.send(LspRequest::ContractOpened(counter_url, response_tx));
    let response = response_rx.recv().expect("Unable to get response");
    assert_eq!(response, Response::default());
}

#[test]
fn test_opening_counter_manifest_should_return_fresh_analysis() {
    use std::sync::mpsc::channel;

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        start_server(rx);
    });

    let mut manifest_path = std::env::current_dir().expect("Unable to get current dir");
    manifest_path.push("examples");
    manifest_path.push("counter");
    manifest_path.push("Clarinet.toml");

    let (response_tx, response_rx) = channel();
    let _ = tx.send(LspRequest::ManifestOpened(
        manifest_path.clone(),
        response_tx.clone(),
    ));
    let response = response_rx.recv().expect("Unable to get response");

    // the counter project should emit 2 warnings and 2 notes coming from counter.clar
    assert_eq!(response.aggregated_diagnostics.len(), 1);
    let (_url, diags) = &response.aggregated_diagnostics[0];
    assert_eq!(diags.len(), 4);

    // re-opening this manifest should not trigger a full analysis
    let _ = tx.send(LspRequest::ManifestOpened(manifest_path, response_tx));
    let response = response_rx.recv().expect("Unable to get response");
    assert_eq!(response, Response::default());
}

#[test]
fn test_opening_simple_nft_manifest_should_return_fresh_analysis() {
    use std::sync::mpsc::channel;

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        start_server(rx);
    });

    let mut manifest_path = std::env::current_dir().expect("Unable to get current dir");
    manifest_path.push("examples");
    manifest_path.push("simple-nft");
    manifest_path.push("Clarinet.toml");

    let (response_tx, response_rx) = channel();
    let _ = tx.send(LspRequest::ManifestOpened(
        manifest_path.clone(),
        response_tx.clone(),
    ));
    let response = response_rx.recv().expect("Unable to get response");

    // the counter project should emit 2 warnings and 2 notes coming from counter.clar
    assert_eq!(response.aggregated_diagnostics.len(), 2);
    let (_url, diags) = &response.aggregated_diagnostics[0];
    assert_eq!(diags.len(), 8);
}
