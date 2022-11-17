use std::collections::BTreeMap;

use super::{FileAccessor, FileLocation};
use bip39::{Language, Mnemonic};
use chainhook_types::{BitcoinNetwork, StacksNetwork};
use clarinet_utils::get_bip39_seed_from_mnemonic;
use clarity_repl::clarity::address::AddressHashMode;
use clarity_repl::clarity::stacks_common::types::chainstate::StacksAddress;
use clarity_repl::clarity::util::hash::bytes_to_hex;
use clarity_repl::clarity::util::secp256k1::Secp256k1PublicKey;
use clarity_repl::clarity::vm::types::QualifiedContractIdentifier;
use libsecp256k1::{PublicKey, SecretKey};
use tiny_hderive::bip32::ExtendedPrivKey;
use toml::value::Value;

pub const DEFAULT_DERIVATION_PATH: &str = "m/44'/5757'/0'/0/0";
pub const DEFAULT_BITCOIN_NODE_IMAGE: &str = "quay.io/hirosystems/bitcoind:devnet-v2";
pub const DEFAULT_STACKS_NODE_IMAGE: &str = "quay.io/hirosystems/stacks-node:devnet-v2";
pub const DEFAULT_STACKS_NODE_NEXT_IMAGE: &str = "quay.io/hirosystems/stacks-node:devnet-v3";
pub const DEFAULT_BITCOIN_EXPLORER_IMAGE: &str = "quay.io/hirosystems/bitcoin-explorer:devnet";
pub const DEFAULT_STACKS_API_IMAGE: &str = "blockstack/stacks-blockchain-api:latest";
pub const DEFAULT_STACKS_EXPLORER_IMAGE: &str = "hirosystems/explorer:latest";
pub const DEFAULT_POSTGRES_IMAGE: &str = "postgres:14";
pub const DEFAULT_SUBNET_NODE_IMAGE: &str = "hirosystems/hyperchains:0.0.4-stretch";
pub const DEFAULT_SUBNET_CONTRACT_ID: &str = "STXMJXCJDCT4WPF2X1HE42T6ZCCK3TPMBRZ51JEG.hc-alpha";
pub const DEFAULT_STACKS_MINER_MNEMONIC: &str = "fragile loan twenty basic net assault jazz absorb diet talk art shock innocent float punch travel gadget embrace caught blossom hockey surround initial reduce";
pub const DEFAULT_FAUCET_MNEMONIC: &str = "shadow private easily thought say logic fault paddle word top book during ignore notable orange flight clock image wealth health outside kitten belt reform";
pub const DEFAULT_SUBNET_MNEMONIC: &str = "female adjust gallery certain visit token during great side clown fitness like hurt clip knife warm bench start reunion globe detail dream depend fortune";
#[cfg(unix)]
pub const DEFAULT_DOCKER_SOCKET: &str = "unix:///var/run/docker.sock";
#[cfg(windows)]
pub const DEFAULT_DOCKER_SOCKET: &str = "npipe:////./pipe/docker_engine";
#[cfg(target_family = "wasm")]
pub const DEFAULT_DOCKER_SOCKET: &str = "/var/run/docker.sock";

pub const DEFAULT_EPOCH_2_0: u64 = 100;
pub const DEFAULT_EPOCH_2_05: u64 = 107;
pub const DEFAULT_EPOCH_2_1: u64 = 114;

#[derive(Serialize, Deserialize, Debug)]
pub struct NetworkManifestFile {
    network: NetworkConfigFile,
    accounts: Option<Value>,
    devnet: Option<DevnetConfigFile>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NetworkConfigFile {
    name: String,
    node_rpc_address: Option<String>,
    stacks_node_rpc_address: Option<String>,
    bitcoin_node_rpc_address: Option<String>,
    deployment_fee_rate: Option<u64>,
    sats_per_bytes: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct DevnetConfigFile {
    pub orchestrator_port: Option<u16>,
    pub orchestrator_control_port: Option<u16>,
    pub bitcoin_node_p2p_port: Option<u16>,
    pub bitcoin_node_rpc_port: Option<u16>,
    pub stacks_node_p2p_port: Option<u16>,
    pub stacks_node_rpc_port: Option<u16>,
    pub stacks_node_events_observers: Option<Vec<String>>,
    pub stacks_node_env_vars: Option<Vec<String>>,
    pub stacks_api_env_vars: Option<Vec<String>>,
    pub stacks_explorer_env_vars: Option<Vec<String>>,
    pub stacks_api_port: Option<u16>,
    pub stacks_api_events_port: Option<u16>,
    pub bitcoin_explorer_port: Option<u16>,
    pub stacks_explorer_port: Option<u16>,
    pub bitcoin_node_username: Option<String>,
    pub bitcoin_node_password: Option<String>,
    pub miner_mnemonic: Option<String>,
    pub miner_derivation_path: Option<String>,
    pub faucet_mnemonic: Option<String>,
    pub faucet_derivation_path: Option<String>,
    pub bitcoin_controller_block_time: Option<u32>,
    pub bitcoin_controller_automining_disabled: Option<bool>,
    pub working_dir: Option<String>,
    pub postgres_port: Option<u16>,
    pub postgres_username: Option<String>,
    pub postgres_password: Option<String>,
    pub stacks_api_postgres_database: Option<String>,
    pub subnet_api_postgres_database: Option<String>,
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
    pub enable_subnet_node: Option<bool>,
    pub subnet_node_image_url: Option<String>,
    pub subnet_leader_mnemonic: Option<String>,
    pub subnet_leader_derivation_path: Option<String>,
    pub subnet_node_p2p_port: Option<u16>,
    pub subnet_node_rpc_port: Option<u16>,
    pub subnet_events_ingestion_port: Option<u16>,
    pub subnet_node_events_observers: Option<Vec<String>>,
    pub subnet_contract_id: Option<String>,
    pub subnet_api_image_url: Option<String>,
    pub subnet_api_port: Option<u16>,
    pub subnet_api_events_port: Option<u16>,
    pub disable_subnet_api: Option<bool>,
    pub docker_host: Option<String>,
    pub components_host: Option<String>,
    pub enable_next_features: Option<bool>,
    pub epoch_2_0: Option<u64>,
    pub epoch_2_05: Option<u64>,
    pub epoch_2_1: Option<u64>,
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
pub struct NetworkManifest {
    pub network: NetworkConfig,
    pub accounts: BTreeMap<String, AccountConfig>,
    pub devnet: Option<DevnetConfig>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NetworkConfig {
    name: String,
    pub stacks_node_rpc_address: Option<String>,
    pub bitcoin_node_rpc_address: Option<String>,
    pub deployment_fee_rate: u64,
    pub sats_per_bytes: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DevnetConfig {
    pub orchestrator_ingestion_port: u16,
    pub orchestrator_control_port: u16,
    pub bitcoin_node_p2p_port: u16,
    pub bitcoin_node_rpc_port: u16,
    pub bitcoin_node_username: String,
    pub bitcoin_node_password: String,
    pub stacks_node_p2p_port: u16,
    pub stacks_node_rpc_port: u16,
    pub stacks_node_events_observers: Vec<String>,
    pub stacks_node_env_vars: Vec<String>,
    pub stacks_api_port: u16,
    pub stacks_api_events_port: u16,
    pub stacks_api_env_vars: Vec<String>,
    pub stacks_explorer_port: u16,
    pub stacks_explorer_env_vars: Vec<String>,
    pub bitcoin_explorer_port: u16,
    pub bitcoin_controller_block_time: u32,
    pub bitcoin_controller_automining_disabled: bool,
    pub miner_stx_address: String,
    pub miner_secret_key_hex: String,
    pub miner_btc_address: String,
    pub miner_mnemonic: String,
    pub miner_derivation_path: String,
    pub faucet_stx_address: String,
    pub faucet_secret_key_hex: String,
    pub faucet_btc_address: String,
    pub faucet_mnemonic: String,
    pub faucet_derivation_path: String,
    pub working_dir: String,
    pub postgres_port: u16,
    pub postgres_username: String,
    pub postgres_password: String,
    pub stacks_api_postgres_database: String,
    pub subnet_api_postgres_database: String,
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
    pub enable_subnet_node: bool,
    pub subnet_node_image_url: String,
    pub subnet_leader_stx_address: String,
    pub subnet_leader_secret_key_hex: String,
    pub subnet_leader_btc_address: String,
    pub subnet_leader_mnemonic: String,
    pub subnet_leader_derivation_path: String,
    pub subnet_node_p2p_port: u16,
    pub subnet_node_rpc_port: u16,
    pub subnet_events_ingestion_port: u16,
    pub subnet_node_events_observers: Vec<String>,
    pub subnet_contract_id: String,
    pub remapped_subnet_contract_id: String,
    pub subnet_api_image_url: String,
    pub subnet_api_port: u16,
    pub subnet_api_events_port: u16,
    pub disable_subnet_api: bool,
    pub docker_host: String,
    pub components_host: String,
    pub enable_next_features: bool,
    pub epoch_2_0: u64,
    pub epoch_2_05: u64,
    pub epoch_2_1: u64,
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
    pub stx_address: String,
    pub btc_address: String,
    pub is_mainnet: bool,
}

impl NetworkManifest {
    pub fn from_project_manifest_location(
        project_manifest_location: &FileLocation,
        networks: &(BitcoinNetwork, StacksNetwork),
        cache_location: Option<&FileLocation>,
    ) -> Result<NetworkManifest, String> {
        let network_manifest_location =
            project_manifest_location.get_network_manifest_location(&networks.1)?;
        NetworkManifest::from_location(&network_manifest_location, networks, cache_location)
    }

    pub async fn from_project_manifest_location_using_file_accessor(
        location: &FileLocation,
        networks: &(BitcoinNetwork, StacksNetwork),
        file_accessor: &Box<dyn FileAccessor>,
    ) -> Result<NetworkManifest, String> {
        let mut network_manifest_location = location.get_parent_location()?;
        network_manifest_location.append_path("settings/Devnet.toml")?;
        let content = file_accessor
            .read_file(network_manifest_location.to_string())
            .await?;

        let mut network_manifest_file: NetworkManifestFile =
            toml::from_slice(&content.as_bytes()).unwrap();
        NetworkManifest::from_network_manifest_file(&mut network_manifest_file, networks, None)
    }

    pub fn from_location(
        location: &FileLocation,
        networks: &(BitcoinNetwork, StacksNetwork),
        cache_location: Option<&FileLocation>,
    ) -> Result<NetworkManifest, String> {
        let network_manifest_file_content = location.read_content()?;
        let mut network_manifest_file: NetworkManifestFile =
            toml::from_slice(&network_manifest_file_content[..]).unwrap();
        NetworkManifest::from_network_manifest_file(
            &mut network_manifest_file,
            networks,
            cache_location,
        )
    }

    pub fn from_network_manifest_file(
        network_manifest_file: &mut NetworkManifestFile,
        networks: &(BitcoinNetwork, StacksNetwork),
        cache_location: Option<&FileLocation>,
    ) -> Result<NetworkManifest, String> {
        let stacks_node_rpc_address = match (
            &network_manifest_file.network.node_rpc_address,
            &network_manifest_file.network.stacks_node_rpc_address,
        ) {
            (Some(_), Some(url)) | (None, Some(url)) | (Some(url), None) => Some(url.clone()),
            _ => None,
        };
        let network = NetworkConfig {
            name: network_manifest_file.network.name.clone(),
            stacks_node_rpc_address: stacks_node_rpc_address,
            bitcoin_node_rpc_address: network_manifest_file
                .network
                .bitcoin_node_rpc_address
                .clone(),
            deployment_fee_rate: network_manifest_file
                .network
                .deployment_fee_rate
                .unwrap_or(10),
            sats_per_bytes: network_manifest_file.network.sats_per_bytes.unwrap_or(10),
        };

        let mut accounts = BTreeMap::new();
        let is_mainnet = networks.1.is_mainnet();

        match &network_manifest_file.accounts {
            Some(Value::Table(entries)) => {
                for (account_name, account_settings) in entries.iter() {
                    match account_settings {
                        Value::Table(account_settings) => {
                            let balance = match account_settings.get("balance") {
                                Some(Value::Integer(balance)) => *balance as u64,
                                _ => 0,
                            };

                            let mnemonic = match account_settings.get("mnemonic") {
                                Some(Value::String(words)) => {
                                    match Mnemonic::parse_in_normalized(Language::English, words) {
                                        Ok(result) => result.to_string(),
                                        Err(e) => {
                                            return Err(format!(
                                                "mnemonic for wallet '{}' invalid: {}",
                                                account_name,
                                                e.to_string()
                                            ));
                                        }
                                    }
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

                            let (stx_address, btc_address, _) =
                                compute_addresses(&mnemonic, &derivation, networks);

                            accounts.insert(
                                account_name.to_string(),
                                AccountConfig {
                                    label: account_name.to_string(),
                                    mnemonic: mnemonic.to_string(),
                                    derivation,
                                    balance,
                                    stx_address,
                                    btc_address,
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

        let devnet = if networks.1.is_devnet() {
            let mut devnet_config = match network_manifest_file.devnet.take() {
                Some(conf) => conf,
                _ => DevnetConfigFile::default(),
            };

            let now = clarity_repl::clarity::util::get_epoch_time_secs();
            let devnet_dir = format!("stacks-devnet-{}/", now);
            let default_working_dir = match cache_location {
                Some(cache_location) => {
                    let mut devnet_location = cache_location.clone();
                    let _ = devnet_location.append_path(&devnet_dir);
                    devnet_location.to_string()
                }
                None => {
                    let mut dir = std::env::temp_dir();
                    dir.push(devnet_dir);
                    dir.display().to_string()
                }
            };

            let miner_mnemonic = devnet_config
                .miner_mnemonic
                .take()
                .unwrap_or(DEFAULT_STACKS_MINER_MNEMONIC.to_string());
            let miner_derivation_path = devnet_config
                .miner_derivation_path
                .take()
                .unwrap_or(DEFAULT_DERIVATION_PATH.to_string());
            let (miner_stx_address, miner_btc_address, miner_secret_key_hex) =
                compute_addresses(&miner_mnemonic, &miner_derivation_path, networks);

            let faucet_mnemonic = devnet_config
                .faucet_mnemonic
                .take()
                .unwrap_or(DEFAULT_FAUCET_MNEMONIC.to_string());
            let faucet_derivation_path = devnet_config
                .faucet_derivation_path
                .take()
                .unwrap_or(DEFAULT_DERIVATION_PATH.to_string());
            let (faucet_stx_address, faucet_btc_address, faucet_secret_key_hex) =
                compute_addresses(&faucet_mnemonic, &faucet_derivation_path, networks);

            let subnet_leader_mnemonic = devnet_config
                .subnet_leader_mnemonic
                .take()
                .unwrap_or(DEFAULT_SUBNET_MNEMONIC.to_string());
            let subnet_leader_derivation_path = devnet_config
                .subnet_leader_derivation_path
                .take()
                .unwrap_or(DEFAULT_DERIVATION_PATH.to_string());
            let (
                subnet_leader_stx_address,
                subnet_leader_btc_address,
                subnet_leader_secret_key_hex,
            ) = compute_addresses(
                &subnet_leader_mnemonic,
                &subnet_leader_derivation_path,
                networks,
            );

            let enable_subnet_node = devnet_config.enable_subnet_node.unwrap_or(false);
            let subnet_events_ingestion_port =
                devnet_config.subnet_events_ingestion_port.unwrap_or(30445);

            let mut stacks_node_events_observers = devnet_config
                .stacks_node_events_observers
                .take()
                .unwrap_or(vec![]);

            if enable_subnet_node {
                // add subnet node to stacks-node observers
                let label = "subnet-leader";
                stacks_node_events_observers.push(format!(
                    "host.docker.internal:{}",
                    subnet_events_ingestion_port
                ));
                accounts.insert(
                    label.to_string(),
                    AccountConfig {
                        label: label.to_string(),
                        mnemonic: subnet_leader_mnemonic.clone(),
                        derivation: subnet_leader_derivation_path.clone(),
                        balance: super::DEFAULT_DEVNET_BALANCE,
                        stx_address: subnet_leader_stx_address.clone(),
                        btc_address: subnet_leader_btc_address.clone(),
                        is_mainnet,
                    },
                );
            }
            let subnet_contract_id = devnet_config
                .subnet_contract_id
                .unwrap_or(DEFAULT_SUBNET_CONTRACT_ID.to_string());
            let contract_id = QualifiedContractIdentifier::parse(&subnet_contract_id)
                .expect("subnet contract_id invalid");
            let default_deployer = accounts
                .get("deployer")
                .expect("default deployer account unavailable");
            let remapped_subnet_contract_id =
                format!("{}.{}", default_deployer.stx_address, contract_id.name);
            let enable_next_features = devnet_config.enable_next_features.unwrap_or(false);

            let mut config = DevnetConfig {
                orchestrator_ingestion_port: devnet_config.orchestrator_port.unwrap_or(20445),
                orchestrator_control_port: devnet_config.orchestrator_control_port.unwrap_or(20446),
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
                    .unwrap_or(90_000),
                bitcoin_controller_automining_disabled: devnet_config
                    .bitcoin_controller_automining_disabled
                    .unwrap_or(false),
                stacks_node_p2p_port: devnet_config.stacks_node_p2p_port.unwrap_or(20444),
                stacks_node_rpc_port: devnet_config.stacks_node_rpc_port.unwrap_or(20443),
                stacks_node_events_observers,
                stacks_api_port: devnet_config.stacks_api_port.unwrap_or(3999),
                stacks_api_events_port: devnet_config.stacks_api_events_port.unwrap_or(3700),
                stacks_explorer_port: devnet_config.stacks_explorer_port.unwrap_or(8000),
                bitcoin_explorer_port: devnet_config.bitcoin_explorer_port.unwrap_or(8001),
                miner_btc_address,
                miner_stx_address,
                miner_mnemonic,
                miner_secret_key_hex,
                miner_derivation_path,
                faucet_btc_address,
                faucet_stx_address,
                faucet_mnemonic,
                faucet_secret_key_hex,
                faucet_derivation_path,
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
                stacks_api_postgres_database: devnet_config
                    .stacks_api_postgres_database
                    .take()
                    .unwrap_or("stacks_api".to_string()),
                subnet_api_postgres_database: devnet_config
                    .subnet_api_postgres_database
                    .take()
                    .unwrap_or("subnet_api".to_string()),
                execute_script: devnet_config.execute_script.take().unwrap_or(vec![]),
                bitcoin_node_image_url: devnet_config
                    .bitcoin_node_image_url
                    .take()
                    .unwrap_or(DEFAULT_BITCOIN_NODE_IMAGE.to_string()),
                stacks_node_image_url: devnet_config.stacks_node_image_url.take().unwrap_or(
                    match enable_next_features {
                        true => DEFAULT_STACKS_NODE_NEXT_IMAGE,
                        false => DEFAULT_STACKS_NODE_IMAGE,
                    }
                    .to_string(),
                ),
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
                enable_subnet_node,
                subnet_node_image_url: devnet_config
                    .subnet_node_image_url
                    .take()
                    .unwrap_or(DEFAULT_SUBNET_NODE_IMAGE.to_string()),
                subnet_leader_btc_address,
                subnet_leader_stx_address,
                subnet_leader_mnemonic,
                subnet_leader_secret_key_hex,
                subnet_leader_derivation_path,
                subnet_node_p2p_port: devnet_config.stacks_node_p2p_port.unwrap_or(30444),
                subnet_node_rpc_port: devnet_config.stacks_node_rpc_port.unwrap_or(30443),
                subnet_events_ingestion_port,
                subnet_node_events_observers: devnet_config
                    .subnet_node_events_observers
                    .take()
                    .unwrap_or(vec![]),
                subnet_contract_id,
                remapped_subnet_contract_id,
                subnet_api_image_url: devnet_config
                    .subnet_api_image_url
                    .take()
                    .unwrap_or(DEFAULT_STACKS_API_IMAGE.to_string()),
                subnet_api_port: devnet_config.subnet_api_port.unwrap_or(13999),
                subnet_api_events_port: devnet_config.stacks_api_events_port.unwrap_or(13700),
                disable_subnet_api: devnet_config.disable_subnet_api.unwrap_or(true),
                docker_host: devnet_config
                    .docker_host
                    .unwrap_or(DEFAULT_DOCKER_SOCKET.into()),
                components_host: devnet_config.components_host.unwrap_or("127.0.0.1".into()),
                epoch_2_0: devnet_config.epoch_2_0.unwrap_or(DEFAULT_EPOCH_2_0),
                epoch_2_05: devnet_config.epoch_2_05.unwrap_or(DEFAULT_EPOCH_2_05),
                epoch_2_1: devnet_config.epoch_2_1.unwrap_or(DEFAULT_EPOCH_2_1),
                stacks_node_env_vars: devnet_config.stacks_node_env_vars.take().unwrap_or(vec![]),
                stacks_api_env_vars: devnet_config.stacks_api_env_vars.take().unwrap_or(vec![]),
                stacks_explorer_env_vars: devnet_config
                    .stacks_explorer_env_vars
                    .take()
                    .unwrap_or(vec![]),
                enable_next_features,
            };
            if !config.disable_stacks_api && config.disable_stacks_api {
                config.disable_stacks_api = false;
            }
            Some(config)
        } else {
            None
        };
        let config = NetworkManifest {
            network,
            accounts,
            devnet,
        };

        Ok(config)
    }
}

pub fn compute_addresses(
    mnemonic: &str,
    derivation_path: &str,
    networks: &(BitcoinNetwork, StacksNetwork),
) -> (String, String, String) {
    let bip39_seed = match get_bip39_seed_from_mnemonic(&mnemonic, "") {
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
    let version = if networks.1.is_mainnet() {
        clarity_repl::clarity::address::C32_ADDRESS_VERSION_MAINNET_SINGLESIG
    } else {
        clarity_repl::clarity::address::C32_ADDRESS_VERSION_TESTNET_SINGLESIG
    };

    let stx_address = StacksAddress::from_public_keys(
        version,
        &AddressHashMode::SerializeP2PKH,
        1,
        &vec![pub_key],
    )
    .unwrap();

    let btc_address = compute_btc_address(&public_key, &networks.0);

    (stx_address.to_string(), btc_address, miner_secret_key_hex)
}

#[cfg(not(feature = "wasm"))]
use bitcoin;

#[cfg(not(feature = "wasm"))]
fn compute_btc_address(public_key: &PublicKey, network: &BitcoinNetwork) -> String {
    let public_key = bitcoin::PublicKey::from_slice(&public_key.serialize_compressed())
        .expect("Unable to recreate public key");
    let btc_address = bitcoin::Address::p2pkh(
        &public_key,
        match network {
            BitcoinNetwork::Regtest => bitcoin::Network::Regtest,
            BitcoinNetwork::Testnet => bitcoin::Network::Testnet,
            BitcoinNetwork::Mainnet => bitcoin::Network::Bitcoin,
        },
    );
    btc_address.to_string()
}

#[cfg(feature = "wasm")]
fn compute_btc_address(_public_key: &PublicKey, _network: &BitcoinNetwork) -> String {
    format!("__not_implemented__")
}
