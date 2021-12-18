use clarity_repl::clarity::analysis::ContractAnalysis;
use clarity_repl::clarity::ast::ContractAST;
use clarity_repl::repl::{Session, SessionSettings};
use tokio;

use serde_json::Value;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{async_trait, Client, LanguageServer, LspService, Server};

use clarity_repl::clarity::analysis::AnalysisDatabase;
use clarity_repl::clarity::costs::LimitedCostTracker;
use clarity_repl::clarity::types::{QualifiedContractIdentifier, StandardPrincipalData};
use clarity_repl::clarity::{analysis, ast};
use clarity_repl::{clarity, repl};
use sha2::Digest;
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::{Arc, Mutex, RwLock};

use crate::poke::load_session_settings;
use crate::publish::Network;
use super::utils;

#[derive(Debug)]
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

#[derive(Debug)]
pub struct CompletionMaps {
    pub inter_contract: Vec<CompletionItem>,
    pub intra_contract: Vec<CompletionItem>,
}

#[derive(Debug)]
pub struct ContractState {
    analysis: ContractAnalysis,
    intellisense: CompletionMaps,
    session: Session,
    // TODO(lgalabru)
    // hash: Vec<u8>,
    // symbols: HashMap<String, Symbol>,
}

type Logs = Vec<String>;

#[derive(Debug)]
pub struct ClarityLanguageBackend {
    clarinet_toml_path: RwLock<Option<PathBuf>>,
    contracts: RwLock<HashMap<Url, ContractState>>,
    client: Client,
    native_functions: Vec<CompletionItem>,
}

impl ClarityLanguageBackend {
    pub fn new(client: Client) -> Self {
        Self {
            clarinet_toml_path: RwLock::new(None),
            contracts: RwLock::new(HashMap::new()),
            client,
            native_functions: utils::build_default_native_keywords_list(),
        }
    }

    pub fn run_full_analysis(
        &self,
    ) -> std::result::Result<(Vec<(Url, Diagnostic)>, Logs), (String, Logs)> {
        let mut logs = vec![];
        logs.push("Full analysis will start".into());

        // Retrieve ./Clarinet.toml and settings/Development.toml paths
        let settings = match self.get_config_files_paths() {
            Err(message) => return Err((message, logs)),
            Ok(Some(clarinet_toml_path)) => {
                // Read these 2 files and build a SessionSetting
                match load_session_settings(clarinet_toml_path, &Network::Devnet) {
                    Err(message) => return Err((message, logs)),
                    Ok((settings, _)) => settings,
                }
            }
            Ok(None) => SessionSettings::default(),
        };

        // Build a blank Session: we will be evaluating the contracts one by one
        let mut incremental_session = repl::Session::new(settings.clone());
        let mut collected_diagnostics = vec![];
        let mainnet = false;

        for (i, contract) in settings.initial_contracts.iter().enumerate() {
            let contract_path =
                PathBuf::from_str(&contract.path).expect("Expect url to be well formatted");
            let contract_url =
                Url::from_file_path(contract_path).expect("Expect url to be well formatted");
            let contract_id = contract
                .get_contract_identifier(mainnet)
                .expect("Expect contract to be named");
            let code = fs::read_to_string(&contract.path).expect("Expect file to be readable");

            logs.push(format!("Analysis #{}: {}", i, contract_id.to_string()));

            // Before doing anything, keep a clone of the session before inserting anything in the datastore.
            let session = incremental_session.clone();

            // Extract the AST, and try to move to the next contract if we throw an error:
            // we're trying to get as many errors as possible
            let mut ast = match incremental_session
                .interpreter
                .build_ast(contract_id.clone(), code.clone())
            {
                Ok(ast) => ast,
                Err((_, Some(diagnostic))) => {
                    collected_diagnostics.push((
                        contract_url.clone(),
                        utils::convert_clarity_diagnotic_to_lsp_diagnostic(diagnostic),
                    ));
                    continue;
                }
                _ => {
                    logs.push("Unable to get ast".into());
                    continue;
                }
            };

            // Run the analysis, and try to move to the next contract if we throw an error:
            // we're trying to get as many errors as possible
            let analysis = match incremental_session
                .interpreter
                .run_analysis(contract_id.clone(), &mut ast)
            {
                Ok(analysis) => analysis,
                Err((_, Some(diagnostic))) => {
                    collected_diagnostics.push((
                        contract_url.clone(),
                        utils::convert_clarity_diagnotic_to_lsp_diagnostic(diagnostic),
                    ));
                    continue;
                }
                _ => {
                    logs.push("Unable to get diagnostic".into());
                    continue;
                }
            };

            // Executing the contract will also save the contract into the Datastore. This is required
            // for the next contracts, that could depend on the current contract.
            let _ = incremental_session.interpreter.execute(
                contract_id.clone(),
                &mut ast,
                code.clone(),
                analysis.clone(),
                false,
                None,
            );

            // We have a legit contract, let's extract some Intellisense data that will be served for
            // auto-completion requests
            let intellisense = utils::build_intellisense(&analysis);

            let contract_state = ContractState {
                analysis,
                session,
                intellisense,
            };

            if let Ok(ref mut contracts_writer) = self.contracts.write() {
                contracts_writer.insert(contract_url, contract_state);
            } else {
                logs.push(format!("Unable to acquire write lock"));
            }
        }
        return Ok((collected_diagnostics, logs));
    }

    pub fn run_single_analysis(
        &self,
        url: Url,
    ) -> std::result::Result<(Vec<(Url, Diagnostic)>, Logs), (String, Logs)> {
        let mut logs = vec![];
        let settings = SessionSettings::default();
        let mut incremental_session = repl::Session::new(settings.clone());
        let mut collected_diagnostics = vec![];
        let mainnet = false;

        let contract_path = url.to_file_path().expect("Expect url to be well formatted");
        let code = fs::read_to_string(&contract_path).expect("Expect file to be readable");

        let contract_id = QualifiedContractIdentifier::transient();

        logs.push(format!("Analysis: {}", contract_id.to_string()));

        // Before doing anything, keep a clone of the session before inserting anything in the datastore.
        let session = incremental_session.clone();

        // Extract the AST, and try to move to the next contract if we throw an error:
        // we're trying to get as many errors as possible
        let mut ast = match incremental_session
            .interpreter
            .build_ast(contract_id.clone(), code.clone())
        {
            Ok(ast) => ast,
            Err((_, Some(diagnostic))) => {
                collected_diagnostics.push((
                    url.clone(),
                    utils::convert_clarity_diagnotic_to_lsp_diagnostic(diagnostic),
                ));
                return Ok((collected_diagnostics, logs));
            }
            _ => {
                logs.push("Unable to get ast".into());
                return Ok((collected_diagnostics, logs));
            }
        };

        // Run the analysis, and try to move to the next contract if we throw an error:
        // we're trying to get as many errors as possible
        let analysis = match incremental_session
            .interpreter
            .run_analysis(contract_id.clone(), &mut ast)
        {
            Ok(analysis) => analysis,
            Err((_, Some(diagnostic))) => {
                collected_diagnostics.push((
                    url.clone(),
                    utils::convert_clarity_diagnotic_to_lsp_diagnostic(diagnostic),
                ));
                return Ok((collected_diagnostics, logs));
            }
            _ => {
                logs.push("Unable to get diagnostic".into());
                return Ok((collected_diagnostics, logs));
            }
        };

        // We have a legit contract, let's extract some Intellisense data that will be served for
        // auto-completion requests
        let intellisense = utils::build_intellisense(&analysis);

        let contract_state = ContractState {
            analysis,
            session,
            intellisense,
        };

        if let Ok(ref mut contracts_writer) = self.contracts.write() {
            contracts_writer.insert(url, contract_state);
        } else {
            logs.push(format!("Unable to acquire write lock"));
        }

        return Ok((collected_diagnostics, logs));
    }

    fn get_contracts_urls(&self) -> Vec<Url> {
        let contracts_reader = self.contracts.read().unwrap();
        contracts_reader.keys().map(|u| u.clone()).collect()
    }

    fn get_config_files_paths(&self) -> std::result::Result<Option<(PathBuf)>, String> {
        match self.clarinet_toml_path.read() {
            Ok(clarinet_toml_path) => {
                match clarinet_toml_path.as_ref() {
                    Some(clarinet_toml_path) => Ok(Some(
                        clarinet_toml_path.clone(),
                    )),
                    _ => Ok(None),
                }
            }
            _ => return Err("Unable to acquire locks".into()),
        }
    }

    fn is_clarinet_workspace(&self) -> bool {
        match self.get_config_files_paths() {
            Ok(Some(clarinet_toml_path)) => true,
            _ => false,
        }
    }
}

impl ClarityLanguageBackend {
    async fn handle_diagnostics(
        &self,
        diagnostics: Option<Vec<(Url, Diagnostic)>>,
        logs: Vec<String>,
    ) {
        // let (diagnostics, messages) = self.run_incremental_analysis(None);
        for m in logs.iter() {
            self.client.log_message(MessageType::Info, m).await;
        }

        if let Some(diagnostics) = diagnostics {
            // Note: None != Some(vec![]): When we pass None, it means that we were unable to get some
            // diagnostics, so don't flush the current diagnostics.
            for url in self.get_contracts_urls().into_iter() {
                self.client.publish_diagnostics(url, vec![], None).await;
            }

            if !diagnostics.is_empty() {
                let erroring_files = diagnostics
                    .iter()
                    .map(|(url, _)| {
                        url.to_file_path()
                            .unwrap()
                            .file_name()
                            .unwrap()
                            .to_str()
                            .unwrap()
                            .to_string()
                    })
                    .collect::<Vec<_>>();
                self.client
                    .show_message(
                        MessageType::Error,
                        format!(
                            "Errors detected in following contracts: {}",
                            erroring_files.join(", ")
                        ),
                    )
                    .await;
            }
            for (url, diagnostic) in diagnostics.into_iter() {
                self.client
                    .publish_diagnostics(url, vec![diagnostic], None)
                    .await;
            }
        }
    }
}

#[async_trait]
impl LanguageServer for ClarityLanguageBackend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        let mut manifest_file_path = None;

        // Are we looking at a workspace that would include a Clarinet project?
        if let Some(workspace_folders) = params.workspace_folders {
            for folder in workspace_folders.iter() {
                let root_path = folder
                    .uri
                    .to_file_path()
                    .expect("Unable to turn URL into path");

                let mut clarinet_toml_path = root_path.clone();
                clarinet_toml_path.push("Clarinet.toml");

                if clarinet_toml_path.exists() {
                    manifest_file_path = Some(clarinet_toml_path);
                    break;
                }
            }
        }

        match (&manifest_file_path, params.root_uri) {
            (None, Some(root_uri)) => {
                // Are we looking at a folder that would include a Clarinet project?
                let root_path = root_uri
                    .to_file_path()
                    .expect("Unable to turn URL into path");

                let mut clarinet_toml_path = root_path.clone();
                clarinet_toml_path.push("Clarinet.toml");

                if clarinet_toml_path.exists() {
                    manifest_file_path = Some(clarinet_toml_path);
                }
            }
            _ => {}
        }

        if let Some(clarinet_toml_path) = manifest_file_path {
            let mut clarinet_toml_path_writer = self.clarinet_toml_path.write().unwrap();
            *clarinet_toml_path_writer = Some(clarinet_toml_path.clone());
        }

        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::Full,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: None,
                    all_commit_characters: None,
                    work_done_progress_options: Default::default(),
                }),
                type_definition_provider: None,
                hover_provider: Some(HoverProviderCapability::Simple(false)),
                declaration_provider: Some(DeclarationCapability::Simple(false)),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, params: InitializedParams) {
        // If we're not in a Clarinet workspace, don't try to be smart.
        if !self.is_clarinet_workspace() {
            return;
        }

        match self.run_full_analysis() {
            Ok((diagnostics, logs)) => {
                self.handle_diagnostics(Some(diagnostics), logs).await;
            }
            Err((message, logs)) => {
                self.handle_diagnostics(None, logs).await;
                self.client.log_message(MessageType::Error, message).await;
            }
        };
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn execute_command(&self, _: ExecuteCommandParams) -> Result<Option<Value>> {
        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let mut keywords = self.native_functions.clone();
        let contract_uri = params.text_document_position.text_document.uri;

        let (mut contract_keywords, mut contract_calls) = {
            let contracts_reader = self.contracts.read().unwrap();
            let contract_keywords = match contracts_reader.get(&contract_uri) {
                Some(entry) => entry.intellisense.intra_contract.clone(),
                _ => vec![],
            };
            let mut contract_calls = vec![];
            for (url, contract_state) in contracts_reader.iter() {
                if !contract_uri.eq(url) {
                    contract_calls.append(&mut contract_state.intellisense.inter_contract.clone());
                }
            }
            (contract_keywords, contract_calls)
        };

        keywords.append(&mut contract_keywords);
        keywords.append(&mut contract_calls);

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
            for item in keywords.iter_mut() {
                match item.kind {
                    Some(CompletionItemKind::Event)
                    | Some(CompletionItemKind::Function)
                    | Some(CompletionItemKind::Module)
                    | Some(CompletionItemKind::Class)
                    | Some(CompletionItemKind::Method) => {
                        item.insert_text = Some(format!("({})", item.insert_text.take().unwrap()));
                    }
                    _ => {}
                }
            }
        }

        Ok(Some(CompletionResponse::from(keywords)))
    }

    async fn did_open(&self, _: DidOpenTextDocumentParams) {}

    // async fn did_change(&self, changes: DidChangeTextDocumentParams) {
    //     if let Some(change) = changes.content_changes.last() {
    //         self.client.log_message(MessageType::Info, change.text.clone()).await;
    //     }
    // }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let results = match self.is_clarinet_workspace() {
            true => self.run_full_analysis(),
            false => self.run_single_analysis(params.text_document.uri),
        };

        match results {
            Ok((diagnostics, logs)) => {
                self.handle_diagnostics(Some(diagnostics), logs).await;
            }
            Err((message, logs)) => {
                self.handle_diagnostics(None, logs).await;
                self.client.log_message(MessageType::Error, message).await;
            }
        };
    }

    async fn did_close(&self, _: DidCloseTextDocumentParams) {}

    // fn symbol(&self, params: WorkspaceSymbolParams) -> Self::SymbolFuture {
    //     Box::new(future::ok(None))
    // }

    // fn goto_declaration(&self, _: TextDocumentPositionParams) -> Self::DeclarationFuture {
    //     Box::new(future::ok(None))
    // }

    // fn goto_definition(&self, _: TextDocumentPositionParams) -> Self::DefinitionFuture {
    //     Box::new(future::ok(None))
    // }

    // fn goto_type_definition(&self, _: TextDocumentPositionParams) -> Self::TypeDefinitionFuture {
    //     Box::new(future::ok(None))
    // }

    // fn hover(&self, _: TextDocumentPositionParams) -> Self::HoverFuture {
    //     // todo(ludo): to implement
    //     let result = Hover {
    //         contents: HoverContents::Scalar(MarkedString::String("".to_string())),
    //         range: None,
    //     };
    //     Box::new(future::ok(None))
    // }

    // fn document_highlight(&self, _: TextDocumentPositionParams) -> Self::HighlightFuture {
    //     Box::new(future::ok(None))
    // }
}
