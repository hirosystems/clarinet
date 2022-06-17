use super::{utils, LspRequest};
use clarinet_files::FileLocation;
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
        let file_url = params.text_document_position.text_document.uri;
        let contract_location = match utils::get_contract_location(&file_url) {
            Some(location) => location,
            _ => return Ok(None),
        };

        let (response_tx, response_rx) = channel();
        let _ = match self.command_tx.lock() {
            Ok(tx) => tx.send(LspRequest::GetIntellisense(contract_location, response_tx)),
            Err(_) => return Ok(None),
        };

        let mut keywords = vec![];
        if let Ok(ref mut response) = response_rx.recv() {
            keywords.append(&mut response.completion_items);
        }

        Ok(Some(CompletionResponse::from(keywords)))
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let response_rx = if let Some(contract_location) =
            utils::get_contract_location(&params.text_document.uri)
        {
            let (response_tx, response_rx) = channel();
            let _ = match self.command_tx.lock() {
                Ok(tx) => tx.send(LspRequest::ContractOpened(contract_location, response_tx)),
                Err(_) => return,
            };
            response_rx
        } else if let Some(manifest_location) =
            utils::get_manifest_location(&params.text_document.uri)
        {
            let (response_tx, response_rx) = channel();
            let _ = match self.command_tx.lock() {
                Ok(tx) => tx.send(LspRequest::ManifestOpened(manifest_location, response_tx)),
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

        for (location, diags) in aggregated_diagnostics.into_iter() {
            if let Ok(url) = location.to_url_string() {
                self.client
                    .publish_diagnostics(Url::parse(&url).unwrap(), diags, None)
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
                Ok(tx) => tx.send(LspRequest::ContractChanged(contract_location, response_tx)),
                Err(_) => return,
            };
            response_rx
        } else if let Some(manifest_location) =
            utils::get_manifest_location(&params.text_document.uri)
        {
            let (response_tx, response_rx) = channel();
            let _ = match self.command_tx.lock() {
                Ok(tx) => tx.send(LspRequest::ManifestChanged(manifest_location, response_tx)),
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

        for (location, diags) in aggregated_diagnostics.into_iter() {
            if let Ok(url) = location.to_url_string() {
                self.client
                    .publish_diagnostics(Url::parse(&url).unwrap(), diags, None)
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

// pub fn diagnostic_lsp_type_to_tower_lsp_type(diagnostic: &mut clarity_lsp::lsp_types::Diagnostic) -> tower_lsp::lsp_types::Diagnostic {
//     tower_lsp::lsp_types::Diagnostic {
//         range: range_lsp_type_to_tower_lsp_type(diagnostic.range),
//         severity: diagnostic.severity.and_then(|s| Some(severity_lsp_type_to_tower_lsp_type(s))),
//         code: diagnostic.code.and_then(|s| Some(number_or_string_lsp_type_to_tower_lsp_type(s))),
//         code_description: diagnostic.code_description.take(),
//         source: diagnostic.source.take(),
//         message: diagnostic.message.take(),
//         related_information: diag.related_information,
//         tags: diag.tags,
//         data: diag.data,
//     }
// }

// pub fn range_lsp_type_to_tower_lsp_type(range: clarity_lsp::lsp_types::Range) -> tower_lsp::lsp_types::Range {
//     tower_lsp::lsp_types::Range {
//         start: position_lsp_type_to_tower_lsp_type(range.start),
//         end: position_lsp_type_to_tower_lsp_type(range.end),
//     }
// }

// pub fn position_lsp_type_to_tower_lsp_type(position: clarity_lsp::lsp_types::Position) -> tower_lsp::lsp_types::Position {
//     tower_lsp::lsp_types::Position {
//         line: position.line,
//         character: position.character,
//     }
// }

// pub fn severity_lsp_type_to_tower_lsp_type(severity: clarity_lsp::lsp_types::DiagnosticSeverity) -> tower_lsp::lsp_types::DiagnosticSeverity {
//     match severity {
//         clarity_lsp::lsp_types::DiagnosticSeverity::ERROR => tower_lsp::lsp_types::DiagnosticSeverity::Error,
//         clarity_lsp::lsp_types::DiagnosticSeverity::WARNING => tower_lsp::lsp_types::DiagnosticSeverity::Warning,
//         clarity_lsp::lsp_types::DiagnosticSeverity::HINT => tower_lsp::lsp_types::DiagnosticSeverity::Hint,
//         clarity_lsp::lsp_types::DiagnosticSeverity::INFORMATION => tower_lsp::lsp_types::DiagnosticSeverity::Information,
//     }
// }

// pub fn number_or_string_lsp_type_to_tower_lsp_type(number_or_string: clarity_lsp::lsp_types::NumberOrString) -> tower_lsp::lsp_types::NumberOrString {
//     match number_or_string {
//         clarity_lsp::lsp_types::NumberOrString::Number(i) => tower_lsp::lsp_types::NumberOrString::Number(i),
//         clarity_lsp::lsp_types::NumberOrString::String(s) => tower_lsp::lsp_types::NumberOrString::Number(s),
//     }
// }

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
