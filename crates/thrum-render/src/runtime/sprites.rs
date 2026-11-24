//! Sprite-specific preparation on the common frame executor.

use super::{
    DriveReport, FrameError, FrameHost, FrameSample, FrameTarget, FrameWorld, Instant,
    PreparedRenderer, PreparedSample, SceneSource, SubmitFrame, SurfacePresenter, duration_ns_u32,
    preparation,
};
use crate::{
    contribution::{EffectTime, SceneContribution, SpriteContributionSurface},
    sprite::SpriteTarget,
    synthetic::{CaptureSource, CullMode, PixelReadback, ShadowSelection},
};
use std::{sync::Arc, time::Duration};

impl<W: FrameWorld, V: SceneSource<W::Page>> FrameHost<W, V> {
    pub(super) fn attach_sprite_contribution(
        &mut self,
        contribution: &mut dyn SceneContribution,
    ) -> Result<(), FrameError> {
        let PreparedRenderer::TwoD(prepared) = &mut self.prepared else {
            unreachable!()
        };
        check_stages(contribution)?;
        let graph = contribution.effect_graph();
        if !graph.after_opaque.is_empty() {
            return Err(FrameError::InvalidState(
                "2D scene refuses contribution after-opaque stage",
            ));
        }
        graph.validate().map_err(FrameError::EffectGraph)?;
        contribution.rebuild_2d(&SpriteContributionSurface {
            capabilities: &prepared.capabilities,
            queue: &self.queue,
        })?;
        check_stages(contribution)?;
        let graph = contribution.effect_graph();
        if !graph.after_opaque.is_empty() {
            return Err(FrameError::InvalidState(
                "2D scene refuses contribution after-opaque stage",
            ));
        }
        let previous = (
            std::mem::replace(&mut prepared.effects, graph),
            std::mem::replace(&mut prepared.contribution_passes, contribution.passes()),
        );
        let slots = preparation::prepare_capture_slots(
            &self.prepared,
            &self.queue,
            self.render_extent,
            self.presenter.is_some(),
            self.presenter.as_ref().map(SurfacePresenter::profile),
            self.slots[0].target.capture_source(),
        )
        .and_then(|mut slots| {
            preparation::bind_presenter(&self.prepared, &mut slots, self.presenter.as_ref())?;
            Ok(slots)
        });
        match slots {
            Ok(slots) => {
                self.slots = slots;
                self.visual_owner = Arc::new(());
                Ok(())
            }
            Err(error) => {
                let PreparedRenderer::TwoD(prepared) = &mut self.prepared else {
                    unreachable!()
                };
                (prepared.effects, prepared.contribution_passes) = previous;
                Err(error)
            }
        }
    }

    pub(super) fn prepare_sprite_frame(
        &mut self,
        slot_index: usize,
        report: &mut DriveReport,
        frame: SubmitFrame,
    ) -> Result<Option<PreparedSample>, FrameError> {
        if frame.source != self.world.source() {
            self.dropped_stale_views = self.dropped_stale_views.saturating_add(1);
            report.dropped_view = true;
            return Ok(None);
        }
        let start = Instant::now();
        let PreparedRenderer::TwoD(prepared) = &mut self.prepared else {
            unreachable!()
        };
        let FrameTarget::TwoD(target) = &mut self.slots[slot_index].target else {
            unreachable!()
        };
        let sprites = self
            .source
            .sprites(frame.source, frame.presentation_alpha, target.extent)?;
        if let Some(entry) = self.contribution.as_mut() {
            entry.commit_quality_knobs();
        }
        let upload = prepared
            .upload(
                &self.queue,
                target,
                sprites,
                frame.presentation_alpha,
                self.timing_requested,
            )
            .map_err(FrameError::Render)?;
        let upload_cpu = start.elapsed();
        Ok(Some(PreparedSample {
            sample: FrameSample {
                tick: frame.source.tick,
                presentation_alpha: frame.presentation_alpha,
                display_phase: frame.display_phase,
                observation: self.observation,
                presentation_lead: self.presentation_lead,
                clock_interpolating: frame.clock_interpolating,
                tick_due_time: frame.tick_due_time,
                platform_target: self.platform_target,
                lead_status: self.lead_status,
                submission_order: 0,
                render_extent: self.render_extent,
                governor: None,
                history_identity: [0; 2],
                jitter_phase: None,
                completed_ticks: frame.completed_ticks,
                queued_ticks: frame.queued_ticks,
                tick_cpu: frame.tick_cpu,
                extract_cpu: frame.extract_cpu,
                visual_age_ticks: 0,
                upload_cpu,
                view_update_ns: duration_ns_u32(upload_cpu),
                encode_cpu: Duration::ZERO,
                wall: Duration::ZERO,
                render_cpu: Duration::ZERO,
                gpu_pass_ns: None,
                gpu_present_ns: None,
                gpu_contribution_ns: None,
                surface_presented: false,
                surface_fault: None,
                timing_fault: false,
                native_segment: None,
                timing_requested: self.timing_requested,
                culling_mode: CullMode::Direct,
                culling_selection: self.culling_selection(),
                shadow_selection: ShadowSelection::default(),
                camera_culling: None,
                uploaded_rows: upload.rows,
                uploaded_bytes: upload.bytes,
                sprites: Some(upload.stats),
                moving: upload.moving,
                dropped_presentation: upload.dropped,
                dropped_light_references: 0,
                clamped_light_channels: 0,
                clamped_shadow_casters: 0,
                visual_revision: self.visual_revision,
            },
            source: frame.source,
            view_history_epoch: 0,
        }))
    }

    pub(super) fn record_sprite_endpoint(
        &mut self,
        readback: bool,
    ) -> Result<(Option<PixelReadback>, wgpu::SubmissionIndex), FrameError> {
        let source = self.world.source();
        let snapshot = self.world.publication()?;
        if super::publication_source(&snapshot) != source {
            return Err(FrameError::InvalidState(
                "capture publication is not current",
            ));
        }
        let PreparedRenderer::TwoD(prepared) = &mut self.prepared else {
            unreachable!()
        };
        prepared.extract(source, |sink| {
            self.source.extract_sprites(sink, &snapshot.page);
        });
        drop(snapshot);
        let mut target =
            SpriteTarget::prepare(prepared, self.render_extent, CaptureSource::Sdr, None)
                .map_err(|error| FrameError::gpu("capture sprite target", error))?;
        let frame = self.source.sprites(source, 1.0, target.extent)?;
        let upload = prepared
            .upload(&self.queue, &mut target, frame, 1.0, false)
            .map_err(|error| FrameError::gpu("capture sprite update", error))?;
        self.dropped_capture_rows = self
            .dropped_capture_rows
            .saturating_add(u64::from(upload.dropped));
        let mut pixels = readback.then(|| target.readback(&self.device));
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let time = EffectTime {
            epoch: source.epoch,
            timeline: source.timeline,
            tick: source.tick,
            tick_duration: self.world.tick_duration(),
            alpha: 1.0,
            display_phase: 1.0,
            capture: true,
        };
        prepared
            .record(
                &mut encoder,
                &target,
                None,
                pixels.as_mut(),
                self.contribution.as_deref_mut(),
                None,
                time,
            )
            .map_err(|error| FrameError::gpu("capture sprite encode", error))?;
        let submitted = self.queue.submit([encoder.finish()]);
        if let Some(contribution) = self.contribution.as_mut() {
            contribution.submitted(false);
        }
        Ok((pixels, submitted))
    }
}

fn check_stages(contribution: &dyn SceneContribution) -> Result<(), FrameError> {
    if let Some(stage) = contribution.draw_stages().first() {
        return Err(FrameError::InvalidState(match stage {
            crate::contribution::SceneDrawStage::Shadow => {
                "2D scene refuses contribution shadow stage"
            }
            crate::contribution::SceneDrawStage::Opaque => {
                "2D scene refuses contribution opaque stage"
            }
        }));
    }
    if !contribution.secondary_view_slots().is_empty() {
        return Err(FrameError::InvalidState(
            "2D scene refuses contribution secondary views",
        ));
    }
    if !contribution.effect_graph().opaque_reads.is_empty() {
        return Err(FrameError::InvalidState(
            "2D scene refuses contribution opaque reads",
        ));
    }
    Ok(())
}
