#[macro_use]
extern crate lazy_static;

mod common;
pub mod types;
pub mod utils;
#[cfg(feature = "wasm")]
pub mod vscode_bridge;

pub use common::backend;
pub use common::state;
pub use lsp_types;
