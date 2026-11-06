//! Exclusive tokens for late drawable acquisition and cancellation.

use super::{
    DriveReport, FrameError, FrameHost, FrameSample, FrameWorld, Instant, SceneSource, SlotState,
    SurfaceOutput,
};
use crate::extract::SourceTick;
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
    /// drawable. The returned token owns the encoder and its frame slot.
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
        let prepared = match host.prepare_extracted(slot_index, report, frame) {
            Ok(Some(prepared)) => prepared,
            Ok(None) => {
                return Err(FrameError::InvalidState(
                    "serial extraction had no current source",
                ));
            }
            Err(error) => {
                host.queue.submit([]);
                return Err(error);
            }
        };
        let (encoder, scene_encode_cpu) = host.encode_scene_only(slot_index)?;
        Ok(EncodedFrame {
            host: self.host.take().ok_or("advanced host consumed")?,
            report: self.report.take(),
            encoder: Some(encoder),
            sample: Some(prepared.sample),
            slot_index,
            wall_start: self.wall_start,
            scene_encode_cpu,
            source: prepared.source,
        })
    }
}

impl<W: FrameWorld, V: SceneSource<W::Page>> Drop for AdvancedFrame<'_, W, V> {
    fn drop(&mut self) {
        let Some((host, report)) = self.host.as_deref_mut().zip(self.report.as_ref()) else {
            return;
        };
        if report.overload.is_some() {
            host.dropped_views = host
                .dropped_views
                .saturating_add(u64::from(report.completed_ticks > 0));
        } else if host.world.has_current_page()
            && (report.completed_ticks > 0 || self.recovered_publication)
        {
            host.dropped_views = host.dropped_views.saturating_add(1);
            if host.presenter.is_some() {
                host.dropped_surface = host.dropped_surface.saturating_add(1);
            }
        }
    }
}

/// One offscreen scene encoded before the platform acquires a drawable.
/// Dropping it discards the unsubmitted encoder and its timestamp reservations.
pub struct EncodedFrame<'a, W: FrameWorld, V: SceneSource<W::Page>> {
    host: &'a mut FrameHost<W, V>,
    report: Option<DriveReport>,
    encoder: Option<wgpu::CommandEncoder>,
    sample: Option<FrameSample>,
    slot_index: usize,
    wall_start: Instant,
    scene_encode_cpu: Duration,
    source: SourceTick,
}

impl<W: FrameWorld, V: SceneSource<W::Page>> EncodedFrame<'_, W, V> {
    /// Discard an encoded scene when drawable acquisition failed. The world
    /// update remains committed; queued uploads are flushed without a scene
    /// pass or temporal-history publication.
    /// # Panics
    /// Panics only if the frame's privately owned report was taken without
    /// consuming the frame, which no public operation permits.
    pub fn skip(mut self) -> DriveReport {
        let mut report = self.report.take().expect("encoded report belongs to frame");
        report.dropped_view = true;
        report.wall = self.wall_start.elapsed();
        report
    }

    /// Append the presentation pass to the existing encoder and submit once.
    /// A missing drawable submits only the scene and counts the missed surface.
    /// # Errors
    /// Returns a GPU submission fault; committed world ticks remain committed.
    pub fn submit(
        mut self,
        surface: Option<&SurfaceOutput<'_>>,
    ) -> Result<DriveReport, FrameError> {
        if surface.is_some() && self.host.presenter.is_none() {
            return Err(FrameError::InvalidState(
                "encoded host has no surface presenter",
            ));
        }
        let mut encoder = self.encoder.take().ok_or("encoded scene consumed")?;
        let present_start = Instant::now();
        let slot = &mut self.host.slots[self.slot_index];
        let presented = if let (Some(presenter), Some(binding), Some(surface)) = (
            self.host.presenter.as_ref(),
            slot.presentation.as_ref(),
            surface,
        ) {
            presenter
                .encode(
                    &mut encoder,
                    binding,
                    surface.view,
                    surface.extent,
                    Some(&mut slot.timings),
                )
                .is_ok()
        } else {
            false
        };
        if self.host.presenter.is_some() && !presented {
            self.host.dropped_surface = self.host.dropped_surface.saturating_add(1);
            if let Some(report) = self.report.as_mut() {
                report.dropped_view = true;
            }
        }
        let encode_cpu = self.scene_encode_cpu + present_start.elapsed();
        let mut report = self.report.take().ok_or("encoded report consumed")?;
        let sample = self.sample.take().ok_or("encoded sample consumed")?;
        self.host.finish_encoded(
            self.slot_index,
            &mut report,
            encoder,
            sample,
            self.wall_start,
            encode_cpu,
            presented,
        )?;
        self.host.prepared.commit_frame();
        self.host.history_reset_pending = false;
        self.host.last_view_source = Some(self.source);
        Ok(report)
    }
}

impl<W: FrameWorld, V: SceneSource<W::Page>> Drop for EncodedFrame<'_, W, V> {
    fn drop(&mut self) {
        if let Some(encoder) = self.encoder.take() {
            drop(encoder);
            let discarded = self.host.slots[self.slot_index]
                .timings
                .discard_unsubmitted(self.host.generation);
            debug_assert!(discarded.is_ok(), "unsubmitted scene timing state changed");
            // Queue::write_buffer stages bytes until submit, even if the
            // command encoder is dropped. Empty submit releases that staging
            // after GPU completion without rendering the hidden scene.
            self.host.queue.submit([]);
            self.host.dropped_views = self.host.dropped_views.saturating_add(1);
            if self.host.presenter.is_some() {
                self.host.dropped_surface = self.host.dropped_surface.saturating_add(1);
            }
        }
    }
}
