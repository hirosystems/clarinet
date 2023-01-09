use std::{collections::HashMap, vec};

use clarity_repl::{
    analysis::ast_visitor::{traverse, ASTVisitor, TypedVar},
    clarity::{
        analysis::ContractAnalysis,
        docs::{make_api_reference, make_define_reference, make_keyword_reference},
        functions::{define::DefineFunctions, NativeFunctions},
        variables::NativeVariables,
        vm::types::{BlockInfoProperty, FunctionType, TypeSignature},
        ClarityName, ClarityVersion, SymbolicExpression,
    },
};
use lazy_static::lazy_static;
use lsp_types::{
    CompletionItem, CompletionItemKind, Documentation, InsertTextFormat, MarkupContent, MarkupKind,
    Position,
};

use super::helpers::is_position_within_span;

lazy_static! {
    static ref COMPLETION_ITEMS_CLARITY_1: Vec<CompletionItem> =
        build_default_native_keywords_list(ClarityVersion::Clarity1);
    static ref COMPLETION_ITEMS_CLARITY_2: Vec<CompletionItem> =
        build_default_native_keywords_list(ClarityVersion::Clarity2);
    static ref VAR_FUNCTIONS: Vec<String> = vec![
        NativeFunctions::SetVar.to_string(),
        NativeFunctions::FetchVar.to_string(),
    ];
    static ref MAP_FUNCTIONS: Vec<String> = vec![
        NativeFunctions::InsertEntry.to_string(),
        NativeFunctions::FetchEntry.to_string(),
        NativeFunctions::SetEntry.to_string(),
        NativeFunctions::DeleteEntry.to_string(),
    ];
    static ref FT_FUNCTIONS: Vec<String> = vec![
        NativeFunctions::GetTokenBalance.to_string(),
        NativeFunctions::GetTokenSupply.to_string(),
        NativeFunctions::BurnToken.to_string(),
        NativeFunctions::MintToken.to_string(),
        NativeFunctions::TransferToken.to_string(),
    ];
    static ref NFT_FUNCTIONS: Vec<String> = vec![
        NativeFunctions::GetAssetOwner.to_string(),
        NativeFunctions::BurnAsset.to_string(),
        NativeFunctions::MintAsset.to_string(),
        NativeFunctions::TransferAsset.to_string(),
    ];
}

#[derive(Clone, Debug, Default)]
pub struct ContractDefinedData {
    position: Position,
    consts: Vec<(String, String)>,
    locals: Vec<(String, String)>,
    pub vars: Vec<String>,
    pub maps: Vec<String>,
    pub fts: Vec<String>,
    pub nfts: Vec<String>,
    pub functions_completion_items: Vec<CompletionItem>,
}

impl<'a> ContractDefinedData {
    pub fn new(expressions: &Vec<SymbolicExpression>, position: &Position) -> Self {
        let mut defined_data = ContractDefinedData {
            position: position.clone(),
            ..Default::default()
        };
        traverse(&mut defined_data, &expressions);
        defined_data
    }

    // this methods is in charge of:
    // 1. set the function completion item with its arguments
    // 2. set the local binding names if the position is within this function
    fn set_function_completion_with_bindings(
        &mut self,
        expr: &SymbolicExpression,
        name: &ClarityName,
        parameters: &Vec<TypedVar<'a>>,
    ) {
        let mut completion_args: Vec<String> = vec![];
        for (i, typed_var) in parameters.iter().enumerate() {
            if let Ok(signature) = TypeSignature::parse_type_repr(typed_var.type_expr, &mut ()) {
                completion_args.push(format!("${{{}:{}:{}}}", i + 1, typed_var.name, signature));

                if is_position_within_span(&self.position, &expr.span, 0) {
                    self.locals
                        .push((typed_var.name.to_string(), signature.to_string()));
                }
            };
        }

        self.functions_completion_items.push(CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::MODULE),
            insert_text: Some(format!("{} {}", name, completion_args.join(" "))),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
    }

    pub fn populate_snippet_with_options(&self, name: &String, snippet: &String) -> Option<String> {
        if VAR_FUNCTIONS.contains(name) && self.vars.len() > 0 {
            let choices = self.vars.join(",");
            return Some(snippet.replace("${1:var}", &format!("${{1|{}|}}", choices)));
        }
        if MAP_FUNCTIONS.contains(name) && self.maps.len() > 0 {
            let choices = self.maps.join(",");
            return Some(snippet.replace("${1:map-name}", &format!("${{1|{}|}}", choices)));
        }
        if FT_FUNCTIONS.contains(name) && self.fts.len() > 0 {
            let choices = self.fts.join(",");
            return Some(snippet.replace("${1:token-name}", &format!("${{1|{}|}}", choices)));
        }
        if NFT_FUNCTIONS.contains(name) && self.nfts.len() > 0 {
            let choices = self.nfts.join(",");
            return Some(snippet.replace("${1:asset-name}", &format!("${{1|{}|}}", choices)));
        }
        None
    }

    pub fn get_contract_completion_items(&self) -> Vec<CompletionItem> {
        [&self.consts[..], &self.locals[..]]
            .concat()
            .iter()
            .map(|(name, definition)| {
                CompletionItem::new_simple(name.to_string(), definition.to_string())
            })
            .collect()
    }
}

impl<'a> ASTVisitor<'a> for ContractDefinedData {
    fn visit_define_constant(
        &mut self,
        _expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        value: &'a SymbolicExpression,
    ) -> bool {
        self.consts.push((name.to_string(), value.to_string()));
        true
    }

    fn visit_define_data_var(
        &mut self,
        _expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        _data_type: &'a SymbolicExpression,
        _initial: &'a SymbolicExpression,
    ) -> bool {
        self.vars.push(name.to_string());
        true
    }

    fn visit_define_map(
        &mut self,
        _expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        _key_type: &'a SymbolicExpression,
        _value_type: &'a SymbolicExpression,
    ) -> bool {
        self.maps.push(name.to_string());
        true
    }

    fn visit_define_ft(
        &mut self,
        _expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        _supply: Option<&'a SymbolicExpression>,
    ) -> bool {
        self.fts.push(name.to_string());
        true
    }

    fn visit_define_nft(
        &mut self,
        _expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        _nft_type: &'a SymbolicExpression,
    ) -> bool {
        self.nfts.push(name.to_string());
        true
    }

    fn visit_define_public(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        parameters: Option<Vec<clarity_repl::analysis::ast_visitor::TypedVar<'a>>>,
        _body: &'a SymbolicExpression,
    ) -> bool {
        if let Some(parameters) = parameters {
            self.set_function_completion_with_bindings(expr, name, &parameters);
        }
        true
    }

    fn visit_define_read_only(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        parameters: Option<Vec<clarity_repl::analysis::ast_visitor::TypedVar<'a>>>,
        _body: &'a SymbolicExpression,
    ) -> bool {
        if let Some(parameters) = parameters {
            self.set_function_completion_with_bindings(expr, name, &parameters);
        }
        true
    }

    fn visit_define_private(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        parameters: Option<Vec<clarity_repl::analysis::ast_visitor::TypedVar<'a>>>,
        _body: &'a SymbolicExpression,
    ) -> bool {
        if let Some(parameters) = parameters {
            self.set_function_completion_with_bindings(expr, name, &parameters);
        }
        true
    }

    fn visit_let(
        &mut self,
        expr: &'a SymbolicExpression,
        bindings: &HashMap<&'a ClarityName, &'a SymbolicExpression>,
        _body: &'a [SymbolicExpression],
    ) -> bool {
        if is_position_within_span(&self.position, &expr.span, 0) {
            for (name, value) in bindings {
                self.locals.push((name.to_string(), value.to_string()));
            }
        }
        true
    }
}

fn build_contract_calls_args(signature: &FunctionType) -> (Vec<String>, Vec<String>) {
    let mut snippet_args = vec![];
    let mut doc_args = vec![];
    if let FunctionType::Fixed(function) = signature {
        for (i, arg) in function.args.iter().enumerate() {
            snippet_args.push(format!("${{{}:{}:{}}}", i + 1, arg.name, arg.signature));
            doc_args.push(format!("{} `{}`", arg.name, arg.signature));
        }
    }
    (snippet_args, doc_args)
}

pub fn get_contract_calls(analysis: &ContractAnalysis) -> Vec<CompletionItem> {
    let mut inter_contract = vec![];
    for (name, signature) in analysis
        .public_function_types
        .iter()
        .chain(analysis.read_only_function_types.iter())
    {
        let (snippet_args, doc_args) = build_contract_calls_args(signature);
        let label = format!(
            "contract-call? .{} {}",
            analysis.contract_identifier.name.to_string(),
            name.to_string()
        );
        let documentation = MarkupContent {
            kind: MarkupKind::Markdown,
            value: [vec![format!("**{}**", name.to_string())], doc_args]
                .concat()
                .join("\n\n"),
        };
        let insert_text = format!(
            "contract-call? .{} {} {}",
            analysis.contract_identifier.name.to_string(),
            name.to_string(),
            snippet_args.join(" ")
        );

        inter_contract.push(CompletionItem {
            label,
            detail: Some(analysis.contract_identifier.name.to_string()),
            documentation: Some(Documentation::MarkupContent(documentation)),
            kind: Some(CompletionItemKind::EVENT),
            insert_text: Some(insert_text),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
    }
    inter_contract
}

pub fn build_completion_item_list(
    clarity_version: &ClarityVersion,
    active_contract_defined_data: &ContractDefinedData,
    contract_calls: Vec<CompletionItem>,
    should_wrap: bool,
    include_native_placeholders: bool,
) -> Vec<CompletionItem> {
    let native_keywords = match clarity_version {
        ClarityVersion::Clarity1 => COMPLETION_ITEMS_CLARITY_1.to_vec(),
        ClarityVersion::Clarity2 => COMPLETION_ITEMS_CLARITY_2.to_vec(),
    };
    let mut completion_items = vec![];
    completion_items.append(&mut active_contract_defined_data.get_contract_completion_items());
    for mut item in [
        native_keywords,
        contract_calls,
        active_contract_defined_data
            .functions_completion_items
            .clone(),
    ]
    .concat()
    .drain(..)
    {
        match item.kind {
            Some(
                CompletionItemKind::EVENT
                | CompletionItemKind::FUNCTION
                | CompletionItemKind::MODULE
                | CompletionItemKind::CLASS,
            ) => {
                let mut snippet = item.insert_text.take().unwrap();
                let mut snippet_has_choices = false;
                if item.kind == Some(CompletionItemKind::FUNCTION) {
                    if let Some(populated_snippet) = active_contract_defined_data
                        .populate_snippet_with_options(&item.label, &snippet)
                    {
                        snippet_has_choices = true;
                        snippet = populated_snippet;
                    }
                }
                if !include_native_placeholders
                    && !snippet_has_choices
                    && (item.kind == Some(CompletionItemKind::FUNCTION)
                        || item.kind == Some(CompletionItemKind::CLASS))
                {
                    match item.label.as_str() {
                        "+ (add)" => {
                            snippet = "+".to_string();
                        }
                        "- (subtract)" => {
                            snippet = "-".to_string();
                        }
                        "/ (divide)" => {
                            snippet = "/".to_string();
                        }
                        "* (multiply)" => {
                            snippet = "*".to_string();
                        }
                        "< (less than)" => {
                            snippet = "<".to_string();
                        }
                        "<= (less than or equal)" => {
                            snippet = "<=".to_string();
                        }
                        "> (greater than)" => {
                            snippet = ">".to_string();
                        }
                        ">= (greater than or equal)" => {
                            snippet = ">=".to_string();
                        }
                        _ => snippet = item.label.clone(),
                    }
                    snippet.push_str(" $0");
                }

                item.insert_text = if should_wrap {
                    Some(format!("({})", snippet))
                } else {
                    Some(snippet)
                };
            }
            Some(CompletionItemKind::TYPE_PARAMETER) => {
                if should_wrap {
                    if let "tuple" | "buff" | "string-ascii" | "string-utf8" | "optional" | "list"
                    | "response" = item.label.as_str()
                    {
                        item.insert_text = Some(format!("({} $0)", item.label));
                        item.insert_text_format = Some(InsertTextFormat::SNIPPET);
                    }
                }
            }
            _ => {}
        }

        completion_items.push(item);
    }
    completion_items
}

pub fn check_if_should_wrap(source: &str, position: &Position) -> bool {
    if let Some(line) = source
        .lines()
        .collect::<Vec<&str>>()
        .get(position.line as usize)
    {
        let mut chars = line.chars();
        while let Some(char) = chars.next_back() {
            match char {
                '(' => return false,
                char => {
                    if char.is_whitespace() {
                        return true;
                    }
                }
            }
        }
    }
    true
}

pub fn build_default_native_keywords_list(version: ClarityVersion) -> Vec<CompletionItem> {
    let clarity2_aliased_functions: Vec<NativeFunctions> =
        vec![NativeFunctions::ElementAt, NativeFunctions::IndexOf];

    let command = lsp_types::Command {
        title: "triggerParameterHints".into(),
        command: "editor.action.triggerParameterHints".into(),
        arguments: None,
    };

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
                command: Some(command.clone()),
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
                command: Some(command.clone()),
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
        "optional",
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

#[cfg(test)]
mod get_contract_global_data_tests {
    use clarity_repl::clarity::ast::build_ast_with_rules;
    use clarity_repl::clarity::stacks_common::types::StacksEpochId;
    use clarity_repl::clarity::{vm::types::QualifiedContractIdentifier, ClarityVersion};
    use lsp_types::Position;

    use super::ContractDefinedData;

    fn get_defined_data(source: &str) -> ContractDefinedData {
        let contract_ast = build_ast_with_rules(
            &QualifiedContractIdentifier::transient(),
            source,
            &mut (),
            ClarityVersion::Clarity2,
            StacksEpochId::Epoch21,
            clarity_repl::clarity::ast::ASTRules::Typical,
        )
        .unwrap();
        ContractDefinedData::new(&contract_ast.expressions, &Position::default())
    }

    #[test]
    fn get_data_vars() {
        let data = get_defined_data(
            "(define-data-var counter uint u1) (define-data-var is-active bool true)",
        );
        assert_eq!(data.vars, ["counter", "is-active"]);
    }

    #[test]
    fn get_map() {
        let data = get_defined_data("(define-map names principal { name: (buff 48) })");
        assert_eq!(data.maps, ["names"]);
    }

    #[test]
    fn get_fts() {
        let data = get_defined_data("(define-fungible-token clarity-coin)");
        assert_eq!(data.fts, ["clarity-coin"]);
    }

    #[test]
    fn get_nfts() {
        let data = get_defined_data("(define-non-fungible-token bitcoin-nft uint)");
        assert_eq!(data.nfts, ["bitcoin-nft"]);
    }
}

#[cfg(test)]
mod get_contract_local_data_tests {
    use clarity_repl::clarity::ast::build_ast_with_rules;
    use clarity_repl::clarity::stacks_common::types::StacksEpochId;
    use clarity_repl::clarity::{vm::types::QualifiedContractIdentifier, ClarityVersion};
    use lsp_types::Position;

    use super::ContractDefinedData;

    fn get_defined_data(source: &str, position: &Position) -> ContractDefinedData {
        let contract_ast = build_ast_with_rules(
            &QualifiedContractIdentifier::transient(),
            source,
            &mut (),
            ClarityVersion::Clarity2,
            StacksEpochId::Epoch21,
            clarity_repl::clarity::ast::ASTRules::Typical,
        )
        .unwrap();
        ContractDefinedData::new(&contract_ast.expressions, position)
    }

    #[test]
    fn get_function_binding() {
        let data = get_defined_data(
            "(define-private (print-arg (arg int)) )",
            &Position {
                line: 1,
                character: 38,
            },
        );
        assert_eq!(data.locals, vec![("arg".to_string(), "int".to_string())]);
        let data = get_defined_data(
            "(define-private (print-arg (arg int)) )",
            &Position {
                line: 1,
                character: 40,
            },
        );
        assert_eq!(data.locals, vec![]);
    }

    #[test]
    fn get_let_binding() {
        let data = get_defined_data(
            "(let ((n u0)) )",
            &Position {
                line: 1,
                character: 15,
            },
        );
        assert_eq!(data.locals, vec![("n".to_string(), "u0".to_string())]);
    }
}

#[cfg(test)]
mod populate_snippet_with_options_tests {
    use clarity_repl::clarity::ast::build_ast_with_rules;
    use clarity_repl::clarity::stacks_common::types::StacksEpochId;
    use clarity_repl::clarity::{vm::types::QualifiedContractIdentifier, ClarityVersion};
    use lsp_types::Position;

    use super::ContractDefinedData;

    fn get_defined_data(source: &str) -> ContractDefinedData {
        let contract_ast = build_ast_with_rules(
            &QualifiedContractIdentifier::transient(),
            source,
            &mut (),
            ClarityVersion::Clarity2,
            StacksEpochId::Epoch21,
            clarity_repl::clarity::ast::ASTRules::Typical,
        )
        .unwrap();
        ContractDefinedData::new(&contract_ast.expressions, &Position::default())
    }

    #[test]
    fn get_data_vars_snippet() {
        let data = get_defined_data(
            "(define-data-var counter uint u1) (define-data-var is-active bool true)",
        );
        let snippet = data
            .populate_snippet_with_options(&"var-get".to_string(), &"var-get ${1:var}".to_string());
        assert_eq!(snippet, Some("var-get ${1|counter,is-active|}".to_string()));
    }

    #[test]
    fn get_map_snippet() {
        let data = get_defined_data("(define-map names principal { name: (buff 48) })");
        let snippet = data.populate_snippet_with_options(
            &"map-get?".to_string(),
            &"map-get? ${1:map-name} ${2:key-tuple}".to_string(),
        );
        assert_eq!(
            snippet,
            Some("map-get? ${1|names|} ${2:key-tuple}".to_string())
        );
    }

    #[test]
    fn get_fts_snippet() {
        let data = get_defined_data("(define-fungible-token btc u21)");
        let snippet = data.populate_snippet_with_options(
            &"ft-mint?".to_string(),
            &"ft-mint? ${1:token-name} ${2:amount} ${3:recipient}".to_string(),
        );
        assert_eq!(
            snippet,
            Some("ft-mint? ${1|btc|} ${2:amount} ${3:recipient}".to_string())
        );
    }

    #[test]
    fn get_nfts_snippet() {
        let data = get_defined_data("(define-non-fungible-token bitcoin-nft uint)");
        let snippet = data.populate_snippet_with_options(
            &"nft-mint?".to_string(),
            &"nft-mint? ${1:asset-name} ${2:asset-identifier} ${3:recipient}".to_string(),
        );
        assert_eq!(
            snippet,
            Some("nft-mint? ${1|bitcoin-nft|} ${2:asset-identifier} ${3:recipient}".to_string())
        );
    }
}
