use clarity::vm::representations::PreSymbolicExpression;

pub fn ignored_exprs(exprs: &[PreSymbolicExpression], source: &str) -> String {
    let mut result = String::new();

    for (i, expr) in exprs.iter().enumerate() {
        if i > 0 {
            result.push('\n');
        }

        // Use the same approach as ignored_exprs for each expression
        let span = expr.span();
        let start_line = span.start_line as usize;
        let end_line = span.end_line as usize;

        let mut is_first = true;

        // Look at lines including one before our span starts
        for (idx, line) in source
            .lines()
            .skip(start_line - 1) // Start one line earlier
            .take(end_line - (start_line - 1) + 1)
            .enumerate()
        {
            if !is_first {
                result.push('\n');
            }

            if idx == 0 {
                // First line (the one with the opening parenthesis)
                if let Some(paren_pos) = line.find('(') {
                    result.push_str(&line[paren_pos..]);
                }
            } else if idx == end_line - (start_line - 1) {
                // Last line - up to end column
                let end_column = span.end_column as usize;
                if end_column <= line.len() {
                    result.push_str(&line[..end_column]);
                } else {
                    result.push_str(line);
                }
            } else {
                // Middle lines - complete line
                result.push_str(line);
            }

            is_first = false;
        }
    }

    result
}
