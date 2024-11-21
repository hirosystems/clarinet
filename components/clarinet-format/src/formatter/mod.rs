use clarity::types::StacksEpochId;
use clarity::vm::ast::{build_ast_with_rules, ASTRules};
use clarity::vm::functions::{define::DefineFunctions, NativeFunctions};
use clarity::vm::types::QualifiedContractIdentifier;
use clarity::vm::{ClarityVersion, SymbolicExpression};

pub enum Indentation {
    Space(usize),
    Tab,
}

pub struct Settings {
    pub indentation: Indentation,
    pub max_line_length: usize,
}

impl Settings {
    pub fn new(indentation: Indentation, max_line_length: usize) -> Self {
        Settings {
            indentation,
            max_line_length,
        }
    }
}
impl Default for Settings {
    fn default() -> Settings {
        Settings {
            indentation: Indentation::Space(2),
            max_line_length: 80,
        }
    }
}
//
pub struct ClarityFormatter {
    settings: Settings,
}
impl ClarityFormatter {
    pub fn new(settings: Settings) -> Self {
        Self { settings }
    }
    pub fn format(&mut self, source: &str) -> String {
        let ast = build_ast_with_rules(
            &QualifiedContractIdentifier::transient(),
            source,
            &mut (),
            ClarityVersion::Clarity3,
            StacksEpochId::Epoch30,
            ASTRules::Typical,
        )
        .unwrap();
        format_source_exprs(&self.settings, &ast.expressions, "")
    }
}

// * functions

// Top level define-<function> should have a line break above and after (except on first line)
// options always on new lines
// Functions Always on multiple lines, even if short
// *begin* never on one line
// *let* never on one line

// * match *
// One line if less than max length (unless the original source has line breaks?)
// Multiple lines if more than max length (should the first arg be on the first line if it fits?)
pub fn format_source_exprs(
    settings: &Settings,
    expressions: &[SymbolicExpression],
    acc: &str,
) -> String {
    if let Some((expr, remaining)) = expressions.split_first() {
        if let Some(list) = expr.match_list() {
            let atom = list.split_first().and_then(|(f, _)| f.match_atom());
            use NativeFunctions::*;
            let formatted = if let Some(
                DefineFunctions::PublicFunction
                | DefineFunctions::ReadOnlyFunction
                | DefineFunctions::PrivateFunction,
            ) = atom.and_then(|a| DefineFunctions::lookup_by_name(a))
            {
                format_function(settings, list)
            } else if let Some(Begin) = atom.and_then(|a| NativeFunctions::lookup_by_name(a)) {
                format_begin(settings, list)
            } else if let Some(Let) = atom.and_then(|a| NativeFunctions::lookup_by_name(a)) {
                format_let(settings, list)
            } else if let Some(TupleCons) = atom.and_then(|a| NativeFunctions::lookup_by_name(a)) {
                format_tuple(settings, list)
            } else {
                format!("({})\n", format_source_exprs(settings, list, acc))
            };
            let pre_comments = format_comments(&expr.pre_comments, settings.max_line_length);
            let post_comments = format_comments(&expr.post_comments, settings.max_line_length);
            let end_line_comment = if let Some(comment) = &expr.end_line_comment {
                print!("here");
                format!(" ;; {}", comment)
            } else {
                print!("there");
                String::new()
            };
            print!("{}", formatted);
            return format!(
                "{pre_comments}{formatted}{end_line_comment}{post_comments}{}",
                format_source_exprs(settings, remaining, acc)
            )
            .trim()
            .to_owned();
        }
        return format!("{} {}", expr, format_source_exprs(settings, remaining, acc))
            .trim()
            .to_owned();
    };
    acc.to_owned()
}

fn format_comments(
    comments: &[(String, clarity::vm::representations::Span)],
    max_line_length: usize,
) -> String {
    if !comments.is_empty() {
        let joined = comments
            .iter()
            .map(|(comment, span)| {
                let mut formatted = String::new();
                let mut current_line = String::new();
                let indent = " ".repeat(span.start_column as usize - 1);
                let max_content_length = max_line_length - span.start_column as usize - 3;

                for word in comment.split_whitespace() {
                    if current_line.len() + word.len() + 1 > max_content_length {
                        // push the current line and start a new one
                        formatted.push_str(&format!("{};; {}\n", indent, current_line.trim_end()));
                        current_line.clear();
                    }
                    // add a space if the current line isn't empty
                    if !current_line.is_empty() {
                        current_line.push(' ');
                    }
                    current_line.push_str(word);
                }

                // push the rest if it exists
                if !current_line.is_empty() {
                    formatted.push_str(&format!("{};; {}", indent, current_line.trim_end()));
                }

                formatted
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!("{joined}\n")
    } else {
        "".to_string()
    }
}

fn indentation_to_string(indentation: &Indentation) -> String {
    match indentation {
        Indentation::Space(i) => " ".repeat(*i),
        Indentation::Tab => "\t".to_string(),
    }
}

fn format_begin(settings: &Settings, exprs: &[SymbolicExpression]) -> String {
    let mut begin_acc = "(begin".to_string();
    let indentation = indentation_to_string(&settings.indentation);
    for arg in exprs.get(1..).unwrap_or_default() {
        if let Some(list) = arg.match_list() {
            begin_acc.push_str(&format!(
                "\n{}({})",
                indentation,
                format_source_exprs(settings, list, "")
            ))
        }
    }
    begin_acc.push_str("\n)\n");
    begin_acc.to_owned()
}

fn format_let(settings: &Settings, exprs: &[SymbolicExpression]) -> String {
    let mut begin_acc = "(let (".to_string();
    let indentation = indentation_to_string(&settings.indentation);
    for arg in exprs.get(1..).unwrap_or_default() {
        if let Some(list) = arg.match_list() {
            begin_acc.push_str(&format!(
                "\n{}({})",
                indentation,
                format_source_exprs(settings, list, "")
            ))
        }
    }
    begin_acc.push_str("\n)  \n");
    begin_acc.to_owned()
}

fn format_tuple(settings: &Settings, exprs: &[SymbolicExpression]) -> String {
    let mut tuple_acc = "{ ".to_string();
    for (i, expr) in exprs[1..].iter().enumerate() {
        let (key, value) = expr
            .match_list()
            .and_then(|list| list.split_first())
            .unwrap();
        if i < exprs.len() - 2 {
            tuple_acc.push_str(&format!(
                "{key}: {}, ",
                format_source_exprs(settings, value, "")
            ));
        } else {
            tuple_acc.push_str(&format!(
                "{key}: {}",
                format_source_exprs(settings, value, "")
            ));
        }
    }
    tuple_acc.push_str(" }");
    tuple_acc.to_string()
}

fn format_function(settings: &Settings, exprs: &[SymbolicExpression]) -> String {
    let func_type = exprs.first().unwrap();
    let indentation = indentation_to_string(&settings.indentation);
    let name_and_args = exprs.get(1).and_then(|f| f.match_list()).unwrap();

    let mut func_acc = format!("({func_type} (");

    if let Some((name, args)) = name_and_args.split_first() {
        func_acc.push_str(&format!("{}", name));
        if args.is_empty() {
            func_acc.push(')');
        } else {
            for arg in args {
                func_acc.push_str(&format!(
                    "\n{}{}{}",
                    indentation,
                    indentation,
                    format_source_exprs(settings, &[arg.clone()], "")
                ));
            }
            func_acc.push_str(&format!("\n{})", indentation));
        }
    }
    for arg in exprs.get(2..).unwrap_or_default() {
        if let Some(list) = arg.match_list() {
            func_acc.push_str(&format!(
                "\n{}({})",
                indentation,
                format_source_exprs(settings, list, "")
            ))
        }
    }
    func_acc.push_str("\n)");
    func_acc.to_owned()
}
#[cfg(test)]
mod tests_formatter {
    use super::{ClarityFormatter, Settings};
    use crate::formatter::Indentation;
    use std::fs;
    use std::path::Path;
    fn format_with_default(source: &str) -> String {
        let mut formatter = ClarityFormatter::new(Settings::default());
        formatter.format(source)
    }
    fn format_with(source: &str, settings: Settings) -> String {
        let mut formatter = ClarityFormatter::new(settings);
        formatter.format(source)
    }
    #[test]
    fn test_simplest_formatter() {
        let result = format_with_default(&String::from("(  ok    true )"));
        assert_eq!(result, "(ok true)");
    }
    #[test]
    fn test_two_expr_formatter() {
        let result = format_with_default(&String::from("(ok true)(ok true)"));
        assert_eq!(result, "(ok true)\n(ok true)");
    }
    #[test]
    fn test_tuple_formatter() {
        let result = format_with_default(&String::from("{n1:1,n2:2,n3:3}"));
        assert_eq!(result, "{ n1: 1, n2: 2, n3: 3 }");
    }
    #[test]
    fn test_function_and_tuple_formatter() {
        let src = "(define-private (my-func) (ok { n1: 1, n2: 2, n3: 3 }))";
        let result = format_with_default(&String::from(src));
        assert_eq!(
            result,
            "(define-private (my-func)\n  (ok { n1: 1, n2: 2, n3: 3 })\n)"
        );
    }

    #[test]
    fn test_function_formatter() {
        let result = format_with_default(&String::from("(define-private (my-func) (ok true))"));
        assert_eq!(result, "(define-private (my-func)\n  (ok true)\n)");
    }

    #[test]
    fn test_function_args_multiline() {
        let src = "(define-public (my-func (amount uint) (sender principal)) (ok true))";
        let result = format_with_default(&String::from(src));
        assert_eq!(
            result,
            "(define-public (my-func\n    (amount uint)\n    (sender principal)\n  )\n  (ok true)\n)"
        );
    }
    #[test]
    fn test_comments_included() {
        let src = ";; this is a comment\n(ok true)";

        let result = format_with_default(&String::from(src));
        assert_eq!(src, result);
    }
    // #[test]
    // fn test_end_of_line_comments_included() {
    //     let src = "(ok true) ;; this is a comment";

    //     let result = format_with_default(&String::from(src));
    //     assert_eq!(src, result);
    // }
    // #[test]
    // fn test_end_of_line_comments_max_line_length() {
    //     let src = "(ok true) ;; this is a comment";

    //     let result = format_with(&String::from(src), Settings::new(Indentation::Space(2), 9));
    //     let expected = ";; this is a comment\n(ok true)";
    //     assert_eq!(result, expected);
    // }
    #[test]
    fn test_comments_only() {
        let src = ";; this is a comment\n(ok true)";

        let result = format_with_default(&String::from(src));
        assert_eq!(src, result);
    }
    #[test]
    fn test_begin_never_one_line() {
        let src = "(begin (ok true))";
        let result = format_with_default(&String::from(src));
        assert_eq!(result, "(begin\n  (ok true)\n)");
    }

    #[test]
    fn test_custom_tab_setting() {
        let src = "(begin (ok true))";
        let result = format_with(&String::from(src), Settings::new(Indentation::Space(4), 80));
        assert_eq!(result, "(begin\n    (ok true)\n)");
    }

    #[test]
    fn test_max_line_length() {
        let src = ";; a comment with line length 32\n(ok true)";
        let result = format_with(&String::from(src), Settings::new(Indentation::Space(2), 32));
        let expected = ";; a comment with line length\n;; 32\n(ok true)";
        assert_eq!(result, expected);
    }
    // #[test]
    // fn test_irl_contracts() {
    //     let golden_dir = "./tests/golden";
    //     let intended_dir = "./tests/golden-intended";

    //     // Iterate over files in the golden directory
    //     for entry in fs::read_dir(golden_dir).expect("Failed to read golden directory") {
    //         let entry = entry.expect("Failed to read directory entry");
    //         let path = entry.path();

    //         if path.is_file() {
    //             let src = fs::read_to_string(&path).expect("Failed to read source file");

    //             let file_name = path.file_name().expect("Failed to get file name");
    //             let intended_path = Path::new(intended_dir).join(file_name);

    //             let intended =
    //                 fs::read_to_string(&intended_path).expect("Failed to read intended file");

    //             // Apply formatting and compare
    //             let result = format_with_default(&src);
    //             assert_eq!(result, intended, "Mismatch for file: {:?}", file_name);
    //         }
    //     }
    // }
}
