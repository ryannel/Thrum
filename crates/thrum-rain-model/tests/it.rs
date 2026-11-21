//! The package's integration tests, one module a file, built into one test binary (GATE-TIME
//! mechanism 10). A new test file is declared here; a file runs as a target of its own only while
//! something names its binary or it has to run apart, as `[[test]]` in Cargo.toml says.

pub mod support;

mod account;
mod conformance;
mod exposure;
mod fill;
mod map;
mod margins;
mod marks;
mod point_test;
mod rain_bands;
mod receivers;
mod rings;
mod state;
mod stores;
mod tree;
mod wet_look;
mod wetness;
