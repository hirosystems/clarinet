pub mod file;

pub use chainhook_event_observer::indexer::IndexerConfig;
use chainhook_types::{BitcoinNetwork, StacksNetwork};
pub use file::ConfigFile;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;

const DEFAULT_MAINNET_TSV_ARCHIVE: &str = "https://storage.googleapis.com/hirosystems-archive/mainnet/api/mainnet-blockchain-api-latest.tar.gz";
const DEFAULT_TESTNET_TSV_ARCHIVE: &str = "https://storage.googleapis.com/hirosystems-archive/testnet/api/testnet-blockchain-api-latest.tar.gz";

#[derive(Clone, Debug)]
pub struct Config {
    pub storage: StorageConfig,
    pub event_sources: Vec<EventSourceConfig>,
    pub chainhooks: ChainhooksConfig,
    pub network: IndexerConfig,
}

#[derive(Clone, Debug)]
pub struct StorageConfig {
    pub driver: StorageDriver,
    pub cache_path: String,
}

#[derive(Clone, Debug)]
pub enum StorageDriver {
    Redis(RedisConfig),
}

#[derive(Clone, Debug)]
pub struct RedisConfig {
    pub uri: String,
}

#[derive(Clone, Debug)]
pub enum EventSourceConfig {
    StacksNode(StacksNodeConfig),
    TsvPath(TsvPathConfig),
    TsvUrl(TsvUrlConfig),
}

#[derive(Clone, Debug)]
pub struct TsvPathConfig {
    pub file_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct TsvUrlConfig {
    pub file_url: String,
}

#[derive(Clone, Debug)]
pub struct StacksNodeConfig {
    pub host: String,
}

#[derive(Clone, Debug)]
pub struct ChainhooksConfig {
    pub max_stacks_registrations: u16,
    pub max_bitcoin_registrations: u16,
}

impl Config {
    pub fn from_file_path(file_path: &str) -> Result<Config, String> {
        let file = File::open(file_path)
            .map_err(|e| format!("unable to read file {}\n{:?}", file_path, e))?;
        let mut file_reader = BufReader::new(file);
        let mut file_buffer = vec![];
        file_reader
            .read_to_end(&mut file_buffer)
            .map_err(|e| format!("unable to read file {}\n{:?}", file_path, e))?;

        let config_file: ConfigFile = match toml::from_slice(&file_buffer) {
            Ok(s) => s,
            Err(e) => {
                return Err(format!("Config file malformatted {}", e.to_string()));
            }
        };
        Config::from_config_file(config_file)
    }

    pub fn from_config_file(config_file: ConfigFile) -> Result<Config, String> {
        let (stacks_network, bitcoin_network) = match config_file.network.mode.as_str() {
            "devnet" => (StacksNetwork::Devnet, BitcoinNetwork::Regtest),
            "testnet" => (StacksNetwork::Testnet, BitcoinNetwork::Testnet),
            "mainnet" => (StacksNetwork::Mainnet, BitcoinNetwork::Mainnet),
            _ => return Err("network.mode not supported".to_string()),
        };

        let config = Config {
            storage: StorageConfig {
                driver: StorageDriver::Redis(RedisConfig {
                    uri: config_file.storage.redis_uri.to_string(),
                }),
                cache_path: "cache".into(),
            },
            event_sources: vec![EventSourceConfig::StacksNode(StacksNodeConfig {
                host: config_file.network.stacks_node_rpc_url.to_string(),
            })],
            chainhooks: ChainhooksConfig {
                max_stacks_registrations: config_file
                    .chainhooks
                    .max_stacks_registrations
                    .unwrap_or(100),
                max_bitcoin_registrations: config_file
                    .chainhooks
                    .max_bitcoin_registrations
                    .unwrap_or(100),
            },
            network: IndexerConfig {
                stacks_node_rpc_url: config_file.network.stacks_node_rpc_url.to_string(),
                bitcoin_node_rpc_url: config_file.network.bitcoin_node_rpc_url.to_string(),
                bitcoin_node_rpc_username: config_file
                    .network
                    .bitcoin_node_rpc_username
                    .to_string(),
                bitcoin_node_rpc_password: config_file
                    .network
                    .bitcoin_node_rpc_password
                    .to_string(),
                stacks_network,
                bitcoin_network,
            },
        };
        Ok(config)
    }

    pub fn is_initial_ingestion_required(&self) -> bool {
        for source in self.event_sources.iter() {
            match source {
                EventSourceConfig::TsvUrl(_) | EventSourceConfig::TsvPath(_) => return true,
                EventSourceConfig::StacksNode(_) => {}
            }
        }
        return false;
    }

    pub fn add_local_tsv_source(&mut self, file_path: &PathBuf) {
        self.event_sources
            .push(EventSourceConfig::TsvPath(TsvPathConfig {
                file_path: file_path.clone(),
            }));
    }

    pub fn expected_redis_config(&self) -> &RedisConfig {
        match self.storage.driver {
            StorageDriver::Redis(ref conf) => conf,
        }
    }

    pub fn expected_local_tsv_file(&self) -> &PathBuf {
        for source in self.event_sources.iter() {
            if let EventSourceConfig::TsvPath(config) = source {
                return &config.file_path;
            }
        }
        panic!("expected local-tsv source")
    }

    pub fn expected_cache_path(&self) -> PathBuf {
        let mut destination_path = std::env::current_dir().expect("unable to get current dir");
        destination_path.push(&self.storage.cache_path);
        destination_path
    }

    pub fn expected_stacks_node_event_source(&self) -> &String {
        for source in self.event_sources.iter() {
            if let EventSourceConfig::StacksNode(config) = source {
                return &config.host;
            }
        }
        panic!("expected remote-tsv source")
    }

    pub fn expected_remote_tsv_url(&self) -> &String {
        for source in self.event_sources.iter() {
            if let EventSourceConfig::TsvUrl(config) = source {
                return &config.file_url;
            }
        }
        panic!("expected remote-tsv source")
    }

    pub fn rely_on_remote_tsv(&self) -> bool {
        for source in self.event_sources.iter() {
            if let EventSourceConfig::TsvUrl(_config) = source {
                return true;
            }
        }
        false
    }

    pub fn should_download_remote_tsv(&self) -> bool {
        let mut rely_on_remote_tsv = false;
        let mut remote_tsv_present_locally = false;
        for source in self.event_sources.iter() {
            if let EventSourceConfig::TsvUrl(_config) = source {
                rely_on_remote_tsv = true;
            }
            if let EventSourceConfig::TsvPath(_config) = source {
                remote_tsv_present_locally = true;
            }
        }
        rely_on_remote_tsv == true && remote_tsv_present_locally == false
    }

    pub fn devnet_default() -> Config {
        Config {
            storage: StorageConfig {
                driver: StorageDriver::Redis(RedisConfig {
                    uri: "redis://localhost:6379/".into(),
                }),
                cache_path: "cache".into(),
            },
            event_sources: vec![EventSourceConfig::StacksNode(StacksNodeConfig {
                host: "http://0.0.0.0:20443".into(),
            })],
            chainhooks: ChainhooksConfig {
                max_stacks_registrations: 50,
                max_bitcoin_registrations: 50,
            },
            network: IndexerConfig {
                stacks_node_rpc_url: "http://0.0.0.0:20443".into(),
                bitcoin_node_rpc_url: "http://0.0.0.0:18443".into(),
                bitcoin_node_rpc_username: "devnet".into(),
                bitcoin_node_rpc_password: "devnet".into(),
                stacks_network: StacksNetwork::Devnet,
                bitcoin_network: BitcoinNetwork::Regtest,
            },
        }
    }

    pub fn testnet_default() -> Config {
        Config {
            storage: StorageConfig {
                driver: StorageDriver::Redis(RedisConfig {
                    uri: "redis://localhost:6379/".into(),
                }),
                cache_path: "cache".into(),
            },
            event_sources: vec![EventSourceConfig::TsvUrl(TsvUrlConfig {
                file_url: DEFAULT_TESTNET_TSV_ARCHIVE.into(),
            })],
            chainhooks: ChainhooksConfig {
                max_stacks_registrations: 10,
                max_bitcoin_registrations: 10,
            },
            network: IndexerConfig {
                stacks_node_rpc_url: "http://0.0.0.0:20443".into(),
                bitcoin_node_rpc_url: "http://0.0.0.0:18332".into(),
                bitcoin_node_rpc_username: "devnet".into(),
                bitcoin_node_rpc_password: "devnet".into(),
                stacks_network: StacksNetwork::Testnet,
                bitcoin_network: BitcoinNetwork::Testnet,
            },
        }
    }

    pub fn mainnet_default() -> Config {
        Config {
            storage: StorageConfig {
                driver: StorageDriver::Redis(RedisConfig {
                    uri: "redis://localhost:6379/".into(),
                }),
                cache_path: "cache".into(),
            },
            event_sources: vec![EventSourceConfig::TsvUrl(TsvUrlConfig {
                file_url: DEFAULT_MAINNET_TSV_ARCHIVE.into(),
            })],
            chainhooks: ChainhooksConfig {
                max_stacks_registrations: 10,
                max_bitcoin_registrations: 10,
            },
            network: IndexerConfig {
                stacks_node_rpc_url: "http://0.0.0.0:20443".into(),
                bitcoin_node_rpc_url: "http://0.0.0.0:8332".into(),
                bitcoin_node_rpc_username: "devnet".into(),
                bitcoin_node_rpc_password: "devnet".into(),
                stacks_network: StacksNetwork::Mainnet,
                bitcoin_network: BitcoinNetwork::Mainnet,
            },
        }
    }
}
