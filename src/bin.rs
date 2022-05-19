extern crate serde;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate rocket;

#[macro_use]
extern crate serde_json;

#[macro_use]
mod macros;

mod deployment;
mod frontend;
mod generate;
mod indexer;
mod integrate;
mod lsp;
mod runnner;
mod types;
mod utils;

use frontend::cli;

pub fn main() {
    cli::main();
}
