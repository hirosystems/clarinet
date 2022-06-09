use crate::clarity::ast::ContractAST;
use crate::clarity::callables::CallableType;
use crate::clarity::contexts::{ContractContext, Environment, GlobalContext, LocalContext};
use crate::clarity::errors::InterpreterResult as Result;
use crate::clarity::representations::SymbolicExpression;
use crate::clarity::types::QualifiedContractIdentifier;
use crate::clarity::{apply, eval_all, Value};
use std::convert::TryInto;

#[derive(Serialize, Deserialize)]
pub struct Contract {
    pub contract_context: ContractContext,
}

// AARON: this is an increasingly useless wrapper around a ContractContext struct.
//          will probably be removed soon.
impl Contract {
    pub fn initialize_from_ast(
        contract_identifier: QualifiedContractIdentifier,
        contract: &ContractAST,
        global_context: &mut GlobalContext,
    ) -> Result<Contract> {
        let mut contract_context = ContractContext::new(contract_identifier);

        eval_all(&contract.expressions, &mut contract_context, global_context)?;

        Ok(Contract {
            contract_context: contract_context,
        })
    }
}
