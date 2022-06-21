use crate::clarity::representations::{SymbolicExpression, SymbolicExpressionType};
use crate::clarity::types::{PrincipalData, TypeSignature, Value};

use crate::clarity::functions::tuples;

use super::check_special_tuple_cons;
use crate::clarity::analysis::type_checker::{
    check_arguments_at_least, no_type, CheckError, CheckErrors, TypeChecker, TypeResult,
    TypingContext,
};

use crate::clarity::costs::cost_functions::ClarityCostFunction;
use crate::clarity::costs::{analysis_typecheck_cost, cost_functions, runtime_cost};

pub fn check_special_fetch_entry(
    checker: &mut TypeChecker,
    args: &[SymbolicExpression],
    context: &TypingContext,
) -> TypeResult {
    check_arguments_at_least(2, args)?;

    let map_name = args[0].match_atom().ok_or(CheckErrors::BadMapName)?;

    let key_type = checker.type_check(&args[1], context)?;

    let (expected_key_type, value_type) = checker
        .contract_context
        .get_map_type(map_name)
        .ok_or(CheckErrors::NoSuchMap(map_name.to_string()))?;

    runtime_cost(
        ClarityCostFunction::AnalysisTypeLookup,
        &mut checker.cost_track,
        expected_key_type.type_size()?,
    )?;
    runtime_cost(
        ClarityCostFunction::AnalysisTypeLookup,
        &mut checker.cost_track,
        value_type.type_size()?,
    )?;
    analysis_typecheck_cost(&mut checker.cost_track, expected_key_type, &key_type)?;

    let option_type = TypeSignature::new_option(value_type.clone())?;

    if !expected_key_type.admits_type(&key_type) {
        return Err(CheckError::new(CheckErrors::TypeError(
            expected_key_type.clone(),
            key_type,
        )));
    } else {
        return Ok(option_type);
    }
}

pub fn check_special_delete_entry(
    checker: &mut TypeChecker,
    args: &[SymbolicExpression],
    context: &TypingContext,
) -> TypeResult {
    check_arguments_at_least(2, args)?;

    let map_name = args[0].match_atom().ok_or(CheckErrors::BadMapName)?;

    let key_type = checker.type_check(&args[1], context)?;

    let (expected_key_type, _) = checker
        .contract_context
        .get_map_type(map_name)
        .ok_or(CheckErrors::NoSuchMap(map_name.to_string()))?;

    runtime_cost(
        ClarityCostFunction::AnalysisTypeLookup,
        &mut checker.cost_track,
        expected_key_type.type_size()?,
    )?;
    analysis_typecheck_cost(&mut checker.cost_track, expected_key_type, &key_type)?;

    if !expected_key_type.admits_type(&key_type) {
        return Err(CheckError::new(CheckErrors::TypeError(
            expected_key_type.clone(),
            key_type,
        )));
    } else {
        return Ok(TypeSignature::BoolType);
    }
}

fn check_set_or_insert_entry(
    checker: &mut TypeChecker,
    args: &[SymbolicExpression],
    context: &TypingContext,
) -> TypeResult {
    check_arguments_at_least(3, args)?;

    let map_name = args[0].match_atom().ok_or(CheckErrors::BadMapName)?;

    let key_type = checker.type_check(&args[1], context)?;
    let value_type = checker.type_check(&args[2], context)?;

    let (expected_key_type, expected_value_type) = checker
        .contract_context
        .get_map_type(map_name)
        .ok_or(CheckErrors::NoSuchMap(map_name.to_string()))?;

    runtime_cost(
        ClarityCostFunction::AnalysisTypeLookup,
        &mut checker.cost_track,
        expected_key_type.type_size()?,
    )?;
    runtime_cost(
        ClarityCostFunction::AnalysisTypeLookup,
        &mut checker.cost_track,
        value_type.type_size()?,
    )?;

    analysis_typecheck_cost(&mut checker.cost_track, expected_key_type, &key_type)?;
    analysis_typecheck_cost(&mut checker.cost_track, expected_value_type, &value_type)?;

    if !expected_key_type.admits_type(&key_type) {
        return Err(CheckError::new(CheckErrors::TypeError(
            expected_key_type.clone(),
            key_type,
        )));
    } else if !expected_value_type.admits_type(&value_type) {
        return Err(CheckError::new(CheckErrors::TypeError(
            expected_value_type.clone(),
            value_type,
        )));
    } else {
        return Ok(TypeSignature::BoolType);
    }
}

pub fn check_special_set_entry(
    checker: &mut TypeChecker,
    args: &[SymbolicExpression],
    context: &TypingContext,
) -> TypeResult {
    check_set_or_insert_entry(checker, args, context)
}

pub fn check_special_insert_entry(
    checker: &mut TypeChecker,
    args: &[SymbolicExpression],
    context: &TypingContext,
) -> TypeResult {
    check_set_or_insert_entry(checker, args, context)
}
