extern crate serde;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate lazy_static;

mod frontend;
mod generators;
mod publish;
mod types;
mod utils;
mod console;
mod test;

use frontend::cli;

pub fn main() {
    cli::main();
}
