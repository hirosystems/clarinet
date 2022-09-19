pub use chainhook_event_observer::indexer::IndexerConfig;

#[derive(Clone, Debug)]
pub struct Config {
    pub seed_tsv_path: String,
    pub redis_url: String,
    pub events_dump_url: String,
    pub topology: Topology,
    pub indexer_config: IndexerConfig,
}

#[derive(Clone, Debug)]
pub enum Topology {
    Bare(Bare),
    ZeroConf(ZeroConf),
}

#[derive(Clone, Debug)]
pub struct Bare {
    pub stacks_node_pool: Vec<String>,
    pub bitcoin_node_pool: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ZeroConf {
    pub chainhook_node_pool: Vec<String>,
    pub poll: u32,
}