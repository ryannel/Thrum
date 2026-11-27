//! Per-view cascade uniforms reuse the scene's caster pipelines and global
//! caster bounds. The distant height map keeps the main scene's fit.
use super::{
    CASTER_VIEW_RANGE, Cascade, FitInputs, RenderError, SAMPLE_BYTES, ShadowBounds, ShadowPrepared,
    SyntheticView,
};

impl ShadowPrepared {
    pub(in crate::synthetic) fn secondary(&self, device: &crate::runtime::DeviceSite) -> Self {
        let cascades = std::array::from_fn(|_| {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("secondary Sun caster view"),
                size: CASTER_VIEW_RANGE,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let view_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("secondary Sun caster view"),
                layout: &self.view_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
            });
            Cascade { buffer, view_group }
        });
        Self {
            pipeline: self.pipeline.clone(),
            pipeline_two_sided: self.pipeline_two_sided.clone(),
            members: self.members.clone(),
            cascades,
            queue: self.queue.clone(),
            view_layout: self.view_layout.clone(),
            inputs: FitInputs { ..self.inputs },
            fitted: self.fitted.clone(),
            sample_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("secondary Sun samples"),
                size: SAMPLE_BYTES as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
        }
    }

    pub(in crate::synthetic) fn write_secondary(
        &self,
        world: &Self,
        camera: &SyntheticView,
        profile: (u32, f32),
        bounds: Option<ShadowBounds>,
    ) -> Result<[super::Mat4; super::MAPS], RenderError> {
        let bounds = bounds.filter(|bounds| world.inputs.fit(Some(*bounds)).is_ok());
        let inputs = FitInputs {
            camera: super::Vec3::from_array(camera.camera_position()),
            profile: Some(profile),
            ..world.inputs
        };
        let mut fitted = inputs.fit(bounds)?;
        let world_fit = world.inputs.fit(bounds)?;
        fitted.matrices[super::HEIGHT_LAYER] = world_fit.matrices[super::HEIGHT_LAYER];
        fitted.samples.height_view_proj = world_fit.samples.height_view_proj;
        fitted.samples.horizon = world_fit.samples.horizon;
        for (index, cascade) in self.cascades.iter().enumerate() {
            self.queue.write_buffer(
                &cascade.buffer,
                0,
                bytemuck::bytes_of(&fitted.caster_view(index)),
            );
        }
        self.queue
            .write_buffer(&self.sample_buffer, 0, bytemuck::bytes_of(&fitted.samples));
        Ok(fitted.matrices)
    }
}
