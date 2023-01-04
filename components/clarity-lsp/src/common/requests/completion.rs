use clarity_repl::clarity::{
    docs::{make_api_reference, make_define_reference, make_keyword_reference},
    functions::{define::DefineFunctions, NativeFunctions},
    variables::NativeVariables,
    vm::types::BlockInfoProperty,
    ClarityVersion, SymbolicExpression,
};
use lazy_static::lazy_static;
use lsp_types::{
    CompletionItem, CompletionItemKind, Documentation, InsertTextFormat, MarkupContent, MarkupKind,
    Position,
};

lazy_static! {
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
    pub static ref COMPLETION_ITEMS_CLARITY_1: Vec<CompletionItem> =
        build_default_native_keywords_list(ClarityVersion::Clarity1);
    pub static ref COMPLETION_ITEMS_CLARITY_2: Vec<CompletionItem> =
        build_default_native_keywords_list(ClarityVersion::Clarity2);
}

#[derive(Clone, Debug, Default)]
pub struct ContractDefinedData {
    pub vars: Vec<String>,
    pub maps: Vec<String>,
    pub fts: Vec<String>,
    pub nfts: Vec<String>,
}

pub fn get_contract_defined_data(
    expressions: Option<&Vec<SymbolicExpression>>,
) -> Option<ContractDefinedData> {
    let mut defined_data = ContractDefinedData {
        ..Default::default()
    };

    for expression in expressions? {
        let (define_function, args) = expression.match_list()?.split_first()?;
        match DefineFunctions::lookup_by_name(define_function.match_atom()?)? {
            DefineFunctions::PersistedVariable => defined_data
                .vars
                .push(args.first()?.match_atom()?.to_string()),
            DefineFunctions::Map => defined_data
                .maps
                .push(args.first()?.match_atom()?.to_string()),
            DefineFunctions::FungibleToken => defined_data
                .fts
                .push(args.first()?.match_atom()?.to_string()),
            DefineFunctions::NonFungibleToken => defined_data
                .nfts
                .push(args.first()?.match_atom()?.to_string()),
            _ => (),
        }
    }
    Some(defined_data)
}

#[cfg(test)]
mod get_contract_defined_data_tests {
    use clarity_repl::clarity::ast::build_ast_with_rules;
    use clarity_repl::clarity::stacks_common::types::StacksEpochId;
    use clarity_repl::clarity::{vm::types::QualifiedContractIdentifier, ClarityVersion};

    use super::{get_contract_defined_data, ContractDefinedData};

    fn get_defined_data(source: &str) -> Option<ContractDefinedData> {
        let contract_ast = build_ast_with_rules(
            &QualifiedContractIdentifier::transient(),
            source,
            &mut (),
            ClarityVersion::Clarity2,
            StacksEpochId::Epoch21,
            clarity_repl::clarity::ast::ASTRules::Typical,
        )
        .unwrap();
        get_contract_defined_data(Some(&contract_ast.expressions))
    }

    #[test]
    fn get_data_vars() {
        let data = get_defined_data(
            "(define-data-var counter uint u1) (define-data-var is-active bool true)",
        )
        .unwrap_or_default();
        assert_eq!(data.vars, ["counter", "is-active"]);
    }

    #[test]
    fn get_map() {
        let data = get_defined_data("(define-map names principal { name: (buff 48) })")
            .unwrap_or_default();
        assert_eq!(data.maps, ["names"]);
    }

    #[test]
    fn get_fts() {
        let data = get_defined_data("(define-fungible-token clarity-coin)").unwrap_or_default();
        assert_eq!(data.fts, ["clarity-coin"]);
    }

    #[test]
    fn get_nfts() {
        let data =
            get_defined_data("(define-non-fungible-token bitcoin-nft uint)").unwrap_or_default();
        assert_eq!(data.nfts, ["bitcoin-nft"]);
    }
}

pub fn populate_snippet_with_options(
    name: &String,
    snippet: &String,
    defined_data: &ContractDefinedData,
) -> String {
    if VAR_FUNCTIONS.contains(name) && defined_data.vars.len() > 0 {
        let choices = defined_data.vars.join(",");
        return snippet.replace("${1:var}", &format!("${{1|{:}|}}", choices));
    } else if MAP_FUNCTIONS.contains(name) && defined_data.maps.len() > 0 {
        let choices = defined_data.maps.join(",");
        return snippet.replace("${1:map-name}", &format!("${{1|{:}|}}", choices));
    } else if FT_FUNCTIONS.contains(name) && defined_data.fts.len() > 0 {
        let choices = defined_data.fts.join(",");
        return snippet.replace("${1:token-name}", &format!("${{1|{:}|}}", choices));
    } else if NFT_FUNCTIONS.contains(name) && defined_data.nfts.len() > 0 {
        let choices = defined_data.nfts.join(",");
        return snippet.replace("${1:asset-name}", &format!("${{1|{:}|}}", choices));
    }
    return snippet.to_string();
}

#[cfg(test)]
mod populate_snippet_with_options_tests {
    use clarity_repl::clarity::ast::build_ast_with_rules;
    use clarity_repl::clarity::stacks_common::types::StacksEpochId;
    use clarity_repl::clarity::{vm::types::QualifiedContractIdentifier, ClarityVersion};

    use super::{get_contract_defined_data, populate_snippet_with_options, ContractDefinedData};

    fn get_defined_data(source: &str) -> Option<ContractDefinedData> {
        let contract_ast = build_ast_with_rules(
            &QualifiedContractIdentifier::transient(),
            source,
            &mut (),
            ClarityVersion::Clarity2,
            StacksEpochId::Epoch21,
            clarity_repl::clarity::ast::ASTRules::Typical,
        )
        .unwrap();
        get_contract_defined_data(Some(&contract_ast.expressions))
    }

    #[test]
    fn get_data_vars_snippet() {
        let data = get_defined_data(
            "(define-data-var counter uint u1) (define-data-var is-active bool true)",
        )
        .unwrap_or_default();

        let snippet = populate_snippet_with_options(
            &"var-get".to_string(),
            &"var-get ${1:var}".to_string(),
            &data,
        );
        assert_eq!(snippet, "var-get ${1|counter,is-active|}");
    }

    #[test]
    fn get_map_snippet() {
        let data = get_defined_data("(define-map names principal { name: (buff 48) })")
            .unwrap_or_default();

        let snippet = populate_snippet_with_options(
            &"map-get?".to_string(),
            &"map-get? ${1:map-name} ${2:key-tuple}".to_string(),
            &data,
        );
        assert_eq!(snippet, "map-get? ${1|names|} ${2:key-tuple}");
    }

    #[test]
    fn get_fts_snippet() {
        let data = get_defined_data("(define-fungible-token btc u21)").unwrap_or_default();
        let snippet = populate_snippet_with_options(
            &"ft-mint?".to_string(),
            &"ft-mint? ${1:token-name} ${2:amount} ${3:recipient}".to_string(),
            &data,
        );
        assert_eq!(snippet, "ft-mint? ${1|btc|} ${2:amount} ${3:recipient}");
    }

    #[test]
    fn get_nfts_snippet() {
        let data =
            get_defined_data("(define-non-fungible-token bitcoin-nft uint)").unwrap_or_default();
        let snippet = populate_snippet_with_options(
            &"nft-mint?".to_string(),
            &"nft-mint? ${1:asset-name} ${2:asset-identifier} ${3:recipient}".to_string(),
            &data,
        );
        assert_eq!(
            snippet,
            "nft-mint? ${1|bitcoin-nft|} ${2:asset-identifier} ${3:recipient}"
        );
    }
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
