extern crate console_error_panic_hook;
use crate::backend::{
    process_notification, process_request, EditorStateInput, LspNotification, LspRequest,
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
use lsp_types::{
    request::{Completion, Request},
    CompletionParams, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DidSaveTextDocumentParams, PublishDiagnosticsParams, Url,
};
use serde_wasm_bindgen::{from_value as decode_from_js, to_value as encode_to_js};
use std::panic;
use std::sync::{Arc, RwLock};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use web_sys::console;

#[wasm_bindgen]
pub struct LspVscodeBridge {
    client_diagnostic_tx: JsFunction,
    _client_notification_tx: JsFunction,
    backend_to_client_tx: JsFunction,
    editor_state: EditorStateInput,
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
            backend_to_client_tx,
            editor_state: EditorStateInput::new(
                None,
                Some(Arc::new(RwLock::new(EditorState::new()))),
            ),
        }
    }

    #[wasm_bindgen(js_name=onNotification)]
    pub fn notification_handler(&self, method: String, js_params: JsValue) -> Promise {
        let file_accessor: Box<dyn FileAccessor> = Box::new(WASMFileSystemAccessor::new(
            self.backend_to_client_tx.clone(),
        ));

        match method.as_str() {
            Initialized::METHOD => {
                log!("clarity extension initialized");
            }

            DidOpenTextDocument::METHOD => {
                let params: DidOpenTextDocumentParams = match decode_from_js(js_params) {
                    Ok(params) => params,
                    Err(err) => return Promise::reject(&JsValue::from(format!("error: {}", err))),
                };
                let uri = params.text_document.uri;
                log!("> opened uri: {:}", &uri.path());

                let command = if let Some(contract_location) = get_contract_location(&uri) {
                    LspNotification::ContractOpened(contract_location.clone())
                } else if let Some(manifest_location) = get_manifest_location(&uri) {
                    LspNotification::ManifestOpened(manifest_location)
                } else {
                    return Promise::reject(&JsValue::from_str("Unsupported file opened"));
                };

                let mut editor_state = self.editor_state.clone();
                let send_diagnostic = self.client_diagnostic_tx.clone();

                return future_to_promise(async move {
                    let mut result =
                        process_notification(command, &mut editor_state, Some(&file_accessor))
                            .await;

                    let mut aggregated_diagnostics = vec![];

                    if let Ok(ref mut response) = result {
                        aggregated_diagnostics.append(&mut response.aggregated_diagnostics);
                    }

                    for (location, mut diags) in aggregated_diagnostics.into_iter() {
                        if let Ok(uri) = Url::parse(&location.to_string()) {
                            let value = PublishDiagnosticsParams {
                                uri,
                                diagnostics: clarity_diagnostics_to_lsp_type(&mut diags),
                                version: None,
                            };

                            send_diagnostic.call1(&JsValue::NULL, &encode_to_js(&value)?)?;
                        }
                    }

                    Ok(JsValue::TRUE)
                });
            }

            DidSaveTextDocument::METHOD => {
                let params: DidSaveTextDocumentParams = match decode_from_js(js_params) {
                    Ok(params) => params,
                    Err(err) => return Promise::reject(&JsValue::from(format!("error: {}", err))),
                };
                let uri = &params.text_document.uri;
                log!("> saved uri: {:}", &uri.path());

                let command = if let Some(contract_location) = get_contract_location(uri) {
                    LspNotification::ContractSaved(contract_location)
                } else if let Some(manifest_location) = get_manifest_location(uri) {
                    LspNotification::ManifestSaved(manifest_location)
                } else {
                    return Promise::reject(&JsValue::from_str("Unsupported file opened"));
                };

                let mut editor_state = self.editor_state.clone();
                let send_diagnostic = self.client_diagnostic_tx.clone();

                return future_to_promise(async move {
                    let mut result =
                        process_notification(command, &mut editor_state, Some(&file_accessor))
                            .await;

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
                });
            }

            DidChangeTextDocument::METHOD => {
                console::time_with_label("handle_did_change");
                let params: DidChangeTextDocumentParams = match decode_from_js(js_params) {
                    Ok(params) => params,
                    Err(err) => return Promise::reject(&JsValue::from(format!("error: {}", err))),
                };
                let uri = &params.text_document.uri;
                log!("> changed uri: {:}", &uri.path());

                let command = if let Some(contract_location) = get_contract_location(uri) {
                    LspNotification::ContractChanged(
                        contract_location,
                        params.content_changes[0].text.to_string(),
                    )
                } else {
                    return Promise::resolve(&JsValue::NULL);
                };

                let mut editor_state = self.editor_state.clone();
                let send_diagnostic = self.client_diagnostic_tx.clone();

                return future_to_promise(async move {
                    let result =
                        process_notification(command, &mut editor_state, Some(&file_accessor))
                            .await?;

                    if let Some((location, diagnostic)) = result.aggregated_diagnostics.get(0) {
                        if let Ok(uri) = Url::parse(&location.to_string()) {
                            send_diagnostic.call1(
                                &JsValue::NULL,
                                &encode_to_js(&PublishDiagnosticsParams {
                                    uri,
                                    diagnostics: clarity_diagnostics_to_lsp_type(diagnostic),
                                    version: None,
                                })?,
                            )?;
                        }
                    }

                    console::time_end_with_label("handle_did_change");
                    Ok(JsValue::TRUE)
                });
            }

            DidCloseTextDocument::METHOD => {
                let params: DidCloseTextDocumentParams = match decode_from_js(js_params) {
                    Ok(params) => params,
                    Err(err) => return Promise::reject(&JsValue::from(format!("error: {}", err))),
                };
                let uri = &params.text_document.uri;
                log!("> closed uri: {:}", &uri.path());

                let command = if let Some(contract_location) = get_contract_location(uri) {
                    LspNotification::ContractClosed(contract_location)
                } else {
                    return Promise::resolve(&JsValue::NULL);
                };

                let mut editor_state = self.editor_state.clone();

                return future_to_promise(async move {
                    process_notification(command, &mut editor_state, Some(&file_accessor)).await?;
                    Ok(JsValue::TRUE)
                });
            }

            _ => {
                #[cfg(debug_assertions)]
                log!("unexpected notification ({})", method);
            }
        }

        return Promise::resolve(&JsValue::NULL);
    }

    #[wasm_bindgen(js_name=onRequest)]
    pub fn request_handler(&self, method: String, js_params: JsValue) -> Result<JsValue, JsValue> {
        match method.as_str() {
            Completion::METHOD => {
                let params: CompletionParams = decode_from_js(js_params)?;
                let file_url = params.text_document_position.text_document.uri;
                let location = get_contract_location(&file_url).ok_or(JsValue::NULL)?;
                let command = LspRequest::GetIntellisense(location);
                let lsp_response = process_request(command, &self.editor_state);

                return encode_to_js(&lsp_response.completion_items).map_err(|_| JsValue::NULL);
            }

            _ => {
                #[cfg(debug_assertions)]
                log!("unexpected request ({})", method);
            }
        }

        return Err(JsValue::NULL);
    }
}
