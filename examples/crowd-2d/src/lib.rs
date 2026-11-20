//! Recorded-trace reference crowd, through the registered simulation runtime.
mod rules;
mod trace;
pub use rules::{Body, Candidate, Crowd, PlayerController, PlayerMovement, Resources, Simulation};
pub use trace::{TraceResult, frames, run_trace};

/// Retained trace for the original crowd conformance workload.
pub const RETAINED_CROWD_TRACE: &str = include_str!("../retained/crowd.txt");
