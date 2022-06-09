use crate::clarity::contexts::{Environment, LocalContext};
use crate::clarity::costs::{cost_functions::ClarityCostFunction, runtime_cost};
use crate::clarity::errors::{InterpreterResult as Result, RuntimeErrorType};
use crate::clarity::types::Value;
use std::convert::TryFrom;

define_named_enum!(NativeVariables {
    ContractCaller("contract-caller"), TxSender("tx-sender"), BlockHeight("block-height"),
    BurnBlockHeight("burn-block-height"), NativeNone("none"),
    NativeTrue("true"), NativeFalse("false"),
    TotalLiquidMicroSTX("stx-liquid-supply"),
    Regtest("is-in-regtest"),
});

pub fn is_reserved_name(name: &str) -> bool {
    NativeVariables::lookup_by_name(name).is_some()
}

pub fn lookup_reserved_variable(
    name: &str,
    _context: &LocalContext,
    env: &mut Environment,
) -> Result<Option<Value>> {
    if let Some(variable) = NativeVariables::lookup_by_name(name) {
        match variable {
            NativeVariables::TxSender => {
                let sender = env
                    .sender
                    .clone()
                    .ok_or(RuntimeErrorType::NoSenderInContext)?;
                Ok(Some(Value::Principal(sender)))
            }
            NativeVariables::ContractCaller => {
                let sender = env
                    .caller
                    .clone()
                    .ok_or(RuntimeErrorType::NoSenderInContext)?;
                Ok(Some(Value::Principal(sender)))
            }
            NativeVariables::BlockHeight => {
                runtime_cost(ClarityCostFunction::FetchVar, env, 1)?;
                let block_height = env.global_context.database.get_current_block_height();
                Ok(Some(Value::UInt(block_height as u128)))
            }
            NativeVariables::BurnBlockHeight => {
                runtime_cost(ClarityCostFunction::FetchVar, env, 1)?;
                let burn_block_height = env
                    .global_context
                    .database
                    .get_current_burnchain_block_height();
                Ok(Some(Value::UInt(burn_block_height as u128)))
            }
            NativeVariables::NativeNone => Ok(Some(Value::none())),
            NativeVariables::NativeTrue => Ok(Some(Value::Bool(true))),
            NativeVariables::NativeFalse => Ok(Some(Value::Bool(false))),
            NativeVariables::TotalLiquidMicroSTX => {
                runtime_cost(ClarityCostFunction::FetchVar, env, 1)?;
                let liq = env.global_context.database.get_total_liquid_ustx();
                Ok(Some(Value::UInt(liq)))
            }
            NativeVariables::Regtest => {
                let reg = true;
                Ok(Some(Value::Bool(reg)))
            }
        }
    } else {
        Ok(None)
    }
}
