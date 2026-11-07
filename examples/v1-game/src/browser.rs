//! Browser WebGPU surface adapter for the same fixed-step V1 frame host.

use std::time::Duration;
use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use thrum_asset::{BrowserFetch, DevClient};
use thrum_asset_gpu::{CookedSource, CookedStats, RemoteFeed};
use thrum_render::present::SurfaceProfile;
use wasm_bindgen::prelude::*;
use web_sys::HtmlCanvasElement;

use crate::{FrameConfig, FrameHost, FrameSample, SurfaceOutput};

mod recovery;

const FRAMES: usize = 600;

/// The cooked valley's dev-server client and the feed of generations it has fetched.
#[wasm_bindgen]
pub struct CookedValley {
    /// Taken out while a poll runs, so overlapping polls are refused.
    client: Rc<RefCell<Option<DevClient<BrowserFetch>>>>,
    feed: RemoteFeed,
}

/// Fetch the current cooked publication from a dev server origin such as
/// `http://127.0.0.1:7878`. Every product is verified against its content address.
///
/// # Errors
/// Returns a malformed origin, a transport failure, an empty store or a product mismatch.
#[wasm_bindgen]
pub async fn fetch_cooked_valley(origin: String) -> Result<CookedValley, JsValue> {
    let mut client = DevClient::new(BrowserFetch, &origin).map_err(js_error)?;
    let generation = client
        .poll()
        .await
        .map_err(js_error)?
        .ok_or_else(|| JsValue::from_str("the dev server has no publication yet"))?;
    Ok(CookedValley {
        client: Rc::new(RefCell::new(Some(client))),
        feed: RemoteFeed::new(generation),
    })
}

#[wasm_bindgen]
impl CookedValley {
    /// Long-poll the dev server once. Resolves `true` when a newer publication was fetched
    /// (the attached valley stages it at its next frame) and `false` when the poll timed out.
    /// Call again after it settles; overlapping calls are refused.
    pub fn next_publication(&self) -> js_sys::Promise {
        let client = Rc::clone(&self.client);
        let feed = self.feed.clone();
        wasm_bindgen_futures::future_to_promise(async move {
            let mut polling = client
                .borrow_mut()
                .take()
                .ok_or_else(|| JsValue::from_str("a cooked publication poll is already running"))?;
            let fetched = polling.poll().await;
            *client.borrow_mut() = Some(polling);
            let fetched = fetched.map_err(js_error)?;
            let changed = fetched.is_some();
            if let Some(generation) = fetched {
                feed.publish(generation);
            }
            Ok(JsValue::from_bool(changed))
        })
    }
}

/// One browser canvas, WebGPU surface and serial V1 game host.
#[wasm_bindgen]
pub struct BrowserGame {
    canvas: HtmlCanvasElement,
    adapter_name: String,
    surface: Rc<wgpu::Surface<'static>>,
    recovery: recovery::Recovery,
    config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    graphics: Arc<GraphicsFault>,
    queue: wgpu::Queue,
    host: FrameHost,
    cooked: Option<Arc<Mutex<CookedStats>>>,
    first_frame_ms: Option<f64>,
    last_frame: Duration,
    visible: bool,
    user_paused: bool,
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
    let surface = Rc::new(
        instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(js_error)?,
    );
    let gpu = recovery::request_device(
        &instance,
        &surface,
        [canvas.width().max(1), canvas.height().max(1)],
        FrameHost::DEVICE_GENERATION,
    )
    .await
    .map_err(js_error)?;
    surface.configure(&gpu.device, &gpu.config);
    let host = FrameHost::new_with_capabilities(
        &gpu.device,
        &gpu.queue,
        &gpu.capabilities,
        FrameConfig {
            render_extent: [gpu.config.width, gpu.config.height],
            surface: Some(SurfaceProfile {
                format: gpu.config.format,
                extent: [gpu.config.width, gpu.config.height],
            }),
        },
    )
    .await
    .map_err(js_error)?;
    let recovery = recovery::Recovery::new(instance, Rc::clone(&surface));
    Ok(BrowserGame {
        canvas,
        adapter_name: gpu.adapter_name,
        surface,
        recovery,
        config: gpu.config,
        device: gpu.device,
        graphics: gpu.fault,
        queue: gpu.queue,
        host,
        cooked: None,
        first_frame_ms: None,
        last_frame: Duration::ZERO,
        visible: true,
        user_paused: false,
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
    /// Draw the authored valley from fetched cooked products, with its authored camera.
    /// The products survive device replacement: a new device stages the newest fetched
    /// generation again. Publications fetched later by [`CookedValley::next_publication`] swap
    /// in as they upload.
    /// # Errors
    /// Returns an invalid embedded recipe or the scene's refusal of the contribution.
    pub fn attach_cooked_valley(&mut self, valley: &CookedValley) -> Result<(), JsValue> {
        let (camera, assets) =
            crate::showcase::prepare_showcase(CookedSource::Remote(valley.feed.clone()))
                .map_err(js_error)?;
        let stats = assets.stats();
        self.host
            .set_scene_contribution(Box::new(assets))
            .map_err(js_error)?;
        self.host.set_scene_camera(camera);
        self.cooked = Some(stats);
        Ok(())
    }

    /// Cooked generation drawn, swaps and loader errors, or `None` without cooked content.
    pub fn cooked_status(&self) -> Option<String> {
        let stats = self.cooked.as_ref()?.lock().ok()?;
        Some(format!(
            "{{\"drawn\":{},\"swaps\":{},\"uploadedBytes\":{},\"submittedFrames\":{},\"errors\":{:?}}}",
            stats
                .drawn
                .map_or("null".to_owned(), |generation| generation.to_string()),
            stats.swaps.len(),
            stats.swaps.last().map_or(0, |swap| swap.uploaded_bytes),
            stats.submitted_frames,
            stats.errors,
        ))
    }

    /// Select the solar time without changing the authoritative world clock.
    /// Zero/one is midnight, one quarter sunrise and three quarters sunset.
    /// # Errors
    /// Refuses a nonfinite time or one outside the normalized day.
    pub fn set_time_of_day(&mut self, fraction: f32) -> Result<(), JsValue> {
        self.host.set_time_of_day(fraction).map_err(js_error)
    }

    /// Observe the solar time of the latest committed world endpoint.
    pub fn time_of_day(&self) -> f32 {
        self.host.time_of_day()
    }

    /// Look up/down within the scene's supported camera range.
    /// # Errors
    /// Refuses a nonfinite pitch or one outside plus/minus 0.6 radians.
    pub fn set_camera_pitch(&mut self, radians: f32) -> Result<(), JsValue> {
        self.host.set_camera_pitch(radians).map_err(js_error)
    }

    /// Turn the camera within plus/minus pi radians.
    /// # Errors
    /// Refuses a nonfinite yaw or one outside that range.
    pub fn set_camera_yaw(&mut self, radians: f32) -> Result<(), JsValue> {
        self.host.set_camera_yaw(radians).map_err(js_error)
    }

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
        if let Some((gpu, reconfigure)) = self.recovery.poll(&mut self.host, extent) {
            self.adapter_name = gpu.adapter_name;
            self.config = gpu.config;
            self.device = gpu.device;
            self.queue = gpu.queue;
            self.graphics = gpu.fault;
            self.needs_reconfigure = reconfigure;
            // Timings belong to one device generation, while world/platform
            // counters retain their identity across graphics reconstruction.
            self.samples.clear();
        }
        if self.graphics.is_lost() {
            self.recovery.awaiting_first_present = false;
            self.unavailable += 1;
            // No polling, completion collection, resize or GPU encoding after
            // loss. Authority and controls continue on the same resident world.
            let report = self.host.advance_without_rendering(now).map_err(js_error)?;
            if let Some(refused_ticks) = report.overload {
                self.recover_overload(refused_ticks, now)?;
            }
            return Ok(());
        }
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
                    self.recovery.awaiting_first_present = false;
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
        self.set_play_state(visible, self.user_paused, now)
    }

    /// Stop admitting world time while retaining camera and sky controls.
    /// Already accepted updates drain through ordinary frame callbacks.
    /// Tab visibility cannot override this explicit local pause preference.
    /// # Errors
    /// Returns a timestamp, retained-update or clock restart failure.
    pub fn set_paused(&mut self, paused: bool, timestamp_ms: f64) -> Result<(), JsValue> {
        let now = self.time_from_browser(timestamp_ms)?;
        self.set_play_state(self.visible, paused, now)
    }

    /// Whether the local user requested pause, independent of tab visibility.
    pub fn paused(&self) -> bool {
        self.user_paused
    }

    /// Persistent device-loss diagnostic. Rendering stops while the world and
    /// local controls remain usable until a complete replacement is published.
    pub fn graphics_fault(&self) -> Option<String> {
        self.graphics.message()
    }

    /// Start detached graphics reconstruction without borrowing the running world.
    /// # Errors
    /// Refuses a healthy device, duplicate attempt or invalid retained inputs.
    pub fn request_graphics_recovery(&mut self) -> Result<(), JsValue> {
        if !self.graphics.is_lost() {
            return Err(js_error("graphics device is still available"));
        }
        self.recovery
            .request(&self.host, [self.config.width, self.config.height])
            .map_err(js_error)
    }

    /// Whether an explicit attempt is preparing or awaiting its first present call.
    pub fn recovering(&self) -> bool {
        self.recovery.is_pending() || self.recovery.awaiting_first_present
    }

    /// Last failed reconstruction attempt; a fresh explicit request clears it.
    pub fn graphics_recovery_error(&self) -> Option<String> {
        self.recovery.error.clone()
    }

    /// Number of surface presentation calls, not physical displayed frames.
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
            let history = percentile(&self.samples, |sample| {
                sample.gpu_pass_ns.map_or(0.0, |passes| passes[4]) / 1e6
            });
            let display = percentile(&self.samples, |sample| {
                sample.gpu_pass_ns.map_or(0.0, |passes| passes[5]) / 1e6
            });
            let present = percentile(&self.samples, |sample| {
                sample.gpu_present_ns.unwrap_or_default() / 1e6
            });
            format!(
                "GPU near/far/cluster/forward/history/display/present p50-p95 ms={:.3}/{:.3}, {:.3}/{:.3}, {:.3}/{:.3}, {:.3}/{:.3}, {:.3}/{:.3}, {:.3}/{:.3}, {:.3}/{:.3}",
                near.0,
                near.1,
                far.0,
                far.1,
                cluster.0,
                cluster.1,
                forward.0,
                forward.1,
                history.0,
                history.1,
                display.0,
                display.1,
                present.0,
                present.1
            )
        } else {
            "GPU timestamps unavailable or incomplete".to_owned()
        };
        format!(
            "adapter={} format={:?} extent={}x{}; callbacks={} presented={} samples={} ticks={} elapsed={:.3}s unavailable={} overload_events={} refused_ticks={} timestamp_clamps={} shed_views={} report_drops={} generation={} recovery_attempts={} recovered={} GPU_timed={} latest_visual_age={}; tick CPU p50/p95={:.1}/{:.1} us; encode CPU p50/p95={:.1}/{:.1} us; {gpu}",
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
            self.recovery.generation,
            self.recovery.attempts,
            self.recovery.recovered,
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
    fn set_play_state(
        &mut self,
        visible: bool,
        user_paused: bool,
        now: Duration,
    ) -> Result<(), JsValue> {
        let was_running = self.visible && !self.user_paused;
        let will_run = visible && !user_paused;
        if will_run && !was_running {
            self.host.drain_and_resume(now).map_err(js_error)?;
        } else if !will_run && was_running {
            self.host.pause().map_err(js_error)?;
        }
        self.visible = visible;
        self.user_paused = user_paused;
        self.last_frame = now;
        Ok(())
    }

    fn recover_overload(&mut self, refused_ticks: u64, now: Duration) -> Result<(), JsValue> {
        self.overload_events += 1;
        self.refused_ticks = self.refused_ticks.saturating_add(refused_ticks);
        self.host.pause().map_err(js_error)?;
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

// The callback owns no world/renderer reference. The hot path observes one
// atomic; the cold diagnostic allocates only when loss is reported/read.
struct GraphicsFault {
    lost: AtomicBool,
    first: Mutex<Option<String>>,
}
impl GraphicsFault {
    fn register(device: &wgpu::Device) -> Arc<Self> {
        let fault = Arc::new(Self {
            lost: AtomicBool::new(false),
            first: Mutex::new(None),
        });
        let signal = Arc::clone(&fault);
        device.set_device_lost_callback(move |reason, message| {
            if let Ok(mut first) = signal.first.lock()
                && first.is_none()
            {
                *first = Some(format!("{reason:?}: {message}"));
            }
            signal.lost.store(true, Ordering::Release);
        });
        fault
    }
    fn is_lost(&self) -> bool {
        self.lost.load(Ordering::Acquire)
    }
    fn message(&self) -> Option<String> {
        if !self.is_lost() {
            return None;
        }
        Some(
            self.first
                .lock()
                .ok()
                .and_then(|first| first.clone())
                .unwrap_or_else(|| "Graphics device lost".to_owned()),
        )
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
