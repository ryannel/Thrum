//! The package's integration tests, one module a file, built into one test binary (GATE-TIME
//! mechanism 10). A new test file is declared here; a file runs as a target of its own only while
//! something names its binary or it has to run apart, as `[[test]]` in Cargo.toml says.

mod await_drawn;
mod cooked_view_gpu;
mod drawn_publication;
mod mapped_indices;
mod static_scene;
mod wgsl_records;
