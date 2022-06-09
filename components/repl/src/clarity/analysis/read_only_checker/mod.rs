use crate::clarity::analysis::types::{AnalysisPass, ContractAnalysis};
use crate::clarity::functions::define::DefineFunctionsParsed;
use crate::clarity::functions::NativeFunctions;
use crate::clarity::representations::SymbolicExpressionType::{
    Atom, AtomValue, Field, List, LiteralValue, TraitReference,
};
use crate::clarity::representations::{ClarityName, SymbolicExpression, SymbolicExpressionType};
use crate::clarity::types::{PrincipalData, Value};

use crate::clarity::variables::NativeVariables;
use std::collections::HashMap;

pub use super::errors::{CheckError, CheckResult};
use crate::clarity::analysis::AnalysisDatabase;
pub use crate::clarity::errors::{check_argument_count, check_arguments_at_least, CheckErrors};

pub struct ReadOnlyChecker<'a, 'b> {
    db: &'a mut AnalysisDatabase<'b>,
    defined_functions: HashMap<ClarityName, bool>,
}

impl<'a, 'b> AnalysisPass for ReadOnlyChecker<'a, 'b> {
    fn run_pass(
        contract_analysis: &mut ContractAnalysis,
        analysis_db: &mut AnalysisDatabase,
    ) -> CheckResult<()> {
        let mut command = ReadOnlyChecker::new(analysis_db);
        command.run(contract_analysis)?;
        Ok(())
    }
}

impl<'a, 'b> ReadOnlyChecker<'a, 'b> {
    fn new(db: &'a mut AnalysisDatabase<'b>) -> ReadOnlyChecker<'a, 'b> {
        Self {
            db,
            defined_functions: HashMap::new(),
        }
    }

    pub fn run(&mut self, contract_analysis: &mut ContractAnalysis) -> CheckResult<()> {
        for exp in contract_analysis.expressions.iter() {
            let mut result = self.check_reads_only_valid(&exp);
            if let Err(ref mut error) = result {
                if !error.has_expression() {
                    error.set_expression(&exp);
                }
            }
            result?
        }

        Ok(())
    }

    fn check_define_function(
        &mut self,
        signature: &[SymbolicExpression],
        body: &SymbolicExpression,
    ) -> CheckResult<(ClarityName, bool)> {
        let function_name = signature
            .get(0)
            .ok_or(CheckErrors::DefineFunctionBadSignature)?
            .match_atom()
            .ok_or(CheckErrors::BadFunctionName)?;

        let is_read_only = self.check_read_only(body)?;

        Ok((function_name.clone(), is_read_only))
    }

    fn check_reads_only_valid(&mut self, expr: &SymbolicExpression) -> CheckResult<()> {
        use crate::clarity::functions::define::DefineFunctionsParsed::*;
        if let Some(define_type) = DefineFunctionsParsed::try_parse(expr)? {
            match define_type {
                // The _arguments_ to Constant, PersistedVariable, FT defines must be checked to ensure that
                //   any _evaluated arguments_ supplied to them are valid with respect to read-only requirements.
                Constant { value, .. } => {
                    self.check_read_only(value)?;
                }
                PersistedVariable { initial, .. } => {
                    self.check_read_only(initial)?;
                }
                BoundedFungibleToken { max_supply, .. } => {
                    // only the *optional* total supply arg is eval'ed
                    self.check_read_only(max_supply)?;
                }
                PrivateFunction { signature, body } | PublicFunction { signature, body } => {
                    let (f_name, is_read_only) = self.check_define_function(signature, body)?;
                    self.defined_functions.insert(f_name, is_read_only);
                }
                ReadOnlyFunction { signature, body } => {
                    let (f_name, is_read_only) = self.check_define_function(signature, body)?;
                    if !is_read_only {
                        return Err(CheckErrors::WriteAttemptedInReadOnly.into());
                    } else {
                        self.defined_functions.insert(f_name, is_read_only);
                    }
                }
                Map { .. } | NonFungibleToken { .. } | UnboundedFungibleToken { .. } => {
                    // No arguments to (define-map ...) or (define-non-fungible-token) or fungible tokens without max supplies are eval'ed.
                }
                Trait { .. } | UseTrait { .. } | ImplTrait { .. } => {
                    // No arguments to (use-trait ...), (define-trait ...). or (impl-trait) are eval'ed.
                }
            }
        } else {
            self.check_read_only(expr)?;
        }
        Ok(())
    }

    /// Checks the supplied symbolic expressions
    ///   (1) for whether or not they are valid with respect to read-only requirements.
    ///   (2) if valid, returns whether or not they are read only.
    /// Note that because of (1), this function _cannot_ short-circuit on read-only.
    fn check_read_only(&mut self, expr: &SymbolicExpression) -> CheckResult<bool> {
        match expr.expr {
            AtomValue(_) | LiteralValue(_) | Atom(_) | TraitReference(_, _) | Field(_) => Ok(true),
            List(ref expression) => self.check_function_application_read_only(expression),
        }
    }

    /// Checks all of the supplied symbolic expressions
    ///   (1) for whether or not they are valid with respect to read-only requirements.
    ///   (2) if valid, returns whether or not they are read only.
    /// Note that because of (1), this function _cannot_ short-circuit on read-only.
    fn check_all_read_only(&mut self, expressions: &[SymbolicExpression]) -> CheckResult<bool> {
        let mut result = true;
        for expr in expressions.iter() {
            let expr_read_only = self.check_read_only(expr)?;
            result = result && expr_read_only;
        }
        Ok(result)
    }

    fn try_native_function_check(
        &mut self,
        function: &str,
        args: &[SymbolicExpression],
    ) -> Option<CheckResult<bool>> {
        NativeFunctions::lookup_by_name(function)
            .map(|function| self.check_native_function(&function, args))
    }

    fn check_native_function(
        &mut self,
        function: &NativeFunctions,
        args: &[SymbolicExpression],
    ) -> CheckResult<bool> {
        use crate::clarity::functions::NativeFunctions::*;

        match function {
            Add | Subtract | Divide | Multiply | CmpGeq | CmpLeq | CmpLess | CmpGreater
            | Modulo | Power | Sqrti | Log2 | BitwiseXOR | And | Or | Not | Hash160 | Sha256
            | Keccak256 | Equals | If | Sha512 | Sha512Trunc256 | Secp256k1Recover
            | Secp256k1Verify | ConsSome | ConsOkay | ConsError | DefaultTo | UnwrapRet
            | UnwrapErrRet | IsOkay | IsNone | Asserts | Unwrap | UnwrapErr | Match | IsErr
            | IsSome | TryRet | ToUInt | ToInt | Append | Concat | AsMaxLen | ContractOf
            | PrincipalOf | ListCons | GetBlockInfo | TupleGet | TupleMerge | Len | Print
            | AsContract | Begin | FetchVar | GetStxBalance | GetTokenBalance | GetAssetOwner
            | GetTokenSupply | ElementAt | IndexOf => self.check_all_read_only(args),
            AtBlock => {
                check_argument_count(2, args)?;

                let is_block_arg_read_only = self.check_read_only(&args[0])?;
                let closure_read_only = self.check_read_only(&args[1])?;
                if !closure_read_only {
                    return Err(CheckErrors::AtBlockClosureMustBeReadOnly.into());
                }
                Ok(is_block_arg_read_only)
            }
            FetchEntry => {
                check_argument_count(2, args)?;
                self.check_all_read_only(args)
            }
            StxTransfer | StxBurn | SetEntry | DeleteEntry | InsertEntry | SetVar | MintAsset
            | MintToken | TransferAsset | TransferToken | BurnAsset | BurnToken => {
                self.check_all_read_only(args)?;
                Ok(false)
            }
            Let => {
                check_arguments_at_least(2, args)?;

                let binding_list = args[0].match_list().ok_or(CheckErrors::BadLetSyntax)?;

                for pair in binding_list.iter() {
                    let pair_expression = pair.match_list().ok_or(CheckErrors::BadSyntaxBinding)?;
                    if pair_expression.len() != 2 {
                        return Err(CheckErrors::BadSyntaxBinding.into());
                    }

                    if !self.check_read_only(&pair_expression[1])? {
                        return Ok(false);
                    }
                }

                self.check_all_read_only(&args[1..args.len()])
            }
            Map => {
                check_arguments_at_least(2, args)?;

                // note -- we do _not_ check here to make sure we're not mapping on
                //      a special function. that check is performed by the type checker.
                //   we're pretty directly violating type checks in this recursive step:
                //   we're asking the read only checker to check whether a function application
                //     of the _mapping function_ onto the rest of the supplied arguments would be
                //     read-only or not.
                self.check_function_application_read_only(args)
            }
            Filter => {
                check_argument_count(2, args)?;
                self.check_function_application_read_only(args)
            }
            Fold => {
                check_argument_count(3, args)?;

                // note -- we do _not_ check here to make sure we're not folding on
                //      a special function. that check is performed by the type checker.
                //   we're pretty directly violating type checks in this recursive step:
                //   we're asking the read only checker to check whether a function application
                //     of the _folding function_ onto the rest of the supplied arguments would be
                //     read-only or not.
                self.check_function_application_read_only(args)
            }
            TupleCons => {
                for pair in args.iter() {
                    let pair_expression =
                        pair.match_list().ok_or(CheckErrors::TupleExpectsPairs)?;
                    if pair_expression.len() != 2 {
                        return Err(CheckErrors::TupleExpectsPairs.into());
                    }

                    if !self.check_read_only(&pair_expression[1])? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            ContractCall => {
                check_arguments_at_least(2, args)?;

                let function_name = args[1]
                    .match_atom()
                    .ok_or(CheckErrors::ContractCallExpectName)?;

                let is_function_read_only = match &args[0].expr {
                    SymbolicExpressionType::LiteralValue(Value::Principal(
                        PrincipalData::Contract(ref contract_identifier),
                    )) => self
                        .db
                        .get_read_only_function_type(&contract_identifier, function_name)?
                        .is_some(),
                    SymbolicExpressionType::Atom(_trait_reference) => {
                        // Dynamic dispatch from a readonly-function can only be guaranteed at runtime,
                        // which would defeat granting a static readonly stamp.
                        // As such dynamic dispatch is currently forbidden.
                        false
                    }
                    _ => return Err(CheckError::new(CheckErrors::ContractCallExpectName)),
                };

                self.check_all_read_only(&args[2..])
                    .map(|args_read_only| args_read_only && is_function_read_only)
            }
        }
    }

    fn check_function_application_read_only(
        &mut self,
        expression: &[SymbolicExpression],
    ) -> CheckResult<bool> {
        let (function_name, args) = expression
            .split_first()
            .ok_or(CheckErrors::NonFunctionApplication)?;

        let function_name = function_name
            .match_atom()
            .ok_or(CheckErrors::NonFunctionApplication)?;

        if let Some(mut result) = self.try_native_function_check(function_name, args) {
            if let Err(ref mut check_err) = result {
                check_err.set_expressions(expression);
            }
            result
        } else {
            let is_function_read_only = self
                .defined_functions
                .get(function_name)
                .ok_or(CheckErrors::UnknownFunction(function_name.to_string()))?
                .clone();
            self.check_all_read_only(args)
                .map(|args_read_only| args_read_only && is_function_read_only)
        }
    }
}
