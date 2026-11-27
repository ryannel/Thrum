//! Final caustic irradiance at each depth and progressively larger footprints.

use super::{RUNGS, SIDE};
use thrum_render::{
    binding,
    contribution::{EffectAccess, EffectCadence, EffectGraph, EffectPass},
    graph::{AccessKind, ResourceDescriptor, ResourceScope},
    runtime::DeviceSite,
};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
pub(super) const MIPS: u32 = SIDE.ilog2() + 1;
const WORKGROUP: u32 = 8;

pub(super) struct Filter {
    pub(super) texture: wgpu::Texture,
    blur: wgpu::ComputePipeline,
    reduce: wgpu::ComputePipeline,
    bindings: Vec<wgpu::BindGroup>,
}

impl Filter {
    pub(super) fn new(
        device: &DeviceSite,
        drawn: &wgpu::Texture,
        frame: &wgpu::BindGroupLayout,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("filtered water caustics"),
            size: drawn.size(),
            mip_level_count: MIPS,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let stages = wgpu::ShaderStages::COMPUTE;
        let dimension = wgpu::TextureViewDimension::D2Array;
        let images = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("water caustic filter images"),
            entries: &[
                binding::texture(
                    0,
                    stages,
                    wgpu::TextureSampleType::Float { filterable: true },
                    dimension,
                ),
                binding::sampler(1, stages, wgpu::SamplerBindingType::Filtering),
                binding::storage_texture(
                    2,
                    stages,
                    wgpu::StorageTextureAccess::WriteOnly,
                    FORMAT,
                    dimension,
                ),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("water caustic filter"),
            bind_group_layouts: &[Some(&images), Some(frame)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("water caustic filter"),
            source: wgpu::ShaderSource::Wgsl(
                concat!(
                    include_str!("rungs.wgsl"),
                    include_str!("frame.wgsl"),
                    include_str!("filter.wgsl")
                )
                .into(),
            ),
        });
        let pipeline = |entry| {
            device.compute(&wgpu::ComputePipelineDescriptor {
                label: Some("water caustic filter"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            })
        };
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("bounded caustic filter"),
            min_filter: wgpu::FilterMode::Linear,
            mag_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let view = |image: &wgpu::Texture, mip| {
            image.create_view(&wgpu::TextureViewDescriptor {
                dimension: Some(dimension),
                base_mip_level: mip,
                mip_level_count: Some(1),
                ..Default::default()
            })
        };
        let bindings = (0..MIPS)
            .map(|mip| {
                let input = if mip == 0 {
                    view(drawn, 0)
                } else {
                    view(&texture, mip - 1)
                };
                let output = view(&texture, mip);
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("water caustic filter images"),
                    layout: &images,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&input),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(&output),
                        },
                    ],
                })
            })
            .collect();
        Self {
            texture,
            blur: pipeline("blur"),
            reduce: pipeline("reduce"),
            bindings,
        }
    }

    pub(super) fn declare(&self, graph: &mut EffectGraph, drawn: usize) {
        let resource = graph.resources.len();
        graph.resources.push((
            ResourceDescriptor::of_texture(&self.texture),
            ResourceScope::Persistent {
                owner: 0x6361_7566,
                epoch: 0,
                initialized: false,
            },
        ));
        let access = |resource, mip, kind| EffectAccess {
            resource,
            range: u64::from(mip * RUNGS)..u64::from((mip + 1) * RUNGS),
            kind,
            view: Some(wgpu::TextureViewDimension::D2Array),
            history: None,
        };
        for mip in 0..MIPS {
            let input = if mip == 0 {
                (drawn, 0)
            } else {
                (resource, mip - 1)
            };
            graph.passes.push(EffectPass {
                name: if mip == 0 {
                    "blur water caustics".into()
                } else {
                    format!("water caustic mip {mip}")
                },
                cadence: EffectCadence::Skippable,
                async_eligible: false,
                accesses: vec![
                    access(input.0, input.1, AccessKind::SampledRead),
                    access(resource, mip, AccessKind::StorageWrite),
                ],
            });
        }
    }

    pub(super) fn encode(
        &self,
        mip: u32,
        pass: &mut wgpu::ComputePass<'_>,
        frame: &wgpu::BindGroup,
    ) {
        pass.set_pipeline(if mip == 0 { &self.blur } else { &self.reduce });
        pass.set_bind_group(0, &self.bindings[mip as usize], &[]);
        pass.set_bind_group(1, frame, &[]);
        let groups = (SIDE >> mip).div_ceil(WORKGROUP);
        pass.dispatch_workgroups(groups, groups, RUNGS);
    }

    pub(super) fn bytes() -> u64 {
        (0..MIPS)
            .map(|mip| u64::from(SIDE >> mip).pow(2))
            .sum::<u64>()
            * u64::from(RUNGS)
            * 8
    }
}
