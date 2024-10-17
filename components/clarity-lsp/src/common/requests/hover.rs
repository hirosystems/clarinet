use clarity_repl::clarity::{SymbolicExpression, SymbolicExpressionType};
use lsp_types::Position;

use super::{api_ref::API_REF, helpers::get_expression_name_at_position};

pub fn get_expression_documentation(
    position: &Position,
    expressions: &Vec<SymbolicExpression>,
) -> Option<String> {
    let expression_name = get_expression_name_at_position(position, expressions)?;

    API_REF
        .get(&expression_name.to_string())
        .map(|(documentation, _)| documentation.to_owned())
}

#[cfg(test)]
mod test {
    use super::expr_type_to_string;

    use clarity_repl::clarity::{
        ast::{self, build_ast_with_rules},
        vm::types::QualifiedContractIdentifier,
        ClarityVersion, StacksEpochId, SymbolicExpression, SymbolicExpressionType, Value,
    };

    fn get_ast(source: &str) -> Vec<SymbolicExpression> {
        let contract_ast = build_ast_with_rules(
            &QualifiedContractIdentifier::transient(),
            source,
            &mut (),
            ClarityVersion::Clarity2,
            StacksEpochId::Epoch25,
            clarity_repl::clarity::ast::ASTRules::Typical,
        )
        .unwrap();
        contract_ast.expressions
    }

    #[test]
    fn test_expr_type_to_string() {
        let snippet = "(define-read-only (func (x uint)) (begin (ok x)))";
        let ast = get_ast(snippet);
        println!("{:#?}", ast);
        let func = &ast[0];
        if let SymbolicExpressionType::List(func) = &func.expr {
            println!("-----");
            println!("{:#?}", &func[2].expr);
        }

        // let v = Value::Int(1);
        // let expr = SymbolicExpression::atom_value(v);
        // dbg!(expr_type_to_string(&expr.expr));
    }
}
