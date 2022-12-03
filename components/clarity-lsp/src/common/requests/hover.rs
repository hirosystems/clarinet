use clarity_repl::clarity::{
    docs::{make_api_reference, make_define_reference, make_keyword_reference},
    functions::{define::DefineFunctions, NativeFunctions},
    variables::NativeVariables,
    ClarityVersion, SymbolicExpression,
};
use lazy_static::lazy_static;
use std::collections::HashMap;

fn code(code: &str) -> String {
    vec!["```clarity", code.trim(), "```"].join("\n")
}

lazy_static! {
    static ref API_REF: HashMap<String, (ClarityVersion, String)> = {
        let mut api_references: HashMap<String, (ClarityVersion, String)> = HashMap::new();
         // "---" can produce h2 if placed under text
        let separator = "- - -";

        for define_function in DefineFunctions::ALL {
            let reference = make_define_reference(define_function);
            api_references.insert(
                define_function.to_string(),
                (reference.version, Vec::from([
                    &code(&reference.signature),
                    separator,
                    "**Description**",
                    &reference.description,
                    separator,
                    "**Example**",
                    &code(&reference.example),
                ])
                .join("\n")),
            );
        }

        for native_function in NativeFunctions::ALL {
            let reference = make_api_reference(native_function);
            api_references.insert(
                native_function.to_string(),
                (reference.version, Vec::from([
                    &code(&reference.signature),
                    separator,
                    "**Description**",
                    &reference.description,
                    separator,
                    "**Example**",
                    &code(&reference.example),
                    separator,
                    &format!("**Introduced in:** {}", &reference.version),
                ])
                .join("\n")),
            );
        }

        for native_keyword in NativeVariables::ALL {
            let reference = make_keyword_reference(native_keyword).unwrap();
            api_references.insert(
                native_keyword.to_string(),
                (reference.version, Vec::from([
                    "**Description**",
                    &reference.description,
                    separator,
                    "**Example**",
                    &code(&reference.example),
                    separator,
                    &format!("**Introduced in:** {}", &reference.version),
                ])
                .join("\n")),
            );
        }

        api_references
    };
}

fn get_expression_name_at_position(
    line: u32,
    column: u32,
    expressions: &Vec<SymbolicExpression>,
) -> Option<String> {
    for expr in expressions {
        let SymbolicExpression { span, .. } = expr;

        if span.start_line <= line && span.end_line >= line {
            if span.end_line > span.start_line {
                if let Some(expressions) = expr.match_list() {
                    return get_expression_name_at_position(line, column, &expressions.to_vec());
                }
                return None;
            }
            if span.start_column <= column && span.end_column >= column {
                if let Some(function_name) = expr.match_atom() {
                    return Some(function_name.to_string());
                } else if let Some(expressions) = expr.match_list() {
                    return get_expression_name_at_position(line, column, &expressions.to_vec());
                }
                return None;
            }
        }
    }
    None
}

pub fn get_expression_documentation(
    line: u32,
    column: u32,
    clarity_version: ClarityVersion,
    expressions: &Vec<SymbolicExpression>,
) -> Option<String> {
    let expression_name = get_expression_name_at_position(line, column, expressions)?;

    match API_REF.get(&expression_name.clone()) {
        Some((version, documentation)) => {
            if version <= &clarity_version {
                return Some(documentation.to_owned());
            }
            None
        }
        None => None,
    }
}
