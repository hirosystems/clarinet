use std::{collections::{BTreeMap, HashMap, HashSet}, vec};

use clarinet_files::FileLocation;
use clarity_repl::{
    analysis::ast_visitor::{traverse, ASTVisitor, TypedVar},
    clarity::{
        analysis::ContractAnalysis, 
        docs::{make_api_reference, make_define_reference, make_keyword_reference}, 
        functions::{define::DefineFunctions, NativeFunctions}, 
        representations::Span, 
        variables::NativeVariables, 
        vm::types::{
            signatures::{MethodSignature, MethodType}, BlockInfoProperty, 
            FunctionType, PrincipalData, QualifiedContractIdentifier, 
            StandardPrincipalData, TraitIdentifier, TypeSignature
        }, 
        ClarityName, ClarityVersion, StacksEpochId, SymbolicExpression
    },
    repl::{DEFAULT_CLARITY_VERSION, DEFAULT_EPOCH},
};
use lazy_static::lazy_static;
use lsp_types::{
    Command, CompletionContext, CompletionItem, CompletionItemKind, CompletionItemLabelDetails, CompletionTextEdit, 
    Documentation, InsertTextFormat, MarkupContent, MarkupKind, Position, Range, TextEdit
};
use regex::Regex;

use crate::state::{ActiveContractData, ProtocolState};

use super::helpers::{get_function_at_position, is_position_within_span};

lazy_static! {
    static ref COMPLETION_ITEMS_CLARITY_1: Vec<CompletionItem> =
        build_default_native_keywords_list(ClarityVersion::Clarity1);
    static ref COMPLETION_ITEMS_CLARITY_2: Vec<CompletionItem> =
        build_default_native_keywords_list(ClarityVersion::Clarity2);
    static ref COMPLETION_ITEMS_CLARITY_3: Vec<CompletionItem> =
        build_default_native_keywords_list(ClarityVersion::Clarity3);
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
    static ref ITERATOR_FUNCTIONS: Vec<String> = vec![
        NativeFunctions::Map.to_string(),
        NativeFunctions::Filter.to_string(),
        NativeFunctions::Fold.to_string(),
    ];
    static ref VALID_MAP_FUNCTIONS_CLARITY_1: Vec<CompletionItem> =
        build_map_valid_cb_completion_items(ClarityVersion::Clarity1);
    static ref VALID_MAP_FUNCTIONS_CLARITY_2: Vec<CompletionItem> =
        build_map_valid_cb_completion_items(ClarityVersion::Clarity2);
    static ref VALID_MAP_FUNCTIONS_CLARITY_3: Vec<CompletionItem> =
        build_map_valid_cb_completion_items(ClarityVersion::Clarity3);
    static ref VALID_FILTER_FUNCTIONS_CLARITY_1: Vec<CompletionItem> =
        build_filter_valid_cb_completion_items(ClarityVersion::Clarity1);
    static ref VALID_FILTER_FUNCTIONS_CLARITY_2: Vec<CompletionItem> =
        build_filter_valid_cb_completion_items(ClarityVersion::Clarity2);
    static ref VALID_FILTER_FUNCTIONS_CLARITY_3: Vec<CompletionItem> =
        build_filter_valid_cb_completion_items(ClarityVersion::Clarity3);
    static ref VALID_FOLD_FUNCTIONS_CLARITY_1: Vec<CompletionItem> =
        build_fold_valid_cb_completion_items(ClarityVersion::Clarity1);
    static ref VALID_FOLD_FUNCTIONS_CLARITY_2: Vec<CompletionItem> =
        build_fold_valid_cb_completion_items(ClarityVersion::Clarity2);
    static ref VALID_FOLD_FUNCTIONS_CLARITY_3: Vec<CompletionItem> =
        build_fold_valid_cb_completion_items(ClarityVersion::Clarity3);
}

#[derive(Clone, Debug)]
pub enum DefineFunctionType {
    FixedFunction{expects_type: bool},
    UseTrait,
    ImplTrait,
    None
}

impl Default for DefineFunctionType {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Clone, Debug)]
pub struct ContractDefinedData<'a> {
    epoch: StacksEpochId,
    clarity_version: ClarityVersion,
    pub position: Position,
    consts: Vec<(String, String)>,
    locals: Vec<(String, String)>,
    pub vars: Vec<String>,
    pub maps: Vec<String>,
    pub fts: Vec<String>,
    pub nfts: Vec<String>,
    pub function_at_position: DefineFunctionType,
    pub public_functions: HashMap<&'a ClarityName, Vec<TypeSignature>>,
    pub read_only_functions: HashMap<&'a ClarityName, Vec<TypeSignature>>,
    pub defined_traits: BTreeMap<&'a ClarityName, BTreeMap<ClarityName, MethodSignature>>,
    pub referenced_traits: HashMap<&'a ClarityName, TraitIdentifier>,
    pub referenced_traits_span: Option<(u32, u32)>,
    pub implemented_traits: HashSet<TraitIdentifier>,
    pub implemented_traits_span: Option<(u32, u32)>,
    pub functions_completion_items: Vec<CompletionItem>,
}

impl<'a> Default for ContractDefinedData<'a> {
    fn default() -> Self {
        Self { 
            epoch: DEFAULT_EPOCH, 
            clarity_version: DEFAULT_CLARITY_VERSION, 
            position: Default::default(), 
            consts: Default::default(), 
            locals: Default::default(), 
            vars: Default::default(), 
            maps: Default::default(), 
            fts: Default::default(), 
            nfts: Default::default(), 
            function_at_position: Default::default(), 
            public_functions: Default::default(), 
            read_only_functions: Default::default(), 
            defined_traits: Default::default(), 
            referenced_traits: Default::default(), 
            referenced_traits_span: Default::default(),
            implemented_traits: Default::default(), 
            implemented_traits_span: Default::default(),
            functions_completion_items: Default::default()
        }
    }
}

impl<'a> ContractDefinedData<'a> {
    pub fn new(
        expressions: &'a [SymbolicExpression], 
        position: Position, 
        epoch: StacksEpochId, 
        clarity_version: ClarityVersion
    ) -> Self {
        let mut defined_data = ContractDefinedData {
            position,
            epoch,
            clarity_version,
            ..Default::default()
        };
        traverse(&mut defined_data, expressions);
        defined_data
    }

    // this methods is in charge of:
    // 1. set the function completion item with its arguments
    // 2. set the local binding names if the position is within this function
    fn set_function_completion_with_bindings(
        &mut self,
        expr: &SymbolicExpression,
        name: &ClarityName,
        parameters: &[TypedVar<'a>],
    ) {
        let mut completion_args: Vec<String> = vec![];
        for (i, typed_var) in parameters.iter().enumerate() {
            if let Ok(signature) =
                TypeSignature::parse_type_repr(DEFAULT_EPOCH, typed_var.type_expr, &mut ())
            {
                completion_args.push(format!("${{{}:{}:{}}}", i + 1, typed_var.name, signature));

                if is_position_within_span(&self.position, &expr.span, 0) {
                    self.locals
                        .push((typed_var.name.to_string(), signature.to_string()));
                }
            };
        }

        let insert_text = match completion_args.len() {
            0 => Some(name.to_string()),
            _ => Some(format!("{} {}", name, completion_args.join(" "))),
        };

        self.functions_completion_items.push(CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::MODULE),
            insert_text,
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
    }

    pub fn populate_snippet_with_options(
        &self,
        version: &ClarityVersion,
        name: &String,
        snippet: &str,
    ) -> Option<String> {
        if VAR_FUNCTIONS.contains(name) && !self.vars.is_empty() {
            let choices = self.vars.join(",");
            return Some(snippet.replace("${1:var}", &format!("${{1|{}|}}", choices)));
        }
        if MAP_FUNCTIONS.contains(name) && !self.maps.is_empty() {
            let choices = self.maps.join(",");
            return Some(snippet.replace("${1:map-name}", &format!("${{1|{}|}}", choices)));
        }
        if FT_FUNCTIONS.contains(name) && !self.fts.is_empty() {
            let choices = self.fts.join(",");
            return Some(snippet.replace("${1:token-name}", &format!("${{1|{}|}}", choices)));
        }
        if NFT_FUNCTIONS.contains(name) && !self.nfts.is_empty() {
            let choices = self.nfts.join(",");
            return Some(snippet.replace("${1:asset-name}", &format!("${{1|{}|}}", choices)));
        }
        if ITERATOR_FUNCTIONS.contains(name) && !self.functions_completion_items.is_empty() {
            let mut choices = self
                .functions_completion_items
                .iter()
                .map(|f| f.label.to_string())
                .collect::<Vec<String>>()
                .join(",");
            choices.push(',');
            choices.push_str(
                &get_iterator_cb_completion_item(version, name)
                    .iter()
                    .map(|i| i.insert_text.clone().unwrap())
                    .collect::<Vec<String>>()
                    .join(","),
            );
            return Some(snippet.replace("${1:func}", &format!("${{1|{}|}}", choices)));
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

impl<'a> ASTVisitor<'a> for ContractDefinedData<'a> {
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
        match &expr.expr {
            clarity_repl::clarity::SymbolicExpressionType::List(list) => {
                let (_, args) = list.split_first().unwrap();
                let signature = args[0].match_list().unwrap();
                match signature.len() {
                    0 | 1 => {},
                    _ => {
                        if is_position_within_span(&zero_to_one_based(&self.position), expr.span(), 0u32){
                            self.function_at_position = DefineFunctionType::FixedFunction {
                                expects_type: check_type_expects(&self.position, &signature[1..])
                            }
                        }
                    },
                }
            },
            _ => {},
        }

        let parameters = parameters.unwrap_or_default();
        let mut args = Vec::new();
        for parameter in &parameters {
            if let Ok(arg) = TypeSignature::parse_type_repr(self.epoch, parameter.type_expr, &mut ()) {
                args.push(arg);
            }
        }
        self.set_function_completion_with_bindings(expr, name, &parameters);
        self.public_functions.insert(name, args);
        true
    }

    fn visit_define_read_only(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        parameters: Option<Vec<clarity_repl::analysis::ast_visitor::TypedVar<'a>>>,
        _body: &'a SymbolicExpression,
    ) -> bool {
        match &expr.expr {
            clarity_repl::clarity::SymbolicExpressionType::List(list) => {
                let (_, args) = list.split_first().unwrap();
                let signature = args[0].match_list().unwrap();
                match signature.len() {
                    0 | 1 => {},
                    _ => {
                        if is_position_within_span(&zero_to_one_based(&self.position), expr.span(), 0u32){
                            self.function_at_position = DefineFunctionType::FixedFunction { 
                                expects_type: check_type_expects(&self.position, &signature[1..])
                            }
                        }
                    },
                }
            },
            _ => {},
        }

        let parameters = parameters.unwrap_or_default();
        let mut args = Vec::new();
        for parameter in &parameters {
            if let Ok(arg) = TypeSignature::parse_type_repr(self.epoch, parameter.type_expr, &mut ()) {
                args.push(arg);
            }
        }
        self.set_function_completion_with_bindings(expr, name, &parameters);
        self.read_only_functions.insert(name, args);
        true
    }

    fn visit_define_private(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        parameters: Option<Vec<clarity_repl::analysis::ast_visitor::TypedVar<'a>>>,
        _body: &'a SymbolicExpression,
    ) -> bool {
        match &expr.expr {
            clarity_repl::clarity::SymbolicExpressionType::List(list) => {
                let (_, args) = list.split_first().unwrap();
                let signature = args[0].match_list().unwrap();
                match signature.len() {
                    0 | 1 => {},
                    _ => {
                        if is_position_within_span(&zero_to_one_based(&self.position), expr.span(), 0u32){
                            self.function_at_position = DefineFunctionType::FixedFunction { 
                                expects_type: check_type_expects(&self.position, &signature[1..])
                            }
                        }
                    },
                }
            },
            _ => {},
        }

        self.set_function_completion_with_bindings(expr, name, &parameters.unwrap_or_default());
        true
    }

    fn visit_let(
        &mut self,
        expr: &'a SymbolicExpression,
        bindings: &HashMap<&'a ClarityName, &'a SymbolicExpression>,
        _body: &'a [SymbolicExpression],
    ) -> bool {
        if is_position_within_span(&zero_to_one_based(&self.position), &expr.span, 0) {
            for (name, value) in bindings {
                self.locals.push((name.to_string(), value.to_string()));
            }
        }
        true
    }

    fn visit_define_trait(
        &mut self,
        _expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        functions: &'a [SymbolicExpression],
    ) -> bool {
        if let Ok(trait_signature) = TypeSignature::parse_trait_type_repr(
            functions, 
            &mut (), 
            self.epoch, 
            self.clarity_version
        ) {
            self.defined_traits.insert(name, trait_signature);
        }
        true
    }

    fn visit_use_trait(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        trait_identifier: &TraitIdentifier,
    ) -> bool {
        self.referenced_traits.insert(name, trait_identifier.clone());

        if let Some((_, e)) = &mut self.referenced_traits_span {
            *e = expr.span.end_line;
        } else {
            self.referenced_traits_span = Some((expr.span.start_line, expr.span.end_line));
        }

        if is_position_within_span(&zero_to_one_based(&self.position), expr.span(), 0u32) {
            self.function_at_position = DefineFunctionType::UseTrait;
        }

        true
    }

    fn visit_impl_trait(
        &mut self,
        expr: &'a SymbolicExpression,
        trait_identifier: &TraitIdentifier,
    ) -> bool {
        self.implemented_traits.insert(trait_identifier.clone());

        if let Some((_, e)) = &mut self.referenced_traits_span {
            *e = expr.span.end_line;
        } else {
            self.referenced_traits_span = Some((expr.span.start_line, expr.span.end_line));
        }

        if is_position_within_span(&zero_to_one_based(&self.position), expr.span(), 0u32)
            && self.position.character >= 12 {
            self.function_at_position = DefineFunctionType::ImplTrait;
        }

        true
    }
}

fn zero_to_one_based(position: &Position) -> Position {
    Position::new(position.line+1, position.character+1)
}

fn check_type_expects(position: &Position, list: &[SymbolicExpression]) -> bool {
    let pos = zero_to_one_based(position);
    for pair_list in list {
        if let Some(pair) = pair_list.match_list() {
            if 1 == pair.len() 
                && pos.line >= pair[0].span.end_line 
                && pos.character > pair[0].span.end_column 
                && is_position_within_span(&pos, pair_list.span(), 0u32) 
            {
                return true
            }

            if 2 == pair.len()
                && is_position_within_span(&pos, pair[1].span(), pair_list.span().end_column - pair[1].span().end_column)
            {
                return true
            }
        }
    }
    false
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
            analysis.contract_identifier.name, name,
        );
        let documentation = MarkupContent {
            kind: MarkupKind::Markdown,
            value: [vec![format!("**{}**", name.to_string())], doc_args]
                .concat()
                .join("\n\n"),
        };
        let insert_text = format!(
            "contract-call? .{} {} {}",
            analysis.contract_identifier.name,
            name,
            snippet_args.join(" "),
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

pub fn build_trait_completion_data(
    issuer: &StandardPrincipalData,
    contract_uri: &FileLocation,
    contract_defined_state: &ContractDefinedData,
    protocol_state: &ProtocolState,
    active_contract: &ActiveContractData,
    position: &Position,
    context: &Option<CompletionContext>,
) -> Option<Vec<CompletionItem>> {
    if let Some(Some(ch)) = &context.to_owned().map(|ctx| ctx.trigger_character) {
        'it: { if let DefineFunctionType::FixedFunction { expects_type: true} = &contract_defined_state.function_at_position
            {
                if "<" != &ch[0..1] {break 'it}
                return Some(
                    get_trait_alias_completion_data(
                        contract_uri, 
                        issuer, 
                        contract_defined_state, 
                        protocol_state
                    )
                )
            }
        }

        if "'" == &ch[0..1] {
            return Some(get_principal_completion_data(protocol_state));
        }

        if "." == &ch[0..1] {
            let precursor_token = active_contract.get_token_at_postion(position).unwrap();
            match precursor_token.as_str() {
                "." => return Some(get_contract_name_completion(issuer, protocol_state)),

                token if '\'' == token.chars().nth(0).unwrap() => {
                    if let Ok(principal) = PrincipalData::parse(&token[..token.len()-1]) {
                        match principal {
                            PrincipalData::Standard(principal) => return Some(
                                get_contract_name_completion(
                                    &principal, 
                                    protocol_state
                                )
                            ),

                            PrincipalData::Contract(contract) => return Some(
                                get_trait_name_completion(
                                    contract_uri, 
                                    contract_defined_state, 
                                    protocol_state, 
                                    contract,
                                    position
                                )
                            ),
                        }
                    }
                },

                token if '.' == token.chars().nth(0).unwrap() => {
                    let Ok(name) = token[1..token.len()-1].to_owned().try_into() else {return Some(vec![])};
                    let contract = QualifiedContractIdentifier::new(
                        issuer.clone(), 
                        name
                    );
                    return Some(
                        get_trait_name_completion(
                            contract_uri, 
                            contract_defined_state, 
                            protocol_state, 
                            contract, 
                            position
                        )
                    )
                },

                _ => {},
            }

            return Some(vec![])
        }
    }

    let precursor_token = active_contract.get_token_at_postion(position)?;
    let sub_tokens = precursor_token.split('.').collect::<Vec<_>>();
    let principal = PrincipalData::parse(&sub_tokens[..sub_tokens.len()-1].join("."));
    match (sub_tokens.len(), precursor_token.chars().next().unwrap()) {
        (2, '\'') | (3, '\'') if principal.is_ok() => {
            match principal.unwrap() {
                PrincipalData::Standard(standard_principal_data) => {
                    return Some(get_contract_name_completion(&standard_principal_data, protocol_state))
                },
                PrincipalData::Contract(qualified_contract_identifier) => {
                    return Some(
                        get_trait_name_completion(
                            contract_uri, 
                            contract_defined_state, 
                            protocol_state, 
                            qualified_contract_identifier, 
                            position
                        )
                    );
                },
            }
        },

        (1, '\'') => return Some(get_principal_completion_data(protocol_state)), 

        (2, '.') => return Some(get_contract_name_completion(issuer, protocol_state)),

        (3, '.') => {
            let Ok(name) = sub_tokens[1].to_owned().try_into() else {return Some(vec![])};
            let contract = QualifiedContractIdentifier::new(
                issuer.clone(), 
                name
            );
            return Some(
                get_trait_name_completion(
                    contract_uri, 
                    contract_defined_state, 
                    protocol_state, 
                    contract, 
                    position
                )
            );
        }

        (1, '<') => {
            if let DefineFunctionType::FixedFunction { expects_type: true} = &contract_defined_state.function_at_position {
                return Some(
                    get_trait_alias_completion_data(
                        contract_uri, 
                        issuer, 
                        contract_defined_state, 
                        protocol_state
                    )
                )
            }
        },

        (1, _) => {},
        (_, _) => return Some(vec![]),
    }

    None
}

fn get_use_trait_suggestions(
    pos: &Position,
    param: Option<u32>,
    contract_uri: &FileLocation,
    issuer: &StandardPrincipalData,
    contract_defined_state: &ContractDefinedData,
    protocol_state: &ProtocolState,
) -> Vec<CompletionItem> {
    let mut list = Vec::new();
    for (ident, _) in protocol_state.get_trait_definitions(contract_uri) {
        let (label, label_details, insert_text, additional_text_edits) = match (param, *issuer == ident.0.issuer) {
            (Some(0), true) => {
                let label = format!("{}", ident.1);
                let label_details = CompletionItemLabelDetails{
                    detail: Some(format!(" .{}.{}", ident.0.name, ident.1)),
                    description: None,
                };
                let insert_text = Some(format!("(use-trait {} .{}.{})", ident.1, ident.0.name, ident.1));
                let additional_text_edits = Some(vec![TextEdit::new(Range::new(Position::new(pos.line, 0), Position::new(pos.line, 999)), "".to_owned())]);

                (label, Some(label_details), insert_text, additional_text_edits)
            },

            (Some(0), false) => {
                let label = format!("{}", ident.1);
                let address = ident.0.issuer.to_address();
                let shorthand_address = format!("'{}..{}", &address[..3], &address[address.len()-3..address.len()]);
                let shorthand_trait_identifier = format!(" {}.{}.{}", shorthand_address, ident.0.name, ident.1);
                let label_details = CompletionItemLabelDetails{
                    detail: Some(shorthand_trait_identifier),
                    description: None,
                };

                let trait_identifier = format!("{}.{}.{}", address, ident.0.name, ident.1);
                let insert_text = Some(format!("(use-trait {} '{})", ident.1, trait_identifier));
                let additional_text_edits = Some(vec![TextEdit::new(Range::new(Position::new(pos.line, 0), Position::new(pos.line, 999)), "".to_owned())]);

                (label, Some(label_details), insert_text, additional_text_edits)
            },

            (Some(1), true) => {
                let label = format!(".{}.{}", ident.0.name, ident.1);
                let insert_text = format!(".{}.{}",ident.0.name, ident.1);

                (label, None, Some(insert_text), None)
            },

            (Some(1), false) => {
                let address = ident.0.issuer.to_address();
                let shorthand_address = format!("'{}..{}", &address[..3], &address[address.len()-3..address.len()]);
                let shorthand_trait_identifier = format!("{}.{}.{}", shorthand_address, ident.0.name, ident.1);
                let label = shorthand_trait_identifier;

                let trait_identifier = format!("{}.{}.{}", address, ident.0.name, ident.1);
                let insert_text = format!("'{}", trait_identifier);

                (label, None, Some(insert_text), None)
            },

            (_, _) => return vec![]
        };

        list.push(CompletionItem{
            label,
            label_details,
            insert_text,
            additional_text_edits,
            ..Default::default()
        })
    }
    list
}

fn get_trait_alias_completion_data(
    contract_uri: &FileLocation,
    issuer: &StandardPrincipalData,
    contract_defined_state: &ContractDefinedData,
    protocol_state: &ProtocolState,
) -> Vec<CompletionItem> {

    let mut list = Vec::new();

    for trait_alias in contract_defined_state.referenced_traits.keys()
    {
        list.push(CompletionItem {
            label: trait_alias.to_string(),
            kind: Some(CompletionItemKind::INTERFACE),
            detail: Some("trait-alias".to_string()),
            insert_text: Some(format!("{}>", trait_alias)),
            ..Default::default()
        });        
    }

    for (trait_identity, _) in protocol_state.get_trait_definitions(contract_uri) {
        if contract_defined_state.referenced_traits.contains_key(&trait_identity.1) {continue;}

        let (principal, contract) = match &trait_identity.0 {
            x if *issuer == x.issuer => ("".to_string(), &x.name),

            x => {
                (x.issuer.to_address(), &x.name)
            },
        };

        let (insert_line, extra_lines) = if let Some((s, _)) = contract_defined_state.referenced_traits_span {
            (s-1, "\n".to_owned())
        } else {
            (0, "\n\n".to_owned())
        };

        let insert_postion = Position::new(insert_line, 0u32);

        let detail = if !principal.is_empty(){
            let shorthand_address = format!("'{}..{}", &principal[..3], &principal[principal.len()-3..principal.len()]);
            Some(format!(" use-trait {} {}.{}.{}", trait_identity.1, shorthand_address, contract, trait_identity.1))
        } else {
            Some(format!(" use-trait {} .{}.{}", trait_identity.1, contract, trait_identity.1))
        };

        let new_text = if principal.is_empty() {
            format!("(use-trait {} .{}.{}){}", trait_identity.1, contract, trait_identity.1, extra_lines)
        } else {
            format!("(use-trait {} '{}.{}.{}){}", trait_identity.1, principal, contract, trait_identity.1, extra_lines)
        };

        list.push(CompletionItem {
            label: trait_identity.1.to_string(),
            label_details: Some(CompletionItemLabelDetails { 
                detail,
                description: None, 
            }),
            kind: Some(CompletionItemKind::INTERFACE),
            detail: Some("trait-alias".to_string()),
            insert_text: Some(format!("{}>", trait_identity.1)),
            additional_text_edits: Some(vec![TextEdit {
                range: Range { start: insert_postion, end: insert_postion},
                new_text,
            }]),
            ..Default::default()
        });
    }

    list
}

fn get_principal_completion_data(protocol_state: &ProtocolState) -> Vec<CompletionItem> {
    let mut set = HashSet::new();
    for address in protocol_state
        .get_contract_identifiers()
        .iter()
        .map(|x|x.issuer.to_address())
    {
        set.insert(address);
    }
    set.into_iter().map(|x| CompletionItem::new_simple(x, "".to_string())).collect::<Vec<_>>()
}

fn get_contract_name_completion(
    principal: &StandardPrincipalData,
    protocol_state: &ProtocolState, 
) -> Vec<CompletionItem> {
    let mut list = Vec::new();
    for contract in protocol_state.get_contract_identifiers() {
        if *principal == contract.issuer {
            list.push(CompletionItem::new_simple(contract.name.to_string(), "".to_string()));
        }
    }

    list
}

fn get_trait_name_completion(
    contract_uri: &FileLocation,
    contract_defined_state: &ContractDefinedData,
    protocol_state: &ProtocolState,
    contract: QualifiedContractIdentifier,
    pos: &Position,
) -> Vec<CompletionItem> {
    let mut list = Vec::new();

    match contract_defined_state.function_at_position {
        DefineFunctionType::ImplTrait => {
            for signature in protocol_state
                .get_trait_definitions(contract_uri)
                .iter()
                .filter(|x| contract == x.0.0)
            {
                let mut methods = String::new();
                let mut methods_are_some = false;
                methods.push_str("\n\n");

                for method in signature.1 {
                    match &method.1.define_type {
                        MethodType::ReadOnly => {
                            if !(contract_defined_state
                                .read_only_functions
                                .get(&method.0)
                                .is_some_and(|x| *x==method.1.args)) 
                            {
                                methods_are_some = true;
                                let mut params = String::new();

                                for (i, x) in method.1.args.iter().enumerate() {
                                    params.push_str(&format!(" (param{}-name {})", i+1, x)[..]);
                                }
                                
                                methods.push_str(format!("(define-read-only ({}{}) body)\n\n", method.0, params).as_str());
                            }
                        },

                        MethodType::Public => {
                            if !(contract_defined_state
                                .public_functions
                                .get(&method.0)
                                .is_some_and(|x| *x==method.1.args))
                            {
                                methods_are_some = true;
                                let mut params = String::new();

                                for (i, x) in method.1.args.iter().enumerate() {
                                    params.push_str(&format!(" (param{}-name {})", i+1, x)[..]);
                                }
                                    
                                methods.push_str(format!("(define-public ({}{}) body)\n\n", method.0, params).as_str());
                            }
                        },

                        MethodType::NotDefined => {
                            if !(contract_defined_state
                                .read_only_functions
                                .get(&method.0)
                                .is_some_and(|x| *x==method.1.args)) 
                                &&
                                !(contract_defined_state
                                .public_functions
                                .get(&method.0)
                                .is_some_and(|x| *x==method.1.args))
                            {
                                methods_are_some = true;
                                let mut params = String::new();

                                for (i, x) in method.1.args.iter().enumerate() {
                                    params.push_str(&format!(" (param{}-name {})", i+1, x)[..]);
                                }
        
                                methods.push_str(format!("(access-modifier-kind ({}{}) body)\n\n", method.0, params).as_str());
                            }
                        },
                    }
                }

                let position = Position::new(pos.line, 999);
                let methods = match methods_are_some {
                    true => Some(vec![TextEdit::new(lsp_types::Range { start: position, end: position }, methods)]),
                    false => None,
                };

                list.push(CompletionItem{
                    label: signature.0.1.to_string(),
                    additional_text_edits: methods,
                    ..Default::default()
                })
            }
        },

        _ => {
            for definition in protocol_state
                .get_trait_definitions(contract_uri)
                .iter()
                .filter(|x| contract == x.0.0)
            {
                list.push(CompletionItem::new_simple(definition.0.1.to_string(), "".to_string()));
            }
        }
    }

    list
}

pub fn get_impl_trait_suggestions(
    pos: &Position,
    contract_uri: &FileLocation,
    issuer: StandardPrincipalData,
    contract_defined_state: &ContractDefinedData,
    protocol_state: &ProtocolState,
) -> Vec<CompletionItem> {
    let mut list = Vec::new();

    for signature in protocol_state.get_trait_definitions(contract_uri) {
        if contract_defined_state
            .implemented_traits
            .contains(&TraitIdentifier::new(
                signature.0.0.issuer.clone(), 
                signature.0.0.name.clone(), 
                signature.0.1.clone())) 
        {
            continue;
        }

        let mut methods_to_insert = String::new();
        methods_to_insert.push_str("\n\n");
        for method in signature.1 {
            match method.1.define_type {
                MethodType::ReadOnly => {
                    if !(contract_defined_state
                        .read_only_functions
                        .get(&method.0)
                        .is_some_and(|x| *x == method.1.args))
                    {
                        let mut params = String::new();

                        for (i, x) in method.1.args.iter().enumerate() {
                            params.push_str(&format!(" (param{}-name {})", i+1, x)[..]);
                        }

                        methods_to_insert.push_str(format!("(define-read-only ({}{}) body)\n\n", method.0, params).as_str());
                    }
                },

                MethodType::Public => {
                    if !(contract_defined_state
                        .public_functions
                        .get(&method.0)
                        .is_some_and(|x| *x == method.1.args))
                    {
                        let mut params = String::new();

                        for (i, x) in method.1.args.iter().enumerate() {
                            params.push_str(&format!(" (param{}-name {})", i+1, x)[..]);
                        }

                        methods_to_insert.push_str(format!("(define-public ({}{}) body)\n\n", method.0, params).as_str());
                    }
                },

                MethodType::NotDefined => {
                    if !(contract_defined_state
                        .read_only_functions
                        .get(&method.0)
                        .is_some_and(|x| *x==method.1.args)) 
                        &&
                        !(contract_defined_state
                        .public_functions
                        .get(&method.0)
                        .is_some_and(|x| *x==method.1.args))
                    {
                        let mut params = String::new();

                        for (i, x) in method.1.args.iter().enumerate() {
                            params.push_str(&format!(" (param{}-name {})", i+1, x)[..]);
                        }

                        methods_to_insert.push_str(format!("(access-modifier-kind ({}{}) body)\n\n", method.0, params).as_str());
                    }
                },
            }
        }

        let additional_text_edits = match methods_to_insert.len() {
            2 => None,
            _ => Some(vec![(TextEdit::new(
                Range::new(Position::new(pos.line, 999), Position::new(pos.line, 999)), 
                methods_to_insert
            ))])
        };

        let shorthand_address = format!("{}..{}", 
            &signature.0.0.issuer.to_address()[..3], 
            &signature.0.0.issuer.to_address()[signature.0.0.issuer.to_address().len()-3..]);
        let label = format!("{}.{}.{}",shorthand_address, signature.0.0.name, signature.0.1);

        let insert_text = if issuer != signature.0.0.issuer {
            Some(format!("'{}.{}.{}", signature.0.0.issuer.to_address(), signature.0.0.name, signature.0.1))
        } else {
            Some(format!(".{}.{}", signature.0.0.name, signature.0.1))
        };

        list.push(CompletionItem{
            label,
            insert_text,
            additional_text_edits,
            ..Default::default()
        });

    }

    list
}

pub fn build_completion_item_list(
    clarity_version: &ClarityVersion,
    expressions: &Vec<SymbolicExpression>,
    contract_uri: &FileLocation,
    position: &Position,
    current_contract_issuer: Option<StandardPrincipalData>,
    active_contract_defined_data: &ContractDefinedData,
    protocol_state: &ProtocolState,
    contract_calls: Vec<CompletionItem>,
    should_wrap: bool,
    include_native_placeholders: bool,
) -> Vec<CompletionItem> {
    if let Some((function_name, param)) = get_function_at_position(position, expressions) {
        // - for var-*, map-*, ft-* or nft-* methods, return the corresponding data names
        let mut completion_strings: Option<Vec<String>> = None;
        match (function_name.to_string(), param) {
            (name, Some(0)) if VAR_FUNCTIONS.contains(&name) => completion_strings = Some(active_contract_defined_data.vars.clone()),

            (name, Some(0)) if MAP_FUNCTIONS.contains(&name) => completion_strings = Some(active_contract_defined_data.maps.clone()),

            (name, Some(0)) if FT_FUNCTIONS.contains(&name) => completion_strings = Some(active_contract_defined_data.fts.clone()),

            (name, Some(0)) if NFT_FUNCTIONS.contains(&name) => completion_strings = Some(active_contract_defined_data.nfts.clone()),

            (name, Some(0)) if name == *"impl-trait" && current_contract_issuer.is_some() => {
                let issuer = current_contract_issuer.unwrap();
                return get_impl_trait_suggestions(
                    &Position::new(position.line-1, position.character-1), 
                    contract_uri,
                    issuer,
                    active_contract_defined_data, 
                    protocol_state
                )
            },

            (name, _) if name == *"impl-trait" => return vec![],

            (name, param) if name == *"use-trait" && current_contract_issuer.is_some() => {
                let issuer = current_contract_issuer.unwrap();
                return get_use_trait_suggestions(
                    &Position::new(position.line-1, position.character-1), 
                    param,
                    contract_uri, 
                    &issuer,
                    active_contract_defined_data,
                    protocol_state
                )
            }

            (name, _) if name == *"use-trait" => return vec![],

            (_, _) => {}
        }

        if let Some(completion_strings) = completion_strings {
            return completion_strings
                .iter()
                .map(|s| CompletionItem::new_simple(String::from(s), String::from("")))
                .collect();
        }

        // - for iterator methods (filter, fold, map) return the list of available and valid functions
        if ITERATOR_FUNCTIONS.contains(&function_name.to_string()) && param == Some(0) {
            let mut completion_items: Vec<CompletionItem> = vec![];
            completion_items.append(
                &mut active_contract_defined_data
                    .functions_completion_items
                    .iter()
                    .map(|f| CompletionItem::new_simple(f.label.clone(), String::from("")))
                    .collect::<Vec<CompletionItem>>(),
            );
            completion_items.append(&mut get_iterator_cb_completion_item(
                clarity_version,
                &function_name.to_string(),
            ));
            return completion_items;
        }
    }

    let native_keywords = match clarity_version {
        ClarityVersion::Clarity1 => COMPLETION_ITEMS_CLARITY_1.to_vec(),
        ClarityVersion::Clarity2 => COMPLETION_ITEMS_CLARITY_2.to_vec(),
        ClarityVersion::Clarity3 => COMPLETION_ITEMS_CLARITY_3.to_vec(),
    };
    let placeholder_pattern = Regex::new(r" \$\{\d+:[\w-]+\}").unwrap();

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
                        .populate_snippet_with_options(clarity_version, &item.label, &snippet)
                    {
                        snippet_has_choices = true;
                        snippet = populated_snippet;
                    }
                }

                if !include_native_placeholders {
                    if snippet_has_choices {
                        // for var-*, map-*, ft-* and nft-* methods
                        // the variable name is kept but the other placeholders are removed
                        let updated_snippet =
                            placeholder_pattern.replace_all(&snippet, "").to_string();
                        if updated_snippet.ne(&snippet) {
                            snippet = updated_snippet;
                            snippet.push_str(" $0");
                        }
                    } else if item.kind == Some(CompletionItemKind::FUNCTION)
                        || item.kind == Some(CompletionItemKind::CLASS)
                    {
                        match item.label.as_str() {
                            "+ (add)"
                            | "- (subtract)"
                            | "/ (divide)"
                            | "* (multiply)"
                            | "< (less than)"
                            | "<= (less than or equal)"
                            | "> (greater than)"
                            | ">= (greater than or equal)" => {
                                snippet = item.label.split_whitespace().next().unwrap().to_string()
                            }
                            _ => snippet.clone_from(&item.label),
                        }
                        snippet.push_str(" $0");
                    }
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

        if *"impl-trait" == item.label {
            item.command = Some(Command::new(
                "triggerSuggest".into(), 
                "editor.action.triggerSuggest".into(), 
                None
            ));
        }

        if *"use-trait" == item.label {
            item.command = Some(Command::new(
                "triggerSuggest".into(), 
                "editor.action.triggerSuggest".into(), 
                None
            ));
        }

        completion_items.push(item);
    }
    completion_items
}

pub fn check_if_should_wrap(source: &str, position: &Position) -> bool {
    if let Some(line) = source.lines().nth(position.line as usize) {
        if position.character as usize > line.len() {
            return false;
        }

        let mut chars = line[..position.character as usize].chars();
        while let Some(char) = chars.next_back() {
            match char {
                '(' => return false,
                char if char.is_whitespace() => return true,
                _ => {}
            }
        }
    }
    true
}

pub fn build_default_native_keywords_list(version: ClarityVersion) -> Vec<CompletionItem> {
    let clarity2_aliased_functions: Vec<NativeFunctions> = vec![
        NativeFunctions::ElementAt,
        NativeFunctions::IndexOf,
        NativeFunctions::BitwiseXor,
    ];

    let command = lsp_types::Command {
        title: "triggerParameterHints".into(),
        command: "editor.action.triggerParameterHints".into(),
        arguments: None,
    };

    let native_functions: Vec<CompletionItem> = NativeFunctions::ALL
        .iter()
        .filter_map(|func| {
            let mut api = make_api_reference(func);
            if version < api.min_version
                || version > api.max_version.unwrap_or(ClarityVersion::latest())
            {
                return None;
            }
            if clarity2_aliased_functions.contains(func) {
                if version >= ClarityVersion::Clarity2 {
                    return None;
                } else if api.min_version == ClarityVersion::Clarity1 {
                    // only for element-at? and index-of?
                    api.snippet = api.snippet.replace('?', "");
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
            let api = make_define_reference(func);
            if version < api.min_version
                || version > api.max_version.unwrap_or(ClarityVersion::latest())
            {
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
            if let Some(api) = make_keyword_reference(var) {
                if version < api.min_version
                    || version > api.max_version.unwrap_or(ClarityVersion::latest())
                {
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
            if var.get_min_version() > version {
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

    let types = [
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

pub fn build_map_valid_cb_completion_items(version: ClarityVersion) -> Vec<CompletionItem> {
    [
        NativeFunctions::Add,
        NativeFunctions::Subtract,
        NativeFunctions::Multiply,
        NativeFunctions::Divide,
        NativeFunctions::CmpGeq,
        NativeFunctions::CmpLeq,
        NativeFunctions::CmpLess,
        NativeFunctions::CmpGreater,
        NativeFunctions::ToInt,
        NativeFunctions::ToUInt,
        NativeFunctions::Modulo,
        NativeFunctions::Power,
        NativeFunctions::Sqrti,
        NativeFunctions::Log2,
        NativeFunctions::BitwiseXor,
        NativeFunctions::And,
        NativeFunctions::Or,
        NativeFunctions::Not,
        NativeFunctions::BuffToIntLe,
        NativeFunctions::BuffToUIntLe,
        NativeFunctions::BuffToIntBe,
        NativeFunctions::BuffToUIntBe,
        NativeFunctions::IsStandard,
        NativeFunctions::PrincipalDestruct,
        NativeFunctions::PrincipalConstruct,
        NativeFunctions::StringToInt,
        NativeFunctions::StringToUInt,
        NativeFunctions::IntToAscii,
        NativeFunctions::IntToUtf8,
        NativeFunctions::Hash160,
        NativeFunctions::Sha256,
        NativeFunctions::Sha512,
        NativeFunctions::Sha512Trunc256,
        NativeFunctions::Keccak256,
        NativeFunctions::BitwiseAnd,
        NativeFunctions::BitwiseOr,
        NativeFunctions::BitwiseNot,
        NativeFunctions::BitwiseLShift,
        NativeFunctions::BitwiseRShift,
        NativeFunctions::BitwiseXor2,
    ]
    .iter()
    .filter_map(|func| build_iterator_cb_completion_item(func, version))
    .collect()
}

pub fn build_filter_valid_cb_completion_items(version: ClarityVersion) -> Vec<CompletionItem> {
    [
        NativeFunctions::And,
        NativeFunctions::Or,
        NativeFunctions::Not,
    ]
    .iter()
    .filter_map(|func| build_iterator_cb_completion_item(func, version))
    .collect()
}

pub fn build_fold_valid_cb_completion_items(version: ClarityVersion) -> Vec<CompletionItem> {
    [
        NativeFunctions::Add,
        NativeFunctions::Subtract,
        NativeFunctions::Multiply,
        NativeFunctions::Divide,
        NativeFunctions::CmpGeq,
        NativeFunctions::CmpLeq,
        NativeFunctions::CmpLess,
        NativeFunctions::CmpGreater,
        NativeFunctions::ToInt,
        NativeFunctions::ToUInt,
        NativeFunctions::Modulo,
        NativeFunctions::Power,
        NativeFunctions::Sqrti,
        NativeFunctions::Log2,
        NativeFunctions::BitwiseXor,
        NativeFunctions::And,
        NativeFunctions::Or,
        NativeFunctions::Not,
        NativeFunctions::IsStandard,
        NativeFunctions::BitwiseAnd,
        NativeFunctions::BitwiseOr,
        NativeFunctions::BitwiseNot,
        NativeFunctions::BitwiseLShift,
        NativeFunctions::BitwiseRShift,
        NativeFunctions::BitwiseXor2,
    ]
    .iter()
    .filter_map(|func| build_iterator_cb_completion_item(func, version))
    .collect()
}

fn build_iterator_cb_completion_item(
    func: &NativeFunctions,
    version: ClarityVersion,
) -> Option<CompletionItem> {
    let api = make_api_reference(func);
    if api.min_version > version {
        return None;
    }

    let insert_text = Some(api.snippet.split_whitespace().next().unwrap().to_string());

    Some(CompletionItem {
        label: api.name.clone(),
        kind: Some(CompletionItemKind::FUNCTION),
        detail: Some(api.name.clone()),
        documentation: Some(Documentation::MarkupContent(MarkupContent {
            kind: MarkupKind::Markdown,
            value: api.description,
        })),
        insert_text,
        insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
        ..Default::default()
    })
}

fn get_iterator_cb_completion_item(version: &ClarityVersion, func: &str) -> Vec<CompletionItem> {
    if func.to_string().eq(&NativeFunctions::Map.to_string()) {
        return match version {
            ClarityVersion::Clarity1 => VALID_MAP_FUNCTIONS_CLARITY_1.to_vec(),
            ClarityVersion::Clarity2 => VALID_MAP_FUNCTIONS_CLARITY_2.to_vec(),
            ClarityVersion::Clarity3 => VALID_MAP_FUNCTIONS_CLARITY_3.to_vec(),
        };
    }
    if func.to_string().eq(&NativeFunctions::Filter.to_string()) {
        return match version {
            ClarityVersion::Clarity1 => VALID_FILTER_FUNCTIONS_CLARITY_1.to_vec(),
            ClarityVersion::Clarity2 => VALID_FILTER_FUNCTIONS_CLARITY_2.to_vec(),
            ClarityVersion::Clarity3 => VALID_FILTER_FUNCTIONS_CLARITY_3.to_vec(),
        };
    }
    match version {
        ClarityVersion::Clarity1 => VALID_FOLD_FUNCTIONS_CLARITY_1.to_vec(),
        ClarityVersion::Clarity2 => VALID_FOLD_FUNCTIONS_CLARITY_2.to_vec(),
        ClarityVersion::Clarity3 => VALID_FOLD_FUNCTIONS_CLARITY_3.to_vec(),
    }
}

#[cfg(test)]
mod get_contract_global_data_tests {
    use clarity_repl::clarity::ast::{build_ast_with_rules, ContractAST};
    use clarity_repl::clarity::vm::types::QualifiedContractIdentifier;
    use clarity_repl::clarity::{ClarityVersion, StacksEpochId};
    use lsp_types::Position;

    use super::ContractDefinedData;

    fn get_ast(source: &str) -> ContractAST {
        build_ast_with_rules(
            &QualifiedContractIdentifier::transient(),
            source,
            &mut (),
            ClarityVersion::Clarity2,
            StacksEpochId::Epoch21,
            clarity_repl::clarity::ast::ASTRules::Typical,
        )
        .unwrap()
    }

    #[test]
    fn get_data_vars() {
        let contract_ast = get_ast(
            "(define-data-var counter uint u1) (define-data-var is-active bool true)",
        );
        let data = ContractDefinedData::new(
            &contract_ast.expressions, 
            Position::default(), 
            StacksEpochId::Epoch21, 
            ClarityVersion::Clarity2
        );
        assert_eq!(data.vars, ["counter", "is-active"]);
    }

    #[test]
    fn get_map() {
        let contract_ast = get_ast("(define-map names principal { name: (buff 48) })");
        let data = ContractDefinedData::new(
            &contract_ast.expressions, 
            Position::default(), 
            StacksEpochId::Epoch21, 
            ClarityVersion::Clarity2
        );
        assert_eq!(data.maps, ["names"]);
    }

    #[test]
    fn get_fts() {
        let contract_ast = get_ast("(define-fungible-token clarity-coin)");
        let data = ContractDefinedData::new(
            &contract_ast.expressions, 
            Position::default(), 
            StacksEpochId::Epoch21, 
            ClarityVersion::Clarity2
        );
        assert_eq!(data.fts, ["clarity-coin"]);
    }

    #[test]
    fn get_nfts() {
        let contract_ast = get_ast("(define-non-fungible-token bitcoin-nft uint)");
        let data = ContractDefinedData::new(
            &contract_ast.expressions, 
            Position::default(), 
            StacksEpochId::Epoch21, 
            ClarityVersion::Clarity2
        );
        assert_eq!(data.nfts, ["bitcoin-nft"]);
    }
}

#[cfg(test)]
mod get_contract_local_data_tests {
    use clarity_repl::clarity::ast::{build_ast_with_rules, ContractAST};
    use clarity_repl::clarity::StacksEpochId;
    use clarity_repl::clarity::{vm::types::QualifiedContractIdentifier, ClarityVersion};
    use lsp_types::Position;

    use super::ContractDefinedData;

    fn get_ast(source: &str) -> ContractAST {
        build_ast_with_rules(
            &QualifiedContractIdentifier::transient(),
            source,
            &mut (),
            ClarityVersion::Clarity2,
            StacksEpochId::Epoch21,
            clarity_repl::clarity::ast::ASTRules::Typical,
        )
        .unwrap()
    }

    #[test]
    fn get_function_binding() {
        let contract_ast = get_ast(
            "(define-private (print-arg (arg int)) )",
        );
        let data = ContractDefinedData::new(
            &contract_ast.expressions, 
            Position {
                line: 1,
                character: 38,
            },
            StacksEpochId::Epoch21, 
            ClarityVersion::Clarity2
        );
        assert_eq!(data.locals, vec![("arg".to_string(), "int".to_string())]);
        let contract_ast = get_ast(
            "(define-private (print-arg (arg int)) )",
        );
        let data = ContractDefinedData::new(
            &contract_ast.expressions, 
            Position {
                line: 1,
                character: 40,
            },
            StacksEpochId::Epoch21, 
            ClarityVersion::Clarity2
        );
        assert_eq!(data.locals, vec![]);
    }

    #[test]
    fn get_let_binding() {
        let contract_ast = get_ast(
            "(let ((n u0)) )",
        );
        let data = ContractDefinedData::new(
            &contract_ast.expressions, 
            Position {
                line: 1,
                character: 15,
            },
            StacksEpochId::Epoch21, 
            ClarityVersion::Clarity2
        );
        assert_eq!(data.locals, vec![("n".to_string(), "u0".to_string())]);
    }
}

#[cfg(test)]
mod populate_snippet_with_options_tests {
    use clarity_repl::clarity::ast::build_ast_with_rules;
    use clarity_repl::clarity::vm::types::QualifiedContractIdentifier;
    use clarity_repl::clarity::{ClarityVersion, StacksEpochId};
    use lsp_types::Position;

    use super::ContractDefinedData;

/*     fn get_defined_data(source: &str) -> ContractDefinedData {
        let contract_ast = build_ast_with_rules(
            &QualifiedContractIdentifier::transient(),
            source,
            &mut (),
            ClarityVersion::Clarity2,
            StacksEpochId::Epoch21,
            clarity_repl::clarity::ast::ASTRules::Typical,
        )
        .unwrap();
        ContractDefinedData::new(&contract_ast.expressions, &Position::default(), StacksEpochId::Epoch21, ClarityVersion::Clarity2)
    } */

    #[test]
    fn get_data_vars_snippet() {
        let source =
            "(define-data-var counter uint u1) (define-data-var is-active bool true)";
        let contract_ast = build_ast_with_rules(
            &QualifiedContractIdentifier::transient(),
            source,
            &mut (),
            ClarityVersion::Clarity2,
            StacksEpochId::Epoch21,
            clarity_repl::clarity::ast::ASTRules::Typical,
        )
        .unwrap();
        let data = ContractDefinedData::new(
            &contract_ast.expressions, 
            Position::default(), 
            StacksEpochId::Epoch21, 
            ClarityVersion::Clarity2
        );
        let snippet = data.populate_snippet_with_options(
            &ClarityVersion::Clarity2,
            &"var-get".to_string(),
            "var-get ${1:var}",
        );
        assert_eq!(snippet, Some("var-get ${1|counter,is-active|}".to_string()));
    }

    #[test]
    fn get_map_snippet() {
        let source = "(define-map names principal { name: (buff 48) })";
        let contract_ast = build_ast_with_rules(
            &QualifiedContractIdentifier::transient(),
            source,
            &mut (),
            ClarityVersion::Clarity2,
            StacksEpochId::Epoch21,
            clarity_repl::clarity::ast::ASTRules::Typical,
        )
        .unwrap();
        let data = ContractDefinedData::new(
            &contract_ast.expressions, 
            Position::default(), 
            StacksEpochId::Epoch21, 
            ClarityVersion::Clarity2
        );
        let snippet = data.populate_snippet_with_options(
            &ClarityVersion::Clarity2,
            &"map-get?".to_string(),
            "map-get? ${1:map-name} ${2:key-tuple}",
        );
        assert_eq!(
            snippet,
            Some("map-get? ${1|names|} ${2:key-tuple}".to_string())
        );
    }

    #[test]
    fn get_fts_snippet() {
        let source = "(define-fungible-token btc u21)";
        let contract_ast = build_ast_with_rules(
            &QualifiedContractIdentifier::transient(),
            source,
            &mut (),
            ClarityVersion::Clarity2,
            StacksEpochId::Epoch21,
            clarity_repl::clarity::ast::ASTRules::Typical,
        )
        .unwrap();
        let data = ContractDefinedData::new(
            &contract_ast.expressions, 
            Position::default(), 
            StacksEpochId::Epoch21, 
            ClarityVersion::Clarity2
        );
        let snippet = data.populate_snippet_with_options(
            &ClarityVersion::Clarity2,
            &"ft-mint?".to_string(),
            "ft-mint? ${1:token-name} ${2:amount} ${3:recipient}",
        );
        assert_eq!(
            snippet,
            Some("ft-mint? ${1|btc|} ${2:amount} ${3:recipient}".to_string())
        );
    }

    #[test]
    fn get_nfts_snippet() {
        let source = "(define-non-fungible-token bitcoin-nft uint)";
        let contract_ast = build_ast_with_rules(
            &QualifiedContractIdentifier::transient(),
            source,
            &mut (),
            ClarityVersion::Clarity2,
            StacksEpochId::Epoch21,
            clarity_repl::clarity::ast::ASTRules::Typical,
        )
        .unwrap();
        let data = ContractDefinedData::new(
            &contract_ast.expressions, 
            Position::default(), 
            StacksEpochId::Epoch21, 
            ClarityVersion::Clarity2
        );
        let snippet = data.populate_snippet_with_options(
            &ClarityVersion::Clarity2,
            &"nft-mint?".to_string(),
            "nft-mint? ${1:asset-name} ${2:asset-identifier} ${3:recipient}",
        );
        assert_eq!(
            snippet,
            Some("nft-mint? ${1|bitcoin-nft|} ${2:asset-identifier} ${3:recipient}".to_string())
        );
    }
}

mod trait_tests {
    use std::{cmp::Ordering, collections::{BTreeMap, HashMap}, vec};

    use clarinet_files::FileLocation;
    use clarity_repl::{
        analysis::ast_visitor::{traverse, ASTVisitor}, 
        clarity::{analysis::ContractAnalysis, 
            ast::{build_ast_with_diagnostics, build_ast_with_rules, parser, ASTRules, ContractAST}, 
            costs::LimitedCostTracker, 
            vm::types::{PrincipalData, QualifiedContractIdentifier, StandardPrincipalData, TypeSignature}}, 
            repl::{DEFAULT_CLARITY_VERSION, DEFAULT_EPOCH}
        };
    use lsp_types::{CompletionContext, CompletionItem, CompletionTriggerKind, Position, Range};

    use crate::{
        common::requests::{completion::ContractDefinedData, 
            helpers::is_position_within_span}, 
            state::{ActiveContractData, ContractState, ProtocolState}
        };

    use super::{build_completion_item_list, build_trait_completion_data, DefineFunctionType};

    #[derive(Debug, Eq, PartialEq, Copy, Clone, Default)]
    struct RangeExoSkeleton {
        inner: Range,
    }

    impl From<Range> for RangeExoSkeleton {
        fn from(value: Range) -> Self {
            Self { inner: value }
        }
    }

    impl Ord for RangeExoSkeleton {
        fn cmp(&self, other: &Self) -> Ordering {
            // for now, this approximation is enough
            self.inner.start.line.cmp(&other.inner.start.line)
            .then_with(|| self.inner.end.line.cmp(&other.inner.end.line))
            .then_with(|| self.inner.end.character.cmp(&other.inner.end.character))
        }
    }

    impl PartialOrd for RangeExoSkeleton {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    fn replace_range_in_place(text: &mut String, mut start_line: usize, mut start_char: usize, mut end_line: usize, mut end_char: usize, replacement: &str) {
        let lines: Vec<&str> = text.lines().collect();
    
        if start_line >= lines.len() {
            start_line = lines.len().saturating_sub(1);
        }

        if end_line >= lines.len() {
            end_line = lines.len().saturating_sub(1);
        }

        let start_line_chars: Vec<(usize, char)> = lines.get(start_line).map(|x| x.char_indices().collect()).unwrap_or_default();
        if start_char > start_line_chars.len() {
            start_char = start_line_chars.len()
        }

        let end_line_chars: Vec<(usize, char)> = lines.get(end_line).map(|x| x.char_indices().collect()).unwrap_or_default();
        if end_char > end_line_chars.len() {
            end_char = end_line_chars.len()
        }

        let mut start_byte_offset = 0;
        let mut end_byte_offset = 0;
        let mut current_offset = 0;
    
        for (i, line) in lines.iter().enumerate() {
            if i < start_line {
                current_offset += line.len()+1;
            } else if i == start_line {
                start_byte_offset = current_offset + start_char;
                break;
            }
        }

        current_offset = 0;

        for (i, line) in lines.iter().enumerate() {
            if i < end_line {
                current_offset += line.len()+1;
            } else if i == end_line {
                end_byte_offset = current_offset + end_char;
                break;
            }
        }

        text.replace_range(start_byte_offset..end_byte_offset, replacement);
    }

    // what is being done here is trying to emulate how a CompletionItem is applied to a raw text file.
    // According to language server protocol documentaion if a CompletionItem contains n TextEdits
    // they are applied from bottom to the top of text document. And it makes sense intuitively
    // to do so beacuse the changes will mimic as if they are applied to the original document.
    // This function simulates this process according to the specific use cases and possibilities.
    fn apply_completion_item(source: &mut String, word: &str,  pos: &Position, item: &CompletionItem) {
        // A sorted map. Since overlapping edits are prohibited, a simple comparison will suffice.
        let mut map: Vec<(RangeExoSkeleton, String)> = Vec::new();

        if let Some(text) = &item.insert_text {
            map.push((Range::new(*pos, *pos).into(), text.replacen(word, "", 1)));
        } else {
            map.push((Range::new(*pos, *pos).into(), item.label.replacen(word, "", 1)))
        }

        if let Some(edits) = &item.additional_text_edits {
            for edit in edits {
                map.push((edit.range.into(), edit.new_text.clone()));
            }
        }

        map.sort_by(|a, b| a.0.cmp(&b.0));

        while let Some((RangeExoSkeleton { inner }, text)) = map.pop() {
            replace_range_in_place(
                source, 
                inner.start.line as usize, 
                inner.start.character as usize, 
                inner.end.line as usize, 
                inner.end.character as usize,
                &text[..]
            );
        }
    }

    #[derive(Debug)]
    struct AnalysisExoSkeleton<'a> {
        inner: &'a mut ContractAnalysis,
    }

    impl<'a> ASTVisitor<'a> for AnalysisExoSkeleton<'a> {
        fn visit_define_trait(
                &mut self,
                expr: &'a clarity_repl::clarity::SymbolicExpression,
                name: &'a clarity_repl::clarity::ClarityName,
                functions: &'a [clarity_repl::clarity::SymbolicExpression],
            ) -> bool {
            let function_types = TypeSignature::parse_trait_type_repr(
                functions, 
                &mut (), 
                DEFAULT_EPOCH, 
                DEFAULT_CLARITY_VERSION
            ).unwrap();
            self.inner.add_defined_trait(name.clone(), function_types);
            true
        }
    }

    fn build_protocol_state() -> ProtocolState {
        let contract1_principal = "ST1HTBVD3JG9C05J7HBJTHGR0GGW7KXW28M5JS8QE";
        let contract1_name = "timelocked-wallet";
        let contract1_source = r#"
(define-trait locked-wallet-trait
    (
        (public lock (principal uint uint) (response bool uint))
        (read-only bestow (principal) (response bool uint))
        (claim () (response bool uint))
    )
)
"#;

    let contract2_principal = "ST1J4G6RR643BCG8G8SR6M2D9Z9KXT2NJDRK3FBTK";
    let contract2_name = "abc-contract";
    let contract2_source = r#"
(define-trait abc
    (
        (public a (principal uint uint) (response bool uint))
        (read-only b (principal) (response bool uint))
        (c () (response bool uint))
    )
)"#;

    let mut state = ProtocolState::new();
    add_contracts_to_state(
        &mut state,
        vec![
            (contract1_principal, contract1_name, contract1_source),
            (contract2_principal, contract2_name, contract2_source),
        ]
    );

    state
    }

    fn get_ast(contract_identifier: &QualifiedContractIdentifier, source_code: &str) -> ContractAST {
        build_ast_with_diagnostics(contract_identifier, source_code, &mut (), DEFAULT_CLARITY_VERSION, DEFAULT_EPOCH).0
    }

    fn add_contracts_to_state(
        state: &mut ProtocolState,
        contracts: Vec<(&str, &str, &str)>, 
    ) {
        let mut locations = HashMap::new();
        let mut asts = BTreeMap::new();
        let mut analyses = HashMap::new();
        let mut clarity_versions = HashMap::new();
        for (contract_principal, contract_name, contract_source) in contracts {
            let contract_identifier = QualifiedContractIdentifier::new(
                PrincipalData::parse_standard_principal(contract_principal).unwrap(), 
                contract_name.into()
            );
            let contract_ast = get_ast(&contract_identifier, contract_source);
            let mut contract_analysis = ContractAnalysis::new(
                contract_identifier.clone(), 
                contract_ast.expressions.clone(), 
                LimitedCostTracker::Free, 
                DEFAULT_EPOCH, 
                DEFAULT_CLARITY_VERSION
            );

            traverse(&mut AnalysisExoSkeleton{ inner: &mut contract_analysis }, &contract_ast.expressions);
            analyses.insert(contract_identifier.clone(), Some(contract_analysis));
            asts.insert(contract_identifier.clone(), contract_ast);
            clarity_versions.insert(contract_identifier.clone(), DEFAULT_CLARITY_VERSION);
            locations.insert(contract_identifier, FileLocation::from_path_string(&format!("/{}.clar", contract_name)).unwrap());
        }

        state.consolidate(
            &mut locations, 
            &mut asts, 
            &mut BTreeMap::new(), 
            &mut HashMap::new(),
            &mut HashMap::new(), 
            &mut analyses, 
            &mut clarity_versions
        );
    }

    fn get_active_contract_data(source_code: &str) -> ActiveContractData {
        ActiveContractData::new(DEFAULT_CLARITY_VERSION, DEFAULT_EPOCH, None, source_code)
    }

    fn get_completion_list(
        contract: &str, 
        issuer: Option<StandardPrincipalData>, 
        pos: &Position, 
        context: &Option<CompletionContext>
    ) -> Option<Vec<CompletionItem>> {
        let state = build_protocol_state();
        let active_contract_data = get_active_contract_data(contract);

        build_trait_completion_data(
            &issuer.unwrap_or(StandardPrincipalData::transient()), 
            &FileLocation::from_path_string("/test.clar").unwrap(),
            &ContractDefinedData::new(
                &active_contract_data.expressions.clone().unwrap_or_default()[..], 
                *pos,
                DEFAULT_EPOCH, 
                DEFAULT_CLARITY_VERSION
            ), 
            &state, 
            &active_contract_data, 
            pos, 
            context
        )
    }

    #[test]
    fn test_principal_autocomplete() {
        let mut contract = "'".to_string();
        let pos = Position::new(0, 1);
        let context = Some(CompletionContext{ 
            trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER, 
            trigger_character: Some("'".to_string())
        });

        let list = get_completion_list(&contract, None, &pos, &context);

        let expected_result = ["'ST1HTBVD3JG9C05J7HBJTHGR0GGW7KXW28M5JS8QE", "'ST1J4G6RR643BCG8G8SR6M2D9Z9KXT2NJDRK3FBTK"];

        assert!(list.is_some_and(|list| 
            list.len() == 2
            && {
                apply_completion_item(&mut contract, "", &pos, &list[0]);
                expected_result.contains(&contract.as_str())
            }
        ));
    }

    #[test]
    fn test_incomplete_principal_completion() {
        let mut contract = "'ST1J".to_string();
        let pos = Position::new(0, 5);
        let context = None;

        let list = get_completion_list(&contract, None, &pos, &context);

        let expected_result = ["'ST1HTBVD3JG9C05J7HBJTHGR0GGW7KXW28M5JS8QE", "'ST1J4G6RR643BCG8G8SR6M2D9Z9KXT2NJDRK3FBTK"];

        assert!(list.is_some_and(|list| 
            list.len() == 2
            && {
                for item in list {
                    if item.label.contains("ST1J") {
                        apply_completion_item(&mut contract, "ST1J", &pos, &item);
                    }
                }
                expected_result.contains(&contract.as_str())
            }
        ));
    }

    #[test]
    fn test_sugared_contract_name_completion() {
        let mut contract = ".".to_string();
        let pos = Position::new(0, 1);
        let context = Some(CompletionContext{ 
            trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER, 
            trigger_character: Some(".".to_string())
        });
        let issuer = PrincipalData::parse_standard_principal("ST1HTBVD3JG9C05J7HBJTHGR0GGW7KXW28M5JS8QE").unwrap();

        let list = get_completion_list(&contract, Some(issuer), &pos, &context);

        let expected_result = ".timelocked-wallet";

        assert!(list.is_some_and(|list| 
            list.len() == 1
            && {
                apply_completion_item(&mut contract, "", &pos, &list[0]);
                expected_result == contract
            }
        ));
    }

    #[test]
    fn test_sugared_partial_contract_name_completion() {
        let mut contract = ".a".to_string();
        let pos = Position::new(0, 2);
        let context = None;
        let issuer = PrincipalData::parse_standard_principal("ST1J4G6RR643BCG8G8SR6M2D9Z9KXT2NJDRK3FBTK").unwrap();

        let list = get_completion_list(&contract, Some(issuer), &pos, &context);

        let expected_result = ".abc-contract";

        assert!(list.is_some_and(|list| 
            list.len() == 1
            && {
                apply_completion_item(&mut contract, "a", &pos, &list[0]);
                expected_result == contract
            }
        ));
    }

    #[test]
    fn test_qualified_contract_name_completion() {
        let mut contract = "'ST1HTBVD3JG9C05J7HBJTHGR0GGW7KXW28M5JS8QE.".to_string();
        let pos = Position::new(0, 43);
        let context =  Some(CompletionContext{ 
            trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER, 
            trigger_character: Some(".".to_string())
        });

        let list = get_completion_list(&contract, None, &pos, &context);

        let expected_result = "'ST1HTBVD3JG9C05J7HBJTHGR0GGW7KXW28M5JS8QE.timelocked-wallet";

        assert!(list.is_some_and(|list| 
            list.len() == 1
            && {
                apply_completion_item(&mut contract, "", &pos, &list[0]);
                expected_result == contract
            }
        ));
    }

    #[test]
    fn test_qualified_partial_contract_name_completion() {
        let mut contract = "'ST1HTBVD3JG9C05J7HBJTHGR0GGW7KXW28M5JS8QE.time".to_string();
        let pos = Position::new(0, 47);
        
        let list = get_completion_list(&contract, None, &pos, &None);

        let expected_result = "'ST1HTBVD3JG9C05J7HBJTHGR0GGW7KXW28M5JS8QE.timelocked-wallet";
        
        assert!(list.is_some_and(|list| 
            list.len() == 1
            && {
                apply_completion_item(&mut contract, "time", &pos, &list[0]);
                expected_result == contract
            }
        ));
    }

    #[test]
    fn test_sugared_trait_name() {
        let mut contract = ".timelocked-wallet.".to_string();
        let pos = Position::new(0, 19);
        let context =  Some(CompletionContext{ 
            trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER, 
            trigger_character: Some(".".to_string())
        });
        let issuer = PrincipalData::parse_standard_principal("ST1HTBVD3JG9C05J7HBJTHGR0GGW7KXW28M5JS8QE").unwrap();

        let list = get_completion_list(&contract, Some(issuer), &pos, &context);

        let expected_result = ".timelocked-wallet.locked-wallet-trait";

        assert!(list.is_some_and(|list| 
            list.len() == 1
            && {
                apply_completion_item(&mut contract, "", &pos, &list[0]);
                expected_result == contract
            }
        ));
    }

    #[test]
    fn test_qualified_trait_name() {
        let mut contract = "'ST1J4G6RR643BCG8G8SR6M2D9Z9KXT2NJDRK3FBTK.abc-contract.".to_string();
        let pos = Position::new(0, 56);
        let context =  Some(CompletionContext{ 
            trigger_kind: CompletionTriggerKind::TRIGGER_CHARACTER, 
            trigger_character: Some(".".to_string())
        });

        let list =  get_completion_list(&contract, None, &pos, &context);

        let expected_result = "'ST1J4G6RR643BCG8G8SR6M2D9Z9KXT2NJDRK3FBTK.abc-contract.abc";

        assert!(list.is_some_and(|list| 
            list.len() == 1
            && {
                apply_completion_item(&mut contract, "", &pos, &list[0]);
                expected_result == contract
            }
        ));
    }

    #[test]
    fn test_sugared_partial_trait_name() {
        let mut contract = "(impl-trait .abc-contract.a)".to_string();
        let pos = Position::new(0, 27);
        let issuer = PrincipalData::parse_standard_principal("ST1J4G6RR643BCG8G8SR6M2D9Z9KXT2NJDRK3FBTK").unwrap();

        let list =  get_completion_list(&contract, Some(issuer), &pos, &None);
        
        let expected_result = "(impl-trait .abc-contract.abc)

(define-public (a (param1-name principal) (param2-name uint) (param3-name uint)) body)

(define-read-only (b (param1-name principal)) body)

(access-modifier-kind (c) body)

"; 

        assert!(list.is_some_and(|list| 
            list.len() == 1
            && {
                apply_completion_item(&mut contract, "a", &pos, &list[0]);
                expected_result == contract
            }
        ));
    }

    fn build_completion_list(contract: &str, pos: &Position, issuer: &StandardPrincipalData) -> Vec<CompletionItem> {
        let state = build_protocol_state();
        let active_contract_data = get_active_contract_data(contract);

        build_completion_item_list(
            &DEFAULT_CLARITY_VERSION, 
            &active_contract_data.expressions.clone().unwrap(), 
            &FileLocation::from_path_string("/test.clar").unwrap(), 
            &Position::new(pos.line+1, pos.character+1), 
            Some(issuer.clone()), 
            &ContractDefinedData::new(
                &active_contract_data.expressions.clone().unwrap_or_default()[..], 
                *pos,
                DEFAULT_EPOCH, 
                DEFAULT_CLARITY_VERSION
            ), 
            &state, 
            vec![], 
            false, 
            false
        )
    }

    #[test]
    fn test_impl_trait_suggestions() {
        let mut contract: String = "(impl-trait )
some-random-contract".into();
        let pos = Position::new(0, 12);
        let issuer = PrincipalData::parse_standard_principal("ST1HTBVD3JG9C05J7HBJTHGR0GGW7KXW28M5JS8QE").unwrap();

        let list = build_completion_list(&contract, &pos, &issuer);

        for item in list {
            if item.label.contains("timelocked-wallet") {
                apply_completion_item(&mut contract, "", &pos, &item)
            }
        }

        let expected_result: String = "(impl-trait .timelocked-wallet.locked-wallet-trait)

(define-read-only (bestow (param1-name principal)) body)

(access-modifier-kind (claim) body)

(define-public (lock (param1-name principal) (param2-name uint) (param3-name uint)) body)


some-random-contract".into();

        assert_eq!(contract, expected_result)
    }

    #[test]
    fn test_use_trait_suggestions() {
        let mut contract = "(use-trait )".to_string();
        let pos = Position::new(0, 11);
        let issuer = StandardPrincipalData::transient();

        let list = build_completion_list(&contract, &pos, &issuer);

        for item in list {
            if item.label.contains("abc") {
                apply_completion_item(&mut contract, "", &pos, &item)
            }
        }

        let expected_result = "(use-trait abc 'ST1J4G6RR643BCG8G8SR6M2D9Z9KXT2NJDRK3FBTK.abc-contract.abc)".to_owned();

        assert_eq!(contract, expected_result);
    }

    #[test]
    fn test_use_trait_suggestions2() {
        let mut contract = "(use-trait abc )".to_string();
        let pos = Position::new(0, 15);
        let issuer = PrincipalData::parse_standard_principal("ST1J4G6RR643BCG8G8SR6M2D9Z9KXT2NJDRK3FBTK").unwrap();

        let list = build_completion_list(&contract, &pos, &issuer);

        for item in list {
            if item.label.contains("abc") {
                apply_completion_item(&mut contract, "", &pos, &item)
            }
        }

        let expected_result = "(use-trait abc .abc-contract.abc)".to_owned();

        assert_eq!(contract, expected_result);
    }

    #[test]
    fn test_trait_alias_completion() {
        let mut contract = "(define-public (set (n <a)) body)".to_string();
        let pos = Position::new(0, 25);
        let issuer = PrincipalData::parse_standard_principal("ST1J4G6RR643BCG8G8SR6M2D9Z9KXT2NJDRK3FBTK").unwrap();

        let list = get_completion_list(&contract, Some(issuer), &pos, &None).unwrap();

        for item in list {
            if item.label.contains("abc") {
                apply_completion_item(&mut contract, "a", &pos, &item)
            }
        }

        let expected_result = "(use-trait abc .abc-contract.abc)

(define-public (set (n <abc>)) body)";

    assert_eq!(contract, expected_result);
    }

    #[test]
    fn test_trait_alias_completion2() {
        let mut contract = "(use-trait xyz .xyz.xyz)

(define-public (set (n <)) body)".to_string();
        let pos = Position::new(2, 24);
        let issuer = PrincipalData::parse_standard_principal("ST1J4G6RR643BCG8G8SR6M2D9Z9KXT2NJDRK3FBTK").unwrap();

        let list = get_completion_list(&contract, Some(issuer), &pos, &None).unwrap();

        for item in list {
            if item.label.contains("abc") {
                apply_completion_item(&mut contract, "", &pos, &item)
            }
        }

        let expected_result = 
"(use-trait abc .abc-contract.abc)
(use-trait xyz .xyz.xyz)

(define-public (set (n <abc>)) body)";

    assert_eq!(contract, expected_result);
    }
}
