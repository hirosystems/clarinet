use crate::lsp_types::MessageType;
use crate::state::{build_state, EditorState, ProtocolState};
use crate::types::{CompletionItem, CompletionItemKind};
use clarinet_files::{FileAccessor, FileLocation};
use clarity_repl::clarity::diagnostic::Diagnostic;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum LspNotification {
    ManifestOpened(FileLocation),
    ManifestChanged(FileLocation),
    ContractOpened(FileLocation),
    ContractChanged(FileLocation, Option<String>),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LspRequest {
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

pub async fn process_notification(
    command: LspNotification,
    editor_state: &mut EditorState,
    file_accessor: Option<&Box<dyn FileAccessor>>,
) -> Result<LspResponse, String> {
    match command {
        LspNotification::ManifestOpened(opened_manifest_location) => {
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
                None,
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
        LspNotification::ContractOpened(contract_location) => {
            // This event can be ignored if the contract is already in the state
            if editor_state
                .contracts_lookup
                .contains_key(&contract_location)
            {
                return Ok(LspResponse::default());
            }

            let manifest_location = contract_location
                .get_project_manifest_location(file_accessor)
                .await?;

            let mut protocol_state = ProtocolState::new();
            match build_state(&manifest_location, &mut protocol_state, file_accessor, None).await {
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
        LspNotification::ManifestChanged(manifest_location) => {
            editor_state.clear_protocol(&manifest_location);

            // We will rebuild the entire state, without to try any optimizations for now
            let mut protocol_state = ProtocolState::new();
            match build_state(&manifest_location, &mut protocol_state, file_accessor, None).await {
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
        LspNotification::ContractChanged(contract_location, overwrite_source) => {
            let manifest_location =
                match editor_state.clear_protocol_associated_with_contract(&contract_location) {
                    Some(manifest_location) => manifest_location,
                    None => {
                        contract_location
                            .get_project_manifest_location(file_accessor)
                            .await?
                    }
                };

            // TODO(lgalabru): introduce partial analysis
            // https://github.com/hirosystems/clarity-lsp/issues/98
            // We will rebuild the entire state, without trying any optimizations for now
            let mut protocol_state = ProtocolState::new();
            match build_state(
                &manifest_location,
                &mut protocol_state,
                file_accessor,
                match overwrite_source {
                    Some(overwrite_source) => Some((contract_location, overwrite_source)),
                    _ => None,
                },
            )
            .await
            {
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

pub fn process_request(command: LspRequest, editor_state: &EditorState) -> LspResponse {
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

            LspResponse {
                aggregated_diagnostics: vec![],
                notification: None,
                completion_items,
            }
        }
    }
}
