//! The package's integration tests, one module a file, built into one test binary (GATE-TIME
//! mechanism 10). A new test file is declared here; a file runs as a target of its own only while
//! something names its binary or it has to run apart, as `[[test]]` in Cargo.toml says.

mod support;

mod bed;
mod breaker_section;
mod breaking;
mod conformance;
mod gains;
mod joins;
mod map;
mod shore_basin;
mod shore_island;
mod shore_transform;
mod stage;
mod surf_cut;
mod surf_joins;
mod surf_surface;
mod surface;
mod swash;
