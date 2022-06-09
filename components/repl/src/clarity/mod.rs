extern crate regex;

pub mod diagnostic;
pub mod errors;

#[macro_use]
pub mod util;

#[macro_use]
pub mod codec;

#[macro_use]
pub mod costs;

pub mod contracts;
pub mod events;
pub mod types;

pub mod ast;
pub mod clarity;
pub mod contexts;
pub mod database;
pub mod representations;

mod callables;
pub mod functions;
pub mod variables;

pub mod analysis;
pub mod docs;

pub mod coverage;
#[cfg(any(feature = "cli", feature = "dap"))]
pub mod debug;

use crate::clarity::callables::CallableType;
use crate::clarity::contexts::GlobalContext;
use crate::clarity::contexts::{CallStack, ContractContext, Environment, LocalContext};
use crate::clarity::costs::{
    cost_functions, runtime_cost, CostOverflowingMath, CostTracker, LimitedCostTracker,
    MemoryConsumer,
};
use crate::clarity::database::Datastore;
use crate::clarity::errors::{
    CheckErrors, Error, InterpreterError, InterpreterResult as Result, RuntimeErrorType,
};
use crate::clarity::functions::define::DefineResult;
pub use crate::clarity::types::Value;
use crate::clarity::types::{
    PrincipalData, QualifiedContractIdentifier, TraitIdentifier, TypeSignature,
};

pub use crate::clarity::representations::{
    ClarityName, ContractName, SymbolicExpression, SymbolicExpressionType,
};

pub use crate::clarity::contexts::MAX_CONTEXT_DEPTH;
use crate::clarity::costs::cost_functions::ClarityCostFunction;
pub use crate::clarity::functions::stx_transfer_consolidated;
use crate::repl::ExecutionResult;
use std::convert::{TryFrom, TryInto};

const MAX_CALL_STACK_DEPTH: usize = 64;

pub struct StacksBlockId(pub [u8; 32]);
impl_array_newtype!(StacksBlockId, u8, 32);
impl_array_hexstring_fmt!(StacksBlockId);
// impl_byte_array_newtype!(StacksBlockId, u8, 32);
// impl_byte_array_from_column!(StacksBlockId);

pub struct BlockHeaderHash(pub [u8; 32]);
impl_array_newtype!(BlockHeaderHash, u8, 32);
impl_array_hexstring_fmt!(BlockHeaderHash);
// impl_byte_array_newtype!(BlockHeaderHash, u8, 32);

pub struct VRFSeed(pub [u8; 32]);
impl_array_newtype!(VRFSeed, u8, 32);
impl_array_hexstring_fmt!(VRFSeed);
// impl_byte_array_newtype!(VRFSeed, u8, 32);

pub struct BurnchainHeaderHash(pub [u8; 32]);
impl_array_newtype!(BurnchainHeaderHash, u8, 32);
impl_array_hexstring_fmt!(BurnchainHeaderHash);
// impl_byte_array_newtype!(BurnchainHeaderHash, u8, 32);

/** EvalHook defines an interface for hooks to execute during evaluation. */
pub trait EvalHook {
    // Called before the expression is evaluated
    fn will_begin_eval(
        &mut self,
        env: &mut Environment,
        context: &LocalContext,
        expr: &SymbolicExpression,
    ) {
    }

    // Called after the expression is evaluated
    fn did_finish_eval(
        &mut self,
        env: &mut Environment,
        context: &LocalContext,
        expr: &SymbolicExpression,
        res: &core::result::Result<Value, crate::clarity::errors::Error>,
    ) {
    }

    // Called upon completion of the execution
    fn did_complete(&mut self, result: core::result::Result<&mut ExecutionResult, String>) {}
}

fn lookup_variable(name: &str, context: &LocalContext, env: &mut Environment) -> Result<Value> {
    if name.starts_with(char::is_numeric) || name.starts_with('\'') {
        Err(InterpreterError::BadSymbolicRepresentation(format!(
            "Unexpected variable name: {}",
            name
        ))
        .into())
    } else {
        if let Some(value) = variables::lookup_reserved_variable(name, context, env)? {
            Ok(value)
        } else {
            runtime_cost(
                ClarityCostFunction::LookupVariableDepth,
                env,
                context.depth(),
            )?;
            if let Some(value) = context
                .lookup_variable(name)
                .or_else(|| env.contract_context.lookup_variable(name))
            {
                runtime_cost(ClarityCostFunction::LookupVariableSize, env, value.size())?;
                Ok(value.clone())
            } else if let Some(value) = context.lookup_callable_contract(name) {
                let contract_identifier = &value.0;
                Ok(Value::Principal(PrincipalData::Contract(
                    contract_identifier.clone(),
                )))
            } else {
                Err(CheckErrors::UndefinedVariable(name.to_string()).into())
            }
        }
    }
}

pub fn lookup_function(name: &str, env: &mut Environment) -> Result<CallableType> {
    runtime_cost(ClarityCostFunction::LookupFunction, env, 0)?;

    if let Some(result) = functions::lookup_reserved_functions(name) {
        Ok(result)
    } else {
        let user_function = env
            .contract_context
            .lookup_function(name)
            .ok_or(CheckErrors::UndefinedFunction(name.to_string()))?;
        Ok(CallableType::UserFunction(user_function))
    }
}

fn add_stack_trace(result: &mut Result<Value>, env: &Environment) {
    if let Err(Error::Runtime(_, ref mut stack_trace)) = result {
        if stack_trace.is_none() {
            stack_trace.replace(env.call_stack.make_stack_trace());
        }
    }
}

pub fn apply(
    function: &CallableType,
    args: &[SymbolicExpression],
    env: &mut Environment,
    context: &LocalContext,
) -> Result<Value> {
    let identifier = function.get_identifier();
    // Aaron: in non-debug executions, we shouldn't track a full call-stack.
    //        only enough to do recursion detection.

    // do recursion check on user functions.
    let track_recursion = match function {
        CallableType::UserFunction(_) => true,
        _ => false,
    };

    if track_recursion && env.call_stack.contains(&identifier) {
        return Err(CheckErrors::CircularReference(vec![identifier.to_string()]).into());
    }

    if env.call_stack.depth() >= MAX_CALL_STACK_DEPTH {
        return Err(RuntimeErrorType::MaxStackDepthReached.into());
    }

    if let CallableType::SpecialFunction(_, function) = function {
        env.call_stack.insert(&identifier, track_recursion);
        let mut resp = function(args, env, context);
        add_stack_trace(&mut resp, env);
        env.call_stack.remove(&identifier, track_recursion)?;
        resp
    } else {
        let mut used_memory = 0;
        let mut evaluated_args = vec![];
        env.call_stack.incr_apply_depth();
        for arg_x in args.iter() {
            let arg_value = match eval(arg_x, env, context) {
                Ok(x) => x,
                Err(e) => {
                    env.drop_memory(used_memory);
                    env.call_stack.decr_apply_depth();
                    return Err(e);
                }
            };
            let arg_use = arg_value.get_memory_use();
            match env.add_memory(arg_use) {
                Ok(_x) => {}
                Err(e) => {
                    env.drop_memory(used_memory);
                    env.call_stack.decr_apply_depth();
                    return Err(Error::from(e));
                }
            };
            used_memory += arg_value.get_memory_use();
            evaluated_args.push(arg_value);
        }
        env.call_stack.decr_apply_depth();

        env.call_stack.insert(&identifier, track_recursion);
        let mut resp = match function {
            CallableType::NativeFunction(_, function, cost_function) => {
                runtime_cost(*cost_function, env, evaluated_args.len())
                    .map_err(Error::from)
                    .and_then(|_| function.apply(evaluated_args))
            }
            CallableType::UserFunction(function) => function.apply(&evaluated_args, env),
            _ => panic!("Should be unreachable."),
        };
        add_stack_trace(&mut resp, env);
        env.drop_memory(used_memory);
        env.call_stack.remove(&identifier, track_recursion)?;
        resp
    }
}

pub fn eval<'a>(
    exp: &SymbolicExpression,
    env: &'a mut Environment,
    context: &LocalContext,
) -> Result<Value> {
    use crate::clarity::representations::SymbolicExpressionType::{
        Atom, AtomValue, Field, List, LiteralValue, TraitReference,
    };

    if let Some(mut eval_hooks) = env.global_context.eval_hooks.take() {
        for hook in eval_hooks.iter_mut() {
            hook.will_begin_eval(env, context, exp);
        }
        env.global_context.eval_hooks = Some(eval_hooks);
    }

    let mut res = match exp.expr {
        AtomValue(ref value) | LiteralValue(ref value) => Ok(value.clone()),
        Atom(ref value) => lookup_variable(&value, context, env),
        List(ref children) => {
            let (function_variable, rest) = children
                .split_first()
                .ok_or(CheckErrors::NonFunctionApplication)?;

            let function_name = function_variable
                .match_atom()
                .ok_or(CheckErrors::BadFunctionName)?;
            let f = lookup_function(&function_name, env)?;
            apply(&f, &rest, env, context)
        }
        TraitReference(_, _) | Field(_) => unreachable!("can't be evaluated"),
    };

    if let Err(ref mut e) = res {
        match e {
            Error::Runtime(_, Some(stack)) => {
                if stack.is_empty() {
                    stack.append(&mut env.call_stack.stack.clone());
                    if let Some(bp) = stack.last_mut() {
                        bp.identifier.push_str(&format!("${}", exp.id))
                    }
                }
            }
            _ => {}
        };
    }

    if let Some(mut eval_hooks) = env.global_context.eval_hooks.take() {
        for hook in eval_hooks.iter_mut() {
            hook.did_finish_eval(env, context, exp, &res);
        }
        env.global_context.eval_hooks = Some(eval_hooks);
    }

    res
}

pub fn is_reserved(name: &str) -> bool {
    if let Some(_result) = functions::lookup_reserved_functions(name) {
        true
    } else if variables::is_reserved_name(name) {
        true
    } else {
        false
    }
}

/* This function evaluates a list of expressions, sharing a global context.
 * It returns the final evaluated result.
 */
pub fn eval_all(
    expressions: &[SymbolicExpression],
    contract_context: &mut ContractContext,
    global_context: &mut GlobalContext,
) -> Result<Option<Value>> {
    let mut last_executed = None;
    let context = LocalContext::new();
    let mut total_memory_use = 0;

    let publisher: PrincipalData = contract_context.contract_identifier.issuer.clone().into();

    finally_drop_memory!(global_context, total_memory_use; {
        for exp in expressions {
            let try_define = global_context.execute(|context| {
                let mut call_stack = CallStack::new();
                let mut env = Environment::new(
                    context, contract_context, &mut call_stack, Some(publisher.clone()), Some(publisher.clone()));
                functions::define::evaluate_define(exp, &mut env)
            })?;
            match try_define {
                DefineResult::Variable(name, value) => {
                    runtime_cost(ClarityCostFunction::BindName, global_context, 0)?;
                    let value_memory_use = value.get_memory_use();
                    global_context.add_memory(value_memory_use)?;
                    total_memory_use += value_memory_use;

                    contract_context.variables.insert(name, value);
                },
                DefineResult::Function(name, value) => {
                    runtime_cost(ClarityCostFunction::BindName, global_context, 0)?;

                    contract_context.functions.insert(name, value);
                },
                DefineResult::PersistedVariable(name, value_type, value) => {
                    runtime_cost(ClarityCostFunction::CreateVar, global_context, value_type.size())?;
                    contract_context.persisted_names.insert(name.clone());

                    global_context.add_memory(value_type.type_size()
                                              .expect("type size should be realizable") as u64)?;

                    global_context.add_memory(value.size() as u64)?;

                    let data_type = global_context.database.create_variable(&contract_context.contract_identifier, &name, value_type);
                    global_context.database.set_variable(&contract_context.contract_identifier, &name, value, &data_type)?;

                    contract_context.meta_data_var.insert(name, data_type);
                },
                DefineResult::Map(name, key_type, value_type) => {
                    runtime_cost(ClarityCostFunction::CreateMap, global_context,
                                  u64::from(key_type.size()).cost_overflow_add(
                                      u64::from(value_type.size()))?)?;
                    contract_context.persisted_names.insert(name.clone());

                    global_context.add_memory(key_type.type_size()
                                              .expect("type size should be realizable") as u64)?;
                    global_context.add_memory(value_type.type_size()
                                              .expect("type size should be realizable") as u64)?;

                    let data_type = global_context.database.create_map(&contract_context.contract_identifier, &name, key_type, value_type);

                    contract_context.meta_data_map.insert(name, data_type);
                },
                DefineResult::FungibleToken(name, total_supply) => {
                    runtime_cost(ClarityCostFunction::CreateFt, global_context, 0)?;
                    contract_context.persisted_names.insert(name.clone());

                    global_context.add_memory(TypeSignature::UIntType.type_size()
                                              .expect("type size should be realizable") as u64)?;

                    let data_type = global_context.database.create_fungible_token(&contract_context.contract_identifier, &name, &total_supply);

                    contract_context.meta_ft.insert(name, data_type);
                },
                DefineResult::NonFungibleAsset(name, asset_type) => {
                    runtime_cost(ClarityCostFunction::CreateNft, global_context, asset_type.size())?;
                    contract_context.persisted_names.insert(name.clone());

                    global_context.add_memory(asset_type.type_size()
                                              .expect("type size should be realizable") as u64)?;

                    let data_type = global_context.database.create_non_fungible_token(&contract_context.contract_identifier, &name, &asset_type);

                    contract_context.meta_nft.insert(name, data_type);
                },
                DefineResult::Trait(name, trait_type) => {
                    contract_context.defined_traits.insert(name, trait_type);
                },
                DefineResult::UseTrait(_name, _trait_identifier) => {},
                DefineResult::ImplTrait(trait_identifier) => {
                    contract_context.implemented_traits.insert(trait_identifier);
                },
                DefineResult::NoDefine => {
                    // not a define function, evaluate normally.
                    global_context.execute(|global_context| {
                        let mut call_stack = CallStack::new();
                        let mut env = Environment::new(
                            global_context, contract_context, &mut call_stack, Some(publisher.clone()), Some(publisher.clone()));

                        let result = eval(exp, &mut env, &context)?;
                        last_executed = Some(result);
                        Ok(())
                    })?;
                }
            }
        }

        contract_context.data_size = total_memory_use;
        Ok(last_executed)
    })
}
