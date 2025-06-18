use crate::lsp_types::MessageType;
use crate::state::{build_state, EditorState, ProtocolState};
use crate::utils::get_contract_location;
use clarinet_files::{FileAccessor, FileLocation, ProjectManifest};
use clarity_repl::clarity::diagnostic::Diagnostic;
use clarity_repl::repl::ContractDeployer;
use lsp_types::{
    CompletionItem, CompletionParams, DocumentFormattingParams, DocumentRangeFormattingParams,
    DocumentSymbol, DocumentSymbolParams, GotoDefinitionParams, Hover, HoverParams,
    InitializeParams, InitializeResult, Location, SignatureHelp, SignatureHelpParams, TextEdit,
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
            EditorStateInput::Owned(editor_state) => Ok(closure(editor_state)),
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

#[derive(Debug, Default, PartialEq, Deserialize, Serialize)]
pub struct LspNotificationResponse {
    pub aggregated_diagnostics: Vec<(FileLocation, Vec<Diagnostic>)>,
    pub notification: Option<(MessageType, String)>,
}

impl LspNotificationResponse {
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
    file_accessor: Option<&dyn FileAccessor>,
) -> Result<LspNotificationResponse, String> {
    match command {
        LspNotification::ManifestOpened(manifest_location) => {
            // Only build the initial protocol state if it does not exist
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
                    Ok(LspNotificationResponse {
                        aggregated_diagnostics,
                        notification,
                    })
                }
                Err(e) => Ok(LspNotificationResponse::error(&e)),
            }
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
                    Ok(LspNotificationResponse {
                        aggregated_diagnostics,
                        notification,
                    })
                }
                Err(e) => Ok(LspNotificationResponse::error(&e)),
            }
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
                    es.contracts_lookup
                        .get(&contract_location)
                        .map(|metadata| (metadata.clarity_version, metadata.deployer.clone()))
                })?;

                // if the contract isn't in lookup yet, fallback on manifest, to be improved in #668
                let clarity_version = match metadata {
                    Some((clarity_version, _)) => clarity_version,
                    None => {
                        match file_accessor {
                            None => ProjectManifest::from_location(&manifest_location, false),
                            Some(file_accessor) => {
                                ProjectManifest::from_file_accessor(
                                    &manifest_location,
                                    false,
                                    file_accessor,
                                )
                                .await
                            }
                        }?
                        .contracts_settings
                        .get(&contract_location)
                        .ok_or(format!(
                            "No Clarinet.toml is associated to the contract {}",
                            &contract_location.get_file_name().unwrap_or_default()
                        ))?
                        .clone()
                        .clarity_version
                    }
                };

                let issuer = metadata.and_then(|(_, deployer)| match deployer {
                    ContractDeployer::ContractIdentifier(id) => Some(id.issuer),
                    _ => None,
                });

                editor_state.try_write(|es| {
                    es.insert_active_contract(
                        contract_location.clone(),
                        clarity_version,
                        issuer,
                        contract_source,
                    )
                })?;
            }

            // Only build the initial protocol state if it does not exist
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
                    Ok(LspNotificationResponse {
                        aggregated_diagnostics,
                        notification,
                    })
                }
                Err(e) => Ok(LspNotificationResponse::error(&e)),
            }
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
                    Ok(LspNotificationResponse {
                        aggregated_diagnostics,
                        notification,
                    })
                }
                Err(e) => Ok(LspNotificationResponse::error(&e)),
            }
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
    Completion(CompletionParams),
    SignatureHelp(SignatureHelpParams),
    Definition(GotoDefinitionParams),
    Hover(HoverParams),
    DocumentSymbol(DocumentSymbolParams),
    DocumentFormatting(DocumentFormattingParams),
    DocumentRangeFormatting(DocumentRangeFormattingParams),
    Initialize(Box<InitializeParams>),
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub enum LspRequestResponse {
    CompletionItems(Vec<CompletionItem>),
    SignatureHelp(Option<SignatureHelp>),
    Definition(Option<Location>),
    DocumentSymbol(Vec<DocumentSymbol>),
    DocumentFormatting(Option<Vec<TextEdit>>),
    DocumentRangeFormatting(Option<Vec<TextEdit>>),
    Hover(Option<Hover>),
    Initialize(Box<InitializeResult>),
}

pub fn process_request(
    command: LspRequest,
    editor_state: &EditorStateInput,
) -> Result<LspRequestResponse, String> {
    match command {
        LspRequest::Completion(params) => {
            let file_url = params.text_document_position.text_document.uri;
            let position = params.text_document_position.position;

            let Some(contract_location) = get_contract_location(&file_url) else {
                return Ok(LspRequestResponse::CompletionItems(vec![]));
            };

            let Ok(completion_items) = editor_state
                .try_read(|es| es.get_completion_items_for_contract(&contract_location, &position))
            else {
                return Ok(LspRequestResponse::CompletionItems(vec![]));
            };

            Ok(LspRequestResponse::CompletionItems(completion_items))
        }

        LspRequest::Definition(params) => {
            let file_url = params.text_document_position_params.text_document.uri;
            let Some(contract_location) = get_contract_location(&file_url) else {
                return Ok(LspRequestResponse::Definition(None));
            };
            let position = params.text_document_position_params.position;
            let location = editor_state
                .try_read(|es| es.get_definition_location(&contract_location, &position))
                .unwrap_or_default();
            Ok(LspRequestResponse::Definition(location))
        }

        LspRequest::SignatureHelp(params) => {
            let file_url = params.text_document_position_params.text_document.uri;
            let Some(contract_location) = get_contract_location(&file_url) else {
                return Ok(LspRequestResponse::SignatureHelp(None));
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
            Ok(LspRequestResponse::SignatureHelp(signature))
        }

        LspRequest::DocumentSymbol(params) => {
            let file_url = params.text_document.uri;
            let Some(contract_location) = get_contract_location(&file_url) else {
                return Ok(LspRequestResponse::DocumentSymbol(vec![]));
            };
            let document_symbols = editor_state
                .try_read(|es| es.get_document_symbols_for_contract(&contract_location))
                .unwrap_or_default();
            Ok(LspRequestResponse::DocumentSymbol(document_symbols))
        }
        LspRequest::DocumentFormatting(param) => {
            let file_url = param.text_document.uri;
            let Some(contract_location) = get_contract_location(&file_url) else {
                return Ok(LspRequestResponse::DocumentFormatting(None));
            };

            let Ok(Some(contract_data)) =
                editor_state.try_read(|es| es.active_contracts.get(&contract_location).cloned())
            else {
                return Ok(LspRequestResponse::DocumentFormatting(None));
            };
            let source = &contract_data.source;

            let tab_size = param.options.tab_size as usize;
            let prefer_space = param.options.insert_spaces;
            let props = param.options.properties;
            let max_line_length = props
                .get("maxLineLength")
                .and_then(|value| {
                    // FormattingProperty can be boolean, number, or string
                    match value {
                        lsp_types::FormattingProperty::Number(num) => Some(*num as usize),
                        lsp_types::FormattingProperty::String(s) => s.parse::<usize>().ok(),
                        _ => None,
                    }
                })
                .unwrap_or(80);
            let formatting_options = clarinet_format::formatter::Settings {
                indentation: if !prefer_space {
                    clarinet_format::formatter::Indentation::Tab
                } else {
                    clarinet_format::formatter::Indentation::Space(tab_size)
                },
                max_line_length,
            };

            let formatter = clarinet_format::formatter::ClarityFormatter::new(formatting_options);
            let formatted_result = formatter.format_file(source);
            let text_edit = lsp_types::TextEdit {
                range: lsp_types::Range {
                    start: lsp_types::Position {
                        line: 0,
                        character: 0,
                    },
                    end: lsp_types::Position {
                        line: source.lines().count() as u32,
                        character: 0,
                    },
                },
                new_text: formatted_result,
            };
            Ok(LspRequestResponse::DocumentFormatting(Some(vec![
                text_edit,
            ])))
        }
        LspRequest::DocumentRangeFormatting(param) => {
            let file_url = param.text_document.uri;
            let Some(contract_location) = get_contract_location(&file_url) else {
                return Ok(LspRequestResponse::DocumentRangeFormatting(None));
            };

            let Ok(Some(contract_data)) =
                editor_state.try_read(|es| es.active_contracts.get(&contract_location).cloned())
            else {
                return Ok(LspRequestResponse::DocumentRangeFormatting(None));
            };

            let source = &contract_data.source;

            let tab_size = param.options.tab_size as usize;
            let max_line_length = param
                .options
                .properties
                .get("maxLineLength")
                .and_then(|value| {
                    // FormattingProperty can be boolean, number, or string
                    match value {
                        lsp_types::FormattingProperty::Number(num) => Some(*num as usize),
                        lsp_types::FormattingProperty::String(s) => s.parse::<usize>().ok(),
                        _ => None,
                    }
                })
                .unwrap_or(80);
            let prefer_space = param.options.insert_spaces;
            let formatting_options = clarinet_format::formatter::Settings {
                indentation: if !prefer_space {
                    clarinet_format::formatter::Indentation::Tab
                } else {
                    clarinet_format::formatter::Indentation::Space(tab_size)
                },
                max_line_length,
            };

            // extract the text of just this range
            let lines: Vec<&str> = source.lines().collect();
            let start_line = param.range.start.line as usize;
            let end_line = param.range.end.line as usize;

            // Validate range boundaries
            if start_line >= lines.len() {
                return Ok(LspRequestResponse::DocumentRangeFormatting(None));
            }

            // Get the substring representing just the selected range
            let range_text = if start_line == end_line {
                // Single line selection
                let line = lines.get(start_line).unwrap_or(&"");
                let start_char = param.range.start.character as usize;
                let end_char = param.range.end.character as usize;
                let start_char = start_char.min(line.len());
                let end_char = end_char.min(line.len());

                if start_char >= end_char {
                    return Ok(LspRequestResponse::DocumentRangeFormatting(None));
                }

                line[start_char..end_char].to_string()
            } else {
                let mut result = String::new();

                // First line (might be partial)
                if let Some(first_line) = lines.get(start_line) {
                    let start_char = (param.range.start.character as usize).min(first_line.len());
                    result.push_str(&first_line[start_char..]);
                }

                // Middle lines (complete lines)
                for line_idx in (start_line + 1)..end_line {
                    if let Some(line) = lines.get(line_idx) {
                        result.push('\n');
                        result.push_str(line);
                    }
                }

                // Last line (might be partial) - only if end_line is different from start_line
                if end_line > start_line && end_line < lines.len() {
                    if let Some(last_line) = lines.get(end_line) {
                        let end_char = (param.range.end.character as usize).min(last_line.len());
                        result.push('\n');
                        result.push_str(&last_line[..end_char]);
                    }
                }

                result
            };

            // If the range text is empty or only whitespace, return None
            if range_text.trim().is_empty() {
                return Ok(LspRequestResponse::DocumentRangeFormatting(None));
            }

            // Count the number of trailing newlines in the original selection
            let mut trailing_newlines = 0;
            let mut temp_text = range_text.clone();
            while temp_text.ends_with('\n') {
                trailing_newlines += 1;
                temp_text.pop();
            }

            let formatter = clarinet_format::formatter::ClarityFormatter::new(formatting_options);

            // Try to format the range text, but handle panics/errors gracefully
            let formatted_result = formatter.format_section(&range_text);

            let formatted_result = match formatted_result {
                Ok(formatted_text) => {
                    let mut result = formatted_text.trim_end().to_string();
                    // Add back the same number of trailing newlines that were in the original
                    for _ in 0..trailing_newlines {
                        result.push('\n');
                    }
                    result
                }
                Err(_) => {
                    // If the selected range contains malformed/incomplete Clarity code,
                    // return None to indicate formatting is not possible
                    return Ok(LspRequestResponse::DocumentRangeFormatting(None));
                }
            };

            let text_edit = lsp_types::TextEdit {
                range: param.range,
                new_text: formatted_result,
            };

            Ok(LspRequestResponse::DocumentRangeFormatting(Some(vec![
                text_edit,
            ])))
        }

        LspRequest::Hover(params) => {
            let file_url = params.text_document_position_params.text_document.uri;
            let Some(contract_location) = get_contract_location(&file_url) else {
                return Ok(LspRequestResponse::Hover(None));
            };
            let position = params.text_document_position_params.position;
            let hover_data = editor_state
                .try_read(|es| es.get_hover_data(&contract_location, &position))
                .unwrap_or_default();
            Ok(LspRequestResponse::Hover(hover_data))
        }
        _ => Err(format!("Unexpected command: {command:?}")),
    }
}

// lsp requests are not supposed to mut the editor_state (only the notifications do)
// this is to ensure there is no concurrency between notifications and requests to
// acquire write lock on the editor state in a wasm context
// except for the Initialize request, which is the first interaction between the client and the server
// and can therefore safely acquire write lock on the editor state
pub fn process_mutating_request(
    command: LspRequest,
    editor_state: &mut EditorStateInput,
) -> Result<LspRequestResponse, String> {
    match command {
        LspRequest::Initialize(params) => {
            let initialization_options: InitializationOptions = params
                .initialization_options
                .and_then(|o| serde_json::from_value(o).ok())
                .unwrap_or_default();

            editor_state
                .try_write(|es| es.settings = initialization_options.clone())
                .map(|_| {
                    LspRequestResponse::Initialize(Box::new(InitializeResult {
                        server_info: None,
                        capabilities: get_capabilities(&initialization_options),
                    }))
                })
        }
        _ => Err(format!(
            "Unexpected command: {command:?}, should not mutate state"
        )),
    }
}

#[cfg(test)]
mod range_formatting_tests {
    use super::*;
    use crate::common::state::EditorState;
    use clarinet_files::FileLocation;
    use clarity_repl::clarity::ClarityVersion;
    use lsp_types::{
        DocumentRangeFormattingParams, FormattingOptions, Position, Range, TextDocumentIdentifier,
        Url, WorkDoneProgressParams,
    };
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn create_test_editor_state(source: String) -> EditorStateInput {
        let mut editor_state = EditorState::new();

        let contract_location = FileLocation::FileSystem {
            path: PathBuf::from("test.clar"),
        };

        editor_state.insert_active_contract(
            contract_location,
            ClarityVersion::Clarity2,
            None,
            source,
        );

        EditorStateInput::Owned(editor_state)
    }

    #[test]
    fn test_range_formatting_comments() {
        let source = "(ok true)\n\n(define-public (foo)\n  ;; this is a comment\n   (ok   true)\n)";

        let editor_state_input = create_test_editor_state(source.to_owned());

        let params = DocumentRangeFormattingParams {
            text_document: TextDocumentIdentifier {
                uri: Url::parse("file:///test.clar").unwrap(),
            },
            range: Range {
                start: Position {
                    line: 3,
                    character: 1,
                },
                end: Position {
                    line: 6,
                    character: 2,
                },
            },
            options: FormattingOptions {
                tab_size: 2,
                insert_spaces: true,
                properties: HashMap::new(),
                trim_trailing_whitespace: None,
                insert_final_newline: None,
                trim_final_newlines: None,
            },
            work_done_progress_params: WorkDoneProgressParams {
                work_done_token: None,
            },
        };

        let request = LspRequest::DocumentRangeFormatting(params);

        assert!(process_request(request, &editor_state_input).is_ok());
    }
}
