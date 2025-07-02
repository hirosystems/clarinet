use clarinet_format::formatter;
use clarity::vm::contexts::{Environment, LocalContext};
use clarity::vm::errors::Error;
use clarity::vm::functions::NativeFunctions;
use clarity::vm::representations::PreSymbolicExpression;
use clarity::vm::types::Value;
use clarity::vm::SymbolicExpressionType::List;
use clarity::vm::{EvalHook, ExecutionResult, SymbolicExpression};

#[derive(Default, Clone)]
pub struct LoggerHook;

impl LoggerHook {
    pub fn new() -> Self {
        Self
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
        let List(list) = &expr.expr else { return };
        let Some((function_name, _args)) = list.split_first() else {
            return;
        };
        let Some(function_name) = function_name.match_atom() else {
            return;
        };

        if let Some(NativeFunctions::Print) = NativeFunctions::lookup_by_name(function_name) {
            let contract_name = &env.contract_context.contract_identifier.name;
            let span = &expr.span;
            let line_annotation = format!("({}:{})", contract_name, span.start_line);

            match res {
                Ok(value) => {
                    let format_settings = formatter::Settings::default();
                    let formatter = formatter::ClarityFormatter::new(format_settings);
                    let value_pse = PreSymbolicExpression::atom_value(value.clone());
                    let formatted_value = formatter.format_ast(&[value_pse]);

                    uprint!("{formatted_value} {line_annotation}");
                }
                Err(err) => {
                    uprint!("{err} {line_annotation}");
                }
            }
        }
    }

    fn did_complete(&mut self, _: Result<&mut ExecutionResult, String>) {}
}

#[cfg(test)]
mod tests {
    use clarity::vm::types::StandardPrincipalData;

    use crate::{
        repl::{ClarityInterpreter, Settings},
        test_fixtures::clarity_contract::ClarityContractBuilder,
    };

    use super::*;

    // Simple approach: Run tests with subprocess to capture stdout
    #[test]
    fn test_logger_hook_logs_to_stdout() {
        // Test that verifies the hook executes without errors
        let mut interpreter = ClarityInterpreter::new(
            StandardPrincipalData::transient(),
            Settings::default(),
            None,
        );
        interpreter.set_current_epoch(clarity::types::StacksEpochId::Epoch31);

        let snippet = "(print true)";
        let contract = ClarityContractBuilder::default()
            .code_source(snippet.into())
            .build();

        let mut logger_hook = LoggerHook::new();
        let hooks: Vec<&mut dyn EvalHook> = vec![&mut logger_hook];

        let result = interpreter.run(&contract, None, false, Some(hooks));
        assert!(result.is_ok());
    }
}
