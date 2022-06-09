use crate::clarity::costs::{CostErrors, ExecutionCost};
use crate::clarity::diagnostic::{DiagnosableError, Diagnostic};
use crate::clarity::representations::{PreSymbolicExpression, SymbolicExpression};
use crate::clarity::types::{TupleTypeSignature, TypeSignature};
use crate::clarity::MAX_CALL_STACK_DEPTH;
use std::error;
use std::fmt;

pub type ParseResult<T> = Result<T, ParseError>;

#[derive(Debug, PartialEq)]
pub enum ParseErrors {
    CostOverflow,
    CostBalanceExceeded(ExecutionCost, ExecutionCost),
    MemoryBalanceExceeded(u64, u64),
    TooManyExpressions,
    ExpressionStackDepthTooDeep,
    FailedCapturingInput,
    SeparatorExpected(String),
    SeparatorExpectedAfterColon(String),
    ProgramTooLarge,
    IllegalVariableName(String),
    IllegalContractName(String),
    UnknownQuotedValue(String),
    FailedParsingIntValue(String),
    FailedParsingBuffer(String),
    FailedParsingHexValue(String, String),
    FailedParsingPrincipal(String),
    FailedParsingField(String),
    FailedParsingRemainder(String),
    ClosingParenthesisUnexpected,
    ClosingParenthesisExpected,
    ClosingTupleLiteralUnexpected,
    ClosingTupleLiteralExpected,
    CircularReference(Vec<String>),
    TupleColonExpected(usize),
    TupleCommaExpected(usize),
    TupleItemExpected(usize),
    NameAlreadyUsed(String),
    TraitReferenceNotAllowed,
    ImportTraitBadSignature,
    DefineTraitBadSignature,
    ImplTraitBadSignature,
    TraitReferenceUnknown(String),
    CommaSeparatorUnexpected,
    ColonSeparatorUnexpected,
    InvalidCharactersDetected,
    InvalidEscaping,
    CostComputationFailed(String),
}

#[derive(Debug, PartialEq)]
pub struct ParseError {
    pub err: ParseErrors,
    pub pre_expressions: Option<Vec<PreSymbolicExpression>>,
    pub diagnostic: Diagnostic,
}

impl ParseError {
    pub fn new(err: ParseErrors) -> ParseError {
        let diagnostic = Diagnostic::err(&err);
        ParseError {
            err,
            pre_expressions: None,
            diagnostic,
        }
    }

    pub fn has_pre_expression(&self) -> bool {
        self.pre_expressions.is_some()
    }

    pub fn set_pre_expression(&mut self, expr: &PreSymbolicExpression) {
        self.diagnostic.spans = vec![expr.span.clone()];
        self.pre_expressions.replace(vec![expr.clone()]);
    }

    pub fn set_pre_expressions(&mut self, exprs: Vec<PreSymbolicExpression>) {
        self.diagnostic.spans = exprs.iter().map(|e| e.span.clone()).collect();
        self.pre_expressions.replace(exprs.to_vec());
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.err {
            _ => write!(f, "{:?}", self.err),
        }?;

        if let Some(ref e) = self.pre_expressions {
            write!(f, "\nNear:\n{:?}", e)?;
        }

        Ok(())
    }
}

impl error::Error for ParseError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self.err {
            _ => None,
        }
    }
}

impl From<ParseErrors> for ParseError {
    fn from(err: ParseErrors) -> Self {
        ParseError::new(err)
    }
}

impl From<CostErrors> for ParseError {
    fn from(err: CostErrors) -> Self {
        match err {
            CostErrors::CostOverflow => ParseError::new(ParseErrors::CostOverflow),
            CostErrors::CostBalanceExceeded(a, b) => {
                ParseError::new(ParseErrors::CostBalanceExceeded(a, b))
            }
            CostErrors::MemoryBalanceExceeded(a, b) => {
                ParseError::new(ParseErrors::MemoryBalanceExceeded(a, b))
            }
            CostErrors::CostComputationFailed(s) => {
                ParseError::new(ParseErrors::CostComputationFailed(s))
            }
            CostErrors::CostContractLoadFailure => ParseError::new(
                ParseErrors::CostComputationFailed("Failed to load cost contract".into()),
            ),
        }
    }
}

impl DiagnosableError for ParseErrors {
    fn message(&self) -> String {
        match &self {
            ParseErrors::CostOverflow => format!("Used up cost budget during the parse"),
            ParseErrors::CostBalanceExceeded(bal, used) => format!(
                "Used up cost budget during the parse: {} balance, {} used",
                bal, used
            ),
            ParseErrors::MemoryBalanceExceeded(bal, used) => format!(
                "Used up memory budget during the parse: {} balance, {} used",
                bal, used
            ),
            ParseErrors::TooManyExpressions => format!("Too many expressions"),
            ParseErrors::FailedCapturingInput => format!("Failed to capture value from input"),
            ParseErrors::SeparatorExpected(found) => {
                format!("Expected whitespace or a close parens. Found: '{}'", found)
            }
            ParseErrors::SeparatorExpectedAfterColon(found) => {
                format!("Whitespace expected after colon (:), Found: '{}'", found)
            }
            ParseErrors::ProgramTooLarge => format!("Program too large to parse"),
            ParseErrors::IllegalContractName(contract_name) => {
                format!("Illegal contract name: '{}'", contract_name)
            }
            ParseErrors::IllegalVariableName(var_name) => {
                format!("Illegal variable name: '{}'", var_name)
            }
            ParseErrors::UnknownQuotedValue(value) => format!("Unknown 'quoted value '{}'", value),
            ParseErrors::FailedParsingIntValue(value) => {
                format!("Failed to parse int literal '{}'", value)
            }
            ParseErrors::FailedParsingHexValue(value, x) => {
                format!("Invalid hex-string literal {}: {}", value, x)
            }
            ParseErrors::FailedParsingPrincipal(value) => {
                format!("Invalid principal literal: {}", value)
            }
            ParseErrors::FailedParsingBuffer(value) => format!("Invalid buffer literal: {}", value),
            ParseErrors::FailedParsingField(value) => format!("Invalid field literal: {}", value),
            ParseErrors::FailedParsingRemainder(remainder) => {
                format!("Failed to lex input remainder: '{}'", remainder)
            }
            ParseErrors::ClosingParenthesisUnexpected => {
                format!("Tried to close list which isn't open.")
            }
            ParseErrors::ClosingParenthesisExpected => {
                format!("List expressions (..) left opened.")
            }
            ParseErrors::ClosingTupleLiteralUnexpected => {
                format!("Tried to close tuple literal which isn't open.")
            }
            ParseErrors::ClosingTupleLiteralExpected => {
                format!("Tuple literal {{..}} left opened.")
            }
            ParseErrors::ColonSeparatorUnexpected => format!("Misplaced colon."),
            ParseErrors::CommaSeparatorUnexpected => format!("Misplaced comma."),
            ParseErrors::TupleColonExpected(i) => {
                format!("Tuple literal construction expects a colon at index {}", i)
            }
            ParseErrors::TupleCommaExpected(i) => {
                format!("Tuple literal construction expects a comma at index {}", i)
            }
            ParseErrors::TupleItemExpected(i) => format!(
                "Tuple literal construction expects a key or value at index {}",
                i
            ),
            ParseErrors::CircularReference(function_names) => format!(
                "detected interdependent functions ({})",
                function_names.join(", ")
            ),
            ParseErrors::NameAlreadyUsed(name) => {
                format!("defining '{}' conflicts with previous value", name)
            }
            ParseErrors::ImportTraitBadSignature => {
                format!("(use-trait ...) expects a trait name and a trait identifier")
            }
            ParseErrors::DefineTraitBadSignature => {
                format!("(define-trait ...) expects a trait name and a trait definition")
            }
            ParseErrors::ImplTraitBadSignature => {
                format!("(impl-trait ...) expects a trait identifier")
            }
            ParseErrors::TraitReferenceNotAllowed => format!("trait references can not be stored"),
            ParseErrors::TraitReferenceUnknown(trait_name) => {
                format!("use of undeclared trait <{}>", trait_name)
            }
            ParseErrors::ExpressionStackDepthTooDeep => format!(
                "AST has too deep of an expression nesting. The maximum stack depth is {}",
                MAX_CALL_STACK_DEPTH
            ),
            ParseErrors::InvalidCharactersDetected => format!("invalid characters detected"),
            ParseErrors::InvalidEscaping => format!("invalid escaping detected in string"),
            ParseErrors::CostComputationFailed(s) => format!("Cost computation failed: {}", s),
        }
    }

    fn suggestion(&self) -> Option<String> {
        match &self {
            _ => None,
        }
    }
}
