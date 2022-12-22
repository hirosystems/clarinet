use crate::lsp_types::MessageType;
use crate::state::{build_state, EditorState, ProtocolState};
use crate::utils::get_contract_location;
use clarinet_files::{FileAccessor, FileLocation, ProjectManifest};
use clarity_repl::clarity::diagnostic::Diagnostic;
use clarity_repl::repl::ContractDeployer;
use lsp_types::{
    CompletionItem, CompletionParams, DocumentSymbol, DocumentSymbolParams, GotoDefinitionParams,
    Hover, HoverParams, InitializeParams, InitializeResult, Location, SignatureHelp,
    SignatureHelpParams,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

use super::requests::capabilities::{get_capabilities, InitializationOptions};

#[derive(Debug, Clone)]
pub enum EditorStateInput {
    Owned(EditorState),
    RwLock(Arc<RwLock<EditorState>>),
}

impl EditorStateInput {
    pub fn try_read<F, R>(&self, closure: F) -> Result<R, String>
    where
        F: FnOnce(&EditorState) -> R,
    {
        match self {
            EditorStateInput::Owned(editor_state) => Ok(closure(&editor_state)),
            EditorStateInput::RwLock(editor_state_lock) => match editor_state_lock.try_read() {
                Ok(editor_state) => Ok(closure(&editor_state)),
                Err(_) => Err("failed to read editor_state".to_string()),
            },
        }
    }

    pub fn try_write<F, R>(&mut self, closure: F) -> Result<R, String>
    where
        F: FnOnce(&mut EditorState) -> R,
    {
        match self {
            EditorStateInput::Owned(editor_state) => Ok(closure(editor_state)),
            EditorStateInput::RwLock(editor_state_lock) => match editor_state_lock.try_write() {
                Ok(mut editor_state) => Ok(closure(&mut editor_state)),
                Err(_) => Err("failed to write editor_state".to_string()),
            },
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
    editor_state: &mut EditorStateInput,
    file_accessor: Option<&Box<dyn FileAccessor>>,
) -> Result<LspNotificationResponse, String> {
    match command {
        LspNotification::ManifestOpened(manifest_location) => {
            // Only build the initial protocal state if it does not exist
            if editor_state.try_read(|es| es.protocols.contains_key(&manifest_location))? {
                return Ok(LspNotificationResponse::default());
            }

            // With this manifest_location, let's initialize our state.
            let mut protocol_state = ProtocolState::new();
            match build_state(&manifest_location, &mut protocol_state, file_accessor).await {
                Ok(_) => {
                    editor_state
                        .try_write(|es| es.index_protocol(manifest_location, protocol_state))?;
                    let (aggregated_diagnostics, notification) =
                        editor_state.try_read(|es| es.get_aggregated_diagnostics())?;
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
                        .try_write(|es| es.index_protocol(manifest_location, protocol_state))?;
                    let (aggregated_diagnostics, notification) =
                        editor_state.try_read(|es| es.get_aggregated_diagnostics())?;
                    return Ok(LspNotificationResponse {
                        aggregated_diagnostics,
                        notification,
                    });
                }
                Err(e) => return Ok(LspNotificationResponse::error(&e)),
            };
        }

        LspNotification::ContractOpened(contract_location) => {
            let manifest_location = contract_location
                .get_project_manifest_location(file_accessor)
                .await?;

            // store the contract in the active_contracts map
            if !editor_state.try_read(|es| es.active_contracts.contains_key(&contract_location))? {
                let contract_source = match file_accessor {
                    None => contract_location.read_content_as_utf8(),
                    Some(file_accessor) => {
                        file_accessor.read_file(contract_location.to_string()).await
                    }
                }?;

                let metadata = editor_state.try_read(|es| {
                    match es.contracts_lookup.get(&contract_location) {
                        Some(metadata) => {
                            Some((metadata.clarity_version, metadata.deployer.clone()))
                        }
                        None => None,
                    }
                })?;

                // if the contract isn't in lookup yet, fallback on manifest, to be improved in #668
                let clarity_version = match metadata {
                    Some((clarity_version, _)) => clarity_version,
                    None => {
                        match file_accessor {
                            None => ProjectManifest::from_location(&manifest_location),
                            Some(file_accessor) => {
                                ProjectManifest::from_file_accessor(
                                    &manifest_location,
                                    file_accessor,
                                )
                                .await
                            }
                        }?
                        .contracts_settings
                        .get(&contract_location)
                        .ok_or("contract not found in manifest")?
                        .clone()
                        .clarity_version
                    }
                };

                let issuer = metadata.and_then(|(_, deployer)| match deployer {
                    ContractDeployer::ContractIdentifier(id) => Some(id.issuer.to_owned()),
                    _ => None,
                });

                editor_state.try_write(|es| {
                    es.insert_active_contract(
                        contract_location.clone(),
                        clarity_version,
                        issuer,
                        contract_source.as_str(),
                    )
                })?;
            }

            // Only build the initial protocal state if it does not exist
            if editor_state.try_read(|es| es.protocols.contains_key(&manifest_location))? {
                return Ok(LspNotificationResponse::default());
            }

            let mut protocol_state = ProtocolState::new();
            match build_state(&manifest_location, &mut protocol_state, file_accessor).await {
                Ok(_) => {
                    editor_state
                        .try_write(|es| es.index_protocol(manifest_location, protocol_state))?;
                    let (aggregated_diagnostics, notification) =
                        editor_state.try_read(|es| es.get_aggregated_diagnostics())?;
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
                .try_write(|es| es.clear_protocol_associated_with_contract(&contract_location))?
            {
                Some(manifest_location) => manifest_location,
                None => {
                    contract_location
                        .get_project_manifest_location(file_accessor)
                        .await?
                }
            };

            // TODO(): introduce partial analysis #604
            let mut protocol_state = ProtocolState::new();
            match build_state(&manifest_location, &mut protocol_state, file_accessor).await {
                Ok(_) => {
                    editor_state.try_write(|es| {
                        es.index_protocol(manifest_location, protocol_state);
                        if let Some(contract) = es.active_contracts.get_mut(&contract_location) {
                            contract.update_definitions();
                        };
                    })?;

                    let (aggregated_diagnostics, notification) =
                        editor_state.try_read(|es| es.get_aggregated_diagnostics())?;
                    return Ok(LspNotificationResponse {
                        aggregated_diagnostics,
                        notification,
                    });
                }
                Err(e) => return Ok(LspNotificationResponse::error(&e)),
            };
        }

        LspNotification::ContractChanged(contract_location, contract_source) => {
            match editor_state.try_write(|es| {
                es.update_active_contract(&contract_location, &contract_source, false)
            })? {
                Ok(_result) => Ok(LspNotificationResponse::default()),
                Err(err) => Ok(LspNotificationResponse::error(&err)),
            }
        }

        LspNotification::ContractClosed(contract_location) => {
            editor_state.try_write(|es| es.active_contracts.remove_entry(&contract_location))?;
            Ok(LspNotificationResponse::default())
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum LspRequest {
    Initialize(InitializeParams),
    Completion(CompletionParams),
    SignatureHelp(SignatureHelpParams),
    Definition(GotoDefinitionParams),
    Hover(HoverParams),
    DocumentSymbol(DocumentSymbolParams),
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub enum LspRequestResponse {
    Initialize(InitializeResult),
    CompletionItems(Vec<CompletionItem>),
    SignatureHelp(Option<SignatureHelp>),
    Definition(Option<Location>),
    DocumentSymbol(Vec<DocumentSymbol>),
    Hover(Option<Hover>),
}

pub fn process_request(
    command: LspRequest,
    editor_state: &mut EditorStateInput,
) -> LspRequestResponse {
    match command {
        LspRequest::Initialize(params) => {
            let initialization_options: InitializationOptions = params
                .initialization_options
                .and_then(|o| serde_json::from_str(o.as_str()?).ok())
                .expect("failed to parse initialization options");

            let _ = editor_state.try_write(|es| es.settings = initialization_options.clone());

            LspRequestResponse::Initialize(InitializeResult {
                server_info: None,
                capabilities: get_capabilities(&initialization_options),
            })
        }

        LspRequest::Completion(params) => {
            let file_url = params.text_document_position.text_document.uri;
            let position = params.text_document_position.position;

            let contract_location = match get_contract_location(&file_url) {
                Some(contract_location) => contract_location,
                None => return LspRequestResponse::CompletionItems(vec![]),
            };

            let completion_items = match editor_state
                .try_read(|es| es.get_completion_items_for_contract(&contract_location, &position))
            {
                Ok(result) => result,
                Err(_) => return LspRequestResponse::CompletionItems(vec![]),
            };

            LspRequestResponse::CompletionItems(completion_items)
        }

        LspRequest::Definition(params) => {
            let file_url = params.text_document_position_params.text_document.uri;
            let contract_location = match get_contract_location(&file_url) {
                Some(contract_location) => contract_location,
                None => return LspRequestResponse::Definition(None),
            };
            let position = params.text_document_position_params.position;
            let location = editor_state
                .try_read(|es| es.get_definition_location(&contract_location, &position))
                .unwrap_or_default();
            LspRequestResponse::Definition(location)
        }

        LspRequest::SignatureHelp(params) => {
            let file_url = params.text_document_position_params.text_document.uri;
            let contract_location = match get_contract_location(&file_url) {
                Some(contract_location) => contract_location,
                None => return LspRequestResponse::SignatureHelp(None),
            };
            let position = params.text_document_position_params.position;

            // if the developer selects a specific signature
            // it can be retrieved in the context and kept selected
            let active_signature = params
                .context
                .and_then(|c| c.active_signature_help)
                .and_then(|s| s.active_signature);

            let signature = editor_state
                .try_read(|es| {
                    es.get_signature_help(&contract_location, &position, active_signature)
                })
                .unwrap_or_default();
            LspRequestResponse::SignatureHelp(signature)
        }

        LspRequest::DocumentSymbol(params) => {
            let file_url = params.text_document.uri;
            let contract_location = match get_contract_location(&file_url) {
                Some(contract_location) => contract_location,
                None => return LspRequestResponse::DocumentSymbol(vec![]),
            };
            let document_symbols = editor_state
                .try_read(|es| es.get_document_symbols_for_contract(&contract_location))
                .unwrap_or_default();
            LspRequestResponse::DocumentSymbol(document_symbols)
        }

        LspRequest::Hover(params) => {
            let file_url = params.text_document_position_params.text_document.uri;
            let contract_location = match get_contract_location(&file_url) {
                Some(contract_location) => contract_location,
                None => return LspRequestResponse::Hover(None),
            };
            let position = params.text_document_position_params.position;
            let hover_data = editor_state
                .try_read(|es| es.get_hover_data(&contract_location, &position))
                .unwrap_or_default();
            LspRequestResponse::Hover(hover_data)
        }
    }
}
