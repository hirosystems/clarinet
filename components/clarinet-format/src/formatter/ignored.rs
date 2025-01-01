use clarity::vm::representations::{PreSymbolicExpression, PreSymbolicExpressionType};

use crate::formatter::helpers::t;

pub fn ignored_exprs(expr: &PreSymbolicExpression) -> String {
    let mut output = String::new();
    let mut current_line = 1;

    let start_line = expr.span().start_line as usize;
    let end_line = expr.span().end_line as usize;
    let start_col = expr.span().start_column as usize;
    let end_col = expr.span().end_column as usize;

    // Add newlines if needed to reach the start line
    while current_line < start_line {
        output.push('\n');
        current_line += 1;
    }

    // Handle single-line expressions
    if start_line == end_line {
        // Add padding spaces before the expression
        output.extend(std::iter::repeat(' ').take(start_col - 1));
        output.push_str(&display_pse_unformatted(expr));
    } else {
        // Handle multi-line expressions
        let expr_str = display_pse_unformatted(expr);
        let lines: Vec<&str> = expr_str.lines().collect();

        // Print first line with proper indentation
        output.extend(std::iter::repeat(' ').take(start_col - 1));
        output.push_str(lines[0]);
        output.push('\n');
        current_line += 1;

        // Print middle lines
        for line in &lines[1..lines.len() - 1] {
            output.push_str(line);
            output.push('\n');
            current_line += 1;
        }

        // Print last line
        if let Some(last_line) = lines.last() {
            output.extend(std::iter::repeat(' ').take(end_col - last_line.len()));
            output.push_str(last_line);
        }
    }

    output
}

fn display_pse_unformatted(pse: &PreSymbolicExpression) -> String {
    match pse.pre_expr {
        PreSymbolicExpressionType::Atom(ref value) => t(value.as_str()).to_string(),
        PreSymbolicExpressionType::AtomValue(ref value) => value.to_string(),
        PreSymbolicExpressionType::List(ref items) => {
            format!("{:?}", items)
        }
        PreSymbolicExpressionType::Tuple(ref items) => {
            format!("{:?}", items)
        }
        PreSymbolicExpressionType::SugaredContractIdentifier(ref name) => {
            format!(".{}", name)
        }
        PreSymbolicExpressionType::SugaredFieldIdentifier(ref contract, ref field) => {
            format!(".{}.{}", contract, field)
        }
        PreSymbolicExpressionType::FieldIdentifier(ref trait_id) => {
            format!("'{}", trait_id)
        }
        PreSymbolicExpressionType::TraitReference(ref name) => {
            println!("trait ref: {}", name);
            name.to_string()
        }
        PreSymbolicExpressionType::Comment(ref text) => {
            format!(";; {}", t(text))
        }
        PreSymbolicExpressionType::Placeholder(ref placeholder) => {
            placeholder.to_string() // Placeholder is for if parsing fails
        }
    }
}
