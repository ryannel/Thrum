//! Sprite-specific preparation on the common frame executor.

use super::{
    DriveReport, FrameError, FrameHost, FrameSample, FrameTarget, FrameWorld, Instant,
    PendingCapture, PreparedRenderer, PreparedSample, SceneSource, SubmitFrame, SurfacePresenter,
    duration_ns_u32, preparation,
};
use crate::{
    contribution::{EffectTime, SceneContribution, SpriteContributionSurface},
    sprite::SpriteTarget,
    synthetic::{CaptureSource, CullMode, ShadowSelection},
};
use std::{sync::Arc, time::Duration};

impl<W: FrameWorld, V: SceneSource<W::Page>> FrameHost<W, V> {
    pub(super) fn attach_sprite_contribution(
        &mut self,
        contribution: &mut dyn SceneContribution,
    ) -> Result<(), String> {
        let PreparedRenderer::TwoD(prepared) = &mut self.prepared else {
            unreachable!()
        };
        check_stages(contribution)?;
        let graph = contribution.effect_graph();
        if !graph.after_opaque.is_empty() {
            return Err("2D scene refuses contribution after-opaque stage".into());
        }
        graph.validate()?;
        contribution.rebuild_2d(&SpriteContributionSurface {
            capabilities: &prepared.capabilities,
            queue: &self.queue,
        })?;
        check_stages(contribution)?;
        let graph = contribution.effect_graph();
        if !graph.after_opaque.is_empty() {
            return Err("2D scene refuses contribution after-opaque stage".into());
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
                Err(error.to_string())
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
            .sprites(frame.source, frame.presentation_alpha, target.extent)
            .map_err(FrameError::Scene)?;
        if let Some(entry) = self.contribution.as_mut() {
            entry.commit_quality_knobs();
        }
        let upload = prepared
            .upload(&self.queue, target, sprites)
            .map_err(|error| FrameError::Scene(format!("sprite view update: {error:?}")))?;
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
                history_identity: [0; 2],
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
                moving: 0,
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

    pub(super) fn begin_sprite_capture(&mut self) -> Result<PendingCapture, FrameError> {
        let source = self.world.source();
        let snapshot = self.world.publication().map_err(FrameError::World)?;
        if super::publication_source(&snapshot) != source {
            return Err(FrameError::InvalidState(
                "capture publication is not current",
            ));
        }
        drop(snapshot);
        let PreparedRenderer::TwoD(prepared) = &mut self.prepared else {
            unreachable!()
        };
        let mut target =
            SpriteTarget::prepare(prepared, self.render_extent, CaptureSource::Sdr, None)
                .map_err(|error| FrameError::Gpu(format!("capture sprite target: {error:?}")))?;
        let frame = self
            .source
            .sprites(source, 1.0, target.extent)
            .map_err(FrameError::Scene)?;
        let upload = prepared
            .upload(&self.queue, &mut target, frame)
            .map_err(|error| FrameError::Gpu(format!("capture sprite update: {error:?}")))?;
        self.dropped_capture_rows = self
            .dropped_capture_rows
            .saturating_add(u64::from(upload.dropped));
        let mut pixels = target.readback(&self.device);
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
                Some(&mut pixels),
                self.contribution.as_deref_mut(),
                time,
            )
            .map_err(|error| FrameError::Gpu(format!("capture sprite encode: {error:?}")))?;
        let submitted = self.queue.submit([encoder.finish()]);
        if let Some(contribution) = self.contribution.as_mut() {
            contribution.submitted(false);
        }
        Ok(PendingCapture {
            pixels,
            submitted,
            owner: Arc::clone(&self.capture_owner),
            generation: self.generation,
            tick: source.tick,
            extent: self.render_extent,
        })
    }
}

fn check_stages(contribution: &dyn SceneContribution) -> Result<(), String> {
    if let Some(stage) = contribution.draw_stages().first() {
        return Err(format!(
            "2D scene refuses contribution {} stage",
            match stage {
                crate::contribution::SceneDrawStage::Shadow => "shadow",
                crate::contribution::SceneDrawStage::Opaque => "opaque",
            }
        ));
    }
    if !contribution.secondary_view_slots().is_empty() {
        return Err("2D scene refuses contribution secondary views".into());
    }
    if !contribution.effect_graph().opaque_reads.is_empty() {
        return Err("2D scene refuses contribution opaque reads".into());
    }
    Ok(())
}
