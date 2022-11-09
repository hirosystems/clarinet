use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::lsp_types::MessageType;
use crate::state::{build_state, EditorState, ProtocolState};
use crate::types::{CompletionItem, CompletionItemKind};
use clarinet_files::{FileAccessor, FileLocation};
use clarity_repl::clarity::diagnostic::Diagnostic;
use clarity_repl::repl::DEFAULT_CLARITY_VERSION;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct EditorStateInput {
    editor_state: Option<EditorState>,
    editor_state_lock: Option<Arc<RwLock<EditorState>>>,
}

impl EditorStateInput {
    pub fn new(
        editor_state: Option<EditorState>,
        editor_state_lock: Option<Arc<RwLock<EditorState>>>,
    ) -> Self {
        EditorStateInput {
            editor_state,
            editor_state_lock,
        }
    }

    pub fn try_read(&self) -> Result<RwLockReadGuard<EditorState>, String> {
        match (self.editor_state.as_ref(), self.editor_state_lock.as_ref()) {
            (Some(_editor_state), None) => unimplemented!(),
            (None, Some(editor_state_lock)) => match editor_state_lock.try_read() {
                Ok(editor_state) => Ok(editor_state),
                Err(_) => Err("failed to read editor_state".to_string()),
            },
            _ => {
                unreachable!();
            }
        }
    }

    pub fn try_write(&self) -> Result<RwLockWriteGuard<EditorState>, String> {
        match (self.editor_state.as_ref(), self.editor_state_lock.as_ref()) {
            (Some(_editor_state), None) => unimplemented!(),
            (None, Some(editor_state_lock)) => match editor_state_lock.try_write() {
                Ok(editor_state) => Ok(editor_state),
                Err(_) => Err("failed to write editor_state".to_string()),
            },
            _ => {
                unreachable!();
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LspNotification {
    ManifestOpened(FileLocation),
    ManifestSaved(FileLocation),
    ContractOpened(FileLocation),
    ContractSaved(FileLocation),
    ContractChanged(FileLocation, String),
    ContractClosed(FileLocation),
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct LspNotificationResponse {
    pub aggregated_diagnostics: Vec<(FileLocation, Vec<Diagnostic>)>,
    pub notification: Option<(MessageType, String)>,
}

impl LspNotificationResponse {
    pub fn default() -> LspNotificationResponse {
        LspNotificationResponse {
            aggregated_diagnostics: vec![],
            notification: None,
        }
    }

    pub fn error(message: &str) -> LspNotificationResponse {
        LspNotificationResponse {
            aggregated_diagnostics: vec![],
            notification: Some((MessageType::ERROR, format!("Internal error: {}", message))),
        }
    }
}

pub async fn process_notification(
    command: LspNotification,
    editor_state: &EditorStateInput,
    file_accessor: Option<&Box<dyn FileAccessor>>,
) -> Result<LspNotificationResponse, String> {
    match command {
        LspNotification::ManifestOpened(manifest_location) => {
            {
                // Only build the initial state if it does not exist
                if editor_state
                    .try_read()?
                    .protocols
                    .contains_key(&manifest_location)
                {
                    return Ok(LspNotificationResponse::default());
                }
            }

            // With this manifest_location, let's initialize our state.
            let mut protocol_state = ProtocolState::new();
            match build_state(&manifest_location, &mut protocol_state, file_accessor).await {
                Ok(_) => {
                    editor_state
                        .try_write()?
                        .index_protocol(manifest_location, protocol_state);
                    let (aggregated_diagnostics, notification) =
                        editor_state.try_read()?.get_aggregated_diagnostics();
                    return Ok(LspNotificationResponse {
                        aggregated_diagnostics,
                        notification,
                    });
                }
                Err(e) => return Ok(LspNotificationResponse::error(&e)),
            };
        }

        LspNotification::ManifestSaved(manifest_location) => {
            // We will rebuild the entire state, without to try any optimizations for now
            let mut protocol_state = ProtocolState::new();
            match build_state(&manifest_location, &mut protocol_state, file_accessor).await {
                Ok(_) => {
                    editor_state
                        .try_write()?
                        .index_protocol(manifest_location, protocol_state);
                    let (aggregated_diagnostics, notification) =
                        editor_state.try_read()?.get_aggregated_diagnostics();
                    return Ok(LspNotificationResponse {
                        aggregated_diagnostics,
                        notification,
                    });
                }
                Err(e) => return Ok(LspNotificationResponse::error(&e)),
            };
        }

        LspNotification::ContractOpened(contract_location) => {
            if !editor_state
                .try_read()?
                .active_contracts
                .contains_key(&contract_location)
            {
                let contract_source = match file_accessor {
                    None => contract_location.read_content_as_utf8(),
                    Some(file_accessor) => {
                        file_accessor.read_file(contract_location.to_string()).await
                    }
                }?;

                let clarity_version = DEFAULT_CLARITY_VERSION;
                editor_state.try_write()?.insert_active_contract(
                    contract_location.clone(),
                    clarity_version,
                    contract_source.as_str(),
                );
            }

            if editor_state
                .try_read()?
                .contracts_lookup
                .contains_key(&contract_location)
            {
                return Ok(LspNotificationResponse::default());
            }

            let manifest_location = contract_location
                .get_project_manifest_location(file_accessor)
                .await?;

            let mut protocol_state = ProtocolState::new();
            match build_state(&manifest_location, &mut protocol_state, file_accessor).await {
                Ok(_) => {
                    editor_state
                        .try_write()?
                        .index_protocol(manifest_location, protocol_state);
                    let (aggregated_diagnostics, notification) =
                        editor_state.try_read()?.get_aggregated_diagnostics();
                    return Ok(LspNotificationResponse {
                        aggregated_diagnostics,
                        notification,
                    });
                }
                Err(e) => return Ok(LspNotificationResponse::error(&e)),
            };
        }

        LspNotification::ContractSaved(contract_location) => {
            let manifest_location = match editor_state
                .try_write()?
                .clear_protocol_associated_with_contract(&contract_location)
            {
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
            match build_state(&manifest_location, &mut protocol_state, file_accessor).await {
                Ok(_contracts_updates) => {
                    editor_state
                        .try_write()?
                        .index_protocol(manifest_location, protocol_state);
                    let (aggregated_diagnostics, notification) =
                        editor_state.try_read()?.get_aggregated_diagnostics();
                    return Ok(LspNotificationResponse {
                        aggregated_diagnostics,
                        notification,
                    });
                }
                Err(e) => return Ok(LspNotificationResponse::error(&e)),
            };
        }

        LspNotification::ContractChanged(contract_location, contract_source) => {
            match editor_state
                .try_write()?
                .update_contract(&contract_location, &contract_source)
            {
                Ok(result) => {
                    let aggregated_diagnostics = match result.diagnostic {
                        Some(diagnostic) => vec![(contract_location, vec![diagnostic])],
                        None => vec![],
                    };
                    return Ok(LspNotificationResponse {
                        aggregated_diagnostics,
                        notification: None,
                    });
                }
                Err(err) => Ok(LspNotificationResponse::error(&err)),
            }
        }

        LspNotification::ContractClosed(contract_location) => {
            editor_state
                .try_write()?
                .active_contracts
                .remove(&contract_location);
            Ok(LspNotificationResponse::default())
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LspRequest {
    GetIntellisense(FileLocation),
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct LspRequestResponse {
    pub completion_items: Vec<CompletionItem>,
}

pub fn process_request(command: LspRequest, editor_state: &EditorStateInput) -> LspRequestResponse {
    match command {
        LspRequest::GetIntellisense(contract_location) => {
            let mut completion_items_src = editor_state
                .try_read()
                .unwrap()
                .get_completion_items_for_contract(&contract_location);
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

            LspRequestResponse { completion_items }
        }
    }
}
