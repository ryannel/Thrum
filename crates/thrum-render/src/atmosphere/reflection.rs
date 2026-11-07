//! Disk-free sky reflection products. All passes record into the existing
//! frame graph; generation/profile validity is owned by AtmospherePrepared.

use crate::synthetic::RenderError;

pub(super) const CUBE_SIZE: u32 = 64;
pub(super) const MIP_COUNT: u32 = 7;
pub(super) const DFG_SIZE: u32 = 65;
pub(super) const PREFILTER_SAMPLES: u32 = 128;
pub(super) const DFG_SAMPLES: u32 = 1024;
pub(super) const SAMPLE_BYTES: u64 = MIP_COUNT as u64 * PREFILTER_SAMPLES as u64 * 16;

pub(super) struct ReflectionPrepared {
    pub(super) cube: wgpu::Texture,
    pub(super) dfg: wgpu::Texture,
    cube_view: wgpu::TextureView,
    dfg_view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    cube_pipeline: wgpu::ComputePipeline,
    cube_groups: [wgpu::BindGroup; MIP_COUNT as usize],
    dfg_pipeline: wgpu::ComputePipeline,
    dfg_group: wgpu::BindGroup,
    samples_pipeline: wgpu::ComputePipeline,
    samples_group: wgpu::BindGroup,
}

impl ReflectionPrepared {
    pub(super) fn new(
        device: &wgpu::Device,
        uniform: &wgpu::Buffer,
        sky: &wgpu::TextureView,
        sky_sampler: &wgpu::Sampler,
    ) -> Self {
        let cube = texture(device, "atmosphere specular cube", CUBE_SIZE, 6, MIP_COUNT);
        let dfg = texture(device, "atmosphere DFG", DFG_SIZE, 1, 1);
        let samples = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("atmosphere prepared GGX samples"),
            size: SAMPLE_BYTES,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let cube_view = cube.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(wgpu::TextureViewDimension::Cube),
            ..Default::default()
        });
        let dfg_view = dfg.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("atmosphere specular trilinear"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            lod_max_clamp: (MIP_COUNT - 1) as f32,
            ..Default::default()
        });
        let constants = format!(
            "const REFLECTION_SIZE:u32={CUBE_SIZE}u;\nconst REFLECTION_MIPS:u32={MIP_COUNT}u;\nconst PREFILTER_SAMPLES:u32={PREFILTER_SAMPLES}u;\nconst DFG_SAMPLES:u32={DFG_SAMPLES}u;\n"
        );
        let cube_pipeline = pipeline(
            device,
            "atmosphere sky specular prefilter",
            "reflection_main",
            format!(
                "{constants}\n{}\n{}\n{}",
                concat!(
                    include_str!("storage.wgsl"),
                    "\n",
                    concat!(
                        include_str!("geometry.wgsl"),
                        "\n",
                        include_str!("common.wgsl")
                    )
                ),
                include_str!("brdf.wgsl"),
                include_str!("reflection.wgsl")
            ),
        );
        let cube_layout = cube_pipeline.get_bind_group_layout(0);
        let cube_groups = std::array::from_fn(|mip| {
            let output = cube.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(wgpu::TextureViewDimension::D2Array),
                base_mip_level: u32::try_from(mip).expect("seven prepared mips"),
                mip_level_count: Some(1),
                ..Default::default()
            });
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("atmosphere specular mip"),
                layout: &cube_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(sky),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(sky_sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(&output),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: samples.as_entire_binding(),
                    },
                ],
            })
        });
        let dfg_pipeline = pipeline(
            device,
            "atmosphere DFG integration",
            "dfg_main",
            format!(
                "{constants}\n{}\n{}",
                include_str!("brdf.wgsl"),
                include_str!("dfg.wgsl")
            ),
        );
        let dfg_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("atmosphere DFG output"),
            layout: &dfg_pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&dfg_view),
            }],
        });
        let samples_pipeline = pipeline(
            device,
            "atmosphere GGX sample preparation",
            "prefilter_samples_main",
            format!(
                "{constants}\n{}\n{}",
                include_str!("brdf.wgsl"),
                include_str!("dfg.wgsl")
            ),
        );
        let samples_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("atmosphere GGX sample output"),
            layout: &samples_pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 1,
                resource: samples.as_entire_binding(),
            }],
        });
        Self {
            cube,
            dfg,
            cube_view,
            dfg_view,
            sampler,
            cube_pipeline,
            cube_groups,
            dfg_pipeline,
            dfg_group,
            samples_pipeline,
            samples_group,
        }
    }

    pub(super) fn mip_count(&self) -> usize {
        self.cube_groups.len()
    }

    pub(super) fn sampled(&self) -> (&wgpu::TextureView, &wgpu::TextureView, &wgpu::Sampler) {
        (&self.cube_view, &self.dfg_view, &self.sampler)
    }

    pub(super) fn encode(
        &self,
        stage: usize,
        pass: &mut wgpu::ComputePass<'_>,
    ) -> Result<(), RenderError> {
        if stage == 0 {
            pass.set_pipeline(&self.dfg_pipeline);
            pass.set_bind_group(0, &self.dfg_group, &[]);
            pass.dispatch_workgroups(DFG_SIZE.div_ceil(8), DFG_SIZE.div_ceil(8), 1);
            pass.set_pipeline(&self.samples_pipeline);
            pass.set_bind_group(0, &self.samples_group, &[]);
            pass.dispatch_workgroups((MIP_COUNT * PREFILTER_SAMPLES).div_ceil(64), 1, 1);
        } else {
            let mip = stage - 1;
            let group = self
                .cube_groups
                .get(mip)
                .ok_or(RenderError::GraphPassMismatch)?;
            pass.set_pipeline(&self.cube_pipeline);
            pass.set_bind_group(0, group, &[]);
            let width = CUBE_SIZE >> mip;
            pass.dispatch_workgroups(width.div_ceil(8), width.div_ceil(8), 6);
        }
        Ok(())
    }
}

fn texture(device: &wgpu::Device, label: &str, size: u32, layers: u32, mips: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: layers,
        },
        mip_level_count: mips,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

fn pipeline(
    device: &wgpu::Device,
    label: &str,
    entry: &str,
    source: String,
) -> wgpu::ComputePipeline {
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(label),
        layout: None,
        module: &module,
        entry_point: Some(entry),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    })
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
