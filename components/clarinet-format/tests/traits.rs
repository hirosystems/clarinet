use clarinet_defaults::DEFAULT_EPOCH;
use clarinet_format::formatter::{ClarityFormatter, Indentation, Settings};
use clarity::vm::ast::parser::v2::parse;
use clarity::vm::ast::stack_depth_checker::StackDepthLimits;
use clarity::vm::representations::{PreSymbolicExpression, PreSymbolicExpressionType};
use indoc::indoc;

#[derive(Debug, Default, PartialEq)]
struct Contents {
    syntax: Vec<String>,
    comments: Vec<String>,
}

impl Contents {
    fn collect(&mut self, expressions: &[PreSymbolicExpression]) {
        for expression in expressions {
            match &expression.pre_expr {
                PreSymbolicExpressionType::Comment(text) => self.comments.push(text.clone()),
                PreSymbolicExpressionType::List(children) => {
                    self.syntax.push("(".into());
                    self.collect(children);
                    self.syntax.push(")".into());
                }
                PreSymbolicExpressionType::Tuple(children) => {
                    self.syntax.push("{".into());
                    self.collect(children);
                    self.syntax.push("}".into());
                }
                leaf => self.syntax.push(format!("{leaf:?}")),
            }
        }
    }
}

fn assert_preserved(source: &str, settings: Settings, expected: Option<&str>) {
    let parse = |source: &str| parse(source, StackDepthLimits::for_epoch(DEFAULT_EPOCH)).unwrap();
    let ast = parse(source);
    let mut original = Contents::default();
    original.collect(&ast);
    let formatter = ClarityFormatter::new(settings);
    for (mode, formatted) in [
        ("file", formatter.format_file(source, None)),
        ("section", formatter.format_section(source, None).unwrap()),
        ("ast", formatter.format_ast(&ast)),
    ] {
        if let Some(expected) = expected {
            assert_eq!(formatted.trim_end(), expected, "{mode}: {source}");
        }
        let reparsed = parse(&formatted);
        let mut actual = Contents::default();
        actual.collect(&reparsed);
        assert_eq!(actual, original, "{mode}: {source}\nformatted: {formatted}");
        let repeated = match mode {
            "file" => formatter.format_file(&formatted, None),
            "section" => formatter.format_section(&formatted, None).unwrap(),
            _ => formatter.format_ast(&reparsed),
        };
        assert_eq!(repeated, formatted, "{mode}: {source}");
    }
}

#[test]
fn comments_do_not_change_trait_header_positions() {
    assert_preserved(
        "(define-trait t ;; name\n ((f () (response uint uint))))",
        Settings::default(),
        Some(indoc! {"
            (define-trait t ;; name
              (
                (f
                  ()
                  (response uint uint)
                )
              )
            )"}),
    );
    assert_preserved(
        "(define-trait ;; definition\n t ())",
        Settings::default(),
        Some(indoc! {"
            (define-trait ;; definition
              t ()
            )"}),
    );
}

#[test]
fn comments_before_the_trait_close_are_preserved() {
    for (source, expected) in [
        (
            "(define-trait t () ;; empty\n)",
            "(define-trait t () ;; empty\n)",
        ),
        (
            "(define-trait t ((f () (response uint uint))) ;; methods\n)",
            indoc! {"
                (define-trait t (
                  (f
                    ()
                    (response uint uint)
                  )
                ) ;; methods
                )"},
        ),
        (
            indoc! {"
                (define-trait t ()
                  ;; first
                  ;; second
                )"},
            indoc! {"
                (define-trait t ()
                  ;; first
                  ;; second
                )"},
        ),
    ] {
        assert_preserved(source, Settings::default(), Some(expected));
    }
}

#[test]
fn comments_before_method_names_preserve_the_names() {
    assert_preserved(
        "(define-trait t ((;; name\n f () (response uint uint))))",
        Settings::default(),
        Some(indoc! {"
            (define-trait t (
              (
                ;; name
                f
                ()
                (response uint uint)
              )
            ))"}),
    );
}

#[test]
fn unusual_trait_forms_preserve_all_syntax() {
    for source in [
        "(define-trait)",
        "(define-trait t)",
        "(define-trait t uint)",
        "(define-trait t () extra)",
        "(define-trait t (unexpected))",
        "(define-trait t (() (f uint (response uint uint) extra)))",
        "(define-trait t ((;; only comment\n)))",
        "(define-trait t ;; name\n uint ;; tail\n)",
    ] {
        assert_preserved(source, Settings::default(), None);
    }
}

#[test]
fn comments_in_trait_gaps_preserve_structure_and_order() {
    // Every subset of the header, method-list, and method-signature gaps. Two
    // comments per gap catch consumers that accidentally handle only the first.
    let tokens = [
        "(",
        "define-trait",
        "t",
        "(",
        "(",
        "f",
        "(",
        "uint",
        ")",
        "(",
        "response",
        "uint",
        "uint",
        ")",
        ")",
        ")",
        ")",
    ];
    let gaps = [2, 3, 4, 5, 6, 9, 14, 15, 16];
    for mask in 0..(1 << gaps.len()) {
        for standalone in [false, true] {
            let mut source = String::new();
            for (i, token) in tokens.iter().enumerate() {
                if i > 0 {
                    source.push(' ');
                }
                if gaps
                    .iter()
                    .enumerate()
                    .any(|(bit, gap)| *gap == i && mask & (1 << bit) != 0)
                {
                    if standalone {
                        source.push('\n');
                    }
                    source.push_str(&format!(";; gap-{i}\n;; more-{i}\n"));
                }
                source.push_str(token);
            }
            for settings in [
                Settings::default(),
                Settings::new(Indentation::Space(4), 20),
                Settings::new(Indentation::Tab, 30),
            ] {
                assert_preserved(&source, settings, None);
            }
        }
    }
}
