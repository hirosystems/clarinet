mod macros;

#[cfg(feature = "tokio_helpers")]
mod tokio_helpers;

#[cfg(feature = "tokio_helpers")]
pub use tokio_helpers::*;

#[cfg(feature = "log")]
pub mod log;

#[cfg(feature = "log")]
pub extern crate slog_scope;

#[cfg(feature = "log")]
pub use slog_scope::*;

#[cfg(feature = "log")]
pub extern crate slog;

use std::thread::Builder;

pub fn thread_named(name: &str) -> Builder {
    Builder::new().name(name.to_string())
}
