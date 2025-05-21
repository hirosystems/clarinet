// This module provides a transpiler that converts TypeScript into Clarity code.
// 1. It parses the TypeScript code into an AST using `swc`
// 2. Transform the AST into a Clarity AST
// traverses the AST to extract relevant information

use clarity::vm::representations::PreSymbolicExpression;
use swc_common::sync::Lrc;
use swc_common::{FileName, SourceMap};
use swc_ecma_ast::{CallExpr, Function, Module};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};
use swc_ecma_visit::{Visit, VisitWith};

#[derive(Debug)]
pub struct AstVisitor {
    function_names: Vec<String>,
    function_calls: Vec<String>,
    data_vars: Vec<(String, String, String)>, // (name, type, initial_value)
}

// Implement the Visit trait
impl Visit for AstVisitor {
    fn visit_var_decl(&mut self, var: &swc_ecma_ast::VarDecl) {
        for decl in &var.decls {
            if let Some(init) = &decl.init {
                if let Some(id) = &decl.name.as_ident() {
                    let var_name = id.sym.to_string();

                    // Try to extract type information
                    let type_ann = id
                        .type_ann
                        .as_ref()
                        .map(|ann| {
                            // Try to extract the type as a string
                            if let swc_ecma_ast::TsType::TsTypeRef(type_ref) = &*ann.type_ann {
                                if let swc_ecma_ast::TsEntityName::Ident(type_ident) =
                                    &type_ref.type_name
                                {
                                    type_ident.sym.to_string()
                                } else {
                                    "unknown".to_string()
                                }
                            } else {
                                "unknown".to_string()
                            }
                        })
                        .unwrap_or_else(|| "unknown".to_string());

                    // Check for DataVar instantiation
                    if let swc_ecma_ast::Expr::New(new_expr) = &**init {
                        if let Some(ident) = new_expr.callee.as_ident() {
                            if ident.sym == "DataVar" {
                                // Get type argument if present
                                let type_arg = if let Some(type_args) = &new_expr.type_args {
                                    if let Some(first) = type_args.params.first() {
                                        match &**first {
                                            swc_ecma_ast::TsType::TsTypeRef(type_ref) => {
                                                if let swc_ecma_ast::TsEntityName::Ident(
                                                    type_ident,
                                                ) = &type_ref.type_name
                                                {
                                                    type_ident.sym.to_string()
                                                } else {
                                                    type_ann.clone()
                                                }
                                            }
                                            _ => type_ann.clone(),
                                        }
                                    } else {
                                        type_ann.clone()
                                    }
                                } else {
                                    type_ann.clone()
                                };
                                // Get initial value
                                let initial_value = if let Some(arg) =
                                    new_expr.args.as_ref().and_then(|args| args.first())
                                {
                                    match &*arg.expr {
                                        swc_ecma_ast::Expr::Lit(swc_ecma_ast::Lit::Num(num)) => {
                                            if type_arg == "Int" {
                                                format!("{}", num.value as i64)
                                            } else if type_arg == "Uint" {
                                                format!("u{}", num.value as u64)
                                            } else {
                                                panic!("Unknown type for number")
                                            }
                                        }
                                        swc_ecma_ast::Expr::Lit(swc_ecma_ast::Lit::Str(s)) => {
                                            s.value.to_string()
                                        }
                                        _ => "unknown".to_string(),
                                    }
                                } else {
                                    "unknown".to_string()
                                };
                                self.data_vars.push((var_name, type_arg, initial_value));
                            }
                        }
                    }

                    // Check initialization for potential type info
                }
            }
        }
        var.visit_children_with(self);
    }

    fn visit_expr(&mut self, expr: &swc_ecma_ast::Expr) {
        if let swc_ecma_ast::Expr::New(new_expr) = expr {
            if let Some(ident) = new_expr.callee.as_ident() {
                if ident.sym == "DataVar" {}
            }
        }
        expr.visit_children_with(self);
    }

    fn visit_module_item(&mut self, item: &swc_ecma_ast::ModuleItem) {
        if let swc_ecma_ast::ModuleItem::Stmt(swc_ecma_ast::Stmt::Decl(swc_ecma_ast::Decl::Var(
            var_decl,
        ))) = item
        {
            if var_decl.kind == swc_ecma_ast::VarDeclKind::Const {
                for decl in &var_decl.decls {
                    if let Some(init) = &decl.init {
                        if let swc_ecma_ast::Expr::New(new_expr) = &**init {
                            if let Some(ident) = new_expr.callee.as_ident() {
                                if ident.sym == "DataVar" {
                                    if let Some(_var_ident) = &decl.name.as_ident() {}
                                }
                            }
                        }
                    }
                }
            }
        }
        item.visit_children_with(self);
    }

    fn visit_fn_decl(&mut self, node: &swc_ecma_ast::FnDecl) {
        let _name = node.ident.sym.to_string();
        self.function_names.push(node.ident.sym.to_string());

        node.visit_children_with(self);
    }

    fn visit_function(&mut self, function: &Function) {
        function.visit_children_with(self);
    }

    fn visit_call_expr(&mut self, call_expr: &CallExpr) {
        if let Some(ident) = call_expr.callee.as_expr().and_then(|expr| expr.as_ident()) {
            self.function_calls.push(ident.sym.to_string());
        }

        call_expr.visit_children_with(self);
    }
}

fn parse_ts(file_name: &str, src: &str) -> Result<Module, swc_ecma_parser::error::Error> {
    let cm: Lrc<SourceMap> = Default::default();

    let fm = cm.new_source_file(FileName::Custom(file_name.into()).into(), src.into());
    let lexer = Lexer::new(
        // We want to parse ecmascript
        Syntax::Typescript(Default::default()),
        // EsVersion defaults to es5
        Default::default(),
        StringInput::from(&*fm),
        None,
    );

    let mut parser = Parser::new_from(lexer);

    parser.parse_module()
}

fn visit_ts_ast(module: Module) -> Result<AstVisitor, swc_ecma_parser::error::Error> {
    let mut visitor = AstVisitor {
        function_names: Vec::new(),
        function_calls: Vec::new(),
        data_vars: Vec::new(),
    };

    // Walk the AST
    module.visit_with(&mut visitor);
    Ok(visitor)
}

fn ast_to_clarity(
    visitor: AstVisitor,
) -> Result<Vec<PreSymbolicExpression>, swc_ecma_parser::error::Error> {
    let mut result = Vec::new();
    for (name, typ, value) in visitor.data_vars {
        // Map both Int and Uint to Clarity's uint
        let typ_lower = typ.to_lowercase();
        let clarity_type = typ_lower.as_str();
        let clarity_code = format!("(define-data-var {} {} {})", name, clarity_type, value);
        let exprs = clarity::vm::ast::parser::v2::parse(&clarity_code).unwrap();
        result.extend(exprs);
    }
    Ok(result)
}

pub fn transpile(
    file_name: &str,
    src: &str,
) -> Result<Vec<PreSymbolicExpression>, swc_ecma_parser::error::Error> {
    let module = parse_ts(file_name, src)?;
    let visitor = visit_ts_ast(module)?;
    ast_to_clarity(visitor)
}

#[cfg(test)]
mod test {
    use crate::{parse_ts, transpile};
    use clarity::vm::ast::parser::v2::parse as clarity_parse;

    #[track_caller]
    fn simple_source_check(ts_source: &str, expected_clarity_output: &str) {
        let expected_output = clarity_parse(expected_clarity_output).unwrap();
        let result = transpile("test.clar.ts", ts_source);
        assert_eq!(result, Ok(expected_output));
    }

    #[test]
    fn can_transpile() {
        let file_name = "test.js";
        let src = "function test() { return 42; }\n export default { readOnly: { test } };";

        let _result = parse_ts(file_name, src);
    }

    #[test]
    fn can_parse_data_var() {
        simple_source_check(
            "const count = new DataVar<Uint>(0);",
            "(define-data-var count uint u0)",
        );
    }

    #[test]
    fn can_parse_data_with_basic_types() {
        simple_source_check(
            "const count = new DataVar<Int>(1);",
            "(define-data-var count int 1)",
        );
    }
}
