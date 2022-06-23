use crate::utils::convert_clarity_diagnotic_to_lsp_diagnostic;
use clarinet_deployments::generate_simnet_deployment_for_snippet;
use clarinet_files::FileLocation;
use lsp_types::{
    notification::{
        DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument,
        Initialized, Notification,
    },
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
    PublishDiagnosticsParams, TextDocumentItem, Url,
};
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::{from_value, to_value};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[derive(Serialize, Deserialize)]
pub struct VFSRequest {
    pub path: String,
}

#[wasm_bindgen]
pub struct ClarityLanguageServer {
    client_diagnostic_tx: js_sys::Function,
    _client_request_tx: js_sys::Function,
}

/// Public API exposed via WASM.
#[wasm_bindgen]
impl ClarityLanguageServer {
    #[wasm_bindgen(constructor)]
    pub fn new(
        client_diagnostic_tx: &js_sys::Function,
        client_request_tx: &js_sys::Function,
    ) -> Self {
        Self {
            client_diagnostic_tx: client_diagnostic_tx.clone(),
            _client_request_tx: client_request_tx.clone(),
        }
    }

    #[wasm_bindgen(js_name=onNotification)]
    pub fn on_notification(&mut self, method: &str, params: JsValue) {
        match method {
            Initialized::METHOD => {
                log("initialized!");
            }

            DidOpenTextDocument::METHOD => {
                let DidOpenTextDocumentParams { text_document } = from_value(params).unwrap();
                log(&format!("> opened: {}", text_document.uri));
                // let res = block_on(self._get_manifest());

                self.get_and_send_diagnostic(&text_document)
            }

            DidCloseTextDocument::METHOD => {
                let DidCloseTextDocumentParams { text_document } = from_value(params).unwrap();
                log(&format!("> closed: {}", text_document.uri));
            }

            DidChangeTextDocument::METHOD => {
                log("> changed");
            }

            DidSaveTextDocument::METHOD => {
                let DidSaveTextDocumentParams {
                    text_document,
                    text,
                } = from_value(params).unwrap();
                log(&format!("> saved: {}", text_document.uri));

                self.get_and_send_diagnostic(&TextDocumentItem {
                    uri: text_document.uri,
                    text: text.unwrap(),
                    language_id: "clarity".into(),
                    version: 1,
                });
            }

            _ => log(&format!("unexpected notification ({})", method)),
        }
    }

    fn get_and_send_diagnostic(&self, text_document: &TextDocumentItem) {
        let location = FileLocation::Url {
            url: text_document.uri.clone(),
        };
        let name = "contract";
        let deployment =
            generate_simnet_deployment_for_snippet(&name, &text_document.text, &location);

        match deployment {
            Ok(result) => {
                let (_, artifacts) = result;
                let iter = artifacts.diags.iter();
                let dst = iter.flat_map(|(_, d)| d).fold(vec![], |mut acc, d| {
                    acc.push(convert_clarity_diagnotic_to_lsp_diagnostic(d));
                    acc
                });

                let data = PublishDiagnosticsParams {
                    uri: Url::parse(&location.to_string()).unwrap(),
                    diagnostics: dst,
                    version: None,
                };

                let response = self
                    .client_diagnostic_tx
                    .call1(&JsValue::null(), &to_value(&data).unwrap());

                match response {
                    Ok(value) => log(&format!("ok: {:?}", value)),
                    Err(err) => log(&format!("err: {:?}", err)),
                }
            }
            Err(err) => log(&format!("error: {}", err)),
        }
    }

    #[wasm_bindgen(js_name=getManifest)]
    pub async fn _get_manifest(self) {
        let request = VFSRequest {
            path: String::from("./Clarinet.toml"),
        };

        let req = self
            ._client_request_tx
            .call2(
                &JsValue::null(),
                &JsValue::from("vfs/readFile"),
                &to_value(&request).unwrap(),
            )
            .unwrap();

        log(&format!("> req: {:?}", req));
        let response = JsFuture::from(js_sys::Promise::from(req)).await;
        log(&format!("> response: {:?}", response));
    }
}
