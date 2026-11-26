//! Replace a scene while preserving the surface and render owner.

use std::{
    sync::{Arc, atomic::Ordering},
    time::Instant,
};

use thrum_render::present::SurfaceProfile;

use super::{MobileGame, Owner, PrepareGpu, SceneReady, SceneRequest, TickPermission};

impl<G: MobileGame> Owner<G> {
    pub(super) fn replace_scene(
        &mut self,
        request: SceneRequest,
    ) -> Result<Option<SceneReady<G>>, String> {
        let gpu = self
            .gpu
            .as_mut()
            .ok_or("iOS render owner is not prepared")?;
        let background = || self.signals.background.load(Ordering::Acquire);
        let epoch = self.lease.epoch();
        let extent = self.lease.observed_extent();
        if background() || extent.contains(&0) || gpu.destroyed || gpu.watch.is_lost() {
            return Ok(None);
        }
        // The last scene's GPU packets must retire before its host goes away.
        let retired = gpu.host.poll_completed();
        if gpu.watch.is_lost() {
            return Ok(None);
        }
        retired.map_err(|error| error.to_string())?;
        if gpu.host.pending_frames() != 0 {
            return Ok(None);
        }
        let origin = Instant::now();
        let candidate = pollster::block_on(G::prepare(
            PrepareGpu {
                device: &gpu.device,
                queue: &gpu.queue,
                capabilities: gpu.host.device_capabilities(),
                surface: SurfaceProfile {
                    format: gpu.format,
                    extent,
                },
            },
            origin,
            crate::ios::notify_world,
            Some(request),
            self.diagnostics,
        ));
        if background() || gpu.watch.is_lost() || !self.lease.is_current(epoch) {
            return Ok(None);
        }
        let (mut candidate, presentation, ready) =
            SceneReady::split(candidate.map_err(|error| error.to_string())?, origin);
        candidate.set_tick_submission_permission(Box::new(TickPermission {
            lease: Arc::clone(&self.lease),
            request_epoch: Arc::clone(&gpu.request_epoch),
            background: Arc::clone(&self.signals.background),
        }));
        // Publish at a live lease without holding it across preparation or
        // stopping the old world lane.
        let mut candidate = Some(candidate);
        let previous = self.lease.section(epoch, |_| {
            if background() || gpu.watch.is_lost() {
                return None;
            }
            candidate
                .take()
                .map(|host| std::mem::replace(&mut gpu.host, host))
        });
        let Some(Some(mut previous)) = previous else {
            return Ok(None);
        };
        gpu.host_extent = extent;
        gpu.presentation = presentation;
        if let Err(error) = self
            .capture
            .cancel(&mut previous, &self.signals, "scene replaced")
        {
            self.capture.failed(&mut previous, &self.signals, &error);
        }
        self.origin = origin;
        Ok(Some(ready))
    }
}
