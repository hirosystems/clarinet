#![allow(unused_imports)]

extern crate serde;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate serde_json;

pub mod rpc_client;
pub mod transactions;

pub use rpc_client::{PoxInfo, StacksRpc};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
