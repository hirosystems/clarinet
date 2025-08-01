use clarity::vm::contexts::GlobalContext;
use clarity::vm::database::{
    DataMapMetadata, DataVariableMetadata, FungibleTokenMetadata, NonFungibleTokenMetadata,
};
use clarity::vm::errors::InterpreterResult as Result;
use clarity::vm::functions::define::DefineResult;
use clarity::vm::types::PrincipalData;
use clarity::vm::{
    eval, functions, CallStack, ContractContext, Environment, LocalContext, SymbolicExpression,
    Value,
};

#[allow(clippy::result_large_err)]
pub fn set_contract_context(
    expressions: &[SymbolicExpression],
    contract_context: &mut ContractContext,
    global_context: &mut GlobalContext,
) -> Result<Option<Value>> {
    let mut last_executed = None;
    let context = LocalContext::new();

    // todo: should set contract_context.data_size?

    let publisher: PrincipalData = contract_context.contract_identifier.issuer.clone().into();

    for exp in expressions {
        let try_define = global_context.execute(|g| {
            let mut call_stack = CallStack::new();
            let mut env = Environment::new(
                g,
                contract_context,
                &mut call_stack,
                Some(publisher.clone()),
                Some(publisher.clone()),
                None,
            );
            functions::define::evaluate_define(exp, &mut env)
        })?;
        match try_define {
            DefineResult::Variable(name, value) => {
                contract_context.variables.insert(name, value);
            }
            DefineResult::Function(name, value) => {
                contract_context.functions.insert(name, value);
            }
            DefineResult::PersistedVariable(name, value_type, _value) => {
                contract_context.persisted_names.insert(name.clone());
                let variable_data = DataVariableMetadata { value_type };
                contract_context.meta_data_var.insert(name, variable_data);
            }
            DefineResult::Map(name, key_type, value_type) => {
                contract_context.persisted_names.insert(name.clone());
                let data_type = DataMapMetadata {
                    key_type,
                    value_type,
                };
                contract_context.meta_data_map.insert(name, data_type);
            }
            DefineResult::FungibleToken(name, total_supply) => {
                contract_context.persisted_names.insert(name.clone());
                let data_type = FungibleTokenMetadata { total_supply };
                contract_context.meta_ft.insert(name, data_type);
            }
            DefineResult::NonFungibleAsset(name, asset_type) => {
                contract_context.persisted_names.insert(name.clone());
                let data_type = NonFungibleTokenMetadata {
                    key_type: asset_type.clone(),
                };
                contract_context.meta_nft.insert(name, data_type);
            }
            DefineResult::Trait(name, trait_type) => {
                contract_context.defined_traits.insert(name, trait_type);
            }
            DefineResult::UseTrait(_name, _trait_identifier) => {}
            DefineResult::ImplTrait(trait_identifier) => {
                contract_context.implemented_traits.insert(trait_identifier);
            }
            DefineResult::NoDefine => {
                // not a define function, evaluate normally.
                global_context.execute(|global_context| {
                    let mut call_stack = CallStack::new();
                    let mut env = Environment::new(
                        global_context,
                        contract_context,
                        &mut call_stack,
                        Some(publisher.clone()),
                        Some(publisher.clone()),
                        None,
                    );

                    let result = eval(exp, &mut env, &context)?;
                    last_executed = Some(result);
                    Ok(())
                })?;
            }
        }
    }

    Ok(last_executed)
}
