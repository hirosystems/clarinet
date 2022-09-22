pub mod parser;
use clarity::vm::costs::cost_functions::ClarityCostFunction;
use clarity::vm::costs::{
    cost_functions, runtime_cost, CostErrors, CostTracker, LimitedCostTracker,
};
use clarity::vm::diagnostic::{Diagnostic, Level};
use clarity::vm::errors::{Error, RuntimeErrorType};

use clarity::vm::representations::SymbolicExpression;
use clarity::vm::types::QualifiedContractIdentifier;

use clarity::vm::ast::definition_sorter::DefinitionSorter;
use clarity::vm::ast::errors::ParseResult;
use clarity::vm::ast::expression_identifier::ExpressionIdentifier;
use clarity::vm::ast::stack_depth_checker::StackDepthChecker;
use clarity::vm::ast::sugar_expander::SugarExpander;
use clarity::vm::ast::traits_resolver::TraitsResolver;
use clarity::vm::ast::types::BuildASTPass;
pub use clarity::vm::ast::types::ContractAST;

use self::parser::error::ParserError;

pub fn build_ast<T: CostTracker>(
    contract_identifier: &QualifiedContractIdentifier,
    source_code: &str,
    cost_track: &mut T,
) -> (ContractAST, Vec<Diagnostic>, bool) {
    let cost_err = match runtime_cost(
        ClarityCostFunction::AstParse,
        cost_track,
        source_code.len() as u64,
    ) {
        Err(e) => Some(e),
        _ => None,
    };

    let (pre_expressions, mut diagnostics, mut success) = parser::parse(source_code);
    if let Some(e) = cost_err {
        diagnostics.insert(
            0,
            Diagnostic {
                level: Level::Error,
                message: format!("runtime_cost error: {:?}", e),
                spans: vec![],
                suggestion: None,
            },
        );
    }
    let mut contract_ast = ContractAST::new(contract_identifier.clone(), pre_expressions);
    match StackDepthChecker::run_pass(&mut contract_ast) {
        Err(e) => {
            diagnostics.push(e.diagnostic);
            success = false;
        }
        _ => (),
    }
    match ExpressionIdentifier::run_pre_expression_pass(&mut contract_ast) {
        Err(e) => {
            diagnostics.push(e.diagnostic);
            success = false;
        }
        _ => (),
    }
    match DefinitionSorter::run_pass(&mut contract_ast, cost_track) {
        Err(e) => {
            diagnostics.push(e.diagnostic);
            success = false;
        }
        _ => (),
    }
    match TraitsResolver::run_pass(&mut contract_ast) {
        Err(e) => {
            diagnostics.push(e.diagnostic);
            success = false;
        }
        _ => (),
    }
    match SugarExpander::run_pass(&mut contract_ast) {
        Err(e) => {
            diagnostics.push(e.diagnostic);
            success = false;
        }
        _ => (),
    }
    match ExpressionIdentifier::run_expression_pass(&mut contract_ast) {
        Err(e) => {
            diagnostics.push(e.diagnostic);
            success = false;
        }
        _ => (),
    }
    (contract_ast, diagnostics, success)
}
