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