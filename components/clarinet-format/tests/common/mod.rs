use clarinet_defaults::DEFAULT_EPOCH;
use clarinet_format::formatter::{ClarityFormatter, Settings};
use clarity::vm::ast::parser::v2::parse;
use clarity::vm::ast::stack_depth_checker::StackDepthLimits;
use clarity::vm::representations::PreSymbolicExpression;

pub fn parse_source(source: &str) -> Vec<PreSymbolicExpression> {
    parse(source, StackDepthLimits::for_epoch(DEFAULT_EPOCH)).unwrap()
}

/// Format `source` through every entry point, pass each result and its reparsed
/// AST to `check`, then assert that formatting the result again changes nothing.
pub fn check_entry_points(
    source: &str,
    settings: Settings,
    mut check: impl FnMut(&str, &str, &[PreSymbolicExpression]),
) {
    let formatter = ClarityFormatter::new(settings);
    let ast = parse_source(source);
    for (mode, formatted) in [
        ("file", formatter.format_file(source, None)),
        ("section", formatter.format_section(source, None).unwrap()),
        ("ast", formatter.format_ast(&ast)),
    ] {
        let reparsed = parse_source(&formatted);
        check(mode, &formatted, &reparsed);
        let repeated = match mode {
            "file" => formatter.format_file(&formatted, None),
            "section" => formatter.format_section(&formatted, None).unwrap(),
            _ => formatter.format_ast(&reparsed),
        };
        assert_eq!(repeated, formatted, "{mode}: {source}");
    }
}
