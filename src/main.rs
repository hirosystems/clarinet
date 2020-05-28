#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
// todo(ludo): would love to eliminate these directives at some point.

#[macro_use] extern crate lazy_static;

pub mod clarity;
pub mod repl;

use repl::Session;

fn main() {
    let mut session = Session::new();
    session.start();
}
