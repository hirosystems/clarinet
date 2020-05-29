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
pub mod frontend;

use repl::Session;
use frontend::CommandLine;

fn main() {
    let mut cli_frontend = CommandLine::new();
    cli_frontend.start();
}
