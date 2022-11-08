extern crate console_error_panic_hook;
use crate::backend::{get_intellisense, process_notification, LspNotification};
use crate::common::active_state::ActiveEditorState;
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
    request::{Completion, HoverRequest, Request},
    CompletionParams, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DidSaveTextDocumentParams, HoverParams, PublishDiagnosticsParams,
    Url,
};
use serde_wasm_bindgen::{from_value as decode_from_js, to_value as encode_to_js};
use std::panic;
use std::sync::{Arc, RwLock};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::future_to_promise;
use web_sys::console;

#[wasm_bindgen]
pub struct LspVscodeBridge {
    editor_state_lock: Arc<RwLock<EditorState>>,
    active_editor_state_lock: Arc<RwLock<ActiveEditorState>>,
    client_diagnostic_tx: JsFunction,
    _client_notification_tx: JsFunction,
    backend_to_client_tx: JsFunction,
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

        let editor_state_lock = Arc::new(RwLock::new(EditorState::new()));
        let active_editor_state_lock = Arc::new(RwLock::new(ActiveEditorState::new()));
        LspVscodeBridge {
            editor_state_lock,
            active_editor_state_lock,
            client_diagnostic_tx,
            _client_notification_tx,
            backend_to_client_tx,
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
                log!("> opened uri: {:?}", &uri);

                let (command, contract_location) =
                    if let Some(contract_location) = get_contract_location(&uri) {
                        (
                            LspNotification::ContractOpened(contract_location.clone()),
                            Some(contract_location),
                        )
                    } else if let Some(manifest_location) = get_manifest_location(&uri) {
                        (LspNotification::ManifestOpened(manifest_location), None)
                    } else {
                        return Promise::reject(&JsValue::from_str("Unsupported file opened"));
                    };

                let editor_state_lock = self.editor_state_lock.clone();
                let active_editor_state_lock = self.active_editor_state_lock.clone();
                let send_diagnostic = self.client_diagnostic_tx.clone();

                return future_to_promise(async move {
                    match contract_location {
                        Some(contract_location) => {
                            let mut active_editor_state = active_editor_state_lock
                                .try_write()
                                .map_err(|_| JsValue::FALSE)?;
                            let manifest_location = contract_location
                                .get_project_manifest_location(Some(&file_accessor))
                                .await?;

                            if active_editor_state
                                .contracts
                                .contains_key(&contract_location)
                            {
                                ()
                            }
                            let source = file_accessor
                                .read_file(contract_location.to_string())
                                .await?;

                            active_editor_state.insert_contract(
                                contract_location,
                                manifest_location,
                                source.as_str(),
                            );
                        }
                        None => (),
                    }

                    let mut editor_state = editor_state_lock
                        .try_write()
                        .map_err(|_| JsValue::from("unable to lock editor_state"))?;

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

                let command = if let Some(contract_location) = get_contract_location(uri) {
                    LspNotification::ContractChanged(contract_location)
                } else if let Some(manifest_location) = get_manifest_location(uri) {
                    LspNotification::ManifestChanged(manifest_location)
                } else {
                    return Promise::reject(&JsValue::from_str("Unsupported file opened"));
                };

                let editor_state_lock = self.editor_state_lock.clone();
                let send_diagnostic = self.client_diagnostic_tx.clone();

                return future_to_promise(async move {
                    // @todo(hugo) - save in active_editor_state

                    let mut editor_state = editor_state_lock
                        .try_write()
                        .map_err(|_| JsValue::from("unable to lock editor_state"))?;
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

            DidCloseTextDocument::METHOD => {
                let params: DidCloseTextDocumentParams = match decode_from_js(js_params) {
                    Ok(params) => params,
                    Err(err) => return Promise::reject(&JsValue::from(format!("error: {}", err))),
                };
                let uri = params.text_document.uri;

                let active_editor_state_lock = self.active_editor_state_lock.clone();

                return future_to_promise(async move {
                    let location = get_contract_location(&uri).ok_or(JsValue::FALSE)?;
                    let mut active_editor_state = active_editor_state_lock
                        .try_write()
                        .map_err(|_| JsValue::from("unable to lock active_editor_state"))?;

                    active_editor_state.contracts.remove(&location);
                    Ok(JsValue::TRUE)
                });
            }

            DidChangeTextDocument::METHOD => {
                console::time_with_label("handle_did_change");
                let params: DidChangeTextDocumentParams = match decode_from_js(js_params) {
                    Ok(params) => params,
                    Err(err) => return Promise::reject(&JsValue::from(format!("error: {}", err))),
                };
                let uri = params.text_document.uri;
                log!("> changed uri: {:?}", uri);

                let contract_location = match get_contract_location(&uri) {
                    Some(location) => location,
                    None => {
                        return Promise::resolve(&JsValue::FALSE);
                    }
                };
                let active_editor_state_lock = self.active_editor_state_lock.clone();
                let send_diagnostic = self.client_diagnostic_tx.clone();

                return future_to_promise(async move {
                    let mut active_editor_state = active_editor_state_lock
                        .try_write()
                        .map_err(|_| JsValue::from("unable to lock active_editor_state"))?;

                    // @todo: sometimes it's not saved in the contract, call insert_contract
                    active_editor_state
                        .update_contract(&contract_location, &params.content_changes[0].text)?;

                    match &active_editor_state
                        .contracts
                        .get(&contract_location)
                        .unwrap()
                        .diagnostic
                    {
                        Some(diagnostic) => {
                            log!("> diagnostic: {:?}", &diagnostic);
                            let value = PublishDiagnosticsParams {
                                uri,
                                diagnostics: clarity_diagnostics_to_lsp_type(&mut vec![
                                    diagnostic.clone()
                                ]),
                                version: None,
                            };

                            send_diagnostic.call1(&JsValue::NULL, &encode_to_js(&value)?)?;
                        }
                        None => (),
                    }

                    console::time_end_with_label("handle_did_change");
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
                let uri = params.text_document_position.text_document.uri;
                let contract_location = get_contract_location(&uri).ok_or(JsValue::NULL)?;
                let editor_state = self
                    .editor_state_lock
                    .try_read()
                    .map_err(|_| JsValue::NULL)?;
                let completion_items = get_intellisense(&editor_state, &contract_location);

                return encode_to_js(&completion_items).map_err(|_| JsValue::NULL);
            }

            HoverRequest::METHOD => {
                let params: HoverParams = decode_from_js(js_params)?;
                let uri = params.text_document_position_params.text_document.uri;
                let contract_location = match get_contract_location(&uri) {
                    Some(location) => location,
                    None => {
                        return Ok(JsValue::NULL);
                    }
                };
                let position = params.text_document_position_params.position;

                let active_editor_state = self
                    .active_editor_state_lock
                    .try_read()
                    .map_err(|_| JsValue::NULL)?;

                return match active_editor_state.get_hover_data(&contract_location, &position) {
                    Some(data) => encode_to_js(&data).map_err(|_| JsValue::NULL),
                    None => Ok(JsValue::NULL),
                };
            }

            _ => {
                #[cfg(debug_assertions)]
                log!("unexpected request ({})", method);
            }
        }

        return Err(JsValue::NULL);
    }
}
