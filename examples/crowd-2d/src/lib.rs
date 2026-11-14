//! Recorded-trace reference crowd, through the registered simulation runtime.
mod rules;
mod trace;
pub use rules::{Body, Candidate, Crowd, Resources, Simulation};
pub use trace::{TraceResult, frames, run_trace};
