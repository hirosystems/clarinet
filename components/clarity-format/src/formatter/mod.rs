use clarity_repl::clarity::ast::build_ast_with_rules;
use clarity_repl::clarity::stacks_common::types::StacksEpochId;
use clarity_repl::clarity::vm::types::QualifiedContractIdentifier;
use clarity_repl::clarity::ClarityVersion;

mod comments;
mod formatters;

use self::comments::{attach_comments_to_ast, extract_comments};
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
    _settings: Settings,
}

impl ClarityFormatter {
    pub fn new(settings: Settings) -> Self {
        Self {
            _settings: settings,
        }
    }

    pub fn format(&mut self, source: &str) -> String {
        let ast = build_ast_with_rules(
            &QualifiedContractIdentifier::transient(),
            source,
            &mut (),
            ClarityVersion::Clarity2,
            StacksEpochId::Epoch21,
            clarity_repl::clarity::ast::ASTRules::Typical,
        )
        .unwrap();
        let comments = extract_comments(source);
        let _attached_comments = attach_comments_to_ast(&comments, &ast.expressions);

        /*
            @todo
            format() should return a vec with one item by line
            Vec<{
                content: String,
                start_expr_id: Option<u64>,
                end_expr_id: Option<u64>,
            }<
        */
        let output = format(&ast.expressions, "");
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
