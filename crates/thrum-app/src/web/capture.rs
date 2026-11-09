//! Browser access to the renderer-owned submitted-output tap.
use super::{WebShell, WebWorld};
use thrum_render::runtime::{SceneSource, SubmittedImage};

impl<W: WebWorld, V: SceneSource<W::Page>> WebShell<W, V> {
    /// Begin a bounded submitted-output session on the healthy canvas device.
    /// # Errors
    /// Refuses a lost device, an active session or a resource allocation failure.
    pub fn begin_submitted_capture(&mut self, session: u64) -> Result<(), String> {
        if self.graphics.is_lost() {
            return Err("graphics device is lost".to_owned());
        }
        self.host
            .enable_submitted_capture(session)
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
