//! The platform owns the surface and event loop; the shared host owns world
//! ticks, GPU scene packets and the prepared presentation pass.

#[path = "macos/device.rs"]
mod device;
#[path = "macos/display_link.rs"]
mod display_link;
#[path = "macos/pause.rs"]
mod pause;
#[path = "macos/recovery.rs"]
mod recovery;
#[path = "macos/render_actor.rs"]
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
use pause::PauseIntent;
use recovery::Recovery;
use render_actor::{
    FrameCounters, FrameRequest, RecoveryFailure, RenderJob, RenderOutcome, RenderReply,
    SurfaceStatus,
};
use thrum_core::sim::WorldLaneFault;
use thrum_render::{
    frame::{FrameWorker, TargetLeadStatus},
    present::SurfaceProfile,
};
use v1_game::{FrameConfig, FrameHost, FrameSample, WorldControls};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

const DEFAULT_FRAMES: usize = 600;
const MAX_FRAMES: usize = 100_000;
const KEY_A: u8 = 1;
const KEY_D: u8 = 2;
const KEY_W: u8 = 4;
const KEY_S: u8 = 8;
const KEY_LEFT: u8 = 16;
const KEY_RIGHT: u8 = 32;
const KEY_UP: u8 = 64;
const KEY_DOWN: u8 = 128;

#[derive(Clone, Copy, Debug)]
enum UserEvent {
    WorldAdvanced,
    DisplayTarget(f64),
    RenderDone,
    DeviceLost(u64),
}

struct App {
    wake: EventLoopProxy<UserEvent>,
    display_link: Option<ViewDisplayLink>,
    window: Option<Arc<Window>>,
    display_target: Option<f64>,
    target_sequence: u64,
    request_sequence: u64,
    submitted_sequence: u64,
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
    render: Option<FrameWorker<RenderJob, RenderReply>>,
    world_controls: Option<WorldControls>,
    pause: PauseIntent,
    recovery: Recovery,
    recovery_error: Option<String>,
    device_losses: u64,
    world_tick: u64,
    render_in_flight: Option<u64>,
    counters: FrameCounters,
    held_keys: u8,
    time_of_day: f32,
    pending_time_of_day: Option<f32>,
    camera_pitch: f32,
    pending_camera_pitch: Option<f32>,
    camera_yaw: f32,
    pending_camera_yaw: Option<f32>,
    origin: Instant,
    last_call: Duration,
    presented: usize,
    target_frames: usize,
    run_deadline: Duration,
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

impl App {
    fn new(wake: EventLoopProxy<UserEvent>, target_frames: usize) -> Self {
        let run_deadline = Duration::from_secs(30).max(
            Duration::from_secs(10)
                + Duration::from_millis(
                    u64::try_from(target_frames).expect("bounded frame count") * 10,
                ),
        );
        Self {
            wake,
            display_link: None,
            window: None,
            display_target: None,
            target_sequence: 0,
            request_sequence: 0,
            submitted_sequence: 0,
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
            world_controls: None,
            pause: PauseIntent::default(),
            recovery: Recovery::new(FrameHost::DEVICE_GENERATION),
            recovery_error: None,
            device_losses: 0,
            world_tick: 0,
            render_in_flight: None,
            counters: FrameCounters::default(),
            held_keys: 0,
            time_of_day: 0.5,
            pending_time_of_day: None,
            camera_pitch: 0.0,
            pending_camera_pitch: None,
            camera_yaw: 0.0,
            pending_camera_yaw: None,
            origin: Instant::now(),
            last_call: Duration::ZERO,
            presented: 0,
            target_frames,
            run_deadline,
            callbacks: 0,
            unavailable: 0,
            occluded: 0,
            surface_faults: 0,
            callback_walls: Vec::with_capacity(target_frames * 8),
            worker_walls: Vec::with_capacity(target_frames * 4),
            acquire_walls: Vec::with_capacity(target_frames * 4),
            samples: Vec::with_capacity(target_frames * 2),
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

    fn controls(&self) -> [f32; 3] {
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

    fn update_controls(&mut self) -> Result<(), String> {
        if let Some(controls) = &self.world_controls {
            controls.set_controls(self.controls())?;
        }
        self.request_sequence = self.request_sequence.saturating_add(1);
        if let Some(window) = &self.window {
            window.request_redraw();
        }
        Ok(())
    }

    fn reconcile_pause(&mut self) -> Result<(), String> {
        let Some(controls) = &self.world_controls else {
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

    fn refresh_title(&self) {
        if let Some(window) = &self.window {
            let title = self.samples.last().map_or_else(
                || format!("Thrum V1 | {}{}", self.pause.label(), self.recovery.label()),
                |sample| profiler_title(sample, self.pause.label(), self.recovery.label()),
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
            self.request_sequence = self.request_sequence.saturating_add(1);
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

    fn set_key(&mut self, code: KeyCode, pressed: bool) -> Result<(), String> {
        if pressed && matches!(code, KeyCode::KeyQ | KeyCode::KeyE) {
            let step = if code == KeyCode::KeyQ {
                -0.087_266_46
            } else {
                0.087_266_46
            };
            self.camera_yaw = (self.camera_yaw + step + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            self.pending_camera_yaw = Some(self.camera_yaw);
            self.request_sequence = self.request_sequence.saturating_add(1);
            if let Some(window) = &self.window {
                window.request_redraw();
            }
            return Ok(());
        }
        if pressed && matches!(code, KeyCode::KeyR | KeyCode::KeyF) {
            let step = if code == KeyCode::KeyR {
                0.087_266_46
            } else {
                -0.087_266_46
            };
            let next = (self.camera_pitch + step).clamp(-0.6, 0.6);
            if next.to_bits() != self.camera_pitch.to_bits() {
                self.camera_pitch = next;
                self.pending_camera_pitch = Some(next);
                self.request_sequence = self.request_sequence.saturating_add(1);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            return Ok(());
        }
        if pressed && matches!(code, KeyCode::BracketLeft | KeyCode::BracketRight) {
            let step = if code == KeyCode::BracketRight {
                1.0 / 24.0
            } else {
                -1.0 / 24.0
            };
            self.time_of_day = (self.time_of_day + step).rem_euclid(1.0);
            self.pending_time_of_day = Some(self.time_of_day);
            self.request_sequence = self.request_sequence.saturating_add(1);
            if let Some(window) = &self.window {
                window.request_redraw();
            }
            return Ok(());
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
            _ => return Ok(()),
        };
        if (self.held_keys & mask != 0) == pressed {
            return Ok(());
        }
        if pressed {
            self.held_keys |= mask;
        } else {
            self.held_keys &= !mask;
        }
        self.update_controls()
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
                        .with_title("Thrum V1 — 10k world / clustered forward+")
                        .with_inner_size(winit::dpi::PhysicalSize::new(1920, 1080)),
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
        let gpu = pollster::block_on(recipe.create(
            FrameHost::DEVICE_GENERATION,
            [size.width, size.height],
            wake,
        ))?;
        if gpu.watch.is_lost() {
            return Err("initial device lost before preparation".to_owned());
        }
        recipe.surface.configure(&gpu.device, &gpu.config);
        if gpu.watch.is_lost() {
            return Err("initial device lost during surface configuration".to_owned());
        }
        let mut host = pollster::block_on(FrameHost::new_with_capabilities(
            &gpu.device,
            &gpu.queue,
            &gpu.capabilities,
            FrameConfig {
                render_extent: [1920, 1080],
                surface: Some(SurfaceProfile {
                    format: gpu.config.format,
                    extent: [gpu.config.width, gpu.config.height],
                }),
            },
        ))?;
        v1_game::attach_cooked_scene(&mut host)?;
        [self.camera_pitch, self.camera_yaw] = host.camera_orientation();
        if gpu.watch.is_lost() {
            return Err("initial device lost during preparation".to_owned());
        }
        self.origin = Instant::now();
        let world_wake = wake.clone();
        let (host, controls) = host.into_threaded(self.origin, move || {
            let _ = world_wake.send_event(UserEvent::WorldAdvanced);
        })?;
        if gpu.watch.is_lost() {
            return Err("initial device lost before render owner ready".to_owned());
        }
        self.world_controls = Some(controls);
        self.counters = FrameCounters::read(&host);
        self.window = Some(window);
        self.display_link = Some(ViewDisplayLink::start(
            self.window.as_ref().ok_or("window missing")?,
            wake.clone(),
        )?);
        self.adapter_name.clone_from(&gpu.adapter_name);
        self.surface_format = Some(gpu.config.format);
        self.surface_extent = [gpu.config.width, gpu.config.height];
        self.observed_extent = self.surface_extent;
        self.render = Some(render_actor::start(
            host,
            gpu,
            recipe,
            Arc::clone(&self.surface_generation),
            Arc::clone(&self.render_stopped),
            self.origin,
            wake,
        )?);
        Ok(())
    }

    fn receive_render(&mut self, reply: RenderReply) -> Result<(), String> {
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
                    self.pending_time_of_day = Some(self.time_of_day);
                    self.pending_camera_pitch = Some(self.camera_pitch);
                    self.pending_camera_yaw = Some(self.camera_yaw);
                    self.pending_display_target = self.display_target.is_some();
                    self.request_sequence = self.request_sequence.saturating_add(1);
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
        self.render_in_flight = None;
        self.counters = reply.counters;
        self.samples.extend(reply.samples.into_iter().flatten());
        self.surface_extent = reply.surface_extent;
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
                        sample,
                        self.pause.label(),
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
            || self.request_sequence == self.submitted_sequence
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
        let job = RenderJob::Frame(FrameRequest {
            sequence: self.target_sequence,
            device_generation: self.recovery.active,
            target_media_seconds: self.display_target.ok_or("display target missing")?,
            extent,
            surface_generation: self.surface_generation.load(Ordering::Acquire),
            time_of_day: self.pending_time_of_day,
            camera_pitch: self.pending_camera_pitch,
            camera_yaw: self.pending_camera_yaw,
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
        self.pending_time_of_day = None;
        self.pending_camera_pitch = None;
        self.pending_camera_yaw = None;
        self.submitted_sequence = self.request_sequence;
        self.render_in_flight = Some(self.submitted_sequence);
        if now >= self.run_deadline {
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
            self.receive_render(reply)?;
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
            presented.len(),
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

impl Drop for App {
    fn drop(&mut self) {
        self.render_stopped.store(true, Ordering::Release);
        drop(self.display_link.take());
        if let Some(render) = self.render.take() {
            render.shutdown();
        }
    }
}

fn profiler_title(sample: &FrameSample, world_status: &str, gpu_status: &str) -> String {
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
        "Thrum V1 | {world_status}{gpu_status} | tick {} ({} updates, view age {}) | CPU tick {:.3}, extract {:.3}, upload {:.3}, encode {:.3} ms | {gpu}",
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

fn axis(negative: bool, positive: bool) -> f32 {
    match (negative, positive) {
        (true, false) => -1.0,
        (false, true) => 1.0,
        _ => 0.0,
    }
}

impl ApplicationHandler<UserEvent> for App {
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
                self.request_sequence = self.request_sequence.saturating_add(1);
            }
            UserEvent::WorldAdvanced => {
                if let Err(error) = self.reconcile_pause() {
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
                self.closed_by_user |= self.presented < self.target_frames;
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
                    } else if code == KeyCode::KeyP {
                        if self
                            .pause
                            .key(event.state == ElementState::Pressed, event.repeat)
                        {
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
            WindowEvent::Focused(false) => {
                self.held_keys = 0;
                self.update_controls()
            }
            _ => Ok(()),
        };
        if let Err(error) = result {
            self.error = Some(error);
            event_loop.exit();
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() && self.render.is_some() {
            if self.origin.elapsed() >= self.run_deadline {
                self.error = Some(self.deadline_error());
                event_loop.exit();
            } else {
                event_loop
                    .set_control_flow(ControlFlow::WaitUntil(self.origin + self.run_deadline));
            }
        }
    }
}

#[expect(
    clippy::print_stdout,
    reason = "the bounded diagnostic reports its measured surface frame"
)]
#[expect(
    clippy::print_stderr,
    reason = "the diagnostic reports setup or GPU faults"
)]
pub(super) fn run() {
    let target_frames = match run_frames(std::env::var("THRUM_V1_FRAMES").ok().as_deref()) {
        Ok(frames) => frames,
        Err(error) => {
            eprintln!("v1-game: {error}");
            std::process::exit(2);
        }
    };
    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .expect("macOS event loop");
    let mut app = App::new(event_loop.create_proxy(), target_frames);
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
        Ok(report) => println!("{report}"),
        Err(error) => {
            eprintln!("v1-game: {error}");
            drop(app);
            std::process::exit(1);
        }
    }
}

fn run_frames(value: Option<&str>) -> Result<usize, String> {
    let Some(value) = value else {
        return Ok(DEFAULT_FRAMES);
    };
    let frames = value
        .parse::<usize>()
        .map_err(|_| "THRUM_V1_FRAMES must be a positive integer".to_owned())?;
    if !(1..=MAX_FRAMES).contains(&frames) {
        return Err(format!(
            "THRUM_V1_FRAMES must be between 1 and {MAX_FRAMES}"
        ));
    }
    Ok(frames)
}

#[cfg(test)]
mod tests {
    use super::run_frames;

    #[test]
    fn frame_count_for_present_qualification_is_bounded() {
        assert_eq!(run_frames(None).expect("default"), 600);
        assert_eq!(run_frames(Some("75000")).expect("ten minute run"), 75_000);
        assert!(run_frames(Some("0")).is_err());
        assert!(run_frames(Some("100001")).is_err());
        assert!(run_frames(Some("nan")).is_err());
    }
}
