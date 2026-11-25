//! Earth atmosphere presentation on the renderer's existing device and graph.
//! The game supplies the sole Sun/time sample; no visual clock lives here.

use crate::shaders::wgsl;
use glam::Vec3;

use crate::error::RenderError;

mod cloud;
mod environment;
pub(crate) mod gpu;
pub(crate) mod graph;
pub(crate) mod ground_band;
mod model;
mod night;
pub(crate) mod overlay;
pub(crate) mod products;
pub(crate) mod reflection;
pub(crate) mod spectrum;
pub(crate) mod stage;

pub(crate) use cloud::CloudPrepared;
pub use cloud::{GpuSunMapping, SunOpticalDepth};
pub use environment::{EnvironmentEpoch, EnvironmentFrame, SurfaceSun, TrueSun};
pub use gpu::medium::ViewVolumeMediumTarget;
pub(crate) use gpu::{AtmospherePrepared, SecondaryAerial};
pub(crate) use graph::overlay_reads;
pub use ground_band::{GroundBand, GroundBandMapping};
pub use products::EnvironmentProducts;

/// Resident storage of the opted-in view volume, for renderer diagnostics.
#[derive(Debug)]
pub struct ViewVolumeStorage {
    /// Cells across, down and along one view ray.
    pub grid: [u32; 3],
    /// Logical bytes of each prepared buffer and texture.
    pub resources: Vec<(&'static str, u64)>,
}

/// Submitted frames for the view volume's temporal blend to settle after a cut.
/// The reset frame already contains a complete transport column.
pub const VIEW_VOLUME_SETTLE_FRAMES: u32 = 8;

/// Cloud sampling source shared by the synthetic and cooked draw shader assemblies.
pub const CLOUD_SHADER_SOURCE: &str = concat!(
    "@group(1) @binding(13) var cloud_optical_depth: texture_2d<f32>;
",
    "@group(1) @binding(14) var<uniform> cloud_field: CloudField;
",
    "fn cloud_sun_direction() -> vec3<f32> { return aerial_frame.sun_lux.xyz; }
",
    wgsl!("atmosphere/cloud.wgsl"),
    wgsl!("atmosphere/cloud_factor.wgsl")
);

/// The Earth profile's aerosol at ground level.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aerosol {
    extinction_550_per_m: f32,
    angstrom_exponent: f32,
}

impl Default for Aerosol {
    fn default() -> Self {
        Self {
            extinction_550_per_m: spectrum::EARTH.mie_extinction,
            angstrom_exponent: 0.0,
        }
    }
}

impl Aerosol {
    /// Construct aerosol from extinction at 550 nm in inverse metres and an
    /// Ångström exponent in 0–2.
    ///
    /// # Errors
    /// Returns invalid lighting for negative or nonfinite extinction or an
    /// exponent outside the supported range.
    pub fn new(extinction_550_per_m: f32, angstrom_exponent: f32) -> Result<Self, RenderError> {
        if !extinction_550_per_m.is_finite()
            || extinction_550_per_m < 0.0
            || !(0.0..=2.0).contains(&angstrom_exponent)
        {
            return Err(RenderError::InvalidLighting);
        }
        Ok(Self {
            extinction_550_per_m,
            angstrom_exponent,
        })
    }

    fn extinction_at(self, wavelength_nm: f32) -> f32 {
        self.extinction_550_per_m * (wavelength_nm / 550.0).powf(-self.angstrom_exponent)
    }

    fn band_extinction(self) -> [f32; 8] {
        spectrum::WAVELENGTHS_NM.map(|wavelength| self.extinction_at(wavelength))
    }

    /// Aerosol-dependent charts are ready in the first submitted frame
    /// carrying a changed aerosol.
    pub const CHART_LAG_FRAMES: u32 = 1;
}

/// Checked presentation sample for the Earth atmosphere profile.
/// Directions use world +Y up; illuminance is perpendicular, above atmosphere.
#[derive(Clone, Copy, Debug)]
pub struct AtmosphereFrame {
    toward_sun: [f32; 3],
    solar_illuminance_lux: f32,
    solar_colour: [f32; 3],
    camera_altitude_m: f32,
    profile_epoch: u64,
    nightglow: bool,
    simulation_time_seconds: f64,
    aerosol: Option<Aerosol>,
    view_volume: bool,
    medium_epoch: Option<u64>,
    volume_depth_factor: u8,
    // One prepared CPU result serves the packed light and the lent identity.
    surface_irradiance_lux: [f32; 3],
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
        let y = Vec3::from_array(solar_colour).dot(crate::light::LUMA);
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
        let toward_sun = (direction / scale).normalize().to_array();
        let transmission = model::sun_transmittance(camera_altitude_m, toward_sun[1]);
        Ok(Self {
            toward_sun,
            solar_illuminance_lux,
            solar_colour,
            camera_altitude_m,
            profile_epoch,
            nightglow: false,
            simulation_time_seconds: 0.0,
            aerosol: None,
            view_volume: false,
            medium_epoch: None,
            volume_depth_factor: 1,
            surface_irradiance_lux: std::array::from_fn(|i| {
                solar_colour[i] * solar_illuminance_lux * transmission[i]
            }),
        })
    }

    /// Select variable aerosol for the scene's prepared atmosphere. Supply
    /// aerosol with subsequent frame samples to update its charts in place.
    #[must_use]
    pub fn with_aerosol(mut self, aerosol: Aerosol) -> Self {
        let transmission = model::sun_transmittance_with_aerosol(
            self.camera_altitude_m,
            self.toward_sun[1],
            aerosol,
        );
        self.surface_irradiance_lux = std::array::from_fn(|i| {
            self.solar_colour[i] * self.solar_illuminance_lux * transmission[i]
        });
        self.aerosol = Some(aerosol);
        self
    }

    pub(crate) fn variable_aerosol(self) -> bool {
        self.aerosol.is_some()
    }

    /// Declare the view volume at scene preparation. Subsequent frame samples
    /// keep the declaration while they change Sun, aerosol or camera.
    #[must_use]
    pub fn with_view_volume(mut self) -> Self {
        self.view_volume = true;
        self.aerosol = Some(self.aerosol.unwrap_or_default());
        self
    }

    /// Declare a provider of the view volume's near medium coefficients.
    /// Change the epoch when a provider reset invalidates the previous samples.
    #[must_use]
    pub fn with_view_volume_medium(mut self, source_epoch: u64) -> Self {
        self = self.with_view_volume();
        self.medium_epoch = Some(source_epoch);
        self
    }

    /// Double the view volume's depth resolution for convergence measurements.
    #[cfg(feature = "creation-counters")]
    #[must_use]
    pub fn with_double_view_volume_slices(mut self) -> Self {
        self = self.with_view_volume();
        self.volume_depth_factor = 2;
        self
    }

    pub(crate) fn view_volume(self) -> bool {
        self.view_volume
    }

    /// Include the clear Earth moonless background and its scattered light.
    /// The profile is supported up to 15 km observer altitude.
    #[must_use]
    pub fn with_nightglow(mut self) -> Self {
        self.nightglow = true;
        self
    }

    /// Set the simulation's sampled presentation time in seconds. Static callers
    /// retain time zero. This changes chart contents without replacing resources.
    ///
    /// # Errors
    /// Returns invalid lighting when the time is nonfinite.
    pub fn with_simulation_time(mut self, seconds: f64) -> Result<Self, RenderError> {
        if !seconds.is_finite() {
            return Err(RenderError::InvalidLighting);
        }
        self.simulation_time_seconds = seconds;
        Ok(self)
    }

    /// Whether the night profile can contribute for this view.
    #[must_use]
    pub fn nightglow_active(self) -> bool {
        self.nightglow
            && self.camera_altitude_m <= crate::shader_constants::NIGHT_TERMINAL_MAX_ALTITUDE
    }

    /// Unit world-space receiver-to-Sun direction.
    #[must_use]
    pub fn toward_sun(self) -> [f32; 3] {
        self.toward_sun
    }

    /// The checked simulation presentation time supplied by the source.
    #[must_use]
    pub fn simulation_time_seconds(self) -> f64 {
        self.simulation_time_seconds
    }

    pub(crate) fn directional(self) -> ([f32; 3], [f32; 3]) {
        (self.toward_sun, self.surface_irradiance_lux)
    }
}

#[cfg(test)]
mod tests;
