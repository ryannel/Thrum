//! The V1 game hosted by the shared macOS window shell: its runtime preparation, demo
//! controls, movement keys and solar-time readback.

use std::time::Instant;
use thrum_render::capabilities::HardwareRequest;
use thrum_render::synthetic::CullMode;

#[cfg(feature = "proving")]
use thrum_app::Game;
use thrum_app::{Config, Gpu, KeyCode, KeyResponse, Prepared, WorldLink};
use thrum_core::sim::{WorldLaneFault, WorldLifecycle, WorldLifecycleReply};

use thrum_render::runtime::FrameConfig;

use crate::{
    GameHost, WorldControls,
    controls::GameControls,
    host::{self, GameScene, GameWorld, Runtime},
    hosted::{HostedGame, RuntimeSetup},
};

#[cfg(feature = "proving")]
mod capture;
#[cfg(all(feature = "proving", feature = "creation-counters"))]
pub use capture::scene::CreationProof;
#[cfg(feature = "proving")]
pub use capture::scene::{RenderScaleTrace, SceneCaptureRequest};

/// Process-edge selection for the labelled native recovery capture fixture.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg(feature = "proving")]
pub enum NativeRecovery {
    /// Ordinary capture, with no failure injection.
    Disabled,
    /// Uninterrupted control with matched reset checkpoints.
    Control,
    /// Presenter replacement, command error, device loss and rejection.
    Faults,
}

/// Environment setting parsed by the binary before starting the capture adapter.
#[cfg(feature = "proving")]
pub const NATIVE_RECOVERY_VARIABLE: &str = "THRUM_CAPTURE_NATIVE_RECOVERY";

/// Process-edge selection for the capture observer comparison.
#[cfg(feature = "proving")]
pub const CAPTURE_OBSERVER_VARIABLE: &str = "THRUM_CAPTURE_OBSERVER_ONLY";

/// Whether a runtime capture records pixels or observes submission cadence alone.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg(feature = "proving")]
pub enum RuntimeCaptureOutput {
    /// Keep every submitted image.
    Images,
    /// Keep every image while driving the realtime render-scale trace.
    ScaleTraceImages,
    /// Run the governor under real-time load along the capture route.
    Governor {
        /// Record submitted images as well as frame and quality records.
        images: bool,
    },
    /// Keep every submitted image with the opted-in cooked mesh stages.
    #[cfg(feature = "draw-probe")]
    MeshStageImages,
    /// Run the same scenario without the image readback and writer.
    ObserverOnly,
}

#[cfg(feature = "proving")]
impl RuntimeCaptureOutput {
    fn cooked_path(self) -> thrum_asset_gpu::CookedDrawPath {
        match self {
            #[cfg(feature = "draw-probe")]
            Self::MeshStageImages => thrum_asset_gpu::CookedDrawPath::MeshStages,
            _ => thrum_asset_gpu::CookedDrawPath::Auto,
        }
    }
}

/// Optional held status values and simulated insets for an offscreen capture.
#[cfg(feature = "proving")]
#[derive(Clone, Copy, Debug, Default)]
pub struct StatusCapture {
    /// Tick supplying values, when a fixed inspection calls for a particular bar level.
    pub tick: Option<u64>,
    /// Left, top, right and bottom in interface points.
    pub insets: [f64; 4],
    /// Density the offscreen output will be presented at.
    pub density: Option<f64>,
}

/// Run a declared unattended capture scenario through the ordinary game.
/// # Errors
/// Returns GPU, control, readback or output failure.
#[cfg(feature = "proving")]
pub fn run_runtime_capture(
    output: &std::path::Path,
    scenario: &str,
    wait: bool,
    hold_full: std::time::Duration,
    recovery: NativeRecovery,
    capture_output: RuntimeCaptureOutput,
    status: StatusCapture,
    menu_state: Option<String>,
    hardware: Vec<HardwareRequest>,
) -> Result<(), String> {
    capture::run(
        output,
        scenario,
        wait,
        hold_full,
        recovery,
        capture_output,
        status,
        menu_state,
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
#[cfg(feature = "proving")]
pub fn run_scene_capture(
    output: &std::path::Path,
    request: &SceneCaptureRequest,
    hardware: &[HardwareRequest],
) -> Result<(), String> {
    pollster::block_on(capture::scene::run(output, request, hardware))
}

/// Dynamic quality selection and optional authored route for a Mac window run.
#[derive(Default)]
#[cfg_attr(not(feature = "proving"), derive(Clone, Copy))]
pub struct WindowGovernor {
    /// Enable the governor for a prepared TAAU scene.
    pub enabled: bool,
    /// Typed override for the display-link request.
    pub frame_rate: Option<thrum_app::MacFrameRate>,
    /// Typed launch setting for the Mac output grid.
    pub output_scale: Option<thrum_render::synthetic::RenderScale>,
    /// A validated route is prepared from this authored route text before the window starts.
    #[cfg(feature = "proving")]
    pub route: Option<String>,
    /// A continuous solar-time sweep for a fullscreen proof cue.
    #[cfg(feature = "proving")]
    pub solar_sweep: Option<WindowSolarSweep>,
    /// Jump to the given solar hour at the chart proof's world tick.
    #[cfg(feature = "proving")]
    pub chart_jump: Option<(u64, f32)>,
    /// Hold the exit route at its starting pose until the saved controls resume it.
    #[cfg(feature = "proving")]
    pub exit_route_starts_paused: bool,
    /// Opt-in observation of the cooked generations drawn by the ordinary exit window.
    #[cfg(feature = "proving")]
    pub exit_content_report: Option<std::path::PathBuf>,
}

/// Options passed from the process edge to the interface scene.
#[cfg(feature = "proving")]
#[derive(Default)]
pub struct WindowMenu {
    /// Optional join ticket supplied as one complete argument.
    pub ticket: Option<String>,
    /// Scripted qualification input and report paths.
    pub script: Option<thrum_app::InterfaceScript>,
}

/// Run the V1 scene in a macOS window and print its frames report when it closes.
/// `THRUM_V1_FRAMES` bounds the run; `THRUM_PARAMS` names a parameter file applied at
/// start and on `L`. Failures are reported on standard error with a nonzero exit.
/// Set `THRUM_REPORT=1` to collect GPU timestamps for the whole run.
pub fn run_window(
    hardware: Vec<HardwareRequest>,
    #[cfg(feature = "proving")] capture: Option<thrum_app::WindowExtentCapture>,
    #[cfg(feature = "proving")] transition: Option<thrum_app::WindowTransitionProof>,
    #[cfg(feature = "proving")] qualification: Option<thrum_app::WindowQualification>,
    #[cfg(feature = "proving")] display: Option<thrum_app::WindowDisplayOptions>,
    #[cfg(feature = "proving")] menu: WindowMenu,
    governor: WindowGovernor,
    metal_preparation: Option<thrum_render::capabilities::metallib::Preparation>,
) {
    // The window title names the content and the requested culling mode;
    // the profiler title adds the mode the renderer runs. Invalid selections
    // are reported when the host is prepared.
    let setup = crate::selected_setup();
    #[cfg(feature = "proving")]
    let sprite_card = matches!(
        &setup,
        Ok(crate::GameSetup {
            sprite_scale: Some(_),
            sprite_probe: None,
            sprite_rows: None,
            ..
        })
    );
    #[cfg(feature = "proving")]
    let interface_text = setup.as_ref().ok().and_then(|setup| setup.interface_text);
    let (window_title, title) = match setup {
        #[cfg(feature = "proving")]
        Ok(crate::GameSetup {
            sprite_scale: Some(_),
            ..
        }) => ("Thrum V1 — sprites", "Thrum V1 sprites"),
        #[cfg(feature = "proving")]
        Ok(crate::GameSetup {
            content: crate::GameContent::Field,
            camera_culling: CullMode::Frustum,
            ..
        }) => ("Thrum V1 — field / Frustum culling", "Thrum V1 field"),
        #[cfg(feature = "proving")]
        Ok(crate::GameSetup {
            content: crate::GameContent::Field,
            camera_culling: CullMode::Occlusion,
            ..
        }) => ("Thrum V1 — field / Occlusion culling", "Thrum V1 field"),
        #[cfg(feature = "proving")]
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
    #[cfg(feature = "proving")]
    let chart_extent = governor
        .chart_jump
        .and_then(|_| governor.route.as_deref())
        .and_then(|route| crate::CameraRoute::declared(route).ok())
        .map(|(extent, _)| extent);
    let config = Config {
        name: "v1-game",
        frames_variable: "THRUM_V1_FRAMES",
        window_title,
        title,
        render_thread: "thrum-v1-render",
        panel_title: "Thrum demo",
        window_position: None,
        #[cfg(feature = "proving")]
        window_always_on_top: capture.as_ref().is_some_and(|proof| proof.status)
            || transition
                .as_ref()
                .is_some_and(|proof| proof.arm == thrum_app::TransitionArm::Status)
            || (interface_text == Some(crate::InterfaceText::Menu) && menu.script.is_some()),
        #[cfg(not(feature = "proving"))]
        window_always_on_top: false,
        frame_budget: None,
        mac_frame_rate: governor.frame_rate,
        mac_output_scale: governor.output_scale,
        metal_preparation,
        #[cfg(feature = "proving")]
        window_size: if let Some(extent) = chart_extent {
            extent
        } else if transition.is_none()
            && display
                .as_ref()
                .is_some_and(|options| options.output.is_some())
        {
            [640, 360]
        } else {
            [1920, 1080]
        },
        #[cfg(not(feature = "proving"))]
        window_size: [1920, 1080],
    };
    #[cfg(feature = "proving")]
    let route_start = if transition
        .as_ref()
        .is_some_and(|proof| proof.arm == thrum_app::TransitionArm::Fullscreen)
        || (matches!(
            setup,
            Ok(crate::GameSetup {
                content: crate::GameContent::Exit,
                ..
            })
        ) && governor.exit_route_starts_paused)
    {
        RouteStart::Paused
    } else {
        RouteStart::Immediate
    };
    #[cfg(feature = "proving")]
    let output_record = transition
        .as_ref()
        .filter(|proof| {
            matches!(
                proof.arm,
                thrum_app::TransitionArm::OutputScaleFullscreen
                    | thrum_app::TransitionArm::OutputScaleMode
            )
        })
        .map(|proof| proof.output.join("game-output.jsonl"));
    let game = || -> Result<_, thrum_app::GameError> {
        Ok(HostedGame(WindowGame {
            demo: setup
                .as_ref()
                .map_or_else(|_| GameControls::new(), |setup| setup.game_controls())
                .map_err(|error| thrum_app::GameError::Rejected(error.to_string()))?,
            world: None,
            held_keys: 0,
            replay_controls: true,
            #[cfg(feature = "proving")]
            interface_text,
            #[cfg(feature = "proving")]
            menu: (interface_text == Some(crate::InterfaceText::Menu)).then(|| window_menu(menu)),
            #[cfg(feature = "proving")]
            capture_route: governor.route.map(|text| RouteInput {
                text,
                start: route_start,
                solar_sweep: governor.solar_sweep,
                chart_jump: governor.chart_jump,
            }),
            governor_enabled: governor.enabled,
            #[cfg(feature = "proving")]
            capture_eye: None,
            #[cfg(feature = "proving")]
            capture_cooked: None,
            #[cfg(feature = "proving")]
            cloud_capture: false,
            #[cfg(feature = "proving")]
            scene: None,
            #[cfg(feature = "proving")]
            exit_content: None,
            #[cfg(feature = "proving")]
            exit_content_output: governor.exit_content_report,
            #[cfg(feature = "proving")]
            output_record: output_record
                .map(std::fs::File::create)
                .transpose()
                .map_err(|error| thrum_app::GameError::Rejected(error.to_string()))?,
            hardware,
        }))
    };
    #[cfg(feature = "proving")]
    if let Some(capture) = capture {
        if sprite_card {
            write_card_texels(&capture.output);
        }
        thrum_app::run_with_extent_capture(config, capture, game);
    } else if let Some(proof) = transition {
        if sprite_card && proof.tap {
            write_card_texels(&proof.output);
        }
        thrum_app::run_with_transition_proof(config, proof, display, game);
    } else if let Some(qualification) = qualification {
        thrum_app::run_with_qualification_and_display(config, qualification, display, game);
    } else if let Some(display) = display {
        thrum_app::run_with_display(config, display, game);
    } else {
        thrum_app::run(config, game);
    }
    #[cfg(not(feature = "proving"))]
    thrum_app::run(config, game);
}

#[cfg(feature = "proving")]
fn write_card_texels(output: &std::path::Path) {
    let _source_receipt = std::fs::write(
        output.join("card-source.rgba"),
        crate::sprite_card::texels(),
    );
}

#[cfg(feature = "proving")]
fn window_menu(config: WindowMenu) -> crate::interface_menu::Menu {
    let mut menu = crate::interface_menu::Menu::new(config.script.is_some());
    menu.script = config.script;
    if let Some(ticket) = config.ticket {
        menu.launch(
            ticket,
            thrum_ui::TextSource::Launch,
            std::time::Duration::ZERO,
        );
    }
    menu
}

struct WindowGame {
    /// Solar time and camera controls shared by keys and the panel.
    demo: GameControls,
    world: Option<WorldControls>,
    held_keys: u8,
    replay_controls: bool,
    #[cfg(feature = "proving")]
    interface_text: Option<crate::InterfaceText>,
    #[cfg(feature = "proving")]
    menu: Option<crate::interface_menu::Menu>,
    #[cfg(feature = "proving")]
    capture_route: Option<RouteInput>,
    governor_enabled: bool,
    #[cfg(feature = "proving")]
    capture_eye: Option<[f32; 3]>,
    #[cfg(feature = "proving")]
    capture_cooked: Option<CaptureCooked>,
    #[cfg(feature = "proving")]
    cloud_capture: bool,
    #[cfg(feature = "proving")]
    scene: Option<SceneCapture>,
    #[cfg(feature = "proving")]
    exit_content: Option<ExitContentReport>,
    #[cfg(feature = "proving")]
    exit_content_output: Option<std::path::PathBuf>,
    #[cfg(feature = "proving")]
    output_record: Option<std::fs::File>,
    hardware: Vec<HardwareRequest>,
}

#[cfg(feature = "proving")]
enum RouteStart {
    Immediate,
    Paused,
}

/// Inclusive tick endpoints and starting/ending hours for the fullscreen solar cue.
#[cfg(feature = "proving")]
pub type WindowSolarSweep = ([u64; 2], [f32; 2]);

#[cfg(feature = "proving")]
struct RouteInput {
    text: String,
    start: RouteStart,
    solar_sweep: Option<WindowSolarSweep>,
    chart_jump: Option<(u64, f32)>,
}

#[cfg(feature = "proving")]
struct SceneCapture {
    setup: crate::GameSetup,
    render_extent: [u32; 2],
    #[cfg(feature = "draw-probe")]
    omit: Option<usize>,
    content: Option<std::sync::Arc<std::sync::Mutex<thrum_asset_gpu::CookedStats>>>,
}

#[cfg(feature = "proving")]
struct ExitContentReport {
    stats: std::sync::Arc<std::sync::Mutex<thrum_asset_gpu::CookedStats>>,
    output: std::path::PathBuf,
    last_publication: std::cell::Cell<Option<(u64, thrum_asset::ContentHash)>>,
}

#[cfg(feature = "proving")]
impl ExitContentReport {
    fn observe(&self) {
        let Ok(stats) = self.stats.lock() else { return };
        let (Some(generation), Some(root)) = (stats.drawn, stats.drawn_manifest_hash) else {
            return;
        };
        if self.last_publication.get() == Some((generation, root)) {
            return;
        }
        let row = serde_json::json!({
            "generation": generation,
            "cooked_root": root.to_string(),
            "device_generation": stats.drawn_device_generation,
            "submitted_frames": stats.submitted_frames,
            "swaps": stats.swaps.len(),
            "errors": stats.errors,
        });
        drop(stats);
        let temporary = self.output.with_extension("partial");
        if std::fs::write(&temporary, row.to_string())
            .and_then(|()| std::fs::rename(&temporary, &self.output))
            .is_ok()
        {
            self.last_publication.set(Some((generation, root)));
        }
    }
}

#[cfg(feature = "proving")]
struct CaptureCooked {
    folder: std::path::PathBuf,
    path: thrum_asset_gpu::CookedDrawPath,
    preview: Option<ContentPreviewCapture>,
    scale_props: bool,
}

#[cfg(feature = "proving")]
type ContentPreviewCapture =
    std::sync::Arc<std::sync::Mutex<Option<crate::content_preview::PreviewRecord>>>;

#[cfg(feature = "proving")]
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
#[cfg(feature = "proving")]
pub fn capture_game() -> Result<impl Game, String> {
    capture_game_with_route_limit(false, None)
}

#[cfg(feature = "proving")]
fn capture_game_with_route_limit(
    cloud_capture: bool,
    capture_cooked: Option<CaptureCooked>,
) -> Result<HostedGame<WindowGame>, String> {
    Ok(HostedGame(WindowGame {
        demo: crate::selected_setup()?
            .game_controls()
            .map_err(|error| error.to_string())?,
        world: None,
        held_keys: 0,
        replay_controls: false,
        interface_text: None,
        menu: None,
        scene: None,
        exit_content: None,
        exit_content_output: None,
        output_record: None,
        cloud_capture,
        capture_cooked,
        capture_eye: fixed_capture_eye()?,
        hardware: Vec::new(),
        governor_enabled: false,
        capture_route: std::env::var_os("THRUM_CAPTURE_RUNTIME_ROUTE")
            .map(std::fs::read_to_string)
            .transpose()
            .map_err(|error| error.to_string())?
            .map(|text| RouteInput {
                text,
                start: RouteStart::Immediate,
                solar_sweep: None,
                chart_jump: None,
            }),
    }))
}

#[cfg(feature = "proving")]
fn fixed_capture_eye() -> Result<Option<[f32; 3]>, String> {
    crate::content::launch_value("THRUM_CAPTURE_FIXED_EYE")
        .ok()
        .map(|value| serde_json::from_str(&value))
        .transpose()
        .map_err(|error| format!("fixed capture eye: {error}"))
}

impl WindowGame {
    #[cfg(feature = "proving")]
    #[expect(
        clippy::print_stderr,
        reason = "surface proof write failures in the game log"
    )]
    fn record_output(
        &mut self,
        source: &str,
        submission: u64,
        output: thrum_render::runtime::FrameOutput,
    ) {
        use std::io::Write as _;
        if let Some(file) = &mut self.output_record {
            let row = serde_json::json!({
                "source": source,
                "submission": submission,
                "backing_extent": output.backing_extent,
                "output_extent": output.output_extent,
                "output_scale": output.output_scale.percent(),
                "output_scale_forced": output.output_scale_forced,
            });
            if let Err(error) = writeln!(file, "{row}") {
                eprintln!("game output record: {error}");
            }
        }
    }
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
    #[cfg(feature = "proving")]
    fn observe_display(&mut self, display: thrum_app::DisplayRecord) {
        self.record_output("display", display.submission, display.output);
    }

    #[cfg(feature = "proving")]
    fn observe_frame(&mut self, sample: &thrum_render::runtime::FrameSample) {
        if sample.surface_presented {
            self.record_output("frame", sample.submission_order, sample.output);
        }
    }

    #[cfg(feature = "proving")]
    fn frame_edits(&self) -> crate::controls::ControlEdits {
        if let Some(report) = &self.exit_content {
            report.observe();
        }
        crate::controls::ControlEdits::default()
    }

    #[cfg(feature = "proving")]
    fn status_display(&self) -> bool {
        self.interface_text == Some(crate::InterfaceText::Status)
    }

    #[cfg(feature = "proving")]
    fn menu(&self) -> Option<&crate::interface_menu::Menu> {
        self.menu.as_ref()
    }

    #[cfg(feature = "proving")]
    fn menu_mut(&mut self) -> Option<&mut crate::interface_menu::Menu> {
        self.menu.as_mut()
    }

    #[cfg(feature = "proving")]
    fn interface_input(
        &mut self,
        event: thrum_ui::Event<KeyCode>,
        now: std::time::Duration,
    ) -> Result<KeyResponse, String> {
        let Some(menu) = &mut self.menu else {
            return Ok(KeyResponse::Ignored);
        };
        menu.event(event, now);
        Ok(KeyResponse::Input)
    }

    fn demo(&self) -> &GameControls {
        &self.demo
    }

    fn demo_mut(&mut self) -> &mut GameControls {
        &mut self.demo
    }

    fn mac_scene_kind(&self) -> thrum_render::runtime::SceneKind {
        use thrum_render::runtime::SceneKind;
        #[cfg(feature = "proving")]
        if crate::selected_setup().is_ok_and(|setup| setup.sprite_scale.is_some()) {
            return SceneKind::TwoD;
        }
        SceneKind::ThreeD
    }

    fn supports_edr(&self) -> bool {
        !thrum_render::synthetic::mac_backing_grid_requested()
            && crate::selected_setup().is_ok_and(|setup| {
                #[cfg(feature = "proving")]
                {
                    setup.display_overlay != thrum_render::synthetic::DisplayOverlay::TracedGeometry
                }
                #[cfg(not(feature = "proving"))]
                {
                    let _ = setup;
                    true
                }
            })
    }

    fn replay(&self) -> crate::controls::ControlEdits {
        // The window queues control edits across threads. Offscreen captures edit
        // the retained scene source directly, including any scripted route.
        if self.replay_controls {
            self.demo.replay()
        } else {
            crate::controls::ControlEdits::default()
        }
    }

    fn hardware_requests(&self) -> &[HardwareRequest] {
        &self.hardware
    }

    async fn prepare_runtime(
        &mut self,
        gpu: Gpu<'_>,
        world_advanced: Box<dyn Fn() + Send>,
    ) -> Result<Prepared<GameWorld, GameScene>, String> {
        #[cfg(feature = "proving")]
        if let Some(scene) = &mut self.scene {
            let mut host = scene
                .setup
                .prepare_startup(
                    gpu.device,
                    gpu.queue,
                    gpu.capabilities,
                    FrameConfig {
                        render_extent: scene.render_extent,
                        surface: Some(gpu.surface),
                    },
                    None,
                )
                .await?;
            crate::apply_cloud_quality(&mut host).map_err(|e| e.to_string())?;
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
        let setup = crate::selected_setup()?;
        #[cfg(feature = "proving")]
        let setup = crate::GameSetup {
            route_scale_props: self
                .capture_cooked
                .as_ref()
                .is_some_and(|capture| capture.scale_props),
            ..setup
        };
        let config = FrameConfig {
            render_extent,
            surface: Some(gpu.surface),
        };
        let mut host = setup
            .prepare_startup(
                gpu.device,
                gpu.queue,
                gpu.capabilities,
                config,
                crate::content::placed_sky()?,
            )
            .await?;
        #[cfg(feature = "proving")]
        crate::apply_cloud_quality(&mut host).map_err(|e| e.to_string())?;
        #[cfg(feature = "proving")]
        let preview = self
            .capture_cooked
            .as_ref()
            .and_then(|cooked| cooked.preview.as_ref());
        #[cfg(feature = "proving")]
        let content = if let Some(preview) = preview {
            *preview.lock().map_err(|error| error.to_string())? =
                Some(crate::content::attach_content_preview(&mut host)?);
            None
        } else if let Some(cooked) = &self.capture_cooked {
            if setup.content == crate::GameContent::Exit {
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
        #[cfg(not(feature = "proving"))]
        crate::attach_cooked_scene(&mut host)?;
        #[cfg(feature = "proving")]
        if setup.content == crate::GameContent::Exit
            && let Some(output) = self.exit_content_output.clone()
            && let Some(stats) = content.as_ref()
        {
            self.exit_content = Some(ExitContentReport {
                stats: std::sync::Arc::clone(stats),
                output,
                last_publication: std::cell::Cell::new(None),
            });
        }
        host.finish_preparation();
        #[cfg(feature = "proving")]
        if let Some(eye) = if self.replay_controls {
            fixed_capture_eye()?
        } else {
            self.capture_eye
        } {
            if self.capture_route.is_some() {
                return Err("a fixed capture eye cannot override an active route".to_owned());
            }
            let camera = host
                .scene_camera()
                .ok_or("fixed capture eye needs a cooked scene")?;
            host.set_scene_camera(camera.at(eye)?)?;
        }
        self.demo.observe_camera(host.camera_orientation());
        #[cfg(feature = "proving")]
        let offset = if let Some(route) = &self.capture_route {
            capture::prepare_route(
                &mut host,
                content,
                &route.text,
                render_extent,
                &gpu,
                self.cloud_capture,
                self.capture_cooked
                    .as_ref()
                    .map(|cooked| cooked.folder.as_path()),
            )?
        } else {
            if self.capture_cooked.is_some() {
                capture::prepare_parity_content(&mut host, content.as_deref())?;
            }
            std::time::Duration::ZERO
        };
        if self.governor_enabled
            && !host.enable_governor(thrum_render::governor::DisplayRecord {
                interval: std::time::Duration::ZERO,
                thermal: thrum_render::governor::ThermalState::Nominal,
            })
        {
            return Err("governor requires a prepared TAAU scene".to_owned());
        }
        #[cfg(feature = "proving")]
        if self
            .capture_route
            .as_ref()
            .is_some_and(|route| matches!(route.start, RouteStart::Immediate))
            && self.replay_controls
        {
            host.world_mut().resume(offset)?;
        }
        #[cfg(feature = "proving")]
        if let Some(route) = &self.capture_route {
            let scene = host.continuous_source_mut();
            scene.solar_sweep = route.solar_sweep;
            scene.chart_jump = route.chart_jump;
        }
        #[cfg(not(feature = "proving"))]
        let origin = Instant::now();
        #[cfg(feature = "proving")]
        let origin = Instant::now()
            .checked_sub(offset)
            .ok_or("capture route origin underflow")?;
        #[cfg(feature = "proving")]
        if let Some(menu) = &self.menu {
            host.world_mut().set_menu(menu.mailbox());
            host.continuous_source_mut().set_menu(menu.mailbox());
        }
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
