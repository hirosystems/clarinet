use std::convert::TryInto;
use crate::clarity::{Value, apply, eval_all};
use crate::clarity::representations::{SymbolicExpression};
use crate::clarity::errors::{InterpreterResult as Result};
use crate::clarity::callables::CallableType;
use crate::clarity::contexts::{Environment, LocalContext, ContractContext, GlobalContext};
use crate::clarity::ast::ContractAST;
use crate::clarity::types::QualifiedContractIdentifier;

pub struct Contract {
    pub contract_context: ContractContext,
}

// AARON: this is an increasingly useless wrapper around a ContractContext struct.
//          will probably be removed soon.
impl Contract {
    pub fn initialize_from_ast (contract_identifier: QualifiedContractIdentifier, contract: &ContractAST, global_context: &mut GlobalContext) -> Result<Contract> {
        let mut contract_context = ContractContext::new(contract_identifier);

        eval_all(&contract.expressions, &mut contract_context, global_context)?;

        Ok(Contract { contract_context: contract_context })
    }

}
