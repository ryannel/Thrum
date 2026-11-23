//! A still scene under a physically based sky: the kit behind the starter.
//!
//! [`SkyGame`] is a complete [`Game`]: the world is still by default (a game may select
//! [`SkyGame::with_tick_duration`]), and the renderer draws Earth's
//! atmosphere and Sun with a camera meter for exposure, a lit ground that receives the Sun's
//! shadows, and cooked meshes ([`SkyGame::with_meshes`]) or another engine contribution in the
//! same lit scene. Each frame the game's update maps its [`Controls`] onto a [`Sky`]: the Sun's
//! elevation and azimuth and a look-at [`Camera`]. The meshes' WGSL material hook, named by
//! `THRUM_MATERIAL`, reloads on `L` beside the parameter file. [`run`] opens the window, or renders one PNG
//! offscreen when `THRUM_CAPTURE_PNG` names a file.
//!
//! World units are metres with +Y up and the ground at Y = 0; north is -Z and east is +X.
//!
//! ```no_run
//! use thrum_app::{Config, sky::{self, Camera, Controls, Display, MeshPlacement, SkyGame}};
//!
//! sky::run(Config::new("my-scene", "My scene"), || {
//!     let mut controls = Controls::new();
//!     let sun = controls
//!         .slider("sun.elevation", "Sun elevation", -10.0..=90.0)
//!         .default(20.0)
//!         .display(Display::plain(1, "°"))
//!         .add()?;
//!     let camera = Camera::look_at([0.0, 1.7, 6.0], [0.0, 1.0, 0.0], 40.0);
//!     let rock = MeshPlacement::new("meshes/rock", "textures/white").on_ground();
//!     let game = SkyGame::new(controls, camera, move |controls, sky| {
//!         sky.sun_elevation_degrees = controls.get(sun);
//!     });
//!     Ok(game.with_meshes(vec![rock])?)
//! });
//! ```

#[cfg(target_arch = "wasm32")]
mod browser;
#[cfg(any(test, target_os = "macos"))]
mod capture;
#[cfg(target_os = "macos")]
pub use capture::{HeadlessDeviceError, headless_device};
#[cfg(target_os = "macos")]
mod live_capture;

use std::{
    error::Error,
    sync::{Arc, Mutex},
    time::Duration,
};

#[cfg(not(target_arch = "wasm32"))]
use thrum_asset_gpu::MaterialOutcome;
use thrum_asset_gpu::{CookedAssets, CookedConfig, CookedStats, MaterialHook};
pub use thrum_asset_gpu::{CookedMaterial, MeshPlacement};
pub use thrum_controls::{Controls, Display};
use thrum_core::{
    WorldEpoch,
    sim::{SnapshotId, WorldAdvance, WorldPublication},
};
#[cfg(any(test, target_os = "macos", target_arch = "wasm32"))]
use thrum_render::{
    abi::{InstanceRecord, MaterialRecord},
    synthetic::SyntheticScene,
};
use thrum_render::{
    atmosphere::AtmosphereFrame,
    contribution::SceneContribution,
    extract::{ExtractReport, RenderExtractor, SourceTick},
    runtime::{FrameWorld, SceneFrame, SceneSource},
    synthetic::{DisplayOverlay, FrameLighting, SdrToneMap, SyntheticView},
};
#[cfg(any(target_os = "macos", target_arch = "wasm32"))]
use web_time::Instant;

#[cfg(not(target_arch = "wasm32"))]
use crate::Config;
#[cfg(any(target_os = "macos", target_arch = "wasm32"))]
use crate::{Edits, Game, GameInput, Gpu, Prepared};
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;
#[cfg(any(test, target_os = "macos", target_arch = "wasm32"))]
use thrum_render::{
    capabilities::DeviceCapabilities,
    runtime::{FrameConfig, FrameHost},
};

/// Environment variable naming a PNG file: [`run`] renders one frame there without a window.
pub const CAPTURE_VARIABLE: &str = "THRUM_CAPTURE_PNG";
/// Environment variable naming the cooked asset store that [`SkyGame::with_meshes`] draws
/// from; `./dev scene <folder>` cooks the folder's `assets/` and sets it.
pub const STORE_VARIABLE: &str = "THRUM_ASSET_STORE";
/// Names the cooked meshes' WGSL material hook, which `./dev scene` sets to the folder's
/// `material.wgsl` when there is one.
pub const MATERIAL_VARIABLE: &str = "THRUM_MATERIAL";

/// Failure before the sky kit can attach its cooked placements.
#[derive(Debug)]
pub enum CookedSetupError {
    /// The scene's cooked publication is unavailable.
    Store(String),
    /// The placement table is invalid.
    Placement(thrum_asset_gpu::PlacementError),
}

impl std::fmt::Display for CookedSetupError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Store(message) => formatter.write_str(message),
            Self::Placement(error) => error.fmt(formatter),
        }
    }
}

impl Error for CookedSetupError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Store(_) => None,
            Self::Placement(error) => Some(error),
        }
    }
}

/// Linear base colour of the default ground: dry, grassy earth.
pub const DEFAULT_GROUND: [f32; 3] = [0.16, 0.15, 0.11];
/// Half the width of the ground square, in metres: it reaches within a pixel of the horizon
/// from standing height, where the sky's own planet surface takes over.
#[cfg(any(test, target_os = "macos", target_arch = "wasm32"))]
const GROUND_HALF_WIDTH_M: f32 = 4_000.0;
/// Half the ground slab's thickness; its top face is the ground at Y = 0.
const GROUND_HALF_DEPTH_M: f32 = 0.5;
#[cfg(any(test, target_os = "macos", target_arch = "wasm32"))]
const GROUND_ROUGHNESS: f32 = 0.95;

/// Near and far clip planes of every kit camera, in metres.
const NEAR_M: f32 = 0.1;
const FAR_M: f32 = 30_000.0;
/// The Sun above the atmosphere: perpendicular illuminance and neutral colour.
const SOLAR_LUX: f32 = 127_000.0;
/// Display exposure used when lighting is not the physical atmosphere.
const FALLBACK_EXPOSURE: f32 = 1.0 / 16_000.0;
/// The still world's one committed endpoint.
const STILL: SourceTick = SourceTick {
    epoch: 1,
    tick: 0,
    timeline: 0,
};

/// A perspective camera at `position` looking at `target`, with +Y up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    /// Eye position in metres; its height must be within 0–100 km for the atmosphere.
    pub position: [f32; 3],
    /// Point the camera looks at, in metres; not straight above or below the eye.
    pub target: [f32; 3],
    /// Vertical field of view in degrees, between 1 and 170.
    pub vertical_fov_degrees: f32,
}

impl Camera {
    /// A camera at `position` looking at `target` with a vertical field of view in degrees.
    #[must_use]
    pub const fn look_at(position: [f32; 3], target: [f32; 3], vertical_fov_degrees: f32) -> Self {
        Self {
            position,
            target,
            vertical_fov_degrees,
        }
    }

    fn view(&self, extent: [u32; 2]) -> Result<SyntheticView, String> {
        if !(0.0..=100_000.0).contains(&self.position[1]) {
            return Err(format!(
                "camera height {} m is outside 0–100 km; move Camera::position above sea level",
                self.position[1]
            ));
        }
        if !(1.0..=170.0).contains(&self.vertical_fov_degrees) {
            return Err(format!(
                "camera field of view {}° is outside 1–170°",
                self.vertical_fov_degrees
            ));
        }
        SyntheticView::perspective(
            self.position,
            self.target,
            [0.0, 1.0, 0.0],
            self.vertical_fov_degrees.to_radians(),
            extent[0] as f32 / extent[1].max(1) as f32,
            NEAR_M,
            FAR_M,
        )
        .map(|view| view.with_sdr_tone_map(SdrToneMap::NeutralHighlights))
        .map_err(|_| {
            "camera position and target must be finite, distinct and not vertically aligned"
                .to_owned()
        })
    }
}

/// What the scene shows this frame; the game's update edits it from controls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sky {
    /// Shared camera exposure bias in stops; positive values brighten the entire scene.
    pub exposure_compensation_ev: f32,
    /// Produce depth, unjittered motion and reactive inputs for temporal consumers.
    pub temporal_inputs: bool,
    /// Sun height above the horizon in degrees, -90 to 90; below zero is twilight or night.
    pub sun_elevation_degrees: f32,
    /// Sun bearing in degrees clockwise from north (-Z) through east (+X).
    pub sun_azimuth_degrees: f32,
    /// The look-at camera.
    pub camera: Camera,
    /// Show exposure zones in place of the picture: exposed luminance in one-stop
    /// false-colour bands, grey at a metered mid-grey card, blues below, yellow to pink above.
    pub exposure_zones: bool,
}

impl Sky {
    /// Unit world direction from the scene toward the Sun.
    #[must_use]
    pub fn toward_sun(&self) -> [f32; 3] {
        let (sin_elevation, cos_elevation) = self.sun_elevation_degrees.to_radians().sin_cos();
        let (sin_azimuth, cos_azimuth) = self.sun_azimuth_degrees.to_radians().sin_cos();
        [
            cos_elevation * sin_azimuth,
            sin_elevation,
            -cos_elevation * cos_azimuth,
        ]
    }

    /// Refuse values the renderer cannot draw, naming the field and the fix.
    #[cfg(any(test, target_os = "macos", target_arch = "wasm32"))]
    fn check(&self) -> Result<(), String> {
        if !(-24.0..=24.0).contains(&self.exposure_compensation_ev) {
            return Err("exposure_compensation_ev must be within -24..=24".into());
        }
        if !(-90.0..=90.0).contains(&self.sun_elevation_degrees) {
            return Err(format!(
                "sun_elevation_degrees {} must be within -90..=90",
                self.sun_elevation_degrees
            ));
        }
        if !self.sun_azimuth_degrees.is_finite() {
            return Err("sun_azimuth_degrees must be finite".to_owned());
        }
        self.camera.view([16, 9]).map(drop)
    }

    fn lighting(&self) -> Result<FrameLighting, String> {
        AtmosphereFrame::new(
            self.toward_sun(),
            SOLAR_LUX,
            [1.0; 3],
            self.camera.position[1],
            1,
        )
        .map(FrameLighting::Atmosphere)
        .map_err(|error| format!("sky lighting: {error:?}"))
    }
}

/// Progress of the scene's contribution, reported by the game's status probe.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ContentStatus {
    /// Nothing drawn yet.
    Loading,
    /// The content generation now drawn; a new number means a hot reload swapped in.
    Drawn(u64),
    /// The content cannot draw; the message names the cause.
    Failed(String),
}

/// The kit's world: still by default, optionally advancing a game's fixed clock.
#[derive(Default)]
pub struct StillWorld {
    tick_duration: Option<Duration>,
    now: Duration,
    wall_now: Duration,
    paused: bool,
    presentation_hold: bool,
    tick: u64,
    fixed_update: Option<FixedUpdate>,
}

type FixedUpdate = Box<dyn FnMut(u64, Duration) + Send>;

impl std::fmt::Debug for StillWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StillWorld")
            .field("tick", &self.tick)
            .field("paused", &self.paused)
            .finish_non_exhaustive()
    }
}

impl StillWorld {
    /// Stop the clock, discarding its partial tick and holding the committed endpoint.
    ///
    /// # Panics
    /// Panics if the partial tick exceeds a `u64` nanosecond count.
    pub fn pause(&mut self) {
        self.paused = true;
        self.presentation_hold = true;
        if let Some(step) = self.tick_duration {
            self.now -= Duration::from_nanos(
                u64::try_from(self.now.as_nanos() % step.as_nanos())
                    .expect("a sub-tick fraction fits in u64 nanoseconds"),
            );
        }
    }

    /// Start a new wall-time span, holding until its first full tick commits.
    pub fn resume(&mut self, now: Duration) {
        self.wall_now = now.max(self.wall_now);
        self.paused = false;
        self.presentation_hold = true;
    }
}

/// Invalid fixed clock configuration for a kit scene.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorldClockError {
    /// The step must be at least one microsecond.
    StepTooShort,
}
impl std::fmt::Display for WorldClockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("world tick duration must be at least one microsecond")
    }
}
impl Error for WorldClockError {}

impl FrameWorld for StillWorld {
    type Page = SourceTick;

    fn source(&self) -> SourceTick {
        SourceTick {
            tick: self.tick,
            ..STILL
        }
    }
    fn tick_duration(&self) -> Duration {
        self.tick_duration
            .unwrap_or(Duration::from_secs_f64(1.0 / 60.0))
    }
    fn has_current_page(&self) -> bool {
        true
    }
    fn is_threaded(&self) -> bool {
        false
    }
    fn advance(&mut self, now: Duration) -> Result<WorldAdvance, String> {
        let previous = self.tick;
        if let Some(step) = self.tick_duration {
            let wall_now = now.max(self.wall_now);
            if !self.paused {
                self.now += wall_now.saturating_sub(self.wall_now);
            }
            self.wall_now = wall_now;
            self.tick = u64::try_from(self.now.as_nanos() / step.as_nanos()).unwrap_or(u64::MAX);
            if let Some(update) = &mut self.fixed_update {
                for tick in (previous..self.tick).map(|tick| tick + 1) {
                    update(tick, step);
                }
            }
            if self.tick > previous {
                self.presentation_hold = false;
            }
        }
        Ok(WorldAdvance {
            tick: self.tick,
            overload: None,
            completed_ticks: u32::try_from(self.tick.saturating_sub(previous)).unwrap_or(u32::MAX),
            queued_ticks: 0,
            projected_changes: 0,
            projection_lost_ticks: 0,
            skipped_publications: 0,
            recovered_publication: false,
            tick_cpu: Duration::ZERO,
        })
    }
    fn publication(&mut self) -> Result<WorldPublication<SourceTick>, String> {
        Ok(WorldPublication {
            id: SnapshotId {
                epoch: WorldEpoch(STILL.epoch),
                tick: self.tick,
            },
            timeline: STILL.timeline,
            page: self.source(),
        })
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "the interpolation fraction is clamped to zero through one"
    )]
    fn presentation_alpha_at(&self, lead: Duration) -> f32 {
        if !self.is_interpolating() {
            return 1.0;
        }
        self.tick_duration.map_or(1.0, |step| {
            ((self.now + lead).as_secs_f64() / step.as_secs_f64() - self.tick as f64)
                .clamp(0.0, 1.0) as f32
        })
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "bounded presentation phase needs only f32 precision"
    )]
    fn display_phase_at(&self, lead: Duration) -> f32 {
        if !self.is_interpolating() {
            return 1.0;
        }
        self.tick_duration.map_or(1.0, |step| {
            ((self.now + lead).as_secs_f64() / step.as_secs_f64() - self.tick as f64)
                .clamp(0.0, 1.0 + 0.050 / step.as_secs_f64()) as f32
        })
    }
    fn is_interpolating(&self) -> bool {
        self.tick_duration.is_some() && !self.paused && !self.presentation_hold
    }
    fn tick_due_time(&self) -> Option<Duration> {
        if self.paused {
            return None;
        }
        self.tick_duration.map(|step| {
            self.wall_now.saturating_sub(Duration::from_nanos(
                u64::try_from(self.now.as_nanos() % step.as_nanos())
                    .expect("a sub-tick fraction fits in u64 nanoseconds"),
            ))
        })
    }
}

/// Sky, Sun, exposure and camera policy of a still scene, sampled by the renderer.
#[derive(Debug)]
pub struct SkyScene {
    sky: Sky,
    presentation_camera: Option<PresentationCamera>,
    ground: bool,
    temporal_sampling: bool,
    overlay: DisplayOverlay,
}

struct PresentationCamera(Box<dyn Fn(f32) -> Camera + Send>);

impl std::fmt::Debug for PresentationCamera {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PresentationCamera").finish_non_exhaustive()
    }
}

impl SceneSource<SourceTick> for SkyScene {
    fn extract(&self, into: &mut RenderExtractor, page: &SourceTick) -> ExtractReport {
        // The still world's only instance is the ground slab in slot 0; other content comes
        // from the contribution.
        into.extract(*page, [0.0; 3], |sink| {
            if self.ground {
                let centre = [0.0, -f64::from(GROUND_HALF_DEPTH_M), 0.0];
                sink.push(0, 1, 1, centre, 0);
            }
        })
    }

    fn frame(
        &self,
        _source: SourceTick,
        alpha: f32,
        extent: [u32; 2],
        lighting: Option<FrameLighting>,
    ) -> Result<SceneFrame, String> {
        let camera = self
            .presentation_camera
            .as_ref()
            .map_or(self.sky.camera, |sample| (sample.0)(alpha));
        let view = camera
            .view(extent)?
            .with_temporal_inputs(self.sky.temporal_inputs || self.temporal_sampling);
        let view = if self.temporal_sampling {
            view.with_reconstruction(true)
                .with_jitter_sequence(0, extent)
        } else {
            view
        };
        let lighting = match lighting {
            Some(published) => published,
            None => self.sky.lighting()?,
        };
        let view = match lighting {
            // A flat incident-light meter, darkened below daylight so twilight reads dim.
            FrameLighting::Atmosphere(_) => view
                .with_incident_exposure(self.sky.exposure_compensation_ev, 0.0, 18.0)
                .and_then(SyntheticView::with_low_light_adaptation),
            FrameLighting::Diagnostic(_) | FrameLighting::Photometric(_) => {
                view.with_exposure(FALLBACK_EXPOSURE)
            }
        }
        .map_err(|error| format!("view exposure: {error:?}"))?
        .with_display_overlay(if self.sky.exposure_zones {
            DisplayOverlay::ExposureZones
        } else {
            self.overlay
        });
        Ok(SceneFrame { view, lighting })
    }
}

/// The desired sky carried to the render owner, when it changed.
#[cfg(any(test, target_os = "macos", target_arch = "wasm32"))]
#[derive(Clone, Copy, Debug, Default)]
pub struct SkyEdits(Option<Sky>);

#[cfg(any(target_os = "macos", target_arch = "wasm32"))]
impl Edits for SkyEdits {
    fn merge(self, newer: Self) -> Self {
        self.merge_edits(newer)
    }
    fn is_empty(&self) -> bool {
        self.no_changes()
    }
}

#[cfg(any(test, target_os = "macos", target_arch = "wasm32"))]
impl SkyEdits {
    fn merge_edits(self, newer: Self) -> Self {
        Self(newer.0.or(self.0))
    }
    fn no_changes(&self) -> bool {
        self.0.is_none()
    }
}

#[cfg(any(test, target_os = "macos", target_arch = "wasm32"))]
type Update = Box<dyn FnMut(&Controls, &mut Sky)>;
type StatusProbe = Box<dyn FnMut() -> ContentStatus>;
#[cfg(any(target_os = "macos", target_arch = "wasm32"))]
type StatusReadouts = Box<dyn FnMut(&mut Controls)>;
type KeptProbe = Box<dyn FnMut() -> Vec<(u64, String)>>;
type Reload = Box<dyn FnMut() -> Result<(), String>>;
type ReloadReport = Box<dyn FnMut() -> Vec<Result<String, String>>>;

/// A still scene under the sky, hosted by the app shell. See the [module](self) example.
pub struct SkyGame {
    tick_duration: Option<Duration>,
    fixed_update: Option<FixedUpdate>,
    presentation_camera: Option<PresentationCamera>,
    temporal_sampling: bool,
    overlay: DisplayOverlay,
    #[cfg(any(test, target_os = "macos", target_arch = "wasm32"))]
    name: &'static str,
    #[cfg(any(test, target_os = "macos", target_arch = "wasm32"))]
    controls: Controls,
    #[cfg(any(test, target_os = "macos", target_arch = "wasm32"))]
    sky: Sky,
    #[cfg(any(test, target_os = "macos", target_arch = "wasm32"))]
    sent: Option<Sky>,
    #[cfg(any(test, target_os = "macos", target_arch = "wasm32"))]
    refused: Option<String>,
    #[cfg(any(test, target_os = "macos", target_arch = "wasm32"))]
    update: Update,
    ground: Option<[f32; 3]>,
    local_lights: Vec<thrum_render::synthetic::SyntheticPointLight>,
    contribution: Option<Box<dyn SceneContribution>>,
    status: Option<StatusProbe>,
    #[cfg(any(target_os = "macos", target_arch = "wasm32"))]
    readouts: Option<StatusReadouts>,
    #[cfg(any(target_os = "macos", target_arch = "wasm32"))]
    shown_status: Option<ContentStatus>,
    /// New loader failures once content has drawn, each with the generation that stays drawn.
    kept: Option<KeptProbe>,
    reload: Option<(Reload, ReloadReport)>,
    /// The files `reload` reads, watched by the window and a live capture.
    #[cfg(target_os = "macos")]
    reload_files: Vec<PathBuf>,
}

impl SkyGame {
    /// A scene with `controls` in the panel, `camera`, the Sun 30° high in the south-east and
    /// the default ground. `update` runs before every frame (and after every control edit) to
    /// map the controls onto the [`Sky`]; a value the renderer cannot draw is reported and the
    /// previous sky kept.
    /// # Panics
    /// Panics if the supplied controls already use the kit's reserved `profiler` keys.
    pub fn new(
        mut controls: Controls,
        camera: Camera,
        update: impl FnMut(&Controls, &mut Sky) + 'static,
    ) -> Self {
        crate::profile::declare(&mut controls).expect("kit reserves the profiler group");
        #[cfg(not(any(test, target_os = "macos", target_arch = "wasm32")))]
        drop((controls, camera, update));
        Self {
            tick_duration: None,
            fixed_update: None,
            presentation_camera: None,
            temporal_sampling: false,
            overlay: DisplayOverlay::Off,
            #[cfg(any(test, target_os = "macos", target_arch = "wasm32"))]
            name: "thrum",
            #[cfg(any(test, target_os = "macos", target_arch = "wasm32"))]
            controls,
            #[cfg(any(test, target_os = "macos", target_arch = "wasm32"))]
            sky: Sky {
                exposure_compensation_ev: 0.0,
                temporal_inputs: false,
                sun_elevation_degrees: 30.0,
                sun_azimuth_degrees: 135.0,
                camera,
                exposure_zones: false,
            },
            #[cfg(any(test, target_os = "macos", target_arch = "wasm32"))]
            sent: None,
            #[cfg(any(test, target_os = "macos", target_arch = "wasm32"))]
            refused: None,
            #[cfg(any(test, target_os = "macos", target_arch = "wasm32"))]
            update: Box::new(update),
            ground: Some(DEFAULT_GROUND),
            local_lights: Vec::new(),
            contribution: None,
            status: None,
            #[cfg(any(target_os = "macos", target_arch = "wasm32"))]
            readouts: None,
            #[cfg(any(target_os = "macos", target_arch = "wasm32"))]
            shown_status: None,
            kept: None,
            reload: None,
            #[cfg(target_os = "macos")]
            reload_files: Vec::new(),
        }
    }

    /// The ground: a flat, rough surface at Y = 0, about 8 km across, lit by the Sun and sky
    /// and receiving the Sun's shadows. `colour` is its linear base colour (each channel 0–1;
    /// [`DEFAULT_GROUND`] unless set), and `None` removes it, leaving the sky's distant planet
    /// surface below the horizon.
    #[must_use]
    pub fn with_ground(mut self, colour: Option<[f32; 3]>) -> Self {
        self.ground = colour;
        self
    }

    /// Place world-space point lights, in metres and RGB candela, in the sky scene.
    #[must_use]
    pub fn with_local_lights(
        mut self,
        lights: Vec<thrum_render::synthetic::SyntheticPointLight>,
    ) -> Self {
        self.local_lights = lights;
        self
    }

    /// Select the registry's temporal reconstructor and jittered sampling when
    /// this scene is prepared. The default kit scene uses spatial sampling.
    #[must_use]
    pub fn with_temporal_sampling(mut self, enabled: bool) -> Self {
        self.temporal_sampling = enabled;
        self
    }
    /// Prepare a renderer diagnostic overlay, including the read-only temporal
    /// input witness. Exposure zones selected by the controls take precedence.
    #[must_use]
    pub fn with_display_overlay(mut self, overlay: DisplayOverlay) -> Self {
        self.overlay = overlay;
        self
    }

    /// Advance this game's world at a fixed step. The default kit world stays still.
    /// # Errors
    /// Rejects zero or sub-microsecond steps.
    pub fn with_tick_duration(mut self, step: Duration) -> Result<Self, WorldClockError> {
        if step < Duration::from_micros(1) {
            return Err(WorldClockError::StepTooShort);
        }
        self.tick_duration = Some(step);
        Ok(self)
    }

    /// Run game-owned updates for each committed tick of the configured fixed clock.
    /// The callback is also called at tick zero to publish the initial game state.
    #[must_use]
    pub fn with_fixed_update(mut self, update: impl FnMut(u64, Duration) + Send + 'static) -> Self {
        self.fixed_update = Some(Box::new(update));
        self
    }

    /// Sample a game-owned camera from the same interpolation fraction used to draw the scene.
    #[must_use]
    pub fn with_presentation_camera(
        mut self,
        camera: impl Fn(f32) -> Camera + Send + 'static,
    ) -> Self {
        self.presentation_camera = Some(PresentationCamera(Box::new(camera)));
        self
    }

    /// Publish game-owned readouts into the panel when the scene status is polled.
    #[must_use]
    pub fn with_status_readouts(self, readouts: impl FnMut(&mut Controls) + 'static) -> Self {
        #[cfg(any(target_os = "macos", target_arch = "wasm32"))]
        {
            let mut game = self;
            game.readouts = Some(Box::new(readouts));
            game
        }
        #[cfg(not(any(target_os = "macos", target_arch = "wasm32")))]
        {
            drop(readouts);
            self
        }
    }

    #[cfg(any(test, target_os = "macos", target_arch = "wasm32"))]
    fn take_world(&mut self) -> StillWorld {
        let mut fixed_update = self.fixed_update.take();
        if let (Some(step), Some(update)) = (self.tick_duration, &mut fixed_update) {
            update(0, step);
        }
        StillWorld {
            tick_duration: self.tick_duration,
            fixed_update,
            ..StillWorld::default()
        }
    }

    /// Draw cooked meshes from the store named by `THRUM_ASSET_STORE` ([`STORE_VARIABLE`]),
    /// which `./dev scene <folder>` cooks from the folder's `assets/` and keeps cooking as
    /// they change. The window logs each newly drawn generation; a capture waits for the first.
    /// When `THRUM_MATERIAL` ([`MATERIAL_VARIABLE`]) names a WGSL material hook, it is loaded
    /// with the window and reloaded on `L`; a hook that fails to compile is explained by file and
    /// line while the previous material keeps drawing.
    ///
    /// # Errors
    /// The variable is unset or names a store nothing was cooked into, or a placement's
    /// transform is not rigid with one uniform scale; each message says what to do.
    pub fn with_meshes(self, meshes: Vec<MeshPlacement>) -> Result<Self, String> {
        self.with_meshes_and_material(meshes, None)
            .map_err(|error| error.to_string())
    }

    /// Draw cooked meshes with an optional initial WGSL material hook on both platforms.
    /// The native file watcher can replace this initial hook when a material file is configured.
    ///
    /// # Errors
    /// Returns the same asset and placement errors as [Self::with_meshes].
    pub fn with_meshes_and_material(
        self,
        meshes: Vec<MeshPlacement>,
        initial: Option<(&str, &str)>,
    ) -> Result<Self, CookedSetupError> {
        self.with_meshes_and_texture_lease(meshes, initial, &[])
            .map(|(game, _)| game)
    }

    /// Draw cooked meshes and lend named 2D textures from the same publication.
    /// # Errors
    /// Returns the same asset and placement errors as [Self::with_meshes].
    pub fn with_meshes_and_texture_lease(
        self,
        meshes: Vec<MeshPlacement>,
        initial: Option<(&str, &str)>,
        textures: &[&str],
    ) -> Result<(Self, thrum_asset_gpu::ProductLease), CookedSetupError> {
        #[cfg(not(target_arch = "wasm32"))]
        let source: thrum_asset_gpu::CookedSource = cooked_store(std::env::var_os(STORE_VARIABLE))
            .map_err(CookedSetupError::Store)?
            .into();
        #[cfg(target_arch = "wasm32")]
        let source = thrum_asset_gpu::CookedSource::Remote(
            browser::feed().map_err(CookedSetupError::Store)?,
        );
        let names = meshes
            .iter()
            .map(|mesh| format!("mesh {} with texture {}", mesh.mesh, mesh.texture))
            .collect::<Vec<_>>()
            .join(", ");
        let status_names = names.clone();
        let mut config = CookedConfig::new(source, meshes);
        config.leased = textures
            .iter()
            .map(|name| thrum_asset_gpu::LeaseRequest {
                name: (*name).into(),
                dimension: wgpu::TextureDimension::D2,
            })
            .collect();
        let assets = CookedAssets::new(config).map_err(CookedSetupError::Placement)?;
        let lease = assets.lease();
        let stats = assets.stats();
        let material = assets.material();
        if let Some((name, source)) = initial {
            material.request(MaterialHook {
                name: name.into(),
                source: source.into(),
            });
        }
        let kept = Arc::clone(&stats);
        let mut shown = 0;
        let mut game =
            self.with_contribution(Box::new(assets), move || match cooked_status(&stats) {
                ContentStatus::Failed(error) => ContentStatus::Failed(format!(
                    "{status_names}: {error}; fix the asset and save again"
                )),
                status => status,
            });
        game.kept = Some(Box::new(move || {
            kept.lock()
                .map_or_else(|_| Vec::new(), |stats| {
                    kept_errors(&stats, &mut shown)
                        .into_iter()
                        .map(|(generation, error)| {
                            (generation, format!("{names}: {error}; restore the named asset under assets/ and save again"))
                        })
                        .collect()
                })
        }));
        #[cfg(target_arch = "wasm32")]
        return Ok((game, lease));
        #[cfg(not(target_arch = "wasm32"))]
        {
            let Some(path) = std::env::var_os(MATERIAL_VARIABLE).map(PathBuf::from) else {
                return Ok((game, lease));
            };
            let reload = material.clone();
            #[cfg(target_os = "macos")]
            let watched = path.clone();
            let report = move || {
                material
                    .take_outcomes()
                    .iter()
                    .map(material_drawn)
                    .collect()
            };
            let game = game.with_reload(
                move || {
                    reload.request(MaterialHook::read(&path).map_err(|error| error.to_string())?);
                    Ok(())
                },
                report,
            );
            #[cfg(target_os = "macos")]
            let game = {
                let mut game = game;
                game.reload_files = vec![watched];
                game
            };
            Ok((game, lease))
        }
    }

    /// Draw an engine contribution lit by the sky. `status` reports its progress: the window
    /// logs each change, and a capture waits until it is drawn. [`Self::with_meshes`] does this
    /// for cooked meshes.
    #[must_use]
    pub fn with_contribution(
        mut self,
        contribution: Box<dyn SceneContribution>,
        mut status: impl FnMut() -> ContentStatus + 'static,
    ) -> Self {
        self.contribution = Some(match self.contribution.take() {
            Some(existing) => Box::new(thrum_render::contribution::SceneContributions::new(vec![
                existing,
                contribution,
            ])),
            None => contribution,
        });
        self.status = Some(match self.status.take() {
            Some(mut existing) => Box::new(move || match (existing(), status()) {
                (ContentStatus::Failed(error), _) | (_, ContentStatus::Failed(error)) => {
                    ContentStatus::Failed(error)
                }
                (ContentStatus::Loading, _) | (_, ContentStatus::Loading) => ContentStatus::Loading,
                (ContentStatus::Drawn(a), ContentStatus::Drawn(b)) => {
                    ContentStatus::Drawn(a.max(b))
                }
            }),
            None => Box::new(status),
        });
        self
    }

    /// Reload a game file when the window opens and on `L`, after the parameter file: `reload`
    /// starts it (an error is shown and nothing changes), and `report`, polled every frame,
    /// returns what became of earlier reloads, problems as errors. A capture reloads once and
    /// waits for the first report, refusing a problem.
    #[must_use]
    pub fn with_reload(
        mut self,
        reload: impl FnMut() -> Result<(), String> + 'static,
        report: impl FnMut() -> Vec<Result<String, String>> + 'static,
    ) -> Self {
        self.reload = Some((Box::new(reload), Box::new(report)));
        self
    }

    /// Show what became of earlier reloads.
    #[expect(
        clippy::print_stderr,
        reason = "reload outcomes are reported to the person editing the file"
    )]
    #[cfg(any(target_os = "macos", target_arch = "wasm32"))]
    fn poll_reload(&mut self) {
        if let Some((_, report)) = self.reload.as_mut() {
            for line in report() {
                match line {
                    Ok(line) | Err(line) => eprintln!("{}: {line}", self.name),
                }
            }
        }
    }

    /// Run the update and return the sky if it changed since it was last sent.
    #[expect(
        clippy::print_stderr,
        reason = "a refused sky is explained to the person editing the scene"
    )]
    #[cfg(any(test, target_os = "macos", target_arch = "wasm32"))]
    fn update(&mut self) -> SkyEdits {
        let mut sky = self.sky;
        (self.update)(&self.controls, &mut sky);
        if let Err(error) = sky.check() {
            if self.refused.as_ref() != Some(&error) {
                eprintln!(
                    "{}: sky refused: {error}; keeping the previous sky",
                    self.name
                );
                self.refused = Some(error);
            }
            return SkyEdits(None);
        }
        self.refused = None;
        self.sky = sky;
        if self.sent == Some(sky) {
            return SkyEdits(None);
        }
        self.sent = Some(sky);
        SkyEdits(Some(sky))
    }

    /// Poll the content probe and log a change. Offscreen callers can wait
    /// for the first cooked generation before recording a capture or timing.
    #[expect(
        clippy::print_stderr,
        reason = "content progress and hot reloads are reported to the person editing assets"
    )]
    #[cfg(any(target_os = "macos", target_arch = "wasm32"))]
    pub fn poll_status(&mut self) -> Option<ContentStatus> {
        if let Some(publish) = self.readouts.as_mut() {
            publish(&mut self.controls);
        }
        let status = (self.status.as_mut()?)();
        if self.shown_status.as_ref() != Some(&status) {
            match &status {
                ContentStatus::Loading => {}
                ContentStatus::Drawn(generation) => {
                    eprintln!("{}: content generation {generation} drawn", self.name);
                }
                ContentStatus::Failed(error) => {
                    eprintln!("{}: content failed: {error}", self.name);
                }
            }
            self.shown_status = Some(status.clone());
        }
        Some(status)
    }

    /// Poll for content failures after the first draw and log each once: the loader refused a
    /// newer generation and the returned one keeps drawing.
    #[expect(
        clippy::print_stderr,
        reason = "a refused asset edit is explained to the person editing assets"
    )]
    #[cfg(any(target_os = "macos", target_arch = "wasm32"))]
    fn poll_kept(&mut self) -> Vec<(u64, String)> {
        let Some(kept) = self.kept.as_mut() else {
            return Vec::new();
        };
        let kept = kept();
        for (generation, error) in &kept {
            eprintln!(
                "{}: content generation {generation} kept: {error}",
                self.name
            );
        }
        kept
    }

    /// Prepare the still scene on `capabilities`' device, with the contribution attached.
    #[cfg(any(test, target_os = "macos", target_arch = "wasm32"))]
    async fn build_host(
        &mut self,
        capabilities: &DeviceCapabilities,
        queue: &wgpu::Queue,
        config: FrameConfig,
    ) -> Result<FrameHost<StillWorld, SkyScene>, String> {
        let _ = self.update();
        self.sky.check()?;
        let world = self.take_world();
        let mut host = FrameHost::new(
            capabilities,
            queue,
            config,
            world,
            SkyScene {
                sky: self.sky,
                presentation_camera: self.presentation_camera.take(),
                ground: self.ground.is_some(),
                temporal_sampling: self.temporal_sampling,
                overlay: self.overlay,
            },
            ground_scene(self.ground, &self.local_lights)?,
            FALLBACK_EXPOSURE,
        )
        .await
        .map_err(|error| error.to_string())?;
        if let Some(contribution) = self.contribution.take() {
            host.set_scene_contribution(contribution)?;
        }
        Ok(host)
    }
}

#[cfg(any(target_os = "macos", target_arch = "wasm32"))]
impl Game for SkyGame {
    type World = StillWorld;
    type Scene = SkyScene;
    type Edits = SkyEdits;
    type Readback = ();

    async fn prepare<'a>(
        &'a mut self,
        gpu: Gpu<'a>,
        _world_advanced: Box<dyn Fn() + Send>,
    ) -> Result<Prepared<StillWorld, SkyScene>, String> {
        let host = self
            .build_host(
                gpu.capabilities,
                gpu.queue,
                FrameConfig {
                    render_extent: gpu.surface.extent,
                    surface: Some(gpu.surface),
                },
            )
            .await?;
        Ok(Prepared {
            host,
            origin: Instant::now(),
        })
    }

    fn controls(&self) -> &Controls {
        &self.controls
    }

    fn controls_mut(&mut self) -> &mut Controls {
        &mut self.controls
    }

    fn take_edits(&mut self) -> (SkyEdits, Option<bool>) {
        (self.update(), None)
    }

    fn frame_edits(&mut self) -> SkyEdits {
        #[cfg(target_os = "macos")]
        {
            drop(self.poll_status());
            drop(self.poll_kept());
        }
        // The browser report consumes the same content outcomes for the live page.
        self.poll_reload();
        self.update()
    }

    #[expect(
        clippy::print_stderr,
        reason = "a file that cannot be reloaded is explained to the person editing it"
    )]
    fn reload(&mut self) {
        if let Some((reload, _)) = self.reload.as_mut()
            && let Err(error) = reload()
        {
            eprintln!(
                "{}: {error}; nothing changed (fix it and save, or press L)",
                self.name
            );
        }
    }

    #[cfg(target_os = "macos")]
    fn reload_files(&self) -> Vec<PathBuf> {
        self.reload_files.clone()
    }

    fn replay(&self) -> SkyEdits {
        SkyEdits(Some(self.sky))
    }

    fn apply(host: &mut FrameHost<StillWorld, SkyScene>, edits: SkyEdits) -> Result<(), String> {
        if let Some(sky) = edits.0
            && host.source().sky != sky
        {
            if sky.temporal_inputs {
                host.continuous_source_mut().sky = sky;
            } else {
                host.source_mut().sky = sky;
                host.invalidate_history();
            }
        }
        Ok(())
    }

    fn readback(_host: &FrameHost<StillWorld, SkyScene>) {}

    fn observe(&mut self, (): ()) {}
}

#[cfg(any(target_os = "macos", target_arch = "wasm32"))]
impl<K> GameInput<K> for SkyGame {}

/// The scene's one instance row: the ground slab, or an inactive row (generation zero) that
/// draws nothing, since the renderer needs at least one.
#[cfg(any(test, target_os = "macos", target_arch = "wasm32"))]
fn ground_scene(
    ground: Option<[f32; 3]>,
    lights: &[thrum_render::synthetic::SyntheticPointLight],
) -> Result<SyntheticScene, String> {
    let mut scene = SyntheticScene::grid(1).map_err(|error| format!("sky scene: {error:?}"))?;
    scene.sun_shadows = true;
    scene.local_lights.extend_from_slice(lights);
    let Some(colour) = ground else {
        scene.instances[0].deactivate();
        return Ok(scene);
    };
    if !colour.iter().all(|channel| (0.0..=1.0).contains(channel)) {
        return Err(format!(
            "ground colour {colour:?} must be linear RGB with each channel within 0–1"
        ));
    }
    let [r, g, b] = colour;
    scene.materials = vec![MaterialRecord {
        base_rgba: [r, g, b, 1.0],
        surface: [1.0, GROUND_ROUGHNESS, 0.0, 1.0],
        ..MaterialRecord::default()
    }];
    // The synthetic cube spans -1..1 on each axis: scale it to a wide, thin slab.
    let half = [
        GROUND_HALF_WIDTH_M,
        GROUND_HALF_DEPTH_M,
        GROUND_HALF_WIDTH_M,
    ];
    let mut slab = InstanceRecord::identity(0, 0);
    for (axis, extent) in half.into_iter().enumerate() {
        slab.current[axis][axis] = extent;
        slab.normal[axis][axis] = extent.recip();
    }
    slab.current[3][1] = -GROUND_HALF_DEPTH_M;
    slab.previous = slab.current;
    scene.instances[0] = slab;
    Ok(scene)
}

/// The cooked store `THRUM_ASSET_STORE` names, refused with the next step when it is unset or
/// nothing was cooked there.
#[cfg(not(target_arch = "wasm32"))]
fn cooked_store(variable: Option<std::ffi::OsString>) -> Result<PathBuf, String> {
    let store = variable.map(PathBuf::from).ok_or(
        "THRUM_ASSET_STORE is not set, so there are no cooked meshes to draw; run the scene \
         with ./dev scene <its folder>, which cooks its assets/ and sets it",
    )?;
    if !store.join("HEAD").is_file() {
        return Err(format!(
            "asset store {} has no HEAD: nothing has been cooked there; run ./dev scene <the \
             scene's folder>, or cargo run --release -p thrum-cook -- cook <its assets folder> {}",
            store.display(),
            store.display()
        ));
    }
    Ok(store)
}

/// What cooked assets show: the drawn generation, or why nothing is drawn yet.
/// A drawn material hook, or why the previous material still draws.
#[cfg(not(target_arch = "wasm32"))]
fn material_drawn(outcome: &MaterialOutcome) -> Result<String, String> {
    match outcome {
        MaterialOutcome::Promoted { .. } => Ok(outcome.to_string()),
        MaterialOutcome::Rejected { .. } => Err(format!(
            "{outcome}; fix the reported material hook and save it again"
        )),
        _ => Err(outcome.to_string()),
    }
}

/// Loader failures recorded after the `shown` count and after the first draw, each with the
/// generation drawn when polled (a failure and a newer swap in one poll pair with the newer one);
/// none before the first draw, when [`cooked_status`] reports the newest as the failure.
fn kept_errors(stats: &CookedStats, shown: &mut u64) -> Vec<(u64, String)> {
    let since = (*shown).max(stats.failures_before_first_draw);
    let fresh = usize::try_from(stats.failures.saturating_sub(since))
        .unwrap_or(usize::MAX)
        .min(stats.errors.len());
    *shown = stats.failures;
    let Some(generation) = stats.drawn else {
        return Vec::new();
    };
    stats.errors[stats.errors.len() - fresh..]
        .iter()
        .map(|error| (generation, error.clone()))
        .collect()
}

fn cooked_status(stats: &Mutex<CookedStats>) -> ContentStatus {
    let Ok(stats) = stats.lock() else {
        return ContentStatus::Failed("cooked asset observations were poisoned".into());
    };
    match (stats.drawn, stats.errors.last()) {
        (Some(generation), _) => ContentStatus::Drawn(generation),
        (None, Some(error)) => ContentStatus::Failed(error.clone()),
        (None, None) => ContentStatus::Loading,
    }
}

/// Open the scene `game` builds in a window until it is closed (`config.frames_variable`
/// bounds a scripted run; `THRUM_REPORT=1` prints the frames report at the end), or, when
/// `THRUM_CAPTURE_PNG` names a file, render one frame there offscreen after applying
/// `THRUM_PARAMS` and waiting for cooked meshes to draw. Failures, including an error `game`
/// returns, are explained on standard error with a nonzero exit.
#[cfg(target_os = "macos")]
pub fn run(config: Config, game: impl FnOnce() -> Result<SkyGame, Box<dyn Error>>) {
    let named = move || {
        game().map_err(|error| error.to_string()).map(|mut game| {
            game.name = config.name;
            game
        })
    };
    match std::env::var_os(CAPTURE_VARIABLE) {
        Some(path) if std::env::var_os(live_capture::LIVE_VARIABLE).is_some() => {
            live_capture::run(config, named, std::path::Path::new(&path));
        }
        Some(path) => capture::run(config, named, std::path::Path::new(&path)),
        None => crate::run_until_closed(config, named),
    }
}

#[cfg(test)]
mod tests;

#[cfg(target_arch = "wasm32")]
pub use browser::{BrowserScene, create_scene, run};

/// Report the available runners on native platforms without a sky window shell.
#[cfg(not(any(target_os = "macos", target_arch = "wasm32")))]
#[expect(
    clippy::print_stderr,
    reason = "the scene runner explains its supported platforms"
)]
pub fn run(config: Config, _game: impl FnOnce() -> Result<SkyGame, Box<dyn Error>>) {
    eprintln!(
        "{}: the native sky window is unavailable on {}; run ./dev scene <folder> --web in Chrome, or run the native scene on macOS",
        config.name,
        std::env::consts::OS
    );
    std::process::exit(1);
}
