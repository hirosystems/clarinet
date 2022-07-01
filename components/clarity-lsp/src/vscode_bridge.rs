extern crate console_error_panic_hook;
use std::panic;
use crate::backend::{self, LspRequest, LspResponse};
use crate::state::EditorState;
use crate::utils;
use crate::utils::spsc::{channel, Receiver, Sender};
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
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

use js_sys::Promise;
use wasm_bindgen_futures::{future_to_promise, spawn_local, JsFuture};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

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
pub async fn notification_handler(bridge: LspVscodeBridge, method: String, params: JsValue) -> LspVscodeBridge {
    
    log("command");

    let file_accessor: Box<dyn FileAccessor> = Box::new(VscodeFilesystemAccessor::new(
        bridge.backend_to_client_tx.clone(),
    ));

    match method.as_str() {
        Initialized::METHOD => {
            log("> initialized!");
        }
        DidOpenTextDocument::METHOD => {
            let params: DidOpenTextDocumentParams = match decode_from_wasm(params) {
                Ok(params) => params,
                _ => return bridge,
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
                return bridge
            };

            let mut result = 
            {
                let mut editor_state = bridge.editor_state.borrow_mut();
                backend::process_command(
                    command,
                    &mut editor_state,
                    Some(&file_accessor),
                )   
                .await
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
                        Err(e) => {
                            log("unable to encode value");
                            return bridge
                        }
                    };
                    return bridge;
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
    return bridge
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
        log("reading manifest");
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
        log("reading contract");
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
