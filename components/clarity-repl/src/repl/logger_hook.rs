use clarity::vm::errors::Error;
use clarity::vm::functions::NativeFunctions;
use clarity::vm::representations::Span;
use clarity::vm::types::QualifiedContractIdentifier;
use clarity::vm::{
    contexts::{Environment, LocalContext},
    types::Value,
    EvalHook, ExecutionResult, SymbolicExpression,
    SymbolicExpressionType::List,
};

pub struct ContractLog {
    contract_id: QualifiedContractIdentifier,
    span: Span,
    result: Value,
}

#[derive(Default)]
pub struct LoggerHook {
    logs: Vec<ContractLog>,
}

impl LoggerHook {
    pub fn new() -> Self {
        LoggerHook::default()
    }
}

impl EvalHook for LoggerHook {
    fn will_begin_eval(&mut self, _: &mut Environment, _: &LocalContext, _: &SymbolicExpression) {}

    fn did_finish_eval(
        &mut self,
        env: &mut Environment,
        _context: &LocalContext,
        expr: &SymbolicExpression,
        res: &Result<Value, Error>,
    ) {
        if let List(list) = &expr.expr {
            if let Some((function_name, _args)) = list.split_first() {
                if let Some(function_name) = function_name.match_atom() {
                    if let Some(NativeFunctions::Print) =
                        NativeFunctions::lookup_by_name(function_name)
                    {
                        if let Ok(value) = res {
                            self.logs.push(ContractLog {
                                contract_id: env.contract_context.contract_identifier.clone(),
                                span: expr.span.clone(),
                                result: value.clone(),
                            });
                        }
                    }
                }
            }
        }
    }

    fn did_complete(&mut self, _: Result<&mut ExecutionResult, String>) {}
}
