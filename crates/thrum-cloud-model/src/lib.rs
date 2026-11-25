//! Deterministic, renderer-free cloud weather and optical references.
//!
//! Distances are metres, time is weather seconds, extinction is m⁻¹ and optical
//! depth is dimensionless. The game supplies all type and placement data.
mod field;
mod ground;
mod library;
mod medium;
mod noise;
mod optics;
mod shape;
mod state;
mod visibility;

pub use field::{Cell, Column, Conformance, Density, Field, Profile, Sample, Type};
pub use ground::{Affinity, Fog, Ground, GroundProfile, GroundState, Surface, SurfacePoint};
pub use library::{HashedShapes, PlacedLibrary, PlacedSlab, ShapeSource};
pub use medium::{Coefficients, Handover, Light};
pub use noise::{BANK_NOISE_PERIOD, DETAIL_PERIOD, DETAIL_SIZE, SHAPE_PERIOD, SHAPE_SIZE};
pub use optics::{Orders, Phase, PhaseTable, Segment};
pub use shape::{Footprint, Shape};
pub use state::{Directed, Parameters, State};
pub use visibility::{Answer, Fidelity, SunPath, Weather, aerosol_extinction, visual_range};

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

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Invalid => "input is nonfinite or outside the supported domain",
            Self::Tick => "weather step must commit exactly the next tick",
            Self::Overlap => "directed shape identity or footprint overlaps",
            Self::Capacity => "directed shape capacity is exhausted",
            Self::Missing => "directed shape identity is absent",
        })
    }
}

impl std::error::Error for Error {}

pub(crate) fn unit(x: f64) -> bool {
    x.is_finite() && (0.0..=1.0).contains(&x)
}
