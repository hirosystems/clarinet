use super::StacksNetwork;
use crate::utils::mnemonic;
use bip39::{Language, Mnemonic};
use clarity_repl::clarity::util::hash::bytes_to_hex;
use clarity_repl::clarity::util::secp256k1::Secp256k1PublicKey;
use clarity_repl::clarity::util::StacksAddress;
use libsecp256k1::{PublicKey, SecretKey};
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::{collections::BTreeMap, fs::File};
use tiny_hderive::bip32::ExtendedPrivKey;
use toml::value::Value;

pub const DEFAULT_DERIVATION_PATH: &str = "m/44'/5757'/0'/0/0";
pub const DEFAULT_BITCOIN_NODE_IMAGE: &str = "quay.io/hirosystems/bitcoind:devnet-v2";
pub const DEFAULT_STACKS_NODE_IMAGE: &str = "quay.io/hirosystems/stacks-node:devnet-v2";
pub const DEFAULT_BITCOIN_EXPLORER_IMAGE: &str = "quay.io/hirosystems/bitcoin-explorer:devnet";
pub const DEFAULT_STACKS_API_IMAGE: &str = "blockstack/stacks-blockchain-api:latest";
pub const DEFAULT_STACKS_EXPLORER_IMAGE: &str = "blockstack/explorer:latest";
pub const DEFAULT_POSTGRES_IMAGE: &str = "postgres:alpine";

#[derive(Serialize, Deserialize, Debug)]
pub struct ChainConfigFile {
    network: NetworkConfigFile,
    accounts: Option<Value>,
    devnet: Option<DevnetConfigFile>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NetworkConfigFile {
    name: String,
    node_rpc_address: Option<String>,
    deployment_fee_rate: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct DevnetConfigFile {
    pub orchestrator_port: Option<u16>,
    pub bitcoin_node_p2p_port: Option<u16>,
    pub bitcoin_node_rpc_port: Option<u16>,
    pub stacks_node_p2p_port: Option<u16>,
    pub stacks_node_rpc_port: Option<u16>,
    pub stacks_node_events_observers: Option<Vec<String>>,
    pub stacks_api_port: Option<u16>,
    pub stacks_api_events_port: Option<u16>,
    pub bitcoin_explorer_port: Option<u16>,
    pub stacks_explorer_port: Option<u16>,
    pub bitcoin_node_username: Option<String>,
    pub bitcoin_node_password: Option<String>,
    pub miner_mnemonic: Option<String>,
    pub miner_derivation_path: Option<String>,
    pub bitcoin_controller_block_time: Option<u32>,
    pub bitcoin_controller_automining_disabled: Option<bool>,
    pub working_dir: Option<String>,
    pub postgres_port: Option<u16>,
    pub postgres_username: Option<String>,
    pub postgres_password: Option<String>,
    pub postgres_database: Option<String>,
    pub pox_stacking_orders: Option<Vec<PoxStackingOrder>>,
    pub execute_script: Option<Vec<ExecuteScript>>,
    pub bitcoin_node_image_url: Option<String>,
    pub bitcoin_explorer_image_url: Option<String>,
    pub stacks_node_image_url: Option<String>,
    pub stacks_api_image_url: Option<String>,
    pub stacks_explorer_image_url: Option<String>,
    pub postgres_image_url: Option<String>,
    pub disable_bitcoin_explorer: Option<bool>,
    pub disable_stacks_explorer: Option<bool>,
    pub disable_stacks_api: Option<bool>,
    pub bind_containers_volumes: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PoxStackingOrderFile {
    pub start_at_cycle: u32,
    pub end_at_cycle: u32,
    pub amount_locked: u32,
    pub wallet_label: String,
    pub bitcoin_address: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecuteScript {
    pub script: String,
    pub allow_wallets: bool,
    pub allow_write: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AccountConfigFile {
    mnemonic: Option<String>,
    derivation: Option<String>,
    balance: Option<u64>,
    is_mainnet: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ChainConfig {
    pub network: NetworkConfig,
    pub accounts: BTreeMap<String, AccountConfig>,
    pub devnet: Option<DevnetConfig>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NetworkConfig {
    name: String,
    pub node_rpc_address: Option<String>,
    pub deployment_fee_rate: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DevnetConfig {
    pub orchestrator_port: u16,
    pub bitcoin_node_p2p_port: u16,
    pub bitcoin_node_rpc_port: u16,
    pub bitcoin_node_username: String,
    pub bitcoin_node_password: String,
    pub stacks_node_p2p_port: u16,
    pub stacks_node_rpc_port: u16,
    pub stacks_node_events_observers: Vec<String>,
    pub stacks_api_port: u16,
    pub stacks_api_events_port: u16,
    pub stacks_explorer_port: u16,
    pub bitcoin_explorer_port: u16,
    pub bitcoin_controller_block_time: u32,
    pub bitcoin_controller_automining_disabled: bool,
    pub miner_stx_address: String,
    pub miner_secret_key_hex: String,
    pub miner_btc_address: String,
    pub miner_mnemonic: String,
    pub miner_derivation_path: String,
    pub working_dir: String,
    pub postgres_port: u16,
    pub postgres_username: String,
    pub postgres_password: String,
    pub postgres_database: String,
    pub pox_stacking_orders: Vec<PoxStackingOrder>,
    pub execute_script: Vec<ExecuteScript>,
    pub bitcoin_node_image_url: String,
    pub stacks_node_image_url: String,
    pub stacks_api_image_url: String,
    pub stacks_explorer_image_url: String,
    pub postgres_image_url: String,
    pub bitcoin_explorer_image_url: String,
    pub disable_bitcoin_explorer: bool,
    pub disable_stacks_explorer: bool,
    pub disable_stacks_api: bool,
    pub bind_containers_volumes: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PoxStackingOrder {
    pub start_at_cycle: u32,
    pub duration: u32,
    pub wallet: String,
    pub slots: u64,
    pub btc_address: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AccountConfig {
    pub label: String,
    pub mnemonic: String,
    pub derivation: String,
    pub balance: u64,
    pub address: String,
    pub is_mainnet: bool,
}

impl ChainConfig {
    #[allow(non_fmt_panics)]
    pub fn from_manifest_path(
        manifest_path: &PathBuf,
        network: &Option<StacksNetwork>,
    ) -> ChainConfig {
        let mut chain_config_path = manifest_path.clone();
        chain_config_path.pop();
        chain_config_path.push("settings");
        chain_config_path.push(match network {
            None | Some(StacksNetwork::Devnet) => "Devnet.toml",
            Some(StacksNetwork::Testnet) => "Testnet.toml",
            Some(StacksNetwork::Mainnet) => "Mainnet.toml",
        });
        let chain_config = ChainConfig::from_path(
            &chain_config_path,
            match network {
                None => &StacksNetwork::Devnet,
                Some(ref network) => network,
            },
        );
        chain_config
    }

    #[allow(non_fmt_panics)]
    pub fn from_path(path: &PathBuf, network: &StacksNetwork) -> ChainConfig {
        let path = match File::open(path) {
            Ok(path) => path,
            Err(_) => {
                let error = format!("Unable to open file {:?}", path.to_str());
                panic!("{}", error)
            }
        };
        let mut chain_config_file_reader = BufReader::new(path);
        let mut chain_config_file_buffer = vec![];
        chain_config_file_reader
            .read_to_end(&mut chain_config_file_buffer)
            .unwrap();
        let mut chain_config_file: ChainConfigFile =
            toml::from_slice(&chain_config_file_buffer[..]).unwrap();
        ChainConfig::from_chain_config_file(&mut chain_config_file, network)
    }

    pub fn from_chain_config_file(
        chain_config_file: &mut ChainConfigFile,
        env: &StacksNetwork,
    ) -> ChainConfig {
        let network = NetworkConfig {
            name: chain_config_file.network.name.clone(),
            node_rpc_address: chain_config_file.network.node_rpc_address.clone(),
            deployment_fee_rate: chain_config_file.network.deployment_fee_rate.unwrap_or(10),
        };

        let mut accounts = BTreeMap::new();
        let is_mainnet = network.name == "mainnet".to_string();

        match &chain_config_file.accounts {
            Some(Value::Table(entries)) => {
                for (account_name, account_settings) in entries.iter() {
                    match account_settings {
                        Value::Table(account_settings) => {
                            let balance = match account_settings.get("balance") {
                                Some(Value::Integer(balance)) => *balance as u64,
                                _ => 0,
                            };

                            let is_mainnet = env == &StacksNetwork::Mainnet;

                            let mnemonic = match account_settings.get("mnemonic") {
                                Some(Value::String(words)) => {
                                    Mnemonic::parse_in_normalized(Language::English, words)
                                        .unwrap()
                                        .to_string()
                                }
                                _ => {
                                    let entropy = &[
                                        0x33, 0xE4, 0x6B, 0xB1, 0x3A, 0x74, 0x6E, 0xA4, 0x1C, 0xDD,
                                        0xE4, 0x5C, 0x90, 0x84, 0x6A, 0x79,
                                    ]; // TODO(lgalabru): rand
                                    Mnemonic::from_entropy(entropy).unwrap().to_string()
                                }
                            };

                            let derivation = match account_settings.get("derivation") {
                                Some(Value::String(derivation)) => derivation.to_string(),
                                _ => DEFAULT_DERIVATION_PATH.to_string(),
                            };

                            let (address, _, _) =
                                compute_addresses(&mnemonic, &derivation, is_mainnet);

                            accounts.insert(
                                account_name.to_string(),
                                AccountConfig {
                                    label: account_name.to_string(),
                                    mnemonic: mnemonic.to_string(),
                                    derivation,
                                    balance,
                                    address,
                                    is_mainnet,
                                },
                            );
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        };

        let devnet = if chain_config_file.network.name == "devnet" {
            let mut devnet_config = match chain_config_file.devnet.take() {
                Some(conf) => conf,
                _ => DevnetConfigFile::default(),
            };

            let now = clarity_repl::clarity::util::get_epoch_time_secs();
            let mut dir = std::env::temp_dir();
            dir.push(format!("stacks-devnet-{}/", now));
            let default_working_dir = dir.display().to_string();

            let miner_mnemonic = devnet_config.miner_mnemonic.take().unwrap_or("fragile loan twenty basic net assault jazz absorb diet talk art shock innocent float punch travel gadget embrace caught blossom hockey surround initial reduce".to_string());
            let miner_derivation_path = devnet_config
                .miner_derivation_path
                .take()
                .unwrap_or(DEFAULT_DERIVATION_PATH.to_string());
            let (miner_stx_address, miner_btc_address, miner_secret_key_hex) =
                compute_addresses(&miner_mnemonic, &miner_derivation_path, is_mainnet);

            let mut config = DevnetConfig {
                orchestrator_port: devnet_config.orchestrator_port.unwrap_or(20445),
                bitcoin_node_p2p_port: devnet_config.bitcoin_node_p2p_port.unwrap_or(18444),
                bitcoin_node_rpc_port: devnet_config.bitcoin_node_rpc_port.unwrap_or(18443),
                bitcoin_node_username: devnet_config
                    .bitcoin_node_username
                    .take()
                    .unwrap_or("devnet".to_string()),
                bitcoin_node_password: devnet_config
                    .bitcoin_node_password
                    .take()
                    .unwrap_or("devnet".to_string()),
                bitcoin_controller_block_time: devnet_config
                    .bitcoin_controller_block_time
                    .unwrap_or(30_000),
                bitcoin_controller_automining_disabled: devnet_config
                    .bitcoin_controller_automining_disabled
                    .unwrap_or(false),
                stacks_node_p2p_port: devnet_config.stacks_node_p2p_port.unwrap_or(20444),
                stacks_node_rpc_port: devnet_config.stacks_node_rpc_port.unwrap_or(20443),
                stacks_node_events_observers: devnet_config
                    .stacks_node_events_observers
                    .take()
                    .unwrap_or(vec![]),
                stacks_api_port: devnet_config.stacks_api_port.unwrap_or(3999),
                stacks_api_events_port: devnet_config.stacks_api_events_port.unwrap_or(3700),
                stacks_explorer_port: devnet_config.stacks_explorer_port.unwrap_or(8000),
                bitcoin_explorer_port: devnet_config.bitcoin_explorer_port.unwrap_or(8001),
                miner_btc_address,
                miner_stx_address,
                miner_mnemonic,
                miner_secret_key_hex,
                miner_derivation_path,
                working_dir: devnet_config
                    .working_dir
                    .take()
                    .unwrap_or(default_working_dir),
                postgres_port: devnet_config.postgres_port.unwrap_or(5432),
                postgres_username: devnet_config
                    .postgres_username
                    .take()
                    .unwrap_or("postgres".to_string()),
                postgres_password: devnet_config
                    .postgres_password
                    .take()
                    .unwrap_or("postgres".to_string()),
                postgres_database: devnet_config
                    .postgres_database
                    .take()
                    .unwrap_or("postgres".to_string()),
                execute_script: devnet_config.execute_script.take().unwrap_or(vec![]),
                bitcoin_node_image_url: devnet_config
                    .bitcoin_node_image_url
                    .take()
                    .unwrap_or(DEFAULT_BITCOIN_NODE_IMAGE.to_string()),
                stacks_node_image_url: devnet_config
                    .stacks_node_image_url
                    .take()
                    .unwrap_or(DEFAULT_STACKS_NODE_IMAGE.to_string()),
                stacks_api_image_url: devnet_config
                    .stacks_api_image_url
                    .take()
                    .unwrap_or(DEFAULT_STACKS_API_IMAGE.to_string()),
                postgres_image_url: devnet_config
                    .postgres_image_url
                    .take()
                    .unwrap_or(DEFAULT_POSTGRES_IMAGE.to_string()),
                stacks_explorer_image_url: devnet_config
                    .stacks_explorer_image_url
                    .take()
                    .unwrap_or(DEFAULT_STACKS_EXPLORER_IMAGE.to_string()),
                bitcoin_explorer_image_url: devnet_config
                    .bitcoin_explorer_image_url
                    .take()
                    .unwrap_or(DEFAULT_BITCOIN_EXPLORER_IMAGE.to_string()),
                pox_stacking_orders: devnet_config.pox_stacking_orders.take().unwrap_or(vec![]),
                disable_bitcoin_explorer: devnet_config.disable_bitcoin_explorer.unwrap_or(false),
                disable_stacks_api: devnet_config.disable_stacks_api.unwrap_or(false),
                disable_stacks_explorer: devnet_config.disable_stacks_explorer.unwrap_or(false),
                bind_containers_volumes: devnet_config.bind_containers_volumes.unwrap_or(false),
            };
            if !config.disable_stacks_api && config.disable_stacks_api {
                config.disable_stacks_api = false;
            }

            Some(config)
        } else {
            None
        };
        let config = ChainConfig {
            network,
            accounts,
            devnet,
        };

        config
    }
}

pub fn compute_addresses(
    mnemonic: &str,
    derivation_path: &str,
    mainnet: bool,
) -> (String, String, String) {
    let bip39_seed = match mnemonic::get_bip39_seed_from_mnemonic(&mnemonic, "") {
        Ok(bip39_seed) => bip39_seed,
        Err(_) => panic!(),
    };

    let ext = ExtendedPrivKey::derive(&bip39_seed[..], derivation_path).unwrap();

    let secret_key = SecretKey::parse_slice(&ext.secret()).unwrap();

    // Enforce a 33 bytes secret key format, expected by Stacks
    let mut secret_key_bytes = secret_key.serialize().to_vec();
    secret_key_bytes.push(1);
    let miner_secret_key_hex = bytes_to_hex(&secret_key_bytes);

    let public_key = PublicKey::from_secret_key(&secret_key);
    let pub_key = Secp256k1PublicKey::from_slice(&public_key.serialize_compressed()).unwrap();
    let version = if mainnet {
        clarity_repl::clarity::util::C32_ADDRESS_VERSION_MAINNET_SINGLESIG
    } else {
        clarity_repl::clarity::util::C32_ADDRESS_VERSION_TESTNET_SINGLESIG
    };

    let stx_address = StacksAddress::from_public_key(version, pub_key).unwrap();

    // TODO(ludo): de-hardcode this
    let btc_address = "n3GRiDLKWuKLCw1DZmV75W1mE35qmW2tQm".to_string();

    (stx_address.to_string(), btc_address, miner_secret_key_hex)
}
