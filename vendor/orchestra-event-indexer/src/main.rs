#[macro_use]
extern crate serde;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate serde_json;

mod cli;

fn main() {
    cli::main();
}
