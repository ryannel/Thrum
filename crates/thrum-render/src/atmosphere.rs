//! Earth atmosphere presentation on the renderer's existing device and graph.
//! The game supplies the sole Sun/time sample; no visual clock lives here.

use glam::Vec3;

use crate::synthetic::RenderError;

mod gpu;
mod graph;
mod model;
mod night;
mod reflection;
mod spectrum;

pub(crate) use gpu::AtmospherePrepared;

pub(crate) const STAGE_NAMES: [&str; 14] = [
    "atmosphere transmittance",
    "atmosphere multiple scattering",
    "atmosphere sky view",
    "atmosphere diffuse irradiance",
    "atmosphere aerial perspective",
    "atmosphere ground irradiance",
    "atmosphere DFG integration",
    "atmosphere reflection mip 0",
    "atmosphere reflection mip 1",
    "atmosphere reflection mip 2",
    "atmosphere reflection mip 3",
    "atmosphere reflection mip 4",
    "atmosphere reflection mip 5",
    "atmosphere reflection mip 6",
];
// Logical stage, existing five-slot atmosphere timing bucket. Specular
// preparation stays in the sky-view bucket; raw queries keep distinct names.
pub(crate) const TIMING_ORDER: [(usize, usize); 14] = [
    (0, 0),
    (1, 1),
    (5, 1),
    (2, 2),
    (3, 3),
    (4, 4),
    (6, 2),
    (7, 2),
    (8, 2),
    (9, 2),
    (10, 2),
    (11, 2),
    (12, 2),
    (13, 2),
];

/// Checked presentation sample for the fixed Earth atmosphere profile.
/// Directions use world +Y up; illuminance is perpendicular, above atmosphere.
#[derive(Clone, Copy, Debug)]
pub struct AtmosphereFrame {
    toward_sun: [f32; 3],
    solar_illuminance_lux: f32,
    solar_colour: [f32; 3],
    camera_altitude_m: f32,
    profile_epoch: u64,
    nightglow: bool,
}

impl AtmosphereFrame {
    /// Admit and normalize one finite Sun sample. The fixed profile supports
    /// camera altitudes from sea level to 100 km and nonnegative Rec.709 colour
    /// with photometric luminance one (within 0.001).
    ///
    /// # Errors
    /// Returns invalid lighting for unsupported altitude, colour or direction.
    pub fn new(
        toward_sun: [f32; 3],
        solar_illuminance_lux: f32,
        solar_colour: [f32; 3],
        camera_altitude_m: f32,
        profile_epoch: u64,
    ) -> Result<Self, RenderError> {
        let direction = Vec3::from_array(toward_sun);
        let scale = direction.abs().max_element();
        let y = Vec3::from_array(solar_colour).dot(spectrum::LUMA);
        if !direction.is_finite()
            || scale <= 0.0
            || !solar_illuminance_lux.is_finite()
            || !(0.0..=1_000_000.0).contains(&solar_illuminance_lux)
            || !solar_colour.iter().all(|x| x.is_finite() && *x >= 0.0)
            || (y - 1.0).abs() > 0.001
            || !camera_altitude_m.is_finite()
            || !(0.0..=100_000.0).contains(&camera_altitude_m)
        {
            return Err(RenderError::InvalidLighting);
        }
        Ok(Self {
            toward_sun: (direction / scale).normalize().to_array(),
            solar_illuminance_lux,
            solar_colour,
            camera_altitude_m,
            profile_epoch,
            nightglow: false,
        })
    }

    /// Include the clear Earth moonless background and its scattered light.
    /// The profile is supported up to 15 km observer altitude.
    #[must_use]
    pub fn with_nightglow(mut self) -> Self {
        self.nightglow = true;
        self
    }

    /// Whether the night profile can contribute for this view.
    #[must_use]
    pub fn nightglow_active(self) -> bool {
        self.nightglow && self.camera_altitude_m <= 15_000.0
    }

    /// Unit world-space receiver-to-Sun direction.
    #[must_use]
    pub fn toward_sun(self) -> [f32; 3] {
        self.toward_sun
    }

    pub(crate) fn directional(self) -> ([f32; 3], [f32; 3]) {
        let transmission = model::sun_transmittance(self.camera_altitude_m, self.toward_sun[1]);
        (
            self.toward_sun,
            std::array::from_fn(|i| {
                self.solar_colour[i] * self.solar_illuminance_lux * transmission[i]
            }),
        )
    }
}

#[cfg(test)]
mod tests;
