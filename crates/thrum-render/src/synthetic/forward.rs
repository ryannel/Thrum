//! Recording of the opaque forward pass: atmosphere, the camera draw and a
//! contribution, and the pipeline choice and bindings Occlusion's depth
//! prepass shares with it.

use super::mesh_arena::MeshId;
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
        self.bind_forward(
            &mut pass,
            target,
            &target.view_bind_group,
            exposure_side,
            false,
        )?;
        self.draw_camera(&mut pass, target)?;
        if let Some(contribution) = contribution {
            contribution.draw(&mut pass);
        }
        Ok(())
    }

    /// The forward attachments, cleared.
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

    /// The opaque pipeline, or with `depth_only` its depth-only twin (same
    /// vertex entry and primitive state), chosen by the same two-sided
    /// count; its bind groups with `view_group`; and indices.
    pub(super) fn bind_forward(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        target: &OffscreenTarget,
        view_group: &wgpu::BindGroup,
        exposure_side: usize,
        depth_only: bool,
    ) -> Result<(), RenderError> {
        let two_sided = usize::from(self.instance_upload.two_sided_count() != 0);
        let pipeline = if depth_only {
            &self
                .depth_prepass
                .as_ref()
                .ok_or(RenderError::GraphPassMismatch)?[two_sided]
        } else if two_sided == 0 {
            &self.pipeline
        } else {
            &self.pipeline_two_sided
        };
        pass.set_pipeline(pipeline);
        let view_offset = self.view_stride * u32::from(target.frame_revision & 1 != 0);
        // The bucket route binds each bucket's segment offset as it draws.
        let offsets = [view_offset, 0];
        let bound = if self.bucket_route.is_some() { 2 } else { 1 };
        pass.set_bind_group(0, view_group, &offsets[..bound]);
        pass.set_bind_group(1, &self.light[exposure_side], &[]);
        pass.set_bind_group(2, &self.materials, &[]);
        pass.set_bind_group(3, self.history.read_group(), &[]);
        let view = self.pending_view.as_ref().unwrap_or(&self.view_sample);
        // The orthographic diagnostic keeps world XY as clip XY while its
        // camera looks from -Z, so it draws the cube's reversed winding. Both
        // rows share the cube's vertices; only the index range differs.
        let mesh = if matches!(view.projection(), Projection::OrthographicFixture) {
            MeshId::CUBE_REVERSED
        } else {
            MeshId::CUBE
        };
        let row = self
            .mesh_arena
            .row(mesh)
            .ok_or(RenderError::GraphPassMismatch)?;
        let buffers = self
            .mesh_arena
            .buffers()
            .ok_or(RenderError::GraphPassMismatch)?;
        pass.set_index_buffer(
            buffers.indices.slice(row.index_bytes()),
            wgpu::IndexFormat::Uint32,
        );
        Ok(())
    }
}
