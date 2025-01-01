pub mod helpers;
pub mod ignored;

use std::fmt::format;

use clarity::vm::functions::{define::DefineFunctions, NativeFunctions};
use clarity::vm::representations::{PreSymbolicExpression, PreSymbolicExpressionType};
use helpers::{name_and_args, t};
use ignored::ignored_exprs;

// commented blocks with this string included will not be formatted
const FORMAT_IGNORE_SYNTAX: &str = "@format-ignore";

// or/and with > N comparisons will be split across multiple lines
// (or
//   true
//   (is-eq 1 1)
//   false
// )
const BOOLEAN_BREAK_LIMIT: usize = 2;

// might need to convert newlines
// https://github.com/rust-lang/rustfmt/blob/master/src/formatting/newline_style.rs
// const LINE_FEED: char = '\n';
// const CARRIAGE_RETURN: char = '\r';
// const WINDOWS_NEWLINE: &str = "\r\n";
// const UNIX_NEWLINE: &str = "\n";

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

#[derive(Clone, Copy, PartialEq)]
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

//
pub struct ClarityFormatter {
    settings: Settings,
}
impl ClarityFormatter {
    pub fn new(settings: Settings) -> Self {
        Self { settings }
    }
    /// formatting for files to ensure a newline at the end
    pub fn format_file(&mut self, source: &str) -> String {
        let pse = clarity::vm::ast::parser::v2::parse(source).unwrap();
        let result = format_toplevel(&self.settings, &pse);

        // make sure the file ends with a newline
        result.trim_end_matches('\n').to_string() + "\n"
    }
    /// Alias `format_file` to `format`
    pub fn format(&mut self, source: &str) -> String {
        self.format_file(source)
    }
    /// for range formatting within editors
    pub fn format_section(&mut self, source: &str) -> String {
        let pse = clarity::vm::ast::parser::v2::parse(source).unwrap();
        format_toplevel(&self.settings, &pse)
    }
}

#[derive(Clone)]
struct Accumulator {
    acc: String,

    /// amount of spaces/tabs indented currently
    indents: usize,

    /// string used for each indent level
    indentation_str: String,

    /// stored from Settings, used for wrapping if needed
    max_line_length: usize,
}

impl Accumulator {
    fn new(indentation_setting: Indentation, max_line_length: usize) -> Accumulator {
        Accumulator {
            acc: String::new(),
            indents: 0,
            indentation_str: indentation_setting.to_string(),
            max_line_length,
        }
    }

    /// get current spacing
    fn current_indent(&mut self) -> String {
        self.indentation_str.repeat(self.indents)
    }

    fn dedent(&mut self) {
        self.indents -= 1
    }
    fn indent(&mut self) {
        self.indents += 1
    }

    fn push(&mut self, char: char) {
        self.acc.push(char)
    }

    fn push_str(&mut self, str: &str) {
        self.acc.push_str(str)
    }

    /// alias of push_str
    fn str(&mut self, str: &str) {
        self.acc.push_str(str)
    }

    /// handles indentation after newlines
    fn newline(&mut self) {
        self.acc.push('\n');
        for _ in 0..self.indents {
            self.acc.push_str(&self.indentation_str)
        }
    }
}

pub fn format_toplevel(settings: &Settings, expressions: &[PreSymbolicExpression]) -> String {
    let acc = Accumulator::new(settings.indentation, settings.max_line_length);
    format_source_exprs(expressions, acc.clone()).acc
}

pub fn format_source_exprs(
    expressions: &[PreSymbolicExpression],
    mut acc: Accumulator,
) -> Accumulator {
    // println!("exprs: {:?}", expressions);
    // println!("previous: {:?}", previous_expr);

    // use peekable to handle trailing comments nicely
    let mut iter = expressions.iter().peekable();

    while let Some(expr) = iter.next() {
        let trailing_comment = match iter.peek().cloned() {
            Some(next) => {
                if is_comment(next) && is_same_line(expr, next) {
                    iter.next();
                    Some(next)
                } else {
                    None
                }
            }
            _ => None,
        };
        let cur = display_pse(expr, &acc);
        if cur.contains(FORMAT_IGNORE_SYNTAX) {
            if let Some(next) = iter.peek() {
                // we need PreSymbolicExpression back into orig Source
                match next.match_list() {
                    Some(list) => acc.str(&ignored_exprs(list)),
                    None => continue,
                }
                iter.next();
            };
            continue;
        }
        if let Some(list) = expr.match_list() {
            if let Some(atom_name) = list.split_first().and_then(|(f, _)| f.match_atom()) {
                if let Some(native) = NativeFunctions::lookup_by_name(atom_name) {
                    match native {
                        NativeFunctions::Let => acc = format_let(list, acc),
                        NativeFunctions::Begin => acc = format_begin(list, acc),
                        NativeFunctions::Match => acc = format_match(list, acc),
                        // NativeFunctions::IndexOf
                        // | NativeFunctions::IndexOfAlias
                        // | NativeFunctions::Asserts
                        // | NativeFunctions::ContractCall => format_general(list, acc),
                        NativeFunctions::TupleCons => {
                            // if the kv map is defined with (tuple (c 1)) then we strip the
                            // ClarityName("tuple") out first and convert it to key/value syntax
                            acc = format_key_value(&list[1..], acc)
                        }
                        NativeFunctions::If => acc = format_if(list, acc),
                        NativeFunctions::ListCons => acc = format_list(list, acc),
                        NativeFunctions::And | NativeFunctions::Or => {
                            acc = format_booleans(list, acc)
                        }
                        _ => {
                            acc = format_source_exprs(&[expr.clone()], acc);
                            if let Some(comment) = trailing_comment {
                                acc.push(' ');
                                acc.str(&display_pse(comment, &acc));
                            }
                        }
                    }
                } else if let Some(define) = DefineFunctions::lookup_by_name(atom_name) {
                    match define {
                        DefineFunctions::PublicFunction
                        | DefineFunctions::ReadOnlyFunction
                        | DefineFunctions::PrivateFunction => acc = format_function(list, acc),
                        DefineFunctions::Constant | DefineFunctions::PersistedVariable => {
                            acc = format_constant(list, acc)
                        }
                        DefineFunctions::Map => acc = format_map(list, acc),
                        DefineFunctions::UseTrait | DefineFunctions::ImplTrait => {
                            // these are the same as the following but need a trailing newline
                            acc.push('(');
                            acc = format_source_exprs(list, acc);
                            acc.push(')');
                            acc.push('\n');
                        }
                        // DefineFunctions::Trait => format_trait(settings, list),
                        // DefineFunctions::PersistedVariable
                        // DefineFunctions::FungibleToken
                        // DefineFunctions::NonFungibleToken
                        _ => {
                            acc.push('(');
                            acc = format_source_exprs(list, acc);
                            acc.push(')');
                        }
                    }
                } else {
                    acc.push('(');
                    acc = format_source_exprs(list, acc);
                    acc.push(')');
                };
                // acc.push_str(t(formatted.acc));
                continue;
            }
        }
        let current = display_pse(expr, &acc);
        let mut between = " ";
        if let Some(next) = iter.peek() {
            if !is_same_line(expr, next) || is_comment(expr) {
                between = "\n";
            }
        } else {
            // no next expression to space out
            between = "";
        }

        acc.push_str(&format!("{current}{between}"));
    }
    acc
}

fn format_constant(exprs: &[PreSymbolicExpression], mut acc: Accumulator) -> Accumulator {
    let func_type = display_pse(exprs.first().unwrap(), &acc);
    acc.str(&format!("({func_type} "));

    if let Some((name, args)) = name_and_args(exprs) {
        acc.push_str(&display_pse(name, &acc));

        // Access the value from args
        if let Some(value) = args.first() {
            if let Some(_list) = value.match_list() {
                acc.newline();
                acc = format_source_exprs(&[value.clone()], acc);
                acc.push_str("\n)");
            } else {
                // Handle non-list values (e.g., literals or simple expressions)
                acc.push(' ');
                acc.push_str(&display_pse(value, &acc));
                acc.push(')');
            }
        }

        acc.push('\n');
        acc
    } else {
        panic!("Expected a valid constant definition with (name value)")
    }
}
fn format_map(exprs: &[PreSymbolicExpression], mut acc: Accumulator) -> Accumulator {
    acc.str(&"(define-map ");
    let space = acc.current_indent();

    if let Some((name, args)) = name_and_args(exprs) {
        acc.push_str(&display_pse(name, &acc));

        for arg in args.iter() {
            match &arg.pre_expr {
                // this is hacked in to handle situations where the contents of
                // map is a 'tuple'
                PreSymbolicExpressionType::Tuple(list) => {
                    acc.newline();
                    acc = format_key_value_sugar(&list.to_vec(), acc);
                }
                _ => {
                    acc.newline();
                    acc = format_source_exprs(&[arg.clone()], acc);
                }
            }
        }

        acc.dedent();
        acc.newline();
        acc.push(')');
        acc
    } else {
        panic!("define-map without a name is invalid")
    }
}

fn is_same_line(expr1: &PreSymbolicExpression, expr2: &PreSymbolicExpression) -> bool {
    expr1.span().start_line == expr2.span().start_line
}

// this is probably un-needed but was getting some weird artifacts for code like
// (something (1 2 3) true) would be formatted as (something (1 2 3)true)
// fn format_general(
//     settings: &Settings,
//     exprs: &[PreSymbolicExpression],
//     mut acc: &Accumulator,
// ) -> Accumulator {
//     let func_type = display_pse(exprs.first().unwrap(), acc);
//     let mut acc = format!("({func_type} ");
//     for (i, arg) in exprs[1..].iter().enumerate() {
//         acc.push_str(&format!(
//             "{}{}",
//             format_source_exprs(&[arg.clone()], acc),
//             if i < exprs[1..].len() - 1 { " " } else { "" }
//         ))
//     }
//     acc.push(')');
//     acc.to_owned()
// }

// *begin* never on one line
fn format_begin(exprs: &[PreSymbolicExpression], mut acc: Accumulator) -> Accumulator {
    acc.str(&"(begin");
    let space = &acc.current_indent();

    let mut iter = exprs.get(1..).unwrap_or_default().iter().peekable();
    while let Some(expr) = iter.next() {
        // cloned() here because of the second mutable borrow on iter.next()
        let trailing = match iter.peek().cloned() {
            Some(next) => {
                if is_comment(next) && is_same_line(expr, next) {
                    iter.next();
                    Some(next)
                } else {
                    None
                }
            }
            _ => None,
        };
        acc.newline();
        acc = format_source_exprs(&[expr.clone()], acc);
        if let Some(comment) = trailing {
            acc.push(' ');
            acc.push_str(&display_pse(comment, &acc));
        }
    }
    acc.dedent();
    acc.newline();
    acc.push(')');
    acc.push('\n');
    acc
}

fn is_comment(pse: &PreSymbolicExpression) -> bool {
    matches!(pse.pre_expr, PreSymbolicExpressionType::Comment(_))
}
pub fn without_comments_len(exprs: &[PreSymbolicExpression]) -> usize {
    exprs.iter().filter(|expr| !is_comment(expr)).count()
}

// formats (and ..) and (or ...)
// if given more than BOOLEAN_BREAK_LIMIT expressions it will break it onto new lines
fn format_booleans(exprs: &[PreSymbolicExpression], mut acc: Accumulator) -> Accumulator {
    let func_type = display_pse(exprs.first().unwrap(), &acc);
    acc.str(&format!("({func_type}"));
    let space = acc.current_indent();
    let break_up = without_comments_len(&exprs[1..]) > BOOLEAN_BREAK_LIMIT;
    if break_up {
        let mut iter = exprs.get(1..).unwrap_or_default().iter().peekable();
        while let Some(expr) = iter.next() {
            let trailing = match iter.peek().cloned() {
                Some(next) => {
                    if is_comment(next) && is_same_line(expr, next) {
                        iter.next();
                        Some(next)
                    } else {
                        None
                    }
                }
                _ => None,
            };
            acc.newline();
            acc = format_source_exprs(&[expr.clone()], acc);
            if let Some(comment) = trailing {
                acc.push(' ');
                acc.push_str(&display_pse(comment, &acc));
            }
        }
    } else {
        acc.push(' ');
        acc = format_source_exprs(&exprs[1..], acc);
    }
    if break_up {
        acc.newline();
    }
    acc.push(')');
    acc
}

fn format_if(exprs: &[PreSymbolicExpression], mut acc: Accumulator) -> Accumulator {
    let func_type = display_pse(exprs.first().unwrap(), &acc);
    let space = acc.current_indent();

    acc.push_str(&format!("({func_type} "));
    let mut iter = exprs[1..].iter().peekable();
    let mut index = 0;

    while let Some(expr) = iter.next() {
        let trailing = match iter.peek().cloned() {
            Some(next) => {
                if is_comment(next) && is_same_line(expr, next) {
                    iter.next();
                    Some(next)
                } else {
                    None
                }
            }
            _ => None,
        };
        // conditional follows `if` on same line
        if index != 0 {
            acc.push('\n');
            acc.push_str(&space);
            // acc.push_str(indentation);
        }
        // expr args
        acc.indent();
        acc = format_source_exprs(&[expr.clone()], acc);
        if let Some(comment) = trailing {
            acc.push(' ');
            acc.push_str(&display_pse(comment, &acc));
        }
        index += 1;
    }
    acc.dedent();
    acc.newline();
    acc.push(')');
    acc
}

// *let* never on one line
fn format_let(exprs: &[PreSymbolicExpression], mut acc: Accumulator) -> Accumulator {
    let space = acc.current_indent();
    acc.str(&space);
    acc.str("(let (");
    if let Some(args) = exprs[1].match_list() {
        for arg in args.iter() {
            acc.newline();
            acc = format_source_exprs(&[arg.clone()], acc);
        }
    }
    acc.dedent();
    acc.newline();
    acc.str(")");
    for e in exprs.get(2..).unwrap_or_default() {
        acc.newline();
        acc = format_source_exprs(&[e.clone()], acc);
    }
    acc.dedent();
    acc.push_str(&")");
    acc
}

// * match *
// always multiple lines
fn format_match(exprs: &[PreSymbolicExpression], mut acc: Accumulator) -> Accumulator {
    acc.str("(match ");

    let space = acc.current_indent();

    // value to match on
    acc = format_source_exprs(&[exprs[1].clone()], acc);
    // branches evenly spaced
    for branch in exprs[2..].iter() {
        acc.newline();
        acc = format_source_exprs(&[branch.clone()], acc);
    }
    acc.dedent();
    acc.newline();
    acc.push_str(")");
    acc
}

fn format_list(exprs: &[PreSymbolicExpression], mut acc: Accumulator) -> Accumulator {
    acc.str("(");
    let breaks = line_length_over_max(acc.max_line_length, exprs);
    for (i, expr) in exprs[0..].iter().enumerate() {
        acc = format_source_exprs(&[expr.clone()], acc);
        // let space = if breaks { '\n' } else { ' ' };
        // if i < exprs.len() - 1 {
        //     acc.str(&acc.acc);
        //     acc.str(&acc.indentation_str);
        // } else {
        //     acc.str(&acc.acc);
        // }
    }
    let cur = acc.current_indent();
    if exprs.len() > 1 {
        acc.str(&cur);
    }
    acc.push(')');
    // t(&acc).to_string()
    acc
}

fn line_length_over_max(max_line_length: usize, exprs: &[PreSymbolicExpression]) -> bool {
    if let Some(last_expr) = exprs.last() {
        last_expr.span.end_column >= max_line_length.try_into().unwrap()
    } else {
        false
    }
}
// used for { n1: 1 } syntax
fn format_key_value_sugar(exprs: &[PreSymbolicExpression], mut acc: Accumulator) -> Accumulator {
    let over_2_kvs = without_comments_len(exprs) > 2;
    acc.str("{");

    // TODO this code is horrible
    // convert it to the peekable version like the rest
    if over_2_kvs {
        acc.newline();
        let mut counter = 1;
        for (i, expr) in exprs.iter().enumerate() {
            if is_comment(expr) {
                acc.indent();
                acc = format_source_exprs(&[expr.clone()], acc);
                acc.newline()
            } else {
                let last = i == exprs.len() - 1;
                // if counter is even we're on the value
                if counter % 2 == 0 {
                    acc.str(": ");
                    acc = format_source_exprs(&[expr.clone()], acc);
                    acc.str(if last { "" } else { "," });
                    acc.newline();
                } else {
                    // if counter is odd we're on the key
                    let cur = acc.current_indent();
                    acc.str(&cur);
                    acc = format_source_exprs(&[expr.clone()], acc);
                }
                counter += 1
            }
        }
    } else {
        // for cases where we keep it on the same line with 1 k/v pair
        let fkey = display_pse(&exprs[0], &acc);
        acc.str(&format!(" {fkey}: "));
        acc = format_source_exprs(&[exprs[1].clone()], acc);
        acc.push(' ');
    }
    if exprs.len() > 2 {
        let indent = acc.current_indent();
        acc.str(&indent);
    }
    acc.str("}");
    acc
}

// used for (tuple (n1  1)) syntax
fn format_key_value(exprs: &[PreSymbolicExpression], mut acc: Accumulator) -> Accumulator {
    let space = acc.current_indent();

    acc.str(&space);
    acc.push('{');

    // for cases where we keep it on the same line with 1 k/v pair
    let multiline = exprs.len() > 1;
    let pre = if multiline {
        format!("\n{}", space)
    } else {
        " ".to_string()
    };
    for (i, expr) in exprs.iter().enumerate() {
        let (key, value) = expr
            .match_list()
            .and_then(|list| list.split_first())
            .unwrap();
        let fkey = display_pse(key, &acc);
        let ending = if multiline {
            if i < exprs.len() - 1 {
                ","
            } else {
                "\n"
            }
        } else {
            " "
        };

        acc.push_str(&format!("{pre}{fkey}: "));
        acc = format_source_exprs(value, acc);
        acc.push_str(&ending);
    }

    let final_space = acc.current_indent();
    acc.str(&final_space);
    acc.push('}');
    acc
}

// This should panic on most things besides atoms and values. Added this to help
// debugging in the meantime
fn display_pse(pse: &PreSymbolicExpression, acc: &Accumulator) -> String {
    match pse.pre_expr {
        PreSymbolicExpressionType::Atom(ref value) => t(value.as_str()).to_string(),
        PreSymbolicExpressionType::AtomValue(ref value) => value.to_string(),
        PreSymbolicExpressionType::List(ref items) => format_list(items, acc.clone()).acc,
        PreSymbolicExpressionType::Tuple(ref items) => {
            format_key_value_sugar(items, acc.clone()).acc
        }
        PreSymbolicExpressionType::SugaredContractIdentifier(ref name) => {
            format!(".{}", name)
        }
        PreSymbolicExpressionType::SugaredFieldIdentifier(ref contract, ref field) => {
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
            format!(";; {}", t(text))
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
fn format_function(exprs: &[PreSymbolicExpression], mut acc: Accumulator) -> Accumulator {
    let func_type = display_pse(exprs.first().unwrap(), &acc.clone());

    acc.str(&format!("({func_type} ("));

    // function name and arguments
    if let Some(def) = exprs.get(1).and_then(|f| f.match_list()) {
        if let Some((name, args)) = def.split_first() {
            acc.str(&display_pse(name, &acc));

            let mut iter = args.iter().peekable();
            while let Some(arg) = iter.next() {
                // cloned() here because of the second mutable borrow on iter.next()
                let trailing = match iter.peek().cloned() {
                    Some(next) => {
                        if is_comment(next) && is_same_line(arg, next) {
                            iter.next();
                            Some(next)
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                acc.indent();
                if let Some(_list) = arg.match_list() {
                    // expr args
                    acc.newline();
                    acc = format_source_exprs(&[arg.clone()], acc);
                } else {
                    // atom args
                    acc = format_source_exprs(&[arg.clone()], acc);
                }
                if let Some(comment) = trailing {
                    acc.str(" ");
                    acc.str(&display_pse(comment, &acc.clone()));
                }
            }
            if args.is_empty() {
                acc.str(")")
            } else {
                acc.newline()
            }
        } else {
            panic!("can't have a nameless function")
        }
    }

    // function body expressions
    // TODO this should account for comments
    for expr in exprs.get(2..).unwrap_or_default() {
        acc.newline();
        acc = format_source_exprs(&[expr.clone()], acc);
    }
    acc.dedent();
    acc.str(")\n\n");
    acc
}

#[cfg(test)]
mod tests_formatter {
    use super::{ClarityFormatter, Settings};
    use crate::formatter::Indentation;
    use std::collections::HashMap;
    use std::fs;
    use std::path::Path;
    fn from_metadata(metadata: &str) -> Settings {
        let mut max_line_length = 80;
        let mut indent = Indentation::Space(2);

        let metadata_map: HashMap<&str, &str> = metadata
            .split(',')
            .map(|pair| pair.trim())
            .filter_map(|kv| kv.split_once(':'))
            .map(|(k, v)| (k.trim(), v.trim()))
            .collect();

        if let Some(length) = metadata_map.get("max_line_length") {
            max_line_length = length.parse().unwrap_or(max_line_length);
        }

        if let Some(&indentation) = metadata_map.get("indentation") {
            indent = match indentation {
                "tab" => Indentation::Tab,
                value => {
                    if let Ok(spaces) = value.parse::<usize>() {
                        Indentation::Space(spaces)
                    } else {
                        Indentation::Space(2) // Fallback to default
                    }
                }
            };
        }

        Settings {
            max_line_length,
            indentation: indent,
        }
    }
    fn format_with_default(source: &str) -> String {
        let mut formatter = ClarityFormatter::new(Settings::default());
        formatter.format_section(source)
    }
    fn format_file_with_metadata(source: &str) -> String {
        let mut lines = source.lines();
        let metadata_line = lines.next().unwrap_or_default();
        let settings = from_metadata(metadata_line);

        let real_source = lines.collect::<Vec<&str>>().join("\n");
        let mut formatter = ClarityFormatter::new(settings);
        formatter.format_file(&real_source)
    }
    fn format_with(source: &str, settings: Settings) -> String {
        let mut formatter = ClarityFormatter::new(settings);
        formatter.format_section(source)
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
        assert_eq!(expected, result);
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
        let src = "(ok true) ;; this is an inline comment";
        let result = format_with_default(&String::from(src));
        assert_eq!(src, result);
    }
    #[test]
    #[ignore]
    fn test_postcomments_included() {
        let src = "(ok true)\n;; this is a post comment";
        let result = format_with_default(&String::from(src));
        assert_eq!(src, result);
    }

    #[test]
    fn test_assert() {
        let src = "(begin (asserts! (is-eq sender tx-sender) err-not-authorised) (ok true))";
        let result = format_with_default(&String::from(src));
        let expected =
            "(begin\n  (asserts! (is-eq sender tx-sender) err-not-authorised)\n  (ok true)\n)\n";
        assert_eq!(expected, result);
    }
    #[test]
    fn test_booleans() {
        let src = "(or true false)";
        let result = format_with_default(&String::from(src));
        assert_eq!(src, result);
        let src = "(or true (is-eq 1 2) (is-eq 1 1))";
        let result = format_with_default(&String::from(src));
        let expected = "(or\n  true\n  (is-eq 1 2)\n  (is-eq 1 1)\n)";
        assert_eq!(expected, result);
    }
    #[test]
    fn test_booleans_with_comments() {
        let src = r#"(or
  true
  ;; pre comment
  (is-eq 1 2) ;; comment
  (is-eq 1 1) ;; b
)"#;
        let result = format_with_default(&String::from(src));
        assert_eq!(src, result);
    }

    #[test]
    #[ignore]
    fn long_line_unwrapping() {
        let src = "(try! (unwrap! (complete-deposit-wrapper (get txid deposit) (get vout-index deposit) (get amount deposit) (get recipient deposit) (get burn-hash deposit) (get burn-height deposit) (get sweep-txid deposit)) (err (+ ERR_DEPOSIT_INDEX_PREFIX (+ u10 index)))))";
        let result = format_with_default(&String::from(src));
        let expected = "(try! (unwrap! (complete-deposit-wrapper\n  (get txid deposit)\n  (get vout-index deposit)\n  (get amount deposit)\n  (get recipient deposit)\n  (get burn-hash deposit)\n  (get burn-height deposit)\n  (get sweep-txid deposit)\n  ) (err (+ ERR_DEPOSIT_INDEX_PREFIX (+ u10 index)))))";
        assert_eq!(expected, result);
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
    fn test_key_value_sugar_comment_midrecord() {
        let src = r#"{
  name: (buff 48),
  ;; comment
  owner: send-to
}"#;
        let result = format_with_default(&String::from(src));
        assert_eq!(src, result);
    }

    #[test]
    fn test_basic_slice() {
        let src = "(slice? (1 2 3 4 5) u5 u9)";
        let result = format_with_default(&String::from(src));
        assert_eq!(src, result);
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
        let src = "(begin (+ 1 1) ;; a\n (ok true))";
        let result = format_with_default(&String::from(src));
        assert_eq!(result, "(begin\n  (+ 1 1) ;; a\n  (ok true)\n)\n");
    }

    #[test]
    fn test_custom_tab_setting() {
        let src = "(begin (ok true))";
        let result = format_with(&String::from(src), Settings::new(Indentation::Space(4), 80));
        assert_eq!(result, "(begin\n    (ok true)\n)\n");
    }

    #[test]
    fn test_if() {
        let src = "(if (<= amount max-supply) (list) (something amount))";
        let result = format_with_default(&String::from(src));
        let expected = "(if (<= amount max-supply)\n  (list)\n  (something amount)\n)";
        assert_eq!(result, expected);
    }
    #[test]
    fn test_ignore_formatting() {
        let src = ";; @format-ignore\n(    begin ( ok true ))";
        let result = format_with(&String::from(src), Settings::new(Indentation::Space(4), 80));
        assert_eq!(src, result);
    }

    #[test]
    fn test_index_of() {
        let src = "(index-of? (contract-call? .pool borroweable) asset)";
        let result = format_with_default(&String::from(src));
        assert_eq!(src, result);
    }
    #[test]
    fn test_traits() {
        let src = "(use-trait token-a-trait 'SPAXYA5XS51713FDTQ8H94EJ4V579CXMTRNBZKSF.token-a.token-trait)\n";
        let result = format_with(&String::from(src), Settings::new(Indentation::Space(4), 80));
        assert_eq!(src, result);
        let src = "(as-contract (contract-call? .tokens mint! u19))";
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
                let result = format_file_with_metadata(&src);
                // println!("intended: {:?}", intended);
                // println!("result: {:?}", result);
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
