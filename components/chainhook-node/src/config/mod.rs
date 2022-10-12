pub mod file;

pub use chainhook_event_observer::indexer::IndexerConfig;
pub use file::ConfigFile;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    pub storage: StorageConfig,
    pub event_sources: Vec<EventSourceConfig>,
    pub chainhooks: ChainhooksConfig,
    pub indexer: IndexerConfig,
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
    pub fn add_local_tsv_source(&mut self, file_path: &PathBuf) {
        self.event_sources
            .push(EventSourceConfig::TsvPath(TsvPathConfig {
                file_path: file_path.clone(),
            }));
    }

    pub fn expected_redis_config(&self) -> &RedisConfig {
        match self.storage.driver {
            StorageDriver::Redis(ref conf) => conf,
            _ => panic!("expected redis configuration"),
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

    pub fn default() -> Config {
        Config {
            storage: StorageConfig {
                driver: StorageDriver::Redis(RedisConfig {
                    uri: "redis://127.0.0.1/".into() 
                }),
                cache_path: "./.cache".into(),
            },
            event_sources: vec![
                EventSourceConfig::TsvUrl(TsvUrlConfig {
                    file_url: "https://storage.googleapis.com/blockstack-publish/archiver-main/api/stacks-node-events-latest.tar.gz".into() 
                })
            ],
            chainhooks: ChainhooksConfig {
                max_stacks_registrations: 10,
                max_bitcoin_registrations: 10,
            },
            indexer: IndexerConfig {
                stacks_node_rpc_url: "http://0.0.0.0:20443".into(),
                bitcoin_node_rpc_url: "http://0.0.0.0:18443".into(),
                bitcoin_node_rpc_username: "devnet".into(),
                bitcoin_node_rpc_password: "devnet".into(),
            },
        }
    }
}
