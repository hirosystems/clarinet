// #![allow(unused_imports)]
// #![allow(unused_variables)]
// #![allow(dead_code)]
// #![allow(non_camel_case_types)]
// #![allow(non_snake_case)]
// #![allow(non_upper_case_globals)]

#[cfg(test)]
pub mod test_fixtures;

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
pub mod codec;
pub mod repl;
pub mod utils;

pub mod clarity {
    #![allow(ambiguous_glob_reexports)]
    pub use ::clarity::stacks_common::*;
    pub use ::clarity::vm::*;
    pub use ::clarity::*;
}

#[cfg(feature = "cli")]
pub mod frontend;

#[cfg(feature = "cli")]
pub use frontend::Terminal;
