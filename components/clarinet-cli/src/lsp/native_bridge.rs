use super::utils;
use crate::lsp::clarity_diagnostics_to_tower_lsp_type;
use clarity_lsp::backend::{
    process_notification, process_request, EditorStateInput, LspNotification,
    LspNotificationResponse, LspRequest, LspRequestResponse,
};
use clarity_lsp::lsp_types::{
    DocumentSymbolParams, DocumentSymbolResponse, GotoDefinitionParams, GotoDefinitionResponse,
    SignatureHelp, SignatureHelpParams,
};
use clarity_lsp::state::EditorState;
use crossbeam_channel::{Receiver as MultiplexableReceiver, Select, Sender as MultiplexableSender};
use serde_json::Value;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::sync::Mutex;
use tower_lsp::jsonrpc::{Error, ErrorCode, Result};
use tower_lsp::lsp_types::{
    CompletionParams, CompletionResponse, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DidSaveTextDocumentParams, ExecuteCommandParams, Hover, HoverParams,
    InitializeParams, InitializeResult, InitializedParams, MessageType, Url,
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
    let mut editor_state = EditorStateInput::Owned(EditorState::new());

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
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        let _ = match self.request_tx.lock() {
            Ok(tx) => tx.send(LspRequest::Initialize(params)),
            Err(_) => return Err(Error::new(ErrorCode::InternalError)),
        };

        let response_rx = self.response_rx.lock().expect("failed to lock response_rx");
        let ref response = response_rx.recv().expect("failed to get value from recv");
        if let LspResponse::Request(LspRequestResponse::Initialize(initialize)) = response {
            return Ok(initialize.to_owned());
        }
        Err(Error::new(ErrorCode::InternalError))
    }

    async fn initialized(&self, _params: InitializedParams) {}

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn execute_command(&self, _: ExecuteCommandParams) -> Result<Option<Value>> {
        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let _ = match self.request_tx.lock() {
            Ok(tx) => tx.send(LspRequest::Completion(params)),
            Err(_) => return Ok(None),
        };

        let response_rx = self.response_rx.lock().expect("failed to lock response_rx");
        let ref response = response_rx.recv().expect("failed to get value from recv");
        if let LspResponse::Request(LspRequestResponse::CompletionItems(items)) = response {
            return Ok(Some(CompletionResponse::from(items.to_vec())));
        }

        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let _ = match self.request_tx.lock() {
            Ok(tx) => tx.send(LspRequest::Definition(params)),
            Err(_) => return Ok(None),
        };

        let response_rx = self.response_rx.lock().expect("failed to lock response_rx");
        let ref response = response_rx.recv().expect("failed to get value from recv");
        if let LspResponse::Request(LspRequestResponse::Definition(Some(data))) = response {
            return Ok(Some(GotoDefinitionResponse::Scalar(data.to_owned())));
        }

        Ok(None)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let _ = match self.request_tx.lock() {
            Ok(tx) => tx.send(LspRequest::DocumentSymbol(params)),
            Err(_) => return Ok(None),
        };

        let response_rx = self.response_rx.lock().expect("failed to lock response_rx");
        let ref response = response_rx.recv().expect("failed to get value from recv");
        if let LspResponse::Request(LspRequestResponse::DocumentSymbol(symbols)) = response {
            return Ok(Some(DocumentSymbolResponse::Nested(symbols.to_vec())));
        }

        Ok(None)
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let _ = match self.request_tx.lock() {
            Ok(tx) => tx.send(LspRequest::Hover(params)),
            Err(_) => return Ok(None),
        };

        let response_rx = self.response_rx.lock().expect("failed to lock response_rx");
        let ref response = response_rx.recv().expect("failed to get value from recv");
        if let LspResponse::Request(LspRequestResponse::Hover(data)) = response {
            return Ok(data.to_owned());
        }

        Ok(None)
    }

    async fn signature_help(&self, params: SignatureHelpParams) -> Result<Option<SignatureHelp>> {
        let _ = match self.request_tx.lock() {
            Ok(tx) => tx.send(LspRequest::SignatureHelp(params)),
            Err(_) => return Ok(None),
        };

        let response_rx = self.response_rx.lock().expect("failed to lock response_rx");
        let ref response = response_rx.recv().expect("failed to get value from recv");
        if let LspResponse::Request(LspRequestResponse::SignatureHelp(data)) = response {
            return Ok(data.to_owned());
        }

        Ok(None)
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
                .log_message(MessageType::WARNING, "Unsupported file opened")
                .await;
            return;
        };

        self.client
            .log_message(
                MessageType::WARNING,
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
        &clarity_lsp::lsp_types::MessageType::ERROR => tower_lsp::lsp_types::MessageType::ERROR,
        &clarity_lsp::lsp_types::MessageType::WARNING => tower_lsp::lsp_types::MessageType::WARNING,
        &clarity_lsp::lsp_types::MessageType::INFO => tower_lsp::lsp_types::MessageType::INFO,
        _ => tower_lsp::lsp_types::MessageType::LOG,
    }
}
