//! Recording of the opaque forward passes: the early pass with atmosphere,
//! the camera draw and a contribution, and the shared attachments, pipeline
//! and bindings the occlusion late pass reuses.

use super::{
    CompiledPass, GpuTimings, GraphColorAttachment, GraphDepthAttachment, OffscreenTarget,
    PreparedSynthetic, Projection, RenderError, SceneContribution,
};

impl PreparedSynthetic {
    pub(super) fn encode_opaque(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &OffscreenTarget,
        compiled: &CompiledPass,
        timings: Option<&mut GpuTimings>,
        contribution: Option<&dyn SceneContribution>,
        exposure_side: usize,
    ) -> Result<(), RenderError> {
        let mut pass = self.begin_forward(encoder, target, compiled, timings)?;
        if let Some(atmosphere) = self.atmosphere.as_ref() {
            atmosphere.draw(&mut pass, exposure_side);
        }
        self.bind_forward(&mut pass, target, &target.view_bind_group, exposure_side);
        self.draw_camera(&mut pass, target);
        if let Some(contribution) = contribution {
            contribution.draw(&mut pass);
        }
        Ok(())
    }

    /// The forward attachments; the graph chooses clear or load per pass.
    pub(super) fn begin_forward<'e>(
        &self,
        encoder: &'e mut wgpu::CommandEncoder,
        target: &OffscreenTarget,
        compiled: &CompiledPass,
        timings: Option<&mut GpuTimings>,
    ) -> Result<wgpu::RenderPass<'e>, RenderError> {
        compiled
            .begin_render(
                encoder,
                [
                    Some(GraphColorAttachment {
                        resource: &target.ids.scene,
                        range: 0..1,
                        view: &target.scene_view,
                        clear: self.background,
                    }),
                    Some(GraphColorAttachment {
                        resource: &target.ids.motion,
                        range: 0..1,
                        view: &target.motion_view,
                        clear: wgpu::Color::TRANSPARENT,
                    }),
                ],
                Some(GraphDepthAttachment {
                    resource: &target.ids.depth,
                    range: 0..1,
                    view: &target.depth,
                    clear: 0.0,
                }),
                self.key.device_generation,
                timings,
            )?
            .ok_or(RenderError::GraphPassMismatch)
    }

    /// The opaque pipeline, its bind groups with `view_group`, and indices.
    pub(super) fn bind_forward(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        target: &OffscreenTarget,
        view_group: &wgpu::BindGroup,
        exposure_side: usize,
    ) {
        pass.set_pipeline(if self.instance_upload.two_sided_count() == 0 {
            &self.pipeline
        } else {
            &self.pipeline_two_sided
        });
        let view_offset = self.view_stride * u32::from(target.frame_revision & 1 != 0);
        pass.set_bind_group(0, view_group, &[view_offset]);
        pass.set_bind_group(1, &self.light[exposure_side], &[]);
        pass.set_bind_group(2, &self.materials, &[]);
        pass.set_bind_group(3, self.history.read_group(), &[]);
        let view = self.pending_view.as_ref().unwrap_or(&self.view_sample);
        let indices = if matches!(view.projection(), Projection::OrthographicFixture) {
            &self.cube_reversed_indices
        } else {
            &self.cube_indices
        };
        pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint16);
    }
}
