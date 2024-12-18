use clarity::vm::functions::{define::DefineFunctions, NativeFunctions};
use clarity::vm::representations::{PreSymbolicExpression, PreSymbolicExpressionType};
use clarity::vm::types::{TupleTypeSignature, TypeSignature};
use clarity::vm::ClarityName;

pub enum Indentation {
    Space(usize),
    Tab,
}

impl ToString for Indentation {
    fn to_string(&self) -> String {
        match self {
            Indentation::Space(count) => " ".repeat(*count),
            Indentation::Tab => "\t".to_string(),
        }
    }
}

// commented blocks with this string included will not be formatted
const FORMAT_IGNORE_SYNTAX: &str = "@format-ignore";

// or/and with > N comparisons will be split across multiple lines
// (or
//   true
//   (is-eq 1 1)
//   false
// )
const BOOLEAN_BREAK_LIMIT: usize = 2;

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
        let pse = clarity::vm::ast::parser::v2::parse(source).unwrap();
        format_source_exprs(&self.settings, &pse, "", None, "")
    }
}

pub fn format_source_exprs(
    settings: &Settings,
    expressions: &[PreSymbolicExpression],
    previous_indentation: &str,
    previous_expr: Option<PreSymbolicExpression>,
    acc: &str,
) -> String {
    // print_pre_expr(expressions);
    // println!("exprs: {:?}", expressions);
    // println!("previous: {:?}", previous_expr);
    if let Some((expr, remaining)) = expressions.split_first() {
        let cur = display_pse(
            &Settings::default(),
            expr,
            previous_indentation,
            previous_expr.is_some(),
        );
        if cur.contains(FORMAT_IGNORE_SYNTAX) {
            println!("Ignoring: {:?}", remaining)
            //     return format!("{}{}", cur, remaining);
        }
        if let Some(list) = expr.match_list() {
            if let Some(atom_name) = list.split_first().and_then(|(f, _)| f.match_atom()) {
                let formatted = if let Some(native) = NativeFunctions::lookup_by_name(atom_name) {
                    match native {
                        NativeFunctions::Let => format_let(settings, list, previous_indentation),
                        NativeFunctions::Begin => {
                            format_begin(settings, list, previous_indentation)
                        }
                        NativeFunctions::Match => {
                            format_match(settings, list, previous_indentation)
                        }
                        // (tuple (name 1))
                        // (Tuple [(PSE)])
                        NativeFunctions::TupleCons => {
                            println!("tuple cons");
                            format_tuple_cons(settings, list, previous_indentation)
                        }
                        NativeFunctions::ListCons => {
                            println!("list cons");
                            format_list(settings, list, previous_indentation)
                        }
                        NativeFunctions::And | NativeFunctions::Or => {
                            format_booleans(settings, list, previous_indentation, previous_expr)
                        }
                        _ => {
                            // println!("fallback: {:?}", list);
                            format!(
                                "({})",
                                format_source_exprs(
                                    settings,
                                    list,
                                    previous_indentation,
                                    previous_expr,
                                    acc
                                )
                            )
                        }
                    }
                } else if let Some(define) = DefineFunctions::lookup_by_name(atom_name) {
                    match define {
                        DefineFunctions::PublicFunction
                        | DefineFunctions::ReadOnlyFunction
                        | DefineFunctions::PrivateFunction => format_function(settings, list),
                        DefineFunctions::Constant => format_constant(settings, list),
                        DefineFunctions::Map => format_map(settings, list, previous_indentation),
                        DefineFunctions::UseTrait | DefineFunctions::ImplTrait => {
                            follow_with_newline(&format!(
                                "({})",
                                format_source_exprs(
                                    settings,
                                    list,
                                    previous_indentation,
                                    previous_expr,
                                    acc
                                )
                            ))
                        }
                        // DefineFunctions::Trait => format_trait(settings, list),
                        // DefineFunctions::PersistedVariable
                        // DefineFunctions::FungibleToken
                        // DefineFunctions::NonFungibleToken
                        _ => {
                            format!(
                                "({})",
                                format_source_exprs(
                                    settings,
                                    list,
                                    previous_indentation,
                                    previous_expr,
                                    acc
                                )
                            )
                        }
                    }
                } else {
                    format!(
                        "({})",
                        format_source_exprs(
                            settings,
                            list,
                            previous_indentation,
                            Some(expr.clone()),
                            acc
                        )
                    )
                };

                return format!(
                    "{}{}",
                    t(&formatted),
                    format_source_exprs(
                        settings,
                        remaining,
                        previous_indentation,
                        Some(expr.clone()),
                        acc
                    )
                )
                .to_owned();
            }
        }
        let current = display_pse(settings, expr, "", previous_expr.is_some());
        let newline = if let Some(rem) = remaining.get(1) {
            expr.span().start_line != rem.span().start_line
        } else {
            false
        };

        // if this IS NOT a pre or post comment, we need to put a space between
        // the last expression
        let pre_space = if is_comment(expr) && previous_expr.is_some() {
            " "
        } else {
            ""
        };

        let between = if remaining.is_empty() {
            ""
        } else if newline {
            "\n"
        } else {
            " "
        };
        // println!("here: {}", current);
        return format!(
            "{pre_space}{}{between}{}",
            t(&current),
            format_source_exprs(
                settings,
                remaining,
                previous_indentation,
                Some(expr.clone()),
                acc
            )
        )
        .to_owned();
    };
    acc.to_owned()
}

fn follow_with_newline(expr: &str) -> String {
    format!("{}\n", expr)
}

// trim but leaves newlines preserved
fn t(input: &str) -> &str {
    let start = input
        .find(|c: char| !c.is_whitespace() || c == '\n')
        .unwrap_or(0);

    let end = input
        .rfind(|c: char| !c.is_whitespace() || c == '\n')
        .map(|pos| pos + 1)
        .unwrap_or(0);

    &input[start..end]
}

fn name_and_args(
    exprs: &[PreSymbolicExpression],
) -> Option<(&PreSymbolicExpression, &[PreSymbolicExpression])> {
    if exprs.len() >= 2 {
        Some((&exprs[1], &exprs[2..]))
    } else {
        None // Return None if there aren't enough items
    }
}

fn format_constant(settings: &Settings, exprs: &[PreSymbolicExpression]) -> String {
    let indentation = &settings.indentation.to_string();
    let mut acc = "(define-constant ".to_string();

    if let Some((name, args)) = name_and_args(exprs) {
        acc.push_str(&display_pse(settings, name, "", false));

        // Access the value from args
        if let Some(value) = args.first() {
            if let Some(list) = value.match_list() {
                acc.push_str(&format!(
                    "\n{}({})",
                    indentation,
                    format_source_exprs(settings, list, "", None, "")
                ));
                acc.push_str("\n)");
            } else {
                // Handle non-list values (e.g., literals or simple expressions)
                acc.push(' ');
                acc.push_str(&display_pse(settings, value, "", false));
                acc.push(')');
            }
        }

        acc.push('\n');
        acc.to_owned()
    } else {
        panic!("Expected a valid constant definition with (name value)")
    }
}
fn format_map(
    settings: &Settings,
    exprs: &[PreSymbolicExpression],
    previous_indentation: &str,
) -> String {
    let mut acc = "(define-map ".to_string();
    let indentation = &settings.indentation.to_string();
    let space = format!("{}{}", previous_indentation, indentation);

    if let Some((name, args)) = name_and_args(exprs) {
        acc.push_str(&display_pse(settings, name, "", false));

        for arg in args.iter() {
            match &arg.pre_expr {
                // this is hacked in to handle situations where the contents of
                // map is a 'tuple'
                PreSymbolicExpressionType::Tuple(list) => acc.push_str(&format!(
                    "\n{}{}",
                    space,
                    format_key_value_sugar(settings, &list.to_vec(), indentation)
                )),
                _ => acc.push_str(&format!(
                    "\n{}{}",
                    space,
                    format_source_exprs(settings, &[arg.clone()], indentation, None, "")
                )),
            }
        }

        acc.push_str(&format!("\n{})\n", previous_indentation));
        acc.to_owned()
    } else {
        panic!("define-map without a name is silly")
    }
}
// *begin* never on one line
fn format_begin(
    settings: &Settings,
    exprs: &[PreSymbolicExpression],
    previous_indentation: &str,
) -> String {
    let mut begin_acc = "(begin".to_string();
    let indentation = &settings.indentation.to_string();
    let space = format!("{}{}", previous_indentation, indentation);

    for arg in exprs.get(1..).unwrap_or_default() {
        if let Some(list) = arg.match_list() {
            begin_acc.push_str(&format!(
                "\n{}({})",
                space,
                format_source_exprs(settings, list, previous_indentation, None, "")
            ))
        }
    }
    begin_acc.push_str(&format!("\n{})\n", previous_indentation));
    begin_acc.to_owned()
}

fn is_comment(pse: &PreSymbolicExpression) -> bool {
    matches!(pse.pre_expr, PreSymbolicExpressionType::Comment(_))
}
pub fn without_comments_len(exprs: &[PreSymbolicExpression]) -> usize {
    exprs.iter().filter(|expr| !is_comment(expr)).count()
}

// formats (and ..) and (or ...)
// if given more than 2 expressions it will break it onto new lines
fn format_booleans(
    settings: &Settings,
    exprs: &[PreSymbolicExpression],
    previous_indentation: &str,
    previous_expr: Option<PreSymbolicExpression>,
) -> String {
    let func_type = display_pse(settings, exprs.first().unwrap(), "", false);
    let mut acc = format!("({func_type}");
    let indentation = &settings.indentation.to_string();
    let space = format!("{}{}", previous_indentation, indentation);
    // println!("exprs: {:?}", exprs);
    if without_comments_len(&exprs[1..]) > BOOLEAN_BREAK_LIMIT {
        // let mut iter = exprs[1..].iter().peekable();

        // while let Some(arg) = iter.next() {
        //     let has_eol_comment = if let Some(next_arg) = iter.peek() {
        //         is_comment(next_arg)
        //     } else {
        //         false
        //     };

        let mut prev: Option<&PreSymbolicExpression> = None;
        for arg in exprs[1..].iter() {
            if is_comment(arg) {
                if let Some(prev_arg) = prev {
                    let newline = prev_arg.span().start_line != arg.span().start_line;
                    // Inline the comment if it follows a non-comment expression
                    acc.push_str(&format!(
                        "{}{}",
                        if newline {
                            format!("\n{}", space)
                        } else {
                            " ".to_string()
                        },
                        format_source_exprs(
                            settings,
                            &[arg.clone()],
                            previous_indentation,
                            None,
                            ""
                        )
                    ));
                }
            } else {
                acc.push_str(&format!(
                    "\n{}{}",
                    space,
                    format_source_exprs(settings, &[arg.clone()], previous_indentation, None, "")
                ));
            }
            prev = Some(arg)
        }
    } else {
        acc.push(' ');
        acc.push_str(&format_source_exprs(
            settings,
            &exprs[1..],
            previous_indentation,
            None,
            "",
        ))
    }
    if without_comments_len(&exprs[1..]) > BOOLEAN_BREAK_LIMIT {
        acc.push_str(&format!("\n{}", previous_indentation));
    }
    acc.push_str(")\n");
    acc.to_owned()
}

// *let* never on one line
fn format_let(
    settings: &Settings,
    exprs: &[PreSymbolicExpression],
    previous_indentation: &str,
) -> String {
    let mut acc = "(let (".to_string();
    let indentation = &settings.indentation.to_string();
    let space = format!("{}{}", previous_indentation, indentation);
    if let Some(args) = exprs[1].match_list() {
        for arg in args.iter() {
            acc.push_str(&format!(
                "\n{}{}",
                space,
                format_source_exprs(settings, &[arg.clone()], previous_indentation, None, "")
            ))
        }
    }
    acc.push_str(&format!("\n{})", previous_indentation));
    for e in exprs.get(2..).unwrap_or_default() {
        acc.push_str(&format!(
            "\n{}{}",
            space,
            format_source_exprs(settings, &[e.clone()], previous_indentation, None, "")
        ))
    }
    acc.push_str(&format!("\n{})", previous_indentation));
    acc.to_owned()
}

// * match *
// always multiple lines
fn format_match(
    settings: &Settings,
    exprs: &[PreSymbolicExpression],
    previous_indentation: &str,
) -> String {
    let mut acc = "(match ".to_string();
    let indentation = &settings.indentation.to_string();
    let space = format!("{}{}", previous_indentation, indentation);

    // value to match on
    acc.push_str(&format_source_exprs(
        settings,
        &[exprs[1].clone()],
        previous_indentation,
        None,
        "",
    ));
    // branches evenly spaced
    for branch in exprs[2..].iter() {
        acc.push_str(&format!(
            "\n{}{}",
            space,
            format_source_exprs(settings, &[branch.clone()], &space, None, "")
        ));
    }
    acc.push_str(&format!("\n{})", previous_indentation));
    acc.to_owned()
}

fn format_list(
    settings: &Settings,
    exprs: &[PreSymbolicExpression],
    previous_indentation: &str,
) -> String {
    let mut acc = "(".to_string();
    let breaks = line_length_over_max(settings, exprs);
    for (i, expr) in exprs[1..].iter().enumerate() {
        let value = format_source_exprs(settings, &[expr.clone()], "", None, "");
        if i < exprs.len() - 2 {
            let space = if breaks { '\n' } else { ' ' };
            acc.push_str(&value.to_string());
            acc.push_str(&format!("{space}"));
        } else {
            acc.push_str(&value.to_string());
        }
    }
    acc.push_str(&format!("\n{})", previous_indentation));
    acc.to_string()
}

fn line_length_over_max(settings: &Settings, exprs: &[PreSymbolicExpression]) -> bool {
    if let Some(last_expr) = exprs.last() {
        last_expr.span.end_column >= settings.max_line_length.try_into().unwrap()
    } else {
        false
    }
}
// used for { n1: 1 } syntax
fn format_key_value_sugar(
    settings: &Settings,
    exprs: &[PreSymbolicExpression],
    previous_indentation: &str,
) -> String {
    let indentation = &settings.indentation.to_string();
    let space = format!("{}{}", previous_indentation, indentation);
    let mut acc = "{".to_string();

    // TODO this logic depends on comments not screwing up the even numbered
    // chunkable attrs
    if exprs.len() > 2 {
        for (i, chunk) in exprs.chunks(2).enumerate() {
            if let [key, value] = chunk {
                let fkey = display_pse(settings, key, "", false);
                if i + 1 < exprs.len() / 2 {
                    acc.push_str(&format!(
                        "\n{}{fkey}: {},\n",
                        space,
                        format_source_exprs(
                            settings,
                            &[value.clone()],
                            previous_indentation,
                            None,
                            ""
                        )
                    ));
                } else {
                    acc.push_str(&format!(
                        "{}{fkey}: {}\n",
                        space,
                        format_source_exprs(
                            settings,
                            &[value.clone()],
                            previous_indentation,
                            None,
                            ""
                        )
                    ));
                }
            } else {
                panic!("Unpaired key values: {:?}", chunk);
            }
        }
    } else {
        // for cases where we keep it on the same line with 1 k/v pair
        let fkey = display_pse(settings, &exprs[0], previous_indentation, false);
        acc.push_str(&format!(
            " {fkey}: {} ",
            format_source_exprs(
                settings,
                &[exprs[1].clone()],
                previous_indentation,
                None,
                ""
            )
        ));
    }
    if exprs.len() > 2 {
        acc.push_str(previous_indentation);
    }
    acc.push('}');
    acc.to_string()
}

// used for (tuple (n1  1)) syntax
fn format_key_value(
    settings: &Settings,
    exprs: &[PreSymbolicExpression],
    previous_indentation: &str,
) -> String {
    let indentation = &settings.indentation.to_string();
    let space = format!("{}{}", previous_indentation, indentation);

    let mut acc = "{".to_string();

    if exprs.len() > 1 {
        for (i, expr) in exprs.iter().enumerate() {
            let (key, value) = expr
                .match_list()
                .and_then(|list| list.split_first())
                .unwrap();
            let fkey = display_pse(settings, key, previous_indentation, false);
            if i < exprs.len() - 1 {
                acc.push_str(&format!(
                    "\n{}{fkey}: {},",
                    space,
                    format_source_exprs(settings, value, previous_indentation, None, "")
                ));
            } else {
                acc.push_str(&format!(
                    "\n{}{fkey}: {}\n",
                    space,
                    format_source_exprs(settings, value, previous_indentation, None, "")
                ));
            }
        }
    } else {
        // for cases where we keep it on the same line with 1 k/v pair
        for expr in exprs[0..].iter() {
            let (key, value) = expr
                .match_list()
                .and_then(|list| list.split_first())
                .unwrap();
            let fkey = display_pse(settings, key, previous_indentation, false);
            acc.push_str(&format!(
                " {fkey}: {} ",
                format_source_exprs(settings, value, &settings.indentation.to_string(), None, "")
            ));
        }
    }
    acc.push_str(previous_indentation);
    acc.push('}');
    acc.to_string()
}
fn format_tuple_cons(
    settings: &Settings,
    exprs: &[PreSymbolicExpression],
    previous_indentation: &str,
) -> String {
    // if the kv map is defined with (tuple (c 1)) then we have to strip the
    // ClarityName("tuple") out first
    format_key_value(settings, &exprs[1..], previous_indentation)
}

// This should panic on most things besides atoms and values. Added this to help
// debugging in the meantime
fn display_pse(
    settings: &Settings,
    pse: &PreSymbolicExpression,
    previous_indentation: &str,
    has_previous_expr: bool,
) -> String {
    match pse.pre_expr {
        PreSymbolicExpressionType::Atom(ref value) => t(value.as_str()).to_string(),
        PreSymbolicExpressionType::AtomValue(ref value) => value.to_string(),
        PreSymbolicExpressionType::List(ref items) => {
            format_list(settings, items, previous_indentation)
        }
        PreSymbolicExpressionType::Tuple(ref items) => {
            format_key_value_sugar(settings, items, previous_indentation)
        }
        PreSymbolicExpressionType::SugaredContractIdentifier(ref name) => {
            format!(".{}", name)
        }
        PreSymbolicExpressionType::SugaredFieldIdentifier(ref contract, ref field) => {
            println!("sugar field id");
            format!(".{}.{}", contract, field)
        }
        PreSymbolicExpressionType::FieldIdentifier(ref trait_id) => {
            format!("'{}", trait_id)
        }
        PreSymbolicExpressionType::TraitReference(ref name) => {
            println!("trait ref: {}", name);
            name.to_string()
        }
        PreSymbolicExpressionType::Comment(ref text) => {
            // println!("{:?}", has_previous_expr);
            if has_previous_expr {
                format!(" ;; {}\n", t(text))
            } else {
                format!(";; {}", t(text))
            }
        }
        PreSymbolicExpressionType::Placeholder(ref placeholder) => {
            placeholder.to_string() // Placeholder is for if parsing fails
        }
    }
}

// * functions

// Top level define-<function> should have a line break above and after (except on first line)
// options always on new lines
// Functions Always on multiple lines, even if short
fn format_function(settings: &Settings, exprs: &[PreSymbolicExpression]) -> String {
    let func_type = display_pse(settings, exprs.first().unwrap(), "", false);
    let indentation = &settings.indentation.to_string();

    let mut acc = format!("({func_type} (");

    // function name and arguments
    if let Some(def) = exprs.get(1).and_then(|f| f.match_list()) {
        if let Some((name, args)) = def.split_first() {
            acc.push_str(&display_pse(settings, name, "", false));
            for arg in args.iter() {
                // TODO account for eol comments
                if let Some(list) = arg.match_list() {
                    acc.push_str(&format!(
                        "\n{}{}({})",
                        indentation,
                        indentation,
                        format_source_exprs(
                            settings,
                            list,
                            &settings.indentation.to_string(),
                            None,
                            ""
                        )
                    ))
                } else {
                    acc.push_str(&format_source_exprs(
                        settings,
                        &[arg.clone()],
                        &settings.indentation.to_string(),
                        None,
                        "",
                    ))
                }
            }
            if args.is_empty() {
                acc.push(')')
            } else {
                acc.push_str(&format!("\n{})", indentation))
            }
        } else {
            panic!("can't have a nameless function")
        }
    }

    // function body expressions
    // TODO this should account for comments
    for expr in exprs.get(2..).unwrap_or_default() {
        acc.push_str(&format!(
            "\n{}{}",
            indentation,
            format_source_exprs(
                settings,
                &[expr.clone()],
                &settings.indentation.to_string(),
                None, // TODO
                ""
            )
        ))
    }
    acc.push_str("\n)\n\n");
    acc.to_owned()
}

// debugging thingy
fn print_pre_expr(exprs: &[PreSymbolicExpression]) {
    for expr in exprs {
        println!(
            "{} -- {:?}",
            display_pse(&Settings::default(), expr, "", false),
            expr.pre_expr
        )
    }
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
    fn test_manual_tuple() {
        let result = format_with_default(&String::from("(tuple (n1 1))"));
        assert_eq!(result, "{ n1: 1 }");
        let result = format_with_default(&String::from("(tuple (n1 1) (n2 2))"));
        assert_eq!(result, "{\n  n1: 1,\n  n2: 2\n}");
    }

    #[test]
    fn test_function_formatter() {
        let result = format_with_default(&String::from("(define-private (my-func) (ok true))"));
        assert_eq!(result, "(define-private (my-func)\n  (ok true)\n)\n\n");
    }

    #[test]
    fn test_multi_function() {
        let src = "(define-public (my-func) (ok true))\n(define-public (my-func2) (ok true))";
        let result = format_with_default(&String::from(src));
        let expected = r#"(define-public (my-func)
  (ok true)
)

(define-public (my-func2)
  (ok true)
)

"#;
        assert_eq!(result, expected);
    }
    #[test]
    fn test_function_args_multiline() {
        let src = "(define-public (my-func (amount uint) (sender principal)) (ok true))";
        let result = format_with_default(&String::from(src));
        assert_eq!(
            result,
            "(define-public (my-func\n    (amount uint)\n    (sender principal)\n  )\n  (ok true)\n)\n\n"
        );
    }
    #[test]
    fn test_pre_comments_included() {
        let src = ";; this is a pre comment\n;; multi\n(ok true)";
        let result = format_with_default(&String::from(src));
        assert_eq!(src, result);
    }

    #[test]
    fn test_inline_comments_included() {
        let src = "(ok true) ;; this is an inline comment\n";
        let result = format_with_default(&String::from(src));
        assert_eq!(src, result);
    }
    #[test]
    fn test_postcomments_included() {
        let src = "(ok true)\n;; this is a post comment\n";
        let result = format_with_default(&String::from(src));
        assert_eq!(src, result);
    }

    #[test]
    fn test_booleans() {
        let src = "(or true false)\n";
        let result = format_with_default(&String::from(src));
        assert_eq!(src, result);
        let src = "(or true (is-eq 1 2) (is-eq 1 1))\n";
        let result = format_with_default(&String::from(src));
        let expected = "(or\n  true\n  (is-eq 1 2)\n  (is-eq 1 1)\n)\n";
        assert_eq!(expected, result);
    }
    #[test]
    fn test_booleans_with_comments() {
        let src = r#"(or
  true
  ;; pre comment
  (is-eq 1 2) ;; comment
  (is-eq 1 1) ;; b
)
"#;
        let result = format_with_default(&String::from(src));
        assert_eq!(src, result);
    }

    #[test]
    fn long_line_unwrapping() {
        let src = "(try! (unwrap! (complete-deposit-wrapper (get txid deposit) (get vout-index deposit) (get amount deposit) (get recipient deposit) (get burn-hash deposit) (get burn-height deposit) (get sweep-txid deposit)) (err (+ ERR_DEPOSIT_INDEX_PREFIX (+ u10 index)))))";
        let result = format_with_default(&String::from(src));
        let expected = "(try! (unwrap! (complete-deposit-wrapper\n  (get txid deposit)\n  (get vout-index deposit)\n  (get amount deposit)\n  (get recipient deposit)\n  (get burn-hash deposit)\n  (get burn-height deposit)\n  (get sweep-txid deposit)\n  ) (err (+ ERR_DEPOSIT_INDEX_PREFIX (+ u10 index)))))";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_map() {
        let src = "(define-map a uint {n1: (buff 20)})";
        let result = format_with_default(&String::from(src));
        assert_eq!(result, "(define-map a\n  uint\n  { n1: (buff 20) }\n)\n");
        let src = "(define-map something { name: (buff 48), a: uint } uint)";
        let result = format_with_default(&String::from(src));
        assert_eq!(
            result,
            "(define-map something\n  {\n    name: (buff 48),\n    a: uint\n  }\n  uint\n)\n"
        );
    }

    #[test]
    fn test_let() {
        let src = "(let ((a 1) (b 2)) (+ a b))";
        let result = format_with_default(&String::from(src));
        let expected = "(let (\n  (a 1)\n  (b 2)\n)\n  (+ a b)\n)";
        assert_eq!(expected, result);
    }

    #[test]
    fn test_option_match() {
        let src = "(match opt value (ok (handle-new-value value)) (ok 1))";
        let result = format_with_default(&String::from(src));
        // "(match opt\n
        let expected = r#"(match opt
  value
  (ok (handle-new-value value))
  (ok 1)
)"#;
        assert_eq!(result, expected);
    }

    #[test]
    fn test_response_match() {
        let src = "(match x value (ok (+ to-add value)) err-value (err err-value))";
        let result = format_with_default(&String::from(src));
        let expected = r#"(match x
  value
  (ok (+ to-add value))
  err-value
  (err err-value)
)"#;
        assert_eq!(result, expected);
    }
    #[test]
    fn test_key_value_sugar() {
        let src = "{name: (buff 48)}";
        let result = format_with_default(&String::from(src));
        assert_eq!(result, "{ name: (buff 48) }");
        let src = "{ name: (buff 48), a: uint }";
        let result = format_with_default(&String::from(src));
        assert_eq!(result, "{\n  name: (buff 48),\n  a: uint\n}");
    }

    #[test]
    fn test_constant() {
        let src = "(define-constant something 1)\n";
        let result = format_with_default(&String::from(src));
        assert_eq!(result, "(define-constant something 1)\n");
        let src2 = "(define-constant something (1 2))\n";
        let result2 = format_with_default(&String::from(src2));
        assert_eq!(result2, "(define-constant something\n  (1 2)\n)\n");
    }

    #[test]
    fn test_begin_never_one_line() {
        let src = "(begin (ok true))";
        let result = format_with_default(&String::from(src));
        assert_eq!(result, "(begin\n  (ok true)\n)\n");
    }

    #[test]
    fn test_begin() {
        let src = "(begin (+ 1 1) (ok true))";
        let result = format_with_default(&String::from(src));
        assert_eq!(result, "(begin\n  (+ 1 1)\n  (ok true)\n)\n");
    }

    #[test]
    fn test_custom_tab_setting() {
        let src = "(begin (ok true))";
        let result = format_with(&String::from(src), Settings::new(Indentation::Space(4), 80));
        assert_eq!(result, "(begin\n    (ok true)\n)\n");
    }

    #[test]
    #[ignore]
    fn test_ignore_formatting() {
        let src = ";; @format-ignore\n(    begin ( ok true ))";
        let result = format_with(&String::from(src), Settings::new(Indentation::Space(4), 80));
        assert_eq!(src, result);
    }

    #[test]
    fn test_traits() {
        let src = "(use-trait token-a-trait 'SPAXYA5XS51713FDTQ8H94EJ4V579CXMTRNBZKSF.token-a.token-trait)\n";
        let result = format_with(&String::from(src), Settings::new(Indentation::Space(4), 80));
        assert_eq!(src, result);
        let src = "(as-contract (contract-call? .tokens mint! u19)) ;; Returns (ok u19)\n";
        let result = format_with(&String::from(src), Settings::new(Indentation::Space(4), 80));
        assert_eq!(src, result);
    }

    #[test]
    #[ignore]
    fn test_irl_contracts() {
        let golden_dir = "./tests/golden";
        let intended_dir = "./tests/golden-intended";

        // Iterate over files in the golden directory
        for entry in fs::read_dir(golden_dir).expect("Failed to read golden directory") {
            let entry = entry.expect("Failed to read directory entry");
            let path = entry.path();

            if path.is_file() {
                let src = fs::read_to_string(&path).expect("Failed to read source file");

                let file_name = path.file_name().expect("Failed to get file name");
                let intended_path = Path::new(intended_dir).join(file_name);

                let intended =
                    fs::read_to_string(&intended_path).expect("Failed to read intended file");

                // Apply formatting and compare
                let result = format_with_default(&src);
                pretty_assertions::assert_eq!(
                    result,
                    intended,
                    "Mismatch in file: {:?}",
                    file_name
                );
            }
        }
    }
}
