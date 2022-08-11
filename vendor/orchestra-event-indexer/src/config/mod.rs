pub use orchestra_event_observer::indexer::IndexerConfig;

#[derive(Clone, Debug)]
pub struct Config {
    pub seed_tsv_path: String,
    pub redis_url: String,
    pub stacks_node_pool: Vec<String>,
    pub bitcoin_node_pool: Vec<String>,
    pub indexer_config: IndexerConfig,
}
