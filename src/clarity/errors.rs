use std::fmt;
use std::error;
pub use super::analysis::errors::{CheckErrors};
pub use super::ast::errors::{ParseErrors};
pub use super::analysis::errors::{check_argument_count, check_arguments_at_least};
use super::types::{Value};
use super::ast::errors::ParseError;
use super::costs::CostErrors;

#[derive(Debug)]
pub struct IncomparableError<T> {
    pub err: T
}

#[derive(Debug)]
pub enum Error {
    Parser(ParseErrors),
    Runtime(RuntimeErrorType),
    Unchecked(CheckErrors),
    ShortReturn(ShortReturnType)
}

#[derive(Debug, PartialEq)]
pub enum ShortReturnType {
    ExpectedValue(Value),
    AssertionFailed(Value),
}

pub type InterpreterResult <R> = Result<R, Error>;

#[derive(Debug, PartialEq)]
pub enum RuntimeErrorType {
    Arithmetic(String),
    ArithmeticOverflow,
    ArithmeticUnderflow,
    SupplyOverflow(u128, u128),
    DivisionByZero,
    // error in parsing types
    ParseError(String),
    // error in parsing the AST
    ASTError(ParseError),
    MaxStackDepthReached,
    MaxContextDepthReached,
    ListDimensionTooHigh,
    BadTypeConstruction,
    ValueTooLarge,
    BadBlockHeight(String),
    TransferNonPositiveAmount,
    NoSuchToken,
    NotImplemented,
    NoSenderInContext,
    NonPositiveTokenSupply,
    AttemptToFetchInTransientContext,
    BadNameValue(&'static str, String),
    BadBlockHash(Vec<u8>),
    UnwrapFailure,
}

impl <T> PartialEq<IncomparableError<T>> for IncomparableError<T> {
    fn eq(&self, _other: &IncomparableError<T>) -> bool {
        return false
    }
}

impl PartialEq<Error> for Error {
    fn eq(&self, other: &Error) -> bool {
        match (self, other) {
            (Error::Unchecked(x), Error::Unchecked(y)) => x == y,
            (Error::ShortReturn(x), Error::ShortReturn(y)) => x == y,
            _ => false
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            _ =>  write!(f, "{:?}", self)
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

// impl error::Error for RuntimeErrorType {
//     fn source(&self) -> Option<&(dyn error::Error + 'static)> {
//         None
//     }
// }

// impl From<CostErrors> for Error {
//     fn from(err: CostErrors) -> Self {
//         Error::from(CheckErrors::from(err))
//     }
// }

// impl From<ParseError> for Error {
//     fn from(err: ParseError) -> Self {
//         Error::from(ParseErrors::from(err))
//     }
// }

// impl From<RuntimeErrorType> for Error {
//     fn from(err: RuntimeErrorType) -> Self {
//         Error::Runtime(err, None)
//     }
// }

impl From<CheckErrors> for Error {
    fn from(err: CheckErrors) -> Self {
        Error::Unchecked(err)
    }
}

impl From<ShortReturnType> for Error {
    fn from(err: ShortReturnType) -> Self {
        Error::ShortReturn(err)
    }
}

impl Into<Value> for ShortReturnType {
    fn into(self) -> Value {
        match self {
            ShortReturnType::ExpectedValue(v) => v,
            ShortReturnType::AssertionFailed(v) => v
        }
    }
}

impl fmt::Display for RuntimeErrorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl error::Error for RuntimeErrorType {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

impl From<RuntimeErrorType> for Error {
    fn from(err: RuntimeErrorType) -> Self {
        Error::Runtime(err)
    }
}
