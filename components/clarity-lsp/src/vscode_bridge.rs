use crate::backend::{self, LspRequest, LspResponse};
use crate::state::EditorState;
use crate::utils;
use async_trait::*;
use clarinet_files::{FileAccessor, FileLocation, PerformFileAccess};
use js_sys::Function as JsFunction;
use lsp_types::{
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument,
        Initialized, Notification,
    },
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
    PublishDiagnosticsParams, TextDocumentItem, Url,
};
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value as decode_from_wasm, to_value as encode_to_wasm};
use std::sync::mpsc::{self, channel, Sender};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{spawn_local, JsFuture};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
pub struct LspVscodeBridge {
    editor_state: EditorState,
    client_diagnostic_tx: JsFunction,
    client_notification_tx: JsFunction,
    backend_to_client_tx: JsFunction,
    file_accessor: Box<dyn FileAccessor>,
}

/// Entry point: the function `initialize_adapter_and_start_backend` is invoked from Javascript for constructing a `LspVscodeBridge`.
#[wasm_bindgen(js_name=initializeAdapterAndStartBackend)]
pub fn initialize_adapter_and_start_backend(
    client_diagnostic_tx: JsFunction,
    client_notification_tx: JsFunction,
    backend_to_client_tx: JsFunction,
) -> LspVscodeBridge {
    LspVscodeBridge::new(
        client_diagnostic_tx,
        client_notification_tx,
        backend_to_client_tx,
    )
}

#[wasm_bindgen]
impl LspVscodeBridge {
    #[wasm_bindgen(constructor)]
    pub fn new(
        client_diagnostic_tx: JsFunction,
        client_notification_tx: JsFunction,
        backend_to_client_tx: JsFunction,
    ) -> LspVscodeBridge {
        let file_accessor = VscodeFilesystemAccessor::new(backend_to_client_tx.clone());
        LspVscodeBridge {
            editor_state: EditorState::new(),
            client_diagnostic_tx,
            client_notification_tx,
            backend_to_client_tx,
            file_accessor: Box::new(file_accessor),
        }
    }

    #[wasm_bindgen(js_name=onNotification)]
    pub async fn on_notification(mut self, method: String, params: JsValue) {
        log("command");
        match method.as_str() {
            Initialized::METHOD => {
                log("> initialized!");
            }
            DidOpenTextDocument::METHOD => {
                let params: DidOpenTextDocumentParams = match decode_from_wasm(params) {
                    Ok(params) => params,
                    _ => return,
                };
                log(&format!("> opened: {}", params.text_document.uri));

                let command = if let Some(contract_location) =
                    utils::get_contract_location(&params.text_document.uri)
                {
                    LspRequest::ContractOpened(contract_location)
                } else if let Some(manifest_location) =
                    utils::get_manifest_location(&params.text_document.uri)
                {
                    LspRequest::ManifestOpened(manifest_location)
                } else {
                    log("Unsupported file opened");
                    return;
                };
                let mut result = backend::process_command(
                    command,
                    &mut self.editor_state,
                    Some(&self.file_accessor),
                )
                .await;

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
                            Err(e) => {
                                log("unable to encode value");
                                return;
                            }
                        };

                        match self.client_diagnostic_tx.call1(&JsValue::null(), &value) {
                            Err(e) => {
                                log("unable to publish diagnostics");
                            }
                            _ => {}
                        };
                    }
                }
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
                // See LspNativeBridge::did_save
            }
            _ => log(&format!("unexpected notification ({})", method)),
        }
    }

    #[wasm_bindgen(js_name=onRequest)]
    pub fn on_request(&mut self, method: &str, params: JsValue, _token: JsValue) -> Option<String> {
        match method {
            "events/v1/cursorMoved" => None,
            "requests/v1/getAst" => None,
            _ => {
                log(&format!("unexpected request ({})", method));
                None
            }
        }
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
        let path = relative_path.clone();
        let req = self
            .client_request_tx
            .call2(
                &JsValue::null(),
                &JsValue::from("vfs/readFile"),
                &encode_to_wasm(&VFSRequest { path }).unwrap(),
            )
            .unwrap();

        return Box::pin(async move {
            let response = JsFuture::from(js_sys::Promise::resolve(&req)).await;
            match response {
                Ok(manifest) => Ok((
                    FileLocation::from_url_string(&String::from(relative_path)).unwrap(),
                    decode_from_wasm(manifest).unwrap(),
                )),
                Err(_) => Err("error".into()),
            }
        });
    }
}
