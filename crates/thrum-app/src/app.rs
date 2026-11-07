//! The platform owns the surface and event loop; the game's runtime host owns world
//! ticks, GPU scene packets and the prepared presentation pass.

mod device;
mod display_link;
mod edit_flow;
mod pause;
mod recovery;
mod render_actor;

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use device::DeviceRecipe;
use display_link::ViewDisplayLink;
use edit_flow::EditFlow;
use pause::PauseIntent;
use recovery::Recovery;
use render_actor::{
    FrameCounters, FrameRequest, RecoveryFailure, RenderJob, RenderOutcome, RenderReply,
    RenderWorker, SurfaceStatus,
};
use thrum_controls::macos::MacPanel;
use thrum_core::sim::WorldLaneFault;
use thrum_render::{frame::TargetLeadStatus, present::SurfaceProfile, runtime::FrameSample};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use crate::{Config, Game, Gpu, KeyResponse};

const DEFAULT_FRAMES: usize = 600;
const MAX_FRAMES: usize = 100_000;
/// Entries each profile history keeps in a run until closed; the oldest half is dropped
/// when one exceeds it, so a long session holds a bounded recent window.
const RETAINED_PROFILES: usize = 36_000;
/// The first device's generation; recovery owns later ones.
const DEVICE_GENERATION: u64 = 1;

#[derive(Clone, Copy, Debug)]
enum UserEvent {
    WorldAdvanced,
    DisplayTarget(f64),
    RenderDone,
    DeviceLost(u64),
    ControlsEdited,
}

struct App<G: Game> {
    config: Config,
    wake: EventLoopProxy<UserEvent>,
    display_link: Option<ViewDisplayLink>,
    window: Option<Arc<Window>>,
    display_target: Option<f64>,
    target_sequence: u64,
    display_link_callbacks: usize,
    late_target_evaluations: usize,
    invalid_target_evaluations: usize,
    clamped_target_evaluations: usize,
    pending_display_target: bool,
    surface_format: Option<wgpu::TextureFormat>,
    surface_extent: [u32; 2],
    observed_extent: [u32; 2],
    surface_generation: Arc<AtomicU64>,
    render_stopped: Arc<AtomicBool>,
    render: Option<RenderWorker<G>>,
    pause: PauseIntent,
    recovery: Recovery,
    recovery_error: Option<String>,
    device_losses: u64,
    world_tick: u64,
    render_in_flight: Option<u64>,
    counters: FrameCounters,
    /// The game's controls are shared by keys and the panel. Edits travel to the render
    /// owner in the next frame request.
    game: G,
    panel: Option<MacPanel>,
    edits: EditFlow<G::Edits>,
    /// `THRUM_PARAMS`: a parameter file applied once the window is up and again on `L`.
    params_path: Option<std::path::PathBuf>,
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
    callback_walls: Vec<Duration>,
    worker_walls: Vec<Duration>,
    acquire_walls: Vec<Duration>,
    samples: Vec<FrameSample>,
    profiler_title_at: Duration,
    adapter_name: String,
    error: Option<String>,
    closed_by_user: bool,
}

impl<G: Game + 'static> App<G> {
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
            config,
            wake,
            display_link: None,
            window: None,
            display_target: None,
            target_sequence: 0,
            display_link_callbacks: 0,
            late_target_evaluations: 0,
            invalid_target_evaluations: 0,
            clamped_target_evaluations: 0,
            pending_display_target: false,
            surface_format: None,
            surface_extent: [0, 0],
            observed_extent: [0, 0],
            surface_generation: Arc::new(AtomicU64::new(1)),
            render_stopped: Arc::new(AtomicBool::new(false)),
            render: None,
            pause: PauseIntent::default(),
            recovery: Recovery::new(DEVICE_GENERATION),
            recovery_error: None,
            device_losses: 0,
            world_tick: 0,
            render_in_flight: None,
            counters: FrameCounters::default(),
            game,
            panel: None,
            edits: EditFlow::default(),
            params_path: std::env::var_os("THRUM_PARAMS").map(std::path::PathBuf::from),
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
            callback_walls: Vec::with_capacity(reserved * 8),
            worker_walls: Vec::with_capacity(reserved * 4),
            acquire_walls: Vec::with_capacity(reserved * 4),
            samples: Vec::with_capacity(reserved * 2),
            profiler_title_at: Duration::ZERO,
            adapter_name: String::new(),
            error: None,
            closed_by_user: false,
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

    fn observe_extent(&mut self, extent: [u32; 2]) -> Result<(), String> {
        if extent != self.observed_extent {
            self.invalidate_surface()?;
            self.observed_extent = extent;
        }
        Ok(())
    }

    /// Take edits from keys, the panel or a parameter file. Presentation edits ride the next
    /// frame request; the pause toggle takes the same path as the P key.
    fn controls_edited(&mut self) -> Result<(), String> {
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
    fn load_params(&mut self) -> Result<(), String> {
        let Some(path) = self.params_path.clone() else {
            return Ok(());
        };
        let applied = std::fs::read_to_string(&path)
            .map_err(|error| error.to_string())
            .and_then(|text| {
                self.game
                    .controls_mut()
                    .apply_file(&text)
                    .map_err(|error| error.to_string())
            });
        match applied {
            Ok(report) => {
                eprintln!("{}: {}: {report}", self.config.name, path.display());
                for problem in &report.problems {
                    eprintln!("  {problem}");
                }
                self.controls_edited()
            }
            Err(error) => {
                eprintln!(
                    "{}: {}: {error}; controls keep their values (fix the file and press L)",
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
            self.pause.acknowledge(reply.request)?;
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
            self.pause.label()
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
            } else {
                title
            };
            window.set_title(&title);
        }
    }

    fn device_lost(&mut self, generation: u64) -> Result<(), String> {
        if self.recovery.lost(generation)? {
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
        let size = self.window.as_ref().ok_or("window missing")?.inner_size();
        let job = RenderJob::Recover {
            from_generation: self.recovery.active,
            generation,
            extent: [size.width, size.height],
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
            self.load_params()?;
            self.game.reload();
            if let Some(window) = &self.window {
                window.request_redraw();
            }
            return Ok(());
        }
        match self.game.key(code, pressed)? {
            KeyResponse::Ignored => Ok(()),
            KeyResponse::Edited => self.controls_edited(),
            KeyResponse::Input => {
                self.input_changed();
                Ok(())
            }
        }
    }

    fn prepare(
        &mut self,
        event_loop: &ActiveEventLoop,
        wake: &EventLoopProxy<UserEvent>,
    ) -> Result<(), String> {
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title(self.config.window_title)
                        .with_inner_size(winit::dpi::PhysicalSize::new(
                            self.config.window_size[0],
                            self.config.window_size[1],
                        )),
                )
                .map_err(|error| format!("window: {error}"))?,
        );
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.backends = wgpu::Backends::METAL;
        let instance = wgpu::Instance::new(descriptor);
        let surface = Arc::new(
            instance
                .create_surface(Arc::clone(&window))
                .map_err(|error| format!("surface: {error}"))?,
        );
        let size = window.inner_size();
        let recipe = DeviceRecipe { instance, surface };
        let gpu =
            pollster::block_on(recipe.create(DEVICE_GENERATION, [size.width, size.height], wake))?;
        if gpu.watch.is_lost() {
            return Err("initial device lost before preparation".to_owned());
        }
        recipe.surface.configure(&gpu.device, &gpu.config);
        if gpu.watch.is_lost() {
            return Err("initial device lost during surface configuration".to_owned());
        }
        let world_wake = wake.clone();
        let prepared = self.game.prepare(
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
        )?;
        if gpu.watch.is_lost() {
            return Err("initial device lost during preparation".to_owned());
        }
        self.origin = prepared.origin;
        let host = prepared.host;
        self.counters = FrameCounters::read(&host);
        self.window = Some(window);
        let panel_wake = wake.clone();
        let panel = MacPanel::new(
            self.game.controls(),
            self.config.panel_title,
            Some(Box::new(move || {
                let _ = panel_wake.send_event(UserEvent::ControlsEdited);
            })),
        )?;
        panel.show(None);
        self.panel = Some(panel);
        self.display_link = Some(ViewDisplayLink::start(
            self.window.as_ref().ok_or("window missing")?,
            wake.clone(),
        )?);
        self.adapter_name.clone_from(&gpu.adapter_name);
        self.surface_format = Some(gpu.config.format);
        self.surface_extent = [gpu.config.width, gpu.config.height];
        self.observed_extent = self.surface_extent;
        self.render = Some(render_actor::start::<G>(
            self.config.render_thread,
            host,
            gpu,
            recipe,
            Arc::clone(&self.surface_generation),
            Arc::clone(&self.render_stopped),
            self.origin,
            wake,
        )?);
        self.load_params()?;
        self.game.reload();
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

    fn receive_render(&mut self, reply: RenderReply<G::Readback>) -> Result<(), String> {
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
            self.counters = reply.counters;
            self.samples.extend(reply.samples.into_iter().flatten());
            match result {
                Ok(info) => {
                    self.adapter_name = info.adapter_name;
                    self.surface_format = Some(info.format);
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
                Err(RecoveryFailure::Unavailable(error)) => self.recovery_error = Some(error),
                Err(RecoveryFailure::Fatal(error)) => return Err(error),
            }
            self.refresh_title();
            return Ok(());
        }
        if reply.device_generation != self.recovery.active {
            return Ok(());
        }
        let request = self.render_in_flight.take();
        self.counters = reply.counters;
        self.samples.extend(reply.samples.into_iter().flatten());
        self.surface_extent = reply.surface_extent;
        if request.is_some_and(|request| self.edits.admits(request)) {
            self.game.observe(reply.readback);
        }
        self.sync_panel();
        self.worker_walls.push(reply.worker_wall);
        if let Some(acquire_wall) = reply.acquire_wall {
            self.acquire_walls.push(acquire_wall);
        }
        match reply.surface_status {
            SurfaceStatus::Presented => {
                self.presented += 1;
                if reply.sequence == self.target_sequence {
                    self.pending_display_target = false;
                }
            }
            SurfaceStatus::NotAttempted => {}
            SurfaceStatus::Unavailable => self.unavailable += 1,
            SurfaceStatus::Occluded => {
                self.unavailable += 1;
                self.occluded += 1;
            }
            SurfaceStatus::NeedsReconfigure => self.surface_faults += 1,
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
                if report.overload.is_some() {
                    self.reconcile_pause()?;
                }
                if self.last_call.saturating_sub(self.profiler_title_at)
                    >= Duration::from_millis(500)
                    && let (Some(window), Some(sample)) = (&self.window, self.samples.last())
                {
                    window.set_title(&profiler_title(
                        self.config.title,
                        sample,
                        self.world_status(),
                        self.recovery.label(),
                    ));
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
            "{} presented frames unavailable: presented={}, tick={}, display_link_callbacks={}, render_callbacks={}, unavailable={}, occluded={}, surface_faults={}, dropped_views={}, dropped_stale_views={}, dropped_surface={}",
            self.target_frames,
            self.presented,
            self.world_tick,
            self.display_link_callbacks,
            self.callbacks,
            self.unavailable,
            self.occluded,
            self.surface_faults,
            self.counters.skipped_views,
            self.counters.expired_views,
            self.counters.missed_surfaces
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
        self.last_call = now;
        self.callbacks += 1;
        let size = self.window.as_ref().ok_or("window missing")?.inner_size();
        let extent = [size.width, size.height];
        self.observe_extent(extent)?;
        self.edits.carry(self.game.frame_edits());
        let job = RenderJob::Frame(FrameRequest {
            sequence: self.target_sequence,
            device_generation: self.recovery.active,
            target_media_seconds: self.display_target.ok_or("display target missing")?,
            extent,
            surface_generation: self.surface_generation.load(Ordering::Acquire),
            edits: self.edits.pending(),
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
            // Closing the window stops the render owner, which may refuse the frame in flight;
            // that refusal ends a run until closed normally.
            if self.window_closed && matches!(reply.outcome, RenderOutcome::Frame(Err(_))) {
                self.render_in_flight = None;
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
        self.counters = final_reply.counters;
        let final_device_loss = match final_reply.outcome {
            RenderOutcome::Finish(result) => {
                let finished = result?;
                self.samples.extend(finished.samples);
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
        if self.presented == 0 {
            return Err("no surface frames were presented".into());
        }
        let missing = recovery::missing_profiles(
            self.presented,
            presented.len() + self.trimmed_presented,
            self.device_losses != 0 || final_device_loss,
        )?;
        if presented.is_empty() {
            return Ok(format!(
                "adapter={}; presented={}; committed_ticks={}; device_losses={}; world_overloads={}; overload_refused_ticks={}; final_drain_device_lost={final_device_loss}; missing_presented_profiles={missing}; GPU/CPU frame profiles unavailable after device loss",
                self.adapter_name,
                self.presented,
                self.world_tick,
                self.device_losses,
                self.pause.overload_count(),
                self.pause.refused_ticks()
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
                sample.gpu_pass_ns.unwrap_or([0.0; 11]).iter().sum::<f64>() / 1e6
            });
            let present = percentile(&presented, |sample| {
                sample.gpu_present_ns.unwrap_or(0.0) / 1e6
            });
            format!(
                "scene GPU p50/p95={:.3}/{:.3} ms; surface GPU p50/p95={:.3}/{:.3} ms",
                scene.0, scene.1, present.0, present.1
            )
        } else {
            "GPU timestamps unavailable".to_owned()
        };
        Ok(format!(
            "adapter={}; surface={:?} {}x{}; presented={}; callbacks={}; display_link_callbacks={}; late_target_evaluations={}; invalid_target_evaluations={}; clamped_target_evaluations={}; committed_ticks={}; unavailable={}; occluded={}; surface_faults={}; dropped_surface={}; dropped_views={}; dropped_stale_views={}; dropped_reports={}; callback wall p50/p95={:.3}/{:.3} ms; render-worker wall p50/p95={:.3}/{:.3} ms; drawable-acquire wall p50/p95={:.3}/{:.3} ms; tick CPU p50/p95={:.3}/{:.3} us; live-view CPU p50/p95={:.3}/{:.3} us; upload CPU p50/p95={:.3}/{:.3} us; encode CPU p50/p95={:.3}/{:.3} us; upload+encode CPU p50/p95={:.3}/{:.3} us; {gpu}; device_losses={}; world_overloads={}; overload_refused_ticks={}; final_drain_device_lost={final_device_loss}; retained_presented_profiles={}; missing_presented_profiles={missing}; profiles describe retained samples",
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
            self.pause.overload_count(),
            self.pause.refused_ticks(),
            presented.len(),
        ))
    }
}

impl<G: Game> Drop for App<G> {
    fn drop(&mut self) {
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
    let gpu = sample.gpu_pass_ns.map_or_else(
        || "GPU timestamps unavailable".to_owned(),
        |passes| {
            let present = sample.gpu_present_ns.unwrap_or(0.0) / 1e6;
            format!(
                "GPU shadow near/far {:.3}/{:.3}, cluster {:.3}, forward {:.3}, history {:.3}, display {:.3}, atmosphere compute {:.3}, present {:.3} ms",
                passes[0] / 1e6,
                passes[1] / 1e6,
                passes[2] / 1e6,
                passes[3] / 1e6,
                passes[4] / 1e6,
                passes[5] / 1e6,
                passes[6..].iter().sum::<f64>() / 1e6,
                present,
            )
        },
    );
    format!(
        "{title} | {world_status}{gpu_status} | tick {} ({} updates, view age {}) | CPU tick {:.3}, extract {:.3}, upload {:.3}, encode {:.3} ms | {gpu}",
        sample.tick,
        sample.completed_ticks,
        sample.visual_age_ticks,
        sample.tick_cpu.as_secs_f64() * 1e3,
        sample.extract_cpu.as_secs_f64() * 1e3,
        sample.upload_cpu.as_secs_f64() * 1e3,
        sample.encode_cpu.as_secs_f64() * 1e3,
    )
}

fn percentile<T>(items: &[T], value: impl Fn(&T) -> f64) -> (f64, f64) {
    let mut sorted: Vec<_> = items.iter().map(value).collect();
    sorted.sort_by(f64::total_cmp);
    (sorted[sorted.len() / 2], sorted[sorted.len() * 95 / 100])
}

impl<G: Game + 'static> ApplicationHandler<UserEvent> for App<G> {
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
        match event {
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
            UserEvent::DisplayTarget(target) => {
                self.display_target = Some(target);
                self.target_sequence = self.target_sequence.saturating_add(1);
                self.display_link_callbacks += 1;
                self.pending_display_target = true;
                self.edits.request();
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
                }
                if let Err(error) = self.controls_edited() {
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
        if self.window.as_ref().is_none_or(|window| window.id() != id) {
            return;
        }
        let result = match event {
            WindowEvent::CloseRequested | WindowEvent::Destroyed => {
                self.render_stopped.store(true, Ordering::Release);
                if self.run_deadline.is_none() {
                    self.window_closed = true;
                } else {
                    self.closed_by_user |= self.presented < self.target_frames;
                }
                let result = self.invalidate_surface();
                event_loop.exit();
                result
            }
            WindowEvent::Resized(size) => self.observe_extent([size.width, size.height]),
            WindowEvent::Occluded(_) => self.invalidate_surface(),
            WindowEvent::RedrawRequested => {
                let result = self.draw();
                if self.presented >= self.target_frames {
                    event_loop.exit();
                }
                result
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
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
                    } else if code == KeyCode::KeyP && self.game.world().is_some() {
                        if self
                            .pause
                            .key(event.state == ElementState::Pressed, event.repeat)
                        {
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
            WindowEvent::Focused(false) => self.game.focus_lost().map(|()| self.input_changed()),
            _ => Ok(()),
        };
        if let Err(error) = result {
            self.error = Some(error);
            event_loop.exit();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some()
            && self.render.is_some()
            && let Some(deadline) = self.run_deadline
        {
            if self.origin.elapsed() >= deadline {
                self.error = Some(self.deadline_error());
                event_loop.exit();
            } else {
                event_loop.set_control_flow(ControlFlow::WaitUntil(self.origin + deadline));
            }
        }
    }
}

/// Run `game` in one window until its bounded frame count presents, then print the frames
/// report. The count comes from `config.frames_variable` (default 600). Setup, GPU and world
/// failures are reported on standard error and exit the process with a nonzero status.
///
/// # Panics
/// If the platform refuses to create the event loop, as on a second call in one process.
pub fn run<G: Game + 'static>(config: Config, game: impl FnOnce() -> Result<G, String>) {
    run_with(config, false, game);
}

/// Run `game` in one window until the person closes it. Closing the window is success. Setting
/// `config.frames_variable` bounds the run exactly as [`run`] does, for scripted checks, and
/// setting [`REPORT_VARIABLE`] to `1` prints the frames report over a bounded window of
/// recent samples at the end. Failures are reported on standard error and exit the process
/// with a nonzero status.
///
/// # Panics
/// If the platform refuses to create the event loop, as on a second call in one process.
pub fn run_until_closed<G: Game + 'static>(
    config: Config,
    game: impl FnOnce() -> Result<G, String>,
) {
    run_with(config, true, game);
}

/// Environment variable that asks [`run_until_closed`] for the frames report: set it to `1`.
/// [`run`] always prints the report.
pub const REPORT_VARIABLE: &str = "THRUM_REPORT";

#[expect(
    clippy::print_stdout,
    reason = "the bounded diagnostic reports its measured surface frame"
)]
#[expect(
    clippy::print_stderr,
    reason = "the diagnostic reports setup or GPU faults"
)]
fn run_with<G: Game + 'static>(
    config: Config,
    until_closed: bool,
    game: impl FnOnce() -> Result<G, String>,
) {
    let target_frames = match frame_limit(
        config.frames_variable,
        std::env::var(config.frames_variable).ok().as_deref(),
        until_closed,
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
    if let Err(error) = event_loop.run_app(&mut app) {
        app.error = Some(format!("event loop: {error}"));
    }
    if app.closed_by_user && app.error.is_none() {
        app.error = Some(format!(
            "presentation cancelled after {}/{} requested frames",
            app.presented, app.target_frames
        ));
    }
    match app.error.take().map_or_else(|| app.finish(), Err) {
        Ok(report) => {
            if prints_report(until_closed, std::env::var(REPORT_VARIABLE).ok().as_deref()) {
                println!("{report}");
            }
        }
        Err(error) => {
            eprintln!("{}: {error}", config.name);
            drop(app);
            std::process::exit(1);
        }
    }
}

/// Whether a finished run prints its frames report: always for a bounded [`run`], and for
/// [`run_until_closed`] only when [`REPORT_VARIABLE`] is `1`.
fn prints_report(until_closed: bool, requested: Option<&str>) -> bool {
    !until_closed || requested == Some("1")
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

#[cfg(test)]
mod tests {
    use super::{frame_limit, prints_report, run_frames};

    #[test]
    fn frame_count_for_present_qualification_is_bounded() {
        let run_frames = |value| run_frames("THRUM_V1_FRAMES", value);
        assert_eq!(run_frames(None).expect("default"), 600);
        assert_eq!(run_frames(Some("75000")).expect("ten minute run"), 75_000);
        assert_eq!(
            run_frames(Some("0")),
            Err("THRUM_V1_FRAMES must be between 1 and 100000".to_owned())
        );
        assert!(run_frames(Some("100001")).is_err());
        assert_eq!(
            run_frames(Some("nan")),
            Err("THRUM_V1_FRAMES must be a positive integer".to_owned())
        );
    }

    #[test]
    fn a_run_until_closed_is_bounded_only_when_its_variable_is_set() {
        assert!(prints_report(false, None), "a bounded run always reports");
        assert!(
            !prints_report(true, None),
            "a closed window reports only on request"
        );
        assert!(!prints_report(true, Some("0")));
        assert!(prints_report(true, Some("1")));
        assert_eq!(frame_limit("THRUM_STARTER_FRAMES", None, true), Ok(None));
        assert_eq!(
            frame_limit("THRUM_STARTER_FRAMES", Some("90"), true),
            Ok(Some(90))
        );
        assert!(frame_limit("THRUM_STARTER_FRAMES", Some("0"), true).is_err());
        assert_eq!(frame_limit("THRUM_V1_FRAMES", None, false), Ok(Some(600)));
    }
}
