use crate::lsp_types::MessageType;
use crate::state::{build_state, EditorState, ProtocolState};
use crate::types::{CompletionItem, CompletionItemKind};
use clarinet_files::{FileAccessor, FileLocation};
use clarity_repl::clarity::diagnostic::Diagnostic;
use serde_wasm_bindgen::to_value as encode_to_wasm;
use std::sync::mpsc::{Receiver, Sender};
use web_sys::console;

pub enum LspRequest {
    ManifestOpened(FileLocation, Sender<LspResponse>),
    ManifestChanged(FileLocation, Sender<LspResponse>),
    ContractOpened(FileLocation, Sender<LspResponse>),
    ContractChanged(FileLocation, Sender<LspResponse>),
    GetIntellisense(FileLocation, Sender<LspResponse>),
}

#[derive(Debug, PartialEq)]
pub struct LspResponse {
    pub aggregated_diagnostics: Vec<(FileLocation, Vec<Diagnostic>)>,
    pub notification: Option<(MessageType, String)>,
    pub completion_items: Vec<CompletionItem>,
}

impl LspResponse {
    pub fn default() -> LspResponse {
        LspResponse {
            aggregated_diagnostics: vec![],
            notification: None,
            completion_items: vec![],
        }
    }
}

impl LspResponse {
    pub fn error(message: &str) -> LspResponse {
        LspResponse {
            aggregated_diagnostics: vec![],
            completion_items: vec![],
            notification: Some((MessageType::ERROR, format!("Internal error: {}", message))),
        }
    }
}

pub async fn start_language_server<'a>(
    command_rx: Receiver<LspRequest>,
    file_accessor: Option<Box<dyn FileAccessor>>,
) {
    let mut editor_state = EditorState::new();

    let file_accessor_ref = match file_accessor {
        Some(ref file_accessor) => Some(file_accessor),
        None => None,
    };

    console::log_1(&encode_to_wasm("start_language_server").unwrap());

    loop {
        console::log_1(&encode_to_wasm("loop").unwrap());
        let command = match command_rx.recv() {
            Ok(command) => {
                console::log_1(&encode_to_wasm("Ok(command)").unwrap());
                command
            }
            Err(_e) => {
                console::log_2(
                    &encode_to_wasm("error").unwrap(),
                    &encode_to_wasm(&_e.to_string()).unwrap(),
                );
                break;
            }
        };

        console::log_1(&encode_to_wasm("command").unwrap());
        match command {
            LspRequest::GetIntellisense(contract_location, response_tx) => {
                let mut completion_items_src =
                    editor_state.get_completion_items_for_contract(&contract_location);
                let mut completion_items = vec![];
                // Little big detail: should we wrap the inserted_text with braces?
                let should_wrap = {
                    // let line = params.text_document_position.position.line;
                    // let char = params.text_document_position.position.character;
                    // let doc = params.text_document_position.text_document.uri;
                    //
                    // TODO(lgalabru): from there, we'd need to get the prior char
                    // and see if a parenthesis was opened. If not, we need to wrap.
                    // The LSP would need to update its local document cache, via
                    // the did_change method.
                    true
                };
                if should_wrap {
                    for mut item in completion_items_src.drain(..) {
                        match item.kind {
                            CompletionItemKind::Event
                            | CompletionItemKind::Function
                            | CompletionItemKind::Module
                            | CompletionItemKind::Class => {
                                item.insert_text =
                                    Some(format!("({})", item.insert_text.take().unwrap()));
                            }
                            _ => {}
                        }
                        completion_items.push(item);
                    }
                }

                let _ = response_tx.send(LspResponse {
                    aggregated_diagnostics: vec![],
                    notification: None,
                    completion_items,
                });
            }
            LspRequest::ManifestOpened(opened_manifest_location, response_tx) => {
                // The only reason why we're waiting for this kind of events, is building our initial state
                // if the system is initialized, move on.
                if editor_state
                    .protocols
                    .contains_key(&opened_manifest_location)
                {
                    let _ = response_tx.send(LspResponse::default());
                    continue;
                }

                // With this manifest_location, let's initialize our state.
                let mut protocol_state = ProtocolState::new();
                match build_state(
                    &opened_manifest_location,
                    &mut protocol_state,
                    file_accessor_ref,
                )
                .await
                {
                    Ok(_) => {
                        editor_state.index_protocol(opened_manifest_location, protocol_state);
                        let (aggregated_diagnostics, notification) =
                            editor_state.get_aggregated_diagnostics();
                        let _ = response_tx.send(LspResponse {
                            aggregated_diagnostics,
                            notification,
                            completion_items: vec![],
                        });
                    }
                    Err(e) => {
                        let _ = response_tx.send(LspResponse::error(&e));
                    }
                };
            }
            LspRequest::ContractOpened(contract_location, response_tx) => {
                // The only reason why we're waiting for this kind of events, is building our initial state
                // if the system is initialized, move on.
                let manifest_location = match contract_location.get_project_manifest_location() {
                    Ok(manifest_location) => manifest_location,
                    _ => {
                        let _ = response_tx.send(LspResponse::default());
                        continue;
                    }
                };

                if editor_state.protocols.contains_key(&manifest_location) {
                    let _ = response_tx.send(LspResponse::default());
                    continue;
                }

                // With this manifest_location, let's initialize our state.
                let mut protocol_state = ProtocolState::new();
                match build_state(&manifest_location, &mut protocol_state, file_accessor_ref).await
                {
                    Ok(_) => {
                        editor_state.index_protocol(manifest_location, protocol_state);
                        let (aggregated_diagnostics, notification) =
                            editor_state.get_aggregated_diagnostics();
                        let _ = response_tx.send(LspResponse {
                            aggregated_diagnostics,
                            notification,
                            completion_items: vec![],
                        });
                    }
                    Err(e) => {
                        let _ = response_tx.send(LspResponse::error(&e));
                    }
                };
            }
            LspRequest::ManifestChanged(manifest_location, response_tx) => {
                editor_state.clear_protocol(&manifest_location);

                // We will rebuild the entire state, without to try any optimizations for now
                let mut protocol_state = ProtocolState::new();
                match build_state(&manifest_location, &mut protocol_state, file_accessor_ref).await
                {
                    Ok(_) => {
                        editor_state.index_protocol(manifest_location, protocol_state);
                        let (aggregated_diagnostics, notification) =
                            editor_state.get_aggregated_diagnostics();
                        let _ = response_tx.send(LspResponse {
                            aggregated_diagnostics,
                            notification,
                            completion_items: vec![],
                        });
                    }
                    Err(e) => {
                        let _ = response_tx.send(LspResponse::error(&e));
                    }
                };
            }
            LspRequest::ContractChanged(contract_location, response_tx) => {
                let manifest_location = match editor_state
                    .clear_protocol_associated_with_contract(&contract_location)
                {
                    Some(manifest_location) => manifest_location,
                    None => match contract_location.get_project_manifest_location() {
                        Ok(manifest_location) => manifest_location,
                        _ => {
                            let _ = response_tx.send(LspResponse::default());
                            continue;
                        }
                    },
                };
                // TODO(lgalabru): introduce partial analysis
                // https://github.com/hirosystems/clarity-lsp/issues/98
                // We will rebuild the entire state, without trying any optimizations for now
                let mut protocol_state = ProtocolState::new();
                match build_state(&manifest_location, &mut protocol_state, file_accessor_ref).await
                {
                    Ok(_contracts_updates) => {
                        editor_state.index_protocol(manifest_location, protocol_state);
                        let (aggregated_diagnostics, notification) =
                            editor_state.get_aggregated_diagnostics();
                        let _ = response_tx.send(LspResponse {
                            aggregated_diagnostics,
                            notification,
                            completion_items: vec![],
                        });
                    }
                    Err(e) => {
                        let _ = response_tx.send(LspResponse::error(&e));
                    }
                };
            }
        }
    }
}
