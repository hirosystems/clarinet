use std::{collections::BTreeMap, fs::File};
use std::path::PathBuf;
use std::{
    io::{BufReader, Read},
};
use toml::value::Value;
use bip39::{Mnemonic};
use crate::utils::mnemonic;
use clarity_repl::clarity::util::StacksAddress;
use clarity_repl::clarity::util::secp256k1::Secp256k1PublicKey;

const DEFAULT_DERIVATION_PATH: &str = "m/44'/5757'/0'/0/0";

#[derive(Serialize, Deserialize, Debug)]
pub struct ChainConfigFile {
    network: NetworkConfigFile,
    accounts: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NetworkConfigFile {
    name: String,
    node_rpc_address: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AccountConfigFile {
    mnemonic: Option<String>,
    derivation: Option<String>,
    balance: Option<u64>,
    is_mainnet: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChainConfig {
    pub network: NetworkConfig,
    pub accounts: BTreeMap<String, AccountConfig>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NetworkConfig {
    name: String,
    node_rpc_address: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AccountConfig {
    pub mnemonic: String,
    pub derivation: String,
    pub balance: u64,
    pub address: String,
    pub is_mainnet: bool
}

impl ChainConfig {
    pub fn from_path(path: &PathBuf) -> ChainConfig {
        let path = File::open(path).unwrap();
        let mut config_file_reader = BufReader::new(path);
        let mut config_file_buffer = vec![];
        config_file_reader
            .read_to_end(&mut config_file_buffer)
            .unwrap();
        let config_file: ChainConfigFile = toml::from_slice(&config_file_buffer[..]).unwrap();
        ChainConfig::from_config_file(config_file)
    }

    pub fn from_config_file(config_file: ChainConfigFile) -> ChainConfig {

        let network = NetworkConfig {
            name: config_file.network.name.clone(),
            node_rpc_address: config_file.network.node_rpc_address.clone(),
        };

        let mut config = ChainConfig {
            network,
            accounts: BTreeMap::new(),
        };

        match config_file.accounts {
            Some(Value::Table(accounts)) => {
                for (account_name, account_settings) in accounts.iter() {
                    match account_settings {
                        Value::Table(account_settings) => {
                            let balance = match account_settings.get("balance") {
                                Some(Value::Integer(balance)) => *balance as u64,
                                _ => 0,
                            };

                            let is_mainnet = match account_settings.get("is_mainnet") {
                                Some(Value::Boolean(is_mainnet)) => *is_mainnet,
                                _ => false,
                            };

                            let mnemonic = match account_settings.get("mnemonic") {
                                Some(Value::String(words)) => Mnemonic::parse(words).unwrap().to_string(),
                                _ => {
                                    let entropy = &[0x33, 0xE4, 0x6B, 0xB1, 0x3A, 0x74, 0x6E, 0xA4, 0x1C, 0xDD, 0xE4, 0x5C, 0x90, 0x84, 0x6A, 0x79]; // todo(ludo): rand
                                    Mnemonic::from_entropy(entropy).unwrap().to_string()
                                }
                            };

                            let derivation = match account_settings.get("derivation") {
                                Some(Value::String(derivation)) => derivation.to_string(),
                                _ => DEFAULT_DERIVATION_PATH.to_string(),
                            }; // todo(ludo): use derivation path

                            let bip39_seed = match mnemonic::get_bip39_seed_from_mnemonic(&mnemonic, "") {
                                Ok(bip39_seed) => bip39_seed,
                                Err(_) => panic!(),
                            };
                        
                            let (_, public_key) = match mnemonic::get_hardened_child_keypair(&bip39_seed, &[888, 0, 0]) {
                                Ok(result) => result,
                                Err(_) => panic!(),
                            };

                            let public_key_hex = hex::decode(&public_key).unwrap();

                            let pub_key = Secp256k1PublicKey::from_slice(&public_key_hex, false).unwrap();
                            let version = 26; // todo(ludo): un-hardcode this
                            let address = StacksAddress::from_public_key(version, pub_key).unwrap().to_string();

                            config.accounts.insert(
                                account_name.to_string(),
                                AccountConfig {
                                    mnemonic,
                                    derivation,
                                    balance,
                                    address,
                                    is_mainnet,
                                }
                            );
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        };
        config
    }
}
