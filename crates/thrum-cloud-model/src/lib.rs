//! Deterministic, renderer-free cloud weather and optical references.
//!
//! Distances are metres, time is weather seconds, extinction is m⁻¹ and optical
//! depth is dimensionless. The game supplies all type and placement data.
mod field;
mod optics;
mod shape;
mod state;

pub use field::{Cell, Column, Conformance, Density, Field, Profile, Sample, Type};
pub use optics::{Orders, Phase, PhaseTable, Segment};
pub use shape::{Footprint, Shape};
pub use state::{Directed, Parameters, State};

/// A rejected authoritative operation; rejection preserves state except its counter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// Nonfinite or out-of-domain input.
    Invalid,
    /// A tick was repeated or skipped.
    Tick,
    /// Directed shape footprints would overlap, or an id is already present.
    Overlap,
    /// The profile's directed shape limit was reached.
    Capacity,
    /// The requested shape id is absent.
    Missing,
}

pub(crate) fn smooth(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

pub(crate) fn unit(x: f64) -> bool {
    x.is_finite() && (0.0..=1.0).contains(&x)
}
