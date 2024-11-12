use clarity_repl::clarity::ast::build_ast_with_rules;
use clarity_repl::clarity::vm::functions::{define::DefineFunctions, NativeFunctions};
use clarity_repl::clarity::vm::types::QualifiedContractIdentifier;
use clarity_repl::clarity::ClarityVersion;
use clarity_repl::clarity::StacksEpochId;
use clarity_repl::clarity::SymbolicExpression;

pub enum Indentation {
    Space(u8),
    Tab,
}

pub struct Settings {
    pub indentation: Indentation,
    pub max_line_length: u8,
}
impl Settings {
    pub fn default() -> Settings {
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
            clarity_repl::clarity::ast::ASTRules::Typical,
        )
        .unwrap();
        let output = format_source_exprs(&self.settings, &ast.expressions, "");
        println!("output: {}", output);
        output
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
                format!("({})", format_source_exprs(settings, list, acc))
            };
            return format!(
                "{formatted} {}",
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

fn format_begin(settings: &Settings, exprs: &[SymbolicExpression]) -> String {
    let mut begin_acc = "(begin\n".to_string();
    for arg in exprs.get(1..).unwrap_or_default() {
        if let Some(list) = arg.match_list() {
            begin_acc.push_str(&format!(
                "\n  ({})",
                format_source_exprs(settings, list, "")
            ))
        }
    }
    begin_acc.push_str("\n)\n");
    begin_acc.to_owned()
}

fn format_let(settings: &Settings, exprs: &[SymbolicExpression]) -> String {
    let mut begin_acc = "(let (\n".to_string();
    for arg in exprs.get(1..).unwrap_or_default() {
        if let Some(list) = arg.match_list() {
            begin_acc.push_str(&format!(
                "\n  ({})",
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
    let name_and_args = exprs.get(1).and_then(|f| f.match_list()).unwrap();
    let mut func_acc = format!(
        "({func_type} ({})",
        format_source_exprs(settings, name_and_args, "")
    );
    for arg in exprs.get(2..).unwrap_or_default() {
        if let Some(list) = arg.match_list() {
            func_acc.push_str(&format!(
                "\n  ({})",
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
    fn format_with_default(source: &str) -> String {
        let mut formatter = ClarityFormatter::new(Settings::default());
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
    fn test_function_formatter() {
        let result = format_with_default(&String::from("(define-private (my-func) (ok true))"));
        assert_eq!(result, "(define-private (my-func)\n  (ok true)\n)");
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
    fn test_function_args_multiline() {
        let src = "(define-public (my-func (amount uint) (sender principal)) (ok true))";
        let result = format_with_default(&String::from(src));
        assert_eq!(
            result,
            "(define-public (my-func\n    (amount uint)\n    (sender principal)\n  )\n  (ok true)\n)"
        );
    }
    #[test]
    fn test_begin_never_one_line() {
        let src = "(begin (ok true))";
        let result = format_with_default(&String::from(src));
        assert_eq!(result, "(begin\n  (ok true)\n)");
    }
}
