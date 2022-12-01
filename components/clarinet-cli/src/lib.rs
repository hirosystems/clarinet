extern crate serde;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate serde_json;

#[macro_use]
extern crate hiro_system_kit;

extern crate lazy_static;

pub extern crate clarity_repl;

pub mod chainhooks;
pub mod deployments;
pub mod generate;
pub mod integrate;

#[cfg(feature = "cli")]
pub mod frontend;
#[cfg(feature = "cli")]
pub mod lsp;
#[cfg(feature = "cli")]
pub mod runner;
