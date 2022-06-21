use crate::clarity::costs::cost_functions::ClarityCostFunction;
use crate::clarity::costs::runtime_cost;
use crate::clarity::errors::{
    check_argument_count, check_arguments_at_least, CheckErrors, InterpreterResult as Result,
};
use crate::clarity::representations::SymbolicExpression;
use crate::clarity::types::{TypeSignature, Value};
use crate::clarity::{eval, Environment, LocalContext};

fn type_force_bool(value: &Value) -> Result<bool> {
    match *value {
        Value::Bool(boolean) => Ok(boolean),
        _ => Err(CheckErrors::TypeValueError(TypeSignature::BoolType, value.clone()).into()),
    }
}

pub fn special_or(
    args: &[SymbolicExpression],
    env: &mut Environment,
    context: &LocalContext,
) -> Result<Value> {
    check_arguments_at_least(1, args)?;

    runtime_cost(ClarityCostFunction::Or, env, args.len())?;

    for arg in args.iter() {
        let evaluated = eval(&arg, env, context)?;
        let result = type_force_bool(&evaluated)?;
        if result {
            return Ok(Value::Bool(true));
        }
    }

    Ok(Value::Bool(false))
}

pub fn special_and(
    args: &[SymbolicExpression],
    env: &mut Environment,
    context: &LocalContext,
) -> Result<Value> {
    check_arguments_at_least(1, args)?;

    runtime_cost(ClarityCostFunction::And, env, args.len())?;

    for arg in args.iter() {
        let evaluated = eval(&arg, env, context)?;
        let result = type_force_bool(&evaluated)?;
        if !result {
            return Ok(Value::Bool(false));
        }
    }

    Ok(Value::Bool(true))
}

pub fn native_not(input: Value) -> Result<Value> {
    let value = type_force_bool(&input)?;
    Ok(Value::Bool(!value))
}
