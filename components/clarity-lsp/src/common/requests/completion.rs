use clarity_repl::clarity::{
    docs::{make_api_reference, make_define_reference, make_keyword_reference},
    functions::{define::DefineFunctions, NativeFunctions},
    variables::NativeVariables,
    vm::types::BlockInfoProperty,
    ClarityVersion,
};
use lazy_static::lazy_static;
use lsp_types::{
    CompletionItem, CompletionItemKind, Documentation, InsertTextFormat, MarkupContent, MarkupKind,
    Position,
};

pub fn check_if_should_wrap(source: &str, position: &Position) -> bool {
    if let Some(line) = source
        .lines()
        .collect::<Vec<&str>>()
        .get(position.line as usize)
    {
        let mut chars = line.chars();
        let mut index = position.character as usize;
        while index > 0 {
            index -= 1;

            match chars.nth(index) {
                Some('(') => return false,
                Some(char) => {
                    if char.is_whitespace() {
                        return true;
                    }
                }
                None => return true,
            }
        }
    }
    true
}

pub fn build_default_native_keywords_list(version: ClarityVersion) -> Vec<CompletionItem> {
    let clarity2_aliased_functions: Vec<NativeFunctions> =
        vec![NativeFunctions::ElementAt, NativeFunctions::IndexOf];

    let native_functions: Vec<CompletionItem> = NativeFunctions::ALL
        .iter()
        .filter_map(|func| {
            let mut api = make_api_reference(&func);
            if api.version > version {
                return None;
            }
            if clarity2_aliased_functions.contains(func) {
                if version >= ClarityVersion::Clarity2 {
                    return None;
                } else if api.version == ClarityVersion::Clarity1 {
                    api.snippet = api.snippet.replace("?", "");
                }
            }

            Some(CompletionItem {
                label: api.name.to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(api.name),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: api.description,
                })),
                insert_text: Some(api.snippet.clone()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            })
        })
        .collect();

    let define_functions: Vec<CompletionItem> = DefineFunctions::ALL
        .iter()
        .filter_map(|func| {
            let api = make_define_reference(&func);
            if api.version > version {
                return None;
            }
            Some(CompletionItem {
                label: api.name.to_string(),
                kind: Some(CompletionItemKind::CLASS),
                detail: Some(api.name),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: api.description,
                })),
                insert_text: Some(api.snippet.clone()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            })
        })
        .collect();

    let native_variables: Vec<CompletionItem> = NativeVariables::ALL
        .iter()
        .filter_map(|var| {
            if let Some(api) = make_keyword_reference(&var) {
                if api.version > version {
                    return None;
                }
                Some(CompletionItem {
                    label: api.name.to_string(),
                    kind: Some(CompletionItemKind::FIELD),
                    detail: Some(api.name.to_string()),
                    documentation: Some(Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: api.description.to_string(),
                    })),
                    insert_text: Some(api.snippet.to_string()),
                    insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                    ..Default::default()
                })
            } else {
                None
            }
        })
        .collect();

    let block_properties: Vec<CompletionItem> = BlockInfoProperty::ALL
        .iter()
        .filter_map(|var| {
            if var.get_version() > version {
                return None;
            }
            Some(CompletionItem {
                label: var.to_string(),
                kind: Some(CompletionItemKind::FIELD),
                insert_text: Some(var.to_string()),
                insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                ..Default::default()
            })
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
    .map(|t| CompletionItem {
        label: t.to_string(),
        kind: Some(CompletionItemKind::TYPE_PARAMETER),
        insert_text: Some(t.to_string()),
        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
        ..Default::default()
    })
    .collect();

    vec![
        native_functions,
        define_functions,
        native_variables,
        block_properties,
        types,
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<CompletionItem>>()
}

lazy_static! {
    pub static ref COMPLETION_ITEMS_CLARITY_1: Vec<CompletionItem> =
        build_default_native_keywords_list(ClarityVersion::Clarity1);
    pub static ref COMPLETION_ITEMS_CLARITY_2: Vec<CompletionItem> =
        build_default_native_keywords_list(ClarityVersion::Clarity2);
}
