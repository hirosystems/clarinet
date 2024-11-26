use clarity::types::StacksEpochId;
use clarity::vm::ast::{build_ast_with_rules, ASTRules};
use clarity::vm::functions::{define::DefineFunctions, NativeFunctions};
use clarity::vm::representations::{PreSymbolicExpression, PreSymbolicExpressionType};
use clarity::vm::types::QualifiedContractIdentifier;
use clarity::vm::{ClarityVersion, SymbolicExpression};

pub enum Indentation {
    Space(usize),
    Tab,
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
        format_source_exprs(&self.settings, &pse, "")
    }
}

pub fn format_source_exprs(
    settings: &Settings,
    expressions: &[PreSymbolicExpression],
    acc: &str,
) -> String {
    if let Some((expr, remaining)) = expressions.split_first() {
        if let Some(list) = expr.match_list() {
            // println!("{:?}", list);
            if let Some(atom_name) = list.split_first().and_then(|(f, _)| f.match_atom()) {
                let formatted = if let Some(native) = NativeFunctions::lookup_by_name(atom_name) {
                    match native {
                        NativeFunctions::Let => format_let(settings, list),
                        NativeFunctions::Begin => format_begin(settings, list),
                        NativeFunctions::Match => format_match(settings, list),
                        NativeFunctions::TupleCons => format_tuple(settings, list),
                        NativeFunctions::ListCons => format_list(settings, list),
                        _ => format!("({})", format_source_exprs(settings, list, acc)),
                    }
                } else if let Some(define) = DefineFunctions::lookup_by_name(atom_name) {
                    match define {
                        DefineFunctions::PublicFunction
                        | DefineFunctions::ReadOnlyFunction
                        | DefineFunctions::PrivateFunction => format_function(settings, list),
                        DefineFunctions::Constant => format_constant(settings, list),
                        DefineFunctions::UseTrait => format_use_trait(settings, list),
                        DefineFunctions::Trait => format_trait(settings, list),
                        DefineFunctions::Map => format_map(settings, list),
                        DefineFunctions::ImplTrait => format_impl_trait(settings, list),
                        // DefineFunctions::PersistedVariable
                        // DefineFunctions::FungibleToken
                        // DefineFunctions::NonFungibleToken
                        _ => format!("({})", format_source_exprs(settings, list, acc)),
                    }
                } else {
                    format!("({})", format_source_exprs(settings, list, acc))
                };

                return format!(
                    "{formatted}{}",
                    format_source_exprs(settings, remaining, acc)
                )
                .trim()
                .to_owned();
            }
        }
        return format!(
            "{} {}",
            display_pse(expr),
            format_source_exprs(settings, remaining, acc)
        )
        .trim()
        .to_owned();
    };
    acc.to_owned()
}

fn format_use_trait(_settings: &Settings, _exprs: &[PreSymbolicExpression]) -> String {
    "".to_string()
}
fn format_impl_trait(_settings: &Settings, _exprs: &[PreSymbolicExpression]) -> String {
    "".to_string()
}
fn format_trait(_settings: &Settings, _exprs: &[PreSymbolicExpression]) -> String {
    "".to_string()
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

// fn format_constant(settings: &Settings, exprs: &[PreSymbolicExpression]) -> String {
//     let indentation = indentation_to_string(&settings.indentation);
//     let mut acc = "(define-constant ".to_string();

//     if let Some((name, args)) = exprs
//         .get(1)
//         .and_then(|f| f.match_list())
//         .and_then(|list| list.split_first())
//     {
//         acc.push_str(&display_pse(name));

//         for arg in args {
//             if let Some(list) = arg.match_list() {
//                 acc.push_str(&format!(
//                     "\n{}({})",
//                     indentation,
//                     format_source_exprs(settings, list, "")
//                 ));
//             }
//         }
//         acc.push_str("\n)\n");
//         acc.to_owned()
//     } else {
//         panic!("Expected a valid constant definition")
//     }
// }

fn format_constant(settings: &Settings, exprs: &[PreSymbolicExpression]) -> String {
    let indentation = indentation_to_string(&settings.indentation);
    let mut acc = "(define-constant ".to_string();

    if let Some((name, args)) = name_and_args(exprs) {
        acc.push_str(&format!("{} ", display_pse(name)));

        // Access the value from args
        if let Some(value) = args.first() {
            if let Some(list) = value.match_list() {
                acc.push_str(&format!(
                    "\n{}({})",
                    indentation,
                    format_source_exprs(settings, list, "")
                ));
            } else {
                // Handle non-list values (e.g., literals or simple expressions)
                acc.push_str(&display_pse(value));
            }
        }

        acc.push_str("\n)\n");
        acc.to_owned()
    } else {
        panic!("Expected a valid constant definition with (name value)")
    }
}
fn format_map(settings: &Settings, exprs: &[PreSymbolicExpression]) -> String {
    let indentation = indentation_to_string(&settings.indentation);
    let mut acc = "(define-map (".to_string();
    let name_and_args = exprs.get(1).and_then(|f| f.match_list()).unwrap();

    if let Some((name, args)) = name_and_args.split_first() {
        acc.push_str(&format!("{}", display_pse(name)));

        for arg in args {
            if let Some(list) = arg.match_list() {
                acc.push_str(&format!(
                    "\n{}{}",
                    indentation,
                    format_source_exprs(settings, list, "")
                ))
            }
        }
        acc.push_str("\n)\n");
        acc.to_owned()
    } else {
        String::new() // this is likely an error or unreachable
    }
}
// *begin* never on one line
fn format_begin(settings: &Settings, exprs: &[PreSymbolicExpression]) -> String {
    let mut begin_acc = "(begin".to_string();
    let indentation = indentation_to_string(&settings.indentation);

    for arg in exprs.get(1..).unwrap_or_default() {
        if let Some(list) = arg.match_list() {
            begin_acc.push_str(&format!(
                "\n{}({})",
                indentation,
                format_source_exprs(settings, list, "")
            ))
        }
    }
    begin_acc.push_str("\n)\n");
    begin_acc.to_owned()
}

// *let* never on one line
fn format_let(settings: &Settings, exprs: &[PreSymbolicExpression]) -> String {
    let mut begin_acc = "(let (".to_string();
    let indentation = indentation_to_string(&settings.indentation);
    for arg in exprs.get(1..).unwrap_or_default() {
        if let Some(list) = arg.match_list() {
            begin_acc.push_str(&format!(
                "\n{}({})",
                indentation,
                format_source_exprs(settings, list, "")
            ))
        }
    }
    begin_acc.push_str("\n)  \n");
    begin_acc.to_owned()
}

// * match *
// One line if less than max length (unless the original source has line breaks?)
// Multiple lines if more than max length (should the first arg be on the first line if it fits?)
fn format_match(settings: &Settings, exprs: &[PreSymbolicExpression]) -> String {
    println!("{:?}", exprs);
    let mut acc = "(match ".to_string();
    let indentation = indentation_to_string(&settings.indentation);

    if let Some((name, args)) = name_and_args(exprs) {
        acc.push_str(&display_pse(name));
        for arg in args.get(1..).unwrap_or_default() {
            if let Some(list) = arg.match_list() {
                acc.push_str(&format!(
                    "\n{}({})",
                    indentation,
                    format_source_exprs(settings, list, "")
                ))
            }
        }
        acc.push_str("\n)");
        acc.to_owned()
    } else {
        "".to_string()
    }
}

fn format_list(settings: &Settings, exprs: &[PreSymbolicExpression]) -> String {
    println!("here");
    let mut acc = "(".to_string();
    for (i, expr) in exprs[1..].iter().enumerate() {
        let value = format_source_exprs(settings, &[expr.clone()], "");
        if i < exprs.len() - 2 {
            acc.push_str(&format!("{value} "));
        } else {
            acc.push_str(&value.to_string());
        }
    }
    acc.push(')');
    acc.to_string()
}

fn format_tuple(settings: &Settings, exprs: &[PreSymbolicExpression]) -> String {
    let mut tuple_acc = "{ ".to_string();
    for (i, expr) in exprs[1..].iter().enumerate() {
        let (key, value) = expr
            .match_list()
            .and_then(|list| list.split_first())
            .unwrap();
        let fkey = display_pse(key);
        if i < exprs.len() - 2 {
            tuple_acc.push_str(&format!(
                "{fkey}: {}, ",
                format_source_exprs(settings, value, "")
            ));
        } else {
            tuple_acc.push_str(&format!(
                "{fkey}: {}",
                format_source_exprs(settings, value, "")
            ));
        }
    }
    tuple_acc.push_str(" }");
    tuple_acc.to_string()
}

// This should panic on most things besides atoms and values. Added this to help
// debugging in the meantime
fn display_pse(pse: &PreSymbolicExpression) -> String {
    match pse.pre_expr {
        PreSymbolicExpressionType::Atom(ref value) => {
            println!("atom: {}", value.as_str());
            value.as_str().trim().to_string()
        }
        PreSymbolicExpressionType::AtomValue(ref value) => {
            println!("atomvalue: {}", value);
            value.to_string()
        }
        PreSymbolicExpressionType::List(ref items) => {
            println!("list: {:?}", items);
            items.iter().map(display_pse).collect::<Vec<_>>().join(" ")
        }
        PreSymbolicExpressionType::Tuple(ref items) => {
            println!("tuple: {:?}", items);
            items.iter().map(display_pse).collect::<Vec<_>>().join(", ")
        }
        PreSymbolicExpressionType::SugaredContractIdentifier(ref name) => name.to_string(),
        PreSymbolicExpressionType::SugaredFieldIdentifier(ref contract, ref field) => {
            format!("{}.{}", contract, field)
        }
        PreSymbolicExpressionType::FieldIdentifier(ref trait_id) => {
            println!("field id: {}", trait_id);
            trait_id.to_string()
        }
        PreSymbolicExpressionType::TraitReference(ref name) => name.to_string(),
        PreSymbolicExpressionType::Comment(ref text) => {
            // println!("comment: {}", text);
            format!(";; {}\n", text.trim())
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
    let func_type = display_pse(exprs.first().unwrap());
    let indentation = indentation_to_string(&settings.indentation);

    let mut acc = format!("({func_type} (");

    // function name and arguments
    if let Some(def) = exprs.get(1).and_then(|f| f.match_list()) {
        if let Some((name, args)) = def.split_first() {
            acc.push_str(&display_pse(name));
            for arg in args.iter() {
                if let Some(list) = arg.match_list() {
                    acc.push_str(&format!(
                        "\n{}{}({})",
                        indentation,
                        indentation,
                        format_source_exprs(settings, list, "")
                    ))
                } else {
                    acc.push_str(&display_pse(arg))
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
        if let Some(list) = expr.match_list() {
            acc.push_str(&format!(
                "\n{}({})",
                indentation,
                format_source_exprs(settings, list, "")
            ))
        }
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
    fn test_two_expr_formatter() {
        let result = format_with_default(&String::from("(ok true)(ok true)"));
        assert_eq!(result, "(ok true)\n(ok true)");
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
            "(define-public (my-func\n    (amount uint)\n    (sender principal)\n  )\n  (ok true)\n)"
        );
    }
    #[test]
    fn test_pre_postcomments_included() {
        let src = ";; this is a pre comment\n(ok true)";

        let result = format_with_default(&String::from(src));
        assert_eq!(src, result);

        let src = "(ok true)\n;; this is a post comment";

        let result = format_with_default(&String::from(src));
        assert_eq!(src, result);
    }
    #[test]
    fn test_end_of_line_comments_included() {
        let src = "(ok true) ;; this is a comment";

        let result = format_with_default(&String::from(src));
        assert_eq!(src, result);
    }

    #[test]
    fn test_map() {
        let src = "(define-map something { name: (buff 48), namespace: (buff 20) } uint\n)";
        let result = format_with_default(&String::from(src));
        assert_eq!(result, "(define-map something\n  uint\n  uint)");
        //         let src2 = "(define-map something { name: (buff 48), namespace: (buff 20) } uint\n)";
        //         let result2 = format_with_default(&String::from(src2));
        //         let expected2 = r#"(define-map something
        //   {
        //     name: (buff 48),
        //     namespace: (buff 20)
        //   }
        //   uint
        // )"#;
        //         assert_eq!(result2, expected2);
    }
    // #[test]
    // fn test_end_of_line_comments_max_line_length() {
    //     let src = "(ok true) ;; this is a comment";

    //     let result = format_with(&String::from(src), Settings::new(Indentation::Space(2), 9));
    //     let expected = ";; this is a comment\n(ok true)";
    //     assert_eq!(result, expected);
    // }
    #[test]
    fn test_begin_never_one_line() {
        let src = "(begin (ok true))";
        let result = format_with_default(&String::from(src));
        assert_eq!(result, "(begin\n  (ok true)\n)");
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
