extern crate serde;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate serde_json;

#[macro_use]
extern crate lazy_static;

pub extern crate clarity_repl;

#[macro_use]
pub mod macros;

pub mod chainhooks;
pub mod deployments;
pub mod generate;
pub mod integrate;
pub mod types;
pub mod utils;

#[cfg(feature = "cli")]
pub mod frontend;
#[cfg(feature = "cli")]
pub mod lsp;
#[cfg(feature = "cli")]
pub mod runner;
