//! The package's integration tests, one module a file, built into one test binary (GATE-TIME
//! mechanism 10). A new test file is declared here; a file runs as a target of its own only while
//! something names its binary or it has to run apart, as `[[test]]` in Cargo.toml says.

mod support;

mod ambient_gpu;
mod declared_accesses;
mod forward_cache_gpu;
mod frames_gpu;
mod packed_transforms;
mod portable_limits_gpu;
mod recipe_limits;
mod upload_bytes;
mod upload_cost;
