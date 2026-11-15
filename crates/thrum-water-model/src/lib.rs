//! CPU water map and f64 surface oracle. Coordinates are metres in x/z order.
//! Preparation owns connectivity; queries share rows with the surface and conformance harness.
mod bed;
mod conformance;
mod cut;
mod map;
mod pyramid;
mod river;
mod section;
mod shore;
mod surf;
mod waves;

pub use bed::Bed;
pub use conformance::{Conformance, Provider, compare};
pub use cut::{Column, CutConfig, Sheet, SheetPoint, cut};
pub use map::{Body, BodyKind, Chart, End, Map, MapSample, Profile, Row, Stage};
pub use pyramid::BedPyramid;
pub use river::{RIVER_FORMAT, RIVER_MAGIC, RiverEnd, RiverFile, RiverReach};
pub use section::{BreakerSection, SectionPoint};
pub use shore::{Lag, ShoreConfig, ShoreTransform, Source, dispersion, shoaling};
pub use surf::{SurfProfile, SurfSheet};
pub use waves::{Field, Limited, Surface, limit, shelter};

/// Invalid authored water, with the declaration's name and actionable cause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaterError {
    /// Body or resource that failed validation.
    pub name: String,
    /// Why preparation refused it.
    pub reason: &'static str,
}

impl WaterError {
    pub(crate) fn new(name: &str, reason: &'static str) -> Self {
        Self {
            name: name.into(),
            reason,
        }
    }
}
