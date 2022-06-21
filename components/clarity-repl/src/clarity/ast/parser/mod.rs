use crate::clarity::ast::errors::{ParseError, ParseErrors, ParseResult};
use crate::clarity::errors::{InterpreterResult as Result, RuntimeErrorType};
use crate::clarity::representations::{
    ClarityName, ContractName, PreSymbolicExpression, PreSymbolicExpressionType, MAX_STRING_LEN,
};
use crate::clarity::types::{PrincipalData, QualifiedContractIdentifier, TraitIdentifier, Value};
use crate::clarity::util::c32::c32_address_decode;
use crate::clarity::util::hash::hex_bytes;
use regex::{Captures, Regex};
use std::cmp;
use std::convert::TryInto;

pub const CONTRACT_MIN_NAME_LENGTH: usize = 1;
pub const CONTRACT_MAX_NAME_LENGTH: usize = 40;

pub enum LexItem {
    LeftParen,
    RightParen,
    LeftCurly,
    RightCurly,
    LiteralValue(usize, Value),
    SugaredContractIdentifier(usize, ContractName),
    SugaredFieldIdentifier(usize, ContractName, ClarityName),
    FieldIdentifier(usize, TraitIdentifier),
    TraitReference(usize, ClarityName),
    Variable(String),
    CommaSeparator,
    ColonSeparator,
    Whitespace,
}

#[derive(Debug)]
enum TokenType {
    Whitespace,
    Comma,
    Colon,
    LParens,
    RParens,
    LCurly,
    RCurly,
    StringASCIILiteral,
    StringUTF8Literal,
    HexStringLiteral,
    UIntLiteral,
    IntLiteral,
    Variable,
    TraitReferenceLiteral,
    PrincipalLiteral,
    SugaredContractIdentifierLiteral,
    FullyQualifiedContractIdentifierLiteral,
    SugaredFieldIdentifierLiteral,
    FullyQualifiedFieldIdentifierLiteral,
}

struct LexMatcher {
    matcher: Regex,
    handler: TokenType,
}

enum LexContext {
    ExpectNothing,
    ExpectClosing,
    ExpectClosingColon,
}

enum ParseContext {
    CollectList,
    CollectTuple,
}

impl LexMatcher {
    fn new(regex_str: &str, handles: TokenType) -> LexMatcher {
        LexMatcher {
            matcher: Regex::new(&format!("^{}", regex_str)).unwrap(),
            handler: handles,
        }
    }
}

fn get_value_or_err(input: &str, captures: Captures) -> ParseResult<String> {
    let matched = captures
        .name("value")
        .ok_or(ParseError::new(ParseErrors::FailedCapturingInput))?;
    Ok(input[matched.start()..matched.end()].to_string())
}

fn get_lines_at(input: &str) -> Vec<usize> {
    let mut out: Vec<_> = input.match_indices("\n").map(|(ix, _)| ix).collect();
    out.reverse();
    out
}

lazy_static! {
    pub static ref STANDARD_PRINCIPAL_REGEX: String =
        "[0123456789ABCDEFGHJKMNPQRSTVWXYZ]{28,41}".into();
    pub static ref CONTRACT_NAME_REGEX: String = format!(
        r#"([a-zA-Z](([a-zA-Z0-9]|[-_])){{{},{}}})"#,
        CONTRACT_MIN_NAME_LENGTH - 1,
        CONTRACT_MAX_NAME_LENGTH - 1
    );
    pub static ref CONTRACT_PRINCIPAL_REGEX: String = format!(
        r#"{}(\.){}"#,
        *STANDARD_PRINCIPAL_REGEX, *CONTRACT_NAME_REGEX
    );
    pub static ref PRINCIPAL_DATA_REGEX: String = format!(
        "({})|({})",
        *STANDARD_PRINCIPAL_REGEX, *CONTRACT_PRINCIPAL_REGEX
    );
    pub static ref CLARITY_NAME_REGEX: String =
        format!(r#"([[:word:]]|[-!?+<>=/*]){{1,{}}}"#, MAX_STRING_LEN);
}

pub fn lex(input: &str) -> ParseResult<Vec<(LexItem, u32, u32)>> {
    // Aaron: I'd like these to be static, but that'd require using
    //    lazy_static (or just hand implementing that), and I'm not convinced
    //    it's worth either (1) an extern macro, or (2) the complexity of hand implementing.

    let lex_matchers: &[LexMatcher] = &[
        LexMatcher::new(
            r##"u"(?P<value>((\\")|([[ -~]&&[^"]]))*)""##,
            TokenType::StringUTF8Literal,
        ),
        LexMatcher::new(
            r##""(?P<value>((\\")|([[ -~]&&[^"]]))*)""##,
            TokenType::StringASCIILiteral,
        ),
        LexMatcher::new(";;[ -~]*", TokenType::Whitespace), // ;; comments.
        LexMatcher::new("[\n]+", TokenType::Whitespace),
        LexMatcher::new("[ \t]+", TokenType::Whitespace),
        LexMatcher::new("[,]", TokenType::Comma),
        LexMatcher::new("[:]", TokenType::Colon),
        LexMatcher::new("[(]", TokenType::LParens),
        LexMatcher::new("[)]", TokenType::RParens),
        LexMatcher::new("[{]", TokenType::LCurly),
        LexMatcher::new("[}]", TokenType::RCurly),
        LexMatcher::new(
            "<(?P<value>([[:word:]]|[-])+)>",
            TokenType::TraitReferenceLiteral,
        ),
        LexMatcher::new("0x(?P<value>[[:xdigit:]]*)", TokenType::HexStringLiteral),
        LexMatcher::new("u(?P<value>[[:digit:]]+)", TokenType::UIntLiteral),
        LexMatcher::new("(?P<value>-?[[:digit:]]+)", TokenType::IntLiteral),
        LexMatcher::new(
            &format!(
                r#"'(?P<value>{}(\.)([[:alnum:]]|[-]){{1,{}}})"#,
                *CONTRACT_PRINCIPAL_REGEX, MAX_STRING_LEN
            ),
            TokenType::FullyQualifiedFieldIdentifierLiteral,
        ),
        LexMatcher::new(
            &format!(
                r#"(?P<value>(\.){}(\.)([[:alnum:]]|[-]){{1,{}}})"#,
                *CONTRACT_NAME_REGEX, MAX_STRING_LEN
            ),
            TokenType::SugaredFieldIdentifierLiteral,
        ),
        LexMatcher::new(
            &format!(r#"'(?P<value>{})"#, *CONTRACT_PRINCIPAL_REGEX),
            TokenType::FullyQualifiedContractIdentifierLiteral,
        ),
        LexMatcher::new(
            &format!(r#"(?P<value>(\.){})"#, *CONTRACT_NAME_REGEX),
            TokenType::SugaredContractIdentifierLiteral,
        ),
        LexMatcher::new(
            &format!("'(?P<value>{})", *STANDARD_PRINCIPAL_REGEX),
            TokenType::PrincipalLiteral,
        ),
        LexMatcher::new(
            &format!("(?P<value>{})", *CLARITY_NAME_REGEX),
            TokenType::Variable,
        ),
    ];

    let mut context = LexContext::ExpectNothing;

    let mut line_indices = get_lines_at(input);
    let mut next_line_break = line_indices.pop();
    let mut current_line: u32 = 1;

    let mut result = Vec::new();
    let mut munch_index = 0;
    let mut column_pos: u32 = 1;
    let mut did_match = true;
    while did_match && munch_index < input.len() {
        if let Some(next_line_ix) = next_line_break {
            if munch_index > next_line_ix {
                next_line_break = line_indices.pop();
                column_pos = 1;
                current_line = current_line
                    .checked_add(1)
                    .ok_or(ParseError::new(ParseErrors::ProgramTooLarge))?;
            }
        }

        did_match = false;
        let current_slice = &input[munch_index..];
        for matcher in lex_matchers.iter() {
            if let Some(captures) = matcher.matcher.captures(current_slice) {
                let whole_match = captures.get(0).unwrap();
                assert_eq!(whole_match.start(), 0);
                munch_index += whole_match.end();

                match context {
                    LexContext::ExpectNothing => Ok(()),
                    LexContext::ExpectClosing => {
                        // expect the next lexed item to be something that typically
                        // "closes" an atom -- i.e., whitespace or a right-parens.
                        // this prevents an atom like 1234abc from getting split into "1234" and "abc"
                        match matcher.handler {
                            TokenType::RParens => Ok(()),
                            TokenType::RCurly => Ok(()),
                            TokenType::Whitespace => Ok(()),
                            TokenType::Comma => Ok(()),
                            TokenType::Colon => Ok(()),
                            _ => Err(ParseError::new(ParseErrors::SeparatorExpected(
                                current_slice[..whole_match.end()].to_string(),
                            ))),
                        }
                    }
                    LexContext::ExpectClosingColon => {
                        // handle the expected whitespace after a `:`
                        match matcher.handler {
                            TokenType::RParens => Ok(()),
                            TokenType::RCurly => Ok(()),
                            TokenType::Whitespace => Ok(()),
                            TokenType::Comma => Ok(()),
                            TokenType::Colon => Ok(()),
                            _ => Err(ParseError::new(ParseErrors::SeparatorExpectedAfterColon(
                                current_slice[..whole_match.end()].to_string(),
                            ))),
                        }
                    }
                }?;

                // default to expect a closing
                context = LexContext::ExpectClosing;

                let token = match matcher.handler {
                    TokenType::LParens => {
                        context = LexContext::ExpectNothing;
                        Ok(LexItem::LeftParen)
                    }
                    TokenType::RParens => Ok(LexItem::RightParen),
                    TokenType::Whitespace => {
                        context = LexContext::ExpectNothing;
                        Ok(LexItem::Whitespace)
                    }
                    TokenType::Comma => {
                        context = LexContext::ExpectNothing;
                        Ok(LexItem::CommaSeparator)
                    }
                    TokenType::Colon => {
                        // colon should not be followed directly by an item,
                        //  e.g., {a:b} should not be legal
                        context = LexContext::ExpectClosingColon;
                        Ok(LexItem::ColonSeparator)
                    }
                    TokenType::LCurly => {
                        context = LexContext::ExpectNothing;
                        Ok(LexItem::LeftCurly)
                    }
                    TokenType::RCurly => Ok(LexItem::RightCurly),
                    TokenType::Variable => {
                        let value = get_value_or_err(current_slice, captures)?;
                        if value.contains("#") {
                            Err(ParseError::new(ParseErrors::IllegalVariableName(value)))
                        } else {
                            Ok(LexItem::Variable(value))
                        }
                    }
                    TokenType::UIntLiteral => {
                        let str_value = get_value_or_err(current_slice, captures)?;
                        let value = match u128::from_str_radix(&str_value, 10) {
                            Ok(parsed) => Ok(Value::UInt(parsed)),
                            Err(_e) => Err(ParseError::new(ParseErrors::FailedParsingIntValue(
                                str_value.clone(),
                            ))),
                        }?;
                        Ok(LexItem::LiteralValue(str_value.len(), value))
                    }
                    TokenType::IntLiteral => {
                        let str_value = get_value_or_err(current_slice, captures)?;
                        let value = match i128::from_str_radix(&str_value, 10) {
                            Ok(parsed) => Ok(Value::Int(parsed)),
                            Err(_e) => Err(ParseError::new(ParseErrors::FailedParsingIntValue(
                                str_value.clone(),
                            ))),
                        }?;
                        Ok(LexItem::LiteralValue(str_value.len(), value))
                    }
                    TokenType::FullyQualifiedContractIdentifierLiteral => {
                        let str_value = get_value_or_err(current_slice, captures)?;
                        let value =
                            match PrincipalData::parse_qualified_contract_principal(&str_value) {
                                Ok(parsed) => Ok(Value::Principal(parsed)),
                                Err(_e) => Err(ParseError::new(
                                    ParseErrors::FailedParsingPrincipal(str_value.clone()),
                                )),
                            }?;
                        Ok(LexItem::LiteralValue(str_value.len(), value))
                    }
                    TokenType::SugaredContractIdentifierLiteral => {
                        let str_value = get_value_or_err(current_slice, captures)?;
                        let value = match str_value[1..].to_string().try_into() {
                            Ok(parsed) => Ok(parsed),
                            Err(_e) => Err(ParseError::new(ParseErrors::FailedParsingPrincipal(
                                str_value.clone(),
                            ))),
                        }?;
                        Ok(LexItem::SugaredContractIdentifier(str_value.len(), value))
                    }
                    TokenType::FullyQualifiedFieldIdentifierLiteral => {
                        let str_value = get_value_or_err(current_slice, captures)?;
                        let value = match TraitIdentifier::parse_fully_qualified(&str_value) {
                            Ok(parsed) => Ok(parsed),
                            Err(_e) => Err(ParseError::new(ParseErrors::FailedParsingField(
                                str_value.clone(),
                            ))),
                        }?;
                        Ok(LexItem::FieldIdentifier(str_value.len(), value))
                    }
                    TokenType::SugaredFieldIdentifierLiteral => {
                        let str_value = get_value_or_err(current_slice, captures)?;
                        let (contract_name, field_name) =
                            match TraitIdentifier::parse_sugared_syntax(&str_value) {
                                Ok((contract_name, field_name)) => Ok((contract_name, field_name)),
                                Err(_e) => Err(ParseError::new(ParseErrors::FailedParsingField(
                                    str_value.clone(),
                                ))),
                            }?;
                        Ok(LexItem::SugaredFieldIdentifier(
                            str_value.len(),
                            contract_name,
                            field_name,
                        ))
                    }
                    TokenType::PrincipalLiteral => {
                        let str_value = get_value_or_err(current_slice, captures)?;
                        let value = match PrincipalData::parse_standard_principal(&str_value) {
                            Ok(parsed) => Ok(Value::Principal(PrincipalData::Standard(parsed))),
                            Err(_e) => Err(ParseError::new(ParseErrors::FailedParsingPrincipal(
                                str_value.clone(),
                            ))),
                        }?;
                        Ok(LexItem::LiteralValue(str_value.len(), value))
                    }
                    TokenType::TraitReferenceLiteral => {
                        let str_value = get_value_or_err(current_slice, captures)?;
                        let data = str_value.clone().try_into().map_err(|_| {
                            ParseError::new(ParseErrors::IllegalVariableName(str_value.to_string()))
                        })?;
                        Ok(LexItem::TraitReference(str_value.len(), data))
                    }
                    TokenType::HexStringLiteral => {
                        let str_value = get_value_or_err(current_slice, captures)?;
                        let byte_vec = hex_bytes(&str_value).map_err(|x| {
                            ParseError::new(ParseErrors::FailedParsingHexValue(
                                str_value.clone(),
                                x.to_string(),
                            ))
                        })?;
                        let value = match Value::buff_from(byte_vec) {
                            Ok(parsed) => Ok(parsed),
                            Err(_e) => Err(ParseError::new(ParseErrors::FailedParsingBuffer(
                                str_value.clone(),
                            ))),
                        }?;
                        Ok(LexItem::LiteralValue(str_value.len(), value))
                    }
                    TokenType::StringASCIILiteral => {
                        let str_value = get_value_or_err(current_slice, captures)?;
                        let str_value_len = str_value.len();
                        let unescaped_str = unescape_ascii_chars(str_value, false)?;
                        let byte_vec = unescaped_str.as_bytes().to_vec();

                        let value = match Value::string_ascii_from_bytes(byte_vec) {
                            Ok(parsed) => Ok(parsed),
                            Err(_e) => Err(ParseError::new(ParseErrors::InvalidCharactersDetected)),
                        }?;
                        Ok(LexItem::LiteralValue(str_value_len, value))
                    }
                    TokenType::StringUTF8Literal => {
                        let str_value = get_value_or_err(current_slice, captures)?;
                        let str_value_len = str_value.len();
                        let unescaped_str = unescape_ascii_chars(str_value, true)?;

                        let value = match Value::string_utf8_from_string_utf8_literal(unescaped_str)
                        {
                            Ok(parsed) => Ok(parsed),
                            Err(_e) => Err(ParseError::new(ParseErrors::InvalidCharactersDetected)),
                        }?;
                        Ok(LexItem::LiteralValue(str_value_len, value))
                    }
                }?;

                result.push((token, current_line, column_pos));
                column_pos += whole_match.end() as u32;
                did_match = true;
                break;
            }
        }
    }

    if munch_index == input.len() {
        Ok(result)
    } else {
        Err(ParseError::new(ParseErrors::FailedParsingRemainder(
            input[munch_index..].to_string(),
        )))
    }
}

fn unescape_ascii_chars(escaped_str: String, allow_unicode_escape: bool) -> ParseResult<String> {
    let mut unescaped_str = String::new();
    let mut chars = escaped_str.chars().into_iter();
    while let Some(char) = chars.next() {
        if char == '\\' {
            if let Some(next) = chars.next() {
                match next {
                    // ASCII escapes based on Rust list (https://doc.rust-lang.org/reference/tokens.html#ascii-escapes)
                    '\\' => unescaped_str.push('\\'),
                    '\"' => unescaped_str.push('\"'),
                    'n' => unescaped_str.push('\n'),
                    't' => unescaped_str.push('\t'),
                    'r' => unescaped_str.push('\r'),
                    '0' => unescaped_str.push('\0'),
                    'u' if allow_unicode_escape == true => unescaped_str.push_str("\\u"),
                    _ => return Err(ParseError::new(ParseErrors::InvalidEscaping)),
                }
            } else {
                return Err(ParseError::new(ParseErrors::InvalidEscaping));
            }
        } else {
            unescaped_str.push(char);
        }
    }
    Ok(unescaped_str)
}

enum ParseStackItem {
    Expression(PreSymbolicExpression),
    Colon,
    Comma,
}

fn handle_expression(
    parse_stack: &mut Vec<(Vec<ParseStackItem>, u32, u32, ParseContext)>,
    outputs: &mut Vec<PreSymbolicExpression>,
    expr: PreSymbolicExpression,
) {
    match parse_stack.last_mut() {
        // no open lists on stack, add current to outputs.
        None => outputs.push(expr),
        // there's an open list or tuple on the stack.
        Some((ref mut list, _, _, _)) => list.push(ParseStackItem::Expression(expr)),
    }
}

pub fn parse_lexed(mut input: Vec<(LexItem, u32, u32)>) -> ParseResult<Vec<PreSymbolicExpression>> {
    let mut parse_stack = Vec::new();

    let mut output_list = Vec::new();

    for (item, line_pos, column_pos) in input.drain(..) {
        match item {
            LexItem::LeftParen => {
                // start new list.
                let new_list = Vec::new();
                parse_stack.push((new_list, line_pos, column_pos, ParseContext::CollectList));
            }
            LexItem::RightParen => {
                // end current list.
                if let Some((list, start_line, start_column, parse_context)) = parse_stack.pop() {
                    match parse_context {
                        ParseContext::CollectList => {
                            let checked_list: ParseResult<Box<[PreSymbolicExpression]>> = list
                                .into_iter()
                                .map(|i| match i {
                                    ParseStackItem::Expression(e) => Ok(e),
                                    ParseStackItem::Colon => {
                                        Err(ParseError::new(ParseErrors::ColonSeparatorUnexpected))
                                    }
                                    ParseStackItem::Comma => {
                                        Err(ParseError::new(ParseErrors::CommaSeparatorUnexpected))
                                    }
                                })
                                .collect();
                            let checked_list = checked_list?;
                            let mut pre_expr = PreSymbolicExpression::list(checked_list);
                            pre_expr.set_span(start_line, start_column, line_pos, column_pos);
                            handle_expression(&mut parse_stack, &mut output_list, pre_expr);
                        }
                        ParseContext::CollectTuple => {
                            let mut error =
                                ParseError::new(ParseErrors::ClosingTupleLiteralExpected);
                            error.diagnostic.add_span(
                                start_line,
                                start_column,
                                line_pos,
                                column_pos,
                            );
                            return Err(error);
                        }
                    }
                } else {
                    return Err(ParseError::new(ParseErrors::ClosingParenthesisUnexpected));
                }
            }
            LexItem::LeftCurly => {
                let new_list = Vec::new();
                parse_stack.push((new_list, line_pos, column_pos, ParseContext::CollectTuple));
            }
            LexItem::RightCurly => {
                if let Some((tuple_list, start_line, start_column, parse_context)) =
                    parse_stack.pop()
                {
                    match parse_context {
                        ParseContext::CollectTuple => {
                            let mut checked_list = Vec::new();
                            for (index, item) in tuple_list.into_iter().enumerate() {
                                // check that tuple items are (expr, colon, expr, comma)
                                match index % 4 {
                                    0 | 2 => {
                                        if let ParseStackItem::Expression(e) = item {
                                            checked_list.push(e);
                                            Ok(())
                                        } else {
                                            Err(ParseErrors::TupleItemExpected(index))
                                        }
                                    }
                                    1 => {
                                        if let ParseStackItem::Colon = item {
                                            Ok(())
                                        } else {
                                            Err(ParseErrors::TupleColonExpected(index))
                                        }
                                    }
                                    3 => {
                                        if let ParseStackItem::Comma = item {
                                            Ok(())
                                        } else {
                                            Err(ParseErrors::TupleCommaExpected(index))
                                        }
                                    }
                                    _ => unreachable!("More than four modulos of four."),
                                }?;
                            }
                            let mut pre_expr =
                                PreSymbolicExpression::tuple(checked_list.into_boxed_slice());
                            pre_expr.set_span(start_line, start_column, line_pos, column_pos);
                            handle_expression(&mut parse_stack, &mut output_list, pre_expr);
                        }
                        ParseContext::CollectList => {
                            let mut error =
                                ParseError::new(ParseErrors::ClosingParenthesisExpected);
                            error.diagnostic.add_span(
                                start_line,
                                start_column,
                                line_pos,
                                column_pos,
                            );
                            return Err(error);
                        }
                    }
                } else {
                    return Err(ParseError::new(ParseErrors::ClosingTupleLiteralUnexpected));
                }
            }
            LexItem::Variable(value) => {
                let end_column = column_pos + (value.len() as u32) - 1;
                let value = value.clone().try_into().map_err(|_| {
                    ParseError::new(ParseErrors::IllegalVariableName(value.to_string()))
                })?;
                let mut pre_expr = PreSymbolicExpression::atom(value);
                pre_expr.set_span(line_pos, column_pos, line_pos, end_column);
                handle_expression(&mut parse_stack, &mut output_list, pre_expr);
            }
            LexItem::LiteralValue(length, value) => {
                let mut end_column = column_pos + (length as u32);
                // Avoid underflows on cases like empty strings
                if length > 0 {
                    end_column = end_column - 1;
                }
                let mut pre_expr = PreSymbolicExpression::atom_value(value);
                pre_expr.set_span(line_pos, column_pos, line_pos, end_column);
                handle_expression(&mut parse_stack, &mut output_list, pre_expr);
            }
            LexItem::SugaredContractIdentifier(length, value) => {
                let mut end_column = column_pos + (length as u32);
                // Avoid underflows on cases like empty strings
                if length > 0 {
                    end_column = end_column - 1;
                }
                let mut pre_expr = PreSymbolicExpression::sugared_contract_identifier(value);
                pre_expr.set_span(line_pos, column_pos, line_pos, end_column);
                handle_expression(&mut parse_stack, &mut output_list, pre_expr);
            }
            LexItem::SugaredFieldIdentifier(length, contract_name, name) => {
                let mut end_column = column_pos + (length as u32);
                // Avoid underflows on cases like empty strings
                if length > 0 {
                    end_column = end_column - 1;
                }
                let mut pre_expr =
                    PreSymbolicExpression::sugared_field_identifier(contract_name, name);
                pre_expr.set_span(line_pos, column_pos, line_pos, end_column);
                handle_expression(&mut parse_stack, &mut output_list, pre_expr);
            }
            LexItem::FieldIdentifier(length, trait_identifier) => {
                let mut end_column = column_pos + (length as u32);
                // Avoid underflows on cases like empty strings
                if length > 0 {
                    end_column = end_column - 1;
                }
                let mut pre_expr = PreSymbolicExpression::field_identifier(trait_identifier);
                pre_expr.set_span(line_pos, column_pos, line_pos, end_column);
                handle_expression(&mut parse_stack, &mut output_list, pre_expr);
            }
            LexItem::TraitReference(_length, value) => {
                let end_column = column_pos + (value.len() as u32) - 1;
                let value = value.clone().try_into().map_err(|_| {
                    ParseError::new(ParseErrors::IllegalVariableName(value.to_string()))
                })?;
                let mut pre_expr = PreSymbolicExpression::trait_reference(value);
                pre_expr.set_span(line_pos, column_pos, line_pos, end_column);
                handle_expression(&mut parse_stack, &mut output_list, pre_expr);
            }
            LexItem::ColonSeparator => {
                match parse_stack.last_mut() {
                    None => return Err(ParseError::new(ParseErrors::ColonSeparatorUnexpected)),
                    Some((ref mut list, ..)) => {
                        list.push(ParseStackItem::Colon);
                    }
                };
            }
            LexItem::CommaSeparator => {
                match parse_stack.last_mut() {
                    None => return Err(ParseError::new(ParseErrors::CommaSeparatorUnexpected)),
                    Some((ref mut list, ..)) => {
                        list.push(ParseStackItem::Comma);
                    }
                };
            }
            LexItem::Whitespace => (),
        };
    }

    // check unfinished stack:
    if parse_stack.len() > 0 {
        let mut error = ParseError::new(ParseErrors::ClosingParenthesisExpected);
        if let Some((_list, start_line, start_column, _parse_context)) = parse_stack.pop() {
            error.diagnostic.add_span(start_line, start_column, 0, 0);
        }
        Err(error)
    } else {
        Ok(output_list)
    }
}

pub fn parse(input: &str) -> ParseResult<Vec<PreSymbolicExpression>> {
    let lexed = lex(input)?;
    parse_lexed(lexed)
}
