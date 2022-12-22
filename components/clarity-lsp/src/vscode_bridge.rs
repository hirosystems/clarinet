extern crate console_error_panic_hook;
use crate::backend::{
    process_notification, process_request, EditorStateInput, LspNotification, LspRequest,
    LspRequestResponse,
};
use crate::state::EditorState;
use crate::utils::{
    clarity_diagnostics_to_lsp_type, get_contract_location, get_manifest_location, log,
};
use clarinet_files::{FileAccessor, WASMFileSystemAccessor};
use js_sys::{Function as JsFunction, Promise};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument,
    Initialized, Notification,
};
use lsp_types::request::{
    Completion, DocumentSymbolRequest, GotoDefinition, HoverRequest, Initialize, Request,
    SignatureHelpRequest,
};
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, PublishDiagnosticsParams, Url,
};
use serde::Serialize;
use serde_wasm_bindgen::{from_value as decode_from_js, to_value as encode_to_js, Serializer};
use std::panic;
use std::sync::{Arc, RwLock};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;

#[wasm_bindgen]
pub struct LspVscodeBridge {
    client_diagnostic_tx: JsFunction,
    _client_notification_tx: JsFunction,
    backend_to_client_tx: JsFunction,
    editor_state_lock: Arc<RwLock<EditorState>>,
}

#[wasm_bindgen]
impl LspVscodeBridge {
    #[wasm_bindgen(constructor)]
    pub fn new(
        client_diagnostic_tx: JsFunction,
        _client_notification_tx: JsFunction,
        backend_to_client_tx: JsFunction,
    ) -> LspVscodeBridge {
        panic::set_hook(Box::new(console_error_panic_hook::hook));

        LspVscodeBridge {
            client_diagnostic_tx,
            _client_notification_tx,
            backend_to_client_tx: backend_to_client_tx.clone(),
            editor_state_lock: Arc::new(RwLock::new(EditorState::new())),
        }
    }

    #[wasm_bindgen(js_name=onNotification)]
    pub fn notification_handler(&self, method: String, js_params: JsValue) -> Promise {
        let command = match method.as_str() {
            Initialized::METHOD => {
                return Promise::resolve(&JsValue::TRUE);
            }

            DidOpenTextDocument::METHOD => {
                let params: DidOpenTextDocumentParams = match decode_from_js(js_params) {
                    Ok(params) => params,
                    Err(err) => return Promise::reject(&JsValue::from(format!("error: {}", err))),
                };
                let uri = &params.text_document.uri;
                if let Some(contract_location) = get_contract_location(uri) {
                    LspNotification::ContractOpened(contract_location.clone())
                } else if let Some(manifest_location) = get_manifest_location(uri) {
                    LspNotification::ManifestOpened(manifest_location)
                } else {
                    return Promise::reject(&JsValue::from_str("Unsupported file opened"));
                }
            }

            DidSaveTextDocument::METHOD => {
                let params: DidSaveTextDocumentParams = match decode_from_js(js_params) {
                    Ok(params) => params,
                    Err(err) => return Promise::reject(&JsValue::from(format!("error: {}", err))),
                };
                let uri = &params.text_document.uri;

                if let Some(contract_location) = get_contract_location(uri) {
                    LspNotification::ContractSaved(contract_location)
                } else if let Some(manifest_location) = get_manifest_location(uri) {
                    LspNotification::ManifestSaved(manifest_location)
                } else {
                    return Promise::reject(&JsValue::from_str("Unsupported file opened"));
                }
            }

            DidChangeTextDocument::METHOD => {
                let params: DidChangeTextDocumentParams = match decode_from_js(js_params) {
                    Ok(params) => params,
                    Err(err) => return Promise::reject(&JsValue::from(format!("error: {}", err))),
                };
                let uri = &params.text_document.uri;

                if let Some(contract_location) = get_contract_location(uri) {
                    LspNotification::ContractChanged(
                        contract_location,
                        params.content_changes[0].text.to_string(),
                    )
                } else {
                    return Promise::resolve(&JsValue::FALSE);
                }
            }

            DidCloseTextDocument::METHOD => {
                let params: DidCloseTextDocumentParams = match decode_from_js(js_params) {
                    Ok(params) => params,
                    Err(err) => return Promise::reject(&JsValue::from(format!("error: {}", err))),
                };
                let uri = &params.text_document.uri;

                if let Some(contract_location) = get_contract_location(uri) {
                    LspNotification::ContractClosed(contract_location)
                } else {
                    return Promise::resolve(&JsValue::FALSE);
                }
            }

            _ => {
                #[cfg(debug_assertions)]
                log!("unexpected notification ({})", method);
                return Promise::resolve(&JsValue::FALSE);
            }
        };

        let mut editor_state_lock = EditorStateInput::RwLock(self.editor_state_lock.clone());
        let send_diagnostic = self.client_diagnostic_tx.clone();
        let file_accessor: Box<dyn FileAccessor> = Box::new(WASMFileSystemAccessor::new(
            self.backend_to_client_tx.clone(),
        ));

        future_to_promise(async move {
            let mut result =
                process_notification(command, &mut editor_state_lock, Some(&file_accessor)).await;

            let mut aggregated_diagnostics = vec![];
            if let Ok(ref mut response) = result {
                aggregated_diagnostics.append(&mut response.aggregated_diagnostics);
            }

            for (location, mut diags) in aggregated_diagnostics.into_iter() {
                if let Ok(uri) = Url::parse(&location.to_string()) {
                    send_diagnostic.call1(
                        &JsValue::NULL,
                        &encode_to_js(&PublishDiagnosticsParams {
                            uri,
                            diagnostics: clarity_diagnostics_to_lsp_type(&mut diags),
                            version: None,
                        })?,
                    )?;
                }
            }

            Ok(JsValue::TRUE)
        })
    }

    #[wasm_bindgen(js_name=onRequest)]
    pub fn request_handler(&self, method: String, js_params: JsValue) -> Result<JsValue, JsValue> {
        let serializer = Serializer::json_compatible();
        match method.as_str() {
            Initialize::METHOD => {
                let lsp_response = process_request(
                    LspRequest::Initialize(decode_from_js(js_params)?),
                    &mut EditorStateInput::RwLock(self.editor_state_lock.clone()),
                );
                if let LspRequestResponse::Initialize(response) = lsp_response {
                    return response.serialize(&serializer).map_err(|_| JsValue::NULL);
                }
            }

            Completion::METHOD => {
                let lsp_response = process_request(
                    LspRequest::Completion(decode_from_js(js_params)?),
                    &mut EditorStateInput::RwLock(self.editor_state_lock.clone()),
                );
                if let LspRequestResponse::CompletionItems(response) = lsp_response {
                    return response.serialize(&serializer).map_err(|_| JsValue::NULL);
                }
            }

            SignatureHelpRequest::METHOD => {
                let lsp_response = process_request(
                    LspRequest::SignatureHelp(decode_from_js(js_params)?),
                    &mut EditorStateInput::RwLock(self.editor_state_lock.clone()),
                );
                if let LspRequestResponse::SignatureHelp(response) = lsp_response {
                    return response.serialize(&serializer).map_err(|_| JsValue::NULL);
                }
            }

            GotoDefinition::METHOD => {
                let lsp_response = process_request(
                    LspRequest::Definition(decode_from_js(js_params)?),
                    &mut EditorStateInput::RwLock(self.editor_state_lock.clone()),
                );
                if let LspRequestResponse::Definition(response) = lsp_response {
                    return response.serialize(&serializer).map_err(|_| JsValue::NULL);
                }
            }

            DocumentSymbolRequest::METHOD => {
                let lsp_response = process_request(
                    LspRequest::DocumentSymbol(decode_from_js(js_params)?),
                    &mut EditorStateInput::RwLock(self.editor_state_lock.clone()),
                );
                if let LspRequestResponse::DocumentSymbol(response) = lsp_response {
                    return response.serialize(&serializer).map_err(|_| JsValue::NULL);
                }
            }

            HoverRequest::METHOD => {
                let lsp_response = process_request(
                    LspRequest::Hover(decode_from_js(js_params)?),
                    &mut EditorStateInput::RwLock(self.editor_state_lock.clone()),
                );
                if let LspRequestResponse::Hover(response) = lsp_response {
                    return response.serialize(&serializer).map_err(|_| JsValue::NULL);
                }
            }

            _ => {
                #[cfg(debug_assertions)]
                log!("unexpected request ({})", method);
            }
        }

        return Err(JsValue::NULL);
    }
}
