use std::convert::TryInto;
use crate::clarity::representations::{PreSymbolicExpression, PreSymbolicExpressionType, SymbolicExpression, SymbolicExpressionType, ClarityName};
use crate::clarity::types::{QualifiedContractIdentifier, Value, PrincipalData, StandardPrincipalData, TraitIdentifier};
use crate::clarity::ast::types::{ContractAST, BuildASTPass, PreExpressionsDrain};
use crate::clarity::ast::errors::{ParseResult, ParseError, ParseErrors};
use crate::clarity::functions::NativeFunctions;
use std::collections::{HashMap, HashSet};

pub struct SugarExpander {
    issuer: StandardPrincipalData,
    defined_traits: HashSet<ClarityName>,
    imported_traits: HashMap<ClarityName, TraitIdentifier>,
}

impl BuildASTPass for SugarExpander {

    fn run_pass(contract_ast: &mut ContractAST) -> ParseResult<()> {
        let pass = SugarExpander::new(contract_ast.contract_identifier.issuer.clone());
        pass.run(contract_ast)?;
        Ok(())
    }
}

impl SugarExpander {

    fn new(issuer: StandardPrincipalData) -> Self {
        Self { 
            issuer,
            defined_traits: HashSet::new(),
            imported_traits: HashMap::new(),
     }
    }

    pub fn run(&self, contract_ast: &mut ContractAST) -> ParseResult<()> {
        let expressions = self.transform(contract_ast.pre_expressions_drain(), contract_ast)?;
        contract_ast.expressions = expressions;
        Ok(())
    }

    pub fn transform(&self, pre_exprs_iter: PreExpressionsDrain, contract_ast: &mut ContractAST) -> ParseResult<Vec<SymbolicExpression>> {
        let mut expressions = Vec::new();

        for pre_expr in pre_exprs_iter {
            let mut expr = match pre_expr.pre_expr {
                PreSymbolicExpressionType::AtomValue(content) => {
                    SymbolicExpression::literal_value(content)
                },
                PreSymbolicExpressionType::Atom(content) => {
                    SymbolicExpression::atom(content)
                },
                PreSymbolicExpressionType::List(pre_exprs) => {
                    let drain = PreExpressionsDrain::new(pre_exprs.to_vec().drain(..), None);
                    let expression = self.transform(drain, contract_ast)?;
                    SymbolicExpression::list(expression.into_boxed_slice())
                },
                PreSymbolicExpressionType::Tuple(pre_exprs) => {
                    let drain = PreExpressionsDrain::new(pre_exprs.to_vec().drain(..), None);
                    let expression = self.transform(drain, contract_ast)?;
                    let mut pairs = expression.chunks(2)
                                     .map(|pair| pair.to_vec().into_boxed_slice())
                                     .map(SymbolicExpression::list)
                                     .collect::<Vec<_>>();
                    pairs.insert(0, SymbolicExpression::atom("tuple".to_string().try_into().unwrap()));
                    SymbolicExpression::list(pairs.into_boxed_slice())
                },
                PreSymbolicExpressionType::SugaredContractIdentifier(contract_name) => {
                    let contract_identifier = QualifiedContractIdentifier::new(self.issuer.clone(), contract_name);
                    SymbolicExpression::literal_value(Value::Principal(PrincipalData::Contract(contract_identifier)))
                },
                PreSymbolicExpressionType::SugaredFieldIdentifier(contract_name, name) => {
                    let contract_identifier = QualifiedContractIdentifier::new(self.issuer.clone(), contract_name);
                    SymbolicExpression::field(TraitIdentifier { name, contract_identifier})
                },
                PreSymbolicExpressionType::FieldIdentifier(trait_identifier) => {
                    SymbolicExpression::field(trait_identifier)
                },
                PreSymbolicExpressionType::TraitReference(name) => {
                    if let Some(trait_reference) = contract_ast.get_referenced_trait(&name) {
                        SymbolicExpression::trait_reference(name, trait_reference.clone())
                    }  else {
                        return Err(ParseErrors::TraitReferenceUnknown(name.to_string()).into())
                    }                    
                },
            };
            // expr.id will be set by the subsequent expression identifier pass.
            expr.span = pre_expr.span.clone();
            expressions.push(expr);
        }
        Ok(expressions)
    }
}
