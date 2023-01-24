use clarity::vm::{representations::depth_traverse, SymbolicExpression};

#[derive(Debug, Default)]
pub struct StaticCost {
    pub runtime: u64,
}

impl StaticCost {
    pub fn add_runtime(&mut self, value: u64) {
        self.runtime += value;
    }
}

pub fn get_cost_for_expr(expression: &SymbolicExpression) -> Result<(), String> {
    println!("{:}", expression);
    Ok(())
}

pub fn run_static_cost_analysis(expressions: &[SymbolicExpression]) -> Result<StaticCost, String> {
    let costs = StaticCost::default();
    for expr in expressions.iter() {
        let _ = depth_traverse(expr, |x| get_cost_for_expr(x));
    }

    Ok(costs)
}

#[cfg(test)]
mod test_run_static_cost_analysis {
    use clarity::types::StacksEpochId;
    use clarity::vm::{
        ast::{build_ast_with_rules, ASTRules},
        types::QualifiedContractIdentifier,
        ClarityVersion, SymbolicExpression,
    };

    use super::run_static_cost_analysis;

    fn get_ast(source: &str) -> Vec<SymbolicExpression> {
        let contract_ast = build_ast_with_rules(
            &QualifiedContractIdentifier::transient(),
            source,
            &mut (),
            ClarityVersion::Clarity2,
            StacksEpochId::Epoch21,
            ASTRules::PrecheckSize,
        )
        .unwrap();

        return contract_ast.expressions;
    }

    #[test]
    fn test_simple_expr() {
        let expressions = get_ast("(define-public (get-counter) (ok u1))");

        if let Ok(cost) = run_static_cost_analysis(&expressions) {
            println!("{:#?}", cost);
        }
    }
}
