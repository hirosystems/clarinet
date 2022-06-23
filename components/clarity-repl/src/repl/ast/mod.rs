pub mod parser;
use crate::clarity::costs::cost_functions::ClarityCostFunction;
use crate::clarity::costs::{
    cost_functions, runtime_cost, CostErrors, CostTracker, LimitedCostTracker,
};
use crate::clarity::diagnostic::{Diagnostic, Level};
use crate::clarity::errors::{Error, RuntimeErrorType};

use crate::clarity::representations::SymbolicExpression;
use crate::clarity::types::QualifiedContractIdentifier;

use crate::clarity::ast::definition_sorter::DefinitionSorter;
use crate::clarity::ast::errors::ParseResult;
use crate::clarity::ast::expression_identifier::ExpressionIdentifier;
use crate::clarity::ast::stack_depth_checker::StackDepthChecker;
use crate::clarity::ast::sugar_expander::SugarExpander;
use crate::clarity::ast::traits_resolver::TraitsResolver;
use crate::clarity::ast::types::BuildASTPass;
pub use crate::clarity::ast::types::ContractAST;

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
