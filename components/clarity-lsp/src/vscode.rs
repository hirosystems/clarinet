use std::{collections::BTreeMap, mem::take};

use crate::utils::clarity_diagnostic_to_lsp_type;
use clarinet_deployments::generate_simnet_deployment_for_snippet;
use clarinet_files::FileLocation;
use clarity_repl::{clarity::SymbolicExpressionType, repl::ast::ContractAST};
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
use wasm_bindgen_futures::{spawn_local, JsFuture};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[derive(Serialize, Deserialize)]
pub struct VFSRequest {
    pub path: String,
}

#[derive(Serialize, Deserialize)]
pub struct FileEvent {
    pub path: String,
}

#[derive(Serialize, Deserialize)]
pub struct ASTEvent {
    pub path: String,
    pub ast: String,
}

#[derive(Serialize, Deserialize)]
pub struct CursorEvent {
    pub path: String,
    pub line: u32,
    pub char: u32,
}

#[derive(Default)]
#[wasm_bindgen]
pub struct ClarityLanguageServer {
    manifest: Option<String>,
    asts: BTreeMap<String, ContractAST>,
    client_diagnostic_tx: js_sys::Function,
    _client_notification_tx: js_sys::Function,
    client_request_tx: js_sys::Function,
}

#[wasm_bindgen]
impl ClarityLanguageServer {
    #[wasm_bindgen(constructor)]
    pub fn new(
        client_diagnostic_tx: js_sys::Function,
        client_notification_tx: js_sys::Function,
        client_request_tx: js_sys::Function,
    ) -> Self {
        Self {
            manifest: None,
            asts: BTreeMap::new(),
            client_diagnostic_tx: client_diagnostic_tx.clone(),
            _client_notification_tx: client_notification_tx.clone(),
            client_request_tx: client_request_tx.clone(),
        }
    }

    #[wasm_bindgen(js_name=onNotification)]
    pub fn on_notification(&mut self, method: &str, params: JsValue) {
        match method {
            Initialized::METHOD => {
                spawn_local(take(self).get_manifest());
                log("> initialized!");
            }

            DidOpenTextDocument::METHOD => {
                let DidOpenTextDocumentParams { text_document } = from_value(params).unwrap();
                log(&format!("> opened: {}", text_document.uri));
                self.get_and_send_diagnostic(&text_document);
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

                let _ast = self.get_and_send_diagnostic(&TextDocumentItem {
                    uri: text_document.uri,
                    text: text.unwrap(),
                    language_id: "clarity".to_string(),
                    version: 1,
                });
            }

            _ => log(&format!("unexpected notification ({})", method)),
        }
    }

    #[wasm_bindgen(js_name=onRequest)]
    pub fn on_request(&mut self, method: &str, params: JsValue, _token: JsValue) -> Option<String> {
        match method {
            "clarity/cursorMove" => {
                let CursorEvent { path, line, .. } = from_value(params).unwrap();

                let ast = self.asts.get(&path);
                if ast.is_none() {
                    return None;
                };
                let ast = ast.unwrap();
                let closest = ast
                    .expressions
                    .iter()
                    .clone()
                    .rev()
                    .find(|expr| expr.span.start_line <= line && expr.span.end_line >= line);

                if closest.is_none() {
                    return None;
                }
                let closest = closest.unwrap();
                if let SymbolicExpressionType::List(ref mut list) = closest.expr.clone() {
                    let func_type = list[0].expr.clone();
                    let func_name =
                        if let SymbolicExpressionType::List(ref mut list) = list[1].expr.clone() {
                            Some(list[0].expr.clone())
                        } else {
                            None
                        };
                    return Some(
                        json!({ "funcType": func_type, "funcName": func_name }).to_string(),
                    );
                }

                None
            }

            "clarity/getAst" => match from_value(params) {
                Ok(FileEvent { path }) => {
                    let ast = self.asts.get(&path);
                    match ast {
                        Some(ast) => Some(serde_json::to_string(&ast.expressions.clone()).unwrap()),
                        None => None,
                    }
                }
                Err(_) => {
                    log(&format!("> invalid params in getAst"));
                    None
                }
            },

            _ => {
                log(&format!("unexpected request ({})", method));
                None
            }
        }
    }

    fn get_and_send_diagnostic(&mut self, text_document: &TextDocumentItem) -> Option<ContractAST> {
        let location = FileLocation::Url {
            url: text_document.uri.clone(),
        };
        let name = "contract";
        let deployment =
            generate_simnet_deployment_for_snippet(&name, &text_document.text, &location);

        match deployment {
            Ok(result) => {
                let (_, (contract_id, artifacts)) = result;
                let ast = artifacts.asts.get(&contract_id);

                let dst = artifacts
                    .diags
                    .iter()
                    .flat_map(|(_, d)| d)
                    .fold(vec![], |mut acc, d| {
                        acc.push(clarity_diagnostic_to_lsp_type(d));
                        acc
                    });

                match self.client_diagnostic_tx.call1(
                    &JsValue::null(),
                    &to_value(&PublishDiagnosticsParams {
                        uri: Url::parse(&location.to_string()).unwrap(),
                        diagnostics: dst,
                        version: None,
                    })
                    .unwrap(),
                ) {
                    Ok(_) => (),
                    Err(err) => log(&format!("err: {:?}", err)),
                };

                match ast {
                    Some(ast) => {
                        self.asts
                            .insert(text_document.uri.path().to_string(), ast.clone());
                        Some(ast.clone())
                    }
                    None => None,
                }
            }
            Err(err) => {
                log(&format!("error: {}", err));
                None
            }
        }
    }

    pub async fn get_manifest(mut self) {
        let req = self
            .client_request_tx
            .call2(
                &JsValue::null(),
                &JsValue::from("vfs/readFile"),
                &to_value(&VFSRequest {
                    path: String::from("./Clarinet.toml"),
                })
                .unwrap(),
            )
            .unwrap();

        let response = JsFuture::from(js_sys::Promise::resolve(&req)).await;

        match response {
            Ok(manifest) => {
                self.manifest = from_value(manifest).unwrap();
                log(&format!("manifest: {:?}", self.manifest));
            }
            Err(_) => (),
        };
    }
}
