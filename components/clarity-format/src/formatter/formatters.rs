use clarity_repl::clarity::vm::functions::{define::DefineFunctions, NativeFunctions};
use clarity_repl::clarity::SymbolicExpression;

fn format_function(exprs: &[SymbolicExpression]) -> String {
    let func_type = exprs.first().unwrap();
    let name_and_args = exprs.get(1).and_then(|f| f.match_list()).unwrap();
    let mut func_acc = format!("({func_type} ({})", format(name_and_args, ""));

    for arg in exprs.get(2..).unwrap_or_default() {
        if let Some(list) = arg.match_list() {
            func_acc.push_str(&format!("\n  ({})", format(list, "")))
        }
    }
    func_acc.push_str("\n)");
    func_acc.to_owned()
}

fn format_begin(exprs: &[SymbolicExpression]) -> String {
    let mut begin_acc = "(begin\n".to_string();

    for arg in exprs.get(1..).unwrap_or_default() {
        if let Some(list) = arg.match_list() {
            begin_acc.push_str(&format!("\n  ({})", format(list, "")))
        }
    }
    begin_acc.push_str("\n)\n");
    begin_acc.to_owned()
}

fn format_tuple(exprs: &[SymbolicExpression]) -> String {
    let mut tuple_acc = "{ ".to_string();
    for (i, expr) in exprs[1..].iter().enumerate() {
        let (key, value) = expr
            .match_list()
            .and_then(|list| list.split_first())
            .unwrap();
        if i < exprs.len() - 2 {
            tuple_acc.push_str(&format!("{key}: {}, ", format(value, "")));
        } else {
            tuple_acc.push_str(&format!("{key}: {}", format(value, "")));
        }
    }
    tuple_acc.push_str(" }");
    tuple_acc.to_string()
}

pub fn format(expressions: &[SymbolicExpression], acc: &str) -> String {
    if let Some((expr, remaining)) = expressions.split_first() {
        if let Some(list) = expr.match_list() {
            let atom = list.split_first().and_then(|(f, _)| f.match_atom());
            use NativeFunctions::*;
            let formatted = if let Some(
                DefineFunctions::PublicFunction
                | DefineFunctions::ReadOnlyFunction
                | DefineFunctions::PrivateFunction,
            ) = atom.and_then(|a| DefineFunctions::lookup_by_name(a))
            {
                format_function(&list)
            } else if let Some(Begin) = atom.and_then(|a| NativeFunctions::lookup_by_name(a)) {
                format_begin(&list)
            } else if let Some(TupleCons) = atom.and_then(|a| NativeFunctions::lookup_by_name(a)) {
                format_tuple(&list)
            } else {
                format!("({})", format(list, acc))
            };

            return format!("{formatted} {}", format(remaining, acc))
                .trim()
                .to_owned();
        }

        return format!("{} {}", expr.to_string(), format(remaining, acc))
            .trim()
            .to_owned();
    };

    acc.to_owned()
}
