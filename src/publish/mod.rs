use crate::poke::{load_session, load_session_settings};
use crate::utils::mnemonic;
use crate::utils::stacks::StacksRpc;
use clarity_repl::clarity::codec::transaction::{
    StacksTransaction, StacksTransactionSigner, TransactionAnchorMode, TransactionAuth,
    TransactionPayload, TransactionPostConditionMode, TransactionPublicKeyEncoding,
    TransactionSmartContract, TransactionSpendingCondition,
};
use clarity_repl::clarity::codec::StacksMessageCodec;
use clarity_repl::clarity::util::{
    C32_ADDRESS_VERSION_MAINNET_SINGLESIG, C32_ADDRESS_VERSION_TESTNET_SINGLESIG,
};
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
use std::collections::HashSet;
use std::collections::{BTreeMap, VecDeque};
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
    node_url: &str,
    deployment_fee_rate: u64,
    network: &Network,
) -> Result<(String, u64, String, String), String> {
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
        match network {
            Network::Mainnet => C32_ADDRESS_VERSION_MAINNET_SINGLESIG,
            _ => C32_ADDRESS_VERSION_TESTNET_SINGLESIG,
        },
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
    Ok((txid, nonce, signer_addr.to_string(), contract_name))
}

pub fn publish_all_contracts(
    manifest_path: &PathBuf,
    network: Network,
    analysis_enabled: bool,
    delay_between_checks: u32,
) -> Result<Vec<String>, Vec<String>> {
    let (settings, chain) = if analysis_enabled {
        let start_repl = false;
        let (session, chain, output) = match load_session(manifest_path, start_repl, &network) {
            Ok((session, chain, output)) => (session, chain, output),
            Err(e) => return Err(vec![e]),
        };

        if let Some(message) = output {
            println!("{}", message);
            println!("{}", yellow!("Would you like to continue [Y/n]:"));
            let mut buffer = String::new();
            std::io::stdin().read_line(&mut buffer).unwrap();
            if buffer == "n\n" {
                println!("{}", red!("Contracts deployment aborted"));
                std::process::exit(1);
            }
        }
        (session.settings, chain)
    } else {
        let (settings, chain) = match load_session_settings(manifest_path, &network) {
            Ok((session, chain, _)) => (session, chain),
            Err(e) => return Err(vec![e]),
        };
        (settings, chain)
    };

    let mut results = vec![];
    let mut deployers_nonces = BTreeMap::new();
    let mut deployers_lookup = BTreeMap::new();

    for account in settings.initial_accounts.iter() {
        if account.name == "deployer" {
            deployers_lookup.insert("*".into(), account.clone());
        }
    }

    let node_url = settings.node.clone();
    let stacks_rpc = StacksRpc::new(&node_url);

    for batch in settings.initial_contracts.chunks(25) {
        let mut contracts_being_deployed = HashSet::new();
        for contract in batch.iter() {
            match publish_contract(
                contract,
                &deployers_lookup,
                &mut deployers_nonces,
                &node_url,
                chain.network.deployment_fee_rate,
                &network,
            ) {
                Ok((txid, nonce, deployer_address, contract_name)) => {
                    results.push(format!(
                        "Contract {} broadcasted in mempool (txid: {}, nonce: {})",
                        contract.name.as_ref().unwrap(),
                        txid,
                        nonce
                    ));
                    contracts_being_deployed.insert((deployer_address, contract_name));
                }
                Err(err) => {
                    panic!("Unable to publish contract: {}", err);
                }
            }
        }

        while contracts_being_deployed.len() > 0 {
            let contracts = contracts_being_deployed.clone();
            for (principal, contract_name) in contracts.into_iter() {
                let res = stacks_rpc.get_contract_source(&principal, &contract_name);
                if let Ok(contract) = res {
                    contracts_being_deployed.remove(&(principal, contract_name));
                }
            }

            // Todo: use delay_between_checks instead
            std::thread::sleep(std::time::Duration::from_secs(5));
        }
    }

    // If devnet, we should be pulling all the links.

    Ok(results)
}
