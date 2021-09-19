#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused_must_use)]

use clarity_repl::repl::Session;

use crate::utils;
use std::path::PathBuf;

pub mod deno;

pub fn run_scripts(
    files: Vec<String>,
    include_coverage: bool,
    watch: bool,
    allow_wallets: bool,
    allow_disk_write: bool,
    manifest_path: PathBuf,
    session: Option<Session>,
) {
    match block_on(deno::do_run_scripts(
        files,
        include_coverage,
        watch,
        allow_wallets,
        allow_disk_write,
        manifest_path,
        session,
    )) {
        Err(e) => std::process::exit(1),
        _ => {}
    };
}

pub fn block_on<F, R>(future: F) -> R
where
    F: std::future::Future<Output = R>,
{
    let rt = utils::create_basic_runtime();
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
