//! The package's integration tests, one module a file, built into one test binary (GATE-TIME
//! mechanism 10). A new test file is declared here; a file runs as a target of its own only while
//! something names its binary or it has to run apart, as `[[test]]` in Cargo.toml says.

mod frame_fixture;
mod interaction_fixture;
mod probe_support;
mod swash_gpu;

mod foam_gpu;
mod gains_gpu;
mod map_gpu;
mod query_gpu;
