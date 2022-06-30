use crate::backend::{self, LspRequest};
use crate::utils;
use clarinet_files::{FileAccessor, FileLocation};
use js_sys::Function as JsFunction;
use lsp_types::{
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument,
        Initialized, Notification,
    },
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
    PublishDiagnosticsParams, TextDocumentItem, Url,
};
use serde_wasm_bindgen::{from_value as decode_from_wasm, to_value as encode_to_wasm};
use std::sync::mpsc::{self, channel, Sender};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen]
pub struct LspVscodeBridge {
    client_diagnostic_tx: JsFunction,
    client_notification_tx: JsFunction,
    client_request_tx: JsFunction,
    backend_command_tx: Sender<LspRequest>,
}

/// Entry point: the function `initialize_adapter_and_start_backend` is invoked from Javascript for constructing a `LspVscodeBridge`.
#[wasm_bindgen]
pub fn initialize_adapter_and_start_backend(
    client_diagnostic_tx: JsFunction,
    client_notification_tx: JsFunction,
    client_request_tx: JsFunction,
) -> LspVscodeBridge {
    let (backend_command_tx, backend_command_rx) = mpsc::channel();

    // Initialize `file_system_accessor` with whatever future is required
    let file_system_accessor = Box::new(VscodeFilesystemAccessor::new());

    // Pass `file_system_accessor` to `start_language_server`, which will
    // be passed whenever the LSP needs to read the content of a file.
    spawn_local(backend::start_language_server(
        backend_command_rx,
        Some(file_system_accessor),
    ));

    LspVscodeBridge {
        client_diagnostic_tx: client_diagnostic_tx,
        client_notification_tx: client_notification_tx,
        client_request_tx: client_request_tx,
        backend_command_tx,
    }
}

#[wasm_bindgen]
impl LspVscodeBridge {
    #[wasm_bindgen(js_name=onNotification)]
    pub fn on_notification(&self, method: &str, params: JsValue) {
        match method {
            Initialized::METHOD => {}
            DidOpenTextDocument::METHOD => {
                let params: DidOpenTextDocumentParams = match decode_from_wasm(params) {
                    Ok(params) => params,
                    _ => return,
                };
                let response_rx = if let Some(contract_location) =
                    utils::get_contract_location(&params.text_document.uri)
                {
                    let (response_tx, response_rx) = channel();
                    let _ = self
                        .backend_command_tx
                        .send(LspRequest::ContractOpened(contract_location, response_tx));
                    response_rx
                } else if let Some(manifest_location) =
                    utils::get_manifest_location(&params.text_document.uri)
                {
                    let (response_tx, response_rx) = channel();
                    let _ = self
                        .backend_command_tx
                        .send(LspRequest::ManifestOpened(manifest_location, response_tx));
                    response_rx
                } else {
                    log("Unsupported file opened");
                    return;
                };

                log("Command submitted to server, waiting for response");

                let mut aggregated_diagnostics = vec![];
                let mut notification = None;
                if let Ok(ref mut response) = response_rx.recv() {
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

pub struct VscodeFilesystemAccessor {}

impl VscodeFilesystemAccessor {
    pub fn new() -> VscodeFilesystemAccessor {
        VscodeFilesystemAccessor {}
    }
}

impl FileAccessor for VscodeFilesystemAccessor {
    fn read_file_content(&self, relative_path: String) -> Result<(FileLocation, String), String> {
        unimplemented!()
    }
}
