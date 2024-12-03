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
        format_source_exprs(&self.settings, &pse, "", "")
    }
}

pub fn format_source_exprs(
    settings: &Settings,
    expressions: &[PreSymbolicExpression],
    previous_indentation: &str,
    acc: &str,
) -> String {
    if let Some((expr, remaining)) = expressions.split_first() {
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
                        NativeFunctions::TupleCons => format_tuple_cons(settings, list),
                        NativeFunctions::ListCons => {
                            format_list(settings, list, previous_indentation)
                        }
                        NativeFunctions::And | NativeFunctions::Or => {
                            format_booleans(settings, list, previous_indentation)
                        }
                        _ => format!("({})", format_source_exprs(settings, list, "", acc)),
                    }
                } else if let Some(define) = DefineFunctions::lookup_by_name(atom_name) {
                    match define {
                        DefineFunctions::PublicFunction
                        | DefineFunctions::ReadOnlyFunction
                        | DefineFunctions::PrivateFunction => format_function(settings, list),
                        DefineFunctions::Constant => format_constant(settings, list),
                        DefineFunctions::UseTrait => format_use_trait(settings, list),
                        DefineFunctions::Trait => format_trait(settings, list),
                        DefineFunctions::Map => format_map(settings, list, previous_indentation),
                        DefineFunctions::ImplTrait => format_impl_trait(settings, list),
                        // DefineFunctions::PersistedVariable
                        // DefineFunctions::FungibleToken
                        // DefineFunctions::NonFungibleToken
                        _ => format!(
                            "({})",
                            format_source_exprs(settings, list, previous_indentation, acc)
                        ),
                    }
                } else {
                    format!(
                        "({})",
                        format_source_exprs(settings, list, previous_indentation, acc)
                    )
                };

                return format!(
                    "{formatted}{}",
                    format_source_exprs(settings, remaining, previous_indentation, acc)
                )
                .trim()
                .to_owned();
            }
        }
        let current = display_pse(settings, expr, "");
        return format!(
            "{}{}{}",
            current,
            if current.ends_with('\n') { "" } else { " " },
            format_source_exprs(settings, remaining, previous_indentation, acc)
        )
        .trim()
        .to_owned();
    };
    acc.to_owned()
}

fn format_use_trait(settings: &Settings, exprs: &[PreSymbolicExpression]) -> String {
    // delegates to display_pse
    format_source_exprs(settings, exprs, "", "")
}
fn format_impl_trait(settings: &Settings, exprs: &[PreSymbolicExpression]) -> String {
    // delegates to display_pse
    format_source_exprs(settings, exprs, "", "")
}
fn format_trait(settings: &Settings, exprs: &[PreSymbolicExpression]) -> String {
    // delegates to display_pse
    format_source_exprs(settings, exprs, "", "")
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
        acc.push_str(&display_pse(settings, name, ""));

        // Access the value from args
        if let Some(value) = args.first() {
            if let Some(list) = value.match_list() {
                acc.push_str(&format!(
                    "\n{}({})",
                    indentation,
                    format_source_exprs(settings, list, "", "")
                ));
                acc.push_str("\n)");
            } else {
                // Handle non-list values (e.g., literals or simple expressions)
                acc.push(' ');
                acc.push_str(&display_pse(settings, value, ""));
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

    if let Some((name, args)) = name_and_args(exprs) {
        acc.push_str(&display_pse(settings, name, ""));

        for arg in args.iter() {
            match &arg.pre_expr {
                // this is hacked in to handle situations where the contents of
                // map is a 'tuple'
                PreSymbolicExpressionType::Tuple(list) => acc.push_str(&format!(
                    "\n{}{}{}",
                    previous_indentation,
                    indentation,
                    format_key_value_sugar(settings, &list.to_vec(), indentation)
                )),
                _ => acc.push_str(&format!(
                    "\n{}{}{}",
                    previous_indentation,
                    indentation,
                    format_source_exprs(settings, &[arg.clone()], indentation, "")
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

    for arg in exprs.get(1..).unwrap_or_default() {
        if let Some(list) = arg.match_list() {
            begin_acc.push_str(&format!(
                "\n{}{}({})",
                previous_indentation,
                indentation,
                format_source_exprs(settings, list, previous_indentation, "")
            ))
        }
    }
    begin_acc.push_str(&format!("\n{})\n", previous_indentation));
    begin_acc.to_owned()
}

// formats (and ..) and (or ...)
// if given more than 2 expressions it will break it onto new lines
fn format_booleans(
    settings: &Settings,
    exprs: &[PreSymbolicExpression],
    previous_indentation: &str,
) -> String {
    let func_type = display_pse(settings, exprs.first().unwrap(), "");
    let mut acc = format!("({func_type}");
    let indentation = &settings.indentation.to_string();
    if exprs[1..].len() > 2 {
        for arg in exprs[1..].iter() {
            acc.push_str(&format!(
                "\n{}{}{}",
                previous_indentation,
                indentation,
                format_source_exprs(settings, &[arg.clone()], previous_indentation, "")
            ))
        }
    } else {
        acc.push(' ');
        acc.push_str(&format_source_exprs(
            settings,
            &exprs[1..],
            previous_indentation,
            "",
        ))
    }
    if exprs[1..].len() > 2 {
        acc.push_str(&format!("\n{}", previous_indentation));
    }
    acc.push(')');
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
    if let Some(args) = exprs[1].match_list() {
        for arg in args.iter() {
            acc.push_str(&format!(
                "\n{}{}{}",
                previous_indentation,
                indentation,
                format_source_exprs(settings, &[arg.clone()], previous_indentation, "")
            ))
        }
    }
    acc.push_str(&format!("\n{})", previous_indentation));
    for e in exprs.get(2..).unwrap_or_default() {
        acc.push_str(&format!(
            "\n{}{}{}",
            previous_indentation,
            indentation,
            format_source_exprs(settings, &[e.clone()], previous_indentation, "")
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

    acc.push_str(&display_pse(settings, &exprs[1], "").to_string());
    // first branch. some or ok binding
    acc.push_str(&format!(
        "\n{}{}{} {}",
        previous_indentation,
        indentation,
        display_pse(settings, &exprs[2], previous_indentation),
        format_source_exprs(settings, &[exprs[3].clone()], previous_indentation, "")
    ));
    // second branch. none or err binding
    if let Some(some_branch) = exprs[4].match_list() {
        acc.push_str(&format!(
            "\n{}{}({})",
            previous_indentation,
            indentation,
            format_source_exprs(settings, some_branch, previous_indentation, "")
        ));
    } else {
        acc.push_str(&format!(
            "\n{}{}{} {}",
            previous_indentation,
            indentation,
            display_pse(settings, &exprs[4], previous_indentation),
            format_source_exprs(settings, &[exprs[5].clone()], previous_indentation, "")
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
    for (i, expr) in exprs[1..].iter().enumerate() {
        let value = format_source_exprs(settings, &[expr.clone()], "", "");
        if i < exprs.len() - 2 {
            acc.push_str(&format!("{value} "));
        } else {
            acc.push_str(&value.to_string());
        }
    }
    acc.push_str(&format!("\n{})", previous_indentation));
    acc.to_string()
}

// used for { n1: 1 } syntax
fn format_key_value_sugar(
    settings: &Settings,
    exprs: &[PreSymbolicExpression],
    previous_indentation: &str,
) -> String {
    let indentation = &settings.indentation.to_string();
    let mut acc = "{".to_string();

    // TODO this logic depends on comments not screwing up the even numbered
    // chunkable attrs
    if exprs.len() > 2 {
        for (i, chunk) in exprs.chunks(2).enumerate() {
            if let [key, value] = chunk {
                let fkey = display_pse(settings, key, "");
                if i + 1 < exprs.len() / 2 {
                    acc.push_str(&format!(
                        "\n{}{}{fkey}: {},\n",
                        previous_indentation,
                        indentation,
                        format_source_exprs(settings, &[value.clone()], previous_indentation, "")
                    ));
                } else {
                    acc.push_str(&format!(
                        "{}{}{fkey}: {}\n",
                        previous_indentation,
                        indentation,
                        format_source_exprs(settings, &[value.clone()], previous_indentation, "")
                    ));
                }
            } else {
                panic!("Unpaired key values: {:?}", chunk);
            }
        }
    } else {
        // for cases where we keep it on the same line with 1 k/v pair
        let fkey = display_pse(settings, &exprs[0], previous_indentation);
        acc.push_str(&format!(
            " {fkey}: {} ",
            format_source_exprs(settings, &[exprs[1].clone()], previous_indentation, "")
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
    let mut acc = "{".to_string();

    if exprs.len() > 1 {
        for (i, expr) in exprs.iter().enumerate() {
            let (key, value) = expr
                .match_list()
                .and_then(|list| list.split_first())
                .unwrap();
            let fkey = display_pse(settings, key, previous_indentation);
            if i < exprs.len() - 1 {
                acc.push_str(&format!(
                    "\n{}{fkey}: {},",
                    indentation,
                    format_source_exprs(settings, value, previous_indentation, "")
                ));
            } else {
                acc.push_str(&format!(
                    "\n{}{fkey}: {}\n",
                    indentation,
                    format_source_exprs(settings, value, previous_indentation, "")
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
            let fkey = display_pse(settings, key, previous_indentation);
            acc.push_str(&format!(
                " {fkey}: {} ",
                format_source_exprs(settings, value, &settings.indentation.to_string(), "")
            ));
        }
    }
    acc.push('}');
    acc.to_string()
}
fn format_tuple_cons(settings: &Settings, exprs: &[PreSymbolicExpression]) -> String {
    // if the kv map is defined with (tuple (c 1)) then we have to strip the
    // ClarityName("tuple") out first
    format_key_value(settings, &exprs[1..], "")
}

// This should panic on most things besides atoms and values. Added this to help
// debugging in the meantime
fn display_pse(
    settings: &Settings,
    pse: &PreSymbolicExpression,
    previous_indentation: &str,
) -> String {
    match pse.pre_expr {
        PreSymbolicExpressionType::Atom(ref value) => {
            // println!("atom: {}", value.as_str());
            value.as_str().trim().to_string()
        }
        PreSymbolicExpressionType::AtomValue(ref value) => {
            // println!("atomvalue: {}", value);
            value.to_string()
        }
        PreSymbolicExpressionType::List(ref items) => {
            format_list(settings, items, previous_indentation)
            // items.iter().map(display_pse).collect::<Vec<_>>().join(" ")
        }
        PreSymbolicExpressionType::Tuple(ref items) => {
            // println!("tuple: {:?}", items);
            format_key_value_sugar(settings, items, previous_indentation)
            // items.iter().map(display_pse).collect::<Vec<_>>().join(", ")
        }
        PreSymbolicExpressionType::SugaredContractIdentifier(ref name) => name.to_string(),
        PreSymbolicExpressionType::SugaredFieldIdentifier(ref contract, ref field) => {
            format!("{}.{}", contract, field)
        }
        PreSymbolicExpressionType::FieldIdentifier(ref trait_id) => {
            // println!("field id: {}", trait_id);
            trait_id.to_string()
        }
        PreSymbolicExpressionType::TraitReference(ref name) => name.to_string(),
        PreSymbolicExpressionType::Comment(ref text) => {
            format!(";; {}\n", text)
        }
        PreSymbolicExpressionType::Placeholder(ref _placeholder) => {
            "".to_string() // Placeholder is for if parsing fails
        }
    }
}

// * functions

// Top level define-<function> should have a line break above and after (except on first line)
// options always on new lines
// Functions Always on multiple lines, even if short
fn format_function(settings: &Settings, exprs: &[PreSymbolicExpression]) -> String {
    let func_type = display_pse(settings, exprs.first().unwrap(), "");
    let indentation = &settings.indentation.to_string();

    let mut acc = format!("({func_type} (");

    // function name and arguments
    if let Some(def) = exprs.get(1).and_then(|f| f.match_list()) {
        if let Some((name, args)) = def.split_first() {
            acc.push_str(&display_pse(settings, name, ""));
            for arg in args.iter() {
                if let Some(list) = arg.match_list() {
                    acc.push_str(&format!(
                        "\n{}{}({})",
                        indentation,
                        indentation,
                        format_source_exprs(settings, list, &settings.indentation.to_string(), "")
                    ))
                } else {
                    acc.push_str(&format_source_exprs(
                        settings,
                        &[arg.clone()],
                        &settings.indentation.to_string(),
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
    for expr in exprs.get(2..).unwrap_or_default() {
        acc.push_str(&format!(
            "\n{}{}",
            indentation,
            format_source_exprs(
                settings,
                &[expr.clone()],
                &settings.indentation.to_string(),
                ""
            )
        ))
    }
    acc.push_str("\n)\n\n");
    acc.to_owned()
}

fn indentation_to_string(indentation: &Indentation) -> String {
    match indentation {
        Indentation::Space(i) => " ".repeat(*i),
        Indentation::Tab => "\t".to_string(),
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
        assert_eq!(result, "(define-private (my-func)\n  (ok true)\n)");
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
)"#;
        assert_eq!(expected, result);
    }
    #[test]
    fn test_function_args_multiline() {
        let src = "(define-public (my-func (amount uint) (sender principal)) (ok true))";
        let result = format_with_default(&String::from(src));
        assert_eq!(
            result,
            "(define-public (my-func\n    (amount uint)\n    (sender principal)\n  )\n  (ok true)\n)"
        );
    }
    #[test]
    fn test_pre_comments_included() {
        let src = ";; this is a pre comment\n(ok true)";
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
        let src = "(ok true)\n;; this is a post comment";
        let result = format_with_default(&String::from(src));
        assert_eq!(src, result);
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
    fn test_map() {
        let src = "(define-map a uint {n1: (buff 20)})";
        let result = format_with_default(&String::from(src));
        assert_eq!(result, "(define-map a\n  uint\n  { n1: (buff 20) }\n)");
        let src = "(define-map something { name: (buff 48), a: uint } uint)";
        let result = format_with_default(&String::from(src));
        assert_eq!(
            result,
            "(define-map something\n  {\n    name: (buff 48),\n    a: uint\n  }\n  uint\n)"
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
        let expected = "(match opt\n  value (ok (handle-new-value value))\n  (ok 1)\n)";
        assert_eq!(result, expected);
    }
    #[test]
    fn test_response_match() {
        let src = "(match x value (ok (+ to-add value)) err-value (err err-value))";
        let result = format_with_default(&String::from(src));
        let expected = "(match x\n  value (ok (+ to-add value))\n  err-value (err err-value)\n)";
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
        let src = "(define-constant something 1)";
        let result = format_with_default(&String::from(src));
        assert_eq!(result, "(define-constant something 1)");
        let src2 = "(define-constant something (1 2))";
        let result2 = format_with_default(&String::from(src2));
        assert_eq!(result2, "(define-constant something\n  (1 2)\n)");
    }

    #[test]
    fn test_begin_never_one_line() {
        let src = "(begin (ok true))";
        let result = format_with_default(&String::from(src));
        assert_eq!(result, "(begin\n  (ok true)\n)");
    }

    #[test]
    fn test_begin() {
        let src = "(begin (+ 1 1) (ok true))";
        let result = format_with_default(&String::from(src));
        assert_eq!(result, "(begin\n  (+ 1 1)\n  (ok true)\n)");
    }

    #[test]
    fn test_custom_tab_setting() {
        let src = "(begin (ok true))";
        let result = format_with(&String::from(src), Settings::new(Indentation::Space(4), 80));
        assert_eq!(result, "(begin\n    (ok true)\n)");
    }

    // #[test]
    // fn test_irl_contracts() {
    //     let golden_dir = "./tests/golden";
    //     let intended_dir = "./tests/golden-intended";

    //     // Iterate over files in the golden directory
    //     for entry in fs::read_dir(golden_dir).expect("Failed to read golden directory") {
    //         let entry = entry.expect("Failed to read directory entry");
    //         let path = entry.path();

    //         if path.is_file() {
    //             let src = fs::read_to_string(&path).expect("Failed to read source file");

    //             let file_name = path.file_name().expect("Failed to get file name");
    //             let intended_path = Path::new(intended_dir).join(file_name);

    //             let intended =
    //                 fs::read_to_string(&intended_path).expect("Failed to read intended file");

    //             // Apply formatting and compare
    //             let result = format_with_default(&src);
    //             assert_eq!(result, intended, "Mismatch for file: {:?}", file_name);
    //         }
    //     }
    // }
}
