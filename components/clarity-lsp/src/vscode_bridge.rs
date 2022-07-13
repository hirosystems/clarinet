extern crate console_error_panic_hook;
use crate::backend::{self, LspRequestAsync, LspRequestSync};
use crate::state::EditorState;
use crate::utils::{self, get_contract_location, get_manifest_location, log};
use async_trait::*;
use clarinet_files::{FileAccessor, FileLocation, PerformFileAccess};
use js_sys::{Function as JsFunction, Promise};
use lsp_types::{
    notification::{DidOpenTextDocument, DidSaveTextDocument, Initialized, Notification},
    request::{Completion, Request},
    DidOpenTextDocumentParams, PublishDiagnosticsParams, Url,
};
use lsp_types::{CompletionParams, DidSaveTextDocumentParams};
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value as decode_from_wasm, to_value as encode_to_wasm};
use std::panic;
use std::sync::{Arc, RwLock};

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{future_to_promise, JsFuture};

#[wasm_bindgen]
pub struct LspVscodeBridge {
    editor_state: Arc<RwLock<EditorState>>,
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

        let editor_state = Arc::new(RwLock::new(EditorState::new()));
        LspVscodeBridge {
            editor_state,
            client_diagnostic_tx,
            _client_notification_tx,
            backend_to_client_tx,
        }
    }

    #[wasm_bindgen(js_name=onNotification)]
    pub fn notification_handler(&self, method: String, params: JsValue) -> Promise {
        log!("> notification method: {}", method);

        let file_accessor: Box<dyn FileAccessor> = Box::new(VscodeFilesystemAccessor::new(
            self.backend_to_client_tx.clone(),
        ));

        match method.as_str() {
            Initialized::METHOD => {
                log!("> initialized");
            }
            DidOpenTextDocument::METHOD => {
                let params: DidOpenTextDocumentParams = match decode_from_wasm(params) {
                    Ok(params) => params,
                    _ => return Promise::resolve(&JsValue::null()),
                };
                let uri = &params.text_document.uri;
                log!("> opened: {}", uri);

                let command = if let Some(contract_location) = get_contract_location(&uri) {
                    LspRequestAsync::ContractOpened(contract_location)
                } else if let Some(manifest_location) = get_manifest_location(&uri) {
                    LspRequestAsync::ManifestOpened(manifest_location)
                } else {
                    log!("Unsupported file opened");
                    return Promise::resolve(&JsValue::null());
                };

                let editor_state = self.editor_state.clone();
                let send_diagnostic = self.client_diagnostic_tx.clone();

                return future_to_promise(async move {
                    let mut result = match editor_state.try_write() {
                        Ok(mut state) => {
                            backend::process_command(command, &mut state, Some(&file_accessor))
                                .await
                        }
                        Err(_) => return Err(JsValue::from("unable to lock")),
                    };

                    let mut aggregated_diagnostics = vec![];
                    if let Ok(ref mut response) = result {
                        aggregated_diagnostics.append(&mut response.aggregated_diagnostics);
                    }

                    for (location, mut diags) in aggregated_diagnostics.into_iter() {
                        if let Ok(url) = Url::parse(&location.to_string()) {
                            let value = PublishDiagnosticsParams {
                                uri: url,
                                diagnostics: utils::clarity_diagnostics_to_lsp_type(&mut diags),
                                version: None,
                            };

                            let value = match encode_to_wasm(&value) {
                                Ok(value) => value,
                                Err(_) => return Err(JsValue::from("unable to encode value")),
                            };

                            let _ = send_diagnostic.call1(&JsValue::null(), &value);
                        }
                    }
                    Ok(JsValue::TRUE)
                });
            }

            DidSaveTextDocument::METHOD => {
                let params: DidSaveTextDocumentParams = match decode_from_wasm(params) {
                    Ok(params) => params,
                    _ => return Promise::resolve(&JsValue::null()),
                };
                let uri = &params.text_document.uri;
                log!("> saved: {}", uri);
                let command = if let Some(contract_location) = utils::get_contract_location(uri) {
                    LspRequestAsync::ContractChanged(contract_location)
                } else if let Some(manifest_location) = utils::get_manifest_location(uri) {
                    LspRequestAsync::ManifestChanged(manifest_location)
                } else {
                    log!("Unsupported file opened");
                    return Promise::resolve(&JsValue::null());
                };

                let editor_state = self.editor_state.clone();
                let send_diagnostic = self.client_diagnostic_tx.clone();

                return future_to_promise(async move {
                    let mut result = match editor_state.try_write() {
                        Ok(mut state) => {
                            backend::process_command(command, &mut state, Some(&file_accessor))
                                .await
                        }
                        Err(_) => return Err(JsValue::from("unable to lock")),
                    };

                    let mut aggregated_diagnostics = vec![];
                    if let Ok(ref mut response) = result {
                        aggregated_diagnostics.append(&mut response.aggregated_diagnostics);
                    }

                    for (location, mut diags) in aggregated_diagnostics.into_iter() {
                        if let Ok(url) = Url::parse(&location.to_string()) {
                            let value = PublishDiagnosticsParams {
                                uri: url,
                                diagnostics: utils::clarity_diagnostics_to_lsp_type(&mut diags),
                                version: None,
                            };

                            let value = match encode_to_wasm(&value) {
                                Ok(value) => value,
                                Err(_) => return Err(JsValue::from("unable to encode value")),
                            };

                            let _ = send_diagnostic.call1(&JsValue::null(), &value);
                        }
                    }
                    Ok(JsValue::TRUE)
                });
            }
            _ => {
                log!("unexpected notification ({})", method);
            }
        }
        return Promise::resolve(&JsValue::null());
    }

    #[wasm_bindgen(js_name=onRequest)]
    pub fn request_handler(&self, method: String, params: JsValue) -> JsValue {
        log!("> request method: {}", method);

        match method.as_str() {
            Completion::METHOD => {
                let params: CompletionParams = match decode_from_wasm(params) {
                    Ok(params) => params,
                    _ => return JsValue::null(),
                };
                let file_url = params.text_document_position.text_document.uri;

                let command = match utils::get_contract_location(&file_url) {
                    Some(location) => LspRequestSync::GetIntellisense(location),
                    _ => return JsValue::null(),
                };

                let result = match self.editor_state.try_read() {
                    Ok(editor_state) => backend::process_command_sync(command, &editor_state),
                    Err(_) => return JsValue::null(),
                };

                let value = match encode_to_wasm(&result.completion_items) {
                    Ok(value) => value,
                    Err(_) => {
                        log!("unable to encode value");
                        return JsValue::null();
                    }
                };
                return value;
            }
            _ => {
                log!("unexpected request ({})", method);
            }
        }

        return JsValue::null();
    }
}

#[derive(Serialize, Deserialize)]
pub struct VFSRequest {
    pub path: String,
}

pub struct VscodeFilesystemAccessor {
    client_request_tx: JsFunction,
}

impl VscodeFilesystemAccessor {
    pub fn new(client_request_tx: JsFunction) -> VscodeFilesystemAccessor {
        VscodeFilesystemAccessor { client_request_tx }
    }
}

#[async_trait]
impl FileAccessor for VscodeFilesystemAccessor {
    fn read_manifest_content(&self, manifest_location: FileLocation) -> PerformFileAccess {
        log!("reading manifest");
        let path = manifest_location.to_string();
        let req = self
            .client_request_tx
            .call2(
                &JsValue::null(),
                &JsValue::from("vfs/readFile"),
                &encode_to_wasm(&VFSRequest { path: path.clone() }).unwrap(),
            )
            .unwrap();

        return Box::pin(async move {
            let response = JsFuture::from(Promise::resolve(&req)).await;
            match response {
                Ok(manifest) => Ok((
                    FileLocation::from_url_string(&path).unwrap(),
                    decode_from_wasm(manifest).unwrap(),
                )),
                Err(_) => Err("error".into()),
            }
        });
    }

    fn read_contract_content(
        &self,
        manifest_location: FileLocation,
        relative_path: String,
    ) -> PerformFileAccess {
        log!("reading contract");
        let mut contract_location = manifest_location.get_parent_location().unwrap();
        let _ = contract_location.append_path(&relative_path);

        let req = self
            .client_request_tx
            .call2(
                &JsValue::null(),
                &JsValue::from("vfs/readFile"),
                &encode_to_wasm(&VFSRequest {
                    path: contract_location.to_string(),
                })
                .unwrap(),
            )
            .unwrap();

        return Box::pin(async move {
            let response = JsFuture::from(Promise::resolve(&req)).await;

            match response {
                Ok(manifest) => Ok((contract_location, decode_from_wasm(manifest).unwrap())),
                Err(_) => Err("error".into()),
            }
        });
    }
}
