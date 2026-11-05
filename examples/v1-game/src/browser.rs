//! Browser WebGPU surface adapter for the same fixed-step V1 frame host.

use std::time::Duration;

use thrum_render::{capabilities::DeviceCapabilities, present::SurfaceProfile};
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use crate::{FrameConfig, FrameHost, FrameSample, SurfaceOutput};

const FRAMES: usize = 600;

/// One browser canvas, WebGPU surface and serial V1 game host.
#[wasm_bindgen]
pub struct BrowserGame {
    canvas: HtmlCanvasElement,
    adapter_name: String,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    queue: wgpu::Queue,
    host: FrameHost,
    first_frame_ms: Option<f64>,
    last_frame: Duration,
    visible: bool,
    callbacks: u64,
    presented: u64,
    unavailable: u64,
    overload_events: u64,
    refused_ticks: u64,
    timestamp_clamps: u64,
    needs_reconfigure: bool,
    samples: Vec<FrameSample>,
}

/// Prepare the existing V1 simulation and renderer for a WebGPU canvas.
///
/// # Errors
/// Returns a JavaScript error when no compatible adapter, surface format or GPU resource exists.
#[wasm_bindgen]
pub async fn create_browser_game(canvas: HtmlCanvasElement) -> Result<BrowserGame, JsValue> {
    std::panic::set_hook(Box::new(|info| {
        web_sys::console::error_1(&JsValue::from_str(&info.to_string()));
    }));
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = wgpu::Backends::BROWSER_WEBGPU;
    let instance = wgpu::Instance::new(descriptor);
    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
        .map_err(js_error)?;
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            compatible_surface: Some(&surface),
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        })
        .await
        .map_err(js_error)?;
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            required_features: DeviceCapabilities::requested_timing_features(&adapter),
            ..Default::default()
        })
        .await
        .map_err(js_error)?;
    let capabilities = DeviceCapabilities::capture(&adapter, &device, FrameHost::DEVICE_GENERATION);
    let mut config = surface
        .get_default_config(&adapter, canvas.width().max(1), canvas.height().max(1))
        .ok_or_else(|| js_error("no compatible canvas surface configuration"))?;
    let formats = surface.get_capabilities(&adapter).formats;
    config.format = [
        wgpu::TextureFormat::Bgra8Unorm,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Bgra8UnormSrgb,
        wgpu::TextureFormat::Rgba8UnormSrgb,
    ]
    .into_iter()
    .find(|format| formats.contains(format))
    .ok_or_else(|| js_error("no compatible 8-bit SDR canvas format"))?;
    surface.configure(&device, &config);
    let host = FrameHost::new_with_capabilities(
        &device,
        &queue,
        &capabilities,
        FrameConfig {
            render_extent: [config.width, config.height],
            surface: Some(SurfaceProfile {
                format: config.format,
                extent: [config.width, config.height],
            }),
        },
    )
    .await
    .map_err(js_error)?;
    let adapter_info = adapter.get_info();
    Ok(BrowserGame {
        canvas,
        adapter_name: if adapter_info.name.is_empty() {
            "browser WebGPU (adapter name withheld)".to_owned()
        } else {
            adapter_info.name
        },
        surface,
        config,
        device,
        queue,
        host,
        first_frame_ms: None,
        last_frame: Duration::ZERO,
        visible: true,
        callbacks: 0,
        presented: 0,
        unavailable: 0,
        overload_events: 0,
        refused_ticks: 0,
        timestamp_clamps: 0,
        needs_reconfigure: false,
        samples: Vec::with_capacity(FRAMES),
    })
}

#[wasm_bindgen]
impl BrowserGame {
    /// Advance the fixed clock and encode the scene before acquiring a drawable.
    ///
    /// # Errors
    /// Returns a JavaScript error for an authoritative tick or GPU host fault.
    pub fn frame(&mut self, timestamp_ms: f64) -> Result<(), JsValue> {
        let now = self.time_from_browser(timestamp_ms)?;
        if !self.visible {
            return Ok(());
        }
        self.callbacks += 1;
        self.last_frame = now;
        let extent = [self.canvas.width(), self.canvas.height()];
        let mut resize = extent != [self.config.width, self.config.height];
        if (resize || self.needs_reconfigure)
            && extent[0] > 0
            && extent[1] > 0
            && self.host.pending_frames() == 0
        {
            self.config.width = extent[0];
            self.config.height = extent[1];
            self.surface.configure(&self.device, &self.config);
            self.host.resize_surface(extent).map_err(js_error)?;
            self.needs_reconfigure = false;
            resize = false;
        }
        let advanced = self.host.advance(now).map_err(js_error)?;
        let can_acquire = advanced.can_acquire_serial()
            && extent[0] > 0
            && extent[1] > 0
            && !resize
            && !self.needs_reconfigure;
        let report = if can_acquire {
            let encoded = advanced.encode_scene().map_err(js_error)?;
            let frame = match self.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(frame) => Some(frame),
                wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                    self.needs_reconfigure = true;
                    Some(frame)
                }
                wgpu::CurrentSurfaceTexture::Lost | wgpu::CurrentSurfaceTexture::Outdated => {
                    self.needs_reconfigure = true;
                    self.unavailable += 1;
                    None
                }
                wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                    self.unavailable += 1;
                    None
                }
                wgpu::CurrentSurfaceTexture::Validation => {
                    return Err(js_error("canvas surface validation fault"));
                }
            };
            if let Some(frame) = frame {
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());
                let report = encoded
                    .submit(Some(&SurfaceOutput {
                        view: &view,
                        extent: [self.config.width, self.config.height],
                    }))
                    .map_err(js_error)?;
                if report.surface_presented {
                    self.queue.present(frame);
                    self.presented += 1;
                }
                report
            } else {
                encoded.skip()
            }
        } else {
            advanced.submit(None).map_err(js_error)?
        };
        if let Some(refused_ticks) = report.overload {
            self.recover_overload(refused_ticks, now)?;
        }
        while let Some(sample) = self.host.poll_completed().map_err(js_error)? {
            if self.samples.len() < FRAMES {
                self.samples.push(sample);
            }
        }
        Ok(())
    }

    /// Sample normalized player movement for the next authoritative world tick.
    ///
    /// # Errors
    /// Returns a JavaScript error for a nonfinite or out-of-range axis.
    pub fn set_controls(&mut self, x: f32, y: f32) -> Result<(), JsValue> {
        self.host.set_controls([x, y, 0.0]).map_err(js_error)
    }

    /// Committed position of the one local player in this V1 fixture.
    #[must_use]
    pub fn controlled_position(&self) -> String {
        self.host.controlled_position().map_or_else(
            || "player unavailable".to_owned(),
            |position| format!("player x={:.3} y={:.3}", position[0], position[1]),
        )
    }

    /// Pause for a hidden tab and restart the clock when it becomes visible.
    /// Queued accepted ticks drain before resume; hidden wall time is not admitted.
    ///
    /// # Errors
    /// Returns a JavaScript error if a retained tick or clock restart fails.
    pub fn set_visible(&mut self, visible: bool, timestamp_ms: f64) -> Result<(), JsValue> {
        let now = self.time_from_browser(timestamp_ms)?;
        if visible && !self.visible {
            self.host.drain_and_resume(now).map_err(js_error)?;
        } else if !visible && self.visible {
            self.host.pause();
        }
        self.visible = visible;
        self.last_frame = now;
        Ok(())
    }

    /// Number of surface frames actually presented.
    #[must_use]
    pub fn presented(&self) -> u64 {
        self.presented
    }

    /// True after 600 GPU-complete frame samples have been collected.
    #[must_use]
    pub fn complete(&self) -> bool {
        self.samples.len() == FRAMES
    }

    /// Final callback, world and GPU report after 600 completed frames.
    #[must_use]
    pub fn report(&self) -> String {
        let timed = self
            .samples
            .iter()
            .filter(|sample| sample.gpu_pass_ns.is_some() && sample.gpu_present_ns.is_some())
            .count();
        let tick = percentile(&self.samples, |sample| sample.tick_cpu.as_secs_f64() * 1e6);
        let encode = percentile(&self.samples, |sample| {
            sample.encode_cpu.as_secs_f64() * 1e6
        });
        let gpu = if timed == self.samples.len() && timed > 0 {
            let near = percentile(&self.samples, |sample| {
                sample.gpu_pass_ns.map_or(0.0, |passes| passes[0]) / 1e6
            });
            let far = percentile(&self.samples, |sample| {
                sample.gpu_pass_ns.map_or(0.0, |passes| passes[1]) / 1e6
            });
            let cluster = percentile(&self.samples, |sample| {
                sample.gpu_pass_ns.map_or(0.0, |passes| passes[2]) / 1e6
            });
            let forward = percentile(&self.samples, |sample| {
                sample.gpu_pass_ns.map_or(0.0, |passes| passes[3]) / 1e6
            });
            let display = percentile(&self.samples, |sample| {
                sample.gpu_pass_ns.map_or(0.0, |passes| passes[4]) / 1e6
            });
            let present = percentile(&self.samples, |sample| {
                sample.gpu_present_ns.unwrap_or_default() / 1e6
            });
            format!(
                "GPU near/far/cluster/forward/display/present p50-p95 ms={:.3}/{:.3}, {:.3}/{:.3}, {:.3}/{:.3}, {:.3}/{:.3}, {:.3}/{:.3}, {:.3}/{:.3}",
                near.0,
                near.1,
                far.0,
                far.1,
                cluster.0,
                cluster.1,
                forward.0,
                forward.1,
                display.0,
                display.1,
                present.0,
                present.1
            )
        } else {
            "GPU timestamps unavailable or incomplete".to_owned()
        };
        format!(
            "adapter={} format={:?} extent={}x{}; callbacks={} presented={} samples={} ticks={} elapsed={:.3}s unavailable={} overload_events={} refused_ticks={} timestamp_clamps={} shed_views={} report_drops={} GPU_timed={} latest_visual_age={}; tick CPU p50/p95={:.1}/{:.1} us; encode CPU p50/p95={:.1}/{:.1} us; {gpu}",
            self.adapter_name,
            self.config.format,
            self.config.width,
            self.config.height,
            self.callbacks,
            self.presented,
            self.samples.len(),
            self.host.tick(),
            self.last_frame.as_secs_f64(),
            self.unavailable,
            self.overload_events,
            self.refused_ticks,
            self.timestamp_clamps,
            self.host.dropped_views(),
            self.host.dropped_reports(),
            timed,
            self.samples
                .last()
                .map_or(0, |sample| sample.visual_age_ticks),
            tick.0,
            tick.1,
            encode.0,
            encode.1,
        )
    }
}

impl BrowserGame {
    fn recover_overload(&mut self, refused_ticks: u64, now: Duration) -> Result<(), JsValue> {
        self.overload_events += 1;
        self.refused_ticks = self.refused_ticks.saturating_add(refused_ticks);
        self.host.pause();
        self.host.drain_and_resume(now).map_err(js_error)
    }

    fn time_from_browser(&mut self, timestamp_ms: f64) -> Result<Duration, JsValue> {
        if !timestamp_ms.is_finite() || timestamp_ms < 0.0 {
            return Err(js_error(
                "animation timestamp must be finite and nonnegative",
            ));
        }
        let first = *self.first_frame_ms.get_or_insert(timestamp_ms);
        let now = Duration::try_from_secs_f64((timestamp_ms - first).max(0.0) / 1_000.0)
            .map_err(|_| js_error("animation timestamp outside supported range"))?;
        if now < self.last_frame {
            // RAF reports the frame-start time; a visibility event samples
            // performance.now() later in that same frame.
            self.timestamp_clamps += 1;
            return Ok(self.last_frame);
        }
        Ok(now)
    }
}

fn percentile(samples: &[FrameSample], value: impl Fn(&FrameSample) -> f64) -> (f64, f64) {
    if samples.is_empty() {
        return (0.0, 0.0);
    }
    let mut values: Vec<_> = samples.iter().map(value).collect();
    values.sort_by(f64::total_cmp);
    (values[values.len() / 2], values[values.len() * 95 / 100])
}

fn js_error(value: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&value.to_string())
}
