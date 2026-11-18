//! Per-view cascade uniforms reuse the scene's caster pipelines and global
//! caster bounds. The distant height map keeps the main scene's fit.
use super::{
    CASTER_VIEW_RANGE, Cascade, FitInputs, FittedShadow, RenderError, SAMPLE_BYTES, ShadowBounds,
    ShadowPrepared, SyntheticView,
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
            fitted: FittedShadow {
                matrices: self.fitted.matrices,
                samples: self.fitted.samples,
                partitioned: self.fitted.partitioned,
            },
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
            projection: camera.projection(),
            profile: Some(profile),
            ..world.inputs
        };
        let mut fitted = inputs.fit(bounds)?;
        let world_fit = world.inputs.fit(bounds)?;
        fitted.matrices[2] = world_fit.matrices[2];
        fitted.samples[192..272].copy_from_slice(&world_fit.samples[192..272]);
        for (index, cascade) in self.cascades.iter().enumerate() {
            self.queue
                .write_buffer(&cascade.buffer, 0, &fitted.caster_view(index));
        }
        self.queue
            .write_buffer(&self.sample_buffer, 0, &fitted.samples);
        Ok(fitted.matrices)
    }
}
