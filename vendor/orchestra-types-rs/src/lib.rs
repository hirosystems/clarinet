extern crate serde;

#[macro_use]
extern crate serde_derive;

mod events;
mod rosetta;

pub use events::*;
pub use rosetta::*;

pub enum Chain {
    Bitcoin,
    Stacks,
}
