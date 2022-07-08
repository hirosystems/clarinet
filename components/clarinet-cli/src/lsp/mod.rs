mod native_bridge;

use clarinet_files::FileLocation;
use clarity_lsp::backend::{self, LspRequestAsync, LspResponse};
use clarity_lsp::lsp_types::MessageType;
use clarity_lsp::state::{build_state, EditorState, ProtocolState};
use clarity_lsp::types::CompletionItemKind;
use clarity_lsp::utils;
use clarity_repl::clarity::diagnostic::{Diagnostic as ClarityDiagnostic, Level as ClarityLevel};
use native_bridge::LspNativeBridge;

use std::sync::mpsc::{self, Receiver, Sender};
use tokio;
use tower_lsp::lsp_types::{
    Diagnostic, DiagnosticSeverity, Documentation, MarkupContent, MarkupKind, Position, Range,
};
use tower_lsp::{LspService, Server};

pub fn run_lsp() {
    match block_on(do_run_lsp()) {
        Err(_e) => std::process::exit(1),
        _ => {}
    };
}

pub fn block_on<F, R>(future: F) -> R
where
    F: std::future::Future<Output = R>,
{
    let rt = crate::utils::create_basic_runtime();
    rt.block_on(future)
}

async fn do_run_lsp() -> Result<(), String> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        crate::utils::nestable_block_on(backend::start_language_server(rx, None));
    });

    let (service, messages) = LspService::new(|client| LspNativeBridge::new(client, tx));
    Server::new(stdin, stdout)
        .interleave(messages)
        .serve(service)
        .await;
    Ok(())
}

pub fn completion_item_type_to_tower_lsp_type(
    item: &mut clarity_lsp::types::CompletionItem,
) -> tower_lsp::lsp_types::CompletionItem {
    tower_lsp::lsp_types::CompletionItem {
        label: item.label.clone(),
        kind: Some(completion_item_kind_lsp_type_to_tower_lsp_type(&item.kind)),
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
        insert_text_format: Some(insert_text_format_lsp_type_to_tower_lsp_type(
            &item.insert_text_format,
        )),
        insert_text_mode: None,
        text_edit: None,
        additional_text_edits: None,
        command: None,
        commit_characters: None,
        data: None,
        tags: None,
    }
}

pub fn completion_item_kind_lsp_type_to_tower_lsp_type(
    kind: &clarity_lsp::types::CompletionItemKind,
) -> tower_lsp::lsp_types::CompletionItemKind {
    match kind {
        clarity_lsp::types::CompletionItemKind::Class => {
            tower_lsp::lsp_types::CompletionItemKind::Class
        }
        clarity_lsp::types::CompletionItemKind::Event => {
            tower_lsp::lsp_types::CompletionItemKind::Event
        }
        clarity_lsp::types::CompletionItemKind::Field => {
            tower_lsp::lsp_types::CompletionItemKind::Field
        }
        clarity_lsp::types::CompletionItemKind::Function => {
            tower_lsp::lsp_types::CompletionItemKind::Function
        }
        clarity_lsp::types::CompletionItemKind::Module => {
            tower_lsp::lsp_types::CompletionItemKind::Module
        }
        clarity_lsp::types::CompletionItemKind::TypeParameter => {
            tower_lsp::lsp_types::CompletionItemKind::TypeParameter
        }
    }
}

pub fn insert_text_format_lsp_type_to_tower_lsp_type(
    kind: &clarity_lsp::types::InsertTextFormat,
) -> tower_lsp::lsp_types::InsertTextFormat {
    match kind {
        clarity_lsp::types::InsertTextFormat::PlainText => {
            tower_lsp::lsp_types::InsertTextFormat::PlainText
        }
        clarity_lsp::types::InsertTextFormat::Snippet => {
            tower_lsp::lsp_types::InsertTextFormat::Snippet
        }
    }
}

pub fn clarity_diagnostics_to_tower_lsp_type(
    diagnostics: &mut Vec<ClarityDiagnostic>,
) -> Vec<tower_lsp::lsp_types::Diagnostic> {
    let mut dst = vec![];
    for d in diagnostics.iter_mut() {
        dst.push(clarity_diagnostic_to_tower_lsp_type(d));
    }
    dst
}

pub fn clarity_diagnostic_to_tower_lsp_type(
    diagnostic: &ClarityDiagnostic,
) -> tower_lsp::lsp_types::Diagnostic {
    let range = match diagnostic.spans.len() {
        0 => Range::default(),
        _ => Range {
            start: Position {
                line: diagnostic.spans[0].start_line - 1,
                character: diagnostic.spans[0].start_column - 1,
            },
            end: Position {
                line: diagnostic.spans[0].end_line - 1,
                character: diagnostic.spans[0].end_column,
            },
        },
    };
    // TODO(lgalabru): add hint for contracts not found errors
    Diagnostic {
        range,
        severity: match diagnostic.level {
            ClarityLevel::Error => Some(DiagnosticSeverity::Error),
            ClarityLevel::Warning => Some(DiagnosticSeverity::Warning),
            ClarityLevel::Note => Some(DiagnosticSeverity::Information),
        },
        code: None,
        code_description: None,
        source: Some("clarity".to_string()),
        message: diagnostic.message.clone(),
        related_information: None,
        tags: None,
        data: None,
    }
}

#[test]
fn test_opening_counter_contract_should_return_fresh_analysis() {
    use std::sync::mpsc::channel;

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        crate::utils::nestable_block_on(backend::start_language_server(rx, None));
    });

    let contract_location = {
        let mut counter_path = std::env::current_dir().expect("Unable to get current dir");
        counter_path.push("examples");
        counter_path.push("counter");
        counter_path.push("contracts");
        counter_path.push("counter.clar");
        FileLocation::from_path(counter_path)
    };
    let (response_tx, response_rx) = channel();
    let _ = tx.send(LspRequestAsync::ContractOpened(
        contract_location.clone(),
        response_tx.clone(),
    ));
    let response = response_rx.recv().expect("Unable to get response");

    // the counter project should emit 2 warnings and 2 notes coming from counter.clar
    assert_eq!(response.aggregated_diagnostics.len(), 1);
    let (_url, diags) = &response.aggregated_diagnostics[0];
    assert_eq!(diags.len(), 4);

    // re-opening this contract should not trigger a full analysis
    let _ = tx.send(LspRequestAsync::ContractOpened(
        contract_location,
        response_tx,
    ));
    let response = response_rx.recv().expect("Unable to get response");
    assert_eq!(response, LspResponse::default());
}

#[test]
fn test_opening_counter_manifest_should_return_fresh_analysis() {
    use std::sync::mpsc::channel;

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        crate::utils::nestable_block_on(backend::start_language_server(rx, None));
    });

    let manifest_location = {
        let mut manifest_path = std::env::current_dir().expect("Unable to get current dir");
        manifest_path.push("examples");
        manifest_path.push("counter");
        manifest_path.push("Clarinet.toml");
        FileLocation::from_path(manifest_path)
    };

    let (response_tx, response_rx) = channel();
    let _ = tx.send(LspRequestAsync::ManifestOpened(
        manifest_location.clone(),
        response_tx.clone(),
    ));
    let response = response_rx.recv().expect("Unable to get response");

    // the counter project should emit 2 warnings and 2 notes coming from counter.clar
    assert_eq!(response.aggregated_diagnostics.len(), 1);
    let (_url, diags) = &response.aggregated_diagnostics[0];
    assert_eq!(diags.len(), 4);

    // re-opening this manifest should not trigger a full analysis
    let _ = tx.send(LspRequestAsync::ManifestOpened(
        manifest_location,
        response_tx,
    ));
    let response = response_rx.recv().expect("Unable to get response");
    assert_eq!(response, LspResponse::default());
}

#[test]
fn test_opening_simple_nft_manifest_should_return_fresh_analysis() {
    use std::sync::mpsc::channel;

    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        crate::utils::nestable_block_on(backend::start_language_server(rx, None));
    });

    let mut manifest_location = std::env::current_dir().expect("Unable to get current dir");
    manifest_location.push("examples");
    manifest_location.push("simple-nft");
    manifest_location.push("Clarinet.toml");

    let (response_tx, response_rx) = channel();
    let _ = tx.send(LspRequestAsync::ManifestOpened(
        FileLocation::from_path(manifest_location),
        response_tx.clone(),
    ));
    let response = response_rx.recv().expect("Unable to get response");

    // the counter project should emit 2 warnings and 2 notes coming from counter.clar
    assert_eq!(response.aggregated_diagnostics.len(), 2);
    let (_, diags_0) = &response.aggregated_diagnostics[0];
    let (_, diags_1) = &response.aggregated_diagnostics[1];
    assert_eq!(diags_0.len().max(diags_1.len()), 8);
}
