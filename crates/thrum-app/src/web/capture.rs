//! Browser access to the renderer-owned submitted-output tap.
use super::WebShell;
use crate::InlineWorld;
use thrum_render::runtime::{FrameError, SceneSource, SubmittedImage};

impl<W: InlineWorld, V: SceneSource<W::Page>> WebShell<W, V> {
    /// Begin a bounded submitted-output session on the healthy canvas device.
    /// # Errors
    /// Refuses a lost device, an active session or a resource allocation failure.
    pub fn begin_submitted_capture(&mut self, session: u64) -> Result<(), String> {
        self.begin_submitted_capture_with_limits(
            session,
            thrum_render::runtime::CaptureLimits::default(),
        )
        .map_err(|error| error.to_string())
    }

    /// Begin a submitted-output session with an explicit bounded observer allowance.
    /// # Errors
    /// Refuses loss, an active session or limits too small for the canvas extent.
    pub fn begin_submitted_capture_with_limits(
        &mut self,
        session: u64,
        limits: thrum_render::runtime::CaptureLimits,
    ) -> Result<(), FrameError> {
        if self.graphics.is_lost() {
            return Err(FrameError::InvalidState("graphics device is lost"));
        }
        self.host
            .enable_submitted_capture_with_limits(session, limits)
    }

    /// Begin scene-linear capture through the same bounded submitted-image tap.
    /// # Errors
    /// Refuses loss, an active session or resources that cannot be prepared.
    pub fn begin_linear_capture(&mut self, session: u64) -> Result<(), String> {
        if self.graphics.is_lost() {
            return Err("graphics device is lost".to_owned());
        }
        self.collect_completed()?;
        self.host
            .enable_submitted_capture_from(
                session,
                thrum_render::runtime::CaptureLimits::default(),
                thrum_render::synthetic::CaptureSource::SceneLinear,
            )
            .map_err(|e| e.to_string())
    }

    /// Poll one exact submitted image without advancing time or redrawing.
    /// # Errors
    /// Refuses loss, a retired generation or a failed mapping.
    pub fn next_submitted_image(&mut self) -> Result<Option<SubmittedImage>, String> {
        if self.graphics.is_lost() {
            return Err("graphics device was lost before the capture finished".to_owned());
        }
        self.collect_completed()?;
        self.host.try_submitted_image().map_err(|e| e.to_string())
    }

    /// Return the exported image's CPU lease to its generation-bound pool.
    pub fn recycle_submitted_image(&mut self, image: SubmittedImage) {
        self.host.recycle_submitted_image(image);
    }

    /// Retire the session, cancelling pending readbacks without waiting.
    pub fn end_submitted_capture(&mut self) {
        self.host.disable_submitted_capture();
    }
}
