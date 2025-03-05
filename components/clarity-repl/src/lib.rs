#[cfg(not(target_arch = "wasm32"))]
#[macro_use]
pub extern crate prettytable;

#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate hiro_system_kit;

#[macro_use]
mod uprint;

pub mod analysis;

pub mod clarity {
    #![allow(ambiguous_glob_reexports)]
    pub use ::clarity::types::*;
    pub use ::clarity::vm::*;
    pub use ::clarity::*;
}
pub mod repl;
pub mod utils;

#[cfg(test)]
pub mod test_fixtures;

#[cfg(not(target_arch = "wasm32"))]
pub mod frontend;

#[cfg(not(target_arch = "wasm32"))]
pub use frontend::Terminal;
