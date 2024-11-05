use clarity_repl::clarity::ast::build_ast_with_rules;
use clarity_repl::clarity::vm::types::QualifiedContractIdentifier;
use clarity_repl::clarity::ClarityVersion;
use clarity_repl::clarity::StacksEpochId;
mod formatters;
use self::formatters::format;
//
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
pub struct ClarityFormatter {
    settings: Settings,
}
impl ClarityFormatter {
    pub fn new(settings: Settings) -> Self {
        Self { settings: settings }
    }
    pub fn format(&mut self, file_path: &str, source: &str) -> String {
        let ast = build_ast_with_rules(
            &QualifiedContractIdentifier::transient(),
            source,
            &mut (),
            ClarityVersion::Clarity3,
            StacksEpochId::Epoch30,
            clarity_repl::clarity::ast::ASTRules::Typical,
        )
        .unwrap();
        let output = format(&ast.expressions, "");
        println!("output: {}", output);
        // @todo mut output and reinject comments based on start and end expr_ids
        output
    }
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
}
