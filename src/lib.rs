#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

#[cfg(feature = "wasm")]
extern crate wasm_bindgen;

#[macro_use] extern crate serde_json;
#[macro_use] extern crate lazy_static;

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

pub mod clarity;
pub mod repl;

use repl::Session;

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn run_snippet(snippet: &str) -> String {
    let mut session = Session::new();
    let result = session.interpret(snippet.to_string());
    match result {
        Ok(result) => format!("{}", result),
        Err((message, _)) => format!("{}", message)
    }
}
