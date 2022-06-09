use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt;
use std::iter::FromIterator;

use crate::clarity::events::StacksTransactionEvent;

use crate::clarity::costs::{cost_functions, runtime_cost};

use crate::clarity::analysis::errors::CheckErrors;
use crate::clarity::contexts::ContractContext;
use crate::clarity::costs::cost_functions::ClarityCostFunction;
use crate::clarity::errors::{check_argument_count, Error, InterpreterResult as Result};
use crate::clarity::representations::{ClarityName, SymbolicExpression};
use crate::clarity::types::Value::UInt;
use crate::clarity::types::{
    FunctionType, PrincipalData, QualifiedContractIdentifier, TraitIdentifier, TypeSignature,
};
use crate::clarity::{eval, Environment, LocalContext, Value};

pub enum CallableType {
    UserFunction(DefinedFunction),
    NativeFunction(&'static str, NativeHandle, ClarityCostFunction),
    SpecialFunction(
        &'static str,
        &'static dyn Fn(&[SymbolicExpression], &mut Environment, &LocalContext) -> Result<Value>,
    ),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DefineType {
    ReadOnly,
    Public,
    Private,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DefinedFunction {
    identifier: FunctionIdentifier,
    pub name: ClarityName,
    pub arg_types: Vec<TypeSignature>,
    pub define_type: DefineType,
    pub arguments: Vec<ClarityName>,
    pub body: SymbolicExpression,
}

pub enum NativeHandle {
    SingleArg(&'static dyn Fn(Value) -> Result<Value>),
    DoubleArg(&'static dyn Fn(Value, Value) -> Result<Value>),
    MoreArg(&'static dyn Fn(Vec<Value>) -> Result<Value>),
}

impl NativeHandle {
    pub fn apply(&self, mut args: Vec<Value>) -> Result<Value> {
        match self {
            NativeHandle::SingleArg(function) => {
                check_argument_count(1, &args)?;
                function(args.pop().unwrap())
            }
            NativeHandle::DoubleArg(function) => {
                check_argument_count(2, &args)?;
                let second = args.pop().unwrap();
                let first = args.pop().unwrap();
                function(first, second)
            }
            NativeHandle::MoreArg(function) => function(args),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct FunctionIdentifier {
    pub identifier: String,
}

impl fmt::Display for FunctionIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.identifier)
    }
}

impl DefinedFunction {
    pub fn new(
        mut arguments: Vec<(ClarityName, TypeSignature)>,
        body: SymbolicExpression,
        define_type: DefineType,
        name: &ClarityName,
        context_name: &str,
    ) -> DefinedFunction {
        let (argument_names, types) = arguments.drain(..).unzip();

        DefinedFunction {
            identifier: FunctionIdentifier::new_user_function(name, context_name),
            name: name.clone(),
            arguments: argument_names,
            define_type,
            body,
            arg_types: types,
        }
    }

    pub fn execute_apply(&self, args: &[Value], env: &mut Environment) -> Result<Value> {
        runtime_cost(
            ClarityCostFunction::UserFunctionApplication,
            env,
            self.arguments.len(),
        )?;

        for arg_type in self.arg_types.iter() {
            runtime_cost(
                ClarityCostFunction::InnerTypeCheckCost,
                env,
                arg_type.size(),
            )?;
        }

        let mut context = LocalContext::new();
        if args.len() != self.arguments.len() {
            Err(CheckErrors::IncorrectArgumentCount(
                self.arguments.len(),
                args.len(),
            ))?
        }

        let mut arg_iterator: Vec<_> = self
            .arguments
            .iter()
            .zip(self.arg_types.iter())
            .zip(args.iter())
            .collect();

        for arg in arg_iterator.drain(..) {
            let ((name, type_sig), value) = arg;

            match (type_sig, value) {
                (
                    TypeSignature::TraitReferenceType(trait_identifier),
                    Value::Principal(PrincipalData::Contract(callee_contract_id)),
                ) => {
                    // Argument is a trait reference, probably leading to a dynamic contract call
                    // We keep a reference of the mapping (var-name: (callee_contract_id, trait_id)) in the context.
                    // The code fetching and checking the trait is implemented in the contract_call eval function.
                    context.callable_contracts.insert(
                        name.clone(),
                        (callee_contract_id.clone(), trait_identifier.clone()),
                    );
                }
                _ => {
                    if !type_sig.admits(value) {
                        return Err(
                            CheckErrors::TypeValueError(type_sig.clone(), value.clone()).into()
                        );
                    }
                    if let Some(_) = context.variables.insert(name.clone(), value.clone()) {
                        return Err(CheckErrors::NameAlreadyUsed(name.to_string()).into());
                    }
                }
            }
        }

        let result = eval(&self.body, env, &context);

        // if the error wasn't actually an error, but a function return,
        //    pull that out and return it.
        match result {
            Ok(r) => Ok(r),
            Err(e) => match e {
                Error::ShortReturn(v) => Ok(v.into()),
                _ => Err(e),
            },
        }
    }

    pub fn check_trait_expectations(
        &self,
        contract_defining_trait: &ContractContext,
        trait_identifier: &TraitIdentifier,
    ) -> Result<()> {
        let trait_name = trait_identifier.name.to_string();
        let constraining_trait = contract_defining_trait
            .lookup_trait_definition(&trait_name)
            .ok_or(CheckErrors::TraitReferenceUnknown(trait_name.to_string()))?;
        let expected_sig =
            constraining_trait
                .get(&self.name)
                .ok_or(CheckErrors::TraitMethodUnknown(
                    trait_name.to_string(),
                    self.name.to_string(),
                ))?;

        let args = self.arg_types.iter().map(|a| a.clone()).collect();
        if !expected_sig.check_args_trait_compliance(args) {
            return Err(
                CheckErrors::BadTraitImplementation(trait_name, self.name.to_string()).into(),
            );
        }

        Ok(())
    }

    pub fn is_read_only(&self) -> bool {
        self.define_type == DefineType::ReadOnly
    }

    pub fn apply(&self, args: &[Value], env: &mut Environment) -> Result<Value> {
        match self.define_type {
            DefineType::Private => self.execute_apply(args, env),
            DefineType::Public => env.execute_function_as_transaction(self, args, None),
            DefineType::ReadOnly => env.execute_function_as_transaction(self, args, None),
        }
    }

    pub fn is_public(&self) -> bool {
        match self.define_type {
            DefineType::Public => true,
            DefineType::Private => false,
            DefineType::ReadOnly => true,
        }
    }

    pub fn get_identifier(&self) -> FunctionIdentifier {
        self.identifier.clone()
    }
}

impl CallableType {
    pub fn get_identifier(&self) -> FunctionIdentifier {
        match self {
            CallableType::UserFunction(f) => f.get_identifier(),
            CallableType::NativeFunction(s, _, _) => FunctionIdentifier::new_native_function(s),
            CallableType::SpecialFunction(s, _) => FunctionIdentifier::new_native_function(s),
        }
    }
}

impl FunctionIdentifier {
    fn new_native_function(name: &str) -> FunctionIdentifier {
        let identifier = format!("_native_:{}", name);
        FunctionIdentifier {
            identifier: identifier,
        }
    }

    fn new_user_function(name: &str, context: &str) -> FunctionIdentifier {
        let identifier = format!("{}:{}", context, name);
        FunctionIdentifier {
            identifier: identifier,
        }
    }
}
