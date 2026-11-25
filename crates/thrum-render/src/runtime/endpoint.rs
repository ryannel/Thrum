//! Detached capture of one committed scene endpoint.

use super::{
    CreationPhase, ExtractReport, FrameError, FrameHost, FrameWorld, OffscreenTarget, SceneKind,
    SceneSource, publication_source,
};

impl<W: FrameWorld, V: SceneSource<W::Page>> FrameHost<W, V> {
    pub(super) fn record_endpoint(
        &mut self,
        readback: bool,
    ) -> Result<
        (
            Option<crate::synthetic::PixelReadback>,
            wgpu::SubmissionIndex,
        ),
        FrameError,
    > {
        if self.native_provider_report().is_some() {
            return Err(FrameError::InvalidState(
                "native providers use the submitted-image tap",
            ));
        }
        let creations = self.creations.clone();
        let _region = creations.region(CreationPhase::Transition, "capture");
        if self.scene_kind() == SceneKind::TwoD {
            return self.record_sprite_endpoint(readback);
        }
        let needs_extract = self.extractor.frame().source().is_none_or(|source| {
            source.epoch != self.world.source().epoch
                || source.timeline != self.world.source().timeline
                || source.tick != self.world.source().tick
        });
        let extracted = if needs_extract {
            let snapshot = self.world.publication()?;
            let source = publication_source(&snapshot);
            if source != self.world.source() {
                return Err(FrameError::InvalidState(
                    "capture publication is not current",
                ));
            }
            let extracted = self.source.extract(&mut self.extractor, &snapshot.page);
            drop(snapshot);
            if self.extractor.frame().source() != Some(source) {
                return Err(FrameError::InvalidState(
                    "capture extraction source does not match its publication",
                ));
            }
            extracted
        } else {
            ExtractReport::default()
        };
        // A capture describes the current committed endpoint, regardless of
        // the last display sample.
        let capture_alpha = 1.0;
        self.source.sample_local_lights(
            self.world.source(),
            capture_alpha,
            &mut self.scene.local_lights,
        );
        let frame_scene = self.source.frame(
            self.world.source(),
            capture_alpha,
            self.render_extent,
            self.visual_lighting(),
        )?;
        let lighting = frame_scene.lighting;
        self.history_reset_pending |= self
            .prepared
            .record_mesh_rows(&self.queue)
            .map_err(|error| FrameError::gpu("capture mesh row update", error))?;
        self.scene.view = self
            .view_with_history_reset(&frame_scene.view)
            .without_jitter();
        if self.prepared.reconstruction() != crate::synthetic::Reconstruction::Off {
            // This detached endpoint oracle owns no submitted temporal sample.
            // The real T2 tap instead captures the ordinary reconstructed frame.
            self.scene.view = self
                .scene
                .view
                .with_reconstruction(false)
                .with_history_reset(true);
        }
        let applied = self
            .scene
            .apply_render_columns(self.extractor.frame(), &mut self.changed_slots);
        self.restore_upload_slots()?;
        let uploaded = match self.prepared.upload_instances(
            &self.queue,
            &self.scene.instances,
            &self.changed_slots,
        ) {
            Ok(uploaded) => uploaded,
            Err(error) => {
                self.retry_upload();
                return Err(FrameError::gpu("capture instance update", error));
            }
        };
        self.upload_reset_pending = false;
        let dropped = u64::try_from(
            extracted.dropped_capacity
                + extracted.dropped_duplicate
                + extracted.dropped_invalid
                + applied.dropped_invalid
                + uploaded.dropped_invalid,
        )
        .unwrap_or(u64::MAX);
        self.dropped_capture_rows = self.dropped_capture_rows.saturating_add(dropped);
        let mut target = OffscreenTarget::new(
            &self.device,
            &self.prepared,
            self.render_extent[0],
            self.render_extent[1],
        )
        .map_err(|error| FrameError::gpu("capture target", error))?;
        if let Err(error) = self.prepared.update_frame(
            &self.queue,
            &mut target,
            &self.scene,
            lighting,
            capture_alpha,
        ) {
            self.retry_upload();
            return Err(FrameError::gpu("capture view update", error));
        }
        let source = self.world.source();
        self.prepared
            .set_effect_time(crate::contribution::EffectTime {
                epoch: source.epoch,
                timeline: source.timeline,
                tick: source.tick,
                tick_duration: self.world.tick_duration(),
                alpha: capture_alpha,
                display_phase: 1.0,
                capture: true,
            });
        self.update_interface(Some(self.render_extent), None)?;
        let mut pixels = readback.then(|| target.readback(&self.device));
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        self.prepared
            .encode_contributed(
                &mut encoder,
                self.generation,
                &target,
                None,
                pixels.as_mut(),
                self.contribution.as_deref_mut(),
                None,
                &mut Vec::new(),
                self.interface.as_deref(),
            )
            .map_err(|error| FrameError::gpu("capture render/readback", error))?;
        let submitted = self.queue.submit([encoder.finish()]);
        self.prepared.submit_mesh_rows();
        if let Some(contribution) = self.contribution.as_mut() {
            contribution.submitted(false);
        }
        Ok((pixels, submitted))
    }
}
