use clarity::vm::costs::ExecutionCost;
use clarity::vm::errors::Error;
use clarity::vm::types::QualifiedContractIdentifier;
use clarity::vm::SymbolicExpressionType;
use clarity::vm::{
    contexts::{Environment, LocalContext},
    types::Value,
    EvalHook, SymbolicExpression,
};
use std::fmt::Display;
use std::io::Write;

struct StackEntry {
    contract: QualifiedContractIdentifier,
    function: String,
    expr_id: u64,
    line: u32,
    column: u32,
    cost_before: ExecutionCost,
    cost_descendents: ExecutionCost,
}

impl Display for StackEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}{}:{}:{}",
            self.contract, self.function, self.line, self.column,
        )
    }
}

pub struct PerfHook {
    /// Writer for outputting performance metrics
    writer: Box<dyn Write>,
    /// Stack of expressions
    expr_stack: Vec<StackEntry>,
}

impl PerfHook {
    pub fn new() -> PerfHook {
        const DEFAULT_OUTPUT: &str = "perf.data";
        let writer: Box<dyn Write> = Box::new(
            std::fs::File::create(DEFAULT_OUTPUT).expect("Failed to create perf output file"),
        );
        PerfHook {
            writer,
            expr_stack: Vec::new(),
        }
    }
}

impl Default for PerfHook {
    fn default() -> Self {
        Self::new()
    }
}

impl EvalHook for PerfHook {
    fn will_begin_eval(
        &mut self,
        env: &mut Environment,
        _context: &LocalContext,
        expr: &SymbolicExpression,
    ) {
        let contract = &env.contract_context.contract_identifier;

        // Find the current function name in the call stack
        let call_stack = env.call_stack.make_stack_trace();
        let mut function = String::new();
        for identifier in call_stack.iter().rev() {
            let s = identifier.to_string();
            if s.starts_with("_native_") {
                continue;
            }

            if let Some(f) = s.strip_prefix(&format!("{contract}:")) {
                function = format!(":{f}");
            } else {
                break;
            }
        }

        // If the expression is a list, extract the function name and append it
        // to the function string
        if let SymbolicExpressionType::List(list) = &expr.expr {
            if let Some((function_name, _args)) = list.split_first() {
                if let Some(function_name) = function_name.match_atom() {
                    function = format!("{function}:{function_name}");
                }
            }
        }

        // Record the cost before evaluating this expression
        let cost_before = env.global_context.cost_track.get_total();

        let line = expr.span.start_line;
        let column = expr.span.start_column;

        self.expr_stack.push(StackEntry {
            contract: contract.clone(),
            function,
            expr_id: expr.id,
            line,
            column,
            cost_before,
            cost_descendents: ExecutionCost::ZERO,
        });
    }

    fn did_finish_eval(
        &mut self,
        env: &mut Environment,
        _context: &LocalContext,
        expr: &SymbolicExpression,
        _res: &Result<Value, Error>,
    ) {
        // Get the full call stack as a string
        let call_stack = self
            .expr_stack
            .iter()
            .map(|e| e.to_string())
            .collect::<Vec<_>>()
            .join(";");

        // Pop the last entry from the expression stack
        let entry = self.expr_stack.pop().expect("expr stack underflow");
        assert_eq!(
            entry.expr_id, expr.id,
            "Expression ID mismatch: expected {}, got {}",
            entry.expr_id, expr.id
        );

        // Get the current cost
        let mut cost = env.global_context.cost_track.get_total();

        // Subtract the cost before evaluation and the cost of descendents to
        // get the cost of this expression
        cost.sub(&entry.cost_before)
            .expect("cost diff calculation failed");
        cost.sub(&entry.cost_descendents)
            .expect("cost diff calculation failed");

        // Write the performance data to the output
        writeln!(self.writer, "{call_stack} {}", cost.runtime)
            .expect("Failed to write to perf output");

        // Add the cost of this expression and its descendents to the parent
        // expression's cost so that it is not double-counted.
        if let Some(parent) = self.expr_stack.last_mut() {
            parent
                .cost_descendents
                .add(&cost)
                .expect("cost addition failed");
            parent
                .cost_descendents
                .add(&entry.cost_descendents)
                .expect("cost addition failed");
        }
    }

    fn did_complete(
        &mut self,
        _result: core::result::Result<&mut clarity::vm::ExecutionResult, String>,
    ) {
        // No action needed after evaluation completion
    }
}
