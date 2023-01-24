use std::string;

use clarity::vm::{
    functions::{define::DefineFunctions, NativeFunctions},
    representations::depth_traverse,
    ClarityName, SymbolicExpression,
};

pub fn format_function(exprs: &[SymbolicExpression]) -> String {
    let func_type = exprs.first().unwrap();
    let name_and_args = exprs.get(1).and_then(|f| f.match_list()).unwrap();
    let mut func_acc = format!("\n({func_type} ({})", format(name_and_args, ""));

    for arg in exprs.get(2..).unwrap_or_default() {
        if let Some(list) = arg.match_list() {
            func_acc.push_str(&format!("\n  ({})", format(list, "")))
        }
    }
    func_acc.push_str("\n)\n");
    func_acc.to_owned()
}

pub fn format_begin(exprs: &[SymbolicExpression]) -> String {
    let mut begin_acc = "(begin\n".to_string();

    for arg in exprs.get(1..).unwrap_or_default() {
        if let Some(list) = arg.match_list() {
            begin_acc.push_str(&format!("\n  ({})", format(list, "")))
        }
    }
    begin_acc.push_str("\n)\n");
    begin_acc.to_owned()
}

pub fn format_tuple(exprs: &[SymbolicExpression]) -> String {
    let mut tuple_acc = "{".to_string();
    // println!("> tuple_acc: {:?}", &expr);
    for expr in exprs.get(1..).unwrap() {
        println!("> tuple_acc: {:?}", &expr);
        let (key, value) = expr
            .match_list()
            .and_then(|list| list.split_first())
            .unwrap();
        tuple_acc.push_str(&format!("\n  {key}: {},", format(value, "").trim()));
    }
    tuple_acc.push_str("\n}");
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

            return format!("{formatted} {}", format(remaining, acc).trim());
        }

        return format!("{} {}", expr.to_string(), format(remaining, acc).trim());
    };

    acc.trim().to_owned()
}

#[cfg(test)]
mod test_format {
    use clarity::types::StacksEpochId;
    use clarity::vm::representations::depth_traverse;
    use clarity::vm::{
        ast::{build_ast_with_rules, ASTRules},
        types::QualifiedContractIdentifier,
        ClarityVersion, SymbolicExpression,
    };

    use super::format;

    fn get_ast(source: &str) -> Vec<SymbolicExpression> {
        let contract_ast = build_ast_with_rules(
            &QualifiedContractIdentifier::transient(),
            source,
            &mut (),
            ClarityVersion::Clarity2,
            StacksEpochId::Epoch21,
            ASTRules::PrecheckSize,
        )
        .unwrap();

        return contract_ast.expressions;
    }

    #[test]
    fn test_simple_function() {
        let expressions = get_ast("(define-public (print-arg (a uint)) (ok (print a)))");

        let output = format(&expressions, "");
        println!("output\n{:}", &output);
    }

    #[test]

    fn test_simple_tuple() {
        let expressions = get_ast("(define-map mymap { id: principal } { items: uint }) (map-insert mymap { id: tx-sender } { items: u0 })");

        let output = format(&expressions, "");
        println!("output\n{:}", &output);
    }

    #[test]
    fn test_simple_let() {
        let expressions = get_ast("(let ((a u1) (b u2) (c (+ a b))) (ok c))");

        let output = format(&expressions, "");
        println!("output\n{:}", &output);
    }

    #[test]
    fn test_two_functions() {
        let expressions =
            get_ast("(define-private (func1) (print u1))(define-private (func2) (print u2))");

        let output = format(&expressions, "");
        println!("output\n{:}", &output);
    }

    #[test]
    fn test_let_bindings() {
        let expressions = get_ast(
            r#"(define-public (vote (pick (string-ascii 6)))
  (begin
    (asserts! (< block-height VOTE_END) VOTE_ENDED)
    (asserts! (is-none (map-get? votes tx-sender)) FORBIDDEN)
    (asserts! (or (is-eq pick "apple") (is-eq pick "orange")) INVALID_CHOICE)

    (if (is-eq pick "apple")
      (var-set apple-votes (+ (var-get apple-votes) u1))
      (var-set orange-votes (+ (var-get orange-votes) u1))
    )

    (map-insert votes tx-sender pick)

    (ok true)
  )
)"#,
        );

        let output = format(&expressions, "");
        println!("output\n{:}", &output);
    }
}
