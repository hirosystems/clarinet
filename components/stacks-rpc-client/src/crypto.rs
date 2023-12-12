use std::str::FromStr;

use crate::clarity::codec::*;

use crate::clarity::vm::types::{PrincipalData, QualifiedContractIdentifier, Value};
use crate::clarity::vm::{ClarityName, ClarityVersion, ContractName};
use clarity_repl::clarity::address::{
    AddressHashMode, C32_ADDRESS_VERSION_MAINNET_SINGLESIG, C32_ADDRESS_VERSION_TESTNET_SINGLESIG,
};
use clarity_repl::clarity::chainstate::StacksAddress;
use clarity_repl::clarity::codec::StacksMessageCodec;
use clarity_repl::clarity::util::secp256k1::{
    MessageSignature, Secp256k1PrivateKey, Secp256k1PublicKey,
};
use hmac::Hmac;
use libsecp256k1::{PublicKey, SecretKey};
use pbkdf2::pbkdf2;
use sha2::Sha512;
use tiny_hderive::bip32::ExtendedPrivKey;

#[derive(Clone, Debug)]
pub struct Wallet {
    pub mnemonic: String,
    pub derivation: String,
    pub mainnet: bool,
}

impl Wallet {
    pub fn compute_stacks_address(&self) -> StacksAddress {
        let keypair = compute_keypair(self);
        compute_stacks_address(&keypair.public_key, self.mainnet)
    }
}

pub struct Keypair {
    pub secret_key: Secp256k1PrivateKey,
    pub public_key: PublicKey,
}

pub fn compute_stacks_address(public_key: &PublicKey, mainnet: bool) -> StacksAddress {
    let wrapped_public_key =
        Secp256k1PublicKey::from_slice(&public_key.serialize_compressed()).unwrap();

    StacksAddress::from_public_keys(
        match mainnet {
            true => C32_ADDRESS_VERSION_MAINNET_SINGLESIG,
            false => C32_ADDRESS_VERSION_TESTNET_SINGLESIG,
        },
        &AddressHashMode::SerializeP2PKH,
        1,
        &vec![wrapped_public_key],
    )
    .unwrap()
}

pub fn compute_keypair(wallet: &Wallet) -> Keypair {
    let bip39_seed = match get_bip39_seed_from_mnemonic(&wallet.mnemonic, "") {
        Ok(bip39_seed) => bip39_seed,
        Err(_) => panic!(),
    };
    let ext = ExtendedPrivKey::derive(&bip39_seed[..], wallet.derivation.as_str()).unwrap();
    let wrapped_secret_key = Secp256k1PrivateKey::from_slice(&ext.secret()).unwrap();
    let secret_key = SecretKey::parse_slice(&ext.secret()).unwrap();
    let public_key = PublicKey::from_secret_key(&secret_key);
    Keypair {
        secret_key: wrapped_secret_key,
        public_key,
    }
}

pub fn sign_transaction_payload(
    wallet: &Wallet,
    payload: TransactionPayload,
    nonce: u64,
    tx_fee: u64,
    anchor_mode: TransactionAnchorMode,
) -> Result<StacksTransaction, String> {
    let keypair = compute_keypair(wallet);
    let signer_addr = compute_stacks_address(&keypair.public_key, wallet.mainnet);

    let spending_condition = TransactionSpendingCondition::Singlesig(SinglesigSpendingCondition {
        signer: signer_addr.bytes,
        nonce,
        tx_fee,
        hash_mode: SinglesigHashMode::P2PKH,
        key_encoding: TransactionPublicKeyEncoding::Compressed,
        signature: MessageSignature::empty(),
    });

    let auth = TransactionAuth::Standard(spending_condition);
    let unsigned_tx = StacksTransaction {
        version: match wallet.mainnet {
            true => TransactionVersion::Mainnet,
            false => TransactionVersion::Testnet,
        },
        chain_id: match wallet.mainnet {
            true => 0x00000001,
            false => 0x80000000,
        },
        auth,
        anchor_mode,
        post_condition_mode: TransactionPostConditionMode::Allow,
        post_conditions: vec![],
        payload,
    };

    let mut unsigned_tx_bytes = vec![];
    unsigned_tx
        .consensus_serialize(&mut unsigned_tx_bytes)
        .expect("FATAL: invalid transaction");

    let mut tx_signer = StacksTransactionSigner::new(&unsigned_tx);
    tx_signer.sign_origin(&keypair.secret_key).unwrap();
    let signed_tx = tx_signer.get_tx().unwrap();
    Ok(signed_tx)
}

pub fn encode_contract_call(
    contract_id: &QualifiedContractIdentifier,
    function_name: ClarityName,
    function_args: Vec<Value>,
    wallet: &Wallet,
    nonce: u64,
    tx_fee: u64,
    anchor_mode: TransactionAnchorMode,
) -> Result<StacksTransaction, String> {
    let payload = TransactionContractCall {
        contract_name: contract_id.name.clone(),
        address: StacksAddress::from(contract_id.issuer.clone()),
        function_name: function_name.clone(),
        function_args: function_args.clone(),
    };
    sign_transaction_payload(
        wallet,
        TransactionPayload::ContractCall(payload),
        nonce,
        tx_fee,
        anchor_mode,
    )
}

pub fn encode_stx_transfer(
    recipient: PrincipalData,
    amount: u64,
    memo: [u8; 34],
    wallet: &Wallet,
    nonce: u64,
    tx_fee: u64,
    anchor_mode: TransactionAnchorMode,
) -> Result<StacksTransaction, String> {
    let payload = TransactionPayload::TokenTransfer(recipient, amount, TokenTransferMemo(memo));
    sign_transaction_payload(wallet, payload, nonce, tx_fee, anchor_mode)
}

pub fn encode_contract_publish(
    contract_name: &ContractName,
    source: &str,
    clarity_version: Option<ClarityVersion>,
    wallet: &Wallet,
    nonce: u64,
    tx_fee: u64,
    anchor_mode: TransactionAnchorMode,
) -> Result<StacksTransaction, String> {
    let payload = TransactionSmartContract {
        name: contract_name.clone(),
        code_body: StacksString::from_str(source).unwrap(),
    };
    sign_transaction_payload(
        wallet,
        TransactionPayload::SmartContract(payload, clarity_version),
        nonce,
        tx_fee,
        anchor_mode,
    )
}

pub fn get_bip39_seed_from_mnemonic(mnemonic: &str, password: &str) -> Result<Vec<u8>, String> {
    const PBKDF2_ROUNDS: u32 = 2048;
    const PBKDF2_BYTES: usize = 64;
    let salt = format!("mnemonic{}", password);
    let mut seed = vec![0u8; PBKDF2_BYTES];

    pbkdf2::<Hmac<Sha512>>(
        mnemonic.as_bytes(),
        salt.as_bytes(),
        PBKDF2_ROUNDS,
        &mut seed,
    )
    .map_err(|e| e.to_string())?;
    Ok(seed)
}
