use crate::poke::load_session;
use crate::types::ProjectManifest;
use crate::utils::mnemonic;
use crate::utils::stacks::StacksRpc;
use clarity_repl::clarity::codec::transaction::{
    StacksTransaction, StacksTransactionSigner, TransactionAnchorMode, TransactionAuth,
    TransactionPayload, TransactionPostConditionMode, TransactionPublicKeyEncoding,
    TransactionSmartContract, TransactionSpendingCondition,
};
use clarity_repl::clarity::codec::StacksMessageCodec;
use clarity_repl::clarity::{
    codec::{
        transaction::{
            RecoverableSignature, SinglesigHashMode, SinglesigSpendingCondition, TransactionVersion,
        },
        StacksString,
    },
    util::{
        address::AddressHashMode,
        secp256k1::{Secp256k1PrivateKey, Secp256k1PublicKey},
        StacksAddress,
    },
};
use clarity_repl::repl::settings::{Account, InitialContract};
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
#[derive(Debug)]
pub enum Network {
    Devnet,
    Testnet,
    Mainnet,
}

pub fn publish_contract(
    contract: &InitialContract,
    deployers_lookup: &BTreeMap<String, Account>,
    deployers_nonces: &mut BTreeMap<String, u64>,
    node_url: &str,
    deployment_fee_rate: u64,
    network: &Network,
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
    let tx_fee = deployment_fee_rate * contract.code.len() as u64;

    let stacks_rpc = StacksRpc::new(&node_url);

    let nonce = match deployers_nonces.get(&deployer.name) {
        Some(nonce) => *nonce,
        None => {
            let nonce = stacks_rpc
                .get_nonce(&deployer.address)
                .expect("Unable to retrieve account");
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
        version: match network {
            Network::Mainnet => TransactionVersion::Mainnet,
            _ => TransactionVersion::Testnet,
        },
        chain_id: match network {
            Network::Mainnet => 0x00000001,
            _ => 0x80000000,
        },
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

    let txid = match stacks_rpc.post_transaction(signed_tx) {
        Ok(res) => res.txid,
        Err(e) => return Err(format!("{:?}", e)),
    };
    deployers_nonces.insert(deployer.name.clone(), nonce + 1);
    Ok((txid, nonce))
}

pub fn publish_all_contracts(
    manifest_path: PathBuf,
    network: &Network,
) -> Result<(Vec<String>, ProjectManifest), Vec<String>> {
    let start_repl = false;
    let (session, chain, manifest) = match load_session(manifest_path, start_repl, &network) {
        Ok((session, chain, manifest)) => (session, chain, manifest),
        Err(e) => return Err(vec![e]),
    };
    let mut results = vec![];
    let mut deployers_nonces = BTreeMap::new();
    let mut deployers_lookup = BTreeMap::new();
    let settings = session.settings;
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
            chain.network.deployment_fee_rate,
            &network,
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

    Ok((results, manifest))
}
