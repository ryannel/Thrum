//! Shared module for Metal ray structure integration tests.

#[cfg(all(target_os = "macos", feature = "creation-counters"))]
pub mod support;
