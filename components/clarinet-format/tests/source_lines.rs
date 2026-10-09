use clarinet_defaults::DEFAULT_EPOCH;
use clarinet_format::formatter::ignored::extract_source_range;
use clarinet_format::formatter::{Aggregator, ClarityFormatter, Settings};
use clarity::vm::ast::parser::v2::parse;
use clarity::vm::ast::stack_depth_checker::StackDepthLimits;

#[test]
fn source_range_boundaries() {
    for newline in ["\n", "\r\n"] {
        let source = ["first", "", "last"].join(newline);
        for (range, expected) in [
            ((1, 2, 1, 4), "irs"),
            ((1, 3, 3, 2), "rst\n\nla"),
            ((2, 1, 3, 99), "\nlast"),
            ((3, 1, 3, 4), "last"),
            ((3, 5, 3, 9), ""),
            ((4, 1, 4, 9), ""),
        ] {
            let (start_line, start_column, end_line, end_column) = range;
            assert_eq!(
                extract_source_range(&source, start_line, start_column, end_line, end_column),
                expected
            );
        }
    }
    assert_eq!(extract_source_range("", 1, 1, 1, 1), "");
}

#[test]
fn source_extraction_preserves_literals_and_ignored_blocks_across_calls() {
    let settings = Settings::default();
    let formatter = ClarityFormatter::new(Settings::default());
    // Reuse one formatter across different sources and newline conventions.
    for name in ["first", "different", "first"] {
        let source = format!(
            r#"(define-constant {name} u"a\n\u{{e9}}")
;; @format-ignore
(define-constant ignored   (list u"x\t"
    u"y"))
(define-constant last u"end")"#
        );
        for newline in ["\n", "\r\n"] {
            for trailing_newline in [false, true] {
                let mut input = source.replace('\n', newline);
                if trailing_newline {
                    input.push_str(newline);
                }
                let expected = format!("{source}\n");
                if newline == "\r\n" {
                    // The parser rejects CRLF; verify extraction using LF-parsed spans.
                    let ast = parse(&source, StackDepthLimits::for_epoch(DEFAULT_EPOCH)).unwrap();
                    let aggregator = Aggregator::new(&settings, &ast, Some(&input));
                    assert_eq!(aggregator.generate().trim_end(), source);
                    continue;
                }
                assert_eq!(formatter.format_file(&input, None), expected);
                assert_eq!(
                    formatter.format_section(&input, None).unwrap().trim_end(),
                    source
                );
                assert_eq!(formatter.format_file(&expected, None), expected);
            }
        }
    }
}
