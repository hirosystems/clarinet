use crate::repl::interpreter::Txid;
use crate::repl::tracer::SymbolicExpressionType::List;
use clarity::vm::errors::Error;
use clarity::vm::functions::define::DefineFunctions;
use clarity::vm::functions::NativeFunctions;
use clarity::vm::{
    contexts::{Environment, LocalContext},
    types::Value,
    EvalHook, SymbolicExpression, SymbolicExpressionType,
};
use clarity::vm::{eval, ClarityVersion, EvaluationResult};

pub struct Tracer {
    stack: Vec<u64>,
    pending_call_string: Vec<String>,
    pending_args: Vec<Vec<u64>>,
    nb_of_emitted_events: usize,
}

impl Tracer {
    pub fn new(snippet: &str) -> Tracer {
        println!("{snippet}  {}", black!("<console>"));
        Tracer {
            stack: vec![u64::MAX],
            pending_call_string: Vec::new(),
            pending_args: Vec::new(),
            nb_of_emitted_events: 0,
        }
    }
}

impl EvalHook for Tracer {
    fn will_begin_eval(
        &mut self,
        env: &mut Environment,
        context: &LocalContext,
        expr: &SymbolicExpression,
    ) {
        let List(list) = &expr.expr else {
            return;
        };
        if let Some((function_name, args)) = list.split_first() {
            if let Some(function_name) = function_name.match_atom() {
                if DefineFunctions::lookup_by_name(function_name).is_some() {
                    return;
                } else if let Some(native_function) = NativeFunctions::lookup_by_name_at_version(
                    function_name,
                    &ClarityVersion::latest(),
                ) {
                    match native_function {
                        NativeFunctions::ContractCall => {
                            let mut call = format!(
                                "{}├── {}  {}\n",
                                "│   ".repeat(
                                    (self.stack.len() - self.pending_call_string.len())
                                        .saturating_sub(1)
                                ),
                                expr,
                                black!(format!(
                                    "{}:{}:{}",
                                    env.contract_context.contract_identifier.name,
                                    expr.span.start_line,
                                    expr.span.start_column,
                                )),
                            );

                            let mut lines = Vec::new();
                            if args[0].match_atom().is_some() {
                                let callee = if let Ok(value) = eval(&args[0], env, context) {
                                    value.to_string()
                                } else {
                                    "?".to_string()
                                };
                                lines.push(format!(
                                    "{}│ {}",
                                    "│   "
                                        .repeat(self.stack.len() - self.pending_call_string.len()),
                                    purple!(format!("↳ callee: {}", callee)),
                                ));
                            }

                            if !args.is_empty() {
                                lines.push(format!(
                                    "{}│ {}",
                                    "│   "
                                        .repeat(self.stack.len() - self.pending_call_string.len()),
                                    purple!("↳ args:"),
                                ));
                                call.push_str(lines.join("\n").as_str());
                                self.pending_call_string.push(call);
                                self.pending_args
                                    .push(args[2..].iter().map(|arg| arg.id).collect());
                            } else {
                                println!(
                                    "{}{}",
                                    "│   "
                                        .repeat(self.stack.len() - self.pending_call_string.len()),
                                    call
                                );
                            }
                        }
                        _ => return,
                    }
                } else {
                    // Call user-defined function
                    let mut call = format!(
                        "{}├── {}  {}\n",
                        "│   ".repeat(
                            (self.stack.len() - self.pending_call_string.len()).saturating_sub(1)
                        ),
                        expr,
                        black!(format!(
                            "{}:{}:{}",
                            env.contract_context.contract_identifier.name,
                            expr.span.start_line,
                            expr.span.start_column,
                        )),
                    );
                    call.push_str(
                        format!(
                            "{}│ {}",
                            "│   ".repeat(self.stack.len() - self.pending_call_string.len()),
                            purple!("↳ args:"),
                        )
                        .as_str(),
                    );
                    if !args.is_empty() {
                        self.pending_call_string.push(call);
                        self.pending_args
                            .push(args.iter().map(|arg| arg.id).collect());
                    } else {
                        println!(
                            "{}{}",
                            "│   ".repeat(self.stack.len() - self.pending_call_string.len()),
                            call
                        );
                    }
                }
            }
        }
        self.stack.push(expr.id);
    }

    fn did_finish_eval(
        &mut self,
        env: &mut Environment,
        _context: &LocalContext,
        expr: &SymbolicExpression,
        res: &Result<Value, Error>,
    ) {
        // Print events as they are emitted
        let emitted_events = env
            .global_context
            .event_batches
            .iter()
            .flat_map(|b| &b.events)
            .collect::<Vec<_>>();
        if emitted_events.len() > self.nb_of_emitted_events {
            for event in emitted_events.iter().skip(self.nb_of_emitted_events) {
                println!(
                    "{}│ {}",
                    "│   ".repeat(
                        (self.stack.len() - self.pending_call_string.len()).saturating_sub(1)
                    ),
                    black!(format!(
                        "✸ {}",
                        event
                            .json_serialize(0, &Txid([0u8; 32]), true)
                            .expect("Failed to serialize event")
                    )),
                )
            }
            self.nb_of_emitted_events = emitted_events.len();
        }

        if let Some(last) = self.stack.last() {
            if *last == expr.id {
                if let Ok(value) = res {
                    println!(
                        "{}└── {}",
                        "│   ".repeat(
                            (self.stack.len() - self.pending_call_string.len()).saturating_sub(1)
                        ),
                        blue!(value.to_string())
                    );
                }
                self.stack.pop();
            }
        }

        // Collect argument values
        if let Some(arg_stack) = self.pending_args.last_mut() {
            if let Some((arg, rest)) = arg_stack.split_first() {
                if *arg == expr.id {
                    if let Ok(value) = res {
                        self.pending_call_string
                            .last_mut()
                            .unwrap()
                            .push_str(format!(" {}", value).as_str());
                    }

                    // If this was the last argument, print the pending call and pop the stack
                    if rest.is_empty() {
                        println!("{}", self.pending_call_string.pop().unwrap());
                        self.pending_args.pop();
                    } else {
                        arg_stack.remove(0);
                    }
                }
            }
        }
    }

    fn did_complete(
        &mut self,
        result: core::result::Result<&mut clarity::vm::ExecutionResult, String>,
    ) {
        match result {
            Ok(result) => {
                match &result.result {
                    EvaluationResult::Contract(contract_result) => {
                        if let Some(value) = &contract_result.result {
                            println!("└── {}", blue!(format!("{}", value)));
                        }
                    }
                    EvaluationResult::Snippet(snippet_result) => {
                        println!("└── {}", blue!(format!("{}", snippet_result.result)));
                    }
                };
            }
            Err(e) => println!("{}", e),
        }
    }
}
