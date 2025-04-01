#[cfg(not(target_arch = "wasm32"))]
pub mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::*;

#[cfg(all(target_arch = "wasm32", feature = "web"))]
mod browser;
#[cfg(all(target_arch = "wasm32", feature = "web"))]
pub use browser::*;

#[cfg(all(target_arch = "wasm32", not(feature = "web")))]
mod node;
#[cfg(all(target_arch = "wasm32", not(feature = "web")))]
pub use node::*;
