extern crate serde;

#[macro_use]
extern crate serde_derive;

mod events;
mod rosetta;

pub use events::*;
pub use rosetta::*;

pub mod messages {
    include!(concat!(env!("OUT_DIR"), "/orchestra.messages.rs"));
}
