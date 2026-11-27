//! The platform owns the surface and event loop; the game's runtime host owns world
//! ticks, GPU scene packets and the prepared presentation pass.

mod device;
mod display;
mod display_link;
pub use display_link::thermal_state;
mod edit_flow;
#[cfg(feature = "interface")]
mod interface;
#[cfg(feature = "interface")]
mod interface_script;
#[cfg(feature = "interface")]
pub use interface_script::InterfaceScript;
mod output_scale;
mod profile_recording;
mod qualification;
mod qualification_signpost;
mod recovery;
mod refresh_account;
mod render_actor;
mod window_mode;

pub use display::{DisplayOverride, DisplayOverrideError, WindowDisplayOptions};
pub use qualification::{
    ExtentWindow, GovernorProof, ScaleReplay, TransitionArm, WindowExtentCapture,
    WindowQualification, WindowTransitionProof,
};

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use device::WindowDevice;
use display_link::{FrameRateRequest, ViewDisplayLink};
use edit_flow::EditFlow;
use output_scale::{ModeChange, ModeEvent, OutputRequest};
use recovery::Recovery;
use render_actor::{
    FrameCounters, FrameEdits, FrameRequest, RecoveryFailure, RenderJob, RenderOutcome,
    RenderReply, RenderWorker, SurfaceStatus,
};
use thrum_controls::macos::MacPanel;
use thrum_core::sim::{WorldLaneFault, WorldLifecyclePolicy, WorldLifecycleStatus};
use thrum_render::{
    capabilities::{FIRST_DEVICE_GENERATION, configure_surface},
    frame::TargetLeadStatus,
    present::SurfaceProfile,
    runtime::{FrameSample, SceneKind},
};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    keyboard::PhysicalKey,
    platform::macos::WindowExtMacOS,
    window::{Window, WindowId},
};

use crate::{
    Config, Game, Gpu, KeyCode, KeyResponse,
    file_watch::{self, FileWatch, Kinds, Reload},
    game::apply_params_file,
};

/// A game whose runtime can move to the window's render thread and accepts physical keys.
pub trait WindowGame: Game<World: Send, Scene: Send> {}

impl<G> WindowGame for G where G: Game<World: Send, Scene: Send> {}

const DEFAULT_FRAMES: usize = 600;
const MAX_FRAMES: usize = 100_000;
/// Entries each profile history keeps in a run until closed; the oldest half is dropped
/// when one exceeds it, so a long session holds a bounded recent window.
const RETAINED_PROFILES: usize = 36_000;

enum TimingPolicy {
    Report,
    Profiler,
}

#[derive(Clone, Copy)]
enum RunMode {
    Ordinary,
    Bounded,
    QuietUntilClosed,
}

#[derive(Clone, Copy, Debug)]
enum UserEvent {
    WorldAdvanced,
    DisplayTarget {
        target: f64,
        callback: f64,
        number: u64,
    },
    DisplayParametersChanged,
    ThermalState(thrum_render::governor::ThermalState),
    RenderDone,
    DeviceLost(u64),
    ControlsEdited,
    ExtentResize([u32; 2]),
    ExtentCaptured,
    /// A watched parameter or game file settled on new bytes; [`FileWatch::take`] says which.
    FilesSaved,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum WindowVisibility {
    Visible,
    Occluded,
    OccludedDuringModeChange,
}

struct App<G: Game> {
    extent_capture: Option<WindowExtentCapture>,
    transition_proof: Option<WindowTransitionProof>,
    config: Config,
    wake: EventLoopProxy<UserEvent>,
    display: Option<display::MacDisplay>,
    display_options: Option<WindowDisplayOptions>,
    display_link: Option<ViewDisplayLink>,
    frame_rate: Option<FrameRateRequest>,
    refresh_period: Option<f64>,
    window: Option<Arc<Window>>,
    display_target: Option<f64>,
    governor_display: thrum_render::governor::DisplayRecord,
    display_tick: u64,
    target_sequence: u64,
    display_link_callbacks: usize,
    late_target_evaluations: usize,
    invalid_target_evaluations: usize,
    clamped_target_evaluations: usize,
    pending_display_target: bool,
    surface_format: Option<wgpu::TextureFormat>,
    surface_extent: [u32; 2],
    presented_output: Option<crate::DisplayRecord>,
    output_mode: ModeChange,
    scene_kind: SceneKind,
    window_mode: Option<window_mode::WindowMode>,
    output_log: Option<std::fs::File>,
    proof_toggles: u8,
    #[cfg(feature = "interface")]
    interface_output: Option<crate::OutputMetrics>,
    #[cfg(feature = "interface")]
    interface_input: interface::WindowInput,
    #[cfg(feature = "interface")]
    interface_script: Option<interface_script::Script>,
    window_visibility: WindowVisibility,
    surface_generation: Arc<AtomicU64>,
    render_stopped: Arc<AtomicBool>,
    render: Option<RenderWorker<G>>,
    pause: WorldLifecyclePolicy,
    recovery: Recovery,
    recovery_error: Option<String>,
    device_losses: u64,
    world_tick: u64,
    render_in_flight: Option<u64>,
    refresh_account: Arc<Mutex<refresh_account::Account>>,
    window_occlusions: u64,
    thermal_states: [bool; 4],
    counters: FrameCounters,
    /// The game's controls are shared by keys and the panel. Edits travel to the render
    /// owner in the next frame request.
    game: G,
    panel: Option<MacPanel>,
    edits: EditFlow<G::Edits>,
    /// `THRUM_PARAMS`: a parameter file applied once the window is up and again on `L` or a save.
    params_path: Option<std::path::PathBuf>,
    /// Watches the parameter file and [`Game::reload_files`] from before the first load; stopped
    /// before the renderer shuts down.
    watch: Option<FileWatch>,
    origin: Instant,
    last_call: Duration,
    presented: usize,
    target_frames: usize,
    /// Wall deadline of a bounded run; a run until closed has none.
    run_deadline: Option<Duration>,
    /// Profile history bound of a run until closed; a bounded run retains every sample.
    retain_limit: Option<usize>,
    /// Presented samples dropped from a bounded history.
    trimmed_presented: usize,
    /// A run until closed ended because its window closed, which is success.
    window_closed: bool,
    callbacks: usize,
    unavailable: usize,
    occluded: usize,
    surface_faults: usize,
    pre_acquire_epoch_changes: usize,
    drawable_acquire_attempts: usize,
    surface_configuration_error: Option<String>,
    callback_walls: Vec<Duration>,
    worker_walls: Vec<Duration>,
    acquire_walls: Vec<Duration>,
    samples: Vec<FrameSample>,
    profile: crate::profile::ProfileWindow,
    profile_recording: Option<profile_recording::Recording>,
    timing_policy: TimingPolicy,
    profiler_title_at: Duration,
    adapter_name: String,
    error: Option<String>,
    closed_by_user: bool,
    qualification: Option<qualification::Pan>,
    transition_ticks: Option<qualification::TransitionTicks>,
    qualification_report: Option<Result<String, String>>,
}

impl<G: WindowGame + 'static> App<G> {
    fn start_display_link(&mut self, wake: &EventLoopProxy<UserEvent>) -> Result<(), String> {
        self.display_link = Some(ViewDisplayLink::start(
            self.window.as_ref().ok_or("window missing")?,
            wake.clone(),
            self.display.is_some(),
            self.frame_rate.ok_or("frame rate missing")?,
        )?);
        if self
            .transition_proof
            .as_ref()
            .is_some_and(|proof| proof.arm == TransitionArm::Fullscreen)
        {
            // The authored route is prepared at its preceding paused tick.
            // Give the ordinary display link and live governor time to settle.
            self.pause.request(true);
            self.reconcile_pause()?;
            self.refresh_account
                .lock()
                .expect("refresh account lock")
                .measure(objc2_quartz_core::CACurrentMediaTime(), 180.0);
        }
        Ok(())
    }

    fn begin_fullscreen_route(&mut self) -> Result<(), String> {
        if self.presented == 90
            && self
                .transition_proof
                .as_ref()
                .is_some_and(|proof| proof.arm == TransitionArm::Fullscreen)
            && self.pause.request(false)
        {
            self.reconcile_pause()?;
        }
        Ok(())
    }

    fn panel_hz(&self) -> f32 {
        self.refresh_period
            .map_or(display_link::PREFERRED_REFRESH_HZ, |period| {
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "AppKit's requested rate is f32"
                )]
                let rate = (1.0 / period) as f32;
                rate
            })
    }

    fn refresh_monitor_period(&mut self) {
        self.refresh_period = self
            .window
            .as_ref()
            .and_then(|window| window.current_monitor())
            .and_then(|monitor| monitor.refresh_rate_millihertz())
            .filter(|&rate| rate > 0)
            .map(|rate| 1000.0 / f64::from(rate));
        let rate = FrameRateRequest::resolve(
            self.panel_hz(),
            self.output_mode.fullscreen(),
            self.config.mac_frame_rate,
        );
        if let Some(link) = &self.display_link {
            link.set_rate(rate);
        }
        self.frame_rate = Some(rate);
    }

    fn output_record(&mut self, row: &serde_json::Value) -> Result<(), String> {
        use std::io::Write as _;
        if let Some(file) = &mut self.output_log {
            writeln!(file, "{row}").map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    fn sample_output(&mut self) -> Result<OutputRequest, String> {
        use objc2_quartz_core::CACurrentMediaTime;
        use thrum_render::capabilities::DeviceCapabilities;

        let window = self.window.as_ref().ok_or("window missing")?;
        let native = self.window_mode.as_ref().ok_or("window mode missing")?;
        let size = window.inner_size();
        let backing = [size.width, size.height];
        let monitor_extent = self.output_log.as_ref().and_then(|_| {
            window.current_monitor().map(|monitor| {
                let size = monitor.size();
                [size.width, size.height]
            })
        });
        let now = self.origin.elapsed();
        let events = native.take();
        let fallback = if events.is_empty() {
            self.output_mode.expire(now, || native.fullscreen())
        } else {
            None
        };
        for (event, media) in events {
            self.output_mode.event(event, now);
            let (phase, fullscreen) = match event {
                ModeEvent::Start(fullscreen) => ("start", fullscreen),
                ModeEvent::Complete(fullscreen) => ("complete", fullscreen),
            };
            if self.output_log.is_some() {
                self.output_record(&serde_json::json!({
                    "event": phase, "media": media, "fullscreen": fullscreen, "backing": backing,
                }))?;
            }
            if matches!(event, ModeEvent::Complete(_)) {
                self.refresh_monitor_period();
            }
        }
        if let Some(fullscreen) = fallback {
            self.refresh_monitor_period();
            if self.output_log.is_some() {
                self.output_record(&serde_json::json!({
                    "event": "limit", "media": CACurrentMediaTime(),
                    "fullscreen": fullscreen, "backing": backing,
                }))?;
            }
        }
        if self.output_mode.held() && self.output_log.is_some() {
            self.record_held_backing(backing)?;
        }

        let hz = self.panel_hz();
        let kind = self.scene_kind;
        let forced = self.config.mac_output_scale;
        let proof = self
            .transition_proof
            .as_ref()
            .and_then(|proof| proof.fullscreen_output_scale);
        let registry = proof.unwrap_or_else(DeviceCapabilities::mac_fullscreen_output_scale);
        if let Some(request) = self.output_mode.observe(backing, |fullscreen| {
            output_scale::resolve(kind, fullscreen, hz, registry, forced)
        }) && self.output_log.is_some()
        {
            let output = request.extent();
            self.output_record(&serde_json::json!({
                "event": "request", "media": CACurrentMediaTime(),
                "fullscreen": self.output_mode.fullscreen(), "panel_hz": hz,
                "backing": request.backing, "output": output, "monitor_extent": monitor_extent,
                "output_scale": request.scale.step.percent(), "forced": request.scale.forced,
                "registry_proof_input": proof.map(thrum_render::synthetic::RenderScale::percent),
                "layer": self.display.as_ref().map(display::MacDisplay::layer_record),
            }))?;
        }
        self.output_mode
            .current()
            .ok_or("no initial output request".into())
    }

    fn record_held_backing(&mut self, backing: [u32; 2]) -> Result<(), String> {
        if self.output_mode.held() && self.output_log.is_some() {
            self.output_record(&serde_json::json!({
                "event": "held_backing", "media": objc2_quartz_core::CACurrentMediaTime(),
                "backing": backing,
            }))?;
        }
        Ok(())
    }

    fn observe_occlusion(&mut self, occluded: bool) -> Result<(), String> {
        self.sample_output()?;
        let held = self.output_mode.held();
        if occluded {
            if self.window_visibility != WindowVisibility::Visible {
                return Ok(());
            }
            self.window_occlusions += 1;
            self.window_visibility = if held {
                WindowVisibility::OccludedDuringModeChange
            } else {
                WindowVisibility::Occluded
            };
            if held {
                // AppKit moves the window between Spaces. The drawable can
                // become temporarily unavailable while its configuration stays valid.
                return Ok(());
            }
        } else {
            let previous =
                std::mem::replace(&mut self.window_visibility, WindowVisibility::Visible);
            if held || previous != WindowVisibility::Occluded {
                return Ok(());
            }
        }
        self.invalidate_surface()
    }

    fn toggle_for_proof(&mut self, request: OutputRequest) {
        if !self
            .transition_proof
            .as_ref()
            .is_some_and(|proof| proof.arm == TransitionArm::OutputScaleMode)
            || self.output_mode.held()
            || self.surface_extent != request.extent()
        {
            return;
        }
        let should_toggle = match self.proof_toggles {
            0 => self.presented >= 12,
            1 => self.output_mode.fullscreen(),
            _ => false,
        };
        if should_toggle && let Some(native) = &self.window_mode {
            self.proof_toggles += 1;
            native.toggle();
        }
    }

    fn refresh_display_record(&mut self) {
        if let (Some(display), Some(window)) = (&mut self.display, &self.window) {
            display.sample(self.target_sequence, window.scale_factor());
            display.configured(self.surface_format == Some(wgpu::TextureFormat::Rgba16Float));
        }
    }

    fn timing_requested(&self) -> bool {
        matches!(self.timing_policy, TimingPolicy::Report)
            || self
                .qualification
                .as_ref()
                .is_some_and(|pan| pan.options.gpu_timestamps)
            || crate::profile::enabled(self.game.controls()).unwrap_or(true)
    }

    /// `target_frames` of `None` runs until the window closes.
    fn new(
        config: Config,
        wake: EventLoopProxy<UserEvent>,
        target_frames: Option<usize>,
        game: G,
    ) -> Self {
        let run_deadline = target_frames.map(|frames| {
            Duration::from_secs(30).max(
                Duration::from_secs(10)
                    + Duration::from_millis(
                        u64::try_from(frames).expect("bounded frame count") * 10,
                    ),
            )
        });
        let retain_limit = target_frames.is_none().then_some(RETAINED_PROFILES);
        let reserved = target_frames.unwrap_or(RETAINED_PROFILES / 8);
        Self {
            extent_capture: None,
            transition_proof: None,
            config,
            wake,
            display: None,
            display_options: None,
            display_link: None,
            frame_rate: None,
            refresh_period: None,
            window: None,
            display_target: None,
            governor_display: thrum_render::governor::DisplayRecord {
                interval: Duration::ZERO,
                thermal: thrum_render::governor::ThermalState::Nominal,
            },
            display_tick: 0,
            target_sequence: 0,
            display_link_callbacks: 0,
            late_target_evaluations: 0,
            invalid_target_evaluations: 0,
            clamped_target_evaluations: 0,
            pending_display_target: false,
            surface_format: None,
            surface_extent: [0, 0],
            presented_output: None,
            output_mode: ModeChange::default(),
            scene_kind: SceneKind::default(),
            window_mode: None,
            output_log: None,
            proof_toggles: 0,
            #[cfg(feature = "interface")]
            interface_output: None,
            #[cfg(feature = "interface")]
            interface_input: interface::WindowInput::default(),
            #[cfg(feature = "interface")]
            interface_script: None,
            window_visibility: WindowVisibility::Visible,
            surface_generation: Arc::new(AtomicU64::new(1)),
            render_stopped: Arc::new(AtomicBool::new(false)),
            render: None,
            pause: WorldLifecyclePolicy::default(),
            recovery: Recovery::new(FIRST_DEVICE_GENERATION),
            recovery_error: None,
            device_losses: 0,
            world_tick: 0,
            render_in_flight: None,
            refresh_account: Arc::default(),
            window_occlusions: 0,
            thermal_states: [false; 4],
            counters: FrameCounters::default(),
            game,
            panel: None,
            edits: EditFlow::default(),
            params_path: std::env::var_os("THRUM_PARAMS").map(std::path::PathBuf::from),
            watch: None,
            origin: Instant::now(),
            last_call: Duration::ZERO,
            presented: 0,
            target_frames: target_frames.unwrap_or(usize::MAX),
            run_deadline,
            retain_limit,
            trimmed_presented: 0,
            window_closed: false,
            callbacks: 0,
            unavailable: 0,
            occluded: 0,
            surface_faults: 0,
            pre_acquire_epoch_changes: 0,
            drawable_acquire_attempts: 0,
            surface_configuration_error: None,
            callback_walls: Vec::with_capacity(reserved * 8),
            worker_walls: Vec::with_capacity(reserved * 4),
            acquire_walls: Vec::with_capacity(reserved * 4),
            samples: Vec::with_capacity(reserved * 2),
            profile: crate::profile::ProfileWindow::new(),
            profile_recording: profile_recording::Recording::from_environment(),
            timing_policy: TimingPolicy::Profiler,
            profiler_title_at: Duration::ZERO,
            adapter_name: String::new(),
            error: None,
            closed_by_user: false,
            qualification: None,
            transition_ticks: None,
            qualification_report: None,
        }
    }

    fn invalidate_surface(&self) -> Result<(), String> {
        self.surface_generation
            .try_update(Ordering::AcqRel, Ordering::Acquire, |generation| {
                generation.checked_add(1)
            })
            .map(|_| ())
            .map_err(|_| "surface generation exhausted".to_owned())
    }

    /// Take edits from keys, the panel or a parameter file. Presentation edits ride the next
    /// frame request; the pause toggle takes the same path as the P key.
    fn controls_edited(&mut self) -> Result<(), String> {
        self.profile.configure(
            crate::profile::enabled(self.game.controls()).unwrap_or(false),
            self.recovery.active,
            u64::MAX,
        );
        let (presentation, paused) = self.game.take_edits();
        if let Some(paused) = paused
            && self.pause.request(paused)
        {
            self.reconcile_pause()?;
        }
        // A frame already in flight reads back the state before these edits.
        if self.edits.queue(presentation)
            && let Some(window) = &self.window
        {
            window.request_redraw();
        }
        self.sync_panel();
        Ok(())
    }

    /// Apply the `THRUM_PARAMS` file through the same edit path as the panel. A missing or
    /// malformed file, or a refused entry, is reported and leaves those controls as they were.
    #[expect(
        clippy::print_stderr,
        reason = "parameter diagnostics are for the person editing the file"
    )]
    fn load_params(&mut self, saved: bool) -> Result<(), String> {
        let Some(path) = self.params_path.clone() else {
            return Ok(());
        };
        let tag = if saved { " (saved)" } else { "" };
        match apply_params_file(&path, self.game.controls_mut()) {
            Ok(report) => {
                eprintln!("{}: {}{tag}: {report}", self.config.name, path.display());
                for problem in &report.problems {
                    eprintln!("  {problem}");
                }
                self.controls_edited()
            }
            Err(error) => {
                eprintln!(
                    "{}: {}{tag}: {error}; controls keep their values (fix the file and save, or press L)",
                    self.config.name,
                    path.display()
                );
                Ok(())
            }
        }
    }

    fn sync_panel(&mut self) {
        if let Some(panel) = &mut self.panel {
            panel.sync(self.game.controls());
        }
    }

    fn window_interface_input(&self) -> bool {
        #[cfg(feature = "interface")]
        {
            self.game.wants_interface_input()
        }
        #[cfg(not(feature = "interface"))]
        {
            false
        }
    }

    /// Continuous game input changed: the next display target renders it.
    fn input_changed(&mut self) {
        self.edits.request();
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn reconcile_pause(&mut self) -> Result<(), String> {
        let Some(controls) = self.game.world() else {
            return Ok(());
        };
        let was_transitioning = self.pause.needs_reply() || self.pause.next_request().is_some();
        if self.pause.needs_reply()
            && let Some(reply) = controls
                .poll_lifecycle_reply()
                .map_err(|error| error.to_string())?
        {
            self.pause
                .acknowledge(reply.request)
                .map_err(|error| error.to_string())?;
            self.world_tick = reply.id.tick;
        }
        // Consume the acknowledgement before checking current health: an old
        // render report may outlive recovery, or a new overload may already
        // follow a successful Resume. Only the world's latest state decides.
        match controls.check_health() {
            Ok(tick) => self.world_tick = tick,
            Err(WorldLaneFault::Overload(refused_ticks)) => self.pause.overloaded(refused_ticks),
            Err(error) => return Err(error.to_string()),
        }
        if let Some(request) = self.pause.next_request() {
            match controls.request_lifecycle(request) {
                Ok(()) => self.pause.accepted(request),
                Err(WorldLaneFault::Busy) => {}
                Err(error) => return Err(error.to_string()),
            }
        }
        if was_transitioning || self.pause.needs_reply() || self.pause.next_request().is_some() {
            self.refresh_title();
        }
        Ok(())
    }

    /// The pause label, or "Still" for a game with no world link, which has nothing to pause.
    fn world_status(&self) -> &'static str {
        if self.game.world().is_some() {
            match self.pause.status() {
                WorldLifecycleStatus::Running => "Running (P)",
                WorldLifecycleStatus::Pausing => "Pausing (P)",
                WorldLifecycleStatus::Paused => "Paused (P)",
                WorldLifecycleStatus::Resuming => "Resuming (P)",
                WorldLifecycleStatus::OverloadDraining => "Overloaded: draining (P)",
                WorldLifecycleStatus::OverloadPaused => "Overloaded: paused (P)",
                WorldLifecycleStatus::OverloadResuming => "Overloaded: resuming (P)",
            }
        } else {
            "Still"
        }
    }

    fn refresh_title(&self) {
        if let Some(window) = &self.window {
            let world_status = self.world_status();
            let title = self.samples.last().map_or_else(
                || {
                    format!(
                        "{} | {}{}",
                        self.config.title,
                        world_status,
                        self.recovery.label()
                    )
                },
                |sample| {
                    profiler_title(
                        self.config.title,
                        sample,
                        world_status,
                        self.recovery.label(),
                    )
                },
            );
            let title = if self.recovery.failed()
                && let Some(error) = &self.recovery_error
            {
                format!("{title} | {error}")
            } else if self.recovery.ready()
                && let Some(error) = &self.surface_configuration_error
            {
                format!(
                    "{} | Surface unavailable (R to retry) | {error}",
                    self.config.title
                )
            } else {
                title
            };
            window.set_title(&title);
        }
    }

    fn device_lost(&mut self, generation: u64) -> Result<(), String> {
        if self.recovery.lost(generation)? {
            self.profile.set_available(false);
            self.device_losses = self.device_losses.saturating_add(1);
            self.invalidate_surface()?;
            self.edits.request();
        }
        self.refresh_title();
        if let Some(window) = &self.window {
            window.request_redraw();
        }
        Ok(())
    }

    fn admit_recovery(&mut self) -> Result<(), String> {
        if self.render_in_flight.is_some() {
            return Ok(());
        }
        let Some(generation) = self.recovery.queued() else {
            return Ok(());
        };
        let output = self.sample_output()?;
        let backing_scale = self
            .window
            .as_ref()
            .ok_or("window missing")?
            .scale_factor()
            .to_bits();
        let job = RenderJob::Recover {
            from_generation: self.recovery.active,
            generation,
            output,
            backing_scale,
            surface_generation: self.surface_generation.load(Ordering::Acquire),
        };
        match self
            .render
            .as_ref()
            .ok_or("render worker missing")?
            .try_send(job)
        {
            Ok(()) => {
                self.recovery.accepted(generation);
                self.render_in_flight = Some(0);
            }
            Err(thrum_render::frame::SendFault::Busy(_)) => {}
            Err(thrum_render::frame::SendFault::Closed(_)) => {
                return Err("render worker stopped during recovery".to_owned());
            }
        }
        Ok(())
    }

    /// `L` reloads the parameter file and the game's own files; every other key belongs to the
    /// game.
    fn set_key(&mut self, code: KeyCode, pressed: bool) -> Result<(), String> {
        if pressed && code == KeyCode::KeyL {
            return file_watch::deliver(self, Kinds::ALL, false);
        }
        match self
            .game
            .key(code, pressed)
            .map_err(|error| error.to_string())?
        {
            KeyResponse::Ignored => Ok(()),
            KeyResponse::Edited => self.controls_edited(),
            KeyResponse::Input => {
                self.input_changed();
                Ok(())
            }
        }
    }

    fn attach_output_policy(&mut self, window: &Arc<Window>) -> Result<OutputRequest, String> {
        self.output_mode.event(
            ModeEvent::Complete(window.fullscreen().is_some() || window.simple_fullscreen()),
            Duration::ZERO,
        );
        self.window_mode = Some(window_mode::WindowMode::attach(window)?);
        self.window = Some(Arc::clone(window));
        self.scene_kind = self.game.mac_scene_kind();
        self.refresh_monitor_period();
        self.sample_output()
    }

    fn prepare_transition_window(&mut self, window: &Window) -> Result<(), String> {
        if let Some(proof) = &self.transition_proof {
            if matches!(
                proof.arm,
                TransitionArm::Fullscreen | TransitionArm::OutputScaleFullscreen
            ) {
                window.set_borderless_game(true);
                if !window.set_simple_fullscreen(true) {
                    return Err("could not enter fullscreen for the route proof".to_owned());
                }
            }
            self.transition_ticks = Some(qualification::TransitionTicks::start(proof)?);
            if matches!(
                proof.arm,
                TransitionArm::OutputScaleFullscreen | TransitionArm::OutputScaleMode
            ) {
                self.output_log = Some(
                    std::fs::File::create(proof.output.join("output-requests.jsonl"))
                        .map_err(|error| error.to_string())?,
                );
            }
            let id = display_link::window_number(window)?;
            std::fs::write(proof.output.join("window-id"), id.to_string())
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    fn prepare(
        &mut self,
        event_loop: &ActiveEventLoop,
        wake: &EventLoopProxy<UserEvent>,
    ) -> Result<(), String> {
        #[cfg(feature = "interface")]
        if self.game.wants_interface_input() {
            self.interface_script = self
                .game
                .interface_script()
                .map(interface_script::Script::new)
                .transpose()?;
        }
        let attributes = self.extent_capture.as_ref().map_or_else(
            || {
                let attributes =
                    Window::default_attributes().with_inner_size(winit::dpi::PhysicalSize::new(
                        self.config.window_size[0],
                        self.config.window_size[1],
                    ));
                match self.config.window_position {
                    Some([x, y]) => {
                        attributes.with_position(winit::dpi::LogicalPosition::new(x, y))
                    }
                    None => attributes,
                }
            },
            WindowExtentCapture::attributes,
        );
        let governor_proof = self
            .transition_proof
            .as_ref()
            .is_some_and(|proof| proof.governor.is_some());
        let attributes = if governor_proof || self.config.window_always_on_top {
            attributes.with_window_level(winit::window::WindowLevel::AlwaysOnTop)
        } else {
            attributes
        };
        let window = Arc::new(
            event_loop
                .create_window(attributes.with_title(self.config.window_title))
                .map_err(|error| format!("window: {error}"))?,
        );
        if let Some(capture) = &self.extent_capture {
            capture.prepare_window(&window)?;
        }
        self.prepare_transition_window(&window)?;
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.backends = wgpu::Backends::METAL;
        let instance = wgpu::Instance::new(descriptor);
        let surface = Arc::new(
            instance
                .create_surface(Arc::clone(&window))
                .map_err(|error| format!("surface: {error}"))?,
        );
        if self.game.supports_edr()
            && self.extent_capture.is_none()
            && (self.transition_proof.is_none()
                || self.display_options.is_some()
                || self
                    .transition_proof
                    .as_ref()
                    .is_some_and(|proof| proof.arm == TransitionArm::Fullscreen))
        {
            let mut display = display::MacDisplay::attach(&window, self.display_options.as_ref())?;
            display.sample(0, window.scale_factor());
            self.display = Some(display);
        }
        let output = self.attach_output_policy(&window)?;
        let backing_scale = window.scale_factor().to_bits();
        let recipe = WindowDevice {
            instance,
            surface,
            hardware: self.game.hardware_requests().to_vec(),
            metal_preparation: self.config.metal_preparation.clone(),
            headroom: self
                .display
                .as_ref()
                .map(|display| Arc::clone(&display.headroom)),
            edr_readback: self
                .display_options
                .as_ref()
                .is_some_and(|options| options.capture && options.output.is_some()),
        };
        let gpu = pollster::block_on(recipe.create(FIRST_DEVICE_GENERATION, output, wake, false))?;
        if gpu.watch.is_lost() {
            return Err("initial device lost before preparation".to_owned());
        }
        configure_surface(&recipe.surface, &gpu.device, &gpu.config)
            .map_err(|error| error.to_string())?;
        if gpu.watch.is_lost() {
            return Err("initial device lost during surface configuration".to_owned());
        }
        if let Some(display) = &mut self.display {
            display.configured(gpu.config.format == wgpu::TextureFormat::Rgba16Float);
        }
        let world_wake = wake.clone();
        let prepared = pollster::block_on(self.game.prepare(
            Gpu {
                device: &gpu.device,
                queue: &gpu.queue,
                capabilities: &gpu.capabilities,
                surface: SurfaceProfile {
                    format: gpu.config.format,
                    extent: [gpu.config.width, gpu.config.height],
                },
            },
            Box::new(move || {
                let _ = world_wake.send_event(UserEvent::WorldAdvanced);
            }),
        ))
        .map_err(|error| error.to_string())?;
        if gpu.watch.is_lost() {
            return Err("initial device lost during preparation".to_owned());
        }
        self.origin = prepared.origin.unwrap_or_else(Instant::now);
        let host = prepared.host;
        if host.scene_kind() != self.scene_kind && output.backing != output.extent() {
            return Err("prepared scene kind differs from the Mac launch scene kind".into());
        }
        self.scene_kind = host.scene_kind();
        self.counters = FrameCounters::read(&host);
        if let Some(observer) = &self.qualification {
            let number = display_link::window_number(&window)?;
            let monitor = window.current_monitor();
            std::fs::write(observer.options.output.join("window.txt"), format!(
                "pid={}\nwindow_number={number}\nadapter={}\nformat={:?}\nextent={}x{}\nmonitor={:?}\nposition={:?}\nscale={}\n",
                std::process::id(), gpu.adapter_name, gpu.config.format, gpu.config.width, gpu.config.height,
                monitor.as_ref().map(|m| (m.name(), m.size(), m.refresh_rate_millihertz())), window.outer_position(), window.scale_factor()
            )).map_err(|e| e.to_string())?;
        }
        let panel_wake = wake.clone();
        let panel = MacPanel::new(
            self.game.controls(),
            self.config.panel_title,
            Some(Box::new(move || {
                let _ = panel_wake.send_event(UserEvent::ControlsEdited);
            })),
        )
        .map_err(|error| error.to_string())?;
        if !governor_proof && !self.config.window_always_on_top {
            panel.show(None);
        }
        self.panel = Some(panel);
        self.thermal_states[thermal_state() as usize] = true;
        self.start_display_link(wake)?;
        self.adapter_name.clone_from(&gpu.adapter_name);
        self.surface_format = Some(gpu.config.format);
        self.surface_extent = [gpu.config.width, gpu.config.height];
        self.render = Some(render_actor::start::<G>(
            &self.config,
            host,
            gpu,
            recipe,
            Arc::clone(&self.surface_generation),
            Arc::clone(&self.render_stopped),
            self.origin,
            wake,
            self.extent_capture.take(),
            output.generation(
                backing_scale,
                self.surface_format.expect("configured format"),
                FIRST_DEVICE_GENERATION,
            ),
            self.transition_proof.as_ref(),
            self.qualification
                .as_ref()
                .map(|observer| &observer.options),
            self.display_options.as_ref(),
            Arc::clone(&self.refresh_account),
        )?);
        // Baseline the watched files before the first load, so no later save is missed.
        self.watch = self.start_watch(wake);
        self.load_params(false)?;
        self.game.reload();
        if (self.config.window_always_on_top
            || self
                .qualification
                .as_ref()
                .is_some_and(|observer| observer.options.refresh_measurement.is_some()))
            && let Some(window) = &self.window
        {
            window.focus_window();
        }
        Ok(())
    }

    /// Drop the oldest half of any profile history over a run until closed's bound.
    fn trim_history(&mut self) {
        let Some(limit) = self.retain_limit else {
            return;
        };
        for walls in [
            &mut self.callback_walls,
            &mut self.worker_walls,
            &mut self.acquire_walls,
        ] {
            if walls.len() > limit {
                walls.drain(..walls.len() - limit / 2);
            }
        }
        if self.samples.len() > limit {
            let excess = self.samples.len() - limit / 2;
            self.trimmed_presented += self
                .samples
                .drain(..excess)
                .filter(|sample| sample.surface_presented)
                .count();
        }
    }

    fn keep_sample(&mut self, sample: FrameSample) {
        self.game.observe_frame(&sample);
        self.samples.push(sample);
    }

    fn receive_render(&mut self, mut reply: RenderReply<G::Readback>) -> Result<(), String> {
        self.trim_history();
        if let RenderOutcome::Recovered { attempted, result } = reply.outcome {
            let success = result.is_ok();
            if !self
                .recovery
                .complete(attempted, reply.device_generation, success)
            {
                return Ok(());
            }
            self.render_in_flight = None;
            self.counters = reply.platform.counters;
            for mut sample in reply.samples.into_iter().flatten() {
                sample.detach_gpu_timing();
                self.keep_sample(sample);
            }
            match result {
                Ok(info) => {
                    self.adapter_name = info.adapter_name;
                    self.surface_format = Some(info.format);
                    if let Some(display) = &mut self.display {
                        display.configured(info.format == wgpu::TextureFormat::Rgba16Float);
                    }
                    self.surface_extent = reply.surface_extent;
                    self.recovery_error = if self.recovery.ready() {
                        None
                    } else {
                        Some("replacement device lost".to_owned())
                    };
                    // Replay the latest desired presentation policy, including
                    // edits accepted by a frame cancelled during device loss.
                    self.edits.recovered(self.game.replay());
                    self.pending_display_target = self.display_target.is_some();
                }
                Err(RecoveryFailure::Unavailable(error)) => {
                    #[cfg(feature = "interface")]
                    if self.interface_script.is_some() {
                        return Err(format!("interface replacement failed: {error}"));
                    }
                    self.recovery_error = Some(error);
                }
                Err(RecoveryFailure::Fatal(error)) => return Err(error),
            }
            self.refresh_title();
            return Ok(());
        }
        if reply.device_generation != self.recovery.active {
            return Ok(());
        }
        let request = self.render_in_flight.take();
        self.counters = reply.platform.counters;
        let enabled = crate::profile::enabled(self.game.controls()).unwrap_or(false);
        self.profile.configure(
            enabled,
            reply.device_generation,
            reply.platform.profile_minimum,
        );
        for sample in reply.samples.iter_mut().flatten() {
            sample.detach_gpu_timing();
        }
        if reply.platform.profile_revision == crate::profile::revision(self.game.controls()) {
            for sample in reply.samples.iter().flatten() {
                self.profile.push(sample, self.origin.elapsed());
            }
        }
        self.profile.publish(
            self.game.controls_mut(),
            self.origin.elapsed(),
            self.display_link_callbacks as u64,
            self.presented as u64,
            false,
        );
        if let Some(display) = reply.platform.presented {
            self.presented_output = Some(display);
            self.game.observe_display(display);
        }
        for sample in reply.samples.into_iter().flatten() {
            self.keep_sample(sample);
        }
        self.surface_extent = reply.surface_extent;
        if self.surface_format != Some(reply.platform.surface_format) {
            self.surface_format = Some(reply.platform.surface_format);
            if let Some(display) = &mut self.display {
                display
                    .configured(reply.platform.surface_format == wgpu::TextureFormat::Rgba16Float);
            }
        }
        #[cfg(feature = "interface")]
        if matches!(reply.platform.surface_status, SurfaceStatus::Presented) {
            self.game.interface_submitted(
                &reply.platform.readback,
                self.presented as u64 + 1,
                self.origin.elapsed(),
            );
        }
        if request.is_some_and(|request| self.edits.admits(request)) {
            self.game.observe(reply.platform.readback);
        }
        self.sync_panel();
        self.worker_walls.push(reply.platform.worker_wall);
        if let Some(acquire_wall) = reply.acquire_wall {
            self.acquire_walls.push(acquire_wall);
            self.drawable_acquire_attempts += 1;
        }
        match reply.platform.surface_status {
            SurfaceStatus::Presented => {
                self.presented += 1;
                if let Some(request) = self.output_mode.current() {
                    self.toggle_for_proof(request);
                }
                self.begin_fullscreen_route()?;
                #[cfg(feature = "interface")]
                self.step_interface_script()?;
                if self.surface_configuration_error.take().is_some() {
                    self.refresh_title();
                }
                if reply.sequence == self.target_sequence {
                    self.pending_display_target = false;
                }
            }
            SurfaceStatus::NotAttempted => {}
            SurfaceStatus::FrameSlotsOccupied
            | SurfaceStatus::WorldOverloaded
            | SurfaceStatus::WorldPageUnavailable
            | SurfaceStatus::SceneFrameUnavailable
            | SurfaceStatus::DrawableTimeout
            | SurfaceStatus::WindowExtentEmpty
            | SurfaceStatus::SurfaceConfigurationBlocked
            | SurfaceStatus::SurfaceConfigurationPending
            | SurfaceStatus::SurfacePreparationBusy
            | SurfaceStatus::SubmissionSkipped => self.unavailable += 1,
            SurfaceStatus::Occluded => {
                self.unavailable += 1;
                self.occluded += 1;
            }
            SurfaceStatus::NeedsReconfigure => {
                self.surface_faults += 1;
            }
            SurfaceStatus::SurfaceEpochChanged => {
                self.surface_faults += 1;
                if reply.acquire_wall.is_none() {
                    self.pre_acquire_epoch_changes += 1;
                }
            }
            SurfaceStatus::ConfigurationRefused(error) => {
                self.surface_faults += 1;
                self.surface_configuration_error = Some(error);
                self.refresh_title();
            }
        }
        if let Some(error) = reply.platform.transition_error {
            self.surface_configuration_error = Some(error);
            self.refresh_title();
        }
        match reply.target_status {
            Some(TargetLeadStatus::Late) => self.late_target_evaluations += 1,
            Some(TargetLeadStatus::Invalid) => self.invalid_target_evaluations += 1,
            Some(TargetLeadStatus::Clamped) => self.clamped_target_evaluations += 1,
            Some(TargetLeadStatus::Future) | None => {}
        }
        match reply.outcome {
            RenderOutcome::Frame(report) => {
                let report = report?;
                if self.game.world().is_none() {
                    self.world_tick = report.tick;
                } else if report.overload.is_some() {
                    self.reconcile_pause()?;
                }
                if self.last_call.saturating_sub(self.profiler_title_at)
                    >= Duration::from_millis(500)
                    && !self.samples.is_empty()
                {
                    self.refresh_title();
                    self.profiler_title_at = self.last_call;
                }
            }
            RenderOutcome::DeviceLost => self.device_lost(reply.device_generation)?,
            RenderOutcome::Recovered { .. } => {
                unreachable!("replacement handled before frame metadata")
            }
            RenderOutcome::Finish(_) => return Err("unexpected final render reply".into()),
        }
        Ok(())
    }

    fn draw(&mut self) -> Result<(), String> {
        let callback_start = Instant::now();
        let result = self.draw_inner();
        self.callback_walls.push(callback_start.elapsed());
        result
    }

    fn deadline_error(&self) -> String {
        format!(
            "{} presented frames unavailable: presented={}, tick={}, display_link_callbacks={}, render_callbacks={}, unavailable={}, occluded={}, surface_faults={}, pre_acquire_epoch_changes={}, drawable_acquire_attempts={}, device_losses={}, configuration_error={}, dropped_views={}, dropped_stale_views={}, dropped_surface={}{}",
            self.target_frames,
            self.presented,
            self.world_tick,
            self.display_link_callbacks,
            self.callbacks,
            self.unavailable,
            self.occluded,
            self.surface_faults,
            self.pre_acquire_epoch_changes,
            self.drawable_acquire_attempts,
            self.device_losses,
            self.surface_configuration_error.is_some(),
            self.counters.skipped_views,
            self.counters.expired_views,
            self.counters.missed_surfaces,
            never_drawn_hint(self.presented, self.display_link_callbacks)
        )
    }

    fn draw_inner(&mut self) -> Result<(), String> {
        if !self.poll_render()? {
            return Ok(());
        }
        if !self.recovery.ready() {
            self.admit_recovery()?;
            return Ok(());
        }
        self.draw_ready()
    }

    fn poll_render(&mut self) -> Result<bool, String> {
        if self.render_in_flight.is_some() {
            let render = self.render.as_ref().ok_or("render worker missing")?;
            let completed = render.try_recv();
            if let Some(reply) = completed {
                self.receive_render(reply)?;
            } else if render.is_closed() {
                return Err("render worker stopped before replying".into());
            } else {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn draw_ready(&mut self) -> Result<(), String> {
        if self.presented >= self.target_frames
            || !self.edits.wants_frame()
            || !self.pending_display_target
        {
            return Ok(());
        }
        let now = self.origin.elapsed();
        #[cfg(feature = "interface")]
        self.game.interface_tick(now);
        self.last_call = now;
        self.callbacks += 1;
        let window = self.window.as_ref().ok_or("window missing")?;
        if let Some([width, height]) = self
            .transition_proof
            .as_ref()
            .and_then(|proof| proof.arm.window_step(self.callbacks))
        {
            let _ = window.request_inner_size(winit::dpi::LogicalSize::new(width, height));
        }
        let actual_scale = window.scale_factor();
        let backing_scale = self
            .transition_proof
            .as_ref()
            .map_or(actual_scale, |proof| {
                proof.arm.backing_scale(actual_scale, self.callbacks)
            })
            .to_bits();
        let output = self.sample_output()?;
        #[cfg(feature = "interface")]
        if self.game.wants_interface_output() {
            let window = self.window.as_ref().ok_or("window missing")?;
            let metrics = crate::OutputMetrics {
                extent: output.extent(),
                density: actual_scale * (f64::from(output.scale.step.percent()) / 100.0),
                insets: display::safe_area(window),
            };
            if self.interface_output != Some(metrics) {
                self.interface_output = Some(metrics);
                self.edits.carry(self.game.interface_output(metrics));
            }
        }
        self.edits.carry(self.game.frame_edits());
        if let Some(pan) = &mut self.qualification {
            pan.request(
                self.target_sequence,
                self.display_target.ok_or("display target missing")?,
                self.game.controls(),
            )?;
        }
        let target = self.display_target.ok_or("display target missing")?;
        let job = RenderJob::Frame(FrameRequest {
            sequence: self.target_sequence,
            device_generation: self.recovery.active,
            target_media_seconds: target,
            extent: output.backing,
            surface_generation: self.surface_generation.load(Ordering::Acquire),
            platform: FrameEdits {
                profile_revision: crate::profile::revision(self.game.controls()),
                timing_requested: self.timing_requested(),
                backing_scale,
                output,
                mode_held: self.output_mode.held(),
                governor_display: self.governor_display,
                display_tick: self.display_tick,
                edits: self.edits.pending(),
            },
        });
        match self
            .render
            .as_ref()
            .ok_or("render worker missing")?
            .try_send(job)
        {
            Ok(()) => {}
            Err(thrum_render::frame::SendFault::Closed(_)) => {
                return Err("render worker stopped".into());
            }
            Err(thrum_render::frame::SendFault::Busy(_)) => {
                return Err("bounded render worker refused an idle request".into());
            }
        }
        self.render_in_flight = Some(self.edits.submitted());
        if self.run_deadline.is_some_and(|deadline| now >= deadline) {
            return Err(self.deadline_error());
        }
        Ok(())
    }

    fn finish(&mut self) -> Result<String, String> {
        let worker = self.render.as_ref().ok_or("render worker missing")?;
        if self.render_in_flight.is_some() {
            let reply = worker
                .recv_timeout(Duration::from_secs(10))
                .ok_or("render worker did not finish its last frame")?;
            // Closing can cancel the frame in flight after it collected completions from
            // earlier submissions. Keep those completions for the final report.
            if self.window_closed && matches!(reply.outcome, RenderOutcome::Frame(Err(_))) {
                self.render_in_flight = None;
                for mut sample in reply.samples.into_iter().flatten() {
                    sample.detach_gpu_timing();
                    self.keep_sample(sample);
                }
            } else {
                self.receive_render(reply)?;
            }
        }
        self.render
            .as_ref()
            .ok_or("render worker missing")?
            .try_send(RenderJob::Finish)
            .map_err(|_| "render worker refused final drain".to_owned())?;
        let final_reply = self
            .render
            .as_ref()
            .ok_or("render worker missing")?
            .recv_timeout(Duration::from_secs(15))
            .ok_or("render worker did not finish GPU drain")?;
        self.counters = final_reply.platform.counters;
        let final_device_loss = match final_reply.outcome {
            RenderOutcome::Finish(result) => {
                let finished = result?;
                for sample in finished.samples {
                    self.keep_sample(sample);
                }
                finished.lost_device
            }
            RenderOutcome::Frame(_)
            | RenderOutcome::DeviceLost
            | RenderOutcome::Recovered { .. } => {
                return Err("frame reply arrived during final drain".into());
            }
        };
        if let Some(worker) = self.render.take() {
            worker.shutdown();
        }
        if self.presented == 0 {
            return closed_without_frames(
                self.window_closed,
                &self.adapter_name,
                self.callbacks,
                self.display_link_callbacks,
            );
        }
        let wall = percentile(&self.callback_walls, |duration| {
            duration.as_secs_f64() * 1e3
        });
        let worker_wall = percentile(&self.worker_walls, |duration| duration.as_secs_f64() * 1e3);
        let acquire_wall = percentile(&self.acquire_walls, |duration| duration.as_secs_f64() * 1e3);
        let presented: Vec<_> = self
            .samples
            .iter()
            .filter(|sample| sample.surface_presented)
            .collect();
        let missing = recovery::missing_profiles(
            self.presented,
            presented.len() + self.trimmed_presented,
            self.counters.lost_reports,
            self.device_losses != 0 || final_device_loss,
        )?;
        if presented.is_empty() {
            return Ok(format!(
                "adapter={}; presented={}; committed_ticks={}; device_losses={}; world_overloads={}; overload_refused_ticks={}; final_drain_device_lost={final_device_loss}; missing_presented_profiles={missing}; GPU/CPU frame profiles unavailable after device loss",
                self.adapter_name,
                self.presented,
                self.world_tick,
                self.device_losses,
                self.pause.overload_count() + self.counters.inline_overloads.0,
                self.pause.refused_ticks() + self.counters.inline_overloads.1
            ));
        }
        let tick = percentile(&presented, |sample| sample.tick_cpu.as_secs_f64() * 1e6);
        let upload = percentile(&presented, |sample| sample.upload_cpu.as_secs_f64() * 1e6);
        let view = percentile(&presented, |sample| f64::from(sample.view_update_ns) / 1e3);
        let encode = percentile(&presented, |sample| sample.encode_cpu.as_secs_f64() * 1e6);
        let frame_cpu = percentile(&presented, |sample| {
            (sample.upload_cpu + sample.encode_cpu).as_secs_f64() * 1e6
        });
        let gpu = if presented
            .iter()
            .all(|sample| sample.gpu_present_ns.is_some())
        {
            let scene = percentile(&presented, |sample| {
                sample
                    .gpu_pass_ns
                    .as_ref()
                    .map_or(0.0, thrum_render::timing::SceneGpuTimings::sum)
                    / 1e6
            });
            let forward = percentile(&presented, |sample| {
                sample
                    .gpu_pass_ns
                    .as_ref()
                    .map_or(0.0, thrum_render::timing::SceneGpuTimings::forward)
                    / 1e6
            });
            let present = percentile(&presented, |sample| {
                sample.gpu_present_ns.unwrap_or(0.0) / 1e6
            });
            format!(
                "scene GPU p50/p95={:.3}/{:.3} ms; forward GPU p50/p95={:.3}/{:.3} ms; surface GPU p50/p95={:.3}/{:.3} ms",
                scene.0, scene.1, forward.0, forward.1, present.0, present.1
            )
        } else if presented.iter().any(|sample| sample.timing_requested) {
            "GPU timestamps unavailable".to_owned()
        } else {
            "GPU timing off".to_owned()
        };
        // The renderer's culling mode and read-back counts, over the frames
        // whose count had settled when they retired; Direct has none.
        let mode = presented.last().map(|sample| sample.culling_mode);
        let selection = presented.last().map(|sample| sample.culling_selection);
        let counted: Vec<_> = presented
            .iter()
            .filter_map(|sample| sample.camera_culling)
            .collect();
        let mut drawn: Vec<u32> = counted.iter().map(|counts| counts.drawn).collect();
        drawn.sort_unstable();
        let culling = match counted.last() {
            Some(last) => format!(
                "culling={:?} drawn p50/p95={}/{} of {} (counted {} of {} samples)",
                last.mode,
                drawn[drawn.len() / 2],
                drawn[drawn.len() * 95 / 100],
                last.active,
                counted.len(),
                presented.len()
            ),
            None => format!(
                "culling={} with no counts",
                mode.map_or_else(String::new, |mode| format!("{mode:?}"))
            ),
        };
        let shadows = presented.last().map(|sample| sample.shadow_selection);
        let culling =
            format!("{culling}; culling_selection={selection:?}; shadow_selection={shadows:?}");
        Ok(format!(
            "adapter={}; surface={:?} {}x{}; presented={}; callbacks={}; display_link_callbacks={}; late_target_evaluations={}; invalid_target_evaluations={}; clamped_target_evaluations={}; committed_ticks={}; unavailable={}; occluded={}; surface_faults={}; dropped_surface={}; dropped_views={}; dropped_stale_views={}; dropped_reports={}; callback wall p50/p95={:.3}/{:.3} ms; render-worker wall p50/p95={:.3}/{:.3} ms; drawable-acquire wall p50/p95={:.3}/{:.3} ms; tick CPU p50/p95={:.3}/{:.3} us; live-view CPU p50/p95={:.3}/{:.3} us; upload CPU p50/p95={:.3}/{:.3} us; encode CPU p50/p95={:.3}/{:.3} us; upload+encode CPU p50/p95={:.3}/{:.3} us; {gpu}; {culling}; device_losses={}; world_overloads={}; overload_refused_ticks={}; final_drain_device_lost={final_device_loss}; retained_presented_profiles={}; missing_presented_profiles={missing}; profiles describe retained samples",
            self.adapter_name,
            self.surface_format.ok_or("surface format missing")?,
            self.surface_extent[0],
            self.surface_extent[1],
            self.presented,
            self.callbacks,
            self.display_link_callbacks,
            self.late_target_evaluations,
            self.invalid_target_evaluations,
            self.clamped_target_evaluations,
            self.world_tick,
            self.unavailable,
            self.occluded,
            self.surface_faults,
            self.counters.missed_surfaces,
            self.counters.skipped_views,
            self.counters.expired_views,
            self.counters.lost_reports,
            wall.0,
            wall.1,
            worker_wall.0,
            worker_wall.1,
            acquire_wall.0,
            acquire_wall.1,
            tick.0,
            tick.1,
            view.0,
            view.1,
            upload.0,
            upload.1,
            encode.0,
            encode.1,
            frame_cpu.0,
            frame_cpu.1,
            self.device_losses,
            self.pause.overload_count() + self.counters.inline_overloads.0,
            self.pause.refused_ticks() + self.counters.inline_overloads.1,
            presented.len(),
        ))
    }
}

impl<G: WindowGame + 'static> App<G> {
    /// Watch the parameter file and the game's files. A watch that cannot start is reported and
    /// leaves `L`.
    #[expect(
        clippy::print_stderr,
        reason = "the person editing files is told that saves will not reload"
    )]
    fn start_watch(&self, wake: &EventLoopProxy<UserEvent>) -> Option<FileWatch> {
        let files = self
            .params_path
            .iter()
            .map(|path| (path.clone(), Kinds::PARAMS))
            .chain(
                self.game
                    .reload_files()
                    .into_iter()
                    .map(|path| (path, Kinds::GAME)),
            )
            .collect();
        let wake = wake.clone();
        FileWatch::start(
            files,
            file_watch::TICK,
            Box::new(move || {
                let _ = wake.send_event(UserEvent::FilesSaved);
            }),
        )
        .unwrap_or_else(|error| {
            eprintln!(
                "{}: cannot watch files ({error}); press L to reload them",
                self.config.name
            );
            None
        })
    }
}

impl<G: WindowGame + 'static> Reload for App<G> {
    fn stopped(&self) -> bool {
        self.render.is_none() || self.render_stopped.load(Ordering::Acquire)
    }

    fn reload_params(&mut self, saved: bool) -> Result<(), String> {
        self.load_params(saved)
    }

    fn reload_game(&mut self) {
        self.game.reload();
    }

    fn redraw(&self) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

impl<G: Game> Drop for App<G> {
    fn drop(&mut self) {
        // No save may reach a renderer that is shutting down.
        drop(self.watch.take());
        self.render_stopped.store(true, Ordering::Release);
        drop(self.display_link.take());
        if let Some(render) = self.render.take() {
            render.shutdown();
        }
    }
}

fn profiler_title(
    title: &str,
    sample: &FrameSample,
    world_status: &str,
    gpu_status: &str,
) -> String {
    let gpu = sample.gpu_pass_ns.as_ref().map_or_else(
        || if sample.timing_requested { "GPU timestamps unavailable" } else { "GPU timing off" }.to_owned(),
        |passes| {
            let present = sample.gpu_present_ns.unwrap_or(0.0) / 1e6;
            format!(
                "GPU shadow near/far {:.3}/{:.3}, cluster {:.3}, forward {:.3}, history {:.3}, display {:.3}, atmosphere compute {:.3}, present {:.3} ms",
                passes.shadow_near() / 1e6,
                passes.shadow_far() / 1e6,
                passes.clusters() / 1e6,
                passes.forward() / 1e6,
                passes.history() / 1e6,
                passes.tone() / 1e6,
                passes.atmosphere() / 1e6,
                present,
            )
        },
    );
    // The renderer's culling mode and its read-back draw counts, which
    // Direct does not have.
    let culling = sample.camera_culling.map_or_else(
        || format!(" | {:?} culling", sample.culling_mode),
        |counts| {
            format!(
                " | {:?} culling drew {} (prepass {}) of {}",
                counts.mode, counts.drawn, counts.prepass, counts.active
            )
        },
    );
    format!(
        "{title} | {world_status}{gpu_status} | tick {} ({} updates, view age {}) | CPU tick {:.3}, extract {:.3}, upload {:.3}, encode {:.3} ms | {gpu}{culling}",
        sample.tick,
        sample.completed_ticks,
        sample.visual_age_ticks,
        sample.tick_cpu.as_secs_f64() * 1e3,
        sample.extract_cpu.as_secs_f64() * 1e3,
        sample.upload_cpu.as_secs_f64() * 1e3,
        sample.encode_cpu.as_secs_f64() * 1e3,
    )
}

/// The p50 and p95 of `items`, or NaN for both when there are none, so a report over a run
/// that recorded no sample of one kind prints `NaN` instead of indexing an empty vector.
fn percentile<T>(items: &[T], value: impl Fn(&T) -> f64) -> (f64, f64) {
    let mut sorted: Vec<_> = items.iter().map(value).collect();
    sorted.sort_by(f64::total_cmp);
    if sorted.is_empty() {
        return (f64::NAN, f64::NAN);
    }
    (sorted[sorted.len() / 2], sorted[sorted.len() * 95 / 100])
}

/// The outcome of a run that presented no surface frame. A run until closed whose window
/// closed first ends normally, as closing is how it ends, whether the close came before the
/// first display callback or while the first frame was in flight. A bounded run, which never
/// sets `window_closed`, still fails.
fn closed_without_frames(
    window_closed: bool,
    adapter: &str,
    callbacks: usize,
    display_link_callbacks: usize,
) -> Result<String, String> {
    if window_closed {
        Ok(format!(
            "adapter={adapter}; presented=0; callbacks={callbacks}; display_link_callbacks={display_link_callbacks}; the window closed before any surface frame was presented{}",
            never_drawn_hint(0, display_link_callbacks)
        ))
    } else {
        Err("no surface frames were presented".into())
    }
}

impl<G: WindowGame + 'static> ApplicationHandler<UserEvent> for App<G> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let wake = self.wake.clone();
        if let Err(error) = self.prepare(event_loop, &wake) {
            self.error = Some(error);
            event_loop.exit();
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        if self.qualification_report.is_some() || self.render_stopped.load(Ordering::Acquire) {
            event_loop.exit();
            return;
        }
        match event {
            UserEvent::ExtentCaptured => {
                self.run_deadline = None;
                event_loop.exit();
                return;
            }
            UserEvent::ExtentResize([width, height]) => {
                if let Some(window) = &self.window {
                    window.set_min_inner_size(None::<winit::dpi::LogicalSize<u32>>);
                    let _ = window.request_inner_size(winit::dpi::LogicalSize::new(width, height));
                }
            }
            UserEvent::DeviceLost(generation) => {
                let result = self.device_lost(generation).and_then(|()| {
                    if self.poll_render()? && self.presented < self.target_frames {
                        self.admit_recovery()?;
                    }
                    Ok(())
                });
                if let Err(error) = result {
                    self.error = Some(error);
                    event_loop.exit();
                    return;
                }
            }
            UserEvent::DisplayTarget {
                target,
                callback,
                number,
            } => {
                let observed_period = target - callback;
                let fixed_period = self
                    .frame_rate
                    .filter(|rate| rate.fixed)
                    .map(|rate| 1.0 / f64::from(rate.preferred));
                let period = fixed_period
                    .or(self.refresh_period)
                    .unwrap_or(observed_period);
                {
                    let mut account = self.refresh_account.lock().expect("refresh account lock");
                    account.set_fixed_rate(fixed_period.is_some());
                    account.callback(target, period);
                }
                self.display_tick = number;
                self.governor_display.interval =
                    Duration::try_from_secs_f64(fixed_period.unwrap_or(observed_period))
                        .unwrap_or_default();
                self.profile.set_available(self.recovery.ready());
                if let Ok(timestamp) = Duration::try_from_secs_f64(callback) {
                    self.profile.callback(timestamp);
                }
                self.profile.publish(
                    self.game.controls_mut(),
                    self.origin.elapsed(),
                    self.display_link_callbacks as u64,
                    self.presented as u64,
                    false,
                );
                self.sync_panel();
                if let Some(pan) = &mut self.qualification {
                    let result = self
                        .panel
                        .as_mut()
                        .ok_or_else(|| "panel missing".to_owned())
                        .and_then(|panel| pan.step(panel, self.game.controls_mut()));
                    match result {
                        Ok(done) => {
                            if let Some((start, length)) = pan.measured_interval() {
                                self.refresh_account
                                    .lock()
                                    .expect("refresh account lock")
                                    .measure(start, length);
                            }
                            if let Err(error) = self.controls_edited() {
                                self.error = Some(error);
                                event_loop.exit();
                                return;
                            }
                            if done {
                                self.qualification_report = Some(self.finish());
                                event_loop.exit();
                                return;
                            }
                        }
                        Err(error) => {
                            self.error = Some(error);
                            event_loop.exit();
                            return;
                        }
                    }
                }
                self.display_target = Some(target);
                self.target_sequence = self.target_sequence.saturating_add(1);
                self.refresh_display_record();
                if let Some(ticks) = &mut self.transition_ticks
                    && let Err(error) = ticks.record(
                        self.target_sequence,
                        number,
                        callback,
                        target,
                        self.render_in_flight.is_some(),
                    )
                {
                    self.error = Some(error.to_string());
                    event_loop.exit();
                    return;
                }
                self.display_link_callbacks += 1;
                self.pending_display_target = true;
                self.edits.request();
            }
            UserEvent::DisplayParametersChanged => {
                self.refresh_monitor_period();
                self.refresh_display_record();
            }
            UserEvent::ThermalState(state) => {
                self.governor_display.thermal = state;
                self.thermal_states[state as usize] = true;
            }
            UserEvent::WorldAdvanced => {
                if let Err(error) = self.reconcile_pause() {
                    self.error = Some(error);
                    event_loop.exit();
                }
                return;
            }
            UserEvent::ControlsEdited => {
                if let Some(panel) = &mut self.panel {
                    panel.apply_edits(self.game.controls_mut());
                    for refused in panel.take_refused() {
                        #[expect(
                            clippy::print_stderr,
                            reason = "a refused panel edit is for the person typing in the panel"
                        )]
                        {
                            eprintln!("{}: control edit refused: {refused}", self.config.name);
                        }
                    }
                }
                if let Err(error) = self.controls_edited() {
                    self.error = Some(error);
                    event_loop.exit();
                }
                return;
            }
            UserEvent::FilesSaved => {
                let kinds = self
                    .watch
                    .as_ref()
                    .map_or_else(Default::default, FileWatch::take);
                if let Err(error) = file_watch::deliver(self, kinds, true) {
                    self.error = Some(error);
                    event_loop.exit();
                }
                return;
            }
            UserEvent::RenderDone => {
                let result = self.poll_render().and_then(|idle| {
                    if idle && self.presented < self.target_frames && !self.recovery.ready() {
                        self.admit_recovery()?;
                    }
                    Ok(())
                });
                if let Err(error) = result {
                    self.error = Some(error);
                    event_loop.exit();
                    return;
                }
            }
        }
        if self.presented >= self.target_frames {
            event_loop.exit();
            return;
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if self.qualification_report.is_some() || self.render_stopped.load(Ordering::Acquire) {
            return;
        }
        if self.window.as_ref().is_none_or(|window| window.id() != id) {
            return;
        }
        #[cfg(feature = "interface")]
        if self.interface_script.is_some()
            && matches!(
                event,
                WindowEvent::KeyboardInput { .. }
                    | WindowEvent::ModifiersChanged(_)
                    | WindowEvent::CursorMoved { .. }
                    | WindowEvent::CursorLeft { .. }
                    | WindowEvent::MouseInput { .. }
                    | WindowEvent::Touch(_)
            )
        {
            // Scripted contacts already enter these same input handlers.
            // Keep unrelated desk input out of the qualification record.
            return;
        }
        let result = match event {
            WindowEvent::CloseRequested | WindowEvent::Destroyed => {
                self.render_stopped.store(true, Ordering::Release);
                if self.run_deadline.is_none() {
                    self.window_closed = true;
                } else {
                    self.closed_by_user |=
                        self.qualification_report.is_none() && self.presented < self.target_frames;
                }
                let result = self.invalidate_surface();
                event_loop.exit();
                result
            }
            WindowEvent::Moved(_) => {
                self.refresh_monitor_period();
                Ok(())
            }
            WindowEvent::Resized(size) => {
                self.refresh_monitor_period();
                self.sample_output()
                    .and_then(|_| self.record_held_backing([size.width, size.height]))
            }
            WindowEvent::Occluded(occluded) => self.observe_occlusion(occluded),
            WindowEvent::RedrawRequested => {
                let result = self.draw();
                if self.presented >= self.target_frames {
                    event_loop.exit();
                }
                result
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key
                    && let Some(code) = KeyCode::from_winit(code)
                {
                    if code == KeyCode::KeyR && !self.recovery.ready() {
                        if self.recovery.failed()
                            && event.state == ElementState::Pressed
                            && !event.repeat
                        {
                            self.recovery.retry().and_then(|()| {
                                self.refresh_title();
                                if let Some(window) = &self.window {
                                    window.request_redraw();
                                }
                                self.admit_recovery()
                            })
                        } else {
                            Ok(())
                        }
                    } else if code == KeyCode::KeyR && self.surface_configuration_error.is_some() {
                        if event.state == ElementState::Pressed && !event.repeat {
                            self.invalidate_surface()
                        } else {
                            Ok(())
                        }
                    } else if self.window_interface_input() {
                        #[cfg(feature = "interface")]
                        let result = self.interface_key(code, event.state, event.repeat, false);
                        #[cfg(not(feature = "interface"))]
                        let result = Ok(());
                        result
                    } else if code == KeyCode::KeyP && self.game.world().is_some() {
                        if event.state == ElementState::Pressed && !event.repeat {
                            self.pause.request(!self.pause.desired());
                            self.game.observe_paused(self.pause.desired());
                            self.sync_panel();
                            self.reconcile_pause()
                        } else {
                            Ok(())
                        }
                    } else {
                        self.set_key(code, event.state == ElementState::Pressed)
                    }
                } else {
                    Ok(())
                }
            }
            #[cfg(feature = "interface")]
            WindowEvent::ModifiersChanged(modifiers) if self.game.wants_interface_input() => {
                self.interface_input.modifiers = modifiers.state();
                Ok(())
            }
            #[cfg(feature = "interface")]
            WindowEvent::CursorMoved { position, .. } if self.game.wants_interface_input() => {
                self.interface_cursor([position.x, position.y])
            }
            #[cfg(feature = "interface")]
            WindowEvent::CursorLeft { .. } if self.game.wants_interface_input() => {
                self.interface_cursor_left()
            }
            #[cfg(feature = "interface")]
            WindowEvent::MouseInput { state, button, .. } if self.game.wants_interface_input() => {
                self.interface_mouse(button, state)
            }
            #[cfg(feature = "interface")]
            WindowEvent::Touch(touch) if self.game.wants_interface_input() => {
                self.interface_touch(touch)
            }
            #[cfg(feature = "interface")]
            WindowEvent::Focused(false) if self.game.wants_interface_input() => {
                self.interface_input = interface::WindowInput::default();
                self.interface_event(thrum_ui::Event::Interrupted)
            }
            WindowEvent::Focused(false) => self
                .game
                .focus_lost()
                .map(|()| self.input_changed())
                .map_err(|error| error.to_string()),
            _ => Ok(()),
        };
        if let Err(error) = result {
            self.error = Some(error);
            event_loop.exit();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.render_stopped.load(Ordering::Acquire) {
            return;
        }
        let now = self.origin.elapsed();
        self.profile.publish(
            self.game.controls_mut(),
            now,
            self.display_link_callbacks as u64,
            self.presented as u64,
            false,
        );
        self.sync_panel();
        if let Some(recording) = &mut self.profile_recording
            && let Err(error) = recording.observe(
                now,
                &self.samples,
                self.game.controls(),
                self.panel.as_ref(),
                [
                    self.display_link_callbacks as u64,
                    self.callbacks as u64,
                    self.presented as u64,
                ],
            )
        {
            self.error = Some(error);
            event_loop.exit();
        }
        let mode_changed = self
            .window_mode
            .as_ref()
            .is_some_and(window_mode::WindowMode::has_pending)
            || self
                .output_mode
                .deadline()
                .is_some_and(|deadline| now >= deadline);
        if mode_changed && let Err(error) = self.sample_output() {
            self.error = Some(error);
            event_loop.exit();
        }
        let refresh = (self.profile.is_enabled() || self.profile_recording.is_some())
            .then_some(now + Duration::from_millis(250))
            .into_iter()
            .chain(self.output_mode.deadline())
            .min();
        event_loop.set_control_flow(refresh.map_or(ControlFlow::Wait, |at| {
            ControlFlow::WaitUntil(self.origin + at)
        }));
        if self.window.is_some()
            && self.render.is_some()
            && let Some(deadline) = self.run_deadline
        {
            if self.origin.elapsed() >= deadline {
                self.error = Some(self.deadline_error());
                event_loop.exit();
            } else {
                event_loop.set_control_flow(ControlFlow::WaitUntil(
                    self.origin + refresh.map_or(deadline, |at| at.min(deadline)),
                ));
            }
        }
    }
}

/// Run `game` in one window until it closes, then print the frames report.
/// Setting `config.frames_variable` ends the run after that many frames present.
/// Setup, GPU and world failures are reported on standard error and exit with a nonzero status.
/// Open the frame profiler or set [`REPORT_VARIABLE`] to `1` to collect GPU timestamps.
///
/// # Panics
/// If the platform refuses to create the event loop, as on a second call in one process.
pub fn run<G: WindowGame + 'static, E: std::error::Error>(
    config: Config,
    game: impl FnOnce() -> Result<G, E>,
) {
    run_with(config, RunMode::Ordinary, None, None, None, None, game);
}

/// Run the ordinary window with a caller-supplied control qualification observer.
/// # Panics
/// If the platform refuses to create the event loop.
pub fn run_with_qualification<G: WindowGame + 'static, E: std::error::Error>(
    config: Config,
    observer: WindowQualification,
    game: impl FnOnce() -> Result<G, E>,
) {
    run_with_qualification_and_display(config, observer, None, game);
}

/// Run the control qualification observer with an optional display override.
/// # Panics
/// If the platform refuses to create the event loop.
pub fn run_with_qualification_and_display<G: WindowGame + 'static, E: std::error::Error>(
    config: Config,
    observer: WindowQualification,
    display: Option<WindowDisplayOptions>,
    game: impl FnOnce() -> Result<G, E>,
) {
    let mode = if observer.refresh_measurement.is_some() {
        RunMode::Ordinary
    } else {
        RunMode::Bounded
    };
    run_with(config, mode, None, None, Some(observer), display, game);
}

/// Run the ordinary window and retain one submitted image with its extent witnesses.
/// # Panics
/// If the platform refuses to create the event loop, as on a second call in one process.
pub fn run_with_extent_capture<G: WindowGame + 'static, E: std::error::Error>(
    config: Config,
    capture: WindowExtentCapture,
    game: impl FnOnce() -> Result<G, E>,
) {
    run_with(
        config,
        RunMode::Bounded,
        Some(capture),
        None,
        None,
        None,
        game,
    );
}

/// Run the ordinary window with a scripted resize and retain the frame records.
/// # Panics
/// If the platform refuses to create the event loop.
pub fn run_with_transition_proof<G: WindowGame + 'static, E: std::error::Error>(
    config: Config,
    proof: WindowTransitionProof,
    display: Option<WindowDisplayOptions>,
    game: impl FnOnce() -> Result<G, E>,
) {
    run_with(
        config,
        RunMode::Bounded,
        None,
        Some(proof),
        None,
        display,
        game,
    );
}

/// Run `game` in one window until the person closes it. Closing the window is success. Setting
/// `config.frames_variable` bounds the run exactly as [`run`] does, for scripted checks, and
/// setting [`REPORT_VARIABLE`] to `1` prints the frames report over a bounded window of
/// recent samples at the end. Failures are reported on standard error and exit the process
/// with a nonzero status.
///
/// # Panics
/// If the platform refuses to create the event loop, as on a second call in one process.
pub fn run_until_closed<G: WindowGame + 'static, E: std::error::Error>(
    config: Config,
    game: impl FnOnce() -> Result<G, E>,
) {
    run_with(
        config,
        RunMode::QuietUntilClosed,
        None,
        None,
        None,
        None,
        game,
    );
}

/// Run the window with a typed display override and optional qualification.
///
/// # Panics
/// If the platform refuses to create the event loop.
pub fn run_with_display<G: WindowGame + 'static, E: std::error::Error>(
    config: Config,
    options: WindowDisplayOptions,
    game: impl FnOnce() -> Result<G, E>,
) {
    run_with(
        config,
        RunMode::Bounded,
        None,
        None,
        None,
        Some(options),
        game,
    );
}

/// Set to `1` to collect GPU timestamps throughout a window run and print the frames report.
/// [`run`] always prints a frames report.
pub const REPORT_VARIABLE: &str = "THRUM_REPORT";

#[expect(
    clippy::print_stdout,
    reason = "the bounded diagnostic reports its measured surface frame"
)]
#[expect(
    clippy::print_stderr,
    reason = "the diagnostic reports setup or GPU faults"
)]
fn run_with<G: WindowGame + 'static, E: std::error::Error>(
    config: Config,
    mode: RunMode,
    extent_capture: Option<WindowExtentCapture>,
    transition_proof: Option<WindowTransitionProof>,
    qualification: Option<WindowQualification>,
    display_options: Option<WindowDisplayOptions>,
    game: impl FnOnce() -> Result<G, E>,
) {
    let target_frames = match frame_limit(
        config.frames_variable,
        std::env::var(config.frames_variable).ok().as_deref(),
        !matches!(mode, RunMode::Bounded),
    ) {
        Ok(frames) => frames,
        Err(error) => {
            eprintln!("{}: {error}", config.name);
            std::process::exit(2);
        }
    };
    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .expect("macOS event loop");
    let mut app = match game() {
        Ok(game) => App::new(config, event_loop.create_proxy(), target_frames, game),
        Err(error) => {
            eprintln!("{}: {error}", config.name);
            std::process::exit(2);
        }
    };
    app.extent_capture = extent_capture;
    app.transition_proof = transition_proof;
    app.qualification = qualification.map(qualification::Pan::new);
    app.display_options = display_options;
    let requested_report = std::env::var(REPORT_VARIABLE).ok();
    app.timing_policy = if requested_report.as_deref() == Some("1") {
        TimingPolicy::Report
    } else {
        TimingPolicy::Profiler
    };
    if let Err(error) = event_loop.run_app(&mut app) {
        app.error = Some(format!("event loop: {error}"));
    }
    if app.closed_by_user && app.error.is_none() {
        app.error = Some(format!(
            "presentation cancelled after {}/{} requested frames",
            app.presented, app.target_frames
        ));
    }
    if let Some(pan) = &app.qualification
        && let Err(error) = pan.finish()
    {
        app.error = Some(error);
    }
    let mut result = app.error.take().map_or_else(
        || {
            app.qualification_report
                .take()
                .unwrap_or_else(|| app.finish())
        },
        Err,
    );
    if result.is_ok() {
        let mut account = app.refresh_account.lock().expect("refresh account lock");
        account.finish();
        if prints_report(mode, requested_report.as_deref()) {
            println!("{}", account.counts());
        }
        let account_output = app
            .qualification
            .as_ref()
            .map(|observer| &observer.options.output)
            .or_else(|| {
                app.transition_proof
                    .as_ref()
                    .filter(|proof| proof.arm == TransitionArm::Fullscreen)
                    .map(|proof| &proof.output)
            });
        if let Some(output) = account_output {
            if let Err(error) = account.write(output) {
                result = Err(format!("refresh account: {error}"));
            }
            if let Some(rate) = app.frame_rate {
                let record = serde_json::json!({
                    "preferred_hz": rate.preferred, "fixed": rate.fixed, "forced": rate.forced,
                });
                if let Err(error) =
                    std::fs::write(output.join("requested-frame-rate.json"), record.to_string())
                {
                    result = Err(format!("frame rate receipt: {error}"));
                }
            }
        }
    }
    if let Some(observer) = &app.qualification {
        let output = &observer.options.output;
        let [nominal, fair, serious, critical] = app.thermal_states;
        if let Err(error) = std::fs::write(
            output.join("refresh-window.txt"),
            format!(
                "occluded_window_events={}\noccluded_surface_attempts={}\nthermal_nominal={nominal}\nthermal_fair={fair}\nthermal_serious={serious}\nthermal_critical={critical}\n",
                app.window_occlusions, app.occluded
            ),
        ) && result.is_ok()
        {
            result = Err(format!("refresh window: {error}"));
        }
        let samples = if observer.options.refresh_measurement.is_some() {
            qualification::write_cadence_samples(output, &app.samples)
        } else {
            std::fs::write(
                output.join("gpu-samples.txt"),
                format!("{:#?}\n", app.samples),
            )
        };
        if let Err(error) = samples {
            eprintln!("{}: sample record: {error}", app.config.name);
            if observer.options.refresh_measurement.is_some() && result.is_ok() {
                result = Err(format!("cadence samples: {error}"));
            }
        }
    }
    match result {
        Ok(report) => {
            if prints_report(mode, requested_report.as_deref()) {
                println!("{report}");
            }
        }
        Err(error) => {
            eprintln!("{}: {error}", app.config.name);
            drop(app);
            std::process::exit(1);
        }
    }
}

/// The ordinary window and proofs print their reports. Starter windows report on request.
fn prints_report(mode: RunMode, requested: Option<&str>) -> bool {
    !matches!(mode, RunMode::QuietUntilClosed) || requested == Some("1")
}

/// The run's presented-frame bound, or `None` to run until the window closes.
fn frame_limit(
    variable: &str,
    value: Option<&str>,
    until_closed: bool,
) -> Result<Option<usize>, String> {
    if until_closed && value.is_none() {
        return Ok(None);
    }
    run_frames(variable, value).map(Some)
}

fn run_frames(variable: &str, value: Option<&str>) -> Result<usize, String> {
    let Some(value) = value else {
        return Ok(DEFAULT_FRAMES);
    };
    let frames = value
        .parse::<usize>()
        .map_err(|_| format!("{variable} must be a positive integer"))?;
    if !(1..=MAX_FRAMES).contains(&frames) {
        return Err(format!("{variable} must be between 1 and {MAX_FRAMES}"));
    }
    Ok(frames)
}

/// The likely cause when the display never asked for a frame, which the counters alone do not say.
fn never_drawn_hint(presented: usize, display_link_callbacks: usize) -> &'static str {
    if presented == 0 && display_link_callbacks == 0 {
        "; the display never asked for a frame, so the window drew nothing. A locked or sleeping screen, or a \
         window on no display, does this; review frames offscreen with THRUM_CAPTURE_PNG instead"
    } else {
        ""
    }
}

#[cfg(test)]
mod tests {
    use super::{
        RunMode, closed_without_frames, frame_limit, never_drawn_hint, percentile, prints_report,
        run_frames,
    };

    #[test]
    fn percentiles_of_no_samples_are_nan_not_a_panic() {
        let none: [f64; 0] = [];
        let (p50, p95) = percentile(&none, |value| *value);
        assert!(p50.is_nan() && p95.is_nan());
        assert_eq!(percentile(&[3.0, 1.0, 2.0], |value| *value), (2.0, 3.0));
    }

    #[test]
    fn a_window_closed_before_any_present_ends_a_run_until_closed_normally() {
        // Closed before the first display callback: nothing was timed at all.
        let before_callback = closed_without_frames(true, "GPU", 0, 0).expect("normal close");
        assert!(before_callback.contains("presented=0; callbacks=0; display_link_callbacks=0"));
        assert!(before_callback.contains("locked or sleeping screen"));
        // Closed while the first frame was in flight: callbacks ran, but no frame reply was
        // received, so the worker and acquire timings are empty.
        let before_reply = closed_without_frames(true, "GPU", 1, 2).expect("normal close");
        assert!(before_reply.contains("callbacks=1; display_link_callbacks=2"));
        assert!(!before_reply.contains("locked or sleeping screen"));
    }

    #[test]
    fn a_bounded_run_that_never_presents_still_fails() {
        assert_eq!(
            closed_without_frames(false, "GPU", 5, 5),
            Err("no surface frames were presented".to_owned())
        );
        assert!(closed_without_frames(false, "GPU", 0, 0).is_err());
    }

    #[test]
    fn a_window_the_display_never_drove_says_why_it_drew_nothing() {
        assert!(never_drawn_hint(0, 0).contains("locked or sleeping screen"));
        assert!(never_drawn_hint(0, 0).contains("THRUM_CAPTURE_PNG"));
        // Once the display drives the window, the counters explain the shortfall.
        assert_eq!(never_drawn_hint(0, 3), "");
        assert_eq!(never_drawn_hint(5, 0), "");
    }

    #[test]
    fn frame_count_for_present_qualification_is_bounded() {
        let run_frames = |value| run_frames("DEMO_FRAME_LIMIT", value);
        assert_eq!(run_frames(None).expect("default"), 600);
        assert_eq!(run_frames(Some("75000")).expect("ten minute run"), 75_000);
        assert_eq!(
            run_frames(Some("0")),
            Err("DEMO_FRAME_LIMIT must be between 1 and 100000".to_owned())
        );
        assert!(run_frames(Some("100001")).is_err());
        assert_eq!(
            run_frames(Some("nan")),
            Err("DEMO_FRAME_LIMIT must be a positive integer".to_owned())
        );
    }

    #[test]
    fn a_run_until_closed_is_bounded_only_when_its_variable_is_set() {
        assert!(
            prints_report(RunMode::Bounded, None),
            "a proof run always reports"
        );
        assert!(
            !prints_report(RunMode::QuietUntilClosed, None),
            "a closed window reports only on request"
        );
        assert!(!prints_report(RunMode::QuietUntilClosed, Some("0")));
        assert!(prints_report(RunMode::QuietUntilClosed, Some("1")));
        assert!(prints_report(RunMode::Ordinary, None));
        assert_eq!(frame_limit("THRUM_STARTER_FRAMES", None, true), Ok(None));
        assert_eq!(
            frame_limit("THRUM_STARTER_FRAMES", Some("90"), true),
            Ok(Some(90))
        );
        assert!(frame_limit("THRUM_STARTER_FRAMES", Some("0"), true).is_err());
        assert_eq!(frame_limit("DEMO_FRAME_LIMIT", None, false), Ok(Some(600)));
    }
}
