use super::utils;

use crate::lsp::{clarity_diagnostics_to_tower_lsp_type, completion_item_type_to_tower_lsp_type};
use clarity_lsp::backend::{
    process_notification, process_request, EditorStateInput, LspNotification,
    LspNotificationResponse, LspRequest, LspRequestResponse,
};
use clarity_lsp::state::EditorState;
use crossbeam_channel::{Receiver as MultiplexableReceiver, Select, Sender as MultiplexableSender};
use serde_json::Value;
use std::sync::mpsc::{Receiver, Sender};
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

pub enum LspResponse {
    Notification(LspNotificationResponse),
    Request(LspRequestResponse),
}

pub async fn start_language_server(
    notification_rx: MultiplexableReceiver<LspNotification>,
    request_rx: MultiplexableReceiver<LspRequest>,
    response_tx: Sender<LspResponse>,
) {
    let mut editor_state = EditorStateInput::new(Some(EditorState::new()), None);

    let mut sel = Select::new();
    let notifications_oper = sel.recv(&notification_rx);
    let requests_oper = sel.recv(&request_rx);

    loop {
        let oper = sel.select();
        match oper.index() {
            i if i == notifications_oper => match oper.recv(&notification_rx) {
                Ok(notification) => {
                    let result = process_notification(notification, &mut editor_state, None).await;
                    if let Ok(response) = result {
                        let _ = response_tx.send(LspResponse::Notification(response));
                    }
                }
                Err(_e) => {
                    continue;
                }
            },
            i if i == requests_oper => match oper.recv(&request_rx) {
                Ok(request) => {
                    let request_response = process_request(request, &mut editor_state);
                    let _ = response_tx.send(LspResponse::Request(request_response));
                }
                Err(_e) => {
                    continue;
                }
            },
            _ => unreachable!(),
        }
    }
}

#[derive(Debug)]
pub struct LspNativeBridge {
    client: Client,
    notification_tx: Arc<Mutex<MultiplexableSender<LspNotification>>>,
    request_tx: Arc<Mutex<MultiplexableSender<LspRequest>>>,
    response_rx: Arc<Mutex<Receiver<LspResponse>>>,
}

impl LspNativeBridge {
    pub fn new(
        client: Client,
        notification_tx: MultiplexableSender<LspNotification>,
        request_tx: MultiplexableSender<LspRequest>,
        response_rx: Receiver<LspResponse>,
    ) -> Self {
        Self {
            client,
            notification_tx: Arc::new(Mutex::new(notification_tx)),
            request_tx: Arc::new(Mutex::new(request_tx)),
            response_rx: Arc::new(Mutex::new(response_rx)),
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
                        will_save: Some(false),
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

        let _ = match self.request_tx.lock() {
            Ok(tx) => tx.send(LspRequest::GetIntellisense(contract_location)),
            Err(_) => return Ok(None),
        };

        let mut keywords = vec![];
        if let Ok(response_rx) = self.response_rx.lock() {
            if let Ok(ref mut response) = response_rx.recv() {
                if let LspResponse::Request(request_response) = response {
                    keywords.append(&mut request_response.completion_items);
                }
            }
        }

        let mut completion_items = vec![];
        for mut item in keywords.drain(..) {
            completion_items.push(completion_item_type_to_tower_lsp_type(&mut item));
        }

        Ok(Some(CompletionResponse::from(completion_items)))
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        if let Some(contract_location) = utils::get_contract_location(&params.text_document.uri) {
            let _ = match self.notification_tx.lock() {
                Ok(tx) => tx.send(LspNotification::ContractOpened(contract_location)),
                Err(_) => return,
            };
        } else if let Some(manifest_location) =
            utils::get_manifest_location(&params.text_document.uri)
        {
            let _ = match self.notification_tx.lock() {
                Ok(tx) => tx.send(LspNotification::ManifestOpened(manifest_location)),
                Err(_) => return,
            };
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
        if let Ok(response_rx) = self.response_rx.lock() {
            if let Ok(ref mut response) = response_rx.recv() {
                if let LspResponse::Notification(notification_response) = response {
                    aggregated_diagnostics
                        .append(&mut notification_response.aggregated_diagnostics);
                    notification = notification_response.notification.take();
                }
            }
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
        if let Some(contract_location) = utils::get_contract_location(&params.text_document.uri) {
            let _ = match self.notification_tx.lock() {
                Ok(tx) => tx.send(LspNotification::ContractSaved(contract_location)),
                Err(_) => return,
            };
        } else if let Some(manifest_location) =
            utils::get_manifest_location(&params.text_document.uri)
        {
            let _ = match self.notification_tx.lock() {
                Ok(tx) => tx.send(LspNotification::ManifestSaved(manifest_location)),
                Err(_) => return,
            };
        } else {
            return;
        };

        let mut aggregated_diagnostics = vec![];
        let mut notification = None;
        if let Ok(response_rx) = self.response_rx.lock() {
            if let Ok(ref mut response) = response_rx.recv() {
                if let LspResponse::Notification(notification_response) = response {
                    aggregated_diagnostics
                        .append(&mut notification_response.aggregated_diagnostics);
                    notification = notification_response.notification.take();
                }
            }
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

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(contract_location) = utils::get_contract_location(&params.text_document.uri) {
            if let Ok(tx) = self.notification_tx.lock() {
                let _ = tx.send(LspNotification::ContractChanged(
                    contract_location,
                    params.content_changes[0].text.to_string(),
                ));
            };
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        if let Some(contract_location) = utils::get_contract_location(&params.text_document.uri) {
            if let Ok(tx) = self.notification_tx.lock() {
                let _ = tx.send(LspNotification::ContractClosed(contract_location));
            };
        }
    }
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
