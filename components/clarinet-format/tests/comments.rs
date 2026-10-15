use clarinet_defaults::DEFAULT_EPOCH;
use clarinet_format::formatter::{ClarityFormatter, Settings};
use clarity::vm::ast::parser::v2::parse;
use clarity::vm::ast::stack_depth_checker::StackDepthLimits;
use indoc::{formatdoc, indoc};

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
            indoc! {"
                (define-constant
                  x ;; c
                  u1
                )"},
        ),
        (
            "(define-data-var v (list 10 uint)\n ;; c\n (list))",
            indoc! {"
                (define-data-var
                  v
                  (list 10 uint)
                  ;; c
                  (list)
                )"},
        ),
        (
            "(define-fungible-token token ;; c\n u10)",
            indoc! {"
                (define-fungible-token
                  token ;; c
                  u10
                )"},
        ),
        (
            "(define-non-fungible-token token\n ;; c\n uint)",
            indoc! {"
                (define-non-fungible-token
                  token
                  ;; c
                  uint
                )"},
        ),
        (
            "(define-constant x u1 ;; end\n)",
            indoc! {"
                (define-constant
                  x
                  u1 ;; end
                )"},
        ),
        (
            indoc! {"
                (define-constant ;; first
                 x
                 ;; second
                 ;; third
                 u1)"},
            indoc! {"
                (define-constant
                  ;; first
                  x
                  ;; second
                  ;; third
                  u1
                )"},
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
            indoc! {"
                (define-read-only (f)
                  (begin
                    (foo u1 ;; c
                    )
                    (ok u1)))"},
            indoc! {"
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
    let expected = indoc! {"
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
        indoc! {"
            (tuple
             ;; before
             (a u1) ;; trailing
             ;; between
             (b u2)
             ;; after
            )"},
        indoc! {"
            {
              ;; before
              a: u1, ;; trailing
              ;; between
              b: u2,
              ;; after
            }"},
    );
    assert_format("(tuple ;; only comment\n)", "{\n  ;; only comment\n}");
}

#[test]
fn commented_tuple_entries_convert_to_sugar() {
    for (source, expected) in [
        (
            "(tuple (a ;; c\n u1))",
            indoc! {"
                {
                  a:
                    ;; c
                    u1,
                }"},
        ),
        ("(tuple (a u1 ;; c\n))", "{\n  a: u1, ;; c\n}"),
        (
            "(tuple (a u1\n ;; after\n))",
            indoc! {"
                {
                  a: u1,
                  ;; after
                }"},
        ),
        (
            "(tuple (a u1 ;; inner\n) ;; outer\n)",
            indoc! {"
                {
                  a: u1, ;; inner
                  ;; outer
                }"},
        ),
        (
            "(tuple (a u1) (b ;; first\n ;; second\n u2))",
            indoc! {"
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
            indoc! {"
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
        indoc! {"
            { a:
             ;; first
             ;; second
             u1 }"},
        indoc! {"
            {
              a:
                ;; first
                ;; second
                u1,
            }"},
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

#[test]
fn ignored_tuple_values_keep_their_source() {
    let formatter = ClarityFormatter::new(Settings::default());
    for value in ["(+   u1   u2)", "(let)"] {
        let expected = formatdoc! {"
            {{
              a:
                ;; before
                ;; @format-ignore
                {value},
            }}"};
        for source in [
            formatdoc! {"
                (tuple (a ;; before
                 ;; @format-ignore
                 {value}))"},
            formatdoc! {"
                {{ a: ;; before
                 ;; @format-ignore
                 {value} }}"},
        ] {
            assert_eq!(
                formatter.format_file(&source, None),
                format!("{expected}\n")
            );
            assert_eq!(formatter.format_section(&source, None).unwrap(), expected);
            assert_eq!(
                formatter.format_file(&expected, None),
                format!("{expected}\n")
            );
        }
    }
    assert_format(
        indoc! {"
            (tuple (a ;; @format-ignore
             ;; ordinary comment
             (+   u1   u2)))"},
        indoc! {"
            {
              a:
                ;; @format-ignore
                ;; ordinary comment
                (+ u1 u2),
            }"},
    );
}
