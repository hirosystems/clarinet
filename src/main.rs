extern crate serde;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate lazy_static;

mod console;
mod frontend;
mod generators;
mod publish;
pub mod test;
mod types;
mod utils;

use frontend::cli;

pub fn main() {
    cli::main();
}
