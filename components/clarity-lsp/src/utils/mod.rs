use super::types::*;
use clarinet_files::FileLocation;
use clarity_repl::clarity::vm::analysis::ContractAnalysis;
use clarity_repl::clarity::vm::diagnostic::{
    Diagnostic as ClarityDiagnostic, Level as ClarityLevel,
};
use clarity_repl::clarity::vm::types::FunctionType;
use lsp_types::{CompletionItem, CompletionItemKind, Diagnostic as LspDiagnostic};
use lsp_types::{DiagnosticSeverity, Position, Range};
use lsp_types::{InsertTextFormat, Url};

#[cfg(feature = "wasm")]
macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

#[cfg(feature = "wasm")]
pub(crate) use log;

pub fn clarity_diagnostics_to_lsp_type(diagnostics: &Vec<ClarityDiagnostic>) -> Vec<LspDiagnostic> {
    let mut dst = vec![];
    for d in diagnostics {
        dst.push(clarity_diagnostic_to_lsp_type(d));
    }
    dst
}

pub fn clarity_diagnostic_to_lsp_type(diagnostic: &ClarityDiagnostic) -> LspDiagnostic {
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
    LspDiagnostic {
        range,
        severity: match diagnostic.level {
            ClarityLevel::Error => Some(DiagnosticSeverity::ERROR),
            ClarityLevel::Warning => Some(DiagnosticSeverity::WARNING),
            ClarityLevel::Note => Some(DiagnosticSeverity::INFORMATION),
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

fn build_intellisense_args(signature: &FunctionType) -> Vec<String> {
    let mut args = vec![];
    match signature {
        FunctionType::Fixed(function) => {
            for (i, arg) in function.args.iter().enumerate() {
                args.push(format!("${{{}:{}:{}}}", i + 1, arg.name, arg.signature));
            }
        }
        _ => {}
    }
    args
}

pub fn build_intellisense(analysis: &ContractAnalysis) -> CompletionMaps {
    let mut intra_contract = vec![];
    let mut inter_contract = vec![];

    for (name, signature) in analysis.public_function_types.iter() {
        let insert_text = format!("{} {}", name, build_intellisense_args(signature).join(" "));
        intra_contract.push(CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::MODULE),
            insert_text: Some(insert_text),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });

        let label = format!(
            "contract-call::{}::{}",
            analysis.contract_identifier.name.to_string(),
            name.to_string()
        );
        let _insert = format!("{} {}", name, build_intellisense_args(signature).join(" "));
        let insert_text = format!(
            "contract-call? .{} {} {}",
            analysis.contract_identifier.name.to_string(),
            name.to_string(),
            build_intellisense_args(signature).join(" ")
        );
        inter_contract.push(CompletionItem {
            label,
            kind: Some(CompletionItemKind::EVENT),
            insert_text: Some(insert_text),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
    }

    for (name, signature) in analysis.read_only_function_types.iter() {
        let insert_text = format!("{} {}", name, build_intellisense_args(signature).join(" "));
        intra_contract.push(CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::MODULE),
            insert_text: Some(insert_text),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });

        let label = format!(
            "contract-call::{}::{}",
            analysis.contract_identifier.name.to_string(),
            name.to_string()
        );
        let insert_text = format!(
            "contract-call? .{} {} {}",
            analysis.contract_identifier.name.to_string(),
            name.to_string(),
            build_intellisense_args(signature).join(" ")
        );
        inter_contract.push(CompletionItem {
            label,
            kind: Some(CompletionItemKind::EVENT),
            insert_text: Some(insert_text),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
    }

    for (name, signature) in analysis.private_function_types.iter() {
        let insert_text = format!("{} {}", name, build_intellisense_args(signature).join(" "));
        intra_contract.push(CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::MODULE),
            insert_text: Some(insert_text),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
    }

    CompletionMaps {
        inter_contract,
        intra_contract,
        data_fields: vec![],
    }
}

pub fn get_manifest_location(text_document_uri: &Url) -> Option<FileLocation> {
    let file_location = text_document_uri.to_string();
    if !file_location.ends_with("Clarinet.toml") {
        return None;
    }
    FileLocation::try_parse(&file_location, None)
}

pub fn get_contract_location(text_document_uri: &Url) -> Option<FileLocation> {
    let file_location = text_document_uri.to_string();
    if !file_location.ends_with(".clar") {
        return None;
    }
    FileLocation::try_parse(&file_location, None)
}
