use crate::lsp_types::MessageType;
use crate::state::{build_state, EditorState, ProtocolState};
use crate::types::{CompletionItemKind, InsertTextFormat};
use crate::utils::get_contract_location;
use clarinet_files::{FileAccessor, FileLocation, ProjectManifest};
use clarity_repl::clarity::diagnostic::Diagnostic;
use lsp_types::{
    CompletionItem, CompletionOptions, CompletionParams, DocumentSymbol, DocumentSymbolParams,
    Documentation, Hover, HoverParams, HoverProviderCapability, InitializeParams, InitializeResult,
    MarkupContent, MarkupKind, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextDocumentSyncOptions, TextDocumentSyncSaveOptions,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

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

                let lookup_clarity_version = editor_state.try_read(|es| {
                    match es.contracts_lookup.get(&contract_location) {
                        Some(metadata) => Some(metadata.clarity_version),
                        None => None,
                    }
                })?;

                let clarity_version = match lookup_clarity_version {
                    Some(clarity_version) => clarity_version,
                    None => {
                        // if the contract isn't in loopkup yet, get version directly from manifest
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
                        .clarity_version
                    }
                };

                editor_state.try_write(|es| {
                    es.insert_active_contract(
                        contract_location.clone(),
                        clarity_version,
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

            // TODO(lgalabru): introduce partial analysis #604
            // We will rebuild the entire state, without trying any optimizations for now
            let mut protocol_state = ProtocolState::new();
            match build_state(&manifest_location, &mut protocol_state, file_accessor).await {
                Ok(_contracts_updates) => {
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

        LspNotification::ContractChanged(contract_location, contract_source) => {
            match editor_state
                .try_write(|es| es.update_active_contract(&contract_location, &contract_source))?
            {
                Ok(_result) => {
                    // In case the source can not be parsed, the diagnostic could be sent but it would
                    // remove the other diagnostic errors (types, check-checker, etc).
                    // Let's address it as part of #604
                    // let aggregated_diagnostics = vec![(contract_location, vec![diagnostic.unwrap()])],
                    return Ok(LspNotificationResponse::default());
                }
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
    Hover(HoverParams),
    DocumentSymbol(DocumentSymbolParams),
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub enum LspRequestResponse {
    Initialize(InitializeResult),
    CompletionItems(Vec<CompletionItem>),
    DocumentSymbol(Vec<DocumentSymbol>),
    Hover(Option<Hover>),
}

pub fn process_request(command: LspRequest, editor_state: &EditorStateInput) -> LspRequestResponse {
    match command {
        LspRequest::Initialize(_params) => LspRequestResponse::Initialize(InitializeResult {
            server_info: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        will_save: Some(false),
                        will_save_wait_until: Some(false),
                        save: Some(TextDocumentSyncSaveOptions::Supported(true)),
                    },
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: None,
                    all_commit_characters: None,
                    work_done_progress_options: Default::default(),
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                document_symbol_provider: Some(lsp_types::OneOf::Left(true)),
                ..ServerCapabilities::default()
            },
        }),

        LspRequest::Completion(params) => {
            let file_url = params.text_document_position.text_document.uri;
            let contract_location = match get_contract_location(&file_url) {
                Some(contract_location) => contract_location,
                None => return LspRequestResponse::CompletionItems(vec![]),
            };
            let mut completion_items_src = match editor_state
                .try_read(|es| es.get_completion_items_for_contract(&contract_location))
            {
                Ok(result) => result,
                Err(_) => return LspRequestResponse::CompletionItems(vec![]),
            };

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

                    let kind = match item.kind {
                        CompletionItemKind::Class => lsp_types::CompletionItemKind::CLASS,
                        CompletionItemKind::Event => lsp_types::CompletionItemKind::EVENT,
                        CompletionItemKind::Field => lsp_types::CompletionItemKind::FIELD,
                        CompletionItemKind::Function => lsp_types::CompletionItemKind::FUNCTION,
                        CompletionItemKind::Module => lsp_types::CompletionItemKind::MODULE,
                        CompletionItemKind::TypeParameter => {
                            lsp_types::CompletionItemKind::TYPE_PARAMETER
                        }
                    };

                    let insert_text_format = match item.insert_text_format {
                        InsertTextFormat::PlainText => lsp_types::InsertTextFormat::PLAIN_TEXT,
                        InsertTextFormat::Snippet => lsp_types::InsertTextFormat::SNIPPET,
                    };

                    let completion_item = CompletionItem {
                        label: item.label.clone(),
                        kind: Some(kind),
                        detail: item.detail.take(),
                        documentation: item.markdown_documentation.take().and_then(|doc| {
                            Some(Documentation::MarkupContent(MarkupContent {
                                kind: MarkupKind::Markdown,
                                value: doc,
                            }))
                        }),
                        deprecated: None,
                        preselect: None,
                        sort_text: None,
                        filter_text: None,
                        insert_text: item.insert_text.take(),
                        insert_text_format: Some(insert_text_format),
                        insert_text_mode: None,
                        text_edit: None,
                        additional_text_edits: None,
                        command: None,
                        commit_characters: None,
                        data: None,
                        tags: None,
                    };
                    completion_items.push(completion_item);
                }
            }

            LspRequestResponse::CompletionItems(completion_items)
        }

        LspRequest::DocumentSymbol(params) => {
            let file_url = params.text_document.uri;
            let contract_location = match get_contract_location(&file_url) {
                Some(contract_location) => contract_location,
                None => return LspRequestResponse::Hover(None),
            };

            LspRequestResponse::DocumentSymbol(
                match editor_state
                    .try_read(|es| es.get_document_symbols_for_contract(&contract_location))
                {
                    Ok(symbols) => symbols,
                    Err(_) => vec![],
                },
            )
        }

        LspRequest::Hover(params) => {
            let file_url = params.text_document_position_params.text_document.uri;
            let contract_location = match get_contract_location(&file_url) {
                Some(contract_location) => contract_location,
                None => return LspRequestResponse::Hover(None),
            };
            let position = params.text_document_position_params.position;
            let hover_data = match editor_state
                .try_read(|es| es.get_hover_data(&contract_location, &position))
            {
                Ok(result) => result,
                Err(_) => return LspRequestResponse::Hover(None),
            };
            LspRequestResponse::Hover(hover_data)
        }
    }
}
