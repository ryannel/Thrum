//! Shared water, lit by the scene's lent environment. The host owns all clocks and submission.
mod column;
pub mod foam;
#[cfg(test)]
mod fresnel_tests;
pub mod interaction;
pub mod map_gpu;
mod prepare;
pub mod query;
mod records;
#[cfg(test)]
mod sampling_tests;
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
    /// Optional body query capacity and age limits.
    pub query: Option<query::QueryLimits>,
    /// Optional two-level interaction waves.
    pub interaction: Option<interaction::Limits>,
}

/// Invalid water configuration or unavailable preparation dependency.
#[derive(Debug)]
pub enum WaterError {
    /// A previous map writer panicked while holding its lock.
    UnavailableMap,
    /// The selected body index does not exist in this map.
    UnknownBody(usize),
    /// The open-sea constructor requires a sea.
    UnsupportedBody,
    /// A physical wave input is outside the supported domain.
    InvalidSpectrum(String),
    /// The water type contains a nonfinite or out-of-range coefficient.
    InvalidOptics,
    /// The map supplied a nonfinite level.
    InvalidLevel,
    /// The map cannot be represented by this GPU profile.
    UnsupportedMap(&'static str),
    /// Interaction capacities exceed the supported implementation limits.
    InvalidInteractionLimits,
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
            Self::UnsupportedMap(reason) => f.write_str(reason),
            Self::InvalidInteractionLimits => {
                f.write_str("interaction requires 1–16 solves and at most 64 sources of each kind")
            }
        }
    }
}
impl std::error::Error for WaterError {}

/// Preparation facts read by diagnostics without retaining GPU resources.
#[derive(Clone, Debug, Default)]
pub struct WaterReport {
    /// Submitted vertices, including the horizon skirt.
    pub vertices: u32,
    /// Submitted vertices by water body, in declaration order.
    pub bodies: Vec<(String, u32)>,
    /// Actual allocation sizes, keyed by resource name.
    pub resources: Vec<(String, u64)>,
    /// Last submitted interaction counters, when enabled.
    pub interaction: Option<interaction::Report>,
    /// Last submitted foam counters, when enabled.
    pub foam: Option<foam::Report>,
    /// Serial of the pattern last used by a history-advancing submission.
    pub foam_pattern: Option<u64>,
    /// Number of submitted changes to the bound foam pattern.
    pub foam_pattern_invalidations: u64,
}

/// A single contribution containing the ambient passes and the displaced surface.
pub struct Water {
    report: Arc<Mutex<WaterReport>>,
    map: Arc<Mutex<Map>>,
    mapped: bool,
    body: usize,
    spectrum: Spectrum,
    long_variances: [[f32; 4]; 8],
    height_rms: f32,
    layer_heights: [f32; 4],
    profile: Profile,
    query: Option<query::QueryMailbox>,
    interaction: Option<interaction::Mailbox>,
    interaction_tap: Option<interaction::WitnessTap>,
    foam: Option<foam::Whitewater>,
    foam_tap: Option<foam::Tap>,
    foam_pattern: thrum_asset_gpu::ProductLease,
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
        Self::configure(map, body, spectrum, profile, false)
    }

    /// Draw every body of the map on the same wave field and surface pass.
    /// Geometry is kept separate by body, while ownership comes from the map.
    ///
    /// # Errors
    /// Rejects an empty map or invalid spectrum or water type.
    pub fn mapped(
        map: Arc<Mutex<Map>>,
        spectrum: Spectrum,
        profile: Profile,
    ) -> Result<Self, WaterError> {
        Self::configure(map, 0, spectrum, profile, true)
    }

    fn configure(
        map: Arc<Mutex<Map>>,
        body: usize,
        spectrum: Spectrum,
        profile: Profile,
        mapped: bool,
    ) -> Result<Self, WaterError> {
        if profile.interaction.is_some_and(|limits| {
            limits.bodies > 64 || limits.impulses > 64 || !(1..=16).contains(&limits.solves)
        }) {
            return Err(WaterError::InvalidInteractionLimits);
        }
        if profile
            .deep_scatter
            .iter()
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
        {
            return Err(WaterError::InvalidOptics);
        }
        let long_waves =
            thrum_sea_model::LongWaves::new(spectrum.long.clone(), profile.waves.choppiness)
                .map_err(|e| WaterError::InvalidSpectrum(format!("long waves: {e:?}")))?;
        let contraction =
            thrum_sea_model::contraction_bound(&spectrum.long, profile.waves.choppiness)
                .map_err(|e| WaterError::InvalidSpectrum(format!("long waves: {e:?}")))?;
        if contraction >= thrum_sea::LONG_WAVE_CONTRACTION_LIMIT {
            return Err(WaterError::InvalidSpectrum(
                "ambient long-wave contraction must be below 0.9".into(),
            ));
        }
        if spectrum.long.len() > column::COMPONENT_LIMIT
            || !spectrum.tail_slope_variance.is_finite()
            || spectrum.tail_slope_variance < 0.0
        {
            return Err(WaterError::InvalidSpectrum(
                "requires at most 192 long components and finite nonnegative slope variance".into(),
            ));
        }
        let guard = map.lock().map_err(|_| WaterError::UnavailableMap)?;
        let declaration = guard.body(body).ok_or(WaterError::UnknownBody(body))?;
        if !mapped && !matches!(declaration.kind, thrum_water_model::BodyKind::Sea) {
            return Err(WaterError::UnsupportedBody);
        }
        let row = guard.row(body, [0.0; 2]);
        drop(guard);
        if !row.level.is_finite() {
            return Err(WaterError::InvalidLevel);
        }
        let layer_heights = records::layer_heights(&spectrum);
        Ok(Self {
            report: Arc::default(),
            map: Arc::clone(&map),
            mapped,
            body,
            long_variances: records::long_variances(&spectrum),
            height_rms: records::height_rms(&spectrum),
            layer_heights,
            spectrum,
            profile,
            query: profile
                .query
                .filter(|limits| limits.points != 0)
                .map(|limits| query::QueryMailbox::new(limits, map, long_waves, layer_heights)),
            interaction: profile.interaction.map(interaction::Mailbox::new),
            interaction_tap: None,
            foam: None,
            foam_tap: None,
            foam_pattern: thrum_asset_gpu::ProductLease::default(),
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
    /// Tick-page writer and answer mailbox for query-enabled water.
    pub fn query_mailbox(&self) -> Option<query::QueryMailbox> {
        self.query.clone()
    }
    /// Tick-page writer for body reactions and impacts.
    pub fn interaction_mailbox(&self) -> Option<interaction::Mailbox> {
        self.interaction.clone()
    }
    /// Attach a diagnostic receiver before transferring the contribution to the host.
    pub fn tap_interaction(&mut self) -> interaction::WitnessTap {
        self.interaction_tap
            .get_or_insert_with(Default::default)
            .clone()
    }
    /// Enable transported whitewater and the shore film on mapped water.
    /// Wind is given in world x/z metres per second.
    ///
    /// # Errors
    /// Requires mapped water and a finite wind vector.
    pub fn enable_foam(&mut self, wind: [f32; 2]) -> Result<foam::Input, &'static str> {
        if !self.mapped || !wind.into_iter().all(f32::is_finite) {
            return Err("foam requires mapped water and a finite wind vector");
        }
        let input = foam::Input::default();
        self.foam = Some(foam::Whitewater {
            input: input.clone(),
            wind,
            long_bounds: column::long_bounds(&self.spectrum.long),
            jacobian_variance: records::jacobian_variance(
                &self.spectrum,
                self.profile.waves.choppiness,
            ),
        });
        Ok(input)
    }
    /// Attach a diagnostic receiver for submitted foam banks.
    pub fn tap_foam(&mut self) -> foam::Tap {
        self.foam_tap.get_or_insert_with(Default::default).clone()
    }
    /// Lend one cooked ranked 2D texture to the whitewater pass.
    pub fn lend_foam_pattern(&mut self, pattern: thrum_asset_gpu::ProductLease) {
        self.foam_pattern = pattern;
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
            if let Some(indices) = &p.indices {
                resources.push(("water surface triangle indices".into(), indices.size()));
            }
            if let Some(map) = &p.map {
                let size = map.texture.size();
                resources.push((
                    "water map lattice".into(),
                    16 * u64::from(size.width)
                        * u64::from(size.height)
                        * u64::from(size.depth_or_array_layers),
                ));
                resources.push(("water map records".into(), map.info.size()));
            }
            if let Some(query) = &p.query {
                resources.extend(query.resource_bytes());
            }
            if let Some(interaction) = &p.interaction {
                resources.extend(interaction.resource_bytes());
            }
            if let Some(foam) = &p.foam {
                resources.extend(foam.resource_bytes());
            }
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
    fn rebuild(
        &mut self,
        surface: &SceneSurface<'_>,
    ) -> Result<(), thrum_render::runtime::FrameError> {
        let map = self.map.lock().map_err(|_| {
            thrum_render::runtime::FrameError::Scene(WaterError::UnavailableMap.to_string())
        })?;
        let (interaction, foam, foam_pattern_invalidations) = {
            let report = self.report.lock().expect("water report");
            (
                report.interaction,
                report.foam,
                report.foam_pattern_invalidations,
            )
        };
        self.prepared = Some(
            prepare::new(
                surface,
                self.profile,
                &map,
                self.mapped,
                self.query.clone(),
                self.interaction.clone(),
                interaction,
                self.foam
                    .clone()
                    .map(|whitewater| (whitewater, foam.unwrap_or_default())),
            )
            .map_err(thrum_render::runtime::FrameError::Scene)?,
        );
        *self.report.lock().expect("water report") = WaterReport {
            vertices: self.vertices(),
            bodies: self.prepared.as_ref().expect("prepared").bodies.clone(),
            resources: self.resource_bytes(),
            interaction,
            foam,
            foam_pattern: None,
            foam_pattern_invalidations,
        };
        self.previous = None;
        if let Some(tap) = &self.foam_tap {
            *tap.0.lock().expect("foam witness") = None;
        }
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
        let map = self.map.lock().expect("water map lock");
        let row = map.row(self.body, [0.0; 2]);
        if let Some(gpu) = &mut p.map {
            gpu.record(&p.queue, &map, frame.time);
        }
        if let Some(interaction) = &mut p.interaction {
            interaction.record(&p.queue, encoder, frame, &map);
        }
        if let Some(foam) = &mut p.foam {
            self.invalid |= foam.record(
                &p.queue,
                encoder,
                frame,
                &self.spectrum,
                self.profile.waves.choppiness,
                now,
            );
        }
        self.invalid |= p.record_pattern(&self.foam_pattern, frame.generation);
        if let Some(query) = &mut p.query {
            query.record(
                &p.queue,
                frame.time,
                &self.spectrum,
                self.profile.waves.choppiness,
                self.layer_heights,
                &map,
                p.interaction.as_ref(),
            );
        }
        drop(map);
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
            self.height_rms,
            self.layer_heights,
            self.profile.deep_scatter,
        );
        p.queue
            .write_buffer(&p.parameters, 0, bytemuck::bytes_of(&parameters));
    }
    fn effect_runs(&self, index: usize, _frame: &ContributionFrame) -> bool {
        self.prepared.as_ref().is_some_and(|p| {
            if index < p.ambient_passes {
                true
            } else if index == p.ambient_passes
                && let Some(interaction) = &p.interaction
            {
                interaction.pending()
            } else if index == p.query_index()
                && let Some(query) = &p.query
            {
                query.pending()
            } else {
                p.foam.as_ref().is_some_and(foam::Foam::pending)
            }
        })
    }
    fn encode_effect(
        &mut self,
        index: usize,
        pass: &mut wgpu::ComputePass<'_>,
        _frame: &ContributionFrame,
    ) {
        if let Some(p) = &self.prepared {
            if index < p.ambient_passes {
                p.ambient.encode_pass(index, pass);
            } else if index == p.ambient_passes
                && let Some(interaction) = &p.interaction
            {
                interaction.encode(pass);
            } else if index == p.query_index()
                && let Some(query) = &p.query
            {
                query.encode(pass);
            } else if let Some(foam) = &p.foam {
                foam.encode(pass);
            }
        }
    }
    fn encode_effect_readback(&mut self, index: usize, encoder: &mut wgpu::CommandEncoder) {
        if let Some(p) = &self.prepared
            && index == p.query_index()
            && let Some(query) = &p.query
        {
            query.copy(encoder);
        }
    }
    fn draw_after_opaque(
        &self,
        index: usize,
        pass: &mut wgpu::RenderPass<'_>,
        frame: &ContributionFrame,
    ) {
        if let Some(p) = &self.prepared {
            let film = p.film.as_ref().filter(|_| index == 0);
            pass.set_pipeline(film.unwrap_or(&p.pipeline));
            pass.set_bind_group(
                2,
                p.environment
                    .group(frame.exposure_side)
                    .expect("exposure side"),
                &[],
            );
            pass.set_bind_group(3, p.render_group(), &[]);
            if film.is_some() {
                pass.draw(0..3, 0..1);
                return;
            }
            pass.set_vertex_buffer(0, p.mesh.slice(..));
            if let Some(indices) = &p.indices {
                pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
                for range in &p.draws {
                    if !range.is_empty() {
                        pass.draw_indexed(range.clone(), 0, 0..1);
                    }
                }
            } else {
                pass.draw(0..p.vertices, 0..1);
            }
        }
    }
    fn submitted(&mut self, history_advanced: bool) {
        if history_advanced {
            if let Some(p) = &mut self.prepared {
                let (pattern, changed) = p.submit_pattern();
                let mut report = self.report.lock().expect("water report");
                report.foam_pattern = pattern;
                report.foam_pattern_invalidations += u64::from(changed);
            }
            self.previous = Some(self.recorded);
            if let Some(map) = self.prepared.as_mut().and_then(|p| p.map.as_mut()) {
                map.commit();
            }
            if let Some(query) = self.prepared.as_mut().and_then(|p| p.query.as_mut()) {
                query.submit();
            }
            if let Some(interaction) = self.prepared.as_mut().and_then(|p| p.interaction.as_mut()) {
                self.report.lock().expect("water report").interaction = interaction.submit();
                if let Some(tap) = &self.interaction_tap {
                    *tap.0.lock().expect("interaction witness") = interaction.witness();
                }
            }
            if let Some(foam) = self.prepared.as_mut().and_then(|p| p.foam.as_mut()) {
                self.report.lock().expect("water report").foam = foam.submit();
                if let Some(tap) = &self.foam_tap {
                    *tap.0.lock().expect("foam witness") = foam.witness();
                }
            }
        }
    }
}
