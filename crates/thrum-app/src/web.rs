//! Browser shell shared by Thrum games (wasm32 only).
//!
//! [`WebShell`] owns a game's prepared [`FrameHost`] together with the WebGPU canvas surface,
//! its device, the device-loss signal and detached device replacement. [`create_page`] exposes the
//! ordinary game through an engine-owned wasm interface. The shared page driver forwards animation,
//! input and visibility events. Explicit capture drivers can supply timestamps directly. A game keeps its report text, controls and authored scene, reading typed
//! counters from [`WebShell::status`] and [`WebShell::samples`].
//!
//! Loss policy: once loss is observed, no call reaches the lost device. Frames only advance
//! authority; edits through [`WebShell::world_mut`] and [`WebShell::source_mut`] (CPU state)
//! stay allowed and the replacement draws them; capture is refused; a scene contribution
//! attached while lost is held and attached right after the replacement is published.

use std::{error::Error, fmt, rc::Rc, time::Duration};

use thrum_core::sim::{WorldLifecycle, WorldLifecyclePolicy};
use thrum_render::{
    capabilities::{DeviceWatch, HardwareLaunchError, SurfaceConfigError, configure_surface},
    contribution::SceneContribution,
    governor::{DisplayRecord, ThermalState},
    present::SurfaceProfile,
    runtime::{FrameError, FrameHost, FrameSample, PendingCapture, SceneSource, SurfaceOutput},
};
use web_sys::HtmlCanvasElement;

use crate::{
    Game, GameError, Gpu, InlineWorld,
    game::apply_frame_edits,
    surface::{SurfaceAction, surface_action},
    web_clock::PageClock,
};

mod capture;
mod controls;
pub use controls::ControlBinding;
mod page;
mod page_exports;
pub use page::{ControlTiming, Page, PageGame, PageRuntime, create_page, parameter_json};
mod recovery;

/// Frame samples kept per device generation, the size of one report.
pub const SAMPLES: usize = 600;

/// Failure while starting or driving a hosted browser game.
#[derive(Debug)]
pub enum WebHostError {
    /// The browser canvas could not be turned into a graphics surface.
    Surface(wgpu::CreateSurfaceError),
    /// The browser device recipe could not open a compatible device.
    Device(HardwareLaunchError),
    /// The surface offers no compatible SDR canvas configuration.
    CanvasConfiguration,
    /// The device refused to configure the canvas surface.
    Configure(SurfaceConfigError),
    /// The game could not prepare its runtime.
    Prepare(GameError),
    /// The game refused presentation edits.
    Edit(GameError),
    /// The world refused a pause or resume.
    Lifecycle(GameError),
    /// A browser frame could not advance or present.
    Frame(FrameError),
    /// The browser timestamp cannot be represented on the page clock.
    Timestamp(&'static str),
    /// Graphics replacement was requested while the device is healthy.
    DeviceAvailable,
    /// A replacement is already being prepared.
    RecoveryPending,
    /// All graphics generation identifiers have been used.
    GenerationExhausted,
}

impl fmt::Display for WebHostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Surface(error) => error.fmt(formatter),
            Self::Configure(error) => error.fmt(formatter),
            Self::Prepare(error) | Self::Edit(error) | Self::Lifecycle(error) => {
                error.fmt(formatter)
            }
            Self::Frame(error) => error.fmt(formatter),
            Self::Device(error) => error.fmt(formatter),
            Self::CanvasConfiguration => {
                formatter.write_str("no compatible SDR canvas configuration")
            }
            Self::Timestamp(message) => formatter.write_str(message),
            Self::DeviceAvailable => formatter.write_str("graphics device is still available"),
            Self::RecoveryPending => formatter.write_str("graphics recovery is already preparing"),
            Self::GenerationExhausted => formatter.write_str("graphics generation exhausted"),
        }
    }
}

impl Error for WebHostError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Surface(error) => Some(error),
            Self::Configure(error) => Some(error),
            Self::Device(error) => Some(error),
            Self::Prepare(error) | Self::Edit(error) | Self::Lifecycle(error) => Some(error),
            Self::Frame(error) => Some(error),
            _ => None,
        }
    }
}

/// Typed counters and state behind a game's report.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct WebStatus {
    /// The current device's adapter.
    pub adapter_name: String,
    /// The canvas surface format.
    pub format: wgpu::TextureFormat,
    /// The configured canvas extent.
    pub extent: [u32; 2],
    /// Time spent preparing the most recent new canvas grid between frames.
    pub resize_prepare: Option<Duration>,
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
pub struct WebShell<W: InlineWorld, V: SceneSource<W::Page>> {
    canvas: HtmlCanvasElement,
    adapter_name: String,
    surface: Rc<wgpu::Surface<'static>>,
    recovery: recovery::Recovery,
    config: wgpu::SurfaceConfiguration,
    device: wgpu::Device,
    graphics: DeviceWatch,
    queue: wgpu::Queue,
    host: FrameHost<W, V>,
    clock: PageClock,
    callbacks: u64,
    presented: u64,
    unavailable: u64,
    lifecycle: WorldLifecyclePolicy,
    queued_ticks: u32,
    needs_reconfigure: bool,
    resize_prepare: Option<Duration>,
    samples: Vec<FrameSample>,
    profile: crate::profile::ProfileWindow,
    profile_minimum: u64,
    profile_enabled: bool,
    held_contribution: Option<Box<dyn SceneContribution>>,
    attach_error: Option<String>,
    governor: Option<BrowserCadence>,
    #[cfg(feature = "interface")]
    interface_output: Option<(f64, [f64; 4])>,

    #[cfg(feature = "interface")]
    reported_interface_output: Option<crate::OutputMetrics>,
}

#[derive(Default)]
struct BrowserCadence {
    callbacks: crate::web_cadence::FrameCadence,
    previous_present: Option<Duration>,
}

/// Create the canvas surface and first device, and let the game prepare its inline runtime.
/// `generation` numbers the first device; replacements count up from it.
/// # Errors
/// Returns a device, surface or game preparation failure.
pub async fn create_game<G: Game>(
    canvas: HtmlCanvasElement,
    generation: u64,
    game: &mut G,
) -> Result<WebShell<G::World, G::Scene>, WebHostError>
where
    G::World: InlineWorld,
{
    create_game_with_feature_mask(canvas, generation, game, wgpu::Features::empty()).await
}

/// Create a browser host with selected features removed from its device recipe.
///
/// # Errors
/// Returns a device, surface or game preparation failure.
pub async fn create_game_with_feature_mask<G: Game>(
    canvas: HtmlCanvasElement,
    generation: u64,
    game: &mut G,
    remove_features: wgpu::Features,
) -> Result<WebShell<G::World, G::Scene>, WebHostError>
where
    G::World: InlineWorld,
{
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
    descriptor.backends = wgpu::Backends::BROWSER_WEBGPU;
    let instance = wgpu::Instance::new(descriptor);
    let surface = Rc::new(
        instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .map_err(WebHostError::Surface)?,
    );
    let gpu = recovery::open_canvas_device(
        &instance,
        &surface,
        [canvas.width().max(1), canvas.height().max(1)],
        generation,
        remove_features,
    )
    .await?;
    configure_surface(&surface, &gpu.device, &gpu.config).map_err(WebHostError::Configure)?;
    let prepared = game
        .prepare(
            Gpu {
                device: &gpu.device,
                queue: &gpu.queue,
                capabilities: &gpu.capabilities,
                surface: SurfaceProfile {
                    format: gpu.config.format,
                    extent: [gpu.config.width, gpu.config.height],
                },
            },
            Box::new(|| {}),
        )
        .await
        .map_err(WebHostError::Prepare)?;
    let recovery =
        recovery::Recovery::new(instance, Rc::clone(&surface), generation, remove_features);
    Ok(WebShell {
        canvas,
        adapter_name: gpu.adapter_name,
        surface,
        recovery,
        config: gpu.config,
        device: gpu.device,
        graphics: gpu.fault,
        queue: gpu.queue,
        host: prepared.host,
        clock: PageClock::new(),
        callbacks: 0,
        presented: 0,
        unavailable: 0,
        lifecycle: WorldLifecyclePolicy::default(),
        queued_ticks: 0,
        needs_reconfigure: false,
        resize_prepare: None,
        samples: Vec::with_capacity(SAMPLES),
        profile: crate::profile::ProfileWindow::new(),
        profile_minimum: 0,
        profile_enabled: false,
        held_contribution: None,
        attach_error: None,
        governor: None,
        #[cfg(feature = "interface")]
        interface_output: None,
        #[cfg(feature = "interface")]
        reported_interface_output: None,
    })
}

impl<W: InlineWorld, V: SceneSource<W::Page>> WebShell<W, V> {
    /// Apply the game's pending control edits, including an explicit pause request.
    /// # Errors
    /// Returns a refused game edit or world lifecycle transition.
    pub fn apply_game_edits<G>(
        &mut self,
        game: &mut G,
        timestamp_ms: f64,
    ) -> Result<(), WebHostError>
    where
        G: Game<World = W, Scene = V>,
    {
        let (edits, paused) = game.take_edits();
        G::apply(&mut self.host, edits).map_err(WebHostError::Edit)?;
        if let Some(paused) = paused {
            self.set_paused(paused, timestamp_ms)?;
            game.observe_paused(self.paused());
        }
        Ok(())
    }

    /// Update the density and safe area observed by the canvas's page.
    #[cfg(feature = "interface")]
    pub fn set_interface_output(&mut self, density: f64, insets: [f64; 4]) {
        if density.is_finite() && density > 0.0 && insets.iter().all(|x| x.is_finite() && *x >= 0.0)
        {
            self.interface_output = Some((density, insets));
        }
    }

    /// Run the game's frame hooks and drive a browser animation callback.
    /// # Errors
    /// Returns a refused game edit or frame error.
    pub fn frame_game<G>(&mut self, game: &mut G, timestamp_ms: f64) -> Result<(), WebHostError>
    where
        G: Game<World = W, Scene = V>,
    {
        game.observe(G::readback(&self.host));
        #[cfg(feature = "interface")]
        let metrics = self
            .interface_output
            .filter(|_| game.wants_interface_output())
            .map(|(density, insets)| crate::OutputMetrics {
                extent: [self.canvas.width(), self.canvas.height()],
                density,
                insets,
            });
        #[cfg(feature = "interface")]
        let mut reported = self.reported_interface_output;
        #[cfg(feature = "interface")]
        let before = self.presented;
        #[cfg(feature = "interface")]
        let now = Duration::try_from_secs_f64(timestamp_ms / 1000.0).unwrap_or_default();
        #[cfg(feature = "interface")]
        game.interface_tick(now);
        let result = self.frame_with_edits(timestamp_ms, |host, replaced| {
            #[cfg(feature = "interface")]
            if let Some(metrics) = metrics
                && (replaced || reported != Some(metrics))
            {
                G::apply(host, game.interface_output(metrics)).map_err(WebHostError::Edit)?;
                reported = Some(metrics);
            }
            apply_frame_edits(game, host, replaced).map_err(WebHostError::Edit)
        });
        #[cfg(feature = "interface")]
        {
            self.reported_interface_output = reported;
            if self.presented != before {
                game.interface_submitted(&G::readback(&self.host), self.presented, now);
            }
        }
        result
    }

    /// Bind a declared profiler to this page. Report pages retain timing demand.
    pub fn update_profiler(
        &mut self,
        controls: &mut thrum_controls::Controls,
        writes_report: bool,
    ) {
        let Some(enabled) = crate::profile::enabled(controls) else {
            return;
        };
        if enabled != self.profile_enabled {
            self.profile_minimum = self.host.last_submission().map_or(0, |order| order + 1);
            self.profile_enabled = enabled;
        }
        if !self.clock.visible() || self.graphics.is_lost() {
            self.profile_minimum = self.host.last_submission().map_or(0, |order| order + 1);
        }
        self.host.set_timing_requested(writes_report || enabled);
        self.profile
            .set_available(self.clock.visible() && !self.graphics.is_lost());
        self.profile
            .configure(enabled, self.recovery.generation, self.profile_minimum);
        self.profile.publish(
            controls,
            self.clock.last(),
            self.callbacks,
            self.presented,
            true,
        );
    }

    /// Advance the world and, with a healthy device, encode before acquiring the canvas
    /// drawable, submit and present. After loss it only advances authority.
    /// # Errors
    /// Returns a timestamp, authoritative tick or GPU host fault.
    pub fn frame(&mut self, timestamp_ms: f64) -> Result<(), WebHostError> {
        self.frame_with_edits(timestamp_ms, |_, _| Ok(()))
    }

    fn frame_with_edits(
        &mut self,
        timestamp_ms: f64,
        apply: impl FnOnce(&mut FrameHost<W, V>, bool) -> Result<(), WebHostError>,
    ) -> Result<(), WebHostError> {
        let now = self
            .clock
            .now(timestamp_ms)
            .map_err(WebHostError::Timestamp)?;
        if !self.clock.visible() {
            return Ok(());
        }
        self.callbacks += 1;
        self.clock.mark(now);
        self.profile.configure(
            self.profile_enabled,
            self.recovery.generation,
            self.profile_minimum,
        );
        self.profile.callback(now);
        let extent = [self.canvas.width(), self.canvas.height()];
        let replacement = self.recovery.poll(&self.canvas, &mut self.host, extent);
        let replaced = replacement.is_some();
        if let Some((gpu, reconfigure)) = replacement {
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
                self.attach_error = Some(error.to_string());
            }
        }
        apply(&mut self.host, replaced)?;
        self.present_frame(now, extent)
    }

    fn present_frame(&mut self, now: Duration, extent: [u32; 2]) -> Result<(), WebHostError> {
        if self.graphics.is_lost() {
            self.recovery.awaiting_first_present = false;
            self.unavailable += 1;
            // No polling, completion collection, resize or GPU encoding after
            // loss. Authority and controls continue on the same resident world.
            let report = self
                .host
                .advance_without_rendering(now)
                .map_err(WebHostError::Frame)?;
            return self.observe_world(report, now);
        }
        let mut resize = extent != [self.config.width, self.config.height];
        if (resize || self.needs_reconfigure)
            && extent[0] > 0
            && extent[1] > 0
            && self.host.pending_frames() == 0
        {
            let start = js_sys::Date::now();
            if resize
                && !self
                    .host
                    .resize_scene_surface(extent)
                    .map_err(WebHostError::Frame)?
            {
                return Err(WebHostError::Frame(FrameError::InvalidState(
                    "canvas resize deferred with a free target ring",
                )));
            }
            self.config.width = extent[0];
            self.config.height = extent[1];
            configure_surface(&self.surface, &self.device, &self.config)
                .map_err(WebHostError::Configure)?;
            if resize {
                self.resize_prepare = Some(Duration::from_secs_f64(
                    (js_sys::Date::now() - start).max(0.0) / 1000.0,
                ));
            }
            self.needs_reconfigure = false;
            resize = false;
        }
        if let Some(cadence) = &mut self.governor {
            let before = cadence.callbacks.interval();
            if let Some(interval) = cadence.callbacks.observe(now) {
                if Some(interval) != before {
                    cadence.previous_present = None;
                }
                self.host.governor_display(
                    DisplayRecord {
                        interval,
                        thermal: ThermalState::Nominal,
                    },
                    resize || self.needs_reconfigure || self.recovery.is_pending(),
                );
            }
        }
        let advanced = self.host.advance(now).map_err(WebHostError::Frame)?;
        let can_acquire = advanced.can_acquire_serial()
            && extent[0] > 0
            && extent[1] > 0
            && !resize
            && !self.needs_reconfigure;
        let report = if can_acquire {
            let encoded = advanced.encode_scene().map_err(WebHostError::Frame)?;
            let frame = if encoded.can_acquire_surface() {
                match surface_action(self.surface.get_current_texture()) {
                    SurfaceAction::Draw { frame, reconfigure } => {
                        self.needs_reconfigure |= reconfigure;
                        Some(frame)
                    }
                    SurfaceAction::Reconfigure { .. } => {
                        self.needs_reconfigure = true;
                        self.unavailable += 1;
                        None
                    }
                    SurfaceAction::Skip { .. } => {
                        self.unavailable += 1;
                        None
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
                    .map_err(WebHostError::Frame)?;
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
            advanced.submit(None).map_err(WebHostError::Frame)?
        };
        if let Some(cadence) = &mut self.governor
            && report.surface_presented
            && !self
                .device
                .features()
                .contains(wgpu::Features::TIMESTAMP_QUERY)
            && let Some(order) = self.host.last_submission()
        {
            if self
                .host
                .governor_frame()
                .is_some_and(|frame| frame.mark.held())
            {
                cadence.previous_present = None;
            } else if let Some(previous) = cadence.previous_present.replace(now) {
                self.host
                    .governor_animation_frame(order, now.saturating_sub(previous));
            }
        }
        self.observe_world(report, now)?;
        self.collect_completed().map_err(WebHostError::Frame)
    }

    /// Collect GPU completions between callbacks without admitting time, and report
    /// whether a frame slot is free for the next encode. A caller pacing its own
    /// steps can drain completion callbacks before a step instead of dropping its
    /// view. Nothing is polled while the device is lost.
    /// # Errors
    /// Returns a device polling fault.
    pub fn poll_completed(&mut self) -> Result<bool, FrameError> {
        if self.graphics.is_lost() {
            return Ok(false);
        }
        self.collect_completed()?;
        Ok(self.host.has_free_slot())
    }

    fn collect_completed(&mut self) -> Result<(), FrameError> {
        while let Some(mut sample) = self.host.poll_completed()? {
            if self.samples.len() < SAMPLES || self.profile.is_enabled() {
                sample.detach_gpu_timing();
            }
            self.profile.push(&sample, self.clock.last());
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
    pub fn set_visible(&mut self, visible: bool, timestamp_ms: f64) -> Result<(), WebHostError> {
        if let Some(cadence) = &mut self.governor {
            cadence.callbacks.suspend();
            cadence.previous_present = None;
        }
        let now = self
            .clock
            .now(timestamp_ms)
            .map_err(WebHostError::Timestamp)?;
        self.set_play(visible, self.clock.user_paused(), now)
    }

    /// Stop admitting world time at the person's request. Visibility cannot override it.
    /// # Errors
    /// Returns a timestamp, retained-update or clock restart failure.
    pub fn set_paused(&mut self, paused: bool, timestamp_ms: f64) -> Result<(), WebHostError> {
        let now = self
            .clock
            .now(timestamp_ms)
            .map_err(WebHostError::Timestamp)?;
        self.set_play(self.clock.visible(), paused, now)
    }

    /// Whether the person paused the world, independent of page visibility.
    pub fn paused(&self) -> bool {
        self.clock.user_paused()
    }

    /// Persistent device-loss diagnostic, or `None` while the device is healthy.
    pub fn graphics_fault(&self) -> Option<String> {
        self.graphics.loss_message().map(str::to_owned)
    }

    /// Whether the current device is lost. GPU-touching game work checks this first.
    pub fn graphics_lost(&self) -> bool {
        self.graphics.is_lost()
    }

    /// Start detached device replacement without borrowing the running world.
    /// # Errors
    /// Refuses a healthy device, a duplicate attempt or invalid retained inputs.
    pub fn request_graphics_recovery(&mut self) -> Result<(), WebHostError> {
        if !self.graphics.is_lost() {
            return Err(WebHostError::DeviceAvailable);
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

    /// Enable dynamic quality driven by the page's animation cadence.
    /// Returns false if the prepared scene has no supported render-scale steps.
    pub fn enable_governor(&mut self) -> bool {
        let admitted = self.host.enable_governor(DisplayRecord {
            interval: Duration::ZERO,
            thermal: ThermalState::Nominal,
        });
        if admitted {
            self.governor = Some(BrowserCadence::default());
        }
        admitted
    }

    /// Stage a scene quality knob for the next frame boundary.
    /// # Errors
    /// Refuses an unknown knob or a value outside its declared range.
    pub fn set_quality_knob(
        &mut self,
        name: &str,
        value: u32,
    ) -> Result<(), thrum_render::contribution::QualityError> {
        self.host.set_quality_knob(name, value)
    }

    /// Set the drawn subrectangle of the next frame.
    ///
    /// # Errors
    /// Reduced scale requires portable temporal reconstruction.
    pub fn set_render_scale(
        &mut self,
        scale: thrum_render::synthetic::RenderScale,
    ) -> Result<(), FrameError> {
        self.host.set_render_scale(scale)
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

    /// Edit a continuous presentation parameter while retaining correspondence.
    /// See [`FrameHost::continuous_source_mut`]; cuts still use [`Self::source_mut`].
    pub fn continuous_source_mut(&mut self) -> &mut V {
        self.host.continuous_source_mut()
    }

    /// Attach a scene contribution. While the device is lost it is held, with no GPU call,
    /// and attached right after the replacement device is published; a later attach
    /// replaces a held one, and a refusal then is reported by [`Self::take_attach_error`].
    /// # Errors
    /// Returns the runtime's refusal of an immediate attach.
    pub fn set_scene_contribution(
        &mut self,
        contribution: Box<dyn SceneContribution>,
    ) -> Result<(), FrameError> {
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
    pub fn begin_capture(&mut self) -> Result<PendingCapture, FrameError> {
        if self.graphics.is_lost() {
            return Err(FrameError::DeviceLost("graphics device is lost".to_owned()));
        }
        self.host.begin_capture()
    }

    /// The capture's sRGB RGBA8 pixels once ready. A capture outstanding at loss is refused.
    /// # Errors
    /// Returns a lost device, a retired device generation or a readback failure.
    pub fn try_finish_capture(
        &mut self,
        pending: &mut PendingCapture,
    ) -> Result<Option<Vec<u8>>, FrameError> {
        if self.graphics.is_lost() {
            return Err(FrameError::DeviceLost(
                "graphics device was lost before the capture finished".to_owned(),
            ));
        }
        self.host.try_finish_capture(pending)
    }

    /// Counters and state for the game's report.
    pub fn status(&self) -> WebStatus {
        WebStatus {
            adapter_name: self.adapter_name.clone(),
            format: self.config.format,
            extent: [self.config.width, self.config.height],
            resize_prepare: self.resize_prepare,
            callbacks: self.callbacks,
            presented: self.presented,
            unavailable: self.unavailable,
            overload_events: self.lifecycle.overload_count(),
            refused_ticks: self.lifecycle.refused_ticks(),
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

    /// Release samples already copied by a caller that streams a longer report.
    pub fn discard_report_samples_before(&mut self, tick: u64) {
        self.samples.retain(|sample| sample.tick >= tick);
    }

    fn set_play(
        &mut self,
        visible: bool,
        user_paused: bool,
        now: Duration,
    ) -> Result<(), WebHostError> {
        self.lifecycle.request(!visible || user_paused);
        self.reconcile_world(now)?;
        self.clock.set_play(visible, user_paused, now);
        Ok(())
    }

    fn observe_world(
        &mut self,
        report: thrum_render::runtime::DriveReport,
        now: Duration,
    ) -> Result<(), WebHostError> {
        self.queued_ticks = report.queued_ticks;
        if let Some(refused) = report.overload {
            self.lifecycle.overloaded(refused);
        }
        self.reconcile_world(now)
    }

    fn reconcile_world(&mut self, now: Duration) -> Result<(), WebHostError> {
        let world = self.host.world_mut();
        self.lifecycle
            .drive_inline(self.queued_ticks, |request| match request {
                WorldLifecycle::Pause => world.pause(),
                WorldLifecycle::Resume => world.resume(now),
            })
            .map_err(WebHostError::Lifecycle)
    }
}

/// Translate a canvas contact to a first-chance interface event.
///
/// # Errors
/// Refuses an unknown pointer source or phase, or an out-of-range mouse button.
#[cfg(feature = "interface")]
pub fn interface_pointer(
    source: &str,
    phase: &str,
    id: u32,
    point: [f64; 2],
) -> Result<thrum_ui::Event<crate::KeyCode>, GameError> {
    use thrum_ui::{Contact, Event};
    let contact = match source {
        "mouse" => Contact::Pointer(
            u16::try_from(id).map_err(|error| GameError::Rejected(error.to_string()))?,
        ),
        "touch" | "pen" => Contact::Touch(u64::from(id)),
        _ => {
            return Err(GameError::Rejected(
                "unknown interface pointer source".into(),
            ));
        }
    };
    Ok(match phase {
        "down" => Event::Press {
            contact,
            position: Some(point),
            intent: None,
        },
        "up" => Event::Release(contact, Some(point)),
        "move" => Event::Move(contact, point),
        "hover" => Event::Hover(point),
        "cancel" => Event::Cancel(contact),
        _ => {
            return Err(GameError::Rejected(
                "unknown interface pointer phase".into(),
            ));
        }
    })
}
