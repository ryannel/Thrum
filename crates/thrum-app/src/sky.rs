//! A still scene under a physically based sky: the kit behind the starter.
//!
//! [`SkyGame`] is a complete [`Game`]: the world never ticks, the renderer draws Earth's
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

mod capture;
mod live_capture;

use std::{
    error::Error,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use thrum_asset_gpu::{CookedAssets, CookedConfig, CookedStats, MaterialHook, MaterialOutcome};
pub use thrum_asset_gpu::{CookedMaterial, MeshPlacement};
pub use thrum_controls::{Controls, Display};
use thrum_core::{
    WorldEpoch,
    sim::{SnapshotId, WorldAdvance, WorldPublication},
};
use thrum_render::{
    abi::{InstanceRecord, MaterialRecord},
    atmosphere::AtmosphereFrame,
    capabilities::DeviceCapabilities,
    contribution::SceneContribution,
    extract::{ExtractReport, RenderExtractor, SourceTick},
    runtime::{FrameConfig, FrameHost, FrameWorld, SceneFrame, SceneSource},
    synthetic::{DisplayOverlay, FrameLighting, SdrToneMap, SyntheticScene, SyntheticView},
};
use winit::keyboard::KeyCode;

use crate::{Config, Edits, Game, Gpu, KeyResponse, Prepared};

/// Environment variable naming a PNG file: [`run`] renders one frame there without a window.
pub const CAPTURE_VARIABLE: &str = "THRUM_CAPTURE_PNG";
/// Environment variable naming the cooked asset store that [`SkyGame::with_meshes`] draws
/// from; `./dev scene <folder>` cooks the folder's `assets/` and sets it.
pub const STORE_VARIABLE: &str = "THRUM_ASSET_STORE";
/// Names the cooked meshes' WGSL material hook, which `./dev scene` sets to the folder's
/// `material.wgsl` when there is one.
pub const MATERIAL_VARIABLE: &str = "THRUM_MATERIAL";

/// Linear base colour of the default ground: dry, grassy earth.
pub const DEFAULT_GROUND: [f32; 3] = [0.16, 0.15, 0.11];
/// Half the width of the ground square, in metres: it reaches within a pixel of the horizon
/// from standing height, where the sky's own planet surface takes over.
const GROUND_HALF_WIDTH_M: f32 = 4_000.0;
/// Half the ground slab's thickness; its top face is the ground at Y = 0.
const GROUND_HALF_DEPTH_M: f32 = 0.5;
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
    fn check(&self) -> Result<(), String> {
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

/// The world of a still scene: one committed endpoint that never ticks.
#[derive(Debug, Default)]
pub struct StillWorld;

impl FrameWorld for StillWorld {
    type Page = SourceTick;

    fn source(&self) -> SourceTick {
        STILL
    }
    fn has_current_page(&self) -> bool {
        true
    }
    fn is_threaded(&self) -> bool {
        false
    }
    fn advance(&mut self, _now: Duration) -> Result<WorldAdvance, String> {
        Ok(WorldAdvance {
            tick: STILL.tick,
            overload: None,
            completed_ticks: 0,
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
                tick: STILL.tick,
            },
            timeline: STILL.timeline,
            page: STILL,
        })
    }
    fn presentation_alpha_at(&self, _lead: Duration) -> f32 {
        1.0
    }
}

/// Sky, Sun, exposure and camera policy of a still scene, sampled by the renderer.
#[derive(Debug)]
pub struct SkyScene {
    sky: Sky,
    ground: bool,
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
        _alpha: f32,
        extent: [u32; 2],
        lighting: Option<FrameLighting>,
    ) -> Result<SceneFrame, String> {
        let view = self.sky.camera.view(extent)?;
        let lighting = match lighting {
            Some(published) => published,
            None => self.sky.lighting()?,
        };
        let view = match lighting {
            // A flat incident-light meter, darkened below daylight so twilight reads dim.
            FrameLighting::Atmosphere(_) => view
                .with_incident_exposure(0.0, 0.0, 18.0)
                .and_then(SyntheticView::with_low_light_adaptation),
            FrameLighting::Diagnostic(_) | FrameLighting::Photometric(_) => {
                view.with_exposure(FALLBACK_EXPOSURE)
            }
        }
        .map_err(|error| format!("view exposure: {error:?}"))?
        .with_display_overlay(if self.sky.exposure_zones {
            DisplayOverlay::ExposureZones
        } else {
            DisplayOverlay::Off
        });
        Ok(SceneFrame { view, lighting })
    }
}

/// The desired sky carried to the render owner, when it changed.
#[derive(Clone, Copy, Debug, Default)]
pub struct SkyEdits(Option<Sky>);

impl Edits for SkyEdits {
    fn merge(self, newer: Self) -> Self {
        Self(newer.0.or(self.0))
    }
    fn is_empty(&self) -> bool {
        self.0.is_none()
    }
}

type Update = Box<dyn FnMut(&Controls, &mut Sky)>;
type StatusProbe = Box<dyn FnMut() -> ContentStatus>;
type KeptProbe = Box<dyn FnMut() -> Vec<(u64, String)>>;
type Reload = Box<dyn FnMut() -> Result<(), String>>;
type ReloadReport = Box<dyn FnMut() -> Vec<Result<String, String>>>;

/// A still scene under the sky, hosted by the app shell. See the [module](self) example.
pub struct SkyGame {
    name: &'static str,
    controls: Controls,
    sky: Sky,
    sent: Option<Sky>,
    refused: Option<String>,
    update: Update,
    ground: Option<[f32; 3]>,
    contribution: Option<Box<dyn SceneContribution>>,
    status: Option<StatusProbe>,
    shown_status: Option<ContentStatus>,
    /// New loader failures once content has drawn, each with the generation that stays drawn.
    kept: Option<KeptProbe>,
    reload: Option<(Reload, ReloadReport)>,
    /// The files `reload` reads, watched by the window and a live capture.
    reload_files: Vec<PathBuf>,
}

impl SkyGame {
    /// A scene with `controls` in the panel, `camera`, the Sun 30° high in the south-east and
    /// the default ground. `update` runs before every frame (and after every control edit) to
    /// map the controls onto the [`Sky`]; a value the renderer cannot draw is reported and the
    /// previous sky kept.
    pub fn new(
        controls: Controls,
        camera: Camera,
        update: impl FnMut(&Controls, &mut Sky) + 'static,
    ) -> Self {
        Self {
            name: "thrum",
            controls,
            sky: Sky {
                sun_elevation_degrees: 30.0,
                sun_azimuth_degrees: 135.0,
                camera,
                exposure_zones: false,
            },
            sent: None,
            refused: None,
            update: Box::new(update),
            ground: Some(DEFAULT_GROUND),
            contribution: None,
            status: None,
            shown_status: None,
            kept: None,
            reload: None,
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
        let store = cooked_store(std::env::var_os(STORE_VARIABLE))?;
        let assets = CookedAssets::new(CookedConfig::new(store, meshes))
            .map_err(|error| error.to_string())?;
        let stats = assets.stats();
        let material = assets.material();
        let kept = Arc::clone(&stats);
        let mut shown = 0;
        let mut game = self.with_contribution(Box::new(assets), move || cooked_status(&stats));
        game.kept = Some(Box::new(move || {
            kept.lock()
                .map(|stats| kept_errors(&stats, &mut shown))
                .unwrap_or_default()
        }));
        let Some(path) = std::env::var_os(MATERIAL_VARIABLE).map(PathBuf::from) else {
            return Ok(game);
        };
        let reload = material.clone();
        let watched = path.clone();
        let report = move || {
            material
                .take_outcomes()
                .iter()
                .map(material_drawn)
                .collect()
        };
        let mut game = game.with_reload(
            move || {
                reload.request(MaterialHook::read(&path)?);
                Ok(())
            },
            report,
        );
        game.reload_files = vec![watched];
        Ok(game)
    }

    /// Draw an engine contribution lit by the sky. `status` reports its progress: the window
    /// logs each change, and a capture waits until it is drawn. [`Self::with_meshes`] does this
    /// for cooked meshes.
    #[must_use]
    pub fn with_contribution(
        mut self,
        contribution: Box<dyn SceneContribution>,
        status: impl FnMut() -> ContentStatus + 'static,
    ) -> Self {
        self.contribution = Some(contribution);
        self.status = Some(Box::new(status));
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

    /// Poll the content probe and log a change.
    #[expect(
        clippy::print_stderr,
        reason = "content progress and hot reloads are reported to the person editing assets"
    )]
    fn poll_status(&mut self) -> Option<ContentStatus> {
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
    fn build_host(
        &mut self,
        capabilities: &DeviceCapabilities,
        queue: &wgpu::Queue,
        config: FrameConfig,
    ) -> Result<FrameHost<StillWorld, SkyScene>, String> {
        let _ = self.update();
        self.sky.check()?;
        let mut host = pollster::block_on(FrameHost::new(
            capabilities,
            queue,
            config,
            StillWorld,
            SkyScene {
                sky: self.sky,
                ground: self.ground.is_some(),
            },
            ground_scene(self.ground)?,
            FALLBACK_EXPOSURE,
        ))
        .map_err(|error| error.to_string())?;
        if let Some(contribution) = self.contribution.take() {
            host.set_scene_contribution(contribution)?;
        }
        Ok(host)
    }
}

impl Game for SkyGame {
    type World = StillWorld;
    type Scene = SkyScene;
    type Edits = SkyEdits;
    type Readback = ();

    fn prepare(
        &mut self,
        gpu: Gpu<'_>,
        _world_advanced: Box<dyn Fn() + Send>,
    ) -> Result<Prepared<StillWorld, SkyScene>, String> {
        let host = self.build_host(
            gpu.capabilities,
            gpu.queue,
            FrameConfig {
                render_extent: gpu.surface.extent,
                surface: Some(gpu.surface),
            },
        )?;
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
        drop(self.poll_status());
        drop(self.poll_kept());
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
            host.source_mut().sky = sky;
            host.invalidate_history();
        }
        Ok(())
    }

    fn readback(_host: &FrameHost<StillWorld, SkyScene>) {}

    fn observe(&mut self, (): ()) {}

    fn key(&mut self, _code: KeyCode, _pressed: bool) -> Result<KeyResponse, String> {
        Ok(KeyResponse::Ignored)
    }
}

/// The scene's one instance row: the ground slab, or an inactive row (generation zero) that
/// draws nothing, since the renderer needs at least one.
fn ground_scene(ground: Option<[f32; 3]>) -> Result<SyntheticScene, String> {
    let mut scene = SyntheticScene::grid(1).map_err(|error| format!("sky scene: {error:?}"))?;
    scene.sun_shadows = true;
    let Some(colour) = ground else {
        scene.instances[0].ids[3] = 0;
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
fn material_drawn(outcome: &MaterialOutcome) -> Result<String, String> {
    match outcome {
        MaterialOutcome::Promoted { .. } => Ok(outcome.to_string()),
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
