use crate::poke::load_session;
use crate::utils::mnemonic;
use clarity_repl::{clarity::{codec::{StacksMessageCodec, StacksString, transaction::{RecoverableSignature, SinglesigHashMode, SinglesigSpendingCondition, StacksTransaction, StacksTransactionSigner, TransactionAnchorMode, TransactionAuth, TransactionPayload, TransactionPostConditionMode, TransactionPublicKeyEncoding, TransactionSmartContract, TransactionSpendingCondition, TransactionVersion}}, util::{
        address::AddressHashMode,
        secp256k1::{Secp256k1PrivateKey, Secp256k1PublicKey},
        StacksAddress,
    }}, repl::{OutputMode, settings::{Account, InitialContract}}};
use libsecp256k1::{PublicKey, SecretKey};
use std::collections::BTreeMap;
use std::path::PathBuf;
use tiny_hderive::bip32::ExtendedPrivKey;

#[derive(Deserialize, Debug)]
struct Balance {
    balance: String,
    nonce: u64,
    balance_proof: String,
    nonce_proof: String,
}

#[allow(dead_code)]
pub enum Network {
    Devnet,
    Testnet,
    Mainnet,
}

pub fn publish_contract(
    contract: &InitialContract,
    deployers_lookup: &BTreeMap<String, Account>,
    deployers_nonces: &mut BTreeMap<String, u64>,
    node: &str,
) -> Result<(String, u64), String> {
    let contract_name = contract.name.clone().unwrap();

    let payload = TransactionSmartContract {
        name: contract_name.as_str().into(),
        code_body: StacksString::from_string(&contract.code).unwrap(),
    };

    let deployer = match deployers_lookup.get(&contract_name) {
        Some(deployer) => deployer,
        None => deployers_lookup.get("*").unwrap(),
    };

    let bip39_seed = match mnemonic::get_bip39_seed_from_mnemonic(&deployer.mnemonic, "") {
        Ok(bip39_seed) => bip39_seed,
        Err(_) => panic!(),
    };
    let ext = ExtendedPrivKey::derive(&bip39_seed[..], deployer.derivation.as_str()).unwrap();
    let secret_key = SecretKey::parse_slice(&ext.secret()).unwrap();
    let public_key = PublicKey::from_secret_key(&secret_key);

    let wrapped_public_key =
        Secp256k1PublicKey::from_slice(&public_key.serialize_compressed()).unwrap();
    let wrapped_secret_key = Secp256k1PrivateKey::from_slice(&ext.secret()).unwrap();

    let anchor_mode = TransactionAnchorMode::Any;
    let tx_fee = 200 + contract.code.len() as u64;

    let nonce = match deployers_nonces.get(&deployer.name) {
        Some(nonce) => *nonce,
        None => {
            let request_url = format!(
                "{host}/v2/accounts/{addr}",
                host = node,
                addr = deployer.address,
            );

            let response: Balance = reqwest::blocking::get(&request_url)
                .expect("Unable to retrieve account")
                .json()
                .expect("Unable to parse contract");
            let nonce = response.nonce;
            deployers_nonces.insert(deployer.name.clone(), nonce);
            nonce
        }
    };

    let signer_addr = StacksAddress::from_public_keys(
        0,
        &AddressHashMode::SerializeP2PKH,
        1,
        &vec![wrapped_public_key],
    )
    .unwrap();

    let spending_condition = TransactionSpendingCondition::Singlesig(SinglesigSpendingCondition {
        signer: signer_addr.bytes.clone(),
        nonce: nonce,
        tx_fee: tx_fee,
        hash_mode: SinglesigHashMode::P2PKH,
        key_encoding: TransactionPublicKeyEncoding::Compressed,
        signature: RecoverableSignature::empty(),
    });

    let auth = TransactionAuth::Standard(spending_condition);
    let unsigned_tx = StacksTransaction {
        version: TransactionVersion::Testnet,
        chain_id: 0x80000000, // MAINNET=0x00000001 TODO(ludo): mainnet handling
        auth: auth,
        anchor_mode: anchor_mode,
        post_condition_mode: TransactionPostConditionMode::Deny,
        post_conditions: vec![],
        payload: TransactionPayload::SmartContract(payload),
    };

    let mut unsigned_tx_bytes = vec![];
    unsigned_tx
        .consensus_serialize(&mut unsigned_tx_bytes)
        .expect("FATAL: invalid transaction");

    let mut tx_signer = StacksTransactionSigner::new(&unsigned_tx);
    tx_signer.sign_origin(&wrapped_secret_key).unwrap();
    let signed_tx = tx_signer.get_tx().unwrap();

    let tx_bytes = signed_tx.serialize_to_vec();
    let client = reqwest::blocking::Client::new();
    let path = format!("{}/v2/transactions", node);
    let res = client
        .post(&path)
        .header("Content-Type", "application/octet-stream")
        .body(tx_bytes)
        .send()
        .unwrap();

    if !res.status().is_success() {
        return Err(format!("{}", res.text().unwrap()));
    }
    let txid: String = res.json().unwrap();
    deployers_nonces.insert(deployer.name.clone(), nonce + 1);
    Ok((txid, nonce))
}

pub fn publish_all_contracts(
    manifest_path: PathBuf,
    network: Network,
) -> Result<Vec<String>, Vec<String>> {
    let start_repl = false;
    let settings = match load_session(manifest_path, start_repl, network, OutputMode::Console) {
        Ok(settings) => settings,
        Err(e) => return Err(vec![e]),
    };
    let mut results = vec![];
    let mut deployers_nonces = BTreeMap::new();
    let mut deployers_lookup = BTreeMap::new();
    for account in settings.initial_accounts.iter() {
        if account.name == "deployer" {
            deployers_lookup.insert("*".into(), account.clone());
        }
    }

    for contract in settings.initial_contracts.iter() {
        match publish_contract(
            contract,
            &deployers_lookup,
            &mut deployers_nonces,
            &settings.node,
        ) {
            Ok((txid, nonce)) => {
                results.push(format!(
                    "Contract {} broadcasted in mempool (txid: {}, nonce: {})",
                    contract.name.as_ref().unwrap(),
                    txid,
                    nonce
                ));
            }
            Err(err) => {
                results.push(err.to_string());
                break;
            }
        }
    }
    // If devnet, we should be pulling all the links.

    Ok(results)
}
