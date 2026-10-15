use clarinet_defaults::DEFAULT_EPOCH;
use clarinet_format::formatter::{ClarityFormatter, Settings};
use clarity::vm::ast::parser::v2::parse;
use clarity::vm::ast::stack_depth_checker::StackDepthLimits;

fn assert_format(source: &str, expected: &str) {
    let formatter = ClarityFormatter::new(Settings::default());
    let parse = |source: &str| parse(source, StackDepthLimits::for_epoch(DEFAULT_EPOCH)).unwrap();
    let ast = parse(source);
    for (mode, formatted) in [
        ("file", formatter.format_file(source, None)),
        ("section", formatter.format_section(source, None).unwrap()),
        ("ast", formatter.format_ast(&ast)),
    ] {
        assert_eq!(formatted.trim_end(), expected, "{mode}: {source}");
        let reparsed = parse(&formatted);
        let repeated = match mode {
            "file" => formatter.format_file(&formatted, None),
            "section" => formatter.format_section(&formatted, None).unwrap(),
            _ => formatter.format_ast(&reparsed),
        };
        assert_eq!(repeated, formatted, "{mode}: {source}");
    }
}

#[test]
fn definition_comments_do_not_swallow_arguments() {
    for (source, expected) in [
        (
            "(define-constant x ;; c\n u1)",
            "(define-constant\n  x ;; c\n  u1\n)",
        ),
        (
            "(define-data-var v (list 10 uint)\n ;; c\n (list))",
            "(define-data-var\n  v\n  (list 10 uint)\n  ;; c\n  (list)\n)",
        ),
        (
            "(define-fungible-token token ;; c\n u10)",
            "(define-fungible-token\n  token ;; c\n  u10\n)",
        ),
        (
            "(define-non-fungible-token token\n ;; c\n uint)",
            "(define-non-fungible-token\n  token\n  ;; c\n  uint\n)",
        ),
        (
            "(define-constant x u1 ;; end\n)",
            "(define-constant\n  x\n  u1 ;; end\n)",
        ),
        (
            "(define-constant ;; first\n x\n ;; second\n ;; third\n u1)",
            "(define-constant\n  ;; first\n  x\n  ;; second\n  ;; third\n  u1\n)",
        ),
    ] {
        assert_format(source, expected);
    }
}

#[test]
fn sole_comment_argument_keeps_closing_parenthesis() {
    for source in ["(foo\n ;; c\n)", "(foo ;; c\n)", "(ok ;; c\n)"] {
        let name = if source.starts_with("(ok") {
            "ok"
        } else {
            "foo"
        };
        let expected = if source.contains("\n ;;") {
            format!("({name}\n  ;; c\n)")
        } else {
            format!("({name} ;; c\n)")
        };
        assert_format(source, &expected);
    }
    assert_format("(foo u1)", "(foo u1)");
}
