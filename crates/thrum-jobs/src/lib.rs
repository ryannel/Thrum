//! Native T2 job ownership for cooperating source and build work.
//!
//! The first Unix profile holds one host owner and conservatively charges a
//! complete F4 materialization phase. It is an operational planning account,
//! not a physical filesystem quota or a ready-build authority.

#[cfg(unix)]
mod unix;

#[cfg(unix)]
pub use unix::{JobError, JobId, JobSpacePlan, MaterializationHost, SpacePressure};
