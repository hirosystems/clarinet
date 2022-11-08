use clarinet_files::FileLocation;
use clarity_repl::clarity::ast::build_ast;
use clarity_repl::clarity::diagnostic::Diagnostic;
use clarity_repl::clarity::docs::{
    make_api_reference, make_define_reference, make_keyword_reference,
};
use clarity_repl::clarity::functions::define::DefineFunctions;
use clarity_repl::clarity::functions::NativeFunctions;
use clarity_repl::clarity::stacks_common::types::StacksEpochId;
use clarity_repl::clarity::variables::NativeVariables;
use clarity_repl::clarity::vm::types::QualifiedContractIdentifier;
use clarity_repl::clarity::{ClarityVersion, SymbolicExpression};
use lsp_types::Hover;
use std::collections::HashMap;

fn code(code: &str) -> String {
    vec!["```clarity", code.trim(), "```"].join("\n")
}

lazy_static! {
    pub static ref API_REF: HashMap<String, String> = {
        let mut api_references = HashMap::new();
        for define_function in DefineFunctions::ALL {
            let reference = make_define_reference(define_function);
            api_references.insert(
                define_function.to_string(),
                Vec::from([
                    &code(&reference.signature),
                    "---",
                    "**Description**",
                    &reference.description,
                    "---",
                    "**Example**",
                    &code(&reference.example),
                ])
                .join("\n"),
            );
        }

        for native_function in NativeFunctions::ALL {
            let reference = make_api_reference(native_function);
            api_references.insert(
                native_function.to_string(),
                Vec::from([
                    &code(&reference.signature),
                    "---",
                    "**Description**",
                    &reference.description,
                    "---",
                    "**Example**",
                    &code(&reference.example),
                ])
                .join("\n"),
            );
        }

        for native_variable in NativeVariables::ALL {
            let reference = make_keyword_reference(native_variable).unwrap();
            api_references.insert(
                native_variable.to_string(),
                vec![
                    "**Description**",
                    &reference.description,
                    "---",
                    "**Example**",
                    &code(&reference.example),
                ]
                .join("\n"),
            );
        }

        api_references
    };
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActiveContractState {
    pub manifest_location: FileLocation,
    pub clarity_version: ClarityVersion,
    pub expressions: Option<Vec<SymbolicExpression>>,
    pub diagnostic: Option<Diagnostic>,
}

impl ActiveContractState {
    pub fn new(
        manifest_location: FileLocation,
        clarity_version: ClarityVersion,
        source: &str,
    ) -> Self {
        match build_ast(
            &QualifiedContractIdentifier::transient(),
            source,
            &mut (),
            clarity_version,
            StacksEpochId::Epoch21,
        ) {
            Ok(ast) => ActiveContractState {
                manifest_location,
                clarity_version,
                expressions: Some(ast.expressions),
                diagnostic: None,
            },
            Err(err) => ActiveContractState {
                manifest_location,
                clarity_version,
                expressions: None,
                diagnostic: Some(err.diagnostic),
            },
        }
    }

    pub fn update(&mut self, source: &str) {
        match build_ast(
            &QualifiedContractIdentifier::transient(),
            source,
            &mut (),
            self.clarity_version,
            StacksEpochId::Epoch21,
        ) {
            Ok(ast) => {
                self.expressions = Some(ast.expressions);
                self.diagnostic = None;
            }
            Err(err) => {
                self.expressions = None;
                self.diagnostic = Some(err.diagnostic);
            }
        };
    }
}

#[derive(Clone, Default, Debug)]
pub struct ActiveEditorState {
    pub contracts: HashMap<FileLocation, ActiveContractState>,
    pub api_reference: HashMap<String, String>,
}

impl ActiveEditorState {
    pub fn new() -> Self {
        let api_reference = API_REF.clone();

        ActiveEditorState {
            contracts: HashMap::new(),
            api_reference,
        }
    }

    pub fn insert_contract(
        &mut self,
        contract_location: FileLocation,
        manifest_location: FileLocation,
        source: &str,
    ) {
        let clarity_version = ClarityVersion::Clarity1;
        let contract = ActiveContractState::new(manifest_location, clarity_version, source);
        self.contracts.insert(contract_location, contract);
    }

    pub fn update_contract(
        &mut self,
        contract_location: &FileLocation,
        source: &str,
    ) -> Result<(), String> {
        let contract_state = self
            .contracts
            .get_mut(contract_location)
            .ok_or("contract not in state")?;

        contract_state.update(source);
        Ok(())
    }

    pub fn get_hover_data(
        &self,
        contract_location: &FileLocation,
        position: &lsp_types::Position,
    ) -> Option<Hover> {
        let contract_state = match self.contracts.get(&contract_location) {
            Some(contract_state) => contract_state,
            None => return None,
        };

        let expressions = match &contract_state.expressions {
            Some(expressions) => expressions,
            None => return None,
        };

        let expression_name = match find_at(position.line + 1, position.character + 1, expressions)
        {
            Some(expression_name) => expression_name,
            None => return None,
        };

        match self.api_reference.get(&expression_name) {
            Some(api_reference) => {
                return Some(Hover {
                    contents: lsp_types::HoverContents::Markup(lsp_types::MarkupContent {
                        kind: lsp_types::MarkupKind::Markdown,
                        value: api_reference.to_string(),
                    }),
                    range: None,
                });
            }
            None => return None,
        };
    }
}

pub fn find_at(line: u32, column: u32, expressions: &Vec<SymbolicExpression>) -> Option<String> {
    for expr in expressions {
        let SymbolicExpression { span, .. } = expr;

        if span.start_line <= line && span.end_line >= line {
            if span.end_line > span.start_line {
                if let Some(expressions) = expr.match_list() {
                    return find_at(line, column, &expressions.to_vec());
                }
                return None;
            }
            if span.start_column <= column && span.end_column >= column {
                if let Some(function_name) = expr.match_atom() {
                    return Some(function_name.to_string());
                } else if let Some(expressions) = expr.match_list() {
                    return find_at(line, column, &expressions.to_vec());
                }
                return None;
            }
        }
    }
    None
}
