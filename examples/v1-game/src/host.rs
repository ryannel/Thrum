//! V1 scene/rules facade over the engine-owned bounded frame runtime.

use crate::{
    environment::DayCycle,
    projection::{GameProjection, ProjectionGap},
};
pub use runtime::{DriveReport, FrameConfig, FrameSample, SurfaceOutput};
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(not(target_arch = "wasm32"))]
use thrum_content_runtime::EnvironmentProduct;
use thrum_render::{capabilities::DeviceCapabilities, runtime};

mod scene_source;
mod world;
use scene_source::GameScene;
use world::{WorldDriver, WorldPort};
#[cfg(not(target_arch = "wasm32"))]
mod world_thread;
#[cfg(not(target_arch = "wasm32"))]
use world_thread::WorldClient;
#[cfg(not(target_arch = "wasm32"))]
pub use world_thread::WorldControls;

type Runtime = runtime::FrameHost<WorldPort, GameScene>;

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
    pub fn submit(self, surface: Option<&SurfaceOutput<'_>>) -> Result<DriveReport, String> {
        self.0.submit(surface).map_err(|error| error.to_string())
    }
    /// Record the scene before the platform acquires its drawable.
    /// # Errors
    /// Returns an engine extraction or recording failure.
    pub fn encode_scene(self) -> Result<EncodedFrame<'a>, String> {
        self.0
            .encode_scene()
            .map(EncodedFrame)
            .map_err(|error| error.to_string())
    }
}
/// One unsubmitted engine scene token, held across late drawable acquisition.
pub struct EncodedFrame<'a>(runtime::EncodedFrame<'a, WorldPort, GameScene>);
impl EncodedFrame<'_> {
    /// Skip a view while retaining its committed world update.
    pub fn skip(self) -> DriveReport {
        self.0.skip()
    }
    /// Append the optional drawable consumer and submit through the engine.
    /// # Errors
    /// Returns an engine submission failure.
    pub fn submit(self, surface: Option<&SurfaceOutput<'_>>) -> Result<DriveReport, String> {
        self.0.submit(surface).map_err(|error| error.to_string())
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

    /// Render the latest immutable world publication.
    /// # Errors
    /// Returns a render or authoritative-world fault.
    pub fn drive(
        &mut self,
        now: Duration,
        surface: Option<&SurfaceOutput<'_>>,
    ) -> Result<DriveReport, String> {
        self.inner.drive(now, surface)
    }

    /// Admit the world observation before scene encoding and late drawable
    /// acquisition on the render owner.
    /// # Errors
    /// Returns a clock, publication or GPU fault without rolling back a tick.
    pub fn advance(&mut self, now: Duration) -> Result<AdvancedFrame<'_>, String> {
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
    pub fn poll_completed(&mut self) -> Result<Option<FrameSample>, String> {
        self.inner.poll_completed()
    }

    /// Resize the surface binding after a coordinated platform change.
    /// # Errors
    /// Refuses an invalid or unavailable surface.
    pub fn resize_surface(&mut self, extent: [u32; 2]) -> Result<(), String> {
        self.inner.resize_surface(extent)
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
}

impl FrameHost {
    /// Maximum submitted scene packets retained by the engine.
    pub const MAX_IN_FLIGHT: usize = Runtime::MAX_IN_FLIGHT;
    /// Device generation selected by the current V1 platform setup.
    pub const DEVICE_GENERATION: u64 = 1;

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
    /// Refuses a mismatched device/generation or game/engine preparation failure.
    pub async fn new_with_capabilities(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        capabilities: &DeviceCapabilities,
        config: FrameConfig,
    ) -> Result<Self, String> {
        if !capabilities.matches(device) || capabilities.generation() != Self::DEVICE_GENERATION {
            return Err("capability record does not match the host device".to_owned());
        }
        let scene = GameScene::scene(config.render_extent)?;
        let world = WorldPort::Inline(Box::new(WorldDriver::new()?));
        let inner = Runtime::new(
            capabilities,
            queue,
            config,
            world,
            GameScene::new(),
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
        let offset = DayCycle::offset_for_time(self.tick(), fraction)
            .ok_or_else(|| "time of day must be finite and in [0, 1]".to_owned())?;
        if self.inner.source().day_phase_offset.to_bits() != offset.to_bits() {
            self.inner.source_mut().day_phase_offset = offset;
            self.inner.invalidate_history();
        }
        Ok(())
    }
    /// Change the game camera elevation and reset presentation history.
    /// # Errors
    /// Refuses a nonfinite pitch outside the supported scene range.
    pub fn set_camera_pitch(&mut self, radians: f32) -> Result<(), String> {
        if !radians.is_finite() || !(-0.6..=0.6).contains(&radians) {
            return Err("camera pitch must be finite and within ±0.6 rad".to_owned());
        }
        if self.inner.source().camera_pitch.to_bits() != radians.to_bits() {
            self.inner.source_mut().camera_pitch = radians;
            self.inner.invalidate_history();
        }
        Ok(())
    }

    /// Turn the game camera and reset presentation history.
    /// # Errors
    /// Refuses a nonfinite yaw or one outside ±π radians.
    pub fn set_camera_yaw(&mut self, radians: f32) -> Result<(), String> {
        if !radians.is_finite()
            || !(-std::f32::consts::PI..=std::f32::consts::PI).contains(&radians)
        {
            return Err("camera yaw must be finite and within ±π rad".to_owned());
        }
        if self.inner.source().camera_yaw.to_bits() != radians.to_bits() {
            self.inner.source_mut().camera_yaw = radians;
            self.inner.invalidate_history();
        }
        Ok(())
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
    /// Stop wall-time admission, retaining accepted ticks.
    pub fn pause(&mut self) {
        self.inner.world_mut().pause();
    }
    /// Resume the drained inline world from a new monotonic origin.
    /// # Errors
    /// Returns a clock state or timestamp failure.
    pub fn resume(&mut self, now: Duration) -> Result<(), String> {
        self.inner.world_mut().resume(now)
    }
    /// Drain accepted ticks before resuming; hidden elapsed time stays excluded.
    /// # Errors
    /// Returns a world/engine fault or a backlog that made no progress.
    pub fn drain_and_resume(&mut self, now: Duration) -> Result<(), String> {
        loop {
            let report = self.drive(now, None)?;
            if report.queued_ticks == 0 {
                break;
            }
            if report.completed_ticks == 0 {
                return Err("paused world updates could not drain".to_owned());
            }
        }
        self.resume(now)
    }
    /// Latest committed authority tick.
    pub fn tick(&self) -> u64 {
        self.inner.world().tick()
    }
    /// Validate and replace continuous local input.
    /// # Errors
    /// Refuses invalid controls or a stopped owner.
    pub fn set_controls(&mut self, axes: [f32; 3]) -> Result<(), String> {
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
    pub fn advance(&mut self, now: Duration) -> Result<AdvancedFrame<'_>, String> {
        self.inner
            .advance(now)
            .map(AdvancedFrame)
            .map_err(|error| error.to_string())
    }
    /// Submitted GPU packets not yet fully retired.
    pub fn pending_frames(&self) -> usize {
        self.inner.pending_frames()
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
    /// Capture the latest committed endpoint for untimed visual inspection.
    /// # Errors
    /// Returns an engine recording, completion or readback failure.
    pub fn capture_pixels(&mut self) -> Result<Vec<u8>, String> {
        self.inner
            .capture_pixels()
            .map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests;
