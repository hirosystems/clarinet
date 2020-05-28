extern crate regex;

#[macro_use]
pub mod costs;

pub mod errors;
pub mod diagnostic;
pub mod types;
pub mod representations;
pub mod ast;
pub mod docs;
pub mod analysis;
pub mod util;
pub mod functions;


pub use types::Value;

pub use representations::{SymbolicExpression, SymbolicExpressionType, ClarityName, ContractName};

const MAX_CALL_STACK_DEPTH: usize = 128;
