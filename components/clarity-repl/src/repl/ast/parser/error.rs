use crate::clarity::diagnostic::{DiagnosableError, Level};
use crate::clarity::representations::Span;
use crate::repl::ast::parser::lexer::error::LexerError;
use crate::repl::ast::parser::lexer::token::Token;

#[derive(Debug, PartialEq)]
pub enum ParserError {
    // Errors
    Lexer(LexerError),
    ContractNameTooLong(String),
    ExpectedContractIdentifier,
    ExpectedTraitIdentifier,
    ExpectedIdentifier,
    ExpectedPrincipal,
    IllegalContractName,
    IllegalTraitName,
    InvalidPrincipalLiteral,
    InvalidBuffer,
    NameTooLong(String),
    UnexpectedToken(Token),
    ExpectedClosing(Token),
    TupleColonExpected,
    TupleCommaExpected,
    TupleValueExpected,
    IllegalClarityName(String),
    IllegalASCIIString(String),
    IllegalUtf8String(String),
    ExpectedWhitespace,
    // Notes
    NoteToMatchThis(Token),
}

pub struct PlacedError {
    pub e: ParserError,
    pub span: Span,
}

impl DiagnosableError for ParserError {
    fn message(&self) -> String {
        use self::ParserError::*;
        match self {
            Lexer(le) => le.message(),
            ContractNameTooLong(name) => format!("contract name '{}' is too long", name),
            ExpectedContractIdentifier => "expected contract identifier".to_string(),
            ExpectedTraitIdentifier => "expected trait identifier".to_string(),
            ExpectedIdentifier => "expected identifier".to_string(),
            ExpectedPrincipal => "expected principal".to_string(),
            IllegalContractName => "illegal contract name".to_string(),
            IllegalTraitName => "illegal trait name".to_string(),
            InvalidPrincipalLiteral => "invalid principal literal".to_string(),
            InvalidBuffer => "invalid hex-string literal".to_string(),
            NameTooLong(name) => format!("illegal name (too long), '{}'", name),
            UnexpectedToken(token) => format!("unexpected '{}'", token),
            ExpectedClosing(token) => format!("expected closing '{}'", token),
            TupleColonExpected => "expected ':' after key in tuple".to_string(),
            TupleCommaExpected => "expected ',' separating key-value pairs in tuple".to_string(),
            TupleValueExpected => "expected value expression for tuple".to_string(),
            IllegalClarityName(name) => format!("illegal clarity name, '{}'", name),
            IllegalASCIIString(s) => format!("illegal ascii string \"{}\"", s),
            IllegalUtf8String(s) => format!("illegal UTF8 string \"{}\"", s),
            ExpectedWhitespace => "expected whitespace before expression".to_string(),
            NoteToMatchThis(token) => format!("to match this '{}'", token),
        }
    }

    fn suggestion(&self) -> Option<String> {
        None
    }

    fn level(&self) -> crate::clarity::diagnostic::Level {
        use self::ParserError::*;
        match self {
            NoteToMatchThis(_) => Level::Note,
            Lexer(lexerError) => lexerError.level(),
            _ => Level::Error,
        }
    }
}
