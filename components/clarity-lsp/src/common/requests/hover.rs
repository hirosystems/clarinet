use clarity_repl::clarity::SymbolicExpression;
use lsp_types::Position;

use super::{api_ref::API_REF, helpers::get_expression_name_at_position};

pub fn get_expression_documentation(
    position: &Position,
    expressions: &Vec<SymbolicExpression>,
) -> Option<String> {
    let expression_name = get_expression_name_at_position(position, expressions)?;
    println!("expression: {}", expression_name);

    API_REF
        .get(&expression_name.to_string())
        .map(|(documentation, _)| documentation.to_owned())
}

#[cfg(test)]
mod tests {
    use std::path;

    use clarity_repl::{
        clarity::{
            ast::{build_ast_with_rules, ASTRules},
            vm::types::{QualifiedContractIdentifier, StandardPrincipalData},
            ClarityVersion, StacksEpochId, SymbolicExpression,
        },
        repl::{
            ClarityCodeSource, ClarityContract, ClarityInterpreter, ContractDeployer, Settings,
        },
    };
    use lsp_types::Position;

    use super::get_expression_documentation;

    fn get_ast(source: &str) -> Vec<SymbolicExpression> {
        let contract_ast = build_ast_with_rules(
            &QualifiedContractIdentifier::transient(),
            source,
            &mut (),
            ClarityVersion::Clarity2,
            StacksEpochId::Epoch25,
            ASTRules::PrecheckSize,
        )
        .unwrap();

        contract_ast.expressions
    }

    fn get_analysis(source: &str) {
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
    }

    #[test]
    fn get_constant_type() {
        let snippet = ["(define-constant owner tx-sender)", "(print owner)"].join("\n");
        let ast = get_ast(&snippet);

        let documentation = get_expression_documentation(
            &Position {
                line: 2,
                character: 8,
            },
            &ast,
        );

        println!("documentation {:#?}", documentation);
    }

    #[test]
    fn get_function_type() {
        let snippet = [
            "(define-data-var count uint u0)",
            "(define-read-only (get-count) (var-get count))",
            "(define-read-only (print-count) (print (get-count)))",
        ]
        .join("\n");

        // let documentation = get_expression_documentation(
        //     &Position {
        //         line: 3,
        //         character: 41,
        //     },
        //     &ast,
        // );

        // println!("documentation {:#?}", documentation);
    }
}
