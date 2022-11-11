use clarity_repl::analysis::ast_visitor::{traverse, ASTVisitor, TypedVar};
use clarity_repl::clarity::vm::analysis::types::ContractAnalysis;
use clarity_repl::clarity::vm::functions::define::DefineFunctions;
use clarity_repl::clarity::vm::functions::NativeFunctions;
use clarity_repl::clarity::vm::representations::SymbolicExpressionType::*;
use clarity_repl::clarity::vm::representations::{Span, TraitDefinition};
use clarity_repl::clarity::vm::types::{TraitIdentifier, TypeSignature, Value};
use clarity_repl::clarity::vm::{ClarityName, ClarityVersion, SymbolicExpression};
use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::hash::{Hash, Hasher};

pub struct Settings {
    pub indentation: Indentation,
}

impl Settings {
    pub fn default() -> Settings {
        Settings {
            indentation: Indentation::Space(2),
        }
    }
}

pub enum Indentation {
    Space(u8),
    Tab,
}

pub struct ClarityLinter {
    pub accumulator: String,
    pub current_indentation_level: u8,
    pub settings: Settings,
}

impl ClarityLinter {
    pub fn new(settings: Settings) -> ClarityLinter {
        Self {
            accumulator: "".into(),
            current_indentation_level: 0,
            settings,
        }
    }

    pub fn run<'a>(mut self, contract_analysis: &'a ContractAnalysis) -> String {
        traverse(&mut self, &contract_analysis.expressions);
        self.accumulator.clone()
    }
}

impl ASTVisitor<'_> for ClarityLinter {
    fn traverse_define_public<'a>(
        &mut self,
        expr: &SymbolicExpression,
        name: &ClarityName,
        parameters: Option<Vec<TypedVar<'a>>>,
        body: &SymbolicExpression,
    ) -> bool {
        self.accumulator
            .push_str(&format!(";; Public function {}\n", name));
        if let Some(parameters) = parameters {
            for parameter in parameters.iter() {
                self.accumulator
                    .push_str(&format!(";; param({}): <comment>\n", parameter.name));
            }
        }
        self.accumulator
            .push_str(&format!("(define-public {} <args>\n", name));
        self.traverse_expr(body);
        self.accumulator.push_str(&format!(")\n"));
        false
    }

    fn visit_define_read_only<'a>(
        &mut self,
        expr: &SymbolicExpression,
        name: &ClarityName,
        parameters: Option<Vec<TypedVar<'a>>>,
        body: &SymbolicExpression,
    ) -> bool {
        true
    }

    fn traverse_define_private<'a>(
        &mut self,
        expr: &SymbolicExpression,
        name: &ClarityName,
        parameters: Option<Vec<TypedVar<'a>>>,
        body: &SymbolicExpression,
    ) -> bool {
        true
    }

    fn traverse_if(
        &mut self,
        expr: &SymbolicExpression,
        cond: &SymbolicExpression,
        then_expr: &SymbolicExpression,
        else_expr: &SymbolicExpression,
    ) -> bool {
        true
    }

    fn traverse_lazy_logical(
        &mut self,
        expr: &SymbolicExpression,
        function: NativeFunctions,
        operands: &[SymbolicExpression],
    ) -> bool {
        true
    }

    fn traverse_let(
        &mut self,
        expr: &SymbolicExpression,
        bindings: &HashMap<&ClarityName, &SymbolicExpression>,
        body: &[SymbolicExpression],
    ) -> bool {
        for (name, val) in bindings {}

        for expr in body {
            if !self.traverse_expr(expr) {
                return false;
            }
        }
        true
    }

    fn traverse_begin(
        &mut self,
        expr: &SymbolicExpression,
        statements: &[SymbolicExpression],
    ) -> bool {
        true
    }

    fn traverse_as_contract(
        &mut self,
        expr: &SymbolicExpression,
        inner: &SymbolicExpression,
    ) -> bool {
        true
    }

    fn visit_asserts(
        &mut self,
        expr: &SymbolicExpression,
        cond: &SymbolicExpression,
        thrown: &SymbolicExpression,
    ) -> bool {
        true
    }

    fn visit_atom(&mut self, expr: &SymbolicExpression, atom: &ClarityName) -> bool {
        true
    }

    fn visit_list(&mut self, expr: &SymbolicExpression, list: &[SymbolicExpression]) -> bool {
        true
    }

    fn visit_stx_burn(
        &mut self,
        expr: &SymbolicExpression,
        amount: &SymbolicExpression,
        sender: &SymbolicExpression,
    ) -> bool {
        true
    }

    fn visit_stx_transfer(
        &mut self,
        expr: &SymbolicExpression,
        amount: &SymbolicExpression,
        sender: &SymbolicExpression,
        recipient: &SymbolicExpression,
        memo: Option<&SymbolicExpression>,
    ) -> bool {
        true
    }

    fn visit_ft_burn(
        &mut self,
        expr: &SymbolicExpression,
        token: &ClarityName,
        amount: &SymbolicExpression,
        sender: &SymbolicExpression,
    ) -> bool {
        true
    }

    fn visit_ft_transfer(
        &mut self,
        expr: &SymbolicExpression,
        token: &ClarityName,
        amount: &SymbolicExpression,
        sender: &SymbolicExpression,
        recipient: &SymbolicExpression,
    ) -> bool {
        true
    }

    fn visit_ft_mint(
        &mut self,
        expr: &SymbolicExpression,
        token: &ClarityName,
        amount: &SymbolicExpression,
        recipient: &SymbolicExpression,
    ) -> bool {
        true
    }

    fn visit_nft_burn(
        &mut self,
        expr: &SymbolicExpression,
        token: &ClarityName,
        identifier: &SymbolicExpression,
        sender: &SymbolicExpression,
    ) -> bool {
        true
    }

    fn visit_nft_transfer(
        &mut self,
        expr: &SymbolicExpression,
        token: &ClarityName,
        identifier: &SymbolicExpression,
        sender: &SymbolicExpression,
        recipient: &SymbolicExpression,
    ) -> bool {
        true
    }

    fn visit_nft_mint(
        &mut self,
        expr: &SymbolicExpression,
        token: &ClarityName,
        identifier: &SymbolicExpression,
        recipient: &SymbolicExpression,
    ) -> bool {
        true
    }

    fn visit_var_set(
        &mut self,
        expr: &SymbolicExpression,
        name: &ClarityName,
        value: &SymbolicExpression,
    ) -> bool {
        true
    }

    fn visit_map_set(
        &mut self,
        expr: &SymbolicExpression,
        name: &ClarityName,
        key: &HashMap<Option<&ClarityName>, &SymbolicExpression>,
        value: &HashMap<Option<&ClarityName>, &SymbolicExpression>,
    ) -> bool {
        true
    }

    fn visit_map_insert(
        &mut self,
        expr: &SymbolicExpression,
        name: &ClarityName,
        key: &HashMap<Option<&ClarityName>, &SymbolicExpression>,
        value: &HashMap<Option<&ClarityName>, &SymbolicExpression>,
    ) -> bool {
        true
    }

    fn visit_map_delete(
        &mut self,
        expr: &SymbolicExpression,
        name: &ClarityName,
        key: &HashMap<Option<&ClarityName>, &SymbolicExpression>,
    ) -> bool {
        true
    }

    fn visit_dynamic_contract_call(
        &mut self,
        expr: &SymbolicExpression,
        trait_ref: &SymbolicExpression,
        function_name: &ClarityName,
        args: &[SymbolicExpression],
    ) -> bool {
        true
    }

    fn visit_call_user_defined(
        &mut self,
        expr: &SymbolicExpression,
        name: &ClarityName,
        args: &[SymbolicExpression],
    ) -> bool {
        true
    }

    fn visit_comparison(
        &mut self,
        expr: &SymbolicExpression,
        func: NativeFunctions,
        operands: &[SymbolicExpression],
    ) -> bool {
        true
    }
}
