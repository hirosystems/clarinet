mod macros;
#[cfg(feature = "tokio_helpers")]
mod tokio_helpers;

#[cfg(feature = "tokio_helpers")]
pub use tokio_helpers::*;

use std::thread::Builder;

pub fn thread_named(name: &str) -> Builder {
    Builder::new().name(name.to_string())
}
