//! V1 scene/rules facade over the engine-owned bounded frame runtime.

use crate::{
    environment::DayCycle,
    projection::{GameProjection, ProjectionGap},
    scene::{GameContent, RouteOverlay},
};
pub use runtime::{DriveReport, FrameConfig, FrameSample, SurfaceOutput};
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(not(target_arch = "wasm32"))]
use thrum_content_runtime::EnvironmentProduct;
use thrum_render::{
    capabilities::DeviceCapabilities,
    runtime,
    synthetic::{CullMode, DisplayOverlay},
};

mod scene_source;
mod world;
pub(crate) use scene_source::GameScene;
pub use scene_source::field_in_view_share;
use world::WorldDriver;
pub(crate) use world::WorldPort;
#[cfg(target_arch = "wasm32")]
pub(crate) mod web;
#[cfg(not(target_arch = "wasm32"))]
mod world_thread;
#[cfg(not(target_arch = "wasm32"))]
use world_thread::WorldClient;
#[cfg(not(target_arch = "wasm32"))]
pub use world_thread::WorldControls;

pub(crate) type Runtime = runtime::FrameHost<WorldPort, GameScene>;

/// Content and camera culling chosen when the host is prepared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameSetup {
    /// Opt-in analytic Sun-path optical depth, a diagnostic cloud-shade stand-in.
    pub diagnostic_clouds: bool,
    /// The grid or the dense flythrough field.
    pub content: GameContent,
    /// How the camera's opaque draw is culled; Direct draws every instance.
    pub camera_culling: CullMode,
    /// Prepare the authored valley's star field. Ignored for the flythrough field.
    pub valley_stars: bool,
    /// Reseed the moving-route proof overlay (11 static grid slots at valley metres,
    /// one mirrored basis and one retuned local light). Grid content only; off by default.
    pub route_overlay: RouteOverlay,
    /// Draw the flythrough field's props as a rock, a column and a spruce,
    /// some mirrored, instead of boxes. Ignored for the grid; refused with
    /// Occlusion, which keeps the cube.
    pub field_meshes: bool,
}

impl Default for GameSetup {
    fn default() -> Self {
        Self {
            diagnostic_clouds: false,
            content: GameContent::Grid,
            camera_culling: CullMode::Direct,
            valley_stars: false,
            route_overlay: RouteOverlay::Off,
            field_meshes: false,
        }
    }
}

/// Prepared V1 game, whose world clock and frame executor belong to the engine.
pub struct FrameHost {
    inner: Runtime,
    #[cfg(not(target_arch = "wasm32"))]
    visual_product: Option<EnvironmentProduct>,
}

/// One admitted callback, preserving the engine's exclusive frame token.
pub struct AdvancedFrame<'a>(runtime::AdvancedFrame<'a, WorldPort, GameScene>);
impl<'a> AdvancedFrame<'a> {
    /// Whether the engine can record a scene before drawable acquisition.
    pub fn can_acquire_serial(&self) -> bool {
        self.0.can_acquire_serial()
    }
    /// Submit the optional presentation of the already admitted world callback.
    /// # Errors
    /// Returns an engine frame failure without undoing world authority.
    pub fn submit(
        self,
        surface: Option<&SurfaceOutput<'_>>,
    ) -> Result<DriveReport, runtime::FrameError> {
        self.0.submit(surface)
    }
    /// Record the scene before the platform acquires its drawable.
    /// # Errors
    /// Returns an engine extraction or recording failure.
    pub fn encode_scene(self) -> Result<EncodedFrame<'a>, runtime::FrameError> {
        self.0.encode_scene().map(EncodedFrame)
    }
}
/// One unsubmitted engine scene token, held across late drawable acquisition.
pub struct EncodedFrame<'a>(runtime::EncodedFrame<'a, WorldPort, GameScene>);
impl EncodedFrame<'_> {
    /// Whether the engine recorded a view that can use a drawable.
    pub fn can_acquire_surface(&self) -> bool {
        self.0.can_acquire_surface()
    }

    /// Actual resources and matrix of this encoded frame.
    pub fn frame_extent(&self) -> Option<runtime::FrameExtent> {
        self.0.frame_extent()
    }

    /// Skip a view while retaining its committed world update.
    pub fn skip(self) -> DriveReport {
        self.0.skip()
    }
    /// Discard the view after the platform observes loss of its device,
    /// without the empty queue submit a healthy skip makes.
    pub fn abandon_lost_device(self) {
        self.0.abandon_lost_device();
    }
    /// Append the optional drawable consumer and submit through the engine.
    /// # Errors
    /// Returns an engine submission failure.
    pub fn submit(
        self,
        surface: Option<&SurfaceOutput<'_>>,
    ) -> Result<DriveReport, runtime::FrameError> {
        self.0.submit(surface)
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// Prepared game product paired with engine-owned replacement resources.
pub struct PreparedEnvironment {
    product: Option<EnvironmentProduct>,
    inner: runtime::PreparedVisual,
}
#[cfg(not(target_arch = "wasm32"))]
/// Detached engine preparation inputs for a cooked game environment.
pub struct VisualPrepareInputs(runtime::VisualPrepareInputs);
#[cfg(not(target_arch = "wasm32"))]
impl VisualPrepareInputs {
    /// Prepare the cooked photometric view without borrowing the active game.
    /// # Errors
    /// Returns a GPU preparation failure.
    pub async fn prepare(self, product: EnvironmentProduct) -> Result<PreparedEnvironment, String> {
        let revision = Some(*product.revision().as_bytes());
        let lighting = thrum_render::synthetic::FrameLighting::Photometric(
            scene_source::photometric(&product),
        );
        let inner = self
            .0
            .prepare(lighting, 1.0, revision)
            .await
            .map_err(|error| error.to_string())?;
        Ok(PreparedEnvironment {
            product: Some(product),
            inner,
        })
    }
}

/// Native frame view after its authoritative world has moved to an autonomous
/// lane. This exposes presentation operations without exposing inline world
/// and projection methods that cannot run on the render owner.
#[cfg(not(target_arch = "wasm32"))]
pub struct ThreadedFrameHost {
    inner: FrameHost,
}

#[cfg(not(target_arch = "wasm32"))]
impl ThreadedFrameHost {
    /// Snapshot GPU reconstruction inputs without moving the resident world.
    /// # Errors
    /// Returns a failure while sampling the current camera or lighting policy.
    pub fn device_prepare_inputs(
        &self,
    ) -> Result<runtime::DevicePrepareInputs, runtime::FrameError> {
        self.inner.inner.device_prepare_inputs()
    }

    /// Install a host-bound replacement without polling the lost device.
    /// # Errors
    /// Refuses stale, consumed or foreign reconstruction resources.
    pub fn publish_device(
        &mut self,
        candidate: &mut runtime::PreparedDevice,
    ) -> Result<(), runtime::FrameError> {
        self.inner.inner.publish_device(candidate)
    }

    /// Set the next display target without advancing the fixed-step world.
    pub fn set_presentation_lead(&mut self, lead: Duration) {
        self.inner.set_presentation_lead(lead);
    }

    /// Select the game's visual time of day without changing simulation time.
    /// # Errors
    /// Refuses a fraction outside the normalized day.
    pub fn set_time_of_day(&mut self, fraction: f32) -> Result<(), String> {
        self.inner.set_time_of_day(fraction)
    }

    /// Committed solar time of the world lane's latest tick: midnight zero, noon one half.
    pub fn time_of_day(&self) -> f32 {
        self.inner.time_of_day()
    }

    /// The controlled actor's committed position in the world lane's latest published page.
    pub fn controlled_position(&self) -> Option<[f64; 3]> {
        self.inner.controlled_position()
    }

    /// Change the game camera elevation and reset GPU history on the next draw.
    /// # Errors
    /// Refuses an unsupported or nonfinite pitch.
    pub fn set_camera_pitch(&mut self, radians: f32) -> Result<(), String> {
        self.inner.set_camera_pitch(radians)
    }

    /// Turn the game camera and reset GPU history on the next draw.
    /// # Errors
    /// Refuses a nonfinite yaw or one outside ±π radians.
    pub fn set_camera_yaw(&mut self, radians: f32) -> Result<(), String> {
        self.inner.set_camera_yaw(radians)
    }

    /// Draw a display diagnostic, such as exposure zones, in place of the tone-mapped scene.
    pub fn set_display_overlay(&mut self, overlay: DisplayOverlay) {
        self.inner.set_display_overlay(overlay);
    }

    /// Render the latest immutable world publication.
    /// # Errors
    /// Returns a render or authoritative-world fault.
    pub fn drive(
        &mut self,
        now: Duration,
        surface: Option<&SurfaceOutput<'_>>,
    ) -> Result<DriveReport, runtime::FrameError> {
        self.inner.inner.drive(now, surface)
    }

    /// As [`FrameHost::set_scene_contribution`].
    /// # Errors
    /// The contribution's refusal to draw into the current scene.
    pub fn set_scene_contribution(
        &mut self,
        contribution: Box<dyn thrum_render::contribution::SceneContribution>,
    ) -> Result<(), String> {
        self.inner.set_scene_contribution(contribution)
    }

    /// Admit the world observation before scene encoding and late drawable
    /// acquisition on the render owner.
    /// # Errors
    /// Returns a clock, publication or GPU fault without rolling back a tick.
    pub fn advance(&mut self, now: Duration) -> Result<AdvancedFrame<'_>, runtime::FrameError> {
        self.inner.advance(now)
    }

    /// Whether a serial render view can use an available GPU slot.
    pub fn has_serial_view(&self) -> bool {
        self.inner.has_serial_view()
    }

    /// Whether the last committed page can be resampled.
    pub fn has_current_scene_for_display(&self) -> bool {
        self.inner.has_current_scene_for_display()
    }

    /// Number of GPU frames still in flight.
    pub fn pending_frames(&self) -> usize {
        self.inner.pending_frames()
    }

    /// Poll one completed GPU sample.
    /// # Errors
    /// Returns a device fault.
    pub fn poll_completed(&mut self) -> Result<Option<FrameSample>, runtime::FrameError> {
        self.inner.inner.poll_completed()
    }

    /// Enable the renderer tap with finite allocation limits.
    /// # Errors
    /// Refuses an active session or invalid limits.
    pub fn enable_submitted_capture(
        &mut self,
        session: u64,
        limits: runtime::CaptureLimits,
    ) -> Result<(), String> {
        self.inner
            .inner
            .enable_submitted_capture_with_limits(session, limits)
            .map_err(|e| e.to_string())
    }

    /// Collect a submitted image without advancing the world.
    /// # Errors
    /// Refuses mapping failure or retired generation.
    pub fn try_submitted_image(&mut self) -> Result<Option<runtime::SubmittedImage>, String> {
        self.inner.try_submitted_image()
    }

    /// Return a CPU lease to its generation-bound pool.
    pub fn recycle_submitted_image(&mut self, image: runtime::SubmittedImage) {
        self.inner.recycle_submitted_image(image);
    }

    /// Stop new copies while pending images drain.
    pub fn stop_submitted_capture(&mut self) {
        self.inner.inner.stop_submitted_capture();
    }

    /// Cancel capture without submitting or waiting for GPU work.
    pub fn disable_submitted_capture(&mut self) {
        self.inner.disable_submitted_capture();
    }

    /// Samples omitted by the renderer's finite ring.
    pub fn submitted_capture_omissions(&self) -> u64 {
        self.inner.inner.submitted_capture_omissions()
    }

    /// Current staging and CPU leases.
    pub fn submitted_capture_usage(&self) -> Option<(usize, usize)> {
        self.inner.inner.submitted_capture_usage()
    }

    /// Resize the surface binding after a coordinated platform change.
    /// # Errors
    /// Refuses an invalid or unavailable surface.
    pub fn resize_surface(&mut self, extent: [u32; 2]) -> Result<(), runtime::FrameError> {
        self.inner.inner.resize_surface(extent)
    }

    /// Resize the scene grid and presenter coherently on the render owner.
    /// # Errors
    /// Returns a preparation fault. Zero extents or busy slots defer.
    pub fn resize_scene_surface(&mut self, extent: [u32; 2]) -> Result<bool, runtime::FrameError> {
        self.inner.inner.resize_scene_surface(extent)
    }

    /// Resize with a final platform lease/epoch validity check.
    /// # Errors
    /// Returns a preparation fault. A stale epoch defers publication.
    pub fn resize_scene_surface_if(
        &mut self,
        extent: [u32; 2],
        current: impl FnOnce() -> bool,
    ) -> Result<bool, runtime::FrameError> {
        self.inner.inner.resize_scene_surface_if(extent, current)
    }

    /// Optional scene views skipped.
    pub fn dropped_views(&self) -> u64 {
        self.inner.dropped_views()
    }

    /// Completed views dropped after source expiry.
    pub fn dropped_stale_views(&self) -> u64 {
        self.inner.dropped_stale_views()
    }

    /// Surface views dropped.
    pub fn dropped_surface(&self) -> u64 {
        self.inner.dropped_surface()
    }

    /// Completed profiling rows dropped.
    pub fn dropped_reports(&self) -> u64 {
        self.inner.dropped_reports()
    }

    /// The engine runtime, for a platform shell that drives it directly.
    #[cfg(target_os = "macos")]
    pub(crate) fn into_runtime(self) -> Runtime {
        self.inner.inner
    }
}

impl FrameHost {
    /// Inspect the renderer's prepared variant storage and resident texture bytes.
    #[must_use]
    pub fn storage_report(&self) -> String {
        self.inner.storage_report()
    }

    /// Maximum submitted scene packets retained by the engine.
    pub const MAX_IN_FLIGHT: usize = Runtime::MAX_IN_FLIGHT;
    /// Initial generation for simple callers; live platforms own later generations.
    pub const DEVICE_GENERATION: u64 = 1;

    /// Retain CPU presentation inputs for detached GPU reconstruction.
    /// # Errors
    /// Returns a current camera or lighting policy failure.
    pub fn device_prepare_inputs(
        &self,
    ) -> Result<runtime::DevicePrepareInputs, runtime::FrameError> {
        self.inner.device_prepare_inputs()
    }

    /// Install complete GPU resources while keeping this game's resident world.
    /// # Errors
    /// Refuses a stale, foreign or consumed device candidate.
    pub fn publish_device(
        &mut self,
        candidate: &mut runtime::PreparedDevice,
    ) -> Result<(), runtime::FrameError> {
        self.inner.publish_device(candidate)
    }

    /// Prepare the fixed V1 scene using a platform-owned device and queue.
    /// # Errors
    /// Returns a scene, world or engine preparation failure.
    pub async fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: FrameConfig,
    ) -> Result<Self, String> {
        let capabilities = DeviceCapabilities::capture_device(device, Self::DEVICE_GENERATION);
        Self::new_with_capabilities(device, queue, &capabilities, config).await
    }
    /// Prepare from the platform's immutable capability record.
    /// # Errors
    /// Refuses a mismatched device or game/engine preparation failure.
    pub async fn new_with_capabilities(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        capabilities: &DeviceCapabilities,
        config: FrameConfig,
    ) -> Result<Self, String> {
        Self::new_with_setup(device, queue, capabilities, config, GameSetup::default()).await
    }
    /// Prepare the selected content and camera culling mode.
    /// # Errors
    /// Refuses a mismatched device or game/engine preparation failure.
    pub async fn new_with_setup(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        capabilities: &DeviceCapabilities,
        config: FrameConfig,
        setup: GameSetup,
    ) -> Result<Self, String> {
        if !capabilities.matches(device) {
            return Err("capability record does not match the host device".to_owned());
        }
        let scene = GameScene::scene(config.render_extent, setup)?;
        let world = WorldPort::Inline(Box::new(WorldDriver::new(
            setup.content,
            setup.route_overlay,
        )?));
        let inner = Runtime::new(
            capabilities,
            queue,
            config,
            world,
            GameScene::new(setup),
            scene,
            1.0 / 16_000.0,
        )
        .await
        .map_err(|error| error.to_string())?;
        Ok(Self {
            inner,
            #[cfg(not(target_arch = "wasm32"))]
            visual_product: None,
        })
    }
    /// Move the existing clock/rules into the engine's resident world lane.
    /// # Errors
    /// Returns world-lane startup or initial-publication failure.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn into_threaded(
        mut self,
        origin: Instant,
        wake: impl Fn() + Send + 'static,
    ) -> Result<(ThreadedFrameHost, WorldControls), String> {
        let WorldPort::Inline(driver) =
            std::mem::replace(self.inner.world_mut(), WorldPort::Starting)
        else {
            return Err("world is already threaded".to_owned());
        };
        let (client, controls) = WorldClient::start(*driver, origin, wake)?;
        *self.inner.world_mut() = WorldPort::Threaded(Box::new(client));
        Ok((ThreadedFrameHost { inner: self }, controls))
    }
    /// Select the platform's next display target without advancing authority.
    pub fn set_presentation_lead(&mut self, lead: Duration) {
        self.inner.set_presentation_lead(lead);
    }
    /// Change the game-authored day phase and reset presentation history.
    /// # Errors
    /// Refuses a nonfinite or out-of-range normalized time.
    pub fn set_time_of_day(&mut self, fraction: f32) -> Result<(), String> {
        set_time_of_day(&mut self.inner, fraction)
    }
    /// Current committed solar time: midnight is zero, noon is one half.
    /// This observes the world's clock; display interpolation may lag one tick.
    pub fn time_of_day(&self) -> f32 {
        time_of_day(&self.inner)
    }
    /// Solar time in the current tick's display interval at the submitted frame's alpha.
    /// Read it before advancing the host or editing the solar phase.
    pub fn rendered_time_of_day(&self, alpha: f32) -> f32 {
        DayCycle::rendered_time_of_day(self.tick(), alpha, self.inner.source().day_phase_offset)
    }

    /// Change the game camera elevation and reset presentation history.
    /// # Errors
    /// Refuses a nonfinite pitch outside the supported scene range.
    pub fn set_camera_pitch(&mut self, radians: f32) -> Result<(), String> {
        set_camera_pitch(&mut self.inner, radians)
    }

    /// Turn the game camera and reset presentation history.
    /// # Errors
    /// Refuses a nonfinite yaw or one outside ±π radians.
    pub fn set_camera_yaw(&mut self, radians: f32) -> Result<(), String> {
        set_camera_yaw(&mut self.inner, radians)
    }

    /// Draw a display diagnostic, such as exposure zones, in place of the tone-mapped scene.
    pub fn set_display_overlay(&mut self, overlay: DisplayOverlay) {
        set_display_overlay(&mut self.inner, overlay);
    }
    /// Inspect whether a native scene view can use a free engine slot.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn has_serial_view(&self) -> bool {
        self.inner.has_serial_view()
    }
    /// Whether the retained committed tick pair can be sampled for display.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn has_current_scene_for_display(&self) -> bool {
        self.inner.has_current_scene_for_display()
    }
    /// Prepare detached cooked visual resources using the engine executor.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn visual_prepare_inputs(&self) -> VisualPrepareInputs {
        VisualPrepareInputs(self.inner.visual_prepare_inputs())
    }
    /// Install a host-bound candidate after old GPU packets retire.
    /// # Errors
    /// Returns an engine ownership/binding/upload failure.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn publish_environment(
        &mut self,
        candidate: &mut PreparedEnvironment,
    ) -> Result<bool, String> {
        let published = self
            .inner
            .publish_visual(&mut candidate.inner)
            .map_err(|error| error.to_string())?;
        if published {
            self.visual_product = candidate.product.take();
        }
        Ok(published)
    }
    /// Delay before the next inline world deadline.
    pub fn next_due_in(&self) -> Option<Duration> {
        self.inner.world().next_due_in()
    }
    /// Stop inline wall-time admission, retaining accepted ticks.
    /// Native threaded transitions use `WorldControls` requests and replies.
    /// # Errors
    /// Refuses a threaded world instead of presenting enqueueing as completion.
    pub fn pause(&mut self) -> Result<(), String> {
        self.inner.world_mut().pause()
    }
    /// Resume the drained inline world from a new monotonic origin.
    /// # Errors
    /// Returns a clock state/timestamp failure, or refuses a threaded world.
    pub fn resume(&mut self, now: Duration) -> Result<(), String> {
        self.inner.world_mut().resume(now)
    }
    /// Resume when accepted ticks have drained on ordinary drives.
    /// A queued resume keeps hidden elapsed time excluded until the drain ends.
    /// # Errors
    /// Refuses a threaded or already-active world, or a backwards timestamp.
    pub fn resume_after_drain(&mut self, now: Duration) -> Result<(), String> {
        self.inner
            .world_mut()
            .inline_lifecycle()?
            .resume_after_drain(now)
    }
    /// Latest committed authority tick.
    pub fn tick(&self) -> u64 {
        self.inner.world().tick()
    }
    /// Validate and replace continuous local input.
    /// # Errors
    /// Refuses invalid controls or a stopped owner.
    pub fn set_controls(&mut self, axes: [f32; 3]) -> Result<(), String> {
        self.inner.source().route_guard()?;
        self.inner.world_mut().set_controls(axes)
    }
    /// Position of the game fixture's controlled actor.
    pub fn controlled_position(&self) -> Option<[f64; 3]> {
        self.inner.world().controlled_position()
    }
    /// Enable the game's optional detached projection outbox.
    /// # Errors
    /// Returns a world publication failure.
    pub fn enable_projection(&mut self) -> Result<(), String> {
        self.inner.world_mut().enable_projection()
    }
    /// Consume game projection output in committed order.
    /// # Errors
    /// Reports an explicit overwritten-tick gap requiring a baseline.
    pub fn poll_projection(
        &mut self,
        consume: impl FnOnce(&GameProjection),
    ) -> Result<Option<bool>, ProjectionGap> {
        self.inner.world_mut().poll_projection(consume)
    }
    /// Reserved bytes in the game's detached projection outbox.
    pub fn projection_reserved_bytes(&self) -> usize {
        self.inner.world().projection_reserved_bytes()
    }
    /// Replace projection deltas with a fresh game baseline.
    /// # Errors
    /// Returns a disabled-output or publication failure.
    pub fn reset_projection_baseline(&mut self) -> Result<(), String> {
        self.inner.world_mut().reset_projection_baseline()
    }
    /// Draw an engine contribution (for example cooked G5 products) inside the scene's forward
    /// pass with the scene's selected lighting. It is rebuilt across visual and device
    /// republication.
    /// # Errors
    /// The contribution's refusal to draw into the current scene.
    pub fn set_scene_contribution(
        &mut self,
        contribution: Box<dyn thrum_render::contribution::SceneContribution>,
    ) -> Result<(), String> {
        self.inner.set_scene_contribution(contribution)
    }
    /// Drive authority and optional presentation through the engine.
    /// # Errors
    /// Returns a world, presentation or GPU fault.
    pub fn drive(
        &mut self,
        now: Duration,
        surface: Option<&SurfaceOutput<'_>>,
    ) -> Result<DriveReport, String> {
        self.inner
            .drive(now, surface)
            .map_err(|error| error.to_string())
    }
    /// Admit or observe the world before late surface acquisition.
    /// # Errors
    /// Returns a world or GPU polling failure.
    pub fn advance(&mut self, now: Duration) -> Result<AdvancedFrame<'_>, runtime::FrameError> {
        self.inner.advance(now).map(AdvancedFrame)
    }
    /// Advance the same world without polling, preparing or submitting GPU work.
    /// # Errors
    /// Returns an authoritative world or clock failure.
    pub fn advance_without_rendering(&mut self, now: Duration) -> Result<DriveReport, String> {
        self.inner
            .advance_without_rendering(now)
            .map_err(|error| error.to_string())
    }
    /// Submitted GPU packets not yet fully retired.
    pub fn pending_frames(&self) -> usize {
        self.inner.pending_frames()
    }
    /// The camera culling mode the prepared renderer runs.
    pub fn camera_culling(&self) -> CullMode {
        self.inner.camera_culling()
    }
    /// Optional scene views shed by the engine.
    pub fn dropped_views(&self) -> u64 {
        self.inner.dropped_views()
    }
    /// Stale source views rejected by the engine.
    pub fn dropped_stale_views(&self) -> u64 {
        self.inner.dropped_stale_views()
    }
    /// Undrained profiler rows discarded by the engine.
    pub fn dropped_reports(&self) -> u64 {
        self.inner.dropped_reports()
    }
    /// Invalid presentation rows omitted during explicit capture.
    pub fn dropped_capture_rows(&self) -> u64 {
        self.inner.dropped_capture_rows()
    }
    /// Surface presentations omitted by the engine.
    pub fn dropped_surface(&self) -> u64 {
        self.inner.dropped_surface()
    }
    /// Resize prepared surface bindings after coordinated platform configuration.
    /// # Errors
    /// Returns a prepared-surface extent or device failure.
    pub fn resize_surface(&mut self, extent: [u32; 2]) -> Result<(), String> {
        self.inner
            .resize_surface(extent)
            .map_err(|error| error.to_string())
    }
    /// Drain one completed engine sample in submission order.
    /// # Errors
    /// Returns a GPU polling failure.
    pub fn poll_completed(&mut self) -> Result<Option<FrameSample>, String> {
        self.inner
            .poll_completed()
            .map_err(|error| error.to_string())
    }
    /// Enable the renderer-owned tap for ordinary submitted sequence frames.
    /// # Errors
    /// Refuses an already active capture session.
    pub fn enable_submitted_capture(&mut self, session: u64) -> Result<(), String> {
        self.inner
            .enable_submitted_capture(session)
            .map_err(|error| error.to_string())
    }
    /// Enable a declared engine stage through the same submitted-image tap.
    /// # Errors
    /// Refuses active capture, pending frames during a stage change or preparation failure.
    pub fn enable_submitted_capture_from(
        &mut self,
        session: u64,
        source: thrum_render::synthetic::CaptureSource,
    ) -> Result<(), String> {
        self.inner
            .enable_submitted_capture_from(session, runtime::CaptureLimits::default(), source)
            .map_err(|error| error.to_string())
    }

    /// Collect an ordinary submitted frame without redrawing an endpoint.
    /// # Errors
    /// Returns mapping or retired-generation failure.
    pub fn try_submitted_image(&mut self) -> Result<Option<runtime::SubmittedImage>, String> {
        self.inner
            .try_submitted_image()
            .map_err(|error| error.to_string())
    }
    /// Return the sequence writer's CPU image lease.
    pub fn recycle_submitted_image(&mut self, image: runtime::SubmittedImage) {
        self.inner.recycle_submitted_image(image);
    }
    /// End the sequence observer without waiting for GPU work.
    pub fn disable_submitted_capture(&mut self) {
        self.inner.disable_submitted_capture();
    }
    /// Capture the latest committed endpoint for untimed visual inspection.
    /// # Errors
    /// Returns an engine recording, completion or readback failure.
    pub fn capture_pixels(&mut self) -> Result<Vec<u8>, String> {
        self.inner
            .capture_pixels()
            .map_err(|error| error.to_string())
    }
    /// Submit a capture of the latest committed endpoint without waiting; the
    /// browser finishes it from a later animation frame.
    /// # Errors
    /// Returns an engine recording or submission failure.
    pub fn begin_capture(&mut self) -> Result<runtime::PendingCapture, String> {
        self.inner
            .begin_capture()
            .map_err(|error| error.to_string())
    }
    /// Nonblocking: the capture's RGBA pixels once its readback completes.
    /// # Errors
    /// Returns a retired device generation or readback failure.
    pub fn try_finish_capture(
        &mut self,
        pending: &mut runtime::PendingCapture,
    ) -> Result<Option<Vec<u8>>, String> {
        self.inner
            .try_finish_capture(pending)
            .map_err(|error| error.to_string())
    }
}

/// Change the game-authored day phase and reset presentation history.
pub(crate) fn set_time_of_day(runtime: &mut Runtime, fraction: f32) -> Result<(), String> {
    if let Some(offset) = day_phase_edit(runtime.world().tick(), runtime.source(), fraction)? {
        runtime.source_mut().day_phase_offset = offset;
        runtime.invalidate_history();
    }
    Ok(())
}

/// Committed solar time of the runtime's latest world tick.
pub(crate) fn time_of_day(runtime: &Runtime) -> f32 {
    DayCycle::time_of_day(runtime.world().tick(), runtime.source().day_phase_offset)
}

/// Change the game camera elevation and reset presentation history.
pub(crate) fn set_camera_pitch(runtime: &mut Runtime, radians: f32) -> Result<(), String> {
    if let Some(pitch) = camera_pitch_edit(runtime.source(), radians)? {
        runtime.source_mut().camera_pitch = pitch;
        runtime.invalidate_history();
    }
    Ok(())
}

/// Turn the game camera and reset presentation history.
pub(crate) fn set_camera_yaw(runtime: &mut Runtime, radians: f32) -> Result<(), String> {
    if let Some(yaw) = camera_yaw_edit(runtime.source(), radians)? {
        runtime.source_mut().camera_yaw = yaw;
        runtime.invalidate_history();
    }
    Ok(())
}

/// Select the display diagnostic. It replaces only the display encode, so history stays valid.
pub(crate) fn set_display_overlay(runtime: &mut Runtime, overlay: DisplayOverlay) {
    runtime.source_mut().display_overlay = overlay;
}

/// The day phase offset that shows `fraction` at `tick`, or `None` when it is already shown.
fn day_phase_edit(tick: u64, scene: &GameScene, fraction: f32) -> Result<Option<f32>, String> {
    scene.route_guard()?;
    let offset = DayCycle::offset_for_time(tick, fraction)
        .ok_or_else(|| "time of day must be finite and in [0, 1]".to_owned())?;
    Ok((scene.day_phase_offset.to_bits() != offset.to_bits()).then_some(offset))
}

/// A checked camera pitch, or `None` when it is already selected.
fn camera_pitch_edit(scene: &GameScene, radians: f32) -> Result<Option<f32>, String> {
    scene.route_guard()?;
    if !radians.is_finite() || !(-0.6..=0.6).contains(&radians) {
        return Err("camera pitch must be finite and within ±0.6 rad".to_owned());
    }
    Ok((scene.camera_pitch.to_bits() != radians.to_bits()).then_some(radians))
}

/// A checked camera yaw, or `None` when it is already selected.
fn camera_yaw_edit(scene: &GameScene, radians: f32) -> Result<Option<f32>, String> {
    scene.route_guard()?;
    if !radians.is_finite() || !(-std::f32::consts::PI..=std::f32::consts::PI).contains(&radians) {
        return Err("camera yaw must be finite and within ±π rad".to_owned());
    }
    Ok((scene.camera_yaw.to_bits() != radians.to_bits()).then_some(radians))
}

#[cfg(test)]
mod tests;
