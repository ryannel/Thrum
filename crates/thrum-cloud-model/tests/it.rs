//! The package's integration tests, one module a file, built into one test binary (GATE-TIME
//! mechanism 10). A new test file is declared here; a file runs as a target of its own only while
//! something names its binary or it has to run apart, as `[[test]]` in Cargo.toml says.

pub mod ground_support;
pub mod support;

mod births;
mod conformance;
mod continuity;
mod density;
mod directed;
mod display_travel;
mod drift;
mod field;
mod ground_absent;
mod ground_day;
mod ground_place;
mod ground_sun_path;
mod haze;
mod medium;
mod optics;
mod placed;
mod placed_sun_path;
mod pooling;
mod puff;
mod state;
mod sun_disc_phase;
mod turret_query;
mod visibility;
