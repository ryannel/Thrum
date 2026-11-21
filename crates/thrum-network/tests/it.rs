//! The package's integration tests, one module a file, built into one test binary (GATE-TIME
//! mechanism 10). A new test file is declared here; a file runs as a target of its own only while
//! something names its binary or it has to run apart, as `[[test]]` in Cargo.toml says.

pub mod support;

mod authority_failure;
mod commands;
mod envelope;
mod future_input;
mod input_batch;
mod negotiation;
mod playout_position;
mod reliable_delivery;
mod results;
mod roster;
mod state_ack;
mod state_delta;
mod untrusted;
