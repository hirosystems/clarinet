use super::types::*;
use clarity_repl::clarity::analysis::ContractAnalysis;
use clarity_repl::clarity::diagnostic::{Diagnostic as ClarityDiagnostic, Level as ClarityLevel};
use clarity_repl::clarity::docs::{
    make_api_reference, make_define_reference, make_keyword_reference,
};
use clarity_repl::clarity::functions::define::DefineFunctions;
use clarity_repl::clarity::functions::NativeFunctions;
use clarity_repl::clarity::types::{BlockInfoProperty, FunctionType};
use clarity_repl::clarity::variables::NativeVariables;
use lsp_types::Diagnostic as LspDiagnostic;
use lsp_types::{DiagnosticSeverity, Position, Range};

macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

pub(crate) use log;

pub fn clarity_diagnostics_to_lsp_type(
    diagnostics: &mut Vec<ClarityDiagnostic>,
) -> Vec<LspDiagnostic> {
    let mut dst = vec![];
    for d in diagnostics.iter_mut() {
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
            kind: CompletionItemKind::Module,
            detail: None,
            markdown_documentation: None,
            insert_text: Some(insert_text),
            insert_text_format: InsertTextFormat::Snippet,
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
            kind: CompletionItemKind::Event,
            detail: None,
            markdown_documentation: None,
            insert_text: Some(insert_text),
            insert_text_format: InsertTextFormat::Snippet,
        });
    }

    for (name, signature) in analysis.read_only_function_types.iter() {
        let insert_text = format!("{} {}", name, build_intellisense_args(signature).join(" "));
        intra_contract.push(CompletionItem {
            label: name.to_string(),
            kind: CompletionItemKind::Module,
            detail: None,
            markdown_documentation: None,
            insert_text: Some(insert_text),
            insert_text_format: InsertTextFormat::Snippet,
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
            kind: CompletionItemKind::Event,
            detail: None,
            markdown_documentation: None,
            insert_text: Some(insert_text),
            insert_text_format: InsertTextFormat::Snippet,
        });
    }

    for (name, signature) in analysis.private_function_types.iter() {
        let insert_text = format!("{} {}", name, build_intellisense_args(signature).join(" "));
        intra_contract.push(CompletionItem {
            label: name.to_string(),
            kind: CompletionItemKind::Module,
            detail: None,
            markdown_documentation: None,
            insert_text: Some(insert_text),
            insert_text_format: InsertTextFormat::Snippet,
        });
    }

    CompletionMaps {
        inter_contract,
        intra_contract,
        data_fields: vec![],
    }
}

pub fn build_default_native_keywords_list() -> Vec<CompletionItem> {
    let native_functions: Vec<CompletionItem> = NativeFunctions::ALL
        .iter()
        .map(|func| {
            let api = make_api_reference(&func);
            CompletionItem {
                label: api.name.to_string(),
                kind: CompletionItemKind::Function,
                detail: Some(api.name.to_string()),
                markdown_documentation: Some(api.description.to_string()),
                insert_text: Some(api.snippet.clone()),
                insert_text_format: InsertTextFormat::Snippet,
            }
        })
        .collect();

    let define_functions: Vec<CompletionItem> = DefineFunctions::ALL
        .iter()
        .map(|func| {
            let api = make_define_reference(&func);
            CompletionItem {
                label: api.name.to_string(),
                kind: CompletionItemKind::Class,
                detail: Some(api.name.to_string()),
                markdown_documentation: Some(api.description.to_string()),
                insert_text: Some(api.snippet.clone()),
                insert_text_format: InsertTextFormat::Snippet,
            }
        })
        .collect();

    let native_variables: Vec<CompletionItem> = NativeVariables::ALL
        .iter()
        .map(|var| {
            let api = make_keyword_reference(&var);
            CompletionItem {
                label: api.name.to_string(),
                kind: CompletionItemKind::Field,
                detail: Some(api.name.to_string()),
                markdown_documentation: Some(api.description.to_string()),
                insert_text: Some(api.snippet.to_string()),
                insert_text_format: InsertTextFormat::PlainText,
            }
        })
        .collect();

    let block_properties: Vec<CompletionItem> = BlockInfoProperty::ALL_NAMES
        .to_vec()
        .iter()
        .map(|func| CompletionItem {
            label: func.to_string(),
            kind: CompletionItemKind::Field,
            detail: None,
            markdown_documentation: None,
            insert_text: Some(func.to_string()),
            insert_text_format: InsertTextFormat::PlainText,
        })
        .collect();

    let types = vec![
        "uint",
        "int",
        "bool",
        "list",
        "tuple",
        "buff",
        "string-ascii",
        "string-utf8",
        "option",
        "response",
        "principal",
    ]
    .iter()
    .map(|var| CompletionItem {
        label: var.to_string(),
        kind: CompletionItemKind::TypeParameter,
        detail: None,
        markdown_documentation: None,
        insert_text: Some(var.to_string()),
        insert_text_format: InsertTextFormat::PlainText,
    })
    .collect();

    let items = vec![
        native_functions,
        define_functions,
        native_variables,
        block_properties,
        types,
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<CompletionItem>>();
    items
}

use clarinet_files::FileLocation;
use lsp_types::Url;

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
