//! Filterable forward fields. The CPU/query buffers remain independent oracle products.
use thrum_render::binding;
use thrum_render::{
    contribution::{EffectAccess, EffectCadence, EffectGraph, EffectPass},
    graph::{AccessKind, ResourceDescriptor, ResourceScope},
    runtime::DeviceSite,
};

pub(super) const SHADER: &str = concat!(
    include_str!("field.wgsl"),
    "\n",
    include_str!("filtered.wgsl")
);
pub(super) const PACK_PASS: &str = "sea filtered pack";
pub(super) const PLANES: u32 = 5;
pub(super) const CASCADE_PLANES: u32 = super::spec::CASCADES * PLANES;
pub(super) const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const USAGE: wgpu::TextureUsages = wgpu::TextureUsages::STORAGE_BINDING
    .union(wgpu::TextureUsages::TEXTURE_BINDING)
    .union(wgpu::TextureUsages::COPY_SRC);
struct Pass {
    pipeline: wgpu::ComputePipeline,
    group: wgpu::BindGroup,
    dispatch: [u32; 3],
}
pub(super) struct Filtered {
    pub(super) textures: [wgpu::Texture; 2],
    pub(super) views: [wgpu::TextureView; 2],
    pub(super) surface_views: Option<[wgpu::TextureView; 2]>,
    cascade_mip: u32,
    passes: Vec<Pass>,
}
impl Filtered {
    pub(super) fn new(
        device: &DeviceSite,
        cascades: &wgpu::Buffer,
        long: &wgpu::Buffer,
        rain: bool,
        flow: bool,
    ) -> Self {
        let cascade_mip = if rain {
            u32::from(super::rain::WIND_BASE_MIP)
        } else {
            0
        };
        let textures = std::array::from_fn(|i| {
            let (width, layers) = match i {
                0 => (
                    256_u32 << cascade_mip,
                    super::flow::first_layer(rain) + if flow { super::flow::LAYERS } else { 0 },
                ),
                _ => (128_u32, 8 * PLANES),
            };
            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("sea filtered fields"),
                size: wgpu::Extent3d {
                    width,
                    height: width,
                    depth_or_array_layers: layers,
                },
                mip_level_count: width.ilog2() + 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: FORMAT,
                usage: USAGE,
                view_formats: &[],
            })
        });
        let wind_view = |mip_level_count| {
            textures[0].create_view(&wgpu::TextureViewDescriptor {
                base_mip_level: cascade_mip,
                mip_level_count,
                array_layer_count: Some(CASCADE_PLANES),
                ..Default::default()
            })
        };
        let views = [
            wind_view(None),
            textures[1].create_view(&wgpu::TextureViewDescriptor::default()),
        ];
        let surface_views = (rain || flow).then(|| {
            [
                textures[0].create_view(&wgpu::TextureViewDescriptor::default()),
                views[1].clone(),
            ]
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sea filtered fields"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let storage = |binding| {
            binding::storage_texture(
                binding,
                wgpu::ShaderStages::COMPUTE,
                wgpu::StorageTextureAccess::WriteOnly,
                FORMAT,
                wgpu::TextureViewDimension::D2Array,
            )
        };
        let buffer = |binding| binding::storage(binding, wgpu::ShaderStages::COMPUTE, true, 0);
        let pack_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sea filtered pack"),
            entries: &[buffer(0), buffer(1), storage(2), storage(3)],
        });
        let reduce_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sea filtered reduce"),
            entries: &[
                binding::texture(
                    4,
                    wgpu::ShaderStages::COMPUTE,
                    wgpu::TextureSampleType::Float { filterable: false },
                    wgpu::TextureViewDimension::D2Array,
                ),
                storage(5),
            ],
        });
        let pipeline = |layout: &wgpu::BindGroupLayout, entry: &str| {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(entry),
                bind_group_layouts: &[Some(layout)],
                immediate_size: 0,
            });
            device.compute(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&layout),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            })
        };
        let pack_pipeline = pipeline(&pack_layout, "pack");
        let reduce_pipeline = pipeline(&reduce_layout, "reduce_texture");
        let mip_view = |i: usize, mip| {
            textures[i].create_view(&wgpu::TextureViewDescriptor {
                base_mip_level: mip + if i == 0 { cascade_mip } else { 0 },
                mip_level_count: Some(1),
                array_layer_count: (i == 0).then_some(CASCADE_PLANES),
                ..Default::default()
            })
        };
        let base = [mip_view(0, 0), mip_view(1, 0)];
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sea filtered pack"),
            layout: &pack_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: cascades.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: long.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&base[0]),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&base[1]),
                },
            ],
        });
        let mut passes = vec![Pass {
            pipeline: pack_pipeline,
            group,
            dispatch: [32, 32, 40],
        }];
        for (i, texture) in textures.iter().enumerate() {
            let first = if i == 0 { cascade_mip } else { 0 };
            for mip in 1..texture.mip_level_count() - first {
                let parent = mip_view(i, mip - 1);
                let child = mip_view(i, mip);
                let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("sea filtered reduce"),
                    layout: &reduce_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: wgpu::BindingResource::TextureView(&parent),
                        },
                        wgpu::BindGroupEntry {
                            binding: 5,
                            resource: wgpu::BindingResource::TextureView(&child),
                        },
                    ],
                });
                let width = texture.width() >> (mip + first);
                passes.push(Pass {
                    pipeline: reduce_pipeline.clone(),
                    group,
                    dispatch: [
                        width.div_ceil(8),
                        width.div_ceil(8),
                        if i == 0 {
                            CASCADE_PLANES
                        } else {
                            texture.depth_or_array_layers()
                        },
                    ],
                });
            }
        }
        Self {
            textures,
            views,
            surface_views,
            cascade_mip,
            passes,
        }
    }
    pub(super) fn pass_count(&self) -> usize {
        self.passes.len()
    }

    pub(super) fn encode(&self, index: usize, pass: &mut wgpu::ComputePass<'_>) {
        let p = &self.passes[index];
        pass.set_pipeline(&p.pipeline);
        pass.set_bind_group(0, &p.group, &[]);
        let [x, y, z] = p.dispatch;
        pass.dispatch_workgroups(x, y, z);
    }
    pub(super) fn append_graph(&self, graph: &mut EffectGraph) {
        let offset = graph.resources.len();
        for (i, texture) in self.textures.iter().enumerate() {
            let scope = if i == 0
                && (self.cascade_mip != 0
                    || self.textures[0].depth_or_array_layers() > CASCADE_PLANES)
            {
                ResourceScope::Persistent {
                    owner: 0x7261_696e,
                    epoch: 0,
                    initialized: true,
                }
            } else {
                ResourceScope::Transient
            };
            graph
                .resources
                .push((ResourceDescriptor::of_texture(texture), scope));
        }
        let buffer_access = |buffer| {
            let resource = super::spec::resources()
                .iter()
                .position(|r| r.key == buffer)
                .expect("field buffer");
            let ResourceDescriptor::Buffer { bytes, .. } = graph.resources[resource].0 else {
                unreachable!()
            };
            EffectAccess {
                resource,
                range: 0..bytes,
                kind: AccessKind::StorageRead,
                view: None,
                history: None,
            }
        };
        let mut accesses = vec![
            buffer_access(super::spec::BufferKey::Cascades),
            buffer_access(super::spec::BufferKey::LongField),
        ];
        accesses.extend((0..2).map(|i| self.access(offset, i, 0, AccessKind::StorageWrite)));
        let pass = |name: String, accesses| EffectPass {
            name,
            cadence: EffectCadence::EveryFrame,
            async_eligible: false,
            accesses,
        };
        graph.passes.push(pass(PACK_PASS.into(), accesses));
        for (i, texture) in self.textures.iter().enumerate() {
            let first = if i == 0 { self.cascade_mip } else { 0 };
            for mip in 1..texture.mip_level_count() - first {
                graph.passes.push(pass(
                    format!("sea filtered {i} mip {mip}"),
                    vec![
                        self.access(offset, i, mip - 1, AccessKind::SampledRead),
                        self.access(offset, i, mip, AccessKind::StorageWrite),
                    ],
                ));
            }
        }
    }
    fn access(&self, offset: usize, i: usize, mip: u32, kind: AccessKind) -> EffectAccess {
        let layers = u64::from(self.textures[i].depth_or_array_layers());
        let first = if i == 0 { self.cascade_mip } else { 0 };
        let offset_layer = u64::from(mip + first) * layers;
        let count = if i == 0 {
            u64::from(CASCADE_PLANES)
        } else {
            layers
        };
        EffectAccess {
            resource: offset + i,
            range: offset_layer..offset_layer + count,
            kind,
            view: Some(wgpu::TextureViewDimension::D2Array),
            history: None,
        }
    }
    pub(super) fn surface_accesses(&self, offset: usize) -> Vec<EffectAccess> {
        if self.cascade_mip != 0 || self.textures[0].depth_or_array_layers() > CASCADE_PLANES {
            let mut accesses: Vec<_> = (0..self.textures[0].mip_level_count() - self.cascade_mip)
                .map(|mip| self.access(offset, 0, mip, AccessKind::SampledRead))
                .collect();
            let mut long = self.access(offset, 1, 0, AccessKind::SampledRead);
            long.range.end *= u64::from(self.textures[1].mip_level_count());
            accesses.push(long);
            return accesses;
        }
        (0..2)
            .map(|i| {
                let mut a = self.access(offset, i, 0, AccessKind::SampledRead);
                a.range.end *= u64::from(self.textures[i].mip_level_count());
                a
            })
            .collect()
    }
    pub(super) fn resource_bytes(&self) -> Vec<(String, u64)> {
        self.textures
            .iter()
            .enumerate()
            .map(|(i, t)| {
                (
                    format!("sea filtered {i}"),
                    (0..t.mip_level_count())
                        .map(|m| {
                            u64::from((t.width() >> m) * (t.height() >> m))
                                * u64::from(t.depth_or_array_layers())
                                * 8
                        })
                        .sum(),
                )
            })
            .collect()
    }
}
