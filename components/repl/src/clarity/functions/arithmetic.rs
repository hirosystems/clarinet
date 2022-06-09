use crate::clarity::errors::{
    check_argument_count, CheckErrors, InterpreterResult, RuntimeErrorType,
};
use crate::clarity::types::{TypeSignature, Value};
use integer_sqrt::IntegerSquareRoot;
use std::convert::TryFrom;

struct U128Ops();
struct I128Ops();

impl U128Ops {
    fn make_value(x: u128) -> InterpreterResult<Value> {
        Ok(Value::UInt(x))
    }
}

impl I128Ops {
    fn make_value(x: i128) -> InterpreterResult<Value> {
        Ok(Value::Int(x))
    }
}

// This macro checks the type of the required two arguments and then dispatches the evaluation
//   to the correct arithmetic type handler (after deconstructing the Clarity Values into
//   the corresponding Rust integer type.
macro_rules! type_force_binary_arithmetic {
    ($function: ident, $x: expr, $y: expr) => {{
        match ($x, $y) {
            (Value::Int(x), Value::Int(y)) => I128Ops::$function(x, y),
            (Value::UInt(x), Value::UInt(y)) => U128Ops::$function(x, y),
            (x, _) => Err(CheckErrors::UnionTypeValueError(
                vec![TypeSignature::IntType, TypeSignature::UIntType],
                x,
            )
            .into()),
        }
    }};
}

macro_rules! type_force_unary_arithmetic {
    ($function: ident, $x: expr) => {{
        match $x {
            Value::Int(x) => I128Ops::$function(x),
            Value::UInt(x) => U128Ops::$function(x),
            x => Err(CheckErrors::UnionTypeValueError(
                vec![TypeSignature::IntType, TypeSignature::UIntType],
                x,
            )
            .into()),
        }
    }};
}

// This macro checks the type of the first argument and then dispatches the evaluation
//   to the correct arithmetic type handler (after deconstructing the Clarity Values into
//   the corresponding Rust integer type.
macro_rules! type_force_variadic_arithmetic {
    ($function: ident, $args: expr) => {{
        let first = $args
            .get(0)
            .ok_or(CheckErrors::IncorrectArgumentCount(1, $args.len()))?;
        match first {
            Value::Int(_) => {
                let typed_args: Result<Vec<_>, _> = $args
                    .drain(..)
                    .map(|x| match x {
                        Value::Int(value) => Ok(value),
                        _ => Err(CheckErrors::TypeValueError(
                            TypeSignature::IntType,
                            x.clone(),
                        )),
                    })
                    .collect();
                let checked_args = typed_args?;
                I128Ops::$function(&checked_args)
            }
            Value::UInt(_) => {
                let typed_args: Result<Vec<_>, _> = $args
                    .drain(..)
                    .map(|x| match x {
                        Value::UInt(value) => Ok(value),
                        _ => Err(CheckErrors::TypeValueError(
                            TypeSignature::UIntType,
                            x.clone(),
                        )),
                    })
                    .collect();
                let checked_args = typed_args?;
                U128Ops::$function(&checked_args)
            }
            _ => Err(CheckErrors::UnionTypeValueError(
                vec![TypeSignature::IntType, TypeSignature::UIntType],
                first.clone(),
            )
            .into()),
        }
    }};
}

// This macro creates all of the operation functions for the two arithmetic types
//  (uint128 and int128) -- this is really hard to do generically because there's no
//  "Integer" trait in rust, so macros were the most straight-forward solution to do this
//  without a bunch of code duplication
macro_rules! make_arithmetic_ops {
    ($struct_name: ident, $type:ty) => {
        impl $struct_name {
            fn xor(x: $type, y: $type) -> InterpreterResult<Value> {
                Self::make_value(x ^ y)
            }
            fn leq(x: $type, y: $type) -> InterpreterResult<Value> {
                Ok(Value::Bool(x <= y))
            }
            fn geq(x: $type, y: $type) -> InterpreterResult<Value> {
                Ok(Value::Bool(x >= y))
            }
            fn greater(x: $type, y: $type) -> InterpreterResult<Value> {
                Ok(Value::Bool(x > y))
            }
            fn less(x: $type, y: $type) -> InterpreterResult<Value> {
                Ok(Value::Bool(x < y))
            }
            fn add(args: &[$type]) -> InterpreterResult<Value> {
                let result = args
                    .iter()
                    .try_fold(0, |acc: $type, x: &$type| acc.checked_add(*x))
                    .ok_or(RuntimeErrorType::ArithmeticOverflow)?;
                Self::make_value(result)
            }
            fn sub(args: &[$type]) -> InterpreterResult<Value> {
                let (first, rest) = args
                    .split_first()
                    .ok_or(CheckErrors::IncorrectArgumentCount(1, 0))?;
                if rest.len() == 0 {
                    // return negation
                    return Self::make_value(
                        first
                            .checked_neg()
                            .ok_or(RuntimeErrorType::ArithmeticUnderflow)?,
                    );
                }

                let result = rest
                    .iter()
                    .try_fold(*first, |acc: $type, x: &$type| acc.checked_sub(*x))
                    .ok_or(RuntimeErrorType::ArithmeticUnderflow)?;
                Self::make_value(result)
            }
            fn mul(args: &[$type]) -> InterpreterResult<Value> {
                let result = args
                    .iter()
                    .try_fold(1, |acc: $type, x: &$type| acc.checked_mul(*x))
                    .ok_or(RuntimeErrorType::ArithmeticOverflow)?;
                Self::make_value(result)
            }
            fn div(args: &[$type]) -> InterpreterResult<Value> {
                let (first, rest) = args
                    .split_first()
                    .ok_or(CheckErrors::IncorrectArgumentCount(1, 0))?;
                let result = rest
                    .iter()
                    .try_fold(*first, |acc: $type, x: &$type| acc.checked_div(*x))
                    .ok_or(RuntimeErrorType::DivisionByZero)?;
                Self::make_value(result)
            }
            fn modulo(numerator: $type, denominator: $type) -> InterpreterResult<Value> {
                let result = numerator
                    .checked_rem(denominator)
                    .ok_or(RuntimeErrorType::DivisionByZero)?;
                Self::make_value(result)
            }
            #[allow(unused_comparisons)]
            fn pow(base: $type, power: $type) -> InterpreterResult<Value> {
                if base == 0 && power == 0 {
                    // Note that 0⁰ (pow(0, 0)) returns 1. Mathematically this is undefined (https://docs.rs/num-traits/0.2.10/num_traits/pow/fn.pow.html)
                    return Self::make_value(1);
                }
                if base == 1 {
                    return Self::make_value(1);
                }

                if base == 0 {
                    return Self::make_value(0);
                }

                if power == 1 {
                    return Self::make_value(base);
                }

                if power < 0 || power > (u32::max_value() as $type) {
                    return Err(RuntimeErrorType::Arithmetic(
                        "Power argument to (pow ...) must be a u32 integer".to_string(),
                    )
                    .into());
                }

                let power_u32 = power as u32;

                let result = base
                    .checked_pow(power_u32)
                    .ok_or(RuntimeErrorType::ArithmeticOverflow)?;
                Self::make_value(result)
            }
            fn sqrti(n: $type) -> InterpreterResult<Value> {
                match n.integer_sqrt_checked() {
                    Some(result) => Self::make_value(result),
                    None => {
                        return Err(RuntimeErrorType::Arithmetic(
                            "sqrti must be passed a positive integer".to_string(),
                        )
                        .into())
                    }
                }
            }
            fn log2(n: $type) -> InterpreterResult<Value> {
                if n < 1 {
                    return Err(RuntimeErrorType::Arithmetic(
                        "log2 must be passed a positive integer".to_string(),
                    )
                    .into());
                }
                let size = std::mem::size_of::<$type>() as u32;
                Self::make_value((size * 8 - 1 - n.leading_zeros()) as $type)
            }
        }
    };
}

make_arithmetic_ops!(U128Ops, u128);
make_arithmetic_ops!(I128Ops, i128);

pub fn native_xor(a: Value, b: Value) -> InterpreterResult<Value> {
    type_force_binary_arithmetic!(xor, a, b)
}
pub fn native_geq(a: Value, b: Value) -> InterpreterResult<Value> {
    type_force_binary_arithmetic!(geq, a, b)
}
pub fn native_leq(a: Value, b: Value) -> InterpreterResult<Value> {
    type_force_binary_arithmetic!(leq, a, b)
}
pub fn native_ge(a: Value, b: Value) -> InterpreterResult<Value> {
    type_force_binary_arithmetic!(greater, a, b)
}
pub fn native_le(a: Value, b: Value) -> InterpreterResult<Value> {
    type_force_binary_arithmetic!(less, a, b)
}
pub fn native_add(mut args: Vec<Value>) -> InterpreterResult<Value> {
    type_force_variadic_arithmetic!(add, args)
}
pub fn native_sub(mut args: Vec<Value>) -> InterpreterResult<Value> {
    type_force_variadic_arithmetic!(sub, args)
}
pub fn native_mul(mut args: Vec<Value>) -> InterpreterResult<Value> {
    type_force_variadic_arithmetic!(mul, args)
}
pub fn native_div(mut args: Vec<Value>) -> InterpreterResult<Value> {
    type_force_variadic_arithmetic!(div, args)
}
pub fn native_pow(a: Value, b: Value) -> InterpreterResult<Value> {
    type_force_binary_arithmetic!(pow, a, b)
}
pub fn native_sqrti(n: Value) -> InterpreterResult<Value> {
    type_force_unary_arithmetic!(sqrti, n)
}
pub fn native_log2(n: Value) -> InterpreterResult<Value> {
    type_force_unary_arithmetic!(log2, n)
}
pub fn native_mod(a: Value, b: Value) -> InterpreterResult<Value> {
    type_force_binary_arithmetic!(modulo, a, b)
}

pub fn native_to_uint(input: Value) -> InterpreterResult<Value> {
    if let Value::Int(int_val) = input {
        let uint_val =
            u128::try_from(int_val).map_err(|_| RuntimeErrorType::ArithmeticUnderflow)?;
        Ok(Value::UInt(uint_val))
    } else {
        Err(CheckErrors::TypeValueError(TypeSignature::IntType, input).into())
    }
}

pub fn native_to_int(input: Value) -> InterpreterResult<Value> {
    if let Value::UInt(uint_val) = input {
        let int_val = i128::try_from(uint_val).map_err(|_| RuntimeErrorType::ArithmeticOverflow)?;
        Ok(Value::Int(int_val))
    } else {
        Err(CheckErrors::TypeValueError(TypeSignature::UIntType, input).into())
    }
}
