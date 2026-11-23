//! The V1 game hosted by the shared macOS window shell: its runtime preparation, demo
//! controls, movement keys and solar-time readback.

use std::time::Instant;
use thrum_render::capabilities::HardwareRequest;
use thrum_render::synthetic::CullMode;

use thrum_app::{Config, Game, GameInput, Gpu, KeyCode, KeyResponse, Prepared, WorldLink};
use thrum_core::sim::{WorldLaneFault, WorldLifecycle, WorldLifecycleReply};

use thrum_render::runtime::FrameConfig;

use crate::{
    GameHost, WorldControls,
    controls::GameControls,
    host::{self, GameScene, GameWorld, Runtime},
    hosted::{HostedGame, RuntimeSetup},
};

mod capture;
pub use capture::scene::{RenderScaleTrace, SceneCaptureRequest};

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
    /// Keep every image while driving the realtime render-scale trace.
    ScaleTraceImages,
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

/// Capture the game to disk with a frozen or fixed-step world on the engine's offscreen driver:
/// lighting stills, parameter routes, consecutive ticks, a motion sequence or a moving route,
/// as `request` selects. `output` must be a new directory.
/// # Errors
/// Returns GPU, control, readback or output failure.
pub fn run_scene_capture(
    output: &std::path::Path,
    request: &SceneCaptureRequest,
    hardware: &[HardwareRequest],
) -> Result<(), String> {
    pollster::block_on(capture::scene::run(output, request, hardware))
}

/// Run the bounded V1 scene in a macOS window and print its frames report.
/// `THRUM_V1_FRAMES` bounds the run; `THRUM_PARAMS` names a parameter file applied at
/// start and on `L`. Failures are reported on standard error with a nonzero exit.
pub fn run_window(
    hardware: Vec<HardwareRequest>,
    capture: Option<thrum_app::WindowExtentCapture>,
    transition: Option<thrum_app::WindowTransitionProof>,
    qualification: Option<thrum_app::WindowQualification>,
) {
    // The window title names the content and the requested culling mode;
    // the profiler title adds the mode the renderer runs. Invalid selections
    // are reported when the host is prepared.
    let setup = crate::selected_setup();
    let sprite_card = matches!(
        &setup,
        Ok(crate::GameSetup {
            sprite_scale: Some(_),
            sprite_probe: None,
            sprite_rows: None,
            ..
        })
    );
    let (window_title, title) = match setup {
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
        Ok(HostedGame(WindowGame {
            demo: GameControls::new().map_err(|error| error.to_string())?,
            world: None,
            held_keys: 0,
            capture_route: None,
            capture_eye: None,
            capture_cooked: None,
            cloud_capture: false,
            scene: None,
            hardware,
        }))
    };
    if let Some(capture) = capture {
        if sprite_card {
            write_card_texels(&capture.output);
        }
        thrum_app::run_with_extent_capture(config, capture, game);
    } else if let Some(proof) = transition {
        if sprite_card && proof.tap {
            write_card_texels(&proof.output);
        }
        thrum_app::run_with_transition_proof(config, proof, game);
    } else if let Some(qualification) = qualification {
        thrum_app::run_with_qualification(config, qualification, game);
    } else {
        thrum_app::run(config, game);
    }
}

fn write_card_texels(output: &std::path::Path) {
    let _source_receipt = std::fs::write(
        output.join("card-source.rgba"),
        crate::sprite_card::texels(),
    );
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
    scene: Option<SceneCapture>,
    hardware: Vec<HardwareRequest>,
}

/// A scene capture's own preparation: its render extent, which may differ from the engine
/// texture standing in for the surface, and an inline world it steps itself.
struct SceneCapture {
    setup: crate::GameSetup,
    render_extent: [u32; 2],
    #[cfg(feature = "draw-probe")]
    omit: Option<usize>,
    content: Option<std::sync::Arc<std::sync::Mutex<thrum_asset_gpu::CookedStats>>>,
}

struct CaptureCooked {
    folder: std::path::PathBuf,
    path: thrum_asset_gpu::CookedDrawPath,
    preview: Option<ContentPreviewCapture>,
    scale_props: bool,
}

type ContentPreviewCapture =
    std::sync::Arc<std::sync::Mutex<Option<crate::content_preview::PreviewRecord>>>;

impl CaptureCooked {
    fn new(folder: &std::path::Path, capture: RuntimeCaptureOutput) -> Self {
        Self {
            folder: folder.to_owned(),
            path: capture.cooked_path(),
            scale_props: capture == RuntimeCaptureOutput::ScaleTraceImages,
            preview: None,
        }
    }
}

/// Construct the ordinary game for the engine's headless presenter adapter.
/// # Errors
/// Returns a demo-control declaration failure.
pub fn capture_game() -> Result<impl Game + GameInput<KeyCode>, String> {
    capture_game_with_route_limit(false, None)
}

fn capture_game_with_route_limit(
    cloud_capture: bool,
    capture_cooked: Option<CaptureCooked>,
) -> Result<HostedGame<WindowGame>, String> {
    Ok(HostedGame(WindowGame {
        demo: GameControls::new().map_err(|error| error.to_string())?,
        world: None,
        held_keys: 0,
        scene: None,
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
    }))
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

impl RuntimeSetup for WindowGame {
    fn demo(&self) -> &GameControls {
        &self.demo
    }

    fn demo_mut(&mut self) -> &mut GameControls {
        &mut self.demo
    }

    fn hardware_requests(&self) -> &[HardwareRequest] {
        &self.hardware
    }

    async fn prepare_runtime(
        &mut self,
        gpu: Gpu<'_>,
        world_advanced: Box<dyn Fn() + Send>,
    ) -> Result<Prepared<GameWorld, GameScene>, String> {
        if let Some(scene) = &mut self.scene {
            let mut host = scene
                .setup
                .prepare(
                    gpu.device,
                    gpu.queue,
                    gpu.capabilities,
                    FrameConfig {
                        render_extent: scene.render_extent,
                        surface: Some(gpu.surface),
                    },
                )
                .await?;
            #[cfg(feature = "draw-probe")]
            {
                scene.content = crate::content::attach_cooked_scene_with_probe(
                    &mut host,
                    scene.omit.map(thrum_asset_gpu::draw_probe::Probe::Omit),
                )?;
            }
            #[cfg(not(feature = "draw-probe"))]
            {
                scene.content = crate::attach_cooked_scene(&mut host)?;
            }
            // The capture steps this inline world on its own clock.
            return Ok(Prepared {
                host,
                origin: Some(Instant::now()),
            });
        }
        let render_extent = gpu.surface.extent;
        let mut setup = crate::selected_setup()?;
        setup.route_scale_props = self
            .capture_cooked
            .as_ref()
            .is_some_and(|capture| capture.scale_props);
        let exit = setup.content == crate::GameContent::Exit;
        let config = FrameConfig {
            render_extent,
            surface: Some(gpu.surface),
        };
        let mut host = if let Some(shapes) = crate::content::placed_sky()? {
            setup
                .prepare_placed(gpu.device, gpu.queue, gpu.capabilities, config, shapes)
                .await
                .map_err(|error| error.to_string())?
        } else {
            setup
                .prepare(gpu.device, gpu.queue, gpu.capabilities, config)
                .await?
        };
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
            origin: Some(origin),
        })
    }

    fn world(&self) -> Option<&dyn WorldLink> {
        self.world.as_ref().map(|world| -> &dyn WorldLink { world })
    }

    fn readback(host: &Runtime) -> f32 {
        host::time_of_day(host)
    }
}

impl GameInput<KeyCode> for WindowGame {
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
