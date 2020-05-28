pub mod parser;
pub mod expression_identifier;
pub mod definition_sorter;
pub mod traits_resolver;

pub mod sugar_expander;
pub mod types;
pub mod errors;
pub mod stack_depth_checker;
use crate::clarity::errors::{Error, RuntimeErrorType};

use crate::clarity::costs::{cost_functions, CostTracker};

use crate::clarity::representations::{SymbolicExpression};
use crate::clarity::types::QualifiedContractIdentifier;

pub use self::types::ContractAST;
use self::types::BuildASTPass;
use self::errors::ParseResult;
use self::expression_identifier::ExpressionIdentifier;
use self::sugar_expander::SugarExpander;
use self::definition_sorter::DefinitionSorter;
use self::traits_resolver::TraitsResolver;
use self::stack_depth_checker::StackDepthChecker;

pub fn build_ast<T: CostTracker>(contract_identifier: &QualifiedContractIdentifier, source_code: &str, cost_track: &mut T) -> ParseResult<ContractAST> {
    runtime_cost!(cost_functions::AST_PARSE, cost_track, source_code.len() as u64)?;
    let pre_expressions = parser::parse(source_code)?;
    let mut contract_ast = ContractAST::new(contract_identifier.clone(), pre_expressions);
    StackDepthChecker::run_pass(&mut contract_ast)?;
    ExpressionIdentifier::run_pre_expression_pass(&mut contract_ast)?;
    DefinitionSorter::run_pass(&mut contract_ast, cost_track)?;
    TraitsResolver::run_pass(&mut contract_ast)?;
    SugarExpander::run_pass(&mut contract_ast)?;
    ExpressionIdentifier::run_expression_pass(&mut contract_ast)?;
    Ok(contract_ast)
}
