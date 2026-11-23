//! Headless opportunities through the ordinary game, world lane and presenter.
//!
//! The output texture substitutes for an OS drawable. This adapter proves no
//! window, compositor, display-link or physical-display behavior.

use std::time::{Duration, Instant};

use thrum_core::sim::{WorldLaneFault, WorldLifecyclePolicy};
use thrum_render::{
    capabilities::DeviceCapabilities,
    present::SurfaceProfile,
    runtime::{
        CaptureLimits, DriveReport, FrameError, FrameHost, FrameSample, SubmittedImage,
        SurfaceOutput,
    },
    synthetic::{CaptureSource, OUTPUT_FORMAT},
};

use crate::{Game, Gpu, WorldLink};

pub mod clock;
pub mod thread_policy;
pub use crate::capture_writer as writer;

/// A prepared game on the calling render-owner thread, with its ordinary
/// autonomous world lane and an engine texture replacing the OS surface.
pub struct Offscreen<G: Game> {
    device: thrum_render::runtime::DeviceSite,
    game: G,
    host: FrameHost<G::World, G::Scene>,
    output: wgpu::Texture,
    view: wgpu::TextureView,
    extent: [u32; 2],
    format: wgpu::TextureFormat,
    origin: Instant,
    lifecycle: WorldLifecyclePolicy,
}

/// Explicit boundary diagnostic, never enabled by the ordinary game loop.
#[derive(Clone, Copy, Debug)]
pub enum NativeFault {
    /// Uninterrupted control with the same history reset checkpoint.
    Control,
    /// Reject optional native work and select the prepared twin.
    Reject,
    /// Replace the engine presenter consumer after an extent refit/restore.
    Surface,
    /// Inject the raw command-buffer error observation after real native writes.
    CommandError,
    /// Destroy the old wgpu device after encoding; abandon without a flush.
    DeviceLoss,
}

impl<G: Game> Offscreen<G> {
    /// Exercise a labelled failure at a drained capture boundary. Capture is
    /// disabled before injected work; the caller restarts it after recovery.
    /// # Errors
    /// Refuses pending frames, missing diagnostic or unexpected error ordering.
    pub fn native_fault(&mut self, fault: NativeFault) -> Result<(), FrameError> {
        if self.host.pending_frames() != 0 {
            return Err(FrameError::InvalidState(
                "native fault needs a drained diagnostic",
            ));
        }
        if matches!(fault, NativeFault::CommandError) && self.host.native_witness_report().is_none()
        {
            return Err(FrameError::InvalidState(
                "command-error injection needs a native diagnostic",
            ));
        }
        self.host.disable_submitted_capture();
        match fault {
            NativeFault::Control => self.host.invalidate_history(),
            NativeFault::Reject => {
                self.host.reject_native_provider();
            }
            NativeFault::Surface => {
                let _transition = self.device.transition("headless surface replacement");
                self.host
                    .resize_surface([self.extent[0] - 16, self.extent[1]])?;
                self.host.resize_surface(self.extent)?;
                self.output = presenter_output(&self.device, self.extent, self.format);
                self.view = self
                    .output
                    .create_view(&wgpu::TextureViewDescriptor::default());
            }
            NativeFault::DeviceLoss => {
                let device = self.device.clone();
                let encoded = self.host.advance(self.origin.elapsed())?.encode_scene()?;
                device.destroy();
                encoded.abandon_lost_device();
            }
            NativeFault::CommandError => {
                self.host.inject_native_completion_error();
                self.host
                    .advance(self.origin.elapsed())?
                    .encode_scene()?
                    .submit(Some(&SurfaceOutput {
                        view: &self.view,
                        extent: self.extent,
                    }))?;
                self.device
                    .poll(wgpu::PollType::Wait {
                        submission_index: None,
                        timeout: Some(Duration::from_secs(10)),
                    })
                    .map_err(|error| FrameError::gpu("native fault wait", error))?;
                if !matches!(
                    self.host.poll_completed(),
                    Err(FrameError::NativeCommandBuffer { .. })
                ) {
                    return Err(FrameError::InvalidState(
                        "native error failed to reach generation owner",
                    ));
                }
            }
        }
        Ok(())
    }

    /// Rebuild GPU resources from the retained game/world after a native error
    /// or explicit device loss. The platform supplies the newer generation.
    /// # Errors
    /// Refuses stale generation or failed detached preparation before publishing.
    pub async fn recover_device(
        &mut self,
        capabilities: &DeviceCapabilities,
        queue: &wgpu::Queue,
    ) -> Result<(), FrameError> {
        let mut candidate = self
            .host
            .device_prepare_inputs()?
            .prepare(
                capabilities,
                queue,
                Some(SurfaceProfile {
                    format: self.format,
                    extent: self.extent,
                }),
            )
            .await?;
        self.host.publish_device(&mut candidate)?;
        self.output = presenter_output(capabilities.site(), self.extent, self.format);
        self.view = self
            .output
            .create_view(&wgpu::TextureViewDescriptor::default());
        self.device = capabilities.site().clone();
        Ok(())
    }

    /// Prepare through `Game::prepare`, including its resident world lane.
    /// Call and drive this on the render-owner thread, without AppKit.
    /// # Errors
    /// Returns the game's preparation error or invalid output dimensions.
    pub fn new(
        game: G,
        device: &wgpu::Device,
        capabilities: &DeviceCapabilities,
        queue: &wgpu::Queue,
        extent: [u32; 2],
    ) -> Result<Self, String> {
        Self::with_surface(
            game,
            device,
            capabilities,
            queue,
            SurfaceProfile {
                format: OUTPUT_FORMAT,
                extent,
            },
        )
    }

    /// As [`Self::new`], with an engine texture of the given format and extent in place of
    /// the OS surface, such as the format a platform's drawable would have.
    /// # Errors
    /// Returns the game's preparation error or invalid output dimensions.
    pub fn with_surface(
        mut game: G,
        device: &wgpu::Device,
        capabilities: &DeviceCapabilities,
        queue: &wgpu::Queue,
        surface: SurfaceProfile,
    ) -> Result<Self, String> {
        let SurfaceProfile { format, extent } = surface;
        if extent.contains(&0) {
            return Err("offscreen output extent must be nonzero".to_owned());
        }
        let prepared = pollster::block_on(game.prepare(
            Gpu {
                device,
                queue,
                capabilities,
                surface,
            },
            Box::new(|| {}),
        ))?;
        let output = presenter_output(capabilities.site(), extent, format);
        let view = output.create_view(&wgpu::TextureViewDescriptor::default());
        Ok(Self {
            device: capabilities.site().clone(),
            game,
            host: prepared.host,
            output,
            view,
            extent,
            format,
            origin: prepared.origin.unwrap_or_else(Instant::now),
            lifecycle: WorldLifecyclePolicy::default(),
        })
    }

    /// The same monotonic origin supplied to the game's world lane.
    pub fn origin(&self) -> Instant {
        self.origin
    }

    /// The prepared game.
    pub fn game(&self) -> &G {
        &self.game
    }

    /// The game's accepted control values, in engine units, for provenance.
    pub fn controls(&self) -> &thrum_controls::Controls {
        self.game.controls()
    }

    /// Apply a validated parameter file through the game's normal semantic
    /// handler. Pause/resume acknowledges the existing world-lane barrier.
    /// # Errors
    /// Refuses invalid controls or a lifecycle acknowledgement timeout.
    pub fn parameters(&mut self, json: &str) -> Result<(), String> {
        let report = self
            .game
            .controls_mut()
            .apply_file(json)
            .map_err(|e| e.to_string())?;
        if !report.problems.is_empty() {
            return Err(format!("offscreen controls: {report}"));
        }
        let (edits, paused) = self.game.take_edits();
        G::apply(&mut self.host, edits)?;
        if let Some(paused) = paused {
            let world = self.game.world().ok_or("game has no world lifecycle")?;
            self.lifecycle.request(paused);
            settle_world_lifecycle(world, &mut self.lifecycle)?;
            self.game.observe_paused(paused);
        }
        Ok(())
    }

    /// Reconcile a current overload through the world lifecycle policy and report its refused ticks.
    /// A render report can outlive an overload, so the current world status decides.
    /// # Errors
    /// Returns a world fault, refused transition or acknowledgement timeout.
    pub fn recover_world_overload(&mut self) -> Result<Option<u64>, FrameError> {
        let world = self
            .game
            .world()
            .ok_or(FrameError::InvalidState("game has no world lifecycle"))?;
        let (episodes, refused) = (
            self.lifecycle.overload_count(),
            self.lifecycle.refused_ticks(),
        );
        settle_world_lifecycle(world, &mut self.lifecycle).map_err(FrameError::World)?;
        Ok((self.lifecycle.overload_count() != episodes)
            .then_some(self.lifecycle.refused_ticks() - refused))
    }

    /// Enable the renderer's three-slot tap without creating another GPU owner.
    /// # Errors
    /// Returns an already-enabled capture error.
    pub fn enable_capture(&mut self, session: u64) -> Result<(), String> {
        self.host
            .enable_submitted_capture(session)
            .map_err(|e| e.to_string())
    }

    /// Enable capture with a measured, explicitly bounded working-set profile.
    /// # Errors
    /// Refuses invalid capacities or a pixel-byte allowance overflow.
    pub fn enable_capture_with_limits(
        &mut self,
        session: u64,
        limits: CaptureLimits,
    ) -> Result<(), String> {
        self.host
            .enable_submitted_capture_with_limits(session, limits)
            .map_err(|e| e.to_string())
    }

    /// Enable the same submitted-image tap at a declared prepared engine stage.
    /// # Errors
    /// Refuses pending frames during a stage change, invalid capture capacities
    /// or preparation failure. Drain ordinary frames before changing stages.
    pub fn enable_capture_from(
        &mut self,
        session: u64,
        limits: CaptureLimits,
        source: CaptureSource,
    ) -> Result<(), String> {
        self.host
            .enable_submitted_capture_from(session, limits, source)
            .map_err(|error| error.to_string())
    }

    /// Submit from actual elapsed host time through the ordinary surface gate.
    /// # Errors
    /// Returns the ordinary world/render failure; no capture admission bypass.
    pub fn step(&mut self) -> Result<DriveReport, String> {
        self.host.set_presentation_lead(Duration::ZERO);
        self.drive(self.origin.elapsed())
    }

    /// Consume a clock target using the native render actor's lead policy.
    /// Expired targets have zero lead; controls/render delay is included.
    /// # Errors
    /// Returns the ordinary world/render failure.
    pub fn step_for(&mut self, opportunity: &clock::Opportunity) -> Result<DriveReport, String> {
        let observation = self.origin.elapsed();
        let lead = thrum_render::frame::display_target_lead(
            opportunity
                .target
                .saturating_duration_since(self.origin)
                .as_secs_f64(),
            observation.as_secs_f64(),
        );
        self.host.set_presentation_lead(lead.duration);
        self.host.set_presentation_target(
            opportunity.target.saturating_duration_since(self.origin),
            lead.status,
        );
        self.drive(observation)
    }

    fn drive(&mut self, observation: Duration) -> Result<DriveReport, String> {
        G::apply(&mut self.host, self.game.frame_edits())?;
        let report = self
            .host
            .drive(
                observation,
                Some(&SurfaceOutput {
                    view: &self.view,
                    extent: self.extent,
                }),
            )
            .map_err(|e| e.to_string())?;
        self.game.observe(G::readback(&self.host));
        Ok(report)
    }

    /// Submit at an explicit observation time on the world's clock, through the ordinary
    /// surface gate, for a capture that steps a fixed clock itself.
    /// # Errors
    /// Returns the ordinary world/render failure.
    pub fn step_at(&mut self, observation: Duration) -> Result<DriveReport, String> {
        self.host.set_presentation_lead(Duration::ZERO);
        self.drive(observation)
    }

    /// The prepared game's frame host, for the game's own operations on it.
    pub fn host(&self) -> &FrameHost<G::World, G::Scene> {
        &self.host
    }

    /// The prepared game's frame host, for the game's own operations on it.
    pub fn host_mut(&mut self) -> &mut FrameHost<G::World, G::Scene> {
        &mut self.host
    }

    /// Submission ID associated with the last successful ordinary step.
    pub fn last_submission(&self) -> Option<u64> {
        self.host.last_submission()
    }

    /// Drain one normal completed frame, driving nonblocking device polling.
    /// # Errors
    /// Returns a GPU completion failure.
    pub fn completed(&mut self) -> Result<Option<FrameSample>, String> {
        self.host.poll_completed().map_err(|e| e.to_string())
    }

    /// Number of ordinary GPU frames still in flight.
    pub fn pending_frames(&self) -> usize {
        self.host.pending_frames()
    }

    /// Prepared scene kind and target storage for capture diagnostics.
    pub fn scene_storage(&self) -> (thrum_render::runtime::SceneKind, u64, String) {
        let bytes = u64::from(self.output.width())
            * u64::from(self.output.height())
            * u64::from(self.output.format().block_copy_size(None).unwrap_or(0));
        (self.host.scene_kind(), bytes, self.host.storage_report())
    }

    /// Pass names compiled for an ordinary frame of the prepared scene.
    /// # Errors
    /// Returns a host graph inspection failure.
    pub fn scene_pass_names(&self) -> Result<Vec<String>, FrameError> {
        self.host
            .scene_graph()
            .map(|graph| graph.pass_names().map(str::to_owned).collect())
    }

    /// Outcomes of the optional labelled native composition diagnostic.
    #[must_use]
    pub fn native_witness_report(&self) -> Option<thrum_render::synthetic::NativeWitnessReport> {
        self.host.native_witness_report()
    }

    /// Reset submitted view correspondence before the next ordinary frame.
    pub fn invalidate_history(&mut self) {
        self.host.invalidate_history();
    }

    /// Drain one submitted image, without a GPU wait or endpoint redraw.
    /// # Errors
    /// Returns a mapping or old-session error.
    pub fn image(&mut self) -> Result<Option<SubmittedImage>, String> {
        self.host.try_submitted_image().map_err(|e| e.to_string())
    }

    /// Explicitly omitted readbacks; any omission invalidates continuous proof.
    pub fn omissions(&self) -> u64 {
        self.host.submitted_capture_omissions()
    }

    /// Current staging occupancy and CPU image leases for observer-cost records.
    pub fn capture_usage(&self) -> Option<(usize, usize)> {
        self.host.submitted_capture_usage()
    }

    /// Return an image only after its writer has released the CPU byte lease.
    pub fn recycle(&mut self, image: SubmittedImage) {
        self.host.recycle_submitted_image(image);
    }

    /// The final output used by the ordinary presenter, for correspondence tests.
    pub fn output(&self) -> &wgpu::Texture {
        &self.output
    }

    /// Collect optional GPU pass timings, which have their own resolve and readback.
    pub fn set_timing_requested(&mut self, requested: bool) {
        self.host.set_timing_requested(requested);
    }
}

fn settle_world_lifecycle(
    world: &dyn WorldLink,
    lifecycle: &mut WorldLifecyclePolicy,
) -> Result<(), String> {
    wait_for_lifecycle(world, lifecycle)?;
    match world.check_health() {
        Ok(_) => {}
        Err(WorldLaneFault::Overload(refused)) => lifecycle.overloaded(refused),
        Err(error) => return Err(error.to_string()),
    }
    while let Some(request) = lifecycle.next_request() {
        world
            .request_lifecycle(request)
            .map_err(|error| error.to_string())?;
        lifecycle.accepted(request);
        wait_for_lifecycle(world, lifecycle)?;
    }
    Ok(())
}

fn wait_for_lifecycle(
    world: &dyn WorldLink,
    lifecycle: &mut WorldLifecyclePolicy,
) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(5);
    while lifecycle.needs_reply() {
        if let Some(reply) = world
            .poll_lifecycle_reply()
            .map_err(|error| error.to_string())?
        {
            return lifecycle
                .acknowledge(reply.request)
                .map_err(|error| error.to_string());
        }
        if Instant::now() >= deadline {
            return Err("world lifecycle timeout".to_owned());
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    Ok(())
}

fn presenter_output(
    device: &thrum_render::runtime::DeviceSite,
    [width, height]: [u32; 2],
    format: wgpu::TextureFormat,
) -> wgpu::Texture {
    let _transition = device.transition("headless presenter output");
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("headless ordinary presenter output"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}
