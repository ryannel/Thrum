//! The platform owns the surface and event loop; the shared host owns world
//! ticks, GPU scene packets and the prepared presentation pass.

use std::{
    sync::Arc,
    sync::mpsc::{self, Receiver, SyncSender, TryRecvError},
    thread,
    time::{Duration, Instant},
};

use thrum_render::{present::SurfaceProfile, timing::GpuTimings};
use v1_game::{FrameHost, FrameSample, SurfaceOutput};
use wgpu::{CurrentSurfaceTexture, TextureViewDescriptor};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

const FRAMES: usize = 600;
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
    Extracted,
    SurfaceAcquired,
}

struct SurfaceState {
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
}

#[derive(Clone, Copy)]
struct AcquireRequest {
    extent: [u32; 2],
    reconfigure: bool,
}

enum Acquired {
    Frame(wgpu::SurfaceTexture, [u32; 2]),
    Unavailable,
    Occluded,
    NeedsReconfigure,
    Fault(String),
}

struct SurfaceWorker {
    request: SyncSender<AcquireRequest>,
    result: Receiver<Acquired>,
}

impl SurfaceWorker {
    fn start(
        mut state: SurfaceState,
        device: wgpu::Device,
        wake: EventLoopProxy<UserEvent>,
    ) -> Result<Self, String> {
        // A zero-capacity request handshake prevents reconfigure until the
        // event loop has presented or dropped the previously acquired frame.
        let (request, requests) = mpsc::sync_channel::<AcquireRequest>(0);
        let (results, result) = mpsc::sync_channel(1);
        thread::Builder::new()
            .name("v1-surface-acquire".into())
            .spawn(move || {
                let mut needs_reconfigure = false;
                while let Ok(request) = requests.recv() {
                    let extent = request.extent;
                    let answer = if extent.contains(&0) {
                        Acquired::Unavailable
                    } else {
                        if state.config.width != extent[0]
                            || state.config.height != extent[1]
                            || needs_reconfigure
                        {
                            if !request.reconfigure {
                                drop(results.send(Acquired::Fault(
                                    "uncoordinated surface reconfigure".into(),
                                )));
                                let _ = wake.send_event(UserEvent::SurfaceAcquired);
                                break;
                            }
                            state.config.width = extent[0];
                            state.config.height = extent[1];
                            state.surface.configure(&device, &state.config);
                            needs_reconfigure = false;
                        }
                        match state.surface.get_current_texture() {
                            CurrentSurfaceTexture::Success(frame)
                            | CurrentSurfaceTexture::Suboptimal(frame) => {
                                Acquired::Frame(frame, extent)
                            }
                            CurrentSurfaceTexture::Timeout => Acquired::Unavailable,
                            CurrentSurfaceTexture::Occluded => Acquired::Occluded,
                            CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Lost => {
                                needs_reconfigure = true;
                                Acquired::NeedsReconfigure
                            }
                            CurrentSurfaceTexture::Validation => {
                                Acquired::Fault("surface acquire validation failure".into())
                            }
                        }
                    };
                    if results.send(answer).is_err() {
                        break;
                    }
                    if wake.send_event(UserEvent::SurfaceAcquired).is_err() {
                        break;
                    }
                }
            })
            .map_err(|error| format!("surface worker: {error}"))?;
        Ok(Self { request, result })
    }
}

struct App {
    wake: EventLoopProxy<UserEvent>,
    window: Option<Arc<Window>>,
    surface: Option<SurfaceWorker>,
    surface_format: Option<wgpu::TextureFormat>,
    surface_extent: [u32; 2],
    worker_extent: [u32; 2],
    needs_reconfigure: bool,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    host: Option<FrameHost>,
    held_keys: u8,
    origin: Instant,
    last_call: Duration,
    presented: usize,
    callbacks: usize,
    unavailable: usize,
    occluded: usize,
    waiting_acquire: usize,
    acquiring: bool,
    stale_acquire: usize,
    surface_faults: usize,
    callback_walls: Vec<Duration>,
    samples: Vec<FrameSample>,
    profiler_title_at: Duration,
    adapter_name: String,
    error: Option<String>,
}

impl App {
    fn new(wake: EventLoopProxy<UserEvent>) -> Self {
        Self {
            wake,
            window: None,
            surface: None,
            surface_format: None,
            surface_extent: [0, 0],
            worker_extent: [0, 0],
            needs_reconfigure: false,
            device: None,
            queue: None,
            host: None,
            held_keys: 0,
            origin: Instant::now(),
            last_call: Duration::ZERO,
            presented: 0,
            callbacks: 0,
            unavailable: 0,
            occluded: 0,
            waiting_acquire: 0,
            acquiring: false,
            stale_acquire: 0,
            surface_faults: 0,
            callback_walls: Vec::with_capacity(FRAMES + 32),
            samples: Vec::with_capacity(FRAMES),
            profiler_title_at: Duration::ZERO,
            adapter_name: String::new(),
            error: None,
        }
    }

    fn update_controls(&mut self) -> Result<(), String> {
        let horizontal = axis(
            self.held_keys & (KEY_A | KEY_LEFT) != 0,
            self.held_keys & (KEY_D | KEY_RIGHT) != 0,
        );
        let vertical = axis(
            self.held_keys & (KEY_S | KEY_DOWN) != 0,
            self.held_keys & (KEY_W | KEY_UP) != 0,
        );
        if let Some(host) = &mut self.host {
            host.set_controls([horizontal, vertical, 0.0])?;
        }
        Ok(())
    }

    fn set_key(&mut self, code: KeyCode, pressed: bool) -> Result<(), String> {
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
                        .with_inner_size(winit::dpi::PhysicalSize::new(1280, 720)),
                )
                .map_err(|error| format!("window: {error}"))?,
        );
        let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        descriptor.backends = wgpu::Backends::METAL;
        let instance = wgpu::Instance::new(descriptor);
        let surface = instance
            .create_surface(Arc::clone(&window))
            .map_err(|error| format!("surface: {error}"))?;
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        }))
        .map_err(|error| format!("adapter: {error:?}"))?;
        self.adapter_name = adapter.get_info().name;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            required_features: GpuTimings::requested_features(&adapter),
            ..Default::default()
        }))
        .map_err(|error| format!("device: {error:?}"))?;
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(&adapter, size.width.max(1), size.height.max(1))
            .ok_or("no compatible surface configuration")?;
        let formats = surface.get_capabilities(&adapter).formats;
        config.format = [
            wgpu::TextureFormat::Bgra8Unorm,
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            wgpu::TextureFormat::Rgba8UnormSrgb,
        ]
        .into_iter()
        .find(|format| formats.contains(format))
        .ok_or("no supported 8-bit SDR surface format")?;
        surface.configure(&device, &config);
        let mut host = pollster::block_on(FrameHost::new(
            &device,
            &queue,
            Some(SurfaceProfile {
                format: config.format,
                extent: [config.width, config.height],
            }),
        ))?;
        let extract_wake = wake.clone();
        host.enable_native_worker(move || {
            let _ = extract_wake.send_event(UserEvent::Extracted);
        })?;
        self.origin = Instant::now();
        self.window = Some(window);
        self.surface_format = Some(config.format);
        self.surface_extent = [config.width, config.height];
        self.worker_extent = self.surface_extent;
        self.surface = Some(SurfaceWorker::start(
            SurfaceState { surface, config },
            device.clone(),
            wake.clone(),
        )?);
        self.device = Some(device);
        self.queue = Some(queue);
        self.host = Some(host);
        Ok(())
    }

    fn draw(&mut self) -> Result<(), String> {
        let callback_start = Instant::now();
        let now = self.origin.elapsed();
        self.last_call = now;
        self.callbacks += 1;
        let size = self.window.as_ref().ok_or("window missing")?.inner_size();
        let state = self.surface.as_ref().ok_or("surface missing")?;
        let frame = if self.acquiring {
            match state.result.try_recv() {
                Ok(Acquired::Frame(frame, extent)) if extent == [size.width, size.height] => {
                    self.acquiring = false;
                    if self.surface_extent != extent {
                        self.host
                            .as_mut()
                            .ok_or("host missing")?
                            .resize_surface(extent)?;
                        self.surface_extent = extent;
                    }
                    self.needs_reconfigure = false;
                    Some(frame)
                }
                Ok(Acquired::Frame(_, _)) => {
                    self.acquiring = false;
                    self.stale_acquire += 1;
                    self.needs_reconfigure = true;
                    None
                }
                Ok(Acquired::Unavailable) => {
                    self.acquiring = false;
                    self.unavailable += 1;
                    None
                }
                Ok(Acquired::Occluded) => {
                    self.acquiring = false;
                    self.unavailable += 1;
                    self.occluded += 1;
                    None
                }
                Ok(Acquired::NeedsReconfigure) => {
                    self.acquiring = false;
                    self.surface_faults += 1;
                    self.needs_reconfigure = true;
                    None
                }
                Ok(Acquired::Fault(error)) => return Err(error),
                Err(TryRecvError::Empty) => {
                    self.waiting_acquire += 1;
                    None
                }
                Err(TryRecvError::Disconnected) => return Err("surface worker stopped".into()),
            }
        } else {
            None
        };
        let view = frame
            .as_ref()
            .map(|frame| frame.texture.create_view(&TextureViewDescriptor::default()));
        let output = view.as_ref().map(|view| SurfaceOutput {
            view,
            extent: self.surface_extent,
        });
        let host = self.host.as_mut().ok_or("host missing")?;
        let report = host.drive(now, output.as_ref())?;
        if let Some(refused_ticks) = report.overload {
            return Err(format!(
                "fixed-clock overload: {} ticks refused, {} still queued",
                refused_ticks, report.queued_ticks
            ));
        }
        if report.surface_presented {
            self.queue
                .as_ref()
                .ok_or("queue missing")?
                .present(frame.ok_or("surface pass without acquired frame")?);
            self.presented += 1;
        }
        while let Some(sample) = host.poll_completed()? {
            self.samples.push(sample);
        }
        if now.saturating_sub(self.profiler_title_at) >= Duration::from_millis(500)
            && let (Some(window), Some(sample)) = (&self.window, self.samples.last())
        {
            window.set_title(&profiler_title(sample));
            self.profiler_title_at = now;
        }
        self.callback_walls.push(callback_start.elapsed());
        if self.occluded > 120 || self.callbacks > FRAMES * 4 {
            return Err(format!(
                "600 presented frames unavailable: presented={}, tick={}, unavailable={}, occluded={}, waiting_acquire={}, stale_acquire={}, surface_faults={}, dropped_views={}, dropped_stale_views={}, reused_views={}, dropped_surface={}",
                self.presented,
                host.tick(),
                self.unavailable,
                self.occluded,
                self.waiting_acquire,
                self.stale_acquire,
                self.surface_faults,
                host.dropped_views(),
                host.dropped_stale_views(),
                host.reused_views(),
                host.dropped_surface()
            ));
        }
        Ok(())
    }

    fn finish(&mut self) -> Result<String, String> {
        let host = self.host.as_mut().ok_or("host missing")?;
        let device = self.device.as_ref().ok_or("device missing")?;
        while host.pending_frames() > 0 {
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(Duration::from_secs(10)),
                })
                .map_err(|error| format!("GPU drain: {error:?}"))?;
            while let Some(sample) = host.poll_completed()? {
                self.samples.push(sample);
            }
        }
        let wall = percentile(&self.callback_walls, |duration| {
            duration.as_secs_f64() * 1e3
        });
        let presented: Vec<_> = self
            .samples
            .iter()
            .filter(|sample| sample.surface_presented)
            .collect();
        if presented.is_empty() {
            return Err("no surface frames were presented".into());
        }
        if presented.len() != self.presented {
            return Err(format!(
                "{} presented samples for {} presents",
                presented.len(),
                self.presented
            ));
        }
        let tick = percentile(&presented, |sample| sample.tick_cpu.as_secs_f64() * 1e6);
        let encode = percentile(&presented, |sample| sample.encode_cpu.as_secs_f64() * 1e6);
        let gpu = if presented
            .iter()
            .all(|sample| sample.gpu_present_ns.is_some())
        {
            let scene = percentile(&presented, |sample| {
                sample.gpu_pass_ns.unwrap_or([0.0; 4]).iter().sum::<f64>() / 1e6
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
            "adapter={}; surface={:?} {}x{}; presented={}; callbacks={}; committed_ticks={}; unavailable={}; occluded={}; waiting_acquire={}; stale_acquire={}; surface_faults={}; dropped_surface={}; dropped_views={}; dropped_stale_views={}; reused_views={}; dropped_reports={}; callback CPU p50/p95={:.3}/{:.3} ms; tick CPU p50/p95={:.3}/{:.3} us; encode CPU p50/p95={:.3}/{:.3} us; {gpu}",
            self.adapter_name,
            self.surface_format.ok_or("surface format missing")?,
            self.surface_extent[0],
            self.surface_extent[1],
            self.presented,
            self.callbacks,
            host.tick(),
            self.unavailable,
            self.occluded,
            self.waiting_acquire,
            self.stale_acquire,
            self.surface_faults,
            host.dropped_surface(),
            host.dropped_views(),
            host.dropped_stale_views(),
            host.reused_views(),
            host.dropped_reports(),
            wall.0,
            wall.1,
            tick.0,
            tick.1,
            encode.0,
            encode.1,
        ))
    }
}

fn profiler_title(sample: &FrameSample) -> String {
    let gpu = sample.gpu_pass_ns.map_or_else(
        || "GPU timestamps unavailable".to_owned(),
        |passes| {
            let present = sample.gpu_present_ns.unwrap_or(0.0) / 1e6;
            format!(
                "GPU shadow near/far {:.3}/{:.3}, forward {:.3}, display {:.3}, present {:.3} ms",
                passes[0] / 1e6,
                passes[1] / 1e6,
                passes[2] / 1e6,
                passes[3] / 1e6,
                present,
            )
        },
    );
    format!(
        "Thrum V1 | tick {} ({} updates, view age {}, reused {}) | CPU tick {:.3}, extract {:.3}, queue {:.3}, ready {:.3}, upload {:.3}, encode {:.3} ms | {gpu}",
        sample.tick,
        sample.completed_ticks,
        sample.visual_age_ticks,
        sample.reused_visual,
        sample.tick_cpu.as_secs_f64() * 1e3,
        sample.extract_cpu.as_secs_f64() * 1e3,
        f64::from(sample.worker_queue_ns) / 1e6,
        f64::from(sample.worker_ready_ns) / 1e6,
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

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: UserEvent) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        if self.window.as_ref().is_none_or(|window| window.id() != id) {
            return;
        }
        let result = match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
                Ok(())
            }
            WindowEvent::RedrawRequested => {
                let result = self.draw();
                if self.presented >= FRAMES {
                    event_loop.exit();
                }
                result
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    self.set_key(code, event.state == ElementState::Pressed)
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
        if let (Some(window), Some(host), Some(surface)) = (&self.window, &self.host, &self.surface)
            && let Some(due) = host.next_due_in()
        {
            let size = window.inner_size();
            let extent = [size.width, size.height];
            let reconfigure = self.needs_reconfigure || extent != self.worker_extent;
            let since_call = self.origin.elapsed().saturating_sub(self.last_call);
            let delay = due.saturating_sub(since_call);
            if (host.has_ready_view() || (delay.is_zero() && host.has_reusable_view()))
                && !self.acquiring
                && host.pending_frames() < if reconfigure { 1 } else { 3 }
            {
                match surface.request.try_send(AcquireRequest {
                    extent,
                    reconfigure,
                }) {
                    Ok(()) => {
                        self.acquiring = true;
                        if !extent.contains(&0) {
                            self.worker_extent = extent;
                        }
                    }
                    Err(mpsc::TrySendError::Full(_)) => {}
                    Err(mpsc::TrySendError::Disconnected(_)) => {
                        self.error = Some("surface worker stopped".into());
                        event_loop.exit();
                        return;
                    }
                }
            }
            if delay.is_zero() {
                window.request_redraw();
                event_loop.set_control_flow(ControlFlow::Wait);
            } else {
                event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + delay));
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
    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .expect("macOS event loop");
    let mut app = App::new(event_loop.create_proxy());
    if let Err(error) = event_loop.run_app(&mut app) {
        app.error = Some(format!("event loop: {error}"));
    }
    match app.error.take().map_or_else(|| app.finish(), Err) {
        Ok(report) => println!("{report}"),
        Err(error) => {
            eprintln!("v1-game: {error}");
            std::process::exit(1);
        }
    }
}
