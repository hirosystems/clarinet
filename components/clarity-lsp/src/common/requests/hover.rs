use std::fmt;

use clarity_repl::clarity::{
    analysis::ContractAnalysis,
    docs::{get_input_type_string, get_output_type_string},
    vm::types::{FunctionType, TypeSignature},
    ClarityName, SymbolicExpression,
};
use lsp_types::Position;

use crate::common::requests::helpers::get_expression_at_position;

use super::{
    api_ref::API_REF,
    helpers::{get_expression_name_at_position, get_function_at_position},
};

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

#[cfg(test)]
mod tests {

    use clarity_repl::{
        clarity::{
            analysis::ContractAnalysis, ast::ContractAST, vm::types::StandardPrincipalData,
            ClarityVersion, StacksEpochId,
        },
        repl::{
            ClarityCodeSource, ClarityContract, ClarityInterpreter, ContractDeployer, Settings,
        },
    };
    use lsp_types::Position;

    use crate::common::requests::{
        helpers::get_expression_name_at_position, hover::get_variable_type,
    };

    fn get_analysis(source: &str) -> (ContractAST, ContractAnalysis) {
        let contract = ClarityContract {
            code_source: ClarityCodeSource::ContractInMemory(source.to_string()),
            name: "contract".into(),
            deployer: ContractDeployer::DefaultDeployer,
            clarity_version: ClarityVersion::Clarity2,
            epoch: StacksEpochId::Epoch25,
        };

        let mut interpreter =
            ClarityInterpreter::new(StandardPrincipalData::transient(), Settings::default());

        let (ast, ..) = interpreter.build_ast(&contract);
        let (annotations, _) = interpreter.collect_annotations(source);

        let (analysis, _) = interpreter
            .run_analysis(&contract, &ast, &annotations)
            .unwrap();

        (ast, analysis)
    }

    #[test]
    fn get_constant_simple_type() {
        let snippet = ["(define-constant owner tx-sender)", "(print owner)"].join("\n");
        let (ast, analysis) = get_analysis(&snippet);

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
        let (ast, analysis) = get_analysis(&snippet);

        let position = Position {
            line: 2,
            character: 8,
        };

        println!("analysis {:#?}", analysis);

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
}
