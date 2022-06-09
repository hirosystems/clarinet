use std::cmp;
use std::convert::{TryFrom, TryInto};

use crate::clarity::functions::tuples;

use crate::clarity::callables::DefineType;
use crate::clarity::costs::cost_functions::ClarityCostFunction;
use crate::clarity::costs::{
    constants as cost_constants, cost_functions, runtime_cost, CostTracker, MemoryConsumer,
};
use crate::clarity::errors::{
    check_argument_count, check_arguments_at_least, CheckErrors, InterpreterError,
    InterpreterResult as Result, RuntimeErrorType,
};
use crate::clarity::representations::{SymbolicExpression, SymbolicExpressionType};
use crate::clarity::types::{
    BlockInfoProperty, BuffData, OptionalData, PrincipalData, SequenceData, TypeSignature, Value,
    BUFF_32,
};
use crate::clarity::StacksBlockId;
use crate::clarity::{eval, Environment, LocalContext};

pub fn special_contract_call(
    args: &[SymbolicExpression],
    env: &mut Environment,
    context: &LocalContext,
) -> Result<Value> {
    check_arguments_at_least(2, args)?;

    // the second part of the contract_call cost (i.e., the load contract cost)
    //   is checked in `execute_contract`, and the function _application_ cost
    //   is checked in callables::DefinedFunction::execute_apply.
    runtime_cost(ClarityCostFunction::ContractCall, env, 0)?;

    let function_name = args[1].match_atom().ok_or(CheckErrors::ExpectedName)?;
    let mut rest_args = vec![];
    let mut rest_args_sizes = vec![];
    for arg in args[2..].iter() {
        let evaluated_arg = eval(arg, env, context)?;
        rest_args_sizes.push(evaluated_arg.size() as u64);
        rest_args.push(SymbolicExpression::atom_value(evaluated_arg));
    }

    let (contract_identifier, type_returns_constraint) = match &args[0].expr {
        SymbolicExpressionType::LiteralValue(Value::Principal(PrincipalData::Contract(
            ref contract_identifier,
        ))) => {
            // Static dispatch
            (contract_identifier, None)
        }
        SymbolicExpressionType::Atom(contract_ref) => {
            // Dynamic dispatch
            match context.lookup_callable_contract(contract_ref) {
                Some((ref contract_identifier, trait_identifier)) => {
                    // Ensure that contract-call is used for inter-contract calls only
                    if *contract_identifier == env.contract_context.contract_identifier {
                        return Err(CheckErrors::CircularReference(vec![contract_identifier
                            .name
                            .to_string()])
                        .into());
                    }

                    let contract_to_check = env
                        .global_context
                        .database
                        .get_contract(contract_identifier)
                        .map_err(|_e| {
                            CheckErrors::NoSuchContract(contract_identifier.to_string())
                        })?;
                    let contract_context_to_check = contract_to_check.contract_context;

                    // Attempt to short circuit the dynamic dispatch checks:
                    // If the contract is explicitely implementing the trait with `impl-trait`,
                    // then we can simply rely on the analysis performed at publish time.
                    if contract_context_to_check.is_explicitly_implementing_trait(&trait_identifier)
                    {
                        (contract_identifier, None)
                    } else {
                        let trait_name = trait_identifier.name.to_string();

                        // Retrieve, from the trait definition, the expected method signature
                        let contract_defining_trait = env
                            .global_context
                            .database
                            .get_contract(&trait_identifier.contract_identifier)
                            .map_err(|_e| {
                                CheckErrors::NoSuchContract(
                                    trait_identifier.contract_identifier.to_string(),
                                )
                            })?;
                        let contract_context_defining_trait =
                            contract_defining_trait.contract_context;

                        // Retrieve the function that will be invoked
                        let function_to_check = contract_context_to_check
                            .lookup_function(function_name)
                            .ok_or(CheckErrors::BadTraitImplementation(
                                trait_name.clone(),
                                function_name.to_string(),
                            ))?;

                        // Check read/write compatibility
                        if env.global_context.is_read_only() {
                            return Err(CheckErrors::TraitBasedContractCallInReadOnly.into());
                        }

                        // Check visibility
                        if function_to_check.define_type == DefineType::Private {
                            return Err(CheckErrors::NoSuchPublicFunction(
                                contract_identifier.to_string(),
                                function_name.to_string(),
                            )
                            .into());
                        }

                        function_to_check.check_trait_expectations(
                            &contract_context_defining_trait,
                            &trait_identifier,
                        )?;

                        // Retrieve the expected method signature
                        let constraining_trait = contract_context_defining_trait
                            .lookup_trait_definition(&trait_name)
                            .ok_or(CheckErrors::TraitReferenceUnknown(trait_name.clone()))?;
                        let expected_sig = constraining_trait.get(function_name).ok_or(
                            CheckErrors::TraitMethodUnknown(trait_name, function_name.to_string()),
                        )?;
                        (contract_identifier, Some(expected_sig.returns.clone()))
                    }
                }
                _ => return Err(CheckErrors::ContractCallExpectName.into()),
            }
        }
        _ => return Err(CheckErrors::ContractCallExpectName.into()),
    };

    let contract_principal = env.contract_context.contract_identifier.clone().into();

    let mut nested_env = env.nest_with_caller(contract_principal);
    let result = if nested_env.short_circuit_contract_call(
        &contract_identifier,
        function_name,
        &rest_args_sizes,
    )? {
        nested_env.run_free(|free_env| {
            free_env.execute_contract(&contract_identifier, function_name, &rest_args, false)
        })
    } else {
        nested_env.execute_contract(&contract_identifier, function_name, &rest_args, false)
    }?;

    // Ensure that the expected type from the trait spec admits
    // the type of the value returned by the dynamic dispatch.
    if let Some(returns_type_signature) = type_returns_constraint {
        let actual_returns = TypeSignature::type_of(&result);
        if !returns_type_signature.admits_type(&actual_returns) {
            return Err(
                CheckErrors::ReturnTypesMustMatch(returns_type_signature, actual_returns).into(),
            );
        }
    }

    Ok(result)
}

pub fn special_fetch_variable(
    args: &[SymbolicExpression],
    env: &mut Environment,
    _context: &LocalContext,
) -> Result<Value> {
    check_argument_count(1, args)?;

    let var_name = args[0].match_atom().ok_or(CheckErrors::ExpectedName)?;

    let contract = &env.contract_context.contract_identifier;

    let data_types = env
        .contract_context
        .meta_data_var
        .get(var_name)
        .ok_or(CheckErrors::NoSuchDataVariable(var_name.to_string()))?;

    runtime_cost(
        ClarityCostFunction::FetchVar,
        env,
        data_types.value_type.size(),
    )?;

    env.global_context
        .database
        .lookup_variable(contract, var_name, data_types)
}

pub fn special_set_variable(
    args: &[SymbolicExpression],
    env: &mut Environment,
    context: &LocalContext,
) -> Result<Value> {
    if env.global_context.is_read_only() {
        return Err(CheckErrors::WriteAttemptedInReadOnly.into());
    }

    check_argument_count(2, args)?;

    let value = eval(&args[1], env, &context)?;

    let var_name = args[0].match_atom().ok_or(CheckErrors::ExpectedName)?;

    let contract = &env.contract_context.contract_identifier;

    let data_types = env
        .contract_context
        .meta_data_var
        .get(var_name)
        .ok_or(CheckErrors::NoSuchDataVariable(var_name.to_string()))?;

    runtime_cost(
        ClarityCostFunction::SetVar,
        env,
        data_types.value_type.size(),
    )?;

    env.add_memory(value.get_memory_use())?;

    env.global_context
        .database
        .set_variable(contract, var_name, value, data_types)
}

pub fn special_fetch_entry(
    args: &[SymbolicExpression],
    env: &mut Environment,
    context: &LocalContext,
) -> Result<Value> {
    check_argument_count(2, args)?;

    let map_name = args[0].match_atom().ok_or(CheckErrors::ExpectedName)?;

    let key = eval(&args[1], env, &context)?;

    let contract = &env.contract_context.contract_identifier;

    let data_types = env
        .contract_context
        .meta_data_map
        .get(map_name)
        .ok_or(CheckErrors::NoSuchMap(map_name.to_string()))?;

    runtime_cost(
        ClarityCostFunction::FetchEntry,
        env,
        data_types.value_type.size() + data_types.key_type.size(),
    )?;

    env.global_context
        .database
        .fetch_entry(contract, map_name, &key, data_types)
}

pub fn special_at_block(
    args: &[SymbolicExpression],
    env: &mut Environment,
    context: &LocalContext,
) -> Result<Value> {
    check_argument_count(2, args)?;

    runtime_cost(ClarityCostFunction::AtBlock, env, 0)?;

    let bhh = match eval(&args[0], env, &context)? {
        Value::Sequence(SequenceData::Buffer(BuffData { data })) => {
            if data.len() != 32 {
                return Err(RuntimeErrorType::BadBlockHash(data).into());
            } else {
                StacksBlockId::from(data.as_slice())
            }
        }
        x => return Err(CheckErrors::TypeValueError(BUFF_32.clone(), x).into()),
    };

    env.add_memory(cost_constants::AT_BLOCK_MEMORY)?;
    let result = env.evaluate_at_block(bhh, &args[1], context);
    env.drop_memory(cost_constants::AT_BLOCK_MEMORY);

    result
}

pub fn special_set_entry(
    args: &[SymbolicExpression],
    env: &mut Environment,
    context: &LocalContext,
) -> Result<Value> {
    if env.global_context.is_read_only() {
        return Err(CheckErrors::WriteAttemptedInReadOnly.into());
    }

    check_argument_count(3, args)?;

    let key = eval(&args[1], env, &context)?;

    let value = eval(&args[2], env, &context)?;

    let map_name = args[0].match_atom().ok_or(CheckErrors::ExpectedName)?;

    let contract = &env.contract_context.contract_identifier;

    let data_types = env
        .contract_context
        .meta_data_map
        .get(map_name)
        .ok_or(CheckErrors::NoSuchMap(map_name.to_string()))?;

    runtime_cost(
        ClarityCostFunction::SetEntry,
        env,
        data_types.value_type.size() + data_types.key_type.size(),
    )?;

    env.add_memory(key.get_memory_use())?;
    env.add_memory(value.get_memory_use())?;

    env.global_context
        .database
        .set_entry(contract, map_name, key, value, data_types)
}

pub fn special_insert_entry(
    args: &[SymbolicExpression],
    env: &mut Environment,
    context: &LocalContext,
) -> Result<Value> {
    if env.global_context.is_read_only() {
        return Err(CheckErrors::WriteAttemptedInReadOnly.into());
    }

    check_argument_count(3, args)?;

    let key = eval(&args[1], env, &context)?;

    let value = eval(&args[2], env, &context)?;

    let map_name = args[0].match_atom().ok_or(CheckErrors::ExpectedName)?;

    let contract = &env.contract_context.contract_identifier;

    let data_types = env
        .contract_context
        .meta_data_map
        .get(map_name)
        .ok_or(CheckErrors::NoSuchMap(map_name.to_string()))?;

    runtime_cost(
        ClarityCostFunction::SetEntry,
        env,
        data_types.value_type.size() + data_types.key_type.size(),
    )?;

    env.add_memory(key.get_memory_use())?;
    env.add_memory(value.get_memory_use())?;

    env.global_context
        .database
        .insert_entry(contract, map_name, key, value, data_types)
}

pub fn special_delete_entry(
    args: &[SymbolicExpression],
    env: &mut Environment,
    context: &LocalContext,
) -> Result<Value> {
    if env.global_context.is_read_only() {
        return Err(CheckErrors::WriteAttemptedInReadOnly.into());
    }

    check_argument_count(2, args)?;

    let key = eval(&args[1], env, &context)?;

    let map_name = args[0].match_atom().ok_or(CheckErrors::ExpectedName)?;

    let contract = &env.contract_context.contract_identifier;

    let data_types = env
        .contract_context
        .meta_data_map
        .get(map_name)
        .ok_or(CheckErrors::NoSuchMap(map_name.to_string()))?;

    runtime_cost(
        ClarityCostFunction::SetEntry,
        env,
        data_types.key_type.size(),
    )?;

    env.add_memory(key.get_memory_use())?;

    env.global_context
        .database
        .delete_entry(contract, map_name, &key, data_types)
}

pub fn special_get_block_info(
    args: &[SymbolicExpression],
    env: &mut Environment,
    context: &LocalContext,
) -> Result<Value> {
    // (get-block-info? property-name block-height-int)
    runtime_cost(ClarityCostFunction::BlockInfo, env, 0)?;

    check_argument_count(2, args)?;

    // Handle the block property name input arg.
    let property_name = args[0]
        .match_atom()
        .ok_or(CheckErrors::GetBlockInfoExpectPropertyName)?;

    let block_info_prop = BlockInfoProperty::lookup_by_name(property_name)
        .ok_or(CheckErrors::GetBlockInfoExpectPropertyName)?;

    // Handle the block-height input arg clause.
    let height_eval = eval(&args[1], env, context)?;
    let height_value = match height_eval {
        Value::UInt(result) => Ok(result),
        x => Err(CheckErrors::TypeValueError(TypeSignature::UIntType, x)),
    }?;

    let height_value = match u32::try_from(height_value) {
        Ok(result) => result,
        _ => return Ok(Value::none()),
    };

    let current_block_height = env.global_context.database.get_current_block_height();
    if height_value >= current_block_height {
        return Ok(Value::none());
    }

    let result = match block_info_prop {
        BlockInfoProperty::Time => {
            let block_time = env.global_context.database.get_block_time(height_value);
            Value::UInt(block_time as u128)
        }
        BlockInfoProperty::VrfSeed => {
            let vrf_seed = env.global_context.database.get_block_vrf_seed(height_value);
            Value::Sequence(SequenceData::Buffer(BuffData {
                data: vrf_seed.as_bytes().to_vec(),
            }))
        }
        BlockInfoProperty::HeaderHash => {
            let header_hash = env
                .global_context
                .database
                .get_block_header_hash(height_value);
            Value::Sequence(SequenceData::Buffer(BuffData {
                data: header_hash.as_bytes().to_vec(),
            }))
        }
        BlockInfoProperty::BurnchainHeaderHash => {
            let burnchain_header_hash = env
                .global_context
                .database
                .get_burnchain_block_header_hash(height_value);
            Value::Sequence(SequenceData::Buffer(BuffData {
                data: burnchain_header_hash.as_bytes().to_vec(),
            }))
        }
        BlockInfoProperty::IdentityHeaderHash => {
            let id_header_hash = env
                .global_context
                .database
                .get_index_block_header_hash(height_value);
            Value::Sequence(SequenceData::Buffer(BuffData {
                data: id_header_hash.as_bytes().to_vec(),
            }))
        }
        BlockInfoProperty::MinerAddress => {
            let miner_address = env.global_context.database.get_miner_address(height_value);
            Value::from(miner_address)
        }
    };

    Ok(Value::some(result)?)
}
