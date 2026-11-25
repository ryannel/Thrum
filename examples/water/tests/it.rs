//! The package's integration tests, one module a file, built into one test binary (GATE-TIME
//! mechanism 10). A new test file is declared here; a file runs as a target of its own only while
//! something names its binary or it has to run apart, as `[[test]]` in Cargo.toml says.

#[cfg(target_os = "macos")]
mod support;

mod foam_lease_gpu;
mod frame_contract_gpu;
mod inland_floating_gpu;
mod level_gpu;
mod scene_gpu;
mod wake_scene_gpu;

mod surf_cut;
