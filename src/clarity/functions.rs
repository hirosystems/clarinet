use std::collections::{HashMap, BTreeMap};
pub use super::representations::{SymbolicExpression, SymbolicExpressionType, ClarityName, ContractName};
pub use super::types::{Value, TraitIdentifier, DefinedFunction, TupleTypeSignature, TypeSignature};
pub use super::types::signatures::FunctionSignature;
pub use crate::clarity::analysis::errors::{CheckErrors, check_argument_count, check_arguments_at_least};

macro_rules! define_named_enum {
    ($Name:ident { $($Variant:ident($VarName:literal),)* }) =>
    {
        #[derive(Debug)]
        pub enum $Name {
            $($Variant),*,
        }
        impl $Name {
            pub const ALL: &'static [$Name] = &[$($Name::$Variant),*];
            pub const ALL_NAMES: &'static [&'static str] = &[$($VarName),*];

            pub fn lookup_by_name(name: &str) -> Option<Self> {
                match name {
                    $(
                        $VarName => Some($Name::$Variant),
                    )*
                    _ => None
                }
            }

            pub fn get_name(&self) -> String {
                match self {
                    $(
                        $Name::$Variant => $VarName.to_string(),
                    )*
                }
            }
        }
    }
}

define_named_enum!(NativeFunctions {
    Add("+"),
    Subtract("-"),
    Multiply("*"),
    Divide("/"),
    CmpGeq(">="),
    CmpLeq("<="),
    CmpLess("<"),
    CmpGreater(">"),
    ToInt("to-int"),
    ToUInt("to-uint"),
    Modulo("mod"),
    Power("pow"),
    BitwiseXOR("xor"),
    And("and"),
    Or("or"),
    Not("not"),
    Equals("is-eq"),
    If("if"),
    Let("let"),
    Map("map"),
    Fold("fold"),
    Append("append"),
    Concat("concat"),
    AsMaxLen("as-max-len?"),
    Len("len"),
    ListCons("list"),
    FetchVar("var-get"),
    SetVar("var-set"),
    FetchEntry("map-get?"),
    SetEntry("map-set"),
    InsertEntry("map-insert"),
    DeleteEntry("map-delete"),
    TupleCons("tuple"),
    TupleGet("get"),
    Begin("begin"),
    Hash160("hash160"),
    Sha256("sha256"),
    Sha512("sha512"),
    Sha512Trunc256("sha512/256"),
    Keccak256("keccak256"),
    Print("print"),
    ContractCall("contract-call?"),
    AsContract("as-contract"),
    AtBlock("at-block"),
    GetBlockInfo("get-block-info?"),
    ConsError("err"),
    ConsOkay("ok"),
    ConsSome("some"),
    DefaultTo("default-to"),
    Asserts("asserts!"),
    UnwrapRet("unwrap!"),
    UnwrapErrRet("unwrap-err!"),
    Unwrap("unwrap-panic"),
    UnwrapErr("unwrap-err-panic"),
    Match("match"),
    TryRet("try!"),
    IsOkay("is-ok"),
    IsNone("is-none"),
    IsErr("is-err"),
    IsSome("is-some"),
    Filter("filter"),
    GetTokenBalance("ft-get-balance"),
    GetAssetOwner("nft-get-owner?"),
    TransferToken("ft-transfer?"),
    TransferAsset("nft-transfer?"),
    MintAsset("nft-mint?"),
    MintToken("ft-mint?"),
    StxTransfer("stx-transfer?"),
    GetStxBalance("stx-get-balance"),
    StxBurn("stx-burn?"),
});

define_named_enum!(DefineFunctions {
    Constant("define-constant"),
    PrivateFunction("define-private"),
    PublicFunction("define-public"),
    ReadOnlyFunction("define-read-only"),
    Map("define-map"),
    PersistedVariable("define-data-var"),
    FungibleToken("define-fungible-token"),
    NonFungibleToken("define-non-fungible-token"),
    Trait("define-trait"),
    UseTrait("use-trait"),
    ImplTrait("impl-trait"),
});

define_named_enum!(NativeVariables {
    ContractCaller("contract-caller"), TxSender("tx-sender"), BlockHeight("block-height"),
    BurnBlockHeight("burn-block-height"), NativeNone("none"),
    NativeTrue("true"), NativeFalse("false"),
});

define_named_enum!(BlockInfoProperty {
    Time("time"),
    VrfSeed("vrf-seed"),
    HeaderHash("header-hash"),
    IdentityHeaderHash("id-header-hash"),
    BurnchainHeaderHash("burnchain-header-hash"),
    MinerAddress("miner-address"),
});

pub enum DefineFunctionsParsed <'a> {
    Constant { name: &'a ClarityName, value: &'a SymbolicExpression },
    PrivateFunction { signature: &'a [SymbolicExpression], body: &'a SymbolicExpression },
    ReadOnlyFunction { signature: &'a [SymbolicExpression], body: &'a SymbolicExpression },
    PublicFunction { signature: &'a [SymbolicExpression], body: &'a SymbolicExpression },
    NonFungibleToken { name: &'a ClarityName, nft_type: &'a SymbolicExpression },
    BoundedFungibleToken { name: &'a ClarityName, max_supply: &'a SymbolicExpression },
    UnboundedFungibleToken { name: &'a ClarityName },
    Map { name: &'a ClarityName, key_type: &'a SymbolicExpression, value_type: &'a SymbolicExpression },
    PersistedVariable  { name: &'a ClarityName, data_type: &'a SymbolicExpression, initial: &'a SymbolicExpression },
    Trait { name: &'a ClarityName, functions: &'a [SymbolicExpression] },
    UseTrait { name: &'a ClarityName, trait_identifier: &'a TraitIdentifier },
    ImplTrait { trait_identifier: &'a TraitIdentifier },
}

pub enum DefineResult {
    Variable(ClarityName, Value),
    Function(ClarityName, DefinedFunction),
    Map(String, TupleTypeSignature, TupleTypeSignature),
    PersistedVariable(String, TypeSignature, Value),
    FungibleToken(String, Option<u128>),
    NonFungibleAsset(String, TypeSignature),
    Trait(ClarityName, BTreeMap<ClarityName, FunctionSignature>),
    UseTrait(ClarityName, TraitIdentifier),
    ImplTrait(TraitIdentifier),
    NoDefine
}

impl <'a> DefineFunctionsParsed <'a> {
    /// Try to parse a Top-Level Expression (e.g., (define-private (foo) 1)) as
    /// a define-statement, returns None if the supplied expression is not a define.
    pub fn try_parse(expression: &'a SymbolicExpression) -> std::result::Result<Option<DefineFunctionsParsed<'a>>, CheckErrors> {
        let (define_type, args) = match DefineFunctions::try_parse(expression) {
            Some(x) => x,
            None => return Ok(None)
        };
        let result = match define_type {
            DefineFunctions::Constant => {
                check_argument_count(2, args)?;
                let name = args[0].match_atom().ok_or(CheckErrors::ExpectedName)?;
                DefineFunctionsParsed::Constant { name, value: &args[1] }
            },
            DefineFunctions::PrivateFunction => {
                check_argument_count(2, args)?;
                let signature = args[0].match_list().ok_or(CheckErrors::DefineFunctionBadSignature)?;
                DefineFunctionsParsed::PrivateFunction { signature, body: &args[1] }
            },
            DefineFunctions::ReadOnlyFunction => {
                check_argument_count(2, args)?;
                let signature = args[0].match_list().ok_or(CheckErrors::DefineFunctionBadSignature)?;
                DefineFunctionsParsed::ReadOnlyFunction { signature, body: &args[1] }
            },
            DefineFunctions::PublicFunction => {
                check_argument_count(2, args)?;
                let signature = args[0].match_list().ok_or(CheckErrors::DefineFunctionBadSignature)?;
                DefineFunctionsParsed::PublicFunction { signature, body: &args[1] }
            },
            DefineFunctions::NonFungibleToken => {
                check_argument_count(2, args)?;
                let name = args[0].match_atom().ok_or(CheckErrors::ExpectedName)?;
                DefineFunctionsParsed::NonFungibleToken { name, nft_type: &args[1] }
            },
            DefineFunctions::FungibleToken => {
                let name = args[0].match_atom().ok_or(CheckErrors::ExpectedName)?;
                if args.len() == 1 {
                    DefineFunctionsParsed::UnboundedFungibleToken { name }
                } else if args.len() == 2 {
                    DefineFunctionsParsed::BoundedFungibleToken { name, max_supply: &args[1] }
                } else {
                    return Err(CheckErrors::IncorrectArgumentCount(1, args.len()).into())
                }
            },
            DefineFunctions::Map => {
                check_argument_count(3, args)?;
                let name = args[0].match_atom().ok_or(CheckErrors::ExpectedName)?;
                DefineFunctionsParsed::Map { name, key_type: &args[1], value_type: &args[2] }
            },
            DefineFunctions::PersistedVariable => {
                check_argument_count(3, args)?;
                let name = args[0].match_atom().ok_or(CheckErrors::ExpectedName)?;
                DefineFunctionsParsed::PersistedVariable { name, data_type: &args[1], initial: &args[2] }
            },
            DefineFunctions::Trait => {
                check_arguments_at_least(2, args)?;
                let name = args[0].match_atom().ok_or(CheckErrors::ExpectedName)?;
                DefineFunctionsParsed::Trait { name, functions: &args[1..] }
            },
            DefineFunctions::UseTrait => {
                check_argument_count(2, args)?;
                let name = args[0].match_atom().ok_or(CheckErrors::ExpectedName)?;
                match &args[1].expr {
                    SymbolicExpressionType::Field(ref field) => DefineFunctionsParsed::UseTrait { 
                        name: &name, 
                        trait_identifier: &field 
                    },
                    _ => return Err(CheckErrors::ExpectedTraitIdentifier.into())
                }
            },
            DefineFunctions::ImplTrait => {
                check_argument_count(1, args)?;
                match &args[0].expr {
                    SymbolicExpressionType::Field(ref field) => DefineFunctionsParsed::ImplTrait { 
                        trait_identifier: &field 
                    },
                    _ => return Err(CheckErrors::ExpectedTraitIdentifier.into())
                }
            },

        };
        Ok(Some(result))
    }
}

pub fn handle_binding_list <F, E> (bindings: &[SymbolicExpression], mut handler: F) -> std::result::Result<(), E>
where F: FnMut(&ClarityName, &SymbolicExpression) -> std::result::Result<(), E>,
      E: From<CheckErrors>
{
    for binding in bindings.iter() {
        let binding_expression = binding.match_list()
            .ok_or(CheckErrors::BadSyntaxBinding)?;
        if binding_expression.len() != 2 {
            return Err(CheckErrors::BadSyntaxBinding.into());
        }
        let var_name = binding_expression[0].match_atom()
            .ok_or(CheckErrors::BadSyntaxBinding)?;
        let var_sexp = &binding_expression[1];

        handler(var_name, var_sexp)?;
    }
    Ok(())
}
