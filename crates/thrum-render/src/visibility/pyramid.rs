//! The reverse-Z min depth pyramid for two-phase occlusion: pipelines shared
//! across targets, and a per-target texture with one storage view per mip.
//! The frame-graph pass (`camera depth pyramid`) is declared with the M2 seam.

/// Format of every pyramid mip: one portable, storage-writable float.
pub(crate) const PYRAMID_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R32Float;
/// Pass name registered for timing once the M2 graph lands.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "registered with the M2 graph seam")
)]
pub(crate) const PASS: &str = "camera depth pyramid";
const GROUP: u32 = 8;

/// The occlusion test function, for inclusion by the late cull shader.
#[cfg_attr(not(test), expect(dead_code, reason = "included by the M2 late cull"))]
pub(crate) const OCCLUSION_WGSL: &str = include_str!("occlusion.wgsl");

/// Compute pipelines and layouts; independent of target size.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "prepared with the M2 graph seam")
)]
pub(crate) struct PyramidPrepared {
    first: wgpu::ComputePipeline,
    down: wgpu::ComputePipeline,
    first_layout: wgpu::BindGroupLayout,
    down_layout: wgpu::BindGroupLayout,
}

/// One target's pyramid and the bind group for each mip's dispatch.
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "prepared with the M2 graph seam")
)]
pub(crate) struct PyramidTarget {
    texture: wgpu::Texture,
    dispatches: Vec<(wgpu::BindGroup, [u32; 2])>,
}

/// The largest power of two not above `n` (`n >= 1`).
#[cfg_attr(
    not(test),
    expect(dead_code, reason = "prepared with the M2 graph seam")
)]
pub(crate) fn previous_pow2(n: u32) -> u32 {
    1 << n.ilog2()
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "prepared with the M2 graph seam")
)]
impl PyramidPrepared {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let storage = wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::StorageTexture {
                access: wgpu::StorageTextureAccess::WriteOnly,
                format: PYRAMID_FORMAT,
                view_dimension: wgpu::TextureViewDimension::D2,
            },
            count: None,
        };
        let source = |sample_type| wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Texture {
                sample_type,
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let layout = |label, sample_type| {
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(label),
                entries: &[source(sample_type), storage],
            })
        };
        let first_layout = layout("depth pyramid mip 0", wgpu::TextureSampleType::Depth);
        let down_layout = layout(
            "depth pyramid down",
            wgpu::TextureSampleType::Float { filterable: false },
        );
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(PASS),
            source: wgpu::ShaderSource::Wgsl(include_str!("pyramid.wgsl").into()),
        });
        let pipeline = |layout: &wgpu::BindGroupLayout, entry: &str| {
            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(PASS),
                bind_group_layouts: &[Some(layout)],
                immediate_size: 0,
            });
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(PASS),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            })
        };
        Self {
            first: pipeline(&first_layout, "pyramid_mip0"),
            down: pipeline(&down_layout, "pyramid_down"),
            first_layout,
            down_layout,
        }
    }

    /// Allocate the pyramid for a depth target of `extent`, reading `depth`.
    pub(crate) fn target(
        &self,
        device: &wgpu::Device,
        depth: &wgpu::TextureView,
        extent: [u32; 2],
    ) -> PyramidTarget {
        let size = [previous_pow2(extent[0]), previous_pow2(extent[1])];
        let levels = size[0].max(size[1]).ilog2() + 1;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(PASS),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: levels,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: PYRAMID_FORMAT,
            // Copy source serves oracle readback and diagnostics.
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let mip = |level| {
            texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some(PASS),
                base_mip_level: level,
                mip_level_count: Some(1),
                ..Default::default()
            })
        };
        let group = |layout, source: &wgpu::TextureView, level| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(PASS),
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(source),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&mip(level)),
                    },
                ],
            })
        };
        let extent_at = |level: u32| [(size[0] >> level).max(1), (size[1] >> level).max(1)];
        let mut dispatches = vec![(group(&self.first_layout, depth, 0), extent_at(0))];
        for level in 1..levels {
            let source = mip(level - 1);
            dispatches.push((group(&self.down_layout, &source, level), extent_at(level)));
        }
        PyramidTarget {
            texture,
            dispatches,
        }
    }

    /// Record every mip in order into an open compute pass. Each dispatch is
    /// its own usage scope, so mip L is written before mip L+1 reads it.
    pub(crate) fn encode(&self, pass: &mut wgpu::ComputePass<'_>, target: &PyramidTarget) {
        for (level, (group, [width, height])) in target.dispatches.iter().enumerate() {
            pass.set_pipeline(if level == 0 { &self.first } else { &self.down });
            pass.set_bind_group(0, group, &[]);
            pass.dispatch_workgroups(width.div_ceil(GROUP), height.div_ceil(GROUP), 1);
        }
    }
}

#[cfg_attr(
    not(test),
    expect(dead_code, reason = "prepared with the M2 graph seam")
)]
impl PyramidTarget {
    /// The whole pyramid, every mip, for the late cull's sampled binding.
    pub(crate) fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }
}

#[cfg(test)]
mod tests;
