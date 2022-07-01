use crate::lsp_types::MessageType;
use crate::state::{build_state, EditorState, ProtocolState};
use crate::types::{CompletionItem, CompletionItemKind};
use clarinet_files::{FileAccessor, FileLocation};
use clarity_repl::clarity::diagnostic::Diagnostic;
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{Receiver, Sender};

#[derive(Debug, Serialize, Deserialize)]
pub enum LspRequest {
    ManifestOpened(FileLocation),
    ManifestChanged(FileLocation),
    ContractOpened(FileLocation),
    ContractChanged(FileLocation),
    GetIntellisense(FileLocation),
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
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

pub async fn start_language_server(
    bridge_to_backend_rx: Receiver<LspRequest>,
    backend_to_bridge_tx: Sender<LspResponse>,
    file_accessor: Option<Box<dyn FileAccessor>>,
) {
    let mut editor_state = EditorState::new();

    let file_accessor_ref = match file_accessor {
        Some(ref file_accessor) => Some(file_accessor),
        None => None,
    };

    loop {
        let command = match bridge_to_backend_rx.recv() {
            Ok(command) => command,
            Err(_e) => {
                continue;
            }
        };

        let result = process_command(command, &mut editor_state, file_accessor_ref).await;
        if let Ok(lsp_response) = result {
            let _ = backend_to_bridge_tx.send(lsp_response);
        }
    }
}

pub async fn process_command(
    command: LspRequest,
    editor_state: &mut EditorState,
    file_accessor: Option<&Box<dyn FileAccessor>>,
) -> Result<LspResponse, String> {
    match command {
        LspRequest::GetIntellisense(contract_location) => {
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

            return Ok(LspResponse {
                aggregated_diagnostics: vec![],
                notification: None,
                completion_items,
            });
        }
        LspRequest::ManifestOpened(opened_manifest_location) => {
            // The only reason why we're waiting for this kind of events, is building our initial state
            // if the system is initialized, move on.
            if editor_state
                .protocols
                .contains_key(&opened_manifest_location)
            {
                return Ok(LspResponse::default());
            }

            // With this manifest_location, let's initialize our state.
            let mut protocol_state = ProtocolState::new();
            match build_state(
                &opened_manifest_location,
                &mut protocol_state,
                file_accessor,
            )
            .await
            {
                Ok(_) => {
                    editor_state.index_protocol(opened_manifest_location, protocol_state);
                    let (aggregated_diagnostics, notification) =
                        editor_state.get_aggregated_diagnostics();
                    return Ok(LspResponse {
                        aggregated_diagnostics,
                        notification,
                        completion_items: vec![],
                    });
                }
                Err(e) => return Ok(LspResponse::error(&e)),
            };
        }
        LspRequest::ContractOpened(contract_location) => {
            // The only reason why we're waiting for this kind of events, is building our initial state
            // if the system is initialized, move on.
            let manifest_location = match contract_location.get_project_manifest_location() {
                Ok(manifest_location) => manifest_location,
                _ => {
                    return Ok(LspResponse::default());
                }
            };

            if editor_state.protocols.contains_key(&manifest_location) {
                return Ok(LspResponse::default());
            }

            // With this manifest_location, let's initialize our state.
            let mut protocol_state = ProtocolState::new();
            match build_state(&manifest_location, &mut protocol_state, file_accessor).await {
                Ok(_) => {
                    editor_state.index_protocol(manifest_location, protocol_state);
                    let (aggregated_diagnostics, notification) =
                        editor_state.get_aggregated_diagnostics();
                    return Ok(LspResponse {
                        aggregated_diagnostics,
                        notification,
                        completion_items: vec![],
                    });
                }
                Err(e) => return Ok(LspResponse::error(&e)),
            };
        }
        LspRequest::ManifestChanged(manifest_location) => {
            editor_state.clear_protocol(&manifest_location);

            // We will rebuild the entire state, without to try any optimizations for now
            let mut protocol_state = ProtocolState::new();
            match build_state(&manifest_location, &mut protocol_state, file_accessor).await {
                Ok(_) => {
                    editor_state.index_protocol(manifest_location, protocol_state);
                    let (aggregated_diagnostics, notification) =
                        editor_state.get_aggregated_diagnostics();
                    return Ok(LspResponse {
                        aggregated_diagnostics,
                        notification,
                        completion_items: vec![],
                    });
                }
                Err(e) => return Ok(LspResponse::error(&e)),
            };
        }
        LspRequest::ContractChanged(contract_location) => {
            let manifest_location =
                match editor_state.clear_protocol_associated_with_contract(&contract_location) {
                    Some(manifest_location) => manifest_location,
                    None => match contract_location.get_project_manifest_location() {
                        Ok(manifest_location) => manifest_location,
                        _ => {
                            return Ok(LspResponse::default());
                        }
                    },
                };
            // TODO(lgalabru): introduce partial analysis
            // https://github.com/hirosystems/clarity-lsp/issues/98
            // We will rebuild the entire state, without trying any optimizations for now
            let mut protocol_state = ProtocolState::new();
            match build_state(&manifest_location, &mut protocol_state, file_accessor).await {
                Ok(_contracts_updates) => {
                    editor_state.index_protocol(manifest_location, protocol_state);
                    let (aggregated_diagnostics, notification) =
                        editor_state.get_aggregated_diagnostics();
                    return Ok(LspResponse {
                        aggregated_diagnostics,
                        notification,
                        completion_items: vec![],
                    });
                }
                Err(e) => return Ok(LspResponse::error(&e)),
            };
        }
    }
}
