#[derive(Clone, Debug)]
pub struct ConfigFile {
    pub storage: Option<StorageConfigFile>,
    pub event_source: Vec<EventSourceConfigFile>,
    pub chainhooks: Option<ChainhooksConfigFile>,
}

#[derive(Clone, Debug)]
pub struct StorageConfigFile {
    pub driver: Option<String>,
    pub redis_uri: Option<String>,
}

#[derive(Clone, Debug)]
pub struct EventSourceConfigFile {
    pub source_type: Option<String>,
    pub stacks_node_url: Option<String>,
    pub chainhook_node_url: Option<String>,
    pub polling_delay: Option<u32>,
    pub tsv_file_path: Option<String>,
    pub tsv_file_url: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ChainhooksConfigFile {
    pub max_stacks_registrations: Option<u16>,
    pub max_bitcoin_registrations: Option<u16>,
}
