use clarity_repl::clarity::{representations::Span, ClarityName, SymbolicExpression};
use lsp_types::{Position, Range};

#[cfg(feature = "wasm")]
#[allow(unused_imports)]
use crate::utils::log;

pub fn span_to_range(span: &Span) -> Range {
    Range::new(
        Position::new(span.start_line - 1, span.start_column - 1),
        Position::new(span.end_line - 1, span.end_column),
    )
}

pub fn is_position_within_span(position: &Position, span: &Span) -> bool {
    if position.line < span.start_line || position.line > span.end_line {
        return false;
    }
    if position.line == span.start_line && position.character < span.start_column {
        return false;
    }
    if position.line == span.end_line && position.character > span.end_column {
        return false;
    }

    return true;
}

pub fn get_expression_name_at_position(
    position: &Position,
    expressions: &Vec<SymbolicExpression>,
) -> Option<ClarityName> {
    for expr in expressions {
        if is_position_within_span(position, &expr.span) {
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
        if is_position_within_span(position, &expr.span) {
            if let Some(_function_name) = expr.match_atom() {
                return Some((expr.span.start_line, expr.span.start_column));
            } else if let Some(expressions) = expr.match_list() {
                return get_atom_start_at_position(position, &expressions.to_vec());
            }
        }
    }
    None
}
