extern crate console_error_panic_hook;
use crate::backend::{self, LspRequest};
use crate::state::EditorState;
use crate::utils::{self, log};
use async_trait::*;
use clarinet_files::{FileAccessor, FileLocation, PerformFileAccess};
use js_sys::Function as JsFunction;
use lsp_types::{
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument,
        Initialized, Notification,
    },
    request::{Completion, Request},
    DidOpenTextDocumentParams, PublishDiagnosticsParams, Url,
};
use lsp_types::{CompletionParams, DidSaveTextDocumentParams};
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value as decode_from_wasm, to_value as encode_to_wasm};
use std::cell::RefCell;
use std::panic;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen]
pub struct LspVscodeBridge {
    editor_state: Rc<RefCell<EditorState>>,
    client_diagnostic_tx: JsFunction,
    client_notification_tx: JsFunction,
    backend_to_client_tx: JsFunction,
}

#[wasm_bindgen]
impl LspVscodeBridge {
    #[wasm_bindgen(constructor)]
    pub fn new(
        client_diagnostic_tx: JsFunction,
        client_notification_tx: JsFunction,
        backend_to_client_tx: JsFunction,
    ) -> LspVscodeBridge {
        panic::set_hook(Box::new(console_error_panic_hook::hook));

        let editor_state = Rc::new(RefCell::new(EditorState::new()));
        LspVscodeBridge {
            editor_state,
            client_diagnostic_tx,
            client_notification_tx,
            backend_to_client_tx,
        }
    }
}

#[wasm_bindgen(js_name=onNotification)]
pub async fn notification_handler(
    bridge: LspVscodeBridge,
    method: String,
    params: JsValue,
) -> LspVscodeBridge {
    log!("> method: {}", method);

    let file_accessor: Box<dyn FileAccessor> = Box::new(VscodeFilesystemAccessor::new(
        bridge.backend_to_client_tx.clone(),
    ));

    match method.as_str() {
        Initialized::METHOD => {
            log!("> initialized!");
        }
        DidOpenTextDocument::METHOD => {
            let params: DidOpenTextDocumentParams = match decode_from_wasm(params) {
                Ok(params) => params,
                _ => return bridge,
            };
            log!("> opened: {}", params.text_document.uri);

            let command = if let Some(contract_location) =
                utils::get_contract_location(&params.text_document.uri)
            {
                LspRequest::ContractOpened(contract_location)
            } else if let Some(manifest_location) =
                utils::get_manifest_location(&params.text_document.uri)
            {
                LspRequest::ManifestOpened(manifest_location)
            } else {
                log!("Unsupported file opened");
                return bridge;
            };

            let mut result = {
                let mut editor_state = bridge.editor_state.borrow_mut();
                backend::process_command(command, &mut editor_state, Some(&file_accessor)).await
            };

            let mut aggregated_diagnostics = vec![];
            let mut notification = None;
            if let Ok(ref mut response) = result {
                aggregated_diagnostics.append(&mut response.aggregated_diagnostics);
                notification = response.notification.take();
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
                        Err(_) => {
                            log!("unable to encode value");
                            return bridge;
                        }
                    };

                    bridge
                        .client_diagnostic_tx
                        .call1(&JsValue::null(), &value)
                        .unwrap();
                }
            }
            return bridge;

            // TODO: display eventual notifications coming from the backend
            // if let Some((level, message)) = notification {
            //     self.client
            //         .show_message(message_level_type_to_tower_lsp_type(&level), message)
            //         .await;
            // }
        }
        DidCloseTextDocument::METHOD => {
            // See LspNativeBridge::did_close
        }
        DidChangeTextDocument::METHOD => {
            // See LspNativeBridge::completion
        }

        DidSaveTextDocument::METHOD => {
            let params: DidSaveTextDocumentParams = match decode_from_wasm(params) {
                Ok(params) => params,
                _ => return bridge,
            };
            log!("> saved: {}", params.text_document.uri);

            let command = if let Some(contract_location) =
                utils::get_contract_location(&params.text_document.uri)
            {
                LspRequest::ContractChanged(contract_location)
            } else if let Some(manifest_location) =
                utils::get_manifest_location(&params.text_document.uri)
            {
                LspRequest::ManifestChanged(manifest_location)
            } else {
                log!("Unsupported file opened");
                return bridge;
            };

            let mut result = {
                let mut editor_state = bridge.editor_state.borrow_mut();
                backend::process_command(command, &mut editor_state, Some(&file_accessor)).await
            };

            let mut aggregated_diagnostics = vec![];
            // let mut notification = None;
            if let Ok(ref mut response) = result {
                aggregated_diagnostics.append(&mut response.aggregated_diagnostics);
                // notification = response.notification.take();
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
                        Err(_) => {
                            log!("unable to encode value");
                            return bridge;
                        }
                    };

                    bridge
                        .client_diagnostic_tx
                        .call1(&JsValue::null(), &value)
                        .unwrap();
                }
            }

            return bridge;
        }
        _ => log!("unexpected notification ({})", method),
    }
    return bridge;
}

#[wasm_bindgen(js_name=onRequest)]
pub async fn request_handler(
    bridge: LspVscodeBridge,
    method: String,
    params: JsValue,
) -> Option<JsValue> {
    log!("> method: {}", method);
    let file_accessor: Box<dyn FileAccessor> = Box::new(VscodeFilesystemAccessor::new(
        bridge.backend_to_client_tx.clone(),
    ));

    match method.as_str() {
        Completion::METHOD => {
            let params: CompletionParams = match decode_from_wasm(params) {
                Ok(params) => params,
                _ => return None,
            };
            let file_url = params.text_document_position.text_document.uri;
            log!("> completions: {}", file_url.to_string());
            let mut editor_state = bridge.editor_state.borrow_mut();
            log!(
                "> editor_state: {:?}",
                editor_state.get_aggregated_diagnostics()
            );

            let command = match utils::get_contract_location(&file_url) {
                Some(location) => LspRequest::GetIntellisense(location),
                _ => return None,
            };

            let result = {
                backend::process_command(command, &mut editor_state, Some(&file_accessor)).await
            };

            match result {
                Ok(result) => return Some(encode_to_wasm(&result.completion_items).unwrap()),
                Err(_) => {
                    return None;
                }
            }
        }
        _ => log!("unexpected request ({})", method),
    }

    return None;
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
            let response = JsFuture::from(js_sys::Promise::resolve(&req)).await;
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
            let response = JsFuture::from(js_sys::Promise::resolve(&req)).await;

            match response {
                Ok(manifest) => Ok((contract_location, decode_from_wasm(manifest).unwrap())),
                Err(_) => Err("error".into()),
            }
        });
    }
}
