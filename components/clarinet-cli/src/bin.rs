extern crate serde;

#[macro_use]
extern crate serde_json;

#[macro_use]
extern crate hiro_system_kit;

mod deployments;
mod devnet;
mod frontend;
mod generate;
mod lsp;

use frontend::cli;

pub fn main() {
    cli::main();
}
