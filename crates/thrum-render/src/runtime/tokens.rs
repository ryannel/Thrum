//! Exclusive tokens for late drawable acquisition and cancellation.

use super::{
    DriveReport, FrameError, FrameHost, FrameWorld, Instant, RecordedScene, SceneSource, SlotState,
    SurfaceOutput,
};
use std::time::Duration;

/// One admitted simulation callback awaiting optional presentation. Consuming
/// it once keeps a delayed surface acquisition tied to the exact world step.
pub struct AdvancedFrame<'a, W: FrameWorld, V: SceneSource<W::Page>> {
    pub(super) host: Option<&'a mut FrameHost<W, V>>,
    pub(super) report: Option<DriveReport>,
    pub(super) tick_cpu: Duration,
    pub(super) wall_start: Instant,
    pub(super) recovered_publication: bool,
}

impl<'a, W: FrameWorld, V: SceneSource<W::Page>> AdvancedFrame<'a, W, V> {
    /// Whether a serial frame can use a drawable after world admission.
    /// Native extraction may finish later and remains separately scheduled.
    pub fn can_acquire_serial(&self) -> bool {
        let Some(host) = self.host.as_deref() else {
            return false;
        };
        let Some(report) = self.report.as_ref() else {
            return false;
        };
        report.overload.is_none()
            && host.world.has_current_page()
            && host.presenter.is_some()
            && host
                .slots
                .iter()
                .any(|slot| matches!(slot.state, SlotState::Free))
    }

    /// Encode and submit the admitted frame once, with an optional acquired surface.
    /// # Errors
    /// Returns a presentation fault after simulation admission; committed ticks
    /// remain committed even when optional rendering fails.
    pub fn submit(
        mut self,
        surface: Option<&SurfaceOutput<'_>>,
    ) -> Result<DriveReport, FrameError> {
        let report = self
            .report
            .take()
            .ok_or("advanced frame already consumed")?;
        self.host
            .take()
            .ok_or("advanced host already consumed")?
            .finish_advance(
                report,
                self.tick_cpu,
                self.wall_start,
                self.recovered_publication,
                surface,
            )
    }

    /// Encode the serial scene while the platform has not yet acquired its
    /// drawable. A stale publication returns a dropped report in the token.
    /// # Errors
    /// Returns an extraction or GPU encoding fault; a failed optional view
    /// never rolls back an admitted world update.
    pub fn encode_scene(mut self) -> Result<EncodedFrame<'a, W, V>, FrameError> {
        if !self.can_acquire_serial() {
            return Err(FrameError::InvalidState(
                "serial scene is not ready for encoding",
            ));
        }
        let host = self.host.as_deref_mut().ok_or("advanced host consumed")?;
        let slot_index = host
            .slots
            .iter()
            .position(|slot| matches!(slot.state, SlotState::Free))
            .ok_or("no free scene slot")?;
        let report = self.report.as_mut().ok_or("advanced report consumed")?;
        let frame =
            host.extract_serial(self.tick_cpu, report.completed_ticks, report.queued_ticks)?;
        let ready = match host.prepare_extracted(slot_index, report, frame) {
            Ok(Some(prepared)) => Some(host.encode_scene_only(slot_index, prepared)?),
            Ok(None) => None,
            Err(error) => {
                host.queue.submit([]);
                return Err(error);
            }
        };
        let host = self.host.take().ok_or("advanced host consumed")?;
        Ok(EncodedFrame {
            ready: ready.map(|recorded| ReadyFrame { host, recorded }),
            report: self.report.take().ok_or("advanced report consumed")?,
            wall_start: self.wall_start,
        })
    }
}

impl<W: FrameWorld, V: SceneSource<W::Page>> Drop for AdvancedFrame<'_, W, V> {
    fn drop(&mut self) {
        let Some((host, report)) = self.host.as_deref_mut().zip(self.report.as_mut()) else {
            return;
        };
        host.skip_advance(report, self.recovered_publication);
    }
}

/// The result of attempting to encode before drawable acquisition.
/// Dropping a ready frame discards its unsubmitted encoder and timing reservations.
pub struct EncodedFrame<'a, W: FrameWorld, V: SceneSource<W::Page>> {
    ready: Option<ReadyFrame<'a, W, V>>,
    report: DriveReport,
    wall_start: Instant,
}

struct ReadyFrame<'a, W: FrameWorld, V: SceneSource<W::Page>> {
    host: &'a mut FrameHost<W, V>,
    recorded: RecordedScene,
}

impl<W: FrameWorld, V: SceneSource<W::Page>> EncodedFrame<'_, W, V> {
    /// Whether a scene is ready for the platform to acquire a drawable.
    pub fn can_acquire_surface(&self) -> bool {
        self.ready.is_some()
    }

    /// Candidate/predecessor identity packed for this exact encoded token.
    pub fn history_identity(&self) -> Option<[u64; 2]> {
        Some(
            self.ready
                .as_ref()?
                .recorded
                .prepared
                .sample
                .history_identity,
        )
    }

    /// Inspect the resources and matrix used by this exact encoded token.
    pub fn frame_extent(&self) -> Option<super::FrameExtent> {
        let ready = self.ready.as_ref()?;
        let host = &ready.host;
        let presenter = host.presenter.as_ref()?;
        let (source, viewport) = presenter.source_viewport();
        let (target_textures, target_bytes) = host.slots[ready.recorded.slot_index]
            .target
            .texture_allocation();
        Some(super::FrameExtent {
            target_textures,
            target_bytes,
            attachments: host.slots[ready.recorded.slot_index]
                .target
                .attachment_extents(),
            view_projection: host.scene.view.view_proj_columns(),
            previous_view_projection: host.prepared.submitted_view_projection(),
            source,
            destination: presenter.extent(),
            viewport,
            submission: host.next_submission_order,
            revision: host.presenter_generation,
        })
    }

    /// Drop optional presentation and return the admitted world progress.
    /// Pending uploads are flushed without a scene or history submission.
    pub fn skip(mut self) -> DriveReport {
        self.report.dropped_view = true;
        self.report.wall = self.wall_start.elapsed();
        self.report
    }

    /// Cancel a frame whose foreground lease was revoked. No queue work is
    /// permitted in the background; resume must reset temporal correspondence.
    pub fn cancel_background(mut self) -> DriveReport {
        self.report.dropped_view = true;
        if let Some(host) = self.discard_recording() {
            host.invalidate_history();
        }
        self.report.wall = self.wall_start.elapsed();
        self.report
    }

    /// Discard this frame after the platform observes loss of its device.
    ///
    /// Committed world progress is retained. The recorded encoder and timing
    /// reservation are released without flushing staged writes to the lost
    /// queue. Use [Self::skip] for a healthy device so later frames can reuse
    /// the uploaded data.
    pub fn abandon_lost_device(mut self) {
        if let Some(host) = self.discard_recording() {
            host.invalidate_history();
        }
    }

    /// Submit the recorded scene and optional surface pass once, or return
    /// the dropped report if the publication was stale.
    /// # Errors
    /// Returns a GPU submission fault; committed world ticks remain committed.
    pub fn submit(
        mut self,
        surface: Option<&SurfaceOutput<'_>>,
    ) -> Result<DriveReport, FrameError> {
        if let Some(ready) = self.ready.take() {
            ready.host.finish_encoded(
                ready.recorded,
                &mut self.report,
                surface,
                self.wall_start,
            )?;
        } else {
            self.report.wall = self.wall_start.elapsed();
        }
        Ok(self.report)
    }

    fn discard_recording(&mut self) -> Option<&mut FrameHost<W, V>> {
        let ReadyFrame { host, recorded } = self.ready.take()?;
        drop(recorded.encoder);
        if let Some(index) = recorded.capture_slot
            && let Some(tap) = host.submitted_capture.as_mut()
        {
            tap.abandon(index);
        }
        let discarded = host.slots[recorded.slot_index]
            .timings
            .discard_unsubmitted(host.generation);
        debug_assert!(discarded.is_ok(), "unsubmitted scene timing state changed");
        host.dropped_views = host.dropped_views.saturating_add(1);
        if host.presenter.is_some() {
            host.dropped_surface = host.dropped_surface.saturating_add(1);
        }
        Some(host)
    }
}

impl<W: FrameWorld, V: SceneSource<W::Page>> Drop for EncodedFrame<'_, W, V> {
    fn drop(&mut self) {
        if let Some(host) = self.discard_recording() {
            // Queue::write_buffer stages bytes until submit. Release that staging
            // even when this encoder is discarded without a scene submission.
            staging_flush::flush(&host.queue);
        }
    }
}

#[cfg(not(test))]
mod staging_flush {
    pub(super) fn flush(queue: &wgpu::Queue) {
        queue.submit([]);
    }
}

#[cfg(test)]
pub(super) mod staging_flush {
    use std::cell::Cell;

    thread_local! {
        static ATTEMPTS: Cell<u64> = const { Cell::new(0) };
    }

    pub(in crate::runtime) fn attempts() -> u64 {
        ATTEMPTS.get()
    }

    pub(super) fn flush(queue: &wgpu::Queue) {
        // Count the attempt before wgpu can silently discard a lost-device submit.
        ATTEMPTS.set(ATTEMPTS.get() + 1);
        queue.submit([]);
    }
}
