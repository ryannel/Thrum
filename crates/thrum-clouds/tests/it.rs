//! The package's integration tests, one module a file, built into one test binary (GATE-TIME
//! mechanism 10). A new test file is declared here; a file runs as a target of its own only while
//! something names its binary or it has to run apart, as `[[test]]` in Cargo.toml says.

mod scene_fixture;
mod shadow_scene;
mod support;

mod portable_limits_gpu;
mod shadow_only_gpu;
mod sun_path_gpu;
mod transport_gpu;
