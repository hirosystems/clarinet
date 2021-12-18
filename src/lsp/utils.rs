use clarity_repl::clarity::analysis::ContractAnalysis;
use clarity_repl::clarity::diagnostic::Diagnostic as ClarityDiagnostic;
use clarity_repl::clarity::docs::{
    make_api_reference, make_define_reference, make_keyword_reference,
};
use clarity_repl::clarity::functions::define::DefineFunctions;
use clarity_repl::clarity::functions::NativeFunctions;
use clarity_repl::clarity::types::{BlockInfoProperty, FunctionType};
use clarity_repl::clarity::variables::NativeVariables;
use tower_lsp::lsp_types::Diagnostic as LspDiagnostic;
use tower_lsp::lsp_types::*;

use super::clarity_language_backend::CompletionMaps;

pub fn convert_clarity_diagnotic_to_lsp_diagnostic(diagnostic: ClarityDiagnostic) -> LspDiagnostic {
    let range = match diagnostic.spans.len() {
        0 => Range::default(),
        _ => Range {
            start: Position {
                line: diagnostic.spans[0].start_line - 1,
                character: diagnostic.spans[0].start_column,
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
        severity: Some(DiagnosticSeverity::Error),
        code: None,
        code_description: None,
        source: Some("clarity".to_string()),
        message: diagnostic.message,
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
            kind: Some(CompletionItemKind::Module),
            detail: None,
            documentation: None,
            deprecated: None,
            preselect: None,
            sort_text: None,
            filter_text: None,
            insert_text: Some(insert_text),
            insert_text_format: Some(InsertTextFormat::Snippet),
            insert_text_mode: None,
            text_edit: None,
            additional_text_edits: None,
            command: None,
            commit_characters: None,
            data: None,
            tags: None,
        });

        let label = format!(
            "contract-call::{}::{}",
            analysis.contract_identifier.name.to_string(),
            name.to_string()
        );
        let insert = format!("{} {}", name, build_intellisense_args(signature).join(" "));
        let insert_text = format!(
            "contract-call? .{} {} {}",
            analysis.contract_identifier.name.to_string(),
            name.to_string(),
            build_intellisense_args(signature).join(" ")
        );
        inter_contract.push(CompletionItem {
            label,
            kind: Some(CompletionItemKind::Event),
            detail: None,
            documentation: None,
            deprecated: None,
            preselect: None,
            sort_text: None,
            filter_text: None,
            insert_text: Some(insert_text),
            insert_text_format: Some(InsertTextFormat::Snippet),
            insert_text_mode: None,
            text_edit: None,
            additional_text_edits: None,
            command: None,
            commit_characters: None,
            data: None,
            tags: None,
        });
    }

    for (name, signature) in analysis.read_only_function_types.iter() {
        let insert_text = format!("{} {}", name, build_intellisense_args(signature).join(" "));
        intra_contract.push(CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::Module),
            detail: None,
            documentation: None,
            deprecated: None,
            preselect: None,
            sort_text: None,
            filter_text: None,
            insert_text: Some(insert_text),
            insert_text_format: Some(InsertTextFormat::Snippet),
            insert_text_mode: None,
            text_edit: None,
            additional_text_edits: None,
            command: None,
            commit_characters: None,
            data: None,
            tags: None,
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
            kind: Some(CompletionItemKind::Event),
            detail: None,
            documentation: None,
            deprecated: None,
            preselect: None,
            sort_text: None,
            filter_text: None,
            insert_text: Some(insert_text),
            insert_text_format: Some(InsertTextFormat::Snippet),
            insert_text_mode: None,
            text_edit: None,
            additional_text_edits: None,
            command: None,
            commit_characters: None,
            data: None,
            tags: None,
        });
    }

    for (name, signature) in analysis.private_function_types.iter() {
        let insert_text = format!("{} {}", name, build_intellisense_args(signature).join(" "));
        intra_contract.push(CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::Module),
            detail: None,
            documentation: None,
            deprecated: None,
            preselect: None,
            sort_text: None,
            filter_text: None,
            insert_text: Some(insert_text),
            insert_text_format: Some(InsertTextFormat::Snippet),
            insert_text_mode: None,
            text_edit: None,
            additional_text_edits: None,
            command: None,
            commit_characters: None,
            data: None,
            tags: None,
        });
    }

    CompletionMaps {
        inter_contract,
        intra_contract,
    }
}

pub fn build_default_native_keywords_list() -> Vec<CompletionItem> {
    let native_functions: Vec<CompletionItem> = NativeFunctions::ALL
        .iter()
        .map(|func| {
            let api = make_api_reference(&func);
            CompletionItem {
                label: api.name.to_string(),
                kind: Some(CompletionItemKind::Function),
                detail: Some(api.name.to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: api.description.to_string(),
                })),
                deprecated: None,
                preselect: None,
                sort_text: None,
                filter_text: None,
                insert_text: Some(api.snippet.clone()),
                insert_text_format: Some(InsertTextFormat::Snippet),
                insert_text_mode: None,
                text_edit: None,
                additional_text_edits: None,
                command: None,
                commit_characters: None,
                data: None,
                tags: None,
            }
        })
        .collect();

    let define_functions: Vec<CompletionItem> = DefineFunctions::ALL
        .iter()
        .map(|func| {
            let api = make_define_reference(&func);
            CompletionItem {
                label: api.name.to_string(),
                kind: Some(CompletionItemKind::Class),
                detail: Some(api.name.to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: api.description.to_string(),
                })),
                deprecated: None,
                preselect: None,
                sort_text: None,
                filter_text: None,
                insert_text: Some(api.snippet.clone()),
                insert_text_format: Some(InsertTextFormat::Snippet),
                insert_text_mode: None,
                text_edit: None,
                additional_text_edits: None,
                command: None,
                commit_characters: None,
                data: None,
                tags: None,
            }
        })
        .collect();

    let native_variables: Vec<CompletionItem> = NativeVariables::ALL
        .iter()
        .map(|var| {
            let api = make_keyword_reference(&var);
            CompletionItem {
                label: api.name.to_string(),
                kind: Some(CompletionItemKind::Field),
                detail: Some(api.name.to_string()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: api.description.to_string(),
                })),
                deprecated: None,
                preselect: None,
                sort_text: None,
                filter_text: None,
                insert_text: Some(api.snippet.to_string()),
                insert_text_format: Some(InsertTextFormat::PlainText),
                insert_text_mode: None,
                text_edit: None,
                additional_text_edits: None,
                command: None,
                commit_characters: None,
                data: None,
                tags: None,
            }
        })
        .collect();

    let block_properties: Vec<CompletionItem> = BlockInfoProperty::ALL_NAMES
        .to_vec()
        .iter()
        .map(|func| CompletionItem::new_simple(func.to_string(), "".to_string()))
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
    ]
    .iter()
    .map(|var| CompletionItem {
        label: var.to_string(),
        kind: Some(CompletionItemKind::TypeParameter),
        detail: None,
        documentation: None,
        deprecated: None,
        preselect: None,
        sort_text: None,
        filter_text: None,
        insert_text: Some(var.to_string()),
        insert_text_format: Some(InsertTextFormat::PlainText),
        insert_text_mode: None,
        text_edit: None,
        additional_text_edits: None,
        command: None,
        commit_characters: None,
        data: None,
        tags: None,
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
