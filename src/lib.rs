extern crate serde;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate rocket;

#[macro_use]
extern crate serde_json;

#[macro_use]
extern crate lazy_static;

pub mod macros;

pub mod integrate;
pub mod poke;
pub mod publish;
#[cfg(feature = "cli")]
pub mod runnner;
pub mod types;
pub mod utils;

pub fn hello_clarinet() -> String {
    format!("Hello Clarinet :)")
}
