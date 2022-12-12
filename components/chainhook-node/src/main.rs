#[macro_use]
extern crate serde_json;

#[macro_use]
extern crate hiro_system_kit;

#[macro_use]
extern crate serde_derive;

extern crate serde;

pub mod archive;
pub mod block;
mod cli;
pub mod config;

fn main() {
    cli::main();
}
