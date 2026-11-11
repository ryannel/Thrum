//! One CPU handoff for the atmosphere sample and its resource identity.

use std::sync::atomic::{AtomicU64, Ordering};

use super::AtmosphereFrame;

// Process-local resource identities, independent of every simulation clock.
// A prepared atmosphere allocates once. Updating its chart contents allocates none.
static NEXT_LUT_GENERATION: AtomicU64 = AtomicU64::new(1);

/// Equality token for the source model, air profile and prepared LUT generation.
/// A time scrub preserves this token. Replacing resources, including on the same
/// device, produces a new token. The numeric components have no time semantics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnvironmentEpoch {
    profile: u64,
    lut_generation: u64,
    nightglow: bool,
}

/// Geometric source facts with an empty applied-attenuation set.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrueSun {
    /// Unit world-space direction from receiver to source, +Y up.
    pub toward_sun: [f32; 3],
    /// Raw perpendicular illuminance above the atmosphere, in lux.
    pub illuminance_lux: f32,
    /// Scene-linear Rec.709 photometric colour with luminance one.
    pub colour: [f32; 3],
    /// Geometric angular radius, in radians.
    pub angular_radius_radians: f32,
}

/// The existing directional surface light at the camera's physical altitude.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfaceSun {
    /// Unit world-space receiver-to-light direction.
    pub toward_sun: [f32; 3],
    /// RGB illuminance after atmospheric transmission, in lux.
    pub irradiance_lux: [f32; 3],
}

/// The one environment sample lent to contributions for a frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnvironmentFrame {
    /// Source and resource identity, independent of chart recomputation.
    pub epoch: EnvironmentEpoch,
    /// Presentation sample time from the simulation, in seconds.
    pub simulation_time_seconds: f64,
    /// Unattenuated geometric Sun for media and independent transport consumers.
    pub true_sun: TrueSun,
    /// Camera-height surface light, kept separate from the raw source.
    pub surface_sun: SurfaceSun,
}

pub(super) struct EnvironmentState {
    lut_generation: u64,
    frame: AtmosphereFrame,
}

impl EnvironmentState {
    pub(super) fn new(frame: AtmosphereFrame) -> Self {
        Self {
            lut_generation: NEXT_LUT_GENERATION.fetch_add(1, Ordering::Relaxed),
            frame,
        }
    }

    pub(super) fn update(&mut self, frame: AtmosphereFrame) {
        self.frame = frame;
    }

    pub(super) fn sample(&self) -> EnvironmentFrame {
        let frame = self.frame;
        let (toward_sun, irradiance_lux) = frame.directional();
        EnvironmentFrame {
            epoch: EnvironmentEpoch {
                profile: frame.profile_epoch,
                lut_generation: self.lut_generation,
                nightglow: frame.nightglow,
            },
            simulation_time_seconds: frame.simulation_time_seconds,
            true_sun: TrueSun {
                toward_sun: frame.toward_sun,
                illuminance_lux: frame.solar_illuminance_lux,
                colour: frame.solar_colour,
                angular_radius_radians: 0.004_654_21,
            },
            surface_sun: SurfaceSun {
                toward_sun,
                irradiance_lux,
            },
        }
    }
}

#[cfg(test)]
mod tests;
