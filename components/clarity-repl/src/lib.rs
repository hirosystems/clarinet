#[cfg(feature = "cli")]
#[macro_use]
pub extern crate prettytable;

#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate hiro_system_kit;

#[macro_use]
mod macros;

pub mod analysis;
#[cfg(feature = "cli")]
pub mod codec;
pub mod repl;
pub mod utils;

#[cfg(test)]
pub mod test_fixtures;

pub mod clarity {
    #![allow(ambiguous_glob_reexports)]
    pub use ::clarity::types::*;
    pub use ::clarity::vm::*;
    pub use ::clarity::*;
}

#[cfg(feature = "cli")]
pub mod frontend;

#[cfg(feature = "cli")]
pub use frontend::Terminal;
