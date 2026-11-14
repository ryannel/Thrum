//! Bounded, nonblocking whole-message links. No sockets are compiled by default.
//! Times in the portable driver are caller-supplied monotonic milliseconds.

mod link;
pub mod memory;
pub mod proof;
pub use link::*;

#[cfg(all(feature = "browser", target_arch = "wasm32"))]
pub mod browser;
#[cfg(all(feature = "native", not(target_arch = "wasm32")))]
pub mod native;
