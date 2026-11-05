//! iOS surface and lifecycle adapter for the shared V1 simulation and scene.

use std::{
    fs,
    sync::Arc,
    time::{Duration, Instant},
};

use thrum_render::{present::SurfaceProfile, timing::GpuTimings};
use wgpu::{CurrentSurfaceTexture, TextureViewDescriptor};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::{Window, WindowId},
};

use v1_game::{FrameHost, FrameSample, SurfaceOutput};

const FRAMES: usize = 600;
const REPORT_NAME: &str = "thrum-v1-ios-report.txt";

#[expect(
    unsafe_code,
    reason = "Swift attaches Winit's borrowed UIView to the UIKit scene"
)]
unsafe extern "C" {
    fn thrum_v1_attach_view(view: *mut std::ffi::c_void);
}

#[expect(
    unsafe_code,
    reason = "the live main-thread Winit window owns this UIView for the call"
)]
fn attach_to_scene(window: &Window) -> Result<(), String> {
    let handle = window
        .window_handle()
        .map_err(|error| format!("iOS window handle: {error}"))?;
    let RawWindowHandle::UiKit(view) = handle.as_raw() else {
        return Err("iOS window has no UIKit view".into());
    };
    // SAFETY: Winit owns `window` and its UIView across this synchronous call.
    // `prepare` runs on UIKit's main thread; Swift does not retain the pointer.
    unsafe { thrum_v1_attach_view(view.ui_view.as_ptr()) };
    Ok(())
}

struct App {
    origin: Instant,
    last_call: Duration,
    instance: Option<wgpu::Instance>,
    adapter: Option<wgpu::Adapter>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    window: Option<Arc<Window>>,
    surface: Option<wgpu::Surface<'static>>,
    config: Option<wgpu::SurfaceConfiguration>,
    host: Option<FrameHost>,
    visible: bool,
    reconfigure: bool,
    callbacks: u64,
    presented: u64,
    unavailable: u64,
    overloads: u64,
    refused_ticks: u64,
    samples: Vec<FrameSample>,
    report_written: bool,
    error: Option<String>,
}

impl App {
    fn new() -> Self {
        Self {
            origin: Instant::now(),
            last_call: Duration::ZERO,
            instance: None,
            adapter: None,
            device: None,
            queue: None,
            window: None,
            surface: None,
            config: None,
            host: None,
            visible: false,
            reconfigure: false,
            callbacks: 0,
            presented: 0,
            unavailable: 0,
            overloads: 0,
            refused_ticks: 0,
            samples: Vec::with_capacity(FRAMES),
            report_written: false,
            error: None,
        }
    }

    fn prepare(&mut self, event_loop: &ActiveEventLoop) -> Result<(), String> {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title("Thrum V1 scene"))
                .map_err(|error| format!("iOS window: {error}"))?,
        );
        attach_to_scene(&window)?;
        let instance = self.instance.get_or_insert_with(|| {
            let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
            descriptor.backends = wgpu::Backends::METAL;
            wgpu::Instance::new(descriptor)
        });
        let surface = instance
            .create_surface(Arc::clone(&window))
            .map_err(|error| format!("iOS surface: {error}"))?;
        if self.adapter.is_none() {
            self.adapter = Some(
                pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                    compatible_surface: Some(&surface),
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    ..Default::default()
                }))
                .map_err(|error| format!("iOS adapter: {error:?}"))?,
            );
        }
        let adapter = self.adapter.as_ref().ok_or("iOS adapter missing")?;
        if self.device.is_none() {
            let (device, queue) =
                pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                    required_features: GpuTimings::requested_features(adapter),
                    required_limits:
                        wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
                    ..Default::default()
                }))
                .map_err(|error| format!("iOS device: {error:?}"))?;
            self.device = Some(device);
            self.queue = Some(queue);
        }
        let device = self.device.as_ref().ok_or("iOS device missing")?;
        let queue = self.queue.as_ref().ok_or("iOS queue missing")?;
        let size = window.inner_size();
        let mut config = surface
            .get_default_config(adapter, size.width.max(1), size.height.max(1))
            .ok_or("no iOS surface configuration")?;
        let formats = surface.get_capabilities(adapter).formats;
        config.format = if let Some(previous) = self.config.as_ref() {
            if !formats.contains(&previous.format) {
                return Err("resumed iOS surface changed format".into());
            }
            previous.format
        } else {
            [
                wgpu::TextureFormat::Bgra8Unorm,
                wgpu::TextureFormat::Rgba8Unorm,
                wgpu::TextureFormat::Bgra8UnormSrgb,
                wgpu::TextureFormat::Rgba8UnormSrgb,
            ]
            .into_iter()
            .find(|format| formats.contains(format))
            .ok_or("no supported iOS SDR surface format")?
        };
        surface.configure(device, &config);
        if let Some(host) = self.host.as_mut() {
            host.resize_surface([config.width, config.height])?;
            host.drain_and_resume(self.origin.elapsed())?;
        } else {
            self.host = Some(pollster::block_on(FrameHost::new(
                device,
                queue,
                Some(SurfaceProfile {
                    format: config.format,
                    extent: [config.width, config.height],
                }),
            ))?);
            self.origin = Instant::now();
            self.last_call = Duration::ZERO;
        }
        self.window = Some(window);
        self.surface = Some(surface);
        self.config = Some(config);
        self.visible = true;
        self.reconfigure = false;
        self.window
            .as_ref()
            .ok_or("iOS window missing")?
            .request_redraw();
        Ok(())
    }

    fn set_visible(&mut self, visible: bool) -> Result<(), String> {
        if visible == self.visible {
            return Ok(());
        }
        let now = self.origin.elapsed();
        let host = self.host.as_mut().ok_or("iOS host missing")?;
        if visible {
            host.drain_and_resume(now)?;
            self.last_call = now;
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        } else {
            host.pause();
        }
        self.visible = visible;
        Ok(())
    }

    fn draw(&mut self) -> Result<(), String> {
        if !self.visible {
            return Ok(());
        }
        let now = self.origin.elapsed();
        self.callbacks += 1;
        let window = self.window.as_ref().ok_or("iOS window missing")?;
        let size = window.inner_size();
        let extent = [size.width, size.height];
        let host = self.host.as_mut().ok_or("iOS host missing")?;
        let device = self.device.as_ref().ok_or("iOS device missing")?;
        let config = self.config.as_mut().ok_or("iOS config missing")?;
        let resized = extent != [config.width, config.height];
        if (resized || self.reconfigure) && !extent.contains(&0) && host.pending_frames() == 0 {
            config.width = extent[0];
            config.height = extent[1];
            self.surface
                .as_ref()
                .ok_or("iOS surface missing")?
                .configure(device, config);
            host.resize_surface(extent)?;
            self.reconfigure = false;
        }
        let due = host
            .next_due_in()
            .is_some_and(|delay| now.saturating_sub(self.last_call) >= delay);
        self.last_call = now;
        let can_acquire = due
            && !extent.contains(&0)
            && !self.reconfigure
            && extent == [config.width, config.height]
            && host.pending_frames() < FrameHost::MAX_IN_FLIGHT;
        let frame = if can_acquire {
            match self
                .surface
                .as_ref()
                .ok_or("iOS surface missing")?
                .get_current_texture()
            {
                CurrentSurfaceTexture::Success(frame) => Some(frame),
                CurrentSurfaceTexture::Suboptimal(frame) => {
                    self.reconfigure = true;
                    Some(frame)
                }
                CurrentSurfaceTexture::Lost | CurrentSurfaceTexture::Outdated => {
                    self.reconfigure = true;
                    self.unavailable += 1;
                    None
                }
                CurrentSurfaceTexture::Timeout | CurrentSurfaceTexture::Occluded => {
                    self.unavailable += 1;
                    None
                }
                CurrentSurfaceTexture::Validation => {
                    return Err("iOS surface validation fault".into());
                }
            }
        } else {
            None
        };
        let view = frame
            .as_ref()
            .map(|frame| frame.texture.create_view(&TextureViewDescriptor::default()));
        let output = view.as_ref().map(|view| SurfaceOutput {
            view,
            extent: [config.width, config.height],
        });
        let report = host.drive(now, output.as_ref())?;
        if report.surface_presented {
            self.queue
                .as_ref()
                .ok_or("iOS queue missing")?
                .present(frame.ok_or("surface pass without acquired iOS frame")?);
            self.presented += 1;
        }
        if let Some(refused) = report.overload {
            self.overloads += 1;
            self.refused_ticks = self.refused_ticks.saturating_add(refused);
            host.pause();
            host.drain_and_resume(now)?;
        }
        while let Some(sample) = host.poll_completed()? {
            if self.samples.len() < FRAMES {
                self.samples.push(sample);
            }
        }
        if self.samples.len() == FRAMES && !self.report_written {
            self.write_report()?;
            self.report_written = true;
        }
        Ok(())
    }

    fn write_report(&self) -> Result<(), String> {
        let host = self.host.as_ref().ok_or("iOS host missing")?;
        let adapter = self.adapter.as_ref().ok_or("iOS adapter missing")?;
        let config = self.config.as_ref().ok_or("iOS surface config missing")?;
        let surface_samples = self
            .samples
            .iter()
            .filter(|sample| sample.surface_presented)
            .count();
        let gpu_timed = self
            .samples
            .iter()
            .filter(|sample| sample.gpu_pass_ns.is_some())
            .count();
        let tick_cpu = percentile(&self.samples, |sample| sample.tick_cpu.as_secs_f64() * 1e6);
        let encode_cpu = percentile(&self.samples, |sample| {
            sample.encode_cpu.as_secs_f64() * 1e6
        });
        let wall = percentile(&self.samples, |sample| sample.wall.as_secs_f64() * 1e3);
        let report = format!(
            "iOS simulator synthetic scene; adapter={} format={:?} extent={}x{}; callbacks={} presented={} completed_gpu={} surface_samples={} gpu_timed={} ticks={} unavailable={} overloads={} refused_ticks={} shed_views={} report_drops={}; host wall p50/p95={:.3}/{:.3} ms; tick CPU p50/p95={:.1}/{:.1} us; encode CPU p50/p95={:.1}/{:.1} us\n",
            adapter.get_info().name,
            config.format,
            config.width,
            config.height,
            self.callbacks,
            self.presented,
            self.samples.len(),
            surface_samples,
            gpu_timed,
            host.tick(),
            self.unavailable,
            self.overloads,
            self.refused_ticks,
            host.dropped_views(),
            host.dropped_reports(),
            wall.0,
            wall.1,
            tick_cpu.0,
            tick_cpu.1,
            encode_cpu.0,
            encode_cpu.1,
        );
        fs::write(std::env::temp_dir().join(REPORT_NAME), report)
            .map_err(|error| format!("iOS report: {error}"))
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: String) {
        drop(fs::write(
            std::env::temp_dir().join(REPORT_NAME),
            format!("fault: {error}\n"),
        ));
        self.error = Some(error);
        event_loop.exit();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            if let Err(error) = self.prepare(event_loop) {
                self.fail(event_loop, error);
            }
        } else if let Err(error) = self.set_visible(true) {
            self.fail(event_loop, error);
        }
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        if let Err(error) = self.set_visible(false) {
            self.fail(event_loop, error);
        }
        self.surface = None;
        self.window = None;
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
            WindowEvent::RedrawRequested => self.draw(),
            WindowEvent::Occluded(occluded) => self.set_visible(!occluded),
            _ => Ok(()),
        };
        if let Err(error) = result {
            self.fail(event_loop, error);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let (Some(window), Some(host)) = (&self.window, &self.host) {
            if self.visible {
                if let Some(due) = host.next_due_in() {
                    let delay =
                        due.saturating_sub(self.origin.elapsed().saturating_sub(self.last_call));
                    if delay.is_zero() {
                        window.request_redraw();
                        event_loop.set_control_flow(ControlFlow::Wait);
                    } else {
                        event_loop.set_control_flow(ControlFlow::WaitUntil(Instant::now() + delay));
                    }
                }
            } else {
                event_loop.set_control_flow(ControlFlow::Wait);
            }
        }
    }
}

pub(super) fn run() {
    let event_loop = EventLoop::new().expect("iOS event loop");
    let mut app = App::new();
    if let Err(error) = event_loop.run_app(&mut app) {
        app.error = Some(format!("iOS event loop: {error}"));
    }
    if let Some(error) = app.error {
        drop(fs::write(
            std::env::temp_dir().join(REPORT_NAME),
            format!("fault: {error}\n"),
        ));
    }
}

fn percentile(samples: &[FrameSample], value: impl Fn(&FrameSample) -> f64) -> (f64, f64) {
    let mut values: Vec<_> = samples.iter().map(value).collect();
    values.sort_by(f64::total_cmp);
    (values[values.len() / 2], values[values.len() * 95 / 100])
}
