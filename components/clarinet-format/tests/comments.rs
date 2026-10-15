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
fn last_comment_argument_keeps_closing_parenthesis() {
    for (source, expected) in [
        ("(foo\n ;; c\n)", "(foo\n  ;; c\n)"),
        ("(foo ;; c\n)", "(foo ;; c\n)"),
        ("(ok ;; c\n)", "(ok ;; c\n)"),
        ("(foo u1 ;; c\n)", "(foo u1 ;; c\n)"),
        (
            "(define-read-only (f)\n  (begin\n    (foo u1 ;; c\n    )\n    (ok u1)))",
            indoc::indoc! {"
                (define-read-only (f)
                  (begin
                    (foo u1 ;; c
                    )
                    (ok u1)
                  )
                )"},
        ),
    ] {
        assert_format(source, expected);
    }
    assert_format("(foo u1)", "(foo u1)");
}

#[test]
fn trait_method_comments_are_preserved() {
    let expected = indoc::indoc! {"
        (define-trait t (
          ;; before
          ;; also before
          (first
            (uint)
            (response uint uint)
          ) ;; trailing
          ;; between
          (second
            ()
            (response bool uint)
          )
          ;; after
        ))"};
    assert_format(expected, expected);
    let comments_only = "(define-trait t (\n  ;; only comment\n))";
    assert_format(comments_only, comments_only);
}

#[test]
fn comments_between_tuple_entries_are_preserved() {
    assert_format(
        "(tuple\n ;; before\n (a u1) ;; trailing\n ;; between\n (b u2)\n ;; after\n)",
        "{\n  ;; before\n  a: u1, ;; trailing\n  ;; between\n  b: u2,\n  ;; after\n}",
    );
    assert_format("(tuple ;; only comment\n)", "{\n  ;; only comment\n}");
}

#[test]
fn commented_tuple_entries_convert_to_sugar() {
    for (source, expected) in [
        ("(tuple (a ;; c\n u1))", "{\n  a:\n    ;; c\n    u1,\n}"),
        ("(tuple (a u1 ;; c\n))", "{\n  a: u1, ;; c\n}"),
        ("(tuple (a u1\n ;; after\n))", "{\n  a: u1,\n  ;; after\n}"),
        (
            "(tuple (a u1 ;; inner\n) ;; outer\n)",
            "{\n  a: u1, ;; inner\n  ;; outer\n}",
        ),
        (
            "(tuple (a u1) (b ;; first\n ;; second\n u2))",
            indoc::indoc! {"
                {
                  a: u1,
                  b:
                    ;; first
                    ;; second
                    u2,
                }"},
        ),
        (
            "(define-public (f)\n(ok (tuple (a ;; c\nu1))))",
            indoc::indoc! {"
                (define-public (f)
                  (ok {
                    a:
                      ;; c
                      u1,
                  })
                )"},
        ),
    ] {
        assert_format(source, expected);
    }
}

#[test]
fn sugar_comments_between_key_and_value_are_preserved() {
    assert_format(
        "{ a:\n ;; first\n ;; second\n u1 }",
        "{\n  a:\n    ;; first\n    ;; second\n    u1,\n}",
    );
}

#[test]
fn unusual_tuple_entries_keep_their_source() {
    let formatter = ClarityFormatter::new(Settings::default());
    for source in [
        "(tuple (a u1 u2))",
        "(tuple (a ;; c\n u1 u2))",
        "(tuple (a ;; c\n))",
        "(tuple (a u1) (b ;; c\n))",
        "(tuple (a))",
        "(tuple ())",
        "(tuple u1)",
        "(tuple ((some a) u1))",
        "(tuple (let))",
    ] {
        assert_eq!(formatter.format_section(source, None).unwrap(), source);
        let file = formatter.format_file(source, None);
        assert_eq!(file, format!("{source}\n"));
        let ast = parse(source, StackDepthLimits::for_epoch(DEFAULT_EPOCH)).unwrap();
        let formatted = formatter.format_ast(&ast);
        assert_eq!(formatted, source.replace("\n ", "\n"));
        let reparsed = parse(&formatted, StackDepthLimits::for_epoch(DEFAULT_EPOCH)).unwrap();
        assert_eq!(formatter.format_ast(&reparsed), formatted);
        assert_eq!(formatter.format_file(&file, None), file);
    }
}

#[test]
fn unusual_nested_tuple_entries_remain_stable() {
    let source = "(define-constant x (tuple (a ;; c\nu1 u2))) ;; outer";
    assert_format(source, source);
    assert_format("(tuple (a (tuple (b u1 u2))))", "{ a: (tuple (b u1 u2)) }");
}
