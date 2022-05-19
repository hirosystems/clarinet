use super::{utils, LspRequest};
use serde_json::Value;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::Mutex;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{async_trait, Client, LanguageServer};

type Logs = Vec<String>;

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
pub struct ClarityLanguageBackend {
    client: Client,
    command_tx: Arc<Mutex<Sender<LspRequest>>>,
}

impl ClarityLanguageBackend {
    pub fn new(client: Client, command_tx: Sender<LspRequest>) -> Self {
        Self {
            client,
            command_tx: Arc::new(Mutex::new(command_tx)),
        }
    }
}

#[async_trait]
impl LanguageServer for ClarityLanguageBackend {
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
        let contract_url = params.text_document_position.text_document.uri;
        if !contract_url.to_string().ends_with(".clar") {
            return Ok(None);
        }

        let (response_tx, response_rx) = channel();
        let _ = match self.command_tx.lock() {
            Ok(tx) => tx.send(LspRequest::GetIntellisense(contract_url, response_tx)),
            Err(_) => return Ok(None),
        };

        let mut keywords = vec![];
        if let Ok(ref mut response) = response_rx.recv() {
            keywords.append(&mut response.completion_items);
        }

        Ok(Some(CompletionResponse::from(keywords)))
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let response_rx = if let Some(contract_path) =
            utils::get_contract_file(&params.text_document.uri)
        {
            let (response_tx, response_rx) = channel();
            let _ = match self.command_tx.lock() {
                Ok(tx) => tx.send(LspRequest::ContractOpened(
                    params.text_document.uri,
                    response_tx,
                )),
                Err(_) => return,
            };
            response_rx
        } else if let Some(manifest_path) = utils::get_manifest_file(&params.text_document.uri) {
            let (response_tx, response_rx) = channel();
            let _ = match self.command_tx.lock() {
                Ok(tx) => tx.send(LspRequest::ManifestOpened(manifest_path, response_tx)),
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

        for (url, diags) in aggregated_diagnostics.into_iter() {
            self.client.publish_diagnostics(url, diags, None).await;
        }
        if let Some((level, message)) = notification {
            self.client.show_message(level, message).await;
        }
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let response_rx = if let Some(contract_path) =
            utils::get_contract_file(&params.text_document.uri)
        {
            let (response_tx, response_rx) = channel();
            let _ = match self.command_tx.lock() {
                Ok(tx) => tx.send(LspRequest::ContractChanged(
                    params.text_document.uri,
                    response_tx,
                )),
                Err(_) => return,
            };
            response_rx
        } else if let Some(manifest_path) = utils::get_contract_file(&params.text_document.uri) {
            let (response_tx, response_rx) = channel();
            let _ = match self.command_tx.lock() {
                Ok(tx) => tx.send(LspRequest::ManifestChanged(manifest_path, response_tx)),
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

        for (url, diags) in aggregated_diagnostics.into_iter() {
            self.client.publish_diagnostics(url, diags, None).await;
        }
        if let Some((level, message)) = notification {
            self.client.show_message(level, message).await;
        }
    }

    async fn did_change(&self, _changes: DidChangeTextDocumentParams) {}

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
