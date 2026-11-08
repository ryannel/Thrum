//! Browser shell shared by Thrum games (wasm32 only).
//!
//! [`WebShell`] owns a game's prepared [`FrameHost`] together with the WebGPU canvas surface,
//! its device, the device-loss signal and detached device replacement. The page keeps its
//! `requestAnimationFrame` loop, DOM events and input, and calls [`WebShell::frame`],
//! [`WebShell::set_visible`] and [`WebShell::set_paused`] with page timestamps; the shell installs
//! no listener. A game keeps its report text, controls and authored scene, reading typed
//! counters from [`WebShell::status`] and [`WebShell::samples`].
//!
//! Loss policy: once loss is observed, no call reaches the lost device. Frames only advance
//! authority; edits through [`WebShell::world_mut`] and [`WebShell::source_mut`] (CPU state)
//! stay allowed and the replacement draws them; capture is refused; a scene contribution
//! attached while lost is held and attached right after the replacement is published.

use std::{
    future::Future,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use thrum_render::{
    capabilities::DeviceCapabilities,
    contribution::SceneContribution,
    present::SurfaceProfile,
    runtime::{
        FrameConfig, FrameHost, FrameSample, FrameWorld, PendingCapture, SceneSource, SurfaceOutput,
    },
};
use wasm_bindgen::JsValue;
use web_sys::HtmlCanvasElement;

use crate::web_clock::{PageClock, Transition};

mod recovery;

/// Frame samples kept per device generation, the size of one report.
pub const SAMPLES: usize = 600;

/// The world's lifecycle as the shell drives it. The world owns its clock and the drain of
/// accepted ticks; the shell decides only when to pause and when to resume.
pub trait WebWorld: FrameWorld {
    /// Stop admitting wall time, keeping accepted ticks.
    /// # Errors
    /// Returns a world lifecycle refusal.
    fn pause(&mut self) -> Result<(), String>;
    /// Resume from `now`, or once the accepted ticks have drained on later frames.
    /// # Errors
    /// Returns a clock state or timestamp refusal.
    fn resume_after_drain(&mut self, now: Duration) -> Result<(), String>;
}

/// The first device, handed to the game to prepare its runtime.
pub struct WebGpu {
    /// The first device; replacements reach the runtime through recovery.
    pub device: wgpu::Device,
    /// The device's queue.
    pub queue: wgpu::Queue,
    /// The recorded capabilities the runtime prepares against.
    pub capabilities: DeviceCapabilities,
    /// The configured canvas extent and surface profile.
    pub frame_config: FrameConfig,
}

/// Typed counters and state behind a game's report.
#[derive(Clone, Debug)]
pub struct WebStatus {
    /// The current device's adapter.
    pub adapter_name: String,
    /// The canvas surface format.
    pub format: wgpu::TextureFormat,
    /// The configured canvas extent.
    pub extent: [u32; 2],
    /// Admitted visible animation callbacks.
    pub callbacks: u64,
    /// Surface presentation calls, not physical displayed frames.
    pub presented: u64,
    /// Callbacks that could not present: no drawable, or the device was lost.
    pub unavailable: u64,
    /// Overloads the world reported; each pauses and resumes after the drain.
    pub overload_events: u64,
    /// Ticks the world refused across those overloads.
    pub refused_ticks: u64,
    /// Page timestamps earlier than the last admitted one, clamped to it.
    pub timestamp_clamps: u64,
    /// Time of the last admitted callback since the first page timestamp.
    pub last_frame: Duration,
    /// The current device generation.
    pub generation: u64,
    /// Explicit replacement attempts.
    pub recovery_attempts: u64,
    /// Replacement devices published.
    pub recovered: u64,
}

/// One canvas, its WebGPU device and a game's frame runtime.
pub struct WebShell<W: WebWorld, V: SceneSource<W::Page>> {
    canvas: HtmlCanvasElement,
    adapter_name: String,
    surface: Rc<wgpu::Surface<'static>>,
    recovery: recovery::Recovery,
    config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    graphics: Arc<GraphicsFault>,
    queue: wgpu::Queue,
    host: FrameHost<W, V>,
    clock: PageClock,
    callbacks: u64,
    presented: u64,
    unavailable: u64,
    overload_events: u64,
    refused_ticks: u64,
    needs_reconfigure: bool,
    samples: Vec<FrameSample>,
    held_contribution: Option<Box<dyn SceneContribution>>,
    attach_error: Option<String>,
}

/// Create the canvas surface and first device, and let the game prepare its runtime on it.
/// `generation` numbers the first device; replacements count up from it.
/// # Errors
/// Returns no compatible adapter, surface format or device, or the game's preparation failure.
pub async fn create<W, V, F, Fut>(
    canvas: HtmlCanvasElement,
    generation: u64,
    prepare: F,
) -> Result<WebShell<W, V>, String>
where
    W: WebWorld,
    V: SceneSource<W::Page>,
    F: FnOnce(WebGpu) -> Fut,
    Fut: Future<Output = Result<FrameHost<W, V>, String>>,
{
    std::panic::set_hook(Box::new(|info| {
        web_sys::console::error_1(&JsValue::from_str(&info.to_string()));
    }));
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = wgpu::Backends::BROWSER_WEBGPU;
    let instance = wgpu::Instance::new(descriptor);
    let surface = Rc::new(
        instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(|error| error.to_string())?,
    );
    let gpu = recovery::request_device(
        &instance,
        &surface,
        [canvas.width().max(1), canvas.height().max(1)],
        generation,
    )
    .await?;
    surface.configure(&gpu.device, &gpu.config);
    let extent = [gpu.config.width, gpu.config.height];
    let host = prepare(WebGpu {
        device: gpu.device.clone(),
        queue: gpu.queue.clone(),
        capabilities: gpu.capabilities.clone(),
        frame_config: FrameConfig {
            render_extent: extent,
            surface: Some(SurfaceProfile {
                format: gpu.config.format,
                extent,
            }),
        },
    })
    .await?;
    let recovery = recovery::Recovery::new(instance, Rc::clone(&surface), generation);
    Ok(WebShell {
        canvas,
        adapter_name: gpu.adapter_name,
        surface,
        recovery,
        config: gpu.config,
        device: gpu.device,
        graphics: gpu.fault,
        queue: gpu.queue,
        host,
        clock: PageClock::new(),
        callbacks: 0,
        presented: 0,
        unavailable: 0,
        overload_events: 0,
        refused_ticks: 0,
        needs_reconfigure: false,
        samples: Vec::with_capacity(SAMPLES),
        held_contribution: None,
        attach_error: None,
    })
}

impl<W: WebWorld, V: SceneSource<W::Page>> WebShell<W, V> {
    /// Advance the world and, with a healthy device, encode before acquiring the canvas
    /// drawable, submit and present. After loss it only advances authority.
    /// # Errors
    /// Returns a timestamp, authoritative tick or GPU host fault.
    pub fn frame(&mut self, timestamp_ms: f64) -> Result<(), String> {
        let now = self.clock.now(timestamp_ms)?;
        if !self.clock.visible() {
            return Ok(());
        }
        self.callbacks += 1;
        self.clock.mark(now);
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
            // The replacement installed free frame slots, so a held attach meets the
            // no-frames-in-flight rule before the new device's first encode.
            if let Some(held) = self.held_contribution.take()
                && let Err(error) = self.host.set_scene_contribution(held)
            {
                self.attach_error = Some(error);
            }
        }
        if self.graphics.is_lost() {
            self.recovery.awaiting_first_present = false;
            self.unavailable += 1;
            // No polling, completion collection, resize or GPU encoding after
            // loss. Authority and controls continue on the same resident world.
            let report = self
                .host
                .advance_without_rendering(now)
                .map_err(|error| error.to_string())?;
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
            self.host
                .resize_surface(extent)
                .map_err(|error| error.to_string())?;
            self.needs_reconfigure = false;
            resize = false;
        }
        let advanced = self.host.advance(now).map_err(|error| error.to_string())?;
        let can_acquire = advanced.can_acquire_serial()
            && extent[0] > 0
            && extent[1] > 0
            && !resize
            && !self.needs_reconfigure;
        let report = if can_acquire {
            let encoded = advanced.encode_scene().map_err(|error| error.to_string())?;
            let frame = if encoded.can_acquire_surface() {
                match self.surface.get_current_texture() {
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
                    wgpu::CurrentSurfaceTexture::Timeout
                    | wgpu::CurrentSurfaceTexture::Occluded => {
                        self.unavailable += 1;
                        None
                    }
                    wgpu::CurrentSurfaceTexture::Validation => {
                        return Err("canvas surface validation fault".to_owned());
                    }
                }
            } else {
                None
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
                    .map_err(|error| error.to_string())?;
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
            advanced.submit(None).map_err(|error| error.to_string())?
        };
        if let Some(refused_ticks) = report.overload {
            self.recover_overload(refused_ticks, now)?;
        }
        while let Some(sample) = self
            .host
            .poll_completed()
            .map_err(|error| error.to_string())?
        {
            if self.samples.len() < SAMPLES {
                self.samples.push(sample);
            }
        }
        Ok(())
    }

    /// Pause for a hidden page and restart the clock when it becomes visible.
    /// Queued accepted ticks drain before resume; hidden wall time is not admitted.
    /// # Errors
    /// Returns a timestamp, retained-update or clock restart failure.
    pub fn set_visible(&mut self, visible: bool, timestamp_ms: f64) -> Result<(), String> {
        let now = self.clock.now(timestamp_ms)?;
        self.set_play(visible, self.clock.user_paused(), now)
    }

    /// Stop admitting world time at the person's request. Visibility cannot override it.
    /// # Errors
    /// Returns a timestamp, retained-update or clock restart failure.
    pub fn set_paused(&mut self, paused: bool, timestamp_ms: f64) -> Result<(), String> {
        let now = self.clock.now(timestamp_ms)?;
        self.set_play(self.clock.visible(), paused, now)
    }

    /// Whether the person paused the world, independent of page visibility.
    pub fn paused(&self) -> bool {
        self.clock.user_paused()
    }

    /// Persistent device-loss diagnostic, or `None` while the device is healthy.
    pub fn graphics_fault(&self) -> Option<String> {
        self.graphics.message()
    }

    /// Whether the current device is lost. GPU-touching game work checks this first.
    pub fn graphics_lost(&self) -> bool {
        self.graphics.is_lost()
    }

    /// Start detached device replacement without borrowing the running world.
    /// # Errors
    /// Refuses a healthy device, a duplicate attempt or invalid retained inputs.
    pub fn request_graphics_recovery(&mut self) -> Result<(), String> {
        if !self.graphics.is_lost() {
            return Err("graphics device is still available".to_owned());
        }
        self.recovery
            .request(&self.host, [self.config.width, self.config.height])
    }

    /// Whether an explicit attempt is preparing or awaiting its first present call.
    pub fn recovering(&self) -> bool {
        self.recovery.is_pending() || self.recovery.awaiting_first_present
    }

    /// Last failed replacement attempt; a fresh explicit request clears it.
    pub fn recovery_error(&self) -> Option<&str> {
        self.recovery.error.as_deref()
    }

    /// The game's runtime, for reads.
    pub fn host(&self) -> &FrameHost<W, V> {
        &self.host
    }

    /// The world, for CPU edits such as input. Allowed while the device is lost.
    pub fn world_mut(&mut self) -> &mut W {
        self.host.world_mut()
    }

    /// The scene source, for CPU edits such as camera or solar time. Allowed while the device
    /// is lost. Like [`FrameHost::source_mut`], it resets presentation history, so compare
    /// through [`Self::host`] first and call it only for a change.
    pub fn source_mut(&mut self) -> &mut V {
        self.host.source_mut()
    }

    /// Attach a scene contribution. While the device is lost it is held, with no GPU call,
    /// and attached right after the replacement device is published; a later attach
    /// replaces a held one, and a refusal then is reported by [`Self::take_attach_error`].
    /// # Errors
    /// Returns the runtime's refusal of an immediate attach.
    pub fn set_scene_contribution(
        &mut self,
        contribution: Box<dyn SceneContribution>,
    ) -> Result<(), String> {
        if self.graphics.is_lost() {
            self.held_contribution = Some(contribution);
            return Ok(());
        }
        self.held_contribution = None;
        self.host.set_scene_contribution(contribution)
    }

    /// The refusal of a held contribution attached on a replacement device, once.
    pub fn take_attach_error(&mut self) -> Option<String> {
        self.attach_error.take()
    }

    /// Submit a capture of the committed endpoint. Refused while the device is lost.
    /// # Errors
    /// Refuses a lost device, or returns a recording failure.
    pub fn begin_capture(&mut self) -> Result<PendingCapture, String> {
        if self.graphics.is_lost() {
            return Err("graphics device is lost".to_owned());
        }
        self.host.begin_capture().map_err(|error| error.to_string())
    }

    /// The capture's sRGB RGBA8 pixels once ready. A capture outstanding at loss is refused.
    /// # Errors
    /// Returns a lost device, a retired device generation or a readback failure.
    pub fn try_finish_capture(
        &mut self,
        pending: &mut PendingCapture,
    ) -> Result<Option<Vec<u8>>, String> {
        if self.graphics.is_lost() {
            return Err("graphics device was lost before the capture finished".to_owned());
        }
        self.host
            .try_finish_capture(pending)
            .map_err(|error| error.to_string())
    }

    /// Counters and state for the game's report.
    pub fn status(&self) -> WebStatus {
        WebStatus {
            adapter_name: self.adapter_name.clone(),
            format: self.config.format,
            extent: [self.config.width, self.config.height],
            callbacks: self.callbacks,
            presented: self.presented,
            unavailable: self.unavailable,
            overload_events: self.overload_events,
            refused_ticks: self.refused_ticks,
            timestamp_clamps: self.clock.clamps(),
            last_frame: self.clock.last(),
            generation: self.recovery.generation,
            recovery_attempts: self.recovery.attempts,
            recovered: self.recovery.recovered,
        }
    }

    /// Surface presentation calls, not physical displayed frames.
    pub fn presented(&self) -> u64 {
        self.presented
    }

    /// GPU-complete frame samples of the current device generation, at most [`SAMPLES`].
    pub fn samples(&self) -> &[FrameSample] {
        &self.samples
    }

    fn set_play(&mut self, visible: bool, user_paused: bool, now: Duration) -> Result<(), String> {
        match self.clock.set_play(visible, user_paused, now) {
            Transition::Resume => self.host.world_mut().resume_after_drain(now),
            Transition::Pause => self.host.world_mut().pause(),
            Transition::None => Ok(()),
        }
    }

    fn recover_overload(&mut self, refused_ticks: u64, now: Duration) -> Result<(), String> {
        self.overload_events += 1;
        self.refused_ticks = self.refused_ticks.saturating_add(refused_ticks);
        self.host.world_mut().pause()?;
        self.host.world_mut().resume_after_drain(now)
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
