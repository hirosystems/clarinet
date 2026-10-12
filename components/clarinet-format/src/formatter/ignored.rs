use clarity::vm::representations::{PreSymbolicExpression, Span};

fn u32_to_usize(val: u32) -> usize {
    usize::try_from(val).unwrap()
}

/// spans use 1-based index, this const is largely only for this comment
const COLUMN_TO_INDEX_OFFSET: usize = 1;

/// Extract the original source text using its span
pub fn extract_expr_source(expr: &PreSymbolicExpression, source: &str) -> String {
    extract_source_range(source, expr.span())
}

pub fn extract_source_range(source: &str, span: &Span) -> String {
    SourceLines::new(source).extract_source_range(span)
}

pub(crate) struct SourceLines<'a> {
    pub(crate) lines: Vec<&'a str>,
}

impl<'a> SourceLines<'a> {
    pub(crate) fn new(source: &'a str) -> Self {
        Self {
            lines: source.lines().collect(),
        }
    }

    pub(crate) fn extract_expr_source(&self, expr: &PreSymbolicExpression) -> String {
        self.extract_source_range(expr.span())
    }

    pub(crate) fn extract_source_range(&self, span: &Span) -> String {
        let start_line = u32_to_usize(span.start_line);
        let end_line = u32_to_usize(span.end_line);
        let start_column = u32_to_usize(span.start_column);
        let end_column = u32_to_usize(span.end_column);

        let lines = &self.lines;
        let mut result = String::new();

        for line_idx in start_line..=end_line {
            if line_idx > lines.len() {
                break;
            }

            let line = lines[line_idx - COLUMN_TO_INDEX_OFFSET];
            let is_first_line = line_idx == start_line;
            let is_last_line = line_idx == end_line;

            if is_first_line && is_last_line {
                let start_col = start_column.saturating_sub(COLUMN_TO_INDEX_OFFSET);
                let end_col = end_column.min(line.len());
                if start_col < end_col {
                    result.push_str(&line[start_col..end_col]);
                }
            } else if is_first_line {
                let start_col = start_column.saturating_sub(COLUMN_TO_INDEX_OFFSET);
                if start_col < line.len() {
                    result.push_str(&line[start_col..]);
                }
                if !is_last_line {
                    result.push('\n');
                }
            } else if is_last_line {
                let end_col = end_column.min(line.len());
                if end_col > 0 {
                    result.push_str(&line[..end_col]);
                }
            } else {
                // Middle lines
                result.push_str(line);
                result.push('\n');
            }
        }

        result
    }
}

pub fn ignored_exprs(exprs: &[PreSymbolicExpression], source: &str) -> String {
    let start = exprs.first().unwrap().span();
    let end = exprs.last().unwrap().span();
    extract_source_range(
        source,
        &Span {
            start_line: start.start_line,
            start_column: start.start_column,
            end_line: end.end_line,
            end_column: end.end_column,
        },
    )
}
