//! The V1 game hosted by the shared macOS window shell: its runtime preparation, demo
//! controls, movement keys and solar-time readback.

use std::time::Instant;
use thrum_render::capabilities::HardwareRequest;
use thrum_render::synthetic::CullMode;

use thrum_app::{Config, Edits, Game, Gpu, KeyCode, KeyResponse, Prepared, WorldLink};
use thrum_controls::Controls;
use thrum_core::sim::{WorldLaneFault, WorldLifecycle, WorldLifecycleReply};

use thrum_render::runtime::FrameConfig;

use crate::{
    GameHost, WorldControls,
    controls::{ControlEdits, GameControls},
    host::{self, GameScene, GameWorld, Runtime},
};

mod capture;

/// Process-edge selection for the labelled native recovery capture fixture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeRecovery {
    /// Ordinary capture, with no failure injection.
    Disabled,
    /// Uninterrupted control with matched reset checkpoints.
    Control,
    /// Presenter replacement, command error, device loss and rejection.
    Faults,
}

/// Environment setting parsed by the binary before starting the capture adapter.
pub const NATIVE_RECOVERY_VARIABLE: &str = "THRUM_CAPTURE_NATIVE_RECOVERY";

/// Process-edge selection for the capture observer comparison.
pub const CAPTURE_OBSERVER_VARIABLE: &str = "THRUM_CAPTURE_OBSERVER_ONLY";

/// Whether a runtime capture records pixels or observes submission cadence alone.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeCaptureOutput {
    /// Keep every submitted image.
    Images,
    /// Keep every submitted image with the opted-in cooked mesh stages.
    #[cfg(feature = "draw-probe")]
    MeshStageImages,
    /// Run the same scenario without the image readback and writer.
    ObserverOnly,
}

impl RuntimeCaptureOutput {
    fn cooked_path(self) -> thrum_asset_gpu::CookedDrawPath {
        match self {
            #[cfg(feature = "draw-probe")]
            Self::MeshStageImages => thrum_asset_gpu::CookedDrawPath::MeshStages,
            _ => thrum_asset_gpu::CookedDrawPath::Auto,
        }
    }
}

/// Run a declared unattended capture scenario through the ordinary game.
/// # Errors
/// Returns GPU, control, readback or output failure.
pub fn run_runtime_capture(
    output: &std::path::Path,
    scenario: &str,
    wait: bool,
    hold_full: std::time::Duration,
    recovery: NativeRecovery,
    capture_output: RuntimeCaptureOutput,
    hardware: Vec<HardwareRequest>,
) -> Result<(), String> {
    capture::run(
        output,
        scenario,
        wait,
        hold_full,
        recovery,
        capture_output,
        hardware,
    )
}

const KEY_A: u8 = 1;
const KEY_D: u8 = 2;
const KEY_W: u8 = 4;
const KEY_S: u8 = 8;
const KEY_LEFT: u8 = 16;
const KEY_RIGHT: u8 = 32;
const KEY_UP: u8 = 64;
const KEY_DOWN: u8 = 128;

/// Run the bounded V1 scene in a macOS window and print its frames report.
/// `THRUM_V1_FRAMES` bounds the run; `THRUM_PARAMS` names a parameter file applied at
/// start and on `L`. Failures are reported on standard error with a nonzero exit.
pub fn run_window(
    hardware: Vec<HardwareRequest>,
    capture: Option<thrum_app::WindowExtentCapture>,
    transition: Option<thrum_app::WindowTransitionProof>,
) {
    // The window title names the content and the requested culling mode;
    // the profiler title adds the mode the renderer runs. Invalid selections
    // are reported when the host is prepared.
    let (window_title, title) = match crate::selected_setup() {
        Ok(crate::GameSetup {
            sprite_scale: Some(_),
            ..
        }) => ("Thrum V1 — sprites", "Thrum V1 sprites"),
        Ok(crate::GameSetup {
            content: crate::GameContent::Field,
            camera_culling: CullMode::Frustum,
            ..
        }) => ("Thrum V1 — field / Frustum culling", "Thrum V1 field"),
        Ok(crate::GameSetup {
            content: crate::GameContent::Field,
            camera_culling: CullMode::Occlusion,
            ..
        }) => ("Thrum V1 — field / Occlusion culling", "Thrum V1 field"),
        Ok(crate::GameSetup {
            content: crate::GameContent::Field,
            ..
        }) => ("Thrum V1 — field / Direct", "Thrum V1 field"),
        Ok(crate::GameSetup {
            camera_culling: CullMode::Frustum,
            ..
        }) => ("Thrum V1 — 10k world / Frustum culling", "Thrum V1"),
        Ok(crate::GameSetup {
            camera_culling: CullMode::Occlusion,
            ..
        }) => ("Thrum V1 — 10k world / Occlusion culling", "Thrum V1"),
        _ => ("Thrum V1 — 10k world / clustered forward+", "Thrum V1"),
    };
    let config = Config {
        name: "v1-game",
        frames_variable: "THRUM_V1_FRAMES",
        window_title,
        title,
        render_thread: "thrum-v1-render",
        panel_title: "Thrum demo",
        window_size: [1920, 1080],
    };
    let game = || {
        Ok(WindowGame {
            demo: GameControls::new().map_err(|error| error.to_string())?,
            world: None,
            held_keys: 0,
            capture_route: None,
            capture_eye: None,
            capture_cooked: None,
            cloud_capture: false,
            hardware,
        })
    };
    if let Some(capture) = capture {
        thrum_app::run_with_extent_capture(config, capture, game);
    } else if let Some(proof) = transition {
        thrum_app::run_with_transition_proof(config, proof, game);
    } else {
        thrum_app::run(config, game);
    }
}

struct WindowGame {
    /// Solar time and camera controls shared by keys and the panel.
    demo: GameControls,
    world: Option<WorldControls>,
    held_keys: u8,
    capture_route: Option<String>,
    capture_eye: Option<[f32; 3]>,
    capture_cooked: Option<CaptureCooked>,
    cloud_capture: bool,
    hardware: Vec<HardwareRequest>,
}

struct CaptureCooked {
    folder: std::path::PathBuf,
    path: thrum_asset_gpu::CookedDrawPath,
    preview: Option<ContentPreviewCapture>,
}

type ContentPreviewCapture =
    std::sync::Arc<std::sync::Mutex<Option<crate::content_preview::PreviewRecord>>>;

impl CaptureCooked {
    fn new(folder: &std::path::Path, capture: RuntimeCaptureOutput) -> Self {
        Self {
            folder: folder.to_owned(),
            path: capture.cooked_path(),
            preview: None,
        }
    }
}

/// Construct the ordinary game for the engine's headless presenter adapter.
/// # Errors
/// Returns a demo-control declaration failure.
pub fn capture_game() -> Result<impl Game, String> {
    capture_game_with_route_limit(false, None)
}

fn capture_game_with_route_limit(
    cloud_capture: bool,
    capture_cooked: Option<CaptureCooked>,
) -> Result<impl Game, String> {
    Ok(WindowGame {
        demo: GameControls::new().map_err(|error| error.to_string())?,
        world: None,
        held_keys: 0,
        cloud_capture,
        capture_cooked,
        capture_eye: crate::content::launch_value("THRUM_CAPTURE_FIXED_EYE")
            .ok()
            .map(|value| serde_json::from_str(&value))
            .transpose()
            .map_err(|error| format!("fixed capture eye: {error}"))?,
        hardware: Vec::new(),
        capture_route: std::env::var_os("THRUM_CAPTURE_RUNTIME_ROUTE")
            .map(std::fs::read_to_string)
            .transpose()
            .map_err(|error| error.to_string())?,
    })
}

impl WindowGame {
    fn axes(&self) -> [f32; 3] {
        let horizontal = axis(
            self.held_keys & (KEY_A | KEY_LEFT) != 0,
            self.held_keys & (KEY_D | KEY_RIGHT) != 0,
        );
        let vertical = axis(
            self.held_keys & (KEY_S | KEY_DOWN) != 0,
            self.held_keys & (KEY_W | KEY_UP) != 0,
        );
        [horizontal, vertical, 0.0]
    }

    fn send_axes(&self) -> Result<(), String> {
        if let Some(world) = &self.world {
            world.set_controls(self.axes())?;
        }
        Ok(())
    }
}

impl Game for WindowGame {
    type World = GameWorld;
    type Scene = GameScene;
    type Edits = ControlEdits;
    /// Committed solar time after a render job.
    type Readback = f32;

    fn hardware_requests(&self) -> &[HardwareRequest] {
        &self.hardware
    }

    fn prepare(
        &mut self,
        gpu: Gpu<'_>,
        world_advanced: Box<dyn Fn() + Send>,
    ) -> Result<Prepared<GameWorld, GameScene>, String> {
        let render_extent = gpu.surface.extent;
        let setup = crate::selected_setup()?;
        let exit = setup.content == crate::GameContent::Exit;
        let mut host = pollster::block_on(setup.prepare(
            gpu.device,
            gpu.queue,
            gpu.capabilities,
            FrameConfig {
                render_extent,
                surface: Some(gpu.surface),
            },
        ))?;
        crate::apply_cloud_quality(&mut host).map_err(|e| e.to_string())?;
        let preview = self
            .capture_cooked
            .as_ref()
            .and_then(|cooked| cooked.preview.as_ref());
        let content = if let Some(preview) = preview {
            *preview.lock().map_err(|error| error.to_string())? =
                Some(crate::content::attach_content_preview(&mut host)?);
            None
        } else if let Some(cooked) = &self.capture_cooked {
            if exit {
                let selection = host.culling_selection();
                let report = serde_json::json!({
                    "requested": format!("{:?}", selection.requested),
                    "resolved": format!("{:?}", selection.resolved),
                    "reason": format!("{:?}", selection.reason),
                });
                std::fs::write(cooked.folder.join("culling.json"), report.to_string())
                    .map_err(|error| error.to_string())?;
            }
            crate::content::attach_cooked_scene_with_path(&mut host, cooked.path)?
        } else {
            crate::attach_cooked_scene(&mut host)?
        };
        if let Some(eye) = self.capture_eye {
            if self.capture_route.is_some() {
                return Err("a fixed capture eye cannot override an active route".to_owned());
            }
            let camera = host
                .scene_camera()
                .ok_or("fixed capture eye needs a cooked scene")?;
            host.set_scene_camera(camera.at(eye)?)?;
        }
        self.demo.observe_camera(host.camera_orientation());
        let offset = if let Some(route) = &self.capture_route {
            capture::prepare_route(
                &mut host,
                content,
                route,
                render_extent,
                &gpu,
                self.cloud_capture,
                self.capture_cooked
                    .as_ref()
                    .map(|cooked| cooked.folder.as_path()),
            )?
        } else {
            std::time::Duration::ZERO
        };
        let origin = Instant::now()
            .checked_sub(offset)
            .ok_or("capture route origin underflow")?;
        let (host, controls) = host.into_threaded(origin, world_advanced)?;
        self.world = Some(controls);
        Ok(Prepared {
            host,
            origin,
        })
    }

    fn world(&self) -> Option<&dyn WorldLink> {
        self.world.as_ref().map(|world| -> &dyn WorldLink { world })
    }

    fn controls(&self) -> &Controls {
        self.demo.model()
    }

    fn controls_mut(&mut self) -> &mut Controls {
        self.demo.model_mut()
    }

    fn take_edits(&mut self) -> (ControlEdits, Option<bool>) {
        let edits = self.demo.take_edits();
        (
            ControlEdits {
                paused: None,
                ..edits
            },
            edits.paused,
        )
    }

    fn replay(&self) -> ControlEdits {
        self.demo.replay()
    }

    fn apply(host: &mut Runtime, edits: ControlEdits) -> Result<(), String> {
        if let Some(value) = edits.time_of_day {
            host::set_time_of_day(host, value)?;
        }
        if let Some(value) = edits.camera_pitch {
            host::set_camera_pitch(host, value)?;
        }
        if let Some(value) = edits.camera_yaw {
            host::set_camera_yaw(host, value)?;
        }
        if let Some(overlay) = edits.display_overlay() {
            host::set_display_overlay(host, overlay);
        }
        Ok(())
    }

    fn readback(host: &Runtime) -> f32 {
        host::time_of_day(host)
    }

    fn observe(&mut self, time_of_day: f32) {
        self.demo.observe_time(time_of_day);
    }

    fn observe_paused(&mut self, paused: bool) {
        self.demo.observe_paused(paused);
    }

    fn key(&mut self, code: KeyCode, pressed: bool) -> Result<KeyResponse, String> {
        const FIVE_DEGREES: f32 = 0.087_266_46;
        if pressed {
            match code {
                KeyCode::KeyQ => self.demo.turn(-FIVE_DEGREES),
                KeyCode::KeyE => self.demo.turn(FIVE_DEGREES),
                KeyCode::KeyR => self.demo.tilt(FIVE_DEGREES),
                KeyCode::KeyF => self.demo.tilt(-FIVE_DEGREES),
                KeyCode::BracketLeft => self.demo.advance_hours(-1.0),
                KeyCode::BracketRight => self.demo.advance_hours(1.0),
                _ => {}
            }
            if matches!(
                code,
                KeyCode::KeyQ
                    | KeyCode::KeyE
                    | KeyCode::KeyR
                    | KeyCode::KeyF
                    | KeyCode::BracketLeft
                    | KeyCode::BracketRight
            ) {
                return Ok(KeyResponse::Edited);
            }
        }
        let mask = match code {
            KeyCode::KeyA => KEY_A,
            KeyCode::KeyD => KEY_D,
            KeyCode::KeyW => KEY_W,
            KeyCode::KeyS => KEY_S,
            KeyCode::ArrowLeft => KEY_LEFT,
            KeyCode::ArrowRight => KEY_RIGHT,
            KeyCode::ArrowUp => KEY_UP,
            KeyCode::ArrowDown => KEY_DOWN,
            _ => return Ok(KeyResponse::Ignored),
        };
        if (self.held_keys & mask != 0) == pressed {
            return Ok(KeyResponse::Ignored);
        }
        if pressed {
            self.held_keys |= mask;
        } else {
            self.held_keys &= !mask;
        }
        self.send_axes()?;
        Ok(KeyResponse::Input)
    }

    fn focus_lost(&mut self) -> Result<(), String> {
        self.held_keys = 0;
        self.send_axes()
    }
}

impl Edits for ControlEdits {
    fn merge(self, newer: Self) -> Self {
        Self {
            time_of_day: newer.time_of_day.or(self.time_of_day),
            camera_pitch: newer.camera_pitch.or(self.camera_pitch),
            camera_yaw: newer.camera_yaw.or(self.camera_yaw),
            paused: newer.paused.or(self.paused),
            exposure_zones: newer.exposure_zones.or(self.exposure_zones),
            motion_view: newer.motion_view.or(self.motion_view),
        }
    }

    fn is_empty(&self) -> bool {
        ControlEdits::is_empty(self)
    }
}

impl WorldLink for WorldControls {
    fn request_lifecycle(&self, request: WorldLifecycle) -> Result<(), WorldLaneFault> {
        WorldControls::request_lifecycle(self, request)
    }

    fn poll_lifecycle_reply(&self) -> Result<Option<WorldLifecycleReply>, WorldLaneFault> {
        WorldControls::poll_lifecycle_reply(self)
    }

    fn check_health(&self) -> Result<u64, WorldLaneFault> {
        WorldControls::check_health(self)
    }
}

fn axis(negative: bool, positive: bool) -> f32 {
    match (negative, positive) {
        (true, false) => -1.0,
        (false, true) => 1.0,
        _ => 0.0,
    }
}
