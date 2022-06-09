use crate::clarity::functions::tuples;

use crate::clarity::costs::cost_functions::ClarityCostFunction;
use crate::clarity::costs::{cost_functions, runtime_cost, CostTracker};
use crate::clarity::database::structures::STXBalance;
use crate::clarity::errors::{
    check_argument_count, CheckErrors, Error, InterpreterError, InterpreterResult as Result,
    RuntimeErrorType,
};
use crate::clarity::representations::SymbolicExpression;
use crate::clarity::types::{
    AssetIdentifier, BlockInfoProperty, BuffData, OptionalData, PrincipalData, TypeSignature, Value,
};
use crate::clarity::{eval, Environment, LocalContext};
use std::convert::TryFrom;

enum MintAssetErrorCodes {
    ALREADY_EXIST = 1,
}
enum MintTokenErrorCodes {
    NON_POSITIVE_AMOUNT = 1,
}
enum TransferAssetErrorCodes {
    NOT_OWNED_BY = 1,
    SENDER_IS_RECIPIENT = 2,
    DOES_NOT_EXIST = 3,
}
enum TransferTokenErrorCodes {
    NOT_ENOUGH_BALANCE = 1,
    SENDER_IS_RECIPIENT = 2,
    NON_POSITIVE_AMOUNT = 3,
}

enum BurnAssetErrorCodes {
    NOT_OWNED_BY = 1,
    DOES_NOT_EXIST = 3,
}
enum BurnTokenErrorCodes {
    NOT_ENOUGH_BALANCE = 1,
    NON_POSITIVE_AMOUNT = 3,
}

enum StxErrorCodes {
    NOT_ENOUGH_BALANCE = 1,
    SENDER_IS_RECIPIENT = 2,
    NON_POSITIVE_AMOUNT = 3,
    SENDER_IS_NOT_TX_SENDER = 4,
}

macro_rules! clarity_ecode {
    ($thing:expr) => {
        Ok(Value::err_uint($thing as u128))
    };
}

pub fn special_stx_balance(
    args: &[SymbolicExpression],
    env: &mut Environment,
    context: &LocalContext,
) -> Result<Value> {
    check_argument_count(1, args)?;

    runtime_cost(ClarityCostFunction::StxBalance, env, 0)?;

    let owner = eval(&args[0], env, context)?;

    if let Value::Principal(ref principal) = owner {
        let balance = {
            let snapshot = env
                .global_context
                .database
                .get_stx_balance_snapshot(principal);
            snapshot.get_available_balance()
        };
        Ok(Value::UInt(balance))
    } else {
        Err(CheckErrors::TypeValueError(TypeSignature::PrincipalType, owner).into())
    }
}

/// Do a "consolidated" STX transfer.
/// If the 'from' principal has locked STX, and they have unlocked, then process the STX unlock
/// and update its balance in addition to spending tokens out of it.
pub fn stx_transfer_consolidated(
    env: &mut Environment,
    from: &PrincipalData,
    to: &PrincipalData,
    amount: u128,
) -> Result<Value> {
    if amount == 0 {
        return clarity_ecode!(StxErrorCodes::NON_POSITIVE_AMOUNT);
    }

    if from == to {
        return clarity_ecode!(StxErrorCodes::SENDER_IS_RECIPIENT);
    }

    if Some(from) != env.sender.as_ref() {
        return clarity_ecode!(StxErrorCodes::SENDER_IS_NOT_TX_SENDER);
    }

    // loading from/to principals and balances
    env.add_memory(TypeSignature::PrincipalType.size() as u64)?;
    env.add_memory(TypeSignature::PrincipalType.size() as u64)?;
    // loading from's locked amount and height
    // TODO: this does not count the inner stacks block header load, but arguably,
    // this could be optimized away, so it shouldn't penalize the caller.
    env.add_memory(STXBalance::size_of as u64)?;
    env.add_memory(STXBalance::size_of as u64)?;

    let sender_snapshot = env.global_context.database.get_stx_balance_snapshot(from);
    if !sender_snapshot.can_transfer(amount) {
        return clarity_ecode!(StxErrorCodes::NOT_ENOUGH_BALANCE);
    }

    sender_snapshot.transfer_to(to, amount)?;

    env.global_context.log_stx_transfer(&from, amount)?;
    env.register_stx_transfer_event(from.clone(), to.clone(), amount)?;
    Ok(Value::okay_true())
}

pub fn special_stx_transfer(
    args: &[SymbolicExpression],
    env: &mut Environment,
    context: &LocalContext,
) -> Result<Value> {
    check_argument_count(3, args)?;

    runtime_cost(ClarityCostFunction::StxTransfer, env, 0)?;

    let amount_val = eval(&args[0], env, context)?;
    let from_val = eval(&args[1], env, context)?;
    let to_val = eval(&args[2], env, context)?;

    if let (Value::Principal(ref from), Value::Principal(ref to), Value::UInt(amount)) =
        (&from_val, to_val, amount_val)
    {
        stx_transfer_consolidated(env, from, to, amount)
    } else {
        Err(CheckErrors::BadTransferSTXArguments.into())
    }
}

pub fn special_stx_burn(
    args: &[SymbolicExpression],
    env: &mut Environment,
    context: &LocalContext,
) -> Result<Value> {
    check_argument_count(2, args)?;

    runtime_cost(ClarityCostFunction::StxTransfer, env, 0)?;

    let amount_val = eval(&args[0], env, context)?;
    let from_val = eval(&args[1], env, context)?;

    if let (Value::Principal(ref from), Value::UInt(amount)) = (&from_val, amount_val) {
        if amount == 0 {
            return clarity_ecode!(StxErrorCodes::NON_POSITIVE_AMOUNT);
        }

        if Some(from) != env.sender.as_ref() {
            return clarity_ecode!(StxErrorCodes::SENDER_IS_NOT_TX_SENDER);
        }

        env.add_memory(TypeSignature::PrincipalType.size() as u64)?;
        env.add_memory(STXBalance::size_of as u64)?;

        let mut burner_snapshot = env.global_context.database.get_stx_balance_snapshot(&from);
        if !burner_snapshot.can_transfer(amount) {
            return clarity_ecode!(StxErrorCodes::NOT_ENOUGH_BALANCE);
        }

        burner_snapshot.debit(amount);
        burner_snapshot.save();

        env.global_context
            .database
            .decrement_ustx_liquid_supply(amount)?;

        env.global_context.log_stx_burn(&from, amount)?;
        env.register_stx_burn_event(from.clone(), amount)?;

        Ok(Value::okay_true())
    } else {
        Err(CheckErrors::BadTransferSTXArguments.into())
    }
}

pub fn special_mint_token(
    args: &[SymbolicExpression],
    env: &mut Environment,
    context: &LocalContext,
) -> Result<Value> {
    check_argument_count(3, args)?;

    runtime_cost(ClarityCostFunction::FtMint, env, 0)?;

    let token_name = args[0].match_atom().ok_or(CheckErrors::BadTokenName)?;

    let amount = eval(&args[1], env, context)?;
    let to = eval(&args[2], env, context)?;

    if let (Value::UInt(amount), Value::Principal(ref to_principal)) = (amount, to) {
        if amount == 0 {
            return clarity_ecode!(MintTokenErrorCodes::NON_POSITIVE_AMOUNT);
        }

        let ft_info = env
            .contract_context
            .meta_ft
            .get(token_name)
            .ok_or(CheckErrors::NoSuchFT(token_name.to_string()))?;

        env.global_context.database.checked_increase_token_supply(
            &env.contract_context.contract_identifier,
            token_name,
            amount,
            ft_info,
        )?;

        let to_bal = env.global_context.database.get_ft_balance(
            &env.contract_context.contract_identifier,
            token_name,
            to_principal,
            Some(ft_info),
        )?;

        let final_to_bal = to_bal.checked_add(amount).expect("STX overflow");

        env.add_memory(TypeSignature::PrincipalType.size() as u64)?;
        env.add_memory(TypeSignature::UIntType.size() as u64)?;

        env.global_context.database.set_ft_balance(
            &env.contract_context.contract_identifier,
            token_name,
            to_principal,
            final_to_bal,
        )?;

        let asset_identifier = AssetIdentifier {
            contract_identifier: env.contract_context.contract_identifier.clone(),
            asset_name: token_name.clone(),
        };
        env.register_ft_mint_event(to_principal.clone(), amount, asset_identifier)?;

        Ok(Value::okay_true())
    } else {
        Err(CheckErrors::BadMintFTArguments.into())
    }
}

pub fn special_mint_asset(
    args: &[SymbolicExpression],
    env: &mut Environment,
    context: &LocalContext,
) -> Result<Value> {
    check_argument_count(3, args)?;

    let asset_name = args[0].match_atom().ok_or(CheckErrors::BadTokenName)?;

    let asset = eval(&args[1], env, context)?;
    let to = eval(&args[2], env, context)?;

    let nft_metadata = env
        .contract_context
        .meta_nft
        .get(asset_name)
        .ok_or(CheckErrors::NoSuchNFT(asset_name.to_string()))?;
    let expected_asset_type = &nft_metadata.key_type;

    runtime_cost(
        ClarityCostFunction::NftMint,
        env,
        expected_asset_type.size(),
    )?;

    if !expected_asset_type.admits(&asset) {
        return Err(CheckErrors::TypeValueError(expected_asset_type.clone(), asset).into());
    }

    if let Value::Principal(ref to_principal) = to {
        match env.global_context.database.get_nft_owner(
            &env.contract_context.contract_identifier,
            asset_name,
            &asset,
            expected_asset_type,
        ) {
            Err(Error::Runtime(RuntimeErrorType::NoSuchToken, _)) => Ok(()),
            Ok(_owner) => return clarity_ecode!(MintAssetErrorCodes::ALREADY_EXIST),
            Err(e) => Err(e),
        }?;

        env.add_memory(TypeSignature::PrincipalType.size() as u64)?;
        env.add_memory(expected_asset_type.size() as u64)?;

        env.global_context.database.set_nft_owner(
            &env.contract_context.contract_identifier,
            asset_name,
            &asset,
            to_principal,
            expected_asset_type,
        )?;

        let asset_identifier = AssetIdentifier {
            contract_identifier: env.contract_context.contract_identifier.clone(),
            asset_name: asset_name.clone(),
        };
        env.register_nft_mint_event(to_principal.clone(), asset, asset_identifier)?;

        Ok(Value::okay_true())
    } else {
        Err(CheckErrors::TypeValueError(TypeSignature::PrincipalType, to).into())
    }
}

pub fn special_transfer_asset(
    args: &[SymbolicExpression],
    env: &mut Environment,
    context: &LocalContext,
) -> Result<Value> {
    check_argument_count(4, args)?;

    let asset_name = args[0].match_atom().ok_or(CheckErrors::BadTokenName)?;

    let asset = eval(&args[1], env, context)?;
    let from = eval(&args[2], env, context)?;
    let to = eval(&args[3], env, context)?;

    let nft_metadata = env
        .contract_context
        .meta_nft
        .get(asset_name)
        .ok_or(CheckErrors::NoSuchNFT(asset_name.to_string()))?;
    let expected_asset_type = &nft_metadata.key_type;

    runtime_cost(
        ClarityCostFunction::NftTransfer,
        env,
        expected_asset_type.size(),
    )?;

    if !expected_asset_type.admits(&asset) {
        return Err(CheckErrors::TypeValueError(expected_asset_type.clone(), asset).into());
    }

    if let (Value::Principal(ref from_principal), Value::Principal(ref to_principal)) = (from, to) {
        if from_principal == to_principal {
            return clarity_ecode!(TransferAssetErrorCodes::SENDER_IS_RECIPIENT);
        }

        let current_owner = match env.global_context.database.get_nft_owner(
            &env.contract_context.contract_identifier,
            asset_name,
            &asset,
            expected_asset_type,
        ) {
            Ok(owner) => Ok(owner),
            Err(Error::Runtime(RuntimeErrorType::NoSuchToken, _)) => {
                return clarity_ecode!(TransferAssetErrorCodes::DOES_NOT_EXIST)
            }
            Err(e) => Err(e),
        }?;

        if current_owner != *from_principal {
            return clarity_ecode!(TransferAssetErrorCodes::NOT_OWNED_BY);
        }

        env.add_memory(TypeSignature::PrincipalType.size() as u64)?;
        env.add_memory(expected_asset_type.size() as u64)?;

        env.global_context.database.set_nft_owner(
            &env.contract_context.contract_identifier,
            asset_name,
            &asset,
            to_principal,
            expected_asset_type,
        )?;

        env.global_context.log_asset_transfer(
            from_principal,
            &env.contract_context.contract_identifier,
            asset_name,
            asset.clone(),
        );

        let asset_identifier = AssetIdentifier {
            contract_identifier: env.contract_context.contract_identifier.clone(),
            asset_name: asset_name.clone(),
        };
        env.register_nft_transfer_event(
            from_principal.clone(),
            to_principal.clone(),
            asset,
            asset_identifier,
        )?;

        Ok(Value::okay_true())
    } else {
        Err(CheckErrors::BadTransferNFTArguments.into())
    }
}

pub fn special_transfer_token(
    args: &[SymbolicExpression],
    env: &mut Environment,
    context: &LocalContext,
) -> Result<Value> {
    check_argument_count(4, args)?;

    runtime_cost(ClarityCostFunction::FtTransfer, env, 0)?;

    let token_name = args[0].match_atom().ok_or(CheckErrors::BadTokenName)?;

    let amount = eval(&args[1], env, context)?;
    let from = eval(&args[2], env, context)?;
    let to = eval(&args[3], env, context)?;

    if let (
        Value::UInt(amount),
        Value::Principal(ref from_principal),
        Value::Principal(ref to_principal),
    ) = (amount, from, to)
    {
        if amount == 0 {
            return clarity_ecode!(TransferTokenErrorCodes::NON_POSITIVE_AMOUNT);
        }

        if from_principal == to_principal {
            return clarity_ecode!(TransferTokenErrorCodes::SENDER_IS_RECIPIENT);
        }

        let ft_info = env
            .contract_context
            .meta_ft
            .get(token_name)
            .ok_or(CheckErrors::NoSuchFT(token_name.to_string()))?;

        let from_bal = env.global_context.database.get_ft_balance(
            &env.contract_context.contract_identifier,
            token_name,
            from_principal,
            Some(ft_info),
        )?;

        if from_bal < amount {
            return clarity_ecode!(TransferTokenErrorCodes::NOT_ENOUGH_BALANCE);
        }

        let final_from_bal = from_bal - amount;

        let to_bal = env.global_context.database.get_ft_balance(
            &env.contract_context.contract_identifier,
            token_name,
            to_principal,
            Some(ft_info),
        )?;

        let final_to_bal = to_bal
            .checked_add(amount)
            .ok_or(RuntimeErrorType::ArithmeticOverflow)?;

        env.add_memory(TypeSignature::PrincipalType.size() as u64)?;
        env.add_memory(TypeSignature::PrincipalType.size() as u64)?;
        env.add_memory(TypeSignature::UIntType.size() as u64)?;
        env.add_memory(TypeSignature::UIntType.size() as u64)?;

        env.global_context.database.set_ft_balance(
            &env.contract_context.contract_identifier,
            token_name,
            from_principal,
            final_from_bal,
        )?;
        env.global_context.database.set_ft_balance(
            &env.contract_context.contract_identifier,
            token_name,
            to_principal,
            final_to_bal,
        )?;

        env.global_context.log_token_transfer(
            from_principal,
            &env.contract_context.contract_identifier,
            token_name,
            amount,
        )?;

        let asset_identifier = AssetIdentifier {
            contract_identifier: env.contract_context.contract_identifier.clone(),
            asset_name: token_name.clone(),
        };
        env.register_ft_transfer_event(
            from_principal.clone(),
            to_principal.clone(),
            amount,
            asset_identifier,
        )?;

        Ok(Value::okay_true())
    } else {
        Err(CheckErrors::BadTransferFTArguments.into())
    }
}

pub fn special_get_balance(
    args: &[SymbolicExpression],
    env: &mut Environment,
    context: &LocalContext,
) -> Result<Value> {
    check_argument_count(2, args)?;

    runtime_cost(ClarityCostFunction::FtBalance, env, 0)?;

    let token_name = args[0].match_atom().ok_or(CheckErrors::BadTokenName)?;

    let owner = eval(&args[1], env, context)?;

    if let Value::Principal(ref principal) = owner {
        let ft_info = env
            .contract_context
            .meta_ft
            .get(token_name)
            .ok_or(CheckErrors::NoSuchFT(token_name.to_string()))?;

        let balance = env.global_context.database.get_ft_balance(
            &env.contract_context.contract_identifier,
            token_name,
            principal,
            Some(ft_info),
        )?;
        Ok(Value::UInt(balance))
    } else {
        Err(CheckErrors::TypeValueError(TypeSignature::PrincipalType, owner).into())
    }
}

pub fn special_get_owner(
    args: &[SymbolicExpression],
    env: &mut Environment,
    context: &LocalContext,
) -> Result<Value> {
    check_argument_count(2, args)?;

    let asset_name = args[0].match_atom().ok_or(CheckErrors::BadTokenName)?;

    let asset = eval(&args[1], env, context)?;

    let nft_metadata = env
        .contract_context
        .meta_nft
        .get(asset_name)
        .ok_or(CheckErrors::NoSuchNFT(asset_name.to_string()))?;
    let expected_asset_type = &nft_metadata.key_type;

    runtime_cost(
        ClarityCostFunction::NftOwner,
        env,
        expected_asset_type.size(),
    )?;

    if !expected_asset_type.admits(&asset) {
        return Err(CheckErrors::TypeValueError(expected_asset_type.clone(), asset).into());
    }

    match env.global_context.database.get_nft_owner(
        &env.contract_context.contract_identifier,
        asset_name,
        &asset,
        expected_asset_type,
    ) {
        Ok(owner) => {
            Ok(Value::some(Value::Principal(owner))
                .expect("Principal should always fit in optional."))
        }
        Err(Error::Runtime(RuntimeErrorType::NoSuchToken, _)) => Ok(Value::none()),
        Err(e) => Err(e),
    }
}

pub fn special_get_token_supply(
    args: &[SymbolicExpression],
    env: &mut Environment,
    _context: &LocalContext,
) -> Result<Value> {
    check_argument_count(1, args)?;

    runtime_cost(ClarityCostFunction::FtSupply, env, 0)?;

    let token_name = args[0].match_atom().ok_or(CheckErrors::BadTokenName)?;

    let supply = env
        .global_context
        .database
        .get_ft_supply(&env.contract_context.contract_identifier, token_name)?;
    Ok(Value::UInt(supply))
}

pub fn special_burn_token(
    args: &[SymbolicExpression],
    env: &mut Environment,
    context: &LocalContext,
) -> Result<Value> {
    check_argument_count(3, args)?;

    runtime_cost(ClarityCostFunction::FtBurn, env, 0)?;

    let token_name = args[0].match_atom().ok_or(CheckErrors::BadTokenName)?;

    let amount = eval(&args[1], env, context)?;
    let from = eval(&args[2], env, context)?;

    if let (Value::UInt(amount), Value::Principal(ref burner)) = (amount, from) {
        if amount == 0 {
            return clarity_ecode!(MintTokenErrorCodes::NON_POSITIVE_AMOUNT);
        }

        let burner_bal = env.global_context.database.get_ft_balance(
            &env.contract_context.contract_identifier,
            token_name,
            burner,
            None,
        )?;

        if amount > burner_bal {
            return clarity_ecode!(BurnTokenErrorCodes::NOT_ENOUGH_BALANCE);
        }

        env.global_context.database.checked_decrease_token_supply(
            &env.contract_context.contract_identifier,
            token_name,
            amount,
        )?;

        let final_burner_bal = burner_bal - amount;

        env.global_context.database.set_ft_balance(
            &env.contract_context.contract_identifier,
            token_name,
            burner,
            final_burner_bal,
        )?;

        let asset_identifier = AssetIdentifier {
            contract_identifier: env.contract_context.contract_identifier.clone(),
            asset_name: token_name.clone(),
        };
        env.register_ft_burn_event(burner.clone(), amount, asset_identifier)?;

        env.add_memory(TypeSignature::PrincipalType.size() as u64)?;
        env.add_memory(TypeSignature::UIntType.size() as u64)?;

        env.global_context.log_token_transfer(
            burner,
            &env.contract_context.contract_identifier,
            token_name,
            amount,
        )?;

        Ok(Value::okay_true())
    } else {
        Err(CheckErrors::BadBurnFTArguments.into())
    }
}

pub fn special_burn_asset(
    args: &[SymbolicExpression],
    env: &mut Environment,
    context: &LocalContext,
) -> Result<Value> {
    check_argument_count(3, args)?;

    runtime_cost(ClarityCostFunction::NftBurn, env, 0)?;

    let asset_name = args[0].match_atom().ok_or(CheckErrors::BadTokenName)?;

    let asset = eval(&args[1], env, context)?;
    let sender = eval(&args[2], env, context)?;

    let nft_metadata = env
        .contract_context
        .meta_nft
        .get(asset_name)
        .ok_or(CheckErrors::NoSuchNFT(asset_name.to_string()))?;
    let expected_asset_type = &nft_metadata.key_type;

    runtime_cost(
        ClarityCostFunction::NftBurn,
        env,
        expected_asset_type.size(),
    )?;

    if !expected_asset_type.admits(&asset) {
        return Err(CheckErrors::TypeValueError(expected_asset_type.clone(), asset).into());
    }

    if let Value::Principal(ref sender_principal) = sender {
        let owner = match env.global_context.database.get_nft_owner(
            &env.contract_context.contract_identifier,
            asset_name,
            &asset,
            expected_asset_type,
        ) {
            Err(Error::Runtime(RuntimeErrorType::NoSuchToken, _)) => {
                return clarity_ecode!(BurnAssetErrorCodes::DOES_NOT_EXIST)
            }
            Ok(owner) => Ok(owner),
            Err(e) => Err(e),
        }?;

        if &owner != sender_principal {
            return clarity_ecode!(BurnAssetErrorCodes::NOT_OWNED_BY);
        }

        env.add_memory(TypeSignature::PrincipalType.size() as u64)?;
        env.add_memory(expected_asset_type.size() as u64)?;

        env.global_context.database.burn_nft(
            &env.contract_context.contract_identifier,
            asset_name,
            &asset,
            expected_asset_type,
        )?;

        env.global_context.log_asset_transfer(
            sender_principal,
            &env.contract_context.contract_identifier,
            asset_name,
            asset.clone(),
        );

        let asset_identifier = AssetIdentifier {
            contract_identifier: env.contract_context.contract_identifier.clone(),
            asset_name: asset_name.clone(),
        };
        env.register_nft_burn_event(sender_principal.clone(), asset, asset_identifier)?;

        Ok(Value::okay_true())
    } else {
        Err(CheckErrors::TypeValueError(TypeSignature::PrincipalType, sender).into())
    }
}
