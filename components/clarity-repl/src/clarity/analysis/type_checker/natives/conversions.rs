use crate::clarity::analysis::read_only_checker::check_argument_count;
use crate::clarity::analysis::type_checker::contexts::TypingContext;
use crate::clarity::analysis::type_checker::{TypeChecker, TypeResult};
use crate::clarity::analysis::CheckError;
use crate::clarity::types::{BufferLength, SequenceSubtype, TypeSignature};
use crate::clarity::SymbolicExpression;
use std::convert::TryFrom;

/// to-consensus-buff admits exactly one argument:
///   * the Clarity value to serialize
/// it returns an `(optional (buff x))` where `x` is the maximum possible
/// consensus buffer length based on the inferred type of the supplied value.
pub fn check_special_to_consensus_buff(
    checker: &mut TypeChecker,
    args: &[SymbolicExpression],
    context: &TypingContext,
) -> TypeResult {
    check_argument_count(1, args)?;
    let input_type = checker.type_check(&args[0], context)?;
    let buffer_max_len = BufferLength::try_from(input_type.max_serialized_size()?)?;
    TypeSignature::new_option(TypeSignature::SequenceType(SequenceSubtype::BufferType(
        buffer_max_len,
    )))
    .map_err(CheckError::from)
}

/// from-consensus-buff admits exactly two arguments:
///   * a type signature indicating the expected return type `t1`
///   * a buffer (of up to max length)
/// it returns an `(optional t1)`
pub fn check_special_from_consensus_buff(
    checker: &mut TypeChecker,
    args: &[SymbolicExpression],
    context: &TypingContext,
) -> TypeResult {
    check_argument_count(2, args)?;
    let result_type = TypeSignature::parse_type_repr(&args[0], checker)?;
    checker.type_check_expects(&args[1], context, &TypeSignature::max_buffer())?;
    TypeSignature::new_option(result_type).map_err(CheckError::from)
}
