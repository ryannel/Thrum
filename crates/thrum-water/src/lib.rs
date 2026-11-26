//! Shared water, lit by the scene's lent environment. The host owns all clocks and submission.
mod column;
pub mod foam;
#[cfg(test)]
mod fresnel_tests;
pub mod interaction;
pub mod map_gpu;
mod optics;
mod prepare;
pub mod query;
mod rain;
mod records;
#[cfg(test)]
mod sampling_tests;
mod surf;
mod underwater;
pub use optics::{BodyBounds, Optics, OpticsBodies};
use std::sync::{Arc, Mutex};
pub use surf::cache::{SurfTap, SurfWitness};
use thrum_render::contribution::{
    ContributionFrame, ContributionPasses, EffectGraph, QualityError, QualityInvalidation,
    QualityKnob, QualitySettings, SceneContribution, SceneSurface,
};
use thrum_sea::display_seconds;
use thrum_sea_model::Spectrum;
use thrum_water_model::Map;

/// The surface pass's timing name.
pub const SURFACE_PASS: &str = "water surface";
/// Rain depth and canopy evaluator used by the surface and its conformance tests.
pub const RAIN_GAIN_WGSL: &str = include_str!("rain_gain.wgsl");
const VIEW_RECORDS: &str = include_str!("view.wgsl");

/// Physical and mesh settings. Named sea states belong to the game.
#[derive(Clone, Copy, Debug, Default)]
pub struct Profile {
    /// Ambient wave field configuration.
    pub waves: thrum_sea::Profile,
    /// Deep-water backscatter divided by extinction, RGB in zero through one.
    /// The game supplies its water type; the default is a pure absorber.
    pub deep_scatter: [f32; 3],
    /// Optional see-through water and screen-space reflections.
    pub optics: Option<Optics>,
    /// Compose the view through water when some of the lens is submerged. Requires optics.
    pub underwater: bool,
    /// Optional body query capacity and age limits.
    pub query: Option<query::QueryLimits>,
    /// Optional two-level interaction waves.
    pub interaction: Option<interaction::Limits>,
    /// Maximum renewal period of carried detail in a current. None selects four seconds.
    pub carriage_period: Option<std::time::Duration>,
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
    /// Surface detail cannot renew at a zero period.
    InvalidCarriagePeriod,
}
impl std::fmt::Display for WaterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnavailableMap => f.write_str("water map lock is poisoned"),
            Self::UnknownBody(index) => write!(f, "water body {index} is absent from the map"),
            Self::UnsupportedBody => f.write_str("this surface profile requires a sea body"),
            Self::InvalidSpectrum(reason) => write!(f, "water spectrum: {reason}"),
            Self::InvalidOptics => {
                f.write_str("water optical coefficients and reflection controls are outside their supported range")
            }
            Self::InvalidLevel => f.write_str("water level must be finite"),
            Self::UnsupportedMap(reason) => f.write_str(reason),
            Self::InvalidCarriagePeriod => f.write_str("carriage period must be positive"),
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
    /// Prepared vertices, including the horizon skirt.
    pub vertices: u32,
    /// Prepared vertices by water body, in declaration order.
    pub bodies: Vec<(String, u32)>,
    /// Hollow vertices referenced by the surface submission, including shore margins.
    pub hollow_surface_vertices: u32,
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
    /// Live drops in the two submitted rain bands, when prepared.
    pub rain_drops: Option<[usize; 2]>,
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
    mirror_roughness: optics::MirrorRoughness,
    layer_heights: [f32; 4],
    offshore_displacement: [f32; 2],
    surf_cache_bounds: Option<[f32; 3]>,
    profile: Profile,
    query: Option<query::QueryMailbox>,
    interaction: Option<interaction::Mailbox>,
    interaction_tap: Option<interaction::WitnessTap>,
    foam: Option<foam::Whitewater>,
    foam_tap: Option<foam::Tap>,
    surf_tap: Option<SurfTap>,
    foam_pattern: thrum_asset_gpu::ProductLease,
    opaque: OpticsBodies,
    underwater: Option<underwater::Underwater>,
    prepared: Option<prepare::Prepared>,
    rain_quality: Option<QualitySettings>,
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
        if profile
            .carriage_period
            .is_some_and(|period| period.is_zero())
        {
            return Err(WaterError::InvalidCarriagePeriod);
        }
        if profile.interaction.is_some_and(|limits| {
            limits.bodies > 64 || limits.impulses > 64 || !(1..=16).contains(&limits.solves)
        }) {
            return Err(WaterError::InvalidInteractionLimits);
        }
        if (profile.underwater && profile.optics.is_none())
            || profile.optics.is_some_and(|optics| !optics.valid())
            || profile
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
        let offshore_displacement = offshore_bound(&spectrum, profile.waves.choppiness);
        let surf_cache_bounds = if profile.interaction.is_none() {
            surf_cache_bounds(
                &guard,
                &spectrum,
                offshore_displacement,
                profile.waves.choppiness,
            )
        } else {
            None
        };
        let rain_quality = guard.rain().map(|_| rain::quality());
        drop(guard);
        if !row.level.is_finite() {
            return Err(WaterError::InvalidLevel);
        }
        let layer_heights = records::layer_heights(&spectrum);
        let underwater = profile
            .underwater
            .then(|| underwater::Underwater::new(long_waves.clone(), &spectrum, layer_heights));
        let query = profile
            .query
            .filter(|limits| limits.points != 0)
            .map(|limits| {
                let mailbox =
                    query::QueryMailbox::new(limits, Arc::clone(&map), long_waves, layer_heights);
                if rain_quality.is_some() {
                    let variance = |band: &[thrum_sea_model::Component]| {
                        band.iter().map(|wave| 0.5 * wave.amplitude.powi(2)).sum()
                    };
                    mailbox.with_rain(
                        [
                            variance(&spectrum.long),
                            variance(spectrum.cascades[0].components()),
                            variance(spectrum.cascades[1].components()),
                            variance(spectrum.cascades[2].components()),
                        ],
                        profile.carriage_period,
                    )
                } else {
                    mailbox
                }
            });
        Ok(Self {
            report: Arc::default(),
            map,
            mapped,
            body,
            long_variances: records::long_variances(&spectrum),
            height_rms: records::height_rms(&spectrum),
            mirror_roughness: profile
                .optics
                .map_or_else(optics::MirrorRoughness::default, |_| {
                    optics::MirrorRoughness::new(
                        &spectrum,
                        profile.waves.choppiness,
                        layer_heights[3],
                    )
                }),
            layer_heights,
            offshore_displacement,
            surf_cache_bounds,
            underwater,
            spectrum,
            profile,
            query,
            interaction: profile.interaction.map(interaction::Mailbox::new),
            interaction_tap: None,
            foam: None,
            foam_tap: None,
            surf_tap: None,
            foam_pattern: thrum_asset_gpu::ProductLease::default(),
            opaque: OpticsBodies::default(),
            prepared: None,
            rain_quality,
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

    /// Observe the same surf cache the submitted geometry consumes.
    pub fn tap_surf(&mut self) -> SurfTap {
        self.surf_tap.get_or_insert_with(Default::default).clone()
    }
    /// Count work in a diagnostic surf cache dispatch. Attach before preparation
    /// and read the counters after the witnessed frame finishes.
    pub fn tap_surf_work(&mut self) -> SurfTap {
        let tap = self.surf_tap.get_or_insert_with(Default::default);
        tap.1 = true;
        tap.clone()
    }
    /// Lend one cooked ranked 2D texture to the whitewater pass.
    pub fn lend_foam_pattern(&mut self, pattern: thrum_asset_gpu::ProductLease) {
        self.foam_pattern = pattern;
    }
    /// Actual submitted mesh vertices (including the horizon skirt).
    pub fn vertices(&self) -> u32 {
        self.prepared.as_ref().map_or(0, |p| p.vertices)
    }

    /// Get the input for the frame's opaque body boxes.
    #[must_use]
    pub fn optics_bodies(&self) -> OpticsBodies {
        self.opaque.clone()
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
            if let Some(tiles) = &p.hollow_tiles {
                resources.push(("water hollows tile indices".into(), tiles.buffer.size()));
            }
            if let Some(gather) = &p.gather {
                resources.push(("water gather regions".into(), gather.bytes()));
                if let Some(lens) = &gather.lens {
                    resources.push(("water lens columns".into(), lens.size()));
                }
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
                if let Some(bytes) = map.hollows_bytes() {
                    resources.push(("water hollows".into(), bytes));
                }
            }
            if let Some(surf) = &p.surf {
                resources.extend(surf.bytes());
            }
            if let Some(cache) = &p.surf_cache {
                resources.push((surf::cache::PASS.into(), cache.bytes()));
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
            if let Some(rain) = &p.rain {
                resources.push((thrum_sea::rain::PASS.into(), rain.bytes()));
            }
            resources
        })
    }
}
#[expect(
    clippy::cast_possible_truncation,
    reason = "round outward to GPU world metres"
)]
fn offshore_bound(spectrum: &Spectrum, chop: f64) -> [f32; 2] {
    let amplitude = spectrum
        .long
        .iter()
        .chain(
            spectrum
                .cascades
                .iter()
                .flat_map(thrum_sea_model::Cascade::components),
        )
        .map(|component| component.amplitude)
        .sum::<f64>();
    // All gain and filter weights on these open-water vertices are at
    // most one. Leave margin for FFT, half storage and reconstruction.
    [chop, 1.0].map(|scale| ((amplitude * scale).mul_add(1.05, 0.05) as f32).next_up())
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "round outward to GPU world metres"
)]
fn surf_cache_bounds(
    map: &Map,
    spectrum: &Spectrum,
    ambient: [f32; 2],
    chop: f64,
) -> Option<[f32; 3]> {
    let surf = map.surf_profile()?;
    let bodies = (0..).map_while(|index| map.body(index));
    let mut limits = [f64::INFINITY, f64::NEG_INFINITY];
    for body in bodies {
        if matches!(body.kind, thrum_water_model::BodyKind::Chart(_)) {
            // A chart can edit its sampled levels independently of these bounds.
            return None;
        }
        limits[0] = limits[0].min(body.level + body.stage.min);
        limits[1] = limits[1].max(body.level + body.stage.max);
    }
    let height_factor = surf
        .section
        .rows()
        .flat_map(|(_, row)| row)
        .map(|point| point.height.abs())
        .fold(1.0_f64, f64::max);
    let amplitude: f64 = spectrum
        .long
        .iter()
        .map(|wave| {
            let [x, z] = wave.wave_vector;
            wave.amplitude * thrum_water_model::shoaling(x.hypot(z), surf.lag_floor).max(1.0)
        })
        .sum();
    let wave_height = height_factor * amplitude * 1.05 + f64::from(ambient[1]) + 1.0;
    let swash = map.profile().margin_height;
    let highest_bed = map
        .bed()
        .heights()
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    // Overlap can lift the surface from the displaced bed; outside the
    // overlap, the section and contour stay inside the wave and swash bounds.
    let lo = ((limits[0] - wave_height - swash) as f32).next_down();
    let hi = ((limits[1].max(highest_bed) + wave_height + swash) as f32).next_up();
    // The phase vector is normalised for chop and throw, and every blend and
    // depth limiter is bounded by one. Add the short-wave bound and round up.
    let horizontal = ((chop + surf.section.maximum_throw()) * amplitude * 1.05
        + f64::from(ambient[0])
        + 0.05) as f32;
    [lo, hi, horizontal]
        .iter()
        .all(|bound| bound.is_finite())
        .then_some([lo, hi, horizontal.next_up()])
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
                self.spectrum.long.len(),
                self.query.clone(),
                self.interaction.clone(),
                interaction,
                self.foam
                    .clone()
                    .map(|whitewater| (whitewater, foam.unwrap_or_default())),
                self.surf_tap.as_ref().is_some_and(|tap| tap.1),
            )
            .map_err(thrum_render::runtime::FrameError::Scene)?,
        );
        *self.report.lock().expect("water report") = WaterReport {
            vertices: self.vertices(),
            hollow_surface_vertices: 0,
            bodies: self.prepared.as_ref().expect("prepared").bodies.clone(),
            resources: self.resource_bytes(),
            interaction,
            foam,
            foam_pattern: None,
            foam_pattern_invalidations,
            rain_drops: None,
        };
        self.previous = None;
        if let Some(tap) = &self.foam_tap {
            *tap.0.lock().expect("foam witness") = None;
        }
        if let Some(tap) = &self.surf_tap {
            *tap.0.lock().expect("surf witness") = None;
        }
        Ok(())
    }
    fn quality_knobs(&self) -> Vec<QualityKnob> {
        self.rain_quality
            .as_ref()
            .map_or_else(Vec::new, |settings| settings.knobs().to_vec())
    }
    fn set_quality_knob(&mut self, name: &str, value: u32) -> Result<(), QualityError> {
        self.rain_quality
            .as_mut()
            .ok_or_else(|| QualityError::UnknownKnob { name: name.into() })?
            .set(name, value)
    }
    fn commit_quality_knobs(&mut self) -> QualityInvalidation {
        let invalidation = self
            .rain_quality
            .as_mut()
            .map_or(QualityInvalidation::None, QualitySettings::commit);
        if invalidation == QualityInvalidation::EffectHistory
            && let Some(rain) = self
                .prepared
                .as_mut()
                .and_then(|prepared| prepared.rain.as_mut())
        {
            rain.invalidate();
        }
        invalidation
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
        let displayed_map = match &mut p.map {
            Some(gpu) => gpu.record(&p.queue, &map, frame.time),
            None => &map,
        };
        if let Some(rain) = &mut p.rain {
            let cadence = self
                .rain_quality
                .as_ref()
                .map_or(1, |settings| settings.knobs()[0].value);
            rain.record(
                &p.queue,
                map.rain(),
                now,
                cadence,
                self.invalid || frame.time.capture,
            );
        }
        if let Some(interaction) = &mut p.interaction {
            interaction.record(&p.queue, encoder, frame, &map);
        }
        if let Some(gather) = &mut p.gather {
            let wake = p.interaction.as_ref().map(|interaction| {
                // An unresolved shore transform can extend the reach beyond
                // the ambient bound. Admit for any live window in that case.
                let horizontal = if displayed_map.surf_profile().is_some() {
                    f64::INFINITY
                } else {
                    f64::from(self.offshore_displacement[0])
                };
                (interaction, horizontal, f64::from(p.wake_cell_padding))
            });
            gather.record(
                &p.queue,
                frame,
                self.mapped.then_some(displayed_map),
                &self.opaque,
                row.level,
                self.offshore_displacement,
                self.mirror_roughness.with_foam(p.foam.is_some()),
                self.underwater
                    .as_mut()
                    .map(|water| {
                        let short_domain = self.interaction.is_none().then_some(if self.mapped {
                            underwater::ShortWaveDomain::Mapped
                        } else {
                            underwater::ShortWaveDomain::Open(row.gains)
                        });
                        water.lens(frame, displayed_map, now, short_domain, wake)
                    })
                    .as_ref(),
            );
        }
        if let Some(tiles) = &mut p.hollow_tiles {
            tiles.record(
                &p.queue,
                p.map.as_ref().expect("hollows map").hollow_levels(),
            );
            self.report
                .lock()
                .expect("water report")
                .hollow_surface_vertices = tiles.index_count() / 6 * 4;
        }
        if let Some(cache) = &mut p.surf_cache {
            cache.record(
                &p.queue,
                &self.spectrum,
                self.layer_heights,
                frame,
                self.surf_cache_bounds,
                [now, now - previous, self.profile.waves.choppiness],
            );
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
            query.record_frame(frame.time, &map);
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
            records::carriage(now, self.profile.carriage_period),
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
            } else if index == p.surf_cache_index() && p.surf_cache.is_some() {
                true
            } else if index >= p.rain_offset() {
                p.rain.as_ref().is_some_and(|rain| rain.pending)
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
            } else if index == p.surf_cache_index()
                && let Some(cache) = &p.surf_cache
            {
                cache.encode(pass);
            } else if index >= p.rain_offset() {
                if let Some(rain) = &p.rain {
                    rain.bands.encode(index - p.rain_offset(), pass);
                }
            } else if let Some(foam) = &p.foam {
                foam.encode(pass);
            }
        }
    }
    fn tick_has_work(&self, _index: usize) -> bool {
        self.prepared
            .as_ref()
            .and_then(|p| p.query.as_ref())
            .is_some_and(query::Readback::has_work)
    }
    fn encode_tick_upload(
        &mut self,
        _index: usize,
        _encoder: &mut wgpu::CommandEncoder,
        _step: std::time::Duration,
    ) {
        let resources_grew = if let Some(p) = &mut self.prepared
            && let Some(query) = &mut p.query
        {
            let map = self.map.lock().expect("water map lock");
            query.record(
                &p.queue,
                &self.spectrum,
                &self.profile,
                self.layer_heights,
                &map,
                p.map.as_ref(),
                p.interaction.as_ref(),
            )
        } else {
            false
        };
        if resources_grew {
            self.report.lock().expect("water report").resources = self.resource_bytes();
        }
    }
    fn encode_tick(&mut self, _index: usize, pass: &mut wgpu::ComputePass<'_>) {
        if let Some(query) = self.prepared.as_ref().and_then(|p| p.query.as_ref()) {
            query.encode(pass);
        }
    }
    fn encode_tick_readback(&mut self, _index: usize, encoder: &mut wgpu::CommandEncoder) {
        if let Some(query) = self.prepared.as_ref().and_then(|p| p.query.as_ref()) {
            query.copy(encoder);
        }
    }
    fn tick_submitted(&mut self, _passes: &[usize]) {
        if let Some(query) = self.prepared.as_mut().and_then(|p| p.query.as_mut()) {
            query.submit();
        }
    }
    fn draw_after_opaque(
        &self,
        index: usize,
        pass: &mut wgpu::RenderPass<'_>,
        frame: &ContributionFrame,
    ) {
        if let Some(p) = &self.prepared {
            let film = p
                .film
                .as_ref()
                .filter(|_| index == usize::from(p.gather.is_some()));
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
            if let Some(underwater) = &p.underwater
                && p.gather.as_ref().is_some_and(|gather| gather.under)
            {
                pass.set_pipeline(underwater);
                pass.draw(0..3, 0..1);
                pass.set_pipeline(&p.pipeline);
            }
            pass.set_vertex_buffer(0, p.mesh.slice(..));
            let mut river_active = false;
            if let Some(indices) = &p.indices {
                pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
                for draw in &p.draws {
                    if !draw.indices.is_empty()
                        && (self.interaction.is_some()
                            || draw.visible(frame, self.offshore_displacement))
                    {
                        if draw.river != river_active {
                            let pipeline = if draw.river {
                                p.river_pipeline.as_ref().expect("sampled river pipeline")
                            } else {
                                &p.pipeline
                            };
                            pass.set_pipeline(pipeline);
                            river_active = draw.river;
                        }
                        pass.draw_indexed(draw.indices.clone(), 0, 0..1);
                    }
                }
            } else if p.map.is_none() {
                pass.draw(0..p.vertices, 0..1);
            }
            if let Some(tiles) = &p.hollow_tiles
                && tiles.index_count() > 0
            {
                if river_active {
                    pass.set_pipeline(&p.pipeline);
                }
                pass.set_index_buffer(tiles.buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.draw_indexed(0..tiles.index_count(), 0, 0..1);
            }
        }
    }
    fn after_opaque_runs(&self, _: usize, _: &ContributionFrame) -> bool {
        self.prepared
            .as_ref()
            .and_then(|p| p.gather.as_ref())
            .is_some_and(optics::Gather::runs)
    }

    fn encode_after_opaque(
        &mut self,
        _: usize,
        pass: &mut wgpu::ComputePass<'_>,
        frame: &ContributionFrame,
    ) {
        if let Some(p) = &self.prepared
            && let Some(gather) = &p.gather
        {
            gather.encode(
                pass,
                p.environment
                    .group(frame.exposure_side)
                    .expect("exposure side"),
            );
        }
    }
    fn submitted(&mut self, history_advanced: bool) {
        if let Some(rain) = self
            .prepared
            .as_mut()
            .and_then(|prepared| prepared.rain.as_mut())
        {
            rain.submitted();
            self.report.lock().expect("water report").rain_drops = Some(rain.recorded_drops);
        }
        if history_advanced {
            if let Some(p) = &mut self.prepared {
                let (pattern, changed) = p.submit_pattern();
                let mut report = self.report.lock().expect("water report");
                report.foam_pattern = pattern;
                report.foam_pattern_invalidations += u64::from(changed);
                if let Some(tap) = &self.surf_tap {
                    *tap.0.lock().expect("surf witness") =
                        p.surf_cache.as_ref().map(|cache| cache.witness(&p.mesh));
                }
            }
            self.previous = Some(self.recorded);
            if let Some(map) = self.prepared.as_mut().and_then(|p| p.map.as_mut()) {
                map.commit();
            }
            if let Some(query) = self.prepared.as_mut().and_then(|p| p.query.as_mut()) {
                query.submit_frame();
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
