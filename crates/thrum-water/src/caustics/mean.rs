use super::{RUNGS, SIDE};
use thrum_render::{
    binding,
    contribution::{EffectAccess, EffectCadence, EffectGraph, EffectPass},
    graph::{AccessKind, ResourceDescriptor, ResourceScope},
    runtime::DeviceSite,
};

const TILE: u32 = 16;
const TILE_COUNT: u32 = SIDE / TILE;
const SUM_TILES: &str = "sum caustic raster tiles";
const SUM_RUNGS: &str = "sum caustic raster rungs";

pub(super) struct Mean {
    pub(super) values: wgpu::Buffer,
    partial: wgpu::Buffer,
    group: wgpu::BindGroup,
    tiles: wgpu::ComputePipeline,
    rungs: wgpu::ComputePipeline,
}

impl Mean {
    pub(super) fn new(device: &DeviceSite, raster: &wgpu::Texture) -> Self {
        let buffer = |label, size| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            })
        };
        let partial = buffer(SUM_TILES, u64::from(RUNGS * TILE_COUNT * TILE_COUNT * 4));
        let values = buffer(SUM_RUNGS, u64::from(RUNGS * 4));
        let compute = wgpu::ShaderStages::COMPUTE;
        let dimension = wgpu::TextureViewDimension::D2Array;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(SUM_RUNGS),
            entries: &[
                binding::texture(
                    0,
                    compute,
                    wgpu::TextureSampleType::Float { filterable: true },
                    dimension,
                ),
                binding::storage(1, compute, false, partial.size()),
                binding::storage(2, compute, false, values.size()),
            ],
        });
        let view = raster.create_view(&wgpu::TextureViewDescriptor {
            dimension: Some(dimension),
            ..Default::default()
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(SUM_RUNGS),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: partial.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: values.as_entire_binding(),
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(SUM_RUNGS),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(SUM_RUNGS),
            source: wgpu::ShaderSource::Wgsl(include_str!("mean.wgsl").into()),
        });
        let pipeline = |label, entry| {
            device.compute(&wgpu::ComputePipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                module: &shader,
                entry_point: Some(entry),
                cache: None,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            })
        };
        Self {
            values,
            partial,
            group,
            tiles: pipeline(SUM_TILES, "sum_tiles"),
            rungs: pipeline(SUM_RUNGS, "sum_rungs"),
        }
    }

    pub(super) fn declare(&self, graph: &mut EffectGraph, raster: EffectAccess) -> EffectAccess {
        let mut bind = |buffer: &wgpu::Buffer, owner| {
            let resource = graph.resources.len();
            graph.resources.push((
                ResourceDescriptor::Buffer {
                    bytes: buffer.size(),
                    usage: buffer.usage(),
                },
                ResourceScope::Persistent {
                    owner,
                    epoch: 0,
                    initialized: false,
                },
            ));
            EffectAccess {
                resource,
                range: 0..buffer.size(),
                kind: AccessKind::StorageRead,
                view: None,
                history: None,
            }
        };
        let partial = bind(&self.partial, 0x6361_756d);
        let values = bind(&self.values, 0x6361_756e);
        let write = |access: &EffectAccess| EffectAccess {
            kind: AccessKind::StorageWrite,
            ..access.clone()
        };
        for (name, accesses) in [
            (SUM_TILES, vec![raster, write(&partial)]),
            (SUM_RUNGS, vec![partial, write(&values)]),
        ] {
            graph.passes.push(EffectPass {
                name: name.into(),
                cadence: EffectCadence::Skippable,
                async_eligible: false,
                accesses,
            });
        }
        values
    }

    pub(super) fn encode(&self, step: u32, pass: &mut wgpu::ComputePass<'_>, rungs: u32) {
        let groups = if step == 0 {
            pass.set_pipeline(&self.tiles);
            TILE_COUNT
        } else {
            pass.set_pipeline(&self.rungs);
            1
        };
        pass.set_bind_group(0, &self.group, &[]);
        pass.dispatch_workgroups(groups, groups, rungs);
    }

    pub(super) fn bytes(&self) -> u64 {
        self.partial.size() + self.values.size()
    }
}
