//! A still scene under a physically based sky: the kit behind the starter.
//!
//! [`SkyGame`] is a complete [`Game`]: the world never ticks, the renderer draws Earth's
//! atmosphere and Sun with a camera meter for exposure, and an optional engine contribution
//! (for example `thrum_asset_gpu::CookedAssets`) draws into the same lit scene. Each frame
//! the game's update maps its [`Controls`] onto a [`Sky`]: the Sun's elevation and azimuth
//! and a look-at [`Camera`]. [`run`] opens the window, or renders one PNG offscreen when
//! `THRUM_CAPTURE_PNG` names a file.
//!
//! World units are metres with +Y up and sea level at Y = 0; north is -Z and east is +X.
//!
//! ```no_run
//! use thrum_app::{Config, sky::{self, Camera, SkyGame}};
//! use thrum_controls::{Controls, Display};
//!
//! sky::run(
//!     Config {
//!         name: "my-scene",
//!         frames_variable: "MY_SCENE_FRAMES",
//!         window_title: "My scene",
//!         title: "My scene",
//!         render_thread: "my-scene-render",
//!         panel_title: "My scene",
//!         window_size: [1280, 720],
//!     },
//!     || {
//!         let mut controls = Controls::new();
//!         let sun = controls
//!             .slider("sun.elevation", "Sun elevation", -10.0..=90.0)
//!             .default(20.0)
//!             .display(Display::plain(1, "°"))
//!             .add()
//!             .map_err(|error| error.to_string())?;
//!         let camera = Camera::look_at([0.0, 1.7, 6.0], [0.0, 1.0, 0.0], 40.0);
//!         Ok(SkyGame::new(controls, camera, move |controls, sky| {
//!             sky.sun_elevation_degrees = controls.get(sun);
//!         }))
//!     },
//! );
//! ```

mod capture;

use std::time::{Duration, Instant};

use thrum_controls::Controls;
use thrum_core::{
    WorldEpoch,
    sim::{SnapshotId, WorldAdvance, WorldPublication},
};
use thrum_render::{
    atmosphere::AtmosphereFrame,
    capabilities::DeviceCapabilities,
    contribution::SceneContribution,
    extract::{ExtractReport, RenderExtractor, SourceTick},
    runtime::{FrameConfig, FrameHost, FrameWorld, SceneFrame, SceneSource},
    synthetic::{FrameLighting, SdrToneMap, SyntheticScene, SyntheticView},
};
use winit::keyboard::KeyCode;

use crate::{Config, Edits, Game, Gpu, KeyResponse, Prepared};

/// Environment variable naming a PNG file: [`run`] renders one frame there without a window.
pub const CAPTURE_VARIABLE: &str = "THRUM_CAPTURE_PNG";

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
}

impl SceneSource<SourceTick> for SkyScene {
    fn extract(&self, into: &mut RenderExtractor, page: &SourceTick) -> ExtractReport {
        // The still world has no instances; content comes from the contribution.
        into.extract(*page, [0.0; 3], |_| {})
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
        .map_err(|error| format!("view exposure: {error:?}"))?;
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

/// A still scene under the sky, hosted by the app shell. See the [module](self) example.
pub struct SkyGame {
    name: &'static str,
    controls: Controls,
    sky: Sky,
    sent: Option<Sky>,
    refused: Option<String>,
    update: Update,
    contribution: Option<Box<dyn SceneContribution>>,
    status: Option<StatusProbe>,
    shown_status: Option<ContentStatus>,
}

impl SkyGame {
    /// A scene with `controls` in the panel and `camera`, the Sun 30° high in the south-east.
    /// `update` runs before every frame (and after every control edit) to map the controls
    /// onto the [`Sky`]; a value the renderer cannot draw is reported and the previous sky kept.
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
            },
            sent: None,
            refused: None,
            update: Box::new(update),
            contribution: None,
            status: None,
            shown_status: None,
        }
    }

    /// Draw an engine contribution (such as cooked meshes) lit by the sky. `status` reports
    /// its progress: the window logs each change, and a capture waits until it is drawn.
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

    /// Prepare the still scene on `capabilities`' device, with the contribution attached.
    fn build_host(
        &mut self,
        capabilities: &DeviceCapabilities,
        queue: &wgpu::Queue,
        config: FrameConfig,
    ) -> Result<FrameHost<StillWorld, SkyScene>, String> {
        let _ = self.update();
        self.sky.check()?;
        // The renderer needs one instance row; an inactive row (generation zero) draws nothing.
        let mut scene = SyntheticScene::grid(1).map_err(|error| format!("sky scene: {error:?}"))?;
        scene.instances[0].ids[3] = 0;
        scene.sun_shadows = true;
        let mut host = pollster::block_on(FrameHost::new(
            capabilities,
            queue,
            config,
            StillWorld,
            SkyScene { sky: self.sky },
            scene,
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
        self.update()
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

/// Open `game` in a window until it is closed (`config.frames_variable` bounds a scripted
/// run), or, when `THRUM_CAPTURE_PNG` names a file, render one frame there offscreen after
/// applying `THRUM_PARAMS` and waiting for the contribution to draw. Failures are explained
/// on standard error with a nonzero exit.
pub fn run(config: Config, game: impl FnOnce() -> Result<SkyGame, String>) {
    let named = move || {
        game().map(|mut game| {
            game.name = config.name;
            game
        })
    };
    match std::env::var_os(CAPTURE_VARIABLE) {
        Some(path) => capture::run(config, named, std::path::Path::new(&path)),
        None => crate::run_until_closed(config, named),
    }
}

#[cfg(test)]
mod tests;
