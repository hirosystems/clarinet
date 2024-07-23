mod common;
pub mod utils;

#[macro_use]
extern crate lazy_static;

#[cfg(feature = "wasm")]
pub mod vscode_bridge;

pub use common::backend;
pub use common::state;
pub use lsp_types;
