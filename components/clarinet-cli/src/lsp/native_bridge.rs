use super::{utils, LspRequestAsync};

use crate::lsp::{clarity_diagnostics_to_tower_lsp_type, completion_item_type_to_tower_lsp_type};
use serde_json::Value;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::Mutex;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::{
    CompletionOptions, CompletionParams, CompletionResponse, DeclarationCapability,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, ExecuteCommandParams, HoverProviderCapability, InitializeParams,
    InitializeResult, InitializedParams, MessageType, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
    TextDocumentSyncSaveOptions, Url,
};
use tower_lsp::{async_trait, Client, LanguageServer};

// The LSP is being initialized when clarity files are being detected in the project.
// We want the LSP to be notified when 2 kind of edits happened:
// - .clar file opened:
//      - if the state is empty
//      - if the state is ready
// - Clarinet.toml file saved
// - .clar files saved
//      - if indexed in `Clarinet.toml`:
//      - if not indexed:
// - Clarinet.toml file saved

#[derive(Debug)]
pub struct LspNativeBridge {
    client: Client,
    command_tx: Arc<Mutex<Sender<LspRequestAsync>>>,
}

impl LspNativeBridge {
    pub fn new(client: Client, command_tx: Sender<LspRequestAsync>) -> Self {
        Self {
            client,
            command_tx: Arc::new(Mutex::new(command_tx)),
        }
    }
}

#[async_trait]
impl LanguageServer for LspNativeBridge {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::Full),
                        will_save: Some(true),
                        will_save_wait_until: Some(false),
                        save: Some(TextDocumentSyncSaveOptions::Supported(true)),
                    },
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

    async fn initialized(&self, _params: InitializedParams) {}

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn execute_command(&self, _: ExecuteCommandParams) -> Result<Option<Value>> {
        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        // We receive notifications for toml and clar files, but only want to achieve this capability
        // for clar files.
        let file_url = params.text_document_position.text_document.uri;
        let contract_location = match utils::get_contract_location(&file_url) {
            Some(location) => location,
            _ => return Ok(None),
        };

        let (response_tx, response_rx) = channel();
        let _ = match self.command_tx.lock() {
            Ok(tx) => tx.send(LspRequestAsync::GetIntellisense(
                contract_location,
                response_tx,
            )),
            Err(_) => return Ok(None),
        };

        let mut keywords = vec![];
        if let Ok(ref mut response) = response_rx.recv() {
            keywords.append(&mut response.completion_items);
        }

        let mut completion_items = vec![];
        for mut item in keywords.drain(..) {
            completion_items.push(completion_item_type_to_tower_lsp_type(&mut item));
        }

        Ok(Some(CompletionResponse::from(completion_items)))
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let response_rx = if let Some(contract_location) =
            utils::get_contract_location(&params.text_document.uri)
        {
            let (response_tx, response_rx) = channel();
            let _ = match self.command_tx.lock() {
                Ok(tx) => tx.send(LspRequestAsync::ContractOpened(
                    contract_location,
                    response_tx,
                )),
                Err(_) => return,
            };
            response_rx
        } else if let Some(manifest_location) =
            utils::get_manifest_location(&params.text_document.uri)
        {
            let (response_tx, response_rx) = channel();
            let _ = match self.command_tx.lock() {
                Ok(tx) => tx.send(LspRequestAsync::ManifestOpened(
                    manifest_location,
                    response_tx,
                )),
                Err(_) => return,
            };
            response_rx
        } else {
            self.client
                .log_message(MessageType::Warning, "Unsupported file opened")
                .await;
            return;
        };

        self.client
            .log_message(
                MessageType::Warning,
                "Command submitted to background thread",
            )
            .await;
        let mut aggregated_diagnostics = vec![];
        let mut notification = None;
        if let Ok(ref mut response) = response_rx.recv() {
            aggregated_diagnostics.append(&mut response.aggregated_diagnostics);
            notification = response.notification.take();
        }

        for (location, mut diags) in aggregated_diagnostics.drain(..) {
            if let Ok(url) = location.to_url_string() {
                self.client
                    .publish_diagnostics(
                        Url::parse(&url).unwrap(),
                        clarity_diagnostics_to_tower_lsp_type(&mut diags),
                        None,
                    )
                    .await;
            }
        }
        if let Some((level, message)) = notification {
            self.client
                .show_message(message_level_type_to_tower_lsp_type(&level), message)
                .await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let response_rx = if let Some(contract_location) =
            utils::get_contract_location(&params.text_document.uri)
        {
            let (response_tx, response_rx) = channel();
            let _ = match self.command_tx.lock() {
                Ok(tx) => tx.send(LspRequestAsync::ContractChanged(
                    contract_location,
                    response_tx,
                )),
                Err(_) => return,
            };
            response_rx
        } else if let Some(manifest_location) =
            utils::get_manifest_location(&params.text_document.uri)
        {
            let (response_tx, response_rx) = channel();
            let _ = match self.command_tx.lock() {
                Ok(tx) => tx.send(LspRequestAsync::ManifestChanged(
                    manifest_location,
                    response_tx,
                )),
                Err(_) => return,
            };
            response_rx
        } else {
            return;
        };

        let mut aggregated_diagnostics = vec![];
        let mut notification = None;
        if let Ok(ref mut response) = response_rx.recv() {
            aggregated_diagnostics.append(&mut response.aggregated_diagnostics);
            notification = response.notification.take();
        }

        for (location, mut diags) in aggregated_diagnostics.drain(..) {
            if let Ok(url) = location.to_url_string() {
                self.client
                    .publish_diagnostics(
                        Url::parse(&url).unwrap(),
                        clarity_diagnostics_to_tower_lsp_type(&mut diags),
                        None,
                    )
                    .await;
            }
        }
        if let Some((level, message)) = notification {
            self.client
                .show_message(message_level_type_to_tower_lsp_type(&level), message)
                .await;
        }
    }

    async fn did_change(&self, _changes: DidChangeTextDocumentParams) {}

    async fn did_close(&self, _: DidCloseTextDocumentParams) {}
}

pub fn message_level_type_to_tower_lsp_type(
    level: &clarity_lsp::lsp_types::MessageType,
) -> tower_lsp::lsp_types::MessageType {
    match level {
        &clarity_lsp::lsp_types::MessageType::ERROR => tower_lsp::lsp_types::MessageType::Error,
        &clarity_lsp::lsp_types::MessageType::WARNING => tower_lsp::lsp_types::MessageType::Warning,
        &clarity_lsp::lsp_types::MessageType::INFO => tower_lsp::lsp_types::MessageType::Info,
        _ => tower_lsp::lsp_types::MessageType::Log,
    }
}
