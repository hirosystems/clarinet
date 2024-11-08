use std::fmt;

use clarity_repl::{
    analysis::ast_visitor::{traverse, ASTVisitor, TypedVar},
    clarity::{
        analysis::{type_checker::contexts::TypeMap, ContractAnalysis},
        docs::{get_input_type_string, get_output_type_string},
        representations::TraitDefinition,
        vm::types::{FunctionType, QualifiedContractIdentifier, TraitIdentifier, TypeSignature},
        ClarityName, SymbolicExpression,
    },
};
use hashbrown::HashMap;
use lsp_types::Position;

use crate::common::requests::helpers::get_expression_at_position;

use super::{api_ref::API_REF, helpers::get_expression_name_at_position};

struct ClarinetTypeSignature(TypeSignature);

fn format_tuple(sig: &TypeSignature, indent_level: usize) -> String {
    let braces_indent = "  ".repeat(indent_level - 1);
    let indent = "  ".repeat(indent_level);
    let key_values_types = match sig {
        TypeSignature::TupleType(sig) => {
            let key_val: Vec<String> = sig
                .get_type_map()
                .iter()
                .map(|(k, v)| format!("{}{}: {}", indent, k, format_tuple(v, indent_level + 1)))
                .collect();
            format!("{{\n{}\n{}}}", key_val.join(",\n"), braces_indent)
        }
        _ => format!("{}", ClarinetTypeSignature(sig.clone())),
    };
    key_values_types
}

impl fmt::Display for ClarinetTypeSignature {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.0 {
            TypeSignature::NoType => write!(f, "unknown"),
            TypeSignature::TupleType(_) => {
                let formatted = format_tuple(&self.0, 1);
                write!(f, "{}", formatted)
            }
            _ => write!(f, "{}", self.0),
        }
    }
}

fn get_function_type(
    function_name: &ClarityName,
    analysis: &ContractAnalysis,
) -> Option<FunctionType> {
    if let Some(t) = analysis.private_function_types.get(function_name) {
        return Some(t.clone());
    }
    if let Some(t) = analysis.read_only_function_types.get(function_name) {
        return Some(t.clone());
    }
    if let Some(t) = analysis.public_function_types.get(function_name) {
        return Some(t.clone());
    }
    None
}

fn get_variable_type(
    variable_name: &ClarityName,
    analysis: &ContractAnalysis,
) -> Option<(String, ClarinetTypeSignature)> {
    if let Some(t) = analysis.persisted_variable_types.get(variable_name) {
        return Some((
            "define-data-var".to_owned(),
            ClarinetTypeSignature(t.clone()),
        ));
    }
    if let Some(t) = analysis.variable_types.get(variable_name) {
        return Some((
            "define-constant".to_owned(),
            ClarinetTypeSignature(t.clone()),
        ));
    }
    None
}

fn get_map_type(
    map_name: &ClarityName,
    analysis: &ContractAnalysis,
) -> Option<(TypeSignature, TypeSignature)> {
    analysis.map_types.get(map_name).cloned()
}

fn get_type_documentation(expr_name: &ClarityName, analysis: &ContractAnalysis) -> Option<String> {
    if let Some((label, sig)) = get_variable_type(expr_name, analysis) {
        return Some(format!(
            "{}: `{}`\n```clarity\n{}\n```",
            label, expr_name, sig
        ));
    }

    if let Some(sig) = get_function_type(expr_name, analysis) {
        return Some(format!(
            "```clarity\n{} -> {}\n```",
            get_input_type_string(&sig),
            get_output_type_string(&sig)
        ));
    }

    if let Some((key_sig, val_sig)) = get_map_type(expr_name, analysis) {
        return Some(format!(
            "define-map {}:\n```clarity\n{}\n{}\n```",
            expr_name,
            ClarinetTypeSignature(key_sig),
            ClarinetTypeSignature(val_sig)
        ));
    }

    None
}

pub fn get_expression_documentation(
    position: &Position,
    expressions: &Vec<SymbolicExpression>,
    analysis: &ContractAnalysis,
) -> Option<String> {
    let expression_name = get_expression_name_at_position(position, expressions)?;
    let doc = API_REF
        .get(&expression_name.to_string())
        .map(|(documentation, _)| documentation.to_owned());
    if let Some(doc) = doc {
        return Some(doc);
    }

    if let Some(definition) = get_type_documentation(&expression_name, analysis) {
        return Some(definition);
    }

    // let and match bindings
    let expr = get_expression_at_position(position, expressions)?;
    if let Some(expr_type) = analysis.type_map.clone()?.get_type(&expr) {
        return Some(format!(
            "```\n{}\n```",
            ClarinetTypeSignature(expr_type.clone())
        ));
    }

    // contract-call?
    // tuple get

    None
}

#[derive(Clone, Debug)]
struct TupleKeyType {
    tuple: String,
    key_name: ClarityName,
    expr_type: TypeSignature,
}

#[derive(Clone, Debug)]
struct ContractCallType {
    contract_id: QualifiedContractIdentifier,
    function_name: ClarityName,
}

#[derive(Clone, Debug)]
enum ExpressionsType {
    TupleGet(TupleKeyType),
    StaticCall(ContractCallType),
    DynamicCall(ContractCallType),
}

#[derive(Clone, Debug)]
pub struct AtomTypes {
    pub types: HashMap<u64, ExpressionsType>,
    type_map: TypeMap,
    referenced_traits: HashMap<ClarityName, TraitDefinition>,
    local_trait_references: HashMap<ClarityName, ClarityName>,
}

impl<'a> AtomTypes {
    pub fn new(
        type_map: TypeMap,
        referenced_traits: HashMap<ClarityName, TraitDefinition>,
    ) -> Self {
        Self {
            types: HashMap::default(),
            local_trait_references: HashMap::default(),
            type_map,
            referenced_traits,
        }
    }

    pub fn run(&mut self, expressions: &'a [SymbolicExpression]) {
        traverse(self, expressions);
    }

    pub fn traverse_function(
        &mut self,
        _expr: &'a SymbolicExpression,
        _name: &'a ClarityName,
        parameters: Option<Vec<TypedVar<'a>>>,
        _body: &'a SymbolicExpression,
    ) -> bool {
        println!("VISIT_FUNCTION");
        if let Some(parameters) = parameters {
            for param in parameters {
                if let Some(trait_reference) = param.type_expr.match_trait_reference() {
                    println!("trait_reference: {:#?}", trait_reference);
                    self.local_trait_references
                        .insert(param.name.clone(), trait_reference.clone());
                }
            }
        }
        true
    }
}

impl<'a> ASTVisitor<'a> for AtomTypes {
    fn traverse_define_private(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        parameters: Option<Vec<TypedVar<'a>>>,
        body: &'a SymbolicExpression,
    ) -> bool {
        self.traverse_function(expr, name, parameters.clone(), body);
        self.traverse_expr(body) && self.visit_define_private(expr, name, parameters, body)
    }

    fn traverse_define_public(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        parameters: Option<Vec<clarity_repl::analysis::ast_visitor::TypedVar<'a>>>,
        body: &'a SymbolicExpression,
    ) -> bool {
        self.traverse_function(expr, name, parameters.clone(), body);
        self.traverse_expr(body) && self.visit_define_public(expr, name, parameters, body)
    }

    fn traverse_define_read_only(
        &mut self,
        expr: &'a SymbolicExpression,
        name: &'a ClarityName,
        parameters: Option<Vec<TypedVar<'a>>>,
        body: &'a SymbolicExpression,
    ) -> bool {
        self.traverse_function(expr, name, parameters.clone(), body);
        self.traverse_expr(body) && self.visit_define_read_only(expr, name, parameters, body)
    }

    fn visit_static_contract_call(
        &mut self,
        expr: &'a SymbolicExpression,
        contract_identifier: &'a QualifiedContractIdentifier,
        function_name: &'a ClarityName,
        _args: &'a [SymbolicExpression],
    ) -> bool {
        if let Some(function_name_expr) = expr.match_list().and_then(|list| list.get(2)) {
            self.types.insert(
                function_name_expr.id,
                ExpressionsType::StaticCall(ContractCallType {
                    contract_id: contract_identifier.clone(),
                    function_name: function_name.clone(),
                }),
            );
        };

        true
    }

    fn visit_dynamic_contract_call(
        &mut self,
        expr: &'a SymbolicExpression,
        trait_ref: &'a SymbolicExpression,
        _function_name: &'a ClarityName,
        _args: &'a [SymbolicExpression],
    ) -> bool {
        let function_name_expr = expr.match_list().and_then(|list| list.get(2));

        let used_trait = trait_ref
            .match_atom()
            .and_then(|trait_name| self.local_trait_references.get(trait_name))
            .and_then(|trait_ref| self.referenced_traits.get(trait_ref));

        if let (Some(function_name_expr), Some(used_trait)) = (function_name_expr, used_trait) {
            let contract_id = match used_trait {
                TraitDefinition::Defined(trait_id) => trait_id.contract_identifier.clone(),
                TraitDefinition::Imported(trait_id) => trait_id.contract_identifier.clone(),
            };
            if let Some(function_name) = function_name_expr.match_atom() {
                self.types.insert(
                    function_name_expr.id,
                    ExpressionsType::DynamicCall(ContractCallType {
                        contract_id,
                        function_name: function_name.clone(),
                    }),
                );
            }
        };

        true
    }

    fn visit_get(
        &mut self,
        expr: &'a SymbolicExpression,
        key: &'a ClarityName,
        tuple: &'a SymbolicExpression,
    ) -> bool {
        if let Some(key_atom_expr) = expr.match_list().and_then(|list| list.get(1)) {
            if let Some(expr_type) = self.type_map.get_type(expr) {
                self.types.insert(
                    key_atom_expr.id,
                    ExpressionsType::TupleGet(TupleKeyType {
                        tuple: tuple.to_string(),
                        key_name: key.clone(),
                        expr_type: expr_type.clone(),
                    }),
                );
            }
        };
        true
    }
}

#[cfg(test)]
mod tests {

    use std::collections::HashMap;

    use clarity_repl::{
        clarity::{
            analysis::{type_checker::contexts::TypeMap, ContractAnalysis},
            ast::ContractAST,
            vm::types::StandardPrincipalData,
            ClarityVersion, ContractEvaluationResult, EvaluationResult, StacksEpochId,
        },
        repl::{
            ClarityCodeSource, ClarityContract, ClarityInterpreter, ContractDeployer, Settings,
        },
    };
    use lsp_types::Position;

    use crate::common::requests::hover::AtomTypes;

    fn get_ast(source: &str) -> ContractAST {
        let contract = ClarityContract {
            code_source: ClarityCodeSource::ContractInMemory(source.to_string()),
            name: "contract".into(),
            deployer: ContractDeployer::DefaultDeployer,
            clarity_version: ClarityVersion::Clarity2,
            epoch: StacksEpochId::Epoch25,
        };

        let interpreter =
            ClarityInterpreter::new(StandardPrincipalData::transient(), Settings::default());

        let (ast, ..) = interpreter.build_ast(&contract);
        ast
    }

    fn get_analyses(sources: &[String]) -> Vec<(ContractAST, ContractAnalysis)> {
        let mut analyses = vec![];
        let mut interpreter =
            ClarityInterpreter::new(StandardPrincipalData::transient(), Settings::default());
        for (i, source) in sources.iter().enumerate() {
            let contract = ClarityContract {
                code_source: ClarityCodeSource::ContractInMemory(source.to_string()),
                name: format!("contract-{}", i),
                deployer: ContractDeployer::DefaultDeployer,
                clarity_version: ClarityVersion::Clarity3,
                epoch: StacksEpochId::Epoch30,
            };

            let result = interpreter
                .run(&contract, None, false, None)
                .unwrap()
                .result;
            if let EvaluationResult::Contract(eval) = result {
                analyses.push((eval.contract.ast, eval.contract.analysis));
            }
        }
        analyses
    }

    #[test]
    fn get_constant_simple_type() {
        let snippet = ["(define-constant owner tx-sender)", "(print owner)"].join("\n");
        let analyses = get_analyses(&[snippet]);
        let (ast, analysis) = analyses.first().unwrap();

        let position = Position {
            line: 2,
            character: 8,
        };

        println!("analysis {:#?}", analysis);

        // if let Some(expr_name) = get_expression_name_at_position(&position, &ast.expressions) {
        //     if let Some(t) = get_variable_type(&expr_name, &analysis) {
        //         println!("{:#?}", t.to_string());
        //     }
        // }
    }

    #[test]
    fn get_constant_complex_type() {
        let snippet = [
            r#"(define-constant val { status: "ok", res: (ok (list (some u1))) })"#,
            "(print val)",
        ]
        .join("\n");
        let analyses = get_analyses(&[snippet]);
        let (ast, _analysis) = analyses.first().unwrap();

        let position = Position {
            line: 2,
            character: 8,
        };

        println!("ast {:#?}", ast);
        // println!("analysis {:#?}", analysis);

        // if let Some(expr_name) = get_expression_name_at_position(&position, &ast.expressions) {
        //     if let Some(t) = get_variable_type(&expr_name, &analysis) {
        //         println!("t {:#?}", t.to_string());
        //     }
        // }
    }

    // #[test]
    // fn get_function_type() {
    //     let snippet = [
    //         "(define-data-var count uint u0)",
    //         "(define-read-only (get-count) (var-get count))",
    //         "(define-read-only (print-count) (print (get-count)))",
    //     ]
    //     .join("\n");
    //     let (ast, analysis) = get_analysis(&snippet);

    //     println!("analysis {:#?}", analysis);
    //     let type_map = analysis.type_map.unwrap();

    //     println!("ast: {:#?}", &ast);
    //     println!("types: \n{:#?}", type_map);

    //     let position = Position {
    //         line: 3,
    //         character: 41,
    //     };

    //     let documentation = get_expression_documentation(&position, &ast.expressions);

    //     let func = get_expression_at_position(&position, &ast.expressions);
    //     println!("func {:#?}", func);

    //     println!("documentation {:#?}", documentation);
    // }

    #[test]
    fn get_static_call_sig() {
        let trait_source = ["(define-public (add-n (n uint))", "  (ok (+ n u1))", ")"].join("\n");
        let contract_source = [
            "(define-public (add-1)",
            "  (contract-call? .contract-0 add-n u1)",
            ")",
        ]
        .join("\n");

        let analyses = get_analyses(&[trait_source, contract_source]);
        let (ast, _analysis) = analyses.get(1).unwrap();

        let mut atom_types = AtomTypes::new(TypeMap::new(false), ast.referenced_traits.clone());
        atom_types.run(&ast.expressions);

        println!("types {:#?}", &atom_types.types);
    }

    #[test]
    fn get_dynamic_call_sig() {
        let trait_source = [
            "(define-trait my-trait",
            "  ((func () (response uint uint)))",
            ")",
        ]
        .join("\n");
        let contract_source = [
            "(use-trait my-trait .contract-0.my-trait)",
            "(define-private (call-trait (t <my-trait>))",
            "  (contract-call? t func)",
            ")",
        ]
        .join("\n");

        let analyses = get_analyses(&[trait_source, contract_source]);
        let (ast, _analysis) = analyses.get(1).unwrap();

        let mut atom_types = AtomTypes::new(TypeMap::new(false), ast.referenced_traits.clone());
        atom_types.run(&ast.expressions);

        println!("types {:#?}", &atom_types.types);
    }
}
