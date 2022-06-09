use crate::clarity::costs::cost_functions::ClarityCostFunction;
use crate::clarity::costs::runtime_cost;
use crate::clarity::errors::{
    check_argument_count, check_arguments_at_least, CheckErrors, InterpreterResult as Result,
};
use crate::clarity::representations::SymbolicExpressionType::List;
use crate::clarity::representations::{SymbolicExpression, SymbolicExpressionType};
use crate::clarity::types::{TupleData, TypeSignature, Value};
use crate::clarity::{eval, Environment, LocalContext};

pub fn tuple_cons(
    args: &[SymbolicExpression],
    env: &mut Environment,
    context: &LocalContext,
) -> Result<Value> {
    //    (tuple (arg-name value)
    //           (arg-name value))
    use super::parse_eval_bindings;

    check_arguments_at_least(1, args)?;

    let bindings = parse_eval_bindings(args, env, context)?;
    runtime_cost(ClarityCostFunction::TupleCons, env, bindings.len())?;

    TupleData::from_data(bindings).map(Value::from)
}

pub fn tuple_get(
    args: &[SymbolicExpression],
    env: &mut Environment,
    context: &LocalContext,
) -> Result<Value> {
    // (get arg-name (tuple ...))
    //    if the tuple argument is an option type, then return option(field-name).
    check_argument_count(2, args)?;

    let arg_name = args[0].match_atom().ok_or(CheckErrors::ExpectedName)?;

    let value = eval(&args[1], env, context)?;

    match value {
        Value::Optional(opt_data) => {
            match opt_data.data {
                Some(data) => {
                    if let Value::Tuple(tuple_data) = *data {
                        runtime_cost(ClarityCostFunction::TupleGet, env, tuple_data.len())?;
                        Ok(Value::some(tuple_data.get_owned(arg_name)?)
                            .expect("Tuple contents should *always* fit in a some wrapper"))
                    } else {
                        Err(CheckErrors::ExpectedTuple(TypeSignature::type_of(&data)).into())
                    }
                }
                None => Ok(Value::none()), // just pass through none-types.
            }
        }
        Value::Tuple(tuple_data) => {
            runtime_cost(ClarityCostFunction::TupleGet, env, tuple_data.len())?;
            tuple_data.get_owned(arg_name)
        }
        _ => Err(CheckErrors::ExpectedTuple(TypeSignature::type_of(&value)).into()),
    }
}

pub fn tuple_merge(base: Value, update: Value) -> Result<Value> {
    let initial_values = match base {
        Value::Tuple(initial_values) => Ok(initial_values),
        _ => Err(CheckErrors::ExpectedTuple(TypeSignature::type_of(&base))),
    }?;

    let new_values = match update {
        Value::Tuple(new_values) => Ok(new_values),
        _ => Err(CheckErrors::ExpectedTuple(TypeSignature::type_of(&update))),
    }?;

    let combined = TupleData::shallow_merge(initial_values, new_values)?;
    Ok(Value::Tuple(combined))
}
