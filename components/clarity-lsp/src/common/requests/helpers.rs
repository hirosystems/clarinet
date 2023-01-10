use std::collections::HashMap;

use clarity_repl::clarity::{
    functions::define::DefineFunctions, representations::Span, ClarityName, SymbolicExpression,
};
use lsp_types::{Position, Range};

#[cfg(feature = "wasm")]
#[allow(unused_imports)]
use crate::utils::log;

pub fn span_to_range(span: &Span) -> Range {
    if span == &Span::zero() {
        return Range::default();
    }

    {
        Range::new(
            Position::new(span.start_line - 1, span.start_column - 1),
            Position::new(span.end_line - 1, span.end_column),
        )
    }
}

// end_offset is usded to include the end position of a keyword, for go to definition in particular
pub fn is_position_within_span(position: &Position, span: &Span, end_offset: u32) -> bool {
    if position.line < span.start_line || position.line > span.end_line {
        return false;
    }
    if position.line == span.start_line && position.character < span.start_column {
        return false;
    }
    if position.line == span.end_line && position.character > span.end_column + end_offset {
        return false;
    }

    return true;
}

pub fn get_expression_name_at_position(
    position: &Position,
    expressions: &Vec<SymbolicExpression>,
) -> Option<ClarityName> {
    for expr in expressions {
        if is_position_within_span(position, &expr.span, 0) {
            if let Some(function_name) = expr.match_atom() {
                return Some(function_name.to_owned());
            } else if let Some(expressions) = expr.match_list() {
                return get_expression_name_at_position(position, &expressions.to_vec());
            }
        }
    }
    None
}

pub fn get_atom_start_at_position(
    position: &Position,
    expressions: &Vec<SymbolicExpression>,
) -> Option<(u32, u32)> {
    for expr in expressions {
        if is_position_within_span(position, &expr.span, 1) {
            if let Some(_function_name) = expr.match_atom() {
                return Some((expr.span.start_line, expr.span.start_column));
            } else if let Some(expressions) = expr.match_list() {
                return get_atom_start_at_position(position, &expressions.to_vec());
            }
        }
    }
    None
}

pub fn get_public_function_definitions(
    expressions: &Vec<SymbolicExpression>,
) -> Option<HashMap<ClarityName, Range>> {
    let mut definitions = HashMap::new();

    for expression in expressions {
        let (define_function, args) = expression.match_list()?.split_first()?;
        match DefineFunctions::lookup_by_name(define_function.match_atom()?)? {
            DefineFunctions::PublicFunction | DefineFunctions::ReadOnlyFunction => {
                let (args_list, _) = args.split_first()?;
                let (function_name, _) = args_list.match_list()?.split_first()?;
                definitions.insert(
                    function_name.match_atom()?.to_owned(),
                    span_to_range(&expression.span),
                );
            }
            _ => (),
        }
    }

    Some(definitions)
}
