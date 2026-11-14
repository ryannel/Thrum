//! CPU-only deep-water spectrum, displaced-surface oracle and provider conformance.
//!
//! Coordinates are metres, time is seconds, angles are radians, and horizontal
//! vectors use x/z order. No graphics device, game content or random global state.
mod cascade;
mod conformance;
mod spectrum;
mod waves;

pub use cascade::{Cascade, Differential, GRID_SIZE, LOOP_SECONDS, cascade_frequency, loop_cycles};
pub use conformance::{Conformance, SeaProvider, compare};
pub use spectrum::{Forcing, Spectrum, Swell, Transition};
pub use waves::{Component, LongWaves, Sample, SeaError};

/// Standard gravitational acceleration in metres per second squared.
pub const GRAVITY: f64 = 9.806_65;
