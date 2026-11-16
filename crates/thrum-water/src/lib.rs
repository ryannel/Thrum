//! Shared water, lit by the scene's lent environment. The host owns all clocks and submission.
mod prepare;
mod records;
use std::sync::{Arc, Mutex};
use thrum_render::contribution::{
    ContributionFrame, ContributionPasses, EffectGraph, SceneContribution, SceneSurface,
};
use thrum_sea::display_seconds;
use thrum_sea_model::Spectrum;
use thrum_water_model::Map;

/// The surface pass's timing name.
pub const SURFACE_PASS: &str = "water surface";

/// Physical and mesh settings. Named sea states belong to the game.
#[derive(Clone, Copy, Debug, Default)]
pub struct Profile {
    /// Ambient wave field configuration.
    pub waves: thrum_sea::Profile,
    /// Deep-water backscatter divided by extinction, RGB in zero through one.
    /// The game supplies its water type; the default is a pure absorber.
    pub deep_scatter: [f32; 3],
}

/// Invalid water configuration or unavailable preparation dependency.
#[derive(Debug)]
pub enum WaterError {
    /// A previous map writer panicked while holding its lock.
    UnavailableMap,
    /// The selected body index does not exist in this map.
    UnknownBody(usize),
    /// This first surface profile draws only a sea.
    UnsupportedBody,
    /// A physical wave input is outside the supported domain.
    InvalidSpectrum(String),
    /// The water type contains a nonfinite or out-of-range coefficient.
    InvalidOptics,
    /// The map supplied a nonfinite level.
    InvalidLevel,
}
impl std::fmt::Display for WaterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnavailableMap => f.write_str("water map lock is poisoned"),
            Self::UnknownBody(index) => write!(f, "water body {index} is absent from the map"),
            Self::UnsupportedBody => f.write_str("this surface profile requires a sea body"),
            Self::InvalidSpectrum(reason) => write!(f, "water spectrum: {reason}"),
            Self::InvalidOptics => {
                f.write_str("water deep scatter must be finite in zero through one")
            }
            Self::InvalidLevel => f.write_str("water level must be finite"),
        }
    }
}
impl std::error::Error for WaterError {}

/// Preparation facts read by diagnostics without retaining GPU resources.
#[derive(Clone, Debug, Default)]
pub struct WaterReport {
    /// Submitted vertices, including the horizon skirt.
    pub vertices: u32,
    /// Actual allocation sizes, keyed by resource name.
    pub resources: Vec<(String, u64)>,
}

/// A single contribution containing the ambient passes and the displaced surface.
pub struct Water {
    report: Arc<Mutex<WaterReport>>,
    map: Arc<Mutex<Map>>,
    body: usize,
    spectrum: Spectrum,
    long_variances: [f32; 8],
    profile: Profile,
    prepared: Option<prepare::Prepared>,
    previous: Option<(f64, u64, f64)>,
    recorded: (f64, u64, f64),
    invalid: bool,
}
impl Water {
    /// Declare a sea surface for the selected map body. Stage changes are read at frame boundaries.
    /// # Errors
    /// Rejects nonfinite map samples or unsupported wave configurations during preparation.
    pub fn new(
        map: Arc<Mutex<Map>>,
        body: usize,
        spectrum: Spectrum,
        profile: Profile,
    ) -> Result<Self, WaterError> {
        if profile
            .deep_scatter
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        {
            return Err(WaterError::InvalidOptics);
        }
        thrum_sea_model::LongWaves::new(spectrum.long.clone(), profile.waves.choppiness)
            .map_err(|e| WaterError::InvalidSpectrum(format!("long waves: {e:?}")))?;
        let contraction: f64 = spectrum
            .long
            .iter()
            .map(|c| {
                profile.waves.choppiness * c.amplitude * c.wave_vector[0].hypot(c.wave_vector[1])
            })
            .sum();
        if contraction >= thrum_sea::LONG_WAVE_CONTRACTION_LIMIT {
            return Err(WaterError::InvalidSpectrum(
                "ambient long-wave contraction must be below 0.9".into(),
            ));
        }
        if spectrum.long.len() > 192
            || !spectrum.tail_slope_variance.is_finite()
            || spectrum.tail_slope_variance < 0.0
        {
            return Err(WaterError::InvalidSpectrum(
                "requires at most 192 long components and finite nonnegative slope variance".into(),
            ));
        }
        let guard = map.lock().map_err(|_| WaterError::UnavailableMap)?;
        let declaration = guard.body(body).ok_or(WaterError::UnknownBody(body))?;
        if !matches!(declaration.kind, thrum_water_model::BodyKind::Sea) {
            return Err(WaterError::UnsupportedBody);
        }
        let row = guard.row(body, [0.0; 2]);
        drop(guard);
        if !row.level.is_finite() {
            return Err(WaterError::InvalidLevel);
        }
        Ok(Self {
            report: Arc::default(),
            map,
            body,
            long_variances: records::long_variances(&spectrum),
            spectrum,
            profile,
            prepared: None,
            previous: None,
            recorded: (0.0, 0, 0.0),
            invalid: true,
        })
    }
    /// Read-only preparation report handle; no per-frame polling is required by the water.
    pub fn report(&self) -> Arc<Mutex<WaterReport>> {
        Arc::clone(&self.report)
    }
    /// Actual submitted mesh vertices (including the horizon skirt).
    pub fn vertices(&self) -> u32 {
        self.prepared.as_ref().map_or(0, |p| p.vertices)
    }
    /// Allocated wave and surface resources, for paired cost records.
    pub fn resource_bytes(&self) -> Vec<(String, u64)> {
        self.prepared.as_ref().map_or_else(Vec::new, |p| {
            let mut resources = p.ambient.resource_bytes();
            resources.push((
                "water parameters".into(),
                size_of::<records::Parameters>() as u64,
            ));
            resources.push(("water mesh".into(), p.mesh.size()));
            resources
        })
    }
}
impl SceneContribution for Water {
    fn passes(&self) -> ContributionPasses {
        ContributionPasses {
            upload: true,
            compute: false,
        }
    }
    fn rebuild(&mut self, surface: &SceneSurface<'_>) -> Result<(), String> {
        self.prepared = Some(prepare::new(surface, self.profile)?);
        *self.report.lock().expect("water report") = WaterReport {
            vertices: self.vertices(),
            resources: self.resource_bytes(),
        };
        self.previous = None;
        Ok(())
    }
    fn effect_graph(&self) -> EffectGraph {
        self.prepared
            .as_ref()
            .map_or_else(EffectGraph::default, prepare::Prepared::graph)
    }
    fn encode_upload(&mut self, encoder: &mut wgpu::CommandEncoder, frame: &ContributionFrame) {
        let Some(p) = self.prepared.as_mut() else {
            return;
        };
        let now = display_seconds(frame.time);
        self.invalid = frame.history_reset
            || self
                .previous
                .is_none_or(|(_, tick, _)| frame.time.tick > tick.saturating_add(1));
        let previous = self.previous.map_or(now, |(time, _, _)| time);
        let row = self.map.lock().expect("water map lock").row(
            self.body,
            [f64::from(frame.camera[0]), f64::from(frame.camera[2])],
        );
        self.recorded = (now, frame.time.tick, row.level);
        let previous_level = self.previous.map_or(row.level, |(_, _, level)| level);
        p.ambient
            .upload(
                &p.queue,
                encoder,
                &self.spectrum,
                now,
                [f64::from(frame.camera[0]), f64::from(frame.camera[2])],
                &[],
            )
            .expect("validated water spectrum and camera");
        let parameters = records::Parameters::new(
            row,
            &self.spectrum,
            &p.ambient,
            now - previous,
            self.invalid,
            previous_level,
            self.long_variances,
            self.profile.deep_scatter,
        );
        p.queue
            .write_buffer(&p.parameters, 0, bytemuck::bytes_of(&parameters));
    }
    fn computes(&self, _frame: &ContributionFrame) -> bool {
        false
    }
    fn encode_compute(&mut self, _pass: &mut wgpu::ComputePass<'_>, _frame: &ContributionFrame) {}
    fn encode_effect(
        &mut self,
        index: usize,
        pass: &mut wgpu::ComputePass<'_>,
        _frame: &ContributionFrame,
    ) {
        if let Some(p) = &self.prepared {
            p.ambient.encode_pass(index, pass);
        }
    }
    fn encode_after_compute(&mut self, _encoder: &mut wgpu::CommandEncoder) {}
    fn draw(&self, _pass: &mut wgpu::RenderPass<'_>) {}
    fn draw_after_opaque(
        &self,
        _index: usize,
        pass: &mut wgpu::RenderPass<'_>,
        frame: &ContributionFrame,
    ) {
        if let Some(p) = &self.prepared {
            pass.set_pipeline(&p.pipeline);
            pass.set_bind_group(
                2,
                p.environment
                    .group(frame.exposure_side)
                    .expect("exposure side"),
                &[],
            );
            pass.set_bind_group(3, &p.group, &[]);
            pass.set_vertex_buffer(0, p.mesh.slice(..));
            pass.draw(0..p.vertices, 0..1);
        }
    }
    fn submitted(&mut self, history_advanced: bool) {
        if history_advanced {
            self.previous = Some(self.recorded);
        }
    }
}
