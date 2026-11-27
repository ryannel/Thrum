//! CPU water map and f64 surface oracle. Coordinates are metres in x/z order.
//! Preparation owns connectivity; queries share rows with the surface and conformance harness.
mod bed;
mod conformance;
mod cut;
mod fall;
mod fall_bank;
mod flow;
mod hollows;
mod map;
mod meter;
mod pyramid;
mod rain;
mod river;
mod samples;
mod section;
mod shore;
mod surf;
mod waves;

pub use bed::Bed;
pub use conformance::{Conformance, Provider, compare};
pub use cut::{Column, CutConfig, Sheet, SheetPoint, WholeCut, cut, highest_triangle};
pub use fall::{
    Below, Births, Fall, FallSegment, GRAVITY, Impulse, PathPoint, PatternLoop, Plunge,
    PlungeProfile, SheetPattern, SheetProfile, SheetSample, WATER_DENSITY, loop_ticks,
};
pub use flow::{FlowBands, FlowBandsProfile, flow_frequency};
pub use hollows::{HollowOptics, Hollows, HollowsProfile, NO_HOLLOW};
pub use map::{BRINK, Body, BodyKind, Chart, End, Map, MapSample, Profile, Row, Stage};
pub use meter::meter_light_factor;
pub use pyramid::BedPyramid;
pub use rain::{
    CanopyGrid, Drop, Exposure, OpenSky, RainBands, RainBandsProfile, RainQuery, RateHistory,
    RingClass, RingTable, Support,
};
pub use river::{
    EndKind, FallSamples, RIVER_FORMAT, RIVER_MAGIC, ReachSample, RiverEnd, RiverFile, RiverReach,
    UpperSample,
};
pub use samples::{ChartSamples, FlowSample, Gauge, River, mix_fall};
pub use section::{BreakerSection, SectionPoint};
pub use shore::{Lag, ShoreConfig, ShoreTransform, Source, dispersion, shoaling};
pub use surf::{SurfProfile, SurfSheet, ThrowReach, shore_sector};
pub use waves::{Field, Limited, Surface, limit, shelter};

/// Surface tension over water density, in m³/s², for capillary-gravity waves.
pub const CAPILLARY_COEFFICIENT: f64 = 7.28e-5;

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
