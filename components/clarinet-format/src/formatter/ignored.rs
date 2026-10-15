use clarity::vm::representations::{PreSymbolicExpression, Span};

fn u32_to_usize(val: u32) -> usize {
    usize::try_from(val).unwrap()
}

/// spans use 1-based index, this const is largely only for this comment
const COLUMN_TO_INDEX_OFFSET: usize = 1;

/// Source split into lines once, so extracting many spans doesn't rescan the source
pub struct SourceLines<'a> {
    lines: Vec<&'a str>,
}

impl<'a> SourceLines<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            lines: source.lines().collect(),
        }
    }

    /// Extract the original source text using its span
    pub(crate) fn extract_expr_source(&self, expr: &PreSymbolicExpression) -> String {
        self.extract_source_range(expr.span())
    }

    pub fn extract_source_range(&self, span: &Span) -> String {
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

            let start_col = if is_first_line {
                start_column.saturating_sub(COLUMN_TO_INDEX_OFFSET)
            } else {
                0
            };
            let end_col = if is_last_line {
                end_column.min(line.len())
            } else {
                line.len()
            };
            if start_col < end_col {
                result.push_str(&line[start_col..end_col]);
            }
            if !is_last_line {
                result.push('\n');
            }
        }

        result
    }
}
