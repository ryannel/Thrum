//! Headless opportunities through the ordinary game, world lane and presenter.
//!
//! The output texture substitutes for an OS drawable. This adapter proves no
//! window, compositor, display-link or physical-display behavior.

use std::time::{Duration, Instant};

use thrum_core::sim::WorldLifecycle;
use thrum_render::{
    capabilities::DeviceCapabilities,
    present::SurfaceProfile,
    runtime::{CaptureLimits, DriveReport, FrameHost, FrameSample, SubmittedImage, SurfaceOutput},
    synthetic::{CaptureSource, OUTPUT_FORMAT},
};

use crate::{Game, Gpu};

pub mod clock;
pub mod thread_policy;
pub use crate::capture_writer as writer;

/// A prepared game on the calling render-owner thread, with its ordinary
/// autonomous world lane and an engine texture replacing the OS surface.
pub struct Offscreen<G: Game> {
    game: G,
    host: FrameHost<G::World, G::Scene>,
    output: wgpu::Texture,
    view: wgpu::TextureView,
    extent: [u32; 2],
    origin: Instant,
}

impl<G: Game> Offscreen<G> {
    /// Prepare through `Game::prepare`, including its resident world lane.
    /// Call and drive this on the render-owner thread, without AppKit.
    /// # Errors
    /// Returns the game's preparation error or invalid output dimensions.
    pub fn new(
        mut game: G,
        device: &wgpu::Device,
        capabilities: &DeviceCapabilities,
        queue: &wgpu::Queue,
        extent: [u32; 2],
    ) -> Result<Self, String> {
        if extent.contains(&0) {
            return Err("offscreen output extent must be nonzero".to_owned());
        }
        let prepared = game.prepare(
            Gpu {
                device,
                queue,
                capabilities,
                surface: SurfaceProfile {
                    format: OUTPUT_FORMAT,
                    extent,
                },
            },
            Box::new(|| {}),
        )?;
        let output = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("headless ordinary presenter output"),
            size: wgpu::Extent3d {
                width: extent[0],
                height: extent[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: OUTPUT_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let view = output.create_view(&wgpu::TextureViewDescriptor::default());
        Ok(Self {
            game,
            host: prepared.host,
            output,
            view,
            extent,
            origin: prepared.origin,
        })
    }

    /// The same monotonic origin supplied to the game's world lane.
    pub fn origin(&self) -> Instant {
        self.origin
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
            let request = if paused {
                WorldLifecycle::Pause
            } else {
                WorldLifecycle::Resume
            };
            world
                .request_lifecycle(request)
                .map_err(|e| format!("lifecycle request: {e:?}"))?;
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                if world
                    .poll_lifecycle_reply()
                    .map_err(|e| format!("lifecycle reply: {e:?}"))?
                    .is_some_and(|reply| reply.request == request)
                {
                    break;
                }
                if Instant::now() >= deadline {
                    return Err("world lifecycle timeout".to_owned());
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            self.game.observe_paused(paused);
        }
        Ok(())
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
        self.drive()
    }

    /// Consume a clock target using the native render actor's lead policy.
    /// Expired targets have zero lead; controls/render delay is included.
    /// # Errors
    /// Returns the ordinary world/render failure.
    pub fn step_for(&mut self, opportunity: &clock::Opportunity) -> Result<DriveReport, String> {
        let lead = thrum_render::frame::display_target_lead(
            opportunity
                .target
                .saturating_duration_since(self.origin)
                .as_secs_f64(),
            self.origin.elapsed().as_secs_f64(),
        );
        self.host.set_presentation_lead(lead.duration);
        self.drive()
    }

    fn drive(&mut self) -> Result<DriveReport, String> {
        let report = self
            .host
            .drive(
                self.origin.elapsed(),
                Some(&SurfaceOutput {
                    view: &self.view,
                    extent: self.extent,
                }),
            )
            .map_err(|e| e.to_string())?;
        Ok(report)
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
}
