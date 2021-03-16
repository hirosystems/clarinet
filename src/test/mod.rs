mod ast;
mod auth_tokens;
mod checksum;
mod colors;
mod deno_dir;
mod diagnostics;
// mod diff;
mod disk_cache;
// mod errors;
mod file_fetcher;
mod file_watcher;
mod flags;
// mod flags_allow_net;
mod fmt_errors;
mod fs_util;
mod http_cache;
mod http_util;
mod import_map;
mod info;
mod lockfile;
mod media_type;
mod module_graph;
mod module_loader;
mod ops;
mod program_state;
mod source_maps;
mod specifier_handler;
// mod standalone;
mod text_encoding;
// mod tokio_util;
mod tools;
mod tsc;
mod tsc_config;
mod version;


mod deno;

pub fn run_tests() {
    block_on(deno::run_tests());
}

pub fn create_basic_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
      .enable_io()
      .enable_time()
      .max_blocking_threads(32)
      .build()
      .unwrap()
}
  
pub fn block_on<F, R>(future: F) -> R
  where
    F: std::future::Future<Output = R>,
{
    let rt = create_basic_runtime();
    rt.block_on(future)
}
  
// struct ClaritestTransaction {
// }

// struct ClaritestBlock {
// }

// struct ClaritestChain {
// }

// struct ClaritestAccount {
// }

// impl ClaritestChain {

//     fn new() -> ClaritestChain {
//         ClaritestChain {
//         }
//     }

//     fn test() {
//         let config = ClarinetConfig::new();
//         let chain = ClaritestChain::new(config);
//         chain.start();

//     }
// }

// #[claritest()]
// fn test_box_btc(chain: ClaritestChain, accounts: Hashmap<String, ClaritestAccount>) {
//     let block = chain.mine_block(vec![
//         tx!("(contract-call? 'ST000000000000000000002AMW42H.bbtc create-box size fee)"),
//     ]);

//     let res = chain.read("(contract-call? 'ST000000000000000000002AMW42H.bbtc create-box size fee)");
// }