extern crate serde;

#[macro_use]
extern crate serde_derive;

mod frontend;
mod generators;
mod publish;
mod types;
mod utils;

use frontend::cli;

pub fn main() {
    cli::main();
}
