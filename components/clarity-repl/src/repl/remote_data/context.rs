use clarity::types::StacksEpochId;
use clarity::vm::callables::{DefineType, DefinedFunction};
use clarity::vm::contexts::GlobalContext;
use clarity::vm::costs::LimitedCostTracker;
use clarity::vm::database::{
    DataMapMetadata, DataVariableMetadata, FungibleTokenMetadata, NonFungibleTokenMetadata,
};
use clarity::vm::errors::{CheckErrors, InterpreterResult as Result};
use clarity::vm::functions::define::DefineFunctionsParsed;
use clarity::vm::types::{parse_name_type_pairs, TypeSignature};
use clarity::vm::{ClarityName, ContractContext, SymbolicExpression, Value};

#[allow(clippy::result_large_err)]
fn handle_function(
    epoch_id: &StacksEpochId,
    signature: &[SymbolicExpression],
    body: SymbolicExpression,
    define_type: DefineType,
    context_name: &str,
    cost_tracker: &mut LimitedCostTracker,
) -> Result<(ClarityName, DefinedFunction)> {
    let (function_symbol, arg_symbols) = signature
        .split_first()
        .ok_or(CheckErrors::DefineFunctionBadSignature)?;
    let function_name = function_symbol
        .match_atom()
        .ok_or(CheckErrors::ExpectedName)?;
    let arguments = parse_name_type_pairs(*epoch_id, arg_symbols, cost_tracker)?;
    Ok((
        function_name.clone(),
        DefinedFunction::new(arguments, body, define_type, function_name, context_name),
    ))
}

#[allow(clippy::result_large_err)]
pub fn set_contract_context(
    expressions: &[SymbolicExpression],
    contract_context: &mut ContractContext,
    global_context: &mut GlobalContext,
) -> Result<()> {
    let context_name = contract_context.contract_identifier.to_string();
    let epoch = global_context.epoch_id;
    let mut ct = LimitedCostTracker::Free;

    for exp in expressions {
        let try_define_exp = DefineFunctionsParsed::try_parse(exp);
        if let Ok(Some(define_exp)) = try_define_exp {
            match define_exp {
                DefineFunctionsParsed::Constant { name, .. } => {
                    contract_context
                        .variables
                        .insert(name.clone(), Value::none());
                }
                DefineFunctionsParsed::PersistedVariable {
                    name, data_type, ..
                } => {
                    contract_context.persisted_names.insert(name.clone());
                    let value_type = TypeSignature::parse_type_repr(epoch, data_type, &mut ct)?;
                    let variable_data = DataVariableMetadata { value_type };
                    contract_context
                        .meta_data_var
                        .insert(name.clone(), variable_data);
                }
                DefineFunctionsParsed::Map {
                    name,
                    key_type,
                    value_type,
                } => {
                    contract_context.persisted_names.insert(name.clone());
                    let key_type = TypeSignature::parse_type_repr(epoch, key_type, &mut ct)?;
                    let value_type = TypeSignature::parse_type_repr(epoch, value_type, &mut ct)?;
                    contract_context.meta_data_map.insert(
                        name.clone(),
                        DataMapMetadata {
                            key_type,
                            value_type,
                        },
                    );
                }
                DefineFunctionsParsed::PrivateFunction { signature, body } => {
                    let (name, func) = handle_function(
                        &epoch,
                        signature,
                        body.clone(),
                        DefineType::Private,
                        &context_name,
                        &mut ct,
                    )?;
                    contract_context.functions.insert(name.clone(), func);
                }
                DefineFunctionsParsed::ReadOnlyFunction { signature, body } => {
                    let (name, func) = handle_function(
                        &epoch,
                        signature,
                        body.clone(),
                        DefineType::ReadOnly,
                        &context_name,
                        &mut ct,
                    )?;
                    contract_context.functions.insert(name.clone(), func);
                }
                DefineFunctionsParsed::PublicFunction { signature, body } => {
                    let (name, func) = handle_function(
                        &epoch,
                        signature,
                        body.clone(),
                        DefineType::Public,
                        &context_name,
                        &mut ct,
                    )?;
                    contract_context.functions.insert(name.clone(), func);
                }
                DefineFunctionsParsed::NonFungibleToken { name, nft_type } => {
                    contract_context.persisted_names.insert(name.clone());
                    let key_type = TypeSignature::parse_type_repr(epoch, nft_type, &mut ct)?;
                    contract_context
                        .meta_nft
                        .insert(name.clone(), NonFungibleTokenMetadata { key_type });
                }
                DefineFunctionsParsed::BoundedFungibleToken { name, .. }
                | DefineFunctionsParsed::UnboundedFungibleToken { name } => {
                    contract_context.persisted_names.insert(name.clone());
                    let data_type = FungibleTokenMetadata { total_supply: None };
                    contract_context.meta_ft.insert(name.clone(), data_type);
                }
                DefineFunctionsParsed::Trait { name, functions } => {
                    let trait_signature = TypeSignature::parse_trait_type_repr(
                        functions,
                        &mut ct,
                        epoch,
                        *contract_context.get_clarity_version(),
                    )?;
                    contract_context
                        .defined_traits
                        .insert(name.clone(), trait_signature);
                }
                DefineFunctionsParsed::ImplTrait { trait_identifier } => {
                    contract_context
                        .implemented_traits
                        .insert(trait_identifier.clone());
                }
                DefineFunctionsParsed::UseTrait { .. } => {}
            }
        }
    }

    Ok(())
}
