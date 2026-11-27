//! Immutable flow spectra use the wind transform's pipelines. The
//! skippable passes retain only their own field and layers of the shared array.
use std::{collections::BTreeMap, f64::consts::TAU};

use bytemuck::Zeroable;
use thrum_render::{
    binding,
    contribution::{EffectAccess, EffectCadence, EffectGraph, EffectPass},
    graph::{AccessKind, ResourceDescriptor, ResourceScope},
    runtime::DeviceSite,
};
use thrum_sea_model::LOOP_SECONDS;
use thrum_water_model::FlowBands;

use crate::{
    PACKED_CHANNELS, PIXELS, TRANSFORM_SIZE,
    filtered::{CASCADE_PLANES, FORMAT, PLANES},
    prepare::PreparedPass,
    records::{Bin, Field, Params},
    spec::{self, BufferKey, PassKey, PassSpec},
    upload::{lattice_mode, loop_fraction},
};

const BANDS: u32 = 2;
pub(super) const LAYERS: u32 = BANDS * PLANES;
const MIPS: u32 = TRANSFORM_SIZE.ilog2() + 1;
const PASS: &str = "river flow bands";

pub(super) const fn first_layer(rain: bool) -> u32 {
    CASCADE_PLANES + if rain { crate::rain::LAYERS } else { 0 }
}

struct Dispatch {
    pipeline: wgpu::ComputePipeline,
    group: wgpu::BindGroup,
    counts: [u32; 3],
}

pub(super) struct Flow {
    bindings: BTreeMap<BufferKey, wgpu::Buffer>,
    bytes: Vec<(String, u64)>,
    transforms: Vec<Dispatch>,
    pyramid: Vec<Dispatch>,
    pub(super) view: wgpu::TextureView,
    pub(super) first: u32,
    layers: u32,
    mip: u32,
    params: Params,
}

impl Flow {
    pub(super) fn new(
        device: &DeviceSite,
        queue: &wgpu::Queue,
        bands: &FlowBands,
        rain: bool,
        texture: &wgpu::Texture,
        wind: &BTreeMap<BufferKey, wgpu::Buffer>,
        pipelines: &BTreeMap<PassKey, PreparedPass>,
        specs: &[PassSpec],
    ) -> Self {
        let profile = bands.profile();
        let scratch_bytes =
            u64::from(BANDS * PACKED_CHANNELS * PIXELS) * size_of::<[f32; 2]>() as u64;
        let entries = [
            (
                BufferKey::Params,
                "river flow time",
                size_of::<Params>() as u64,
                wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            ),
            (
                BufferKey::Modes,
                "river flow spectrum",
                u64::from(BANDS * PIXELS) * size_of::<Bin>() as u64,
                wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            ),
            (
                BufferKey::Cascades,
                "river flow lattice",
                u64::from(BANDS * PIXELS) * size_of::<Field>() as u64,
                wgpu::BufferUsages::STORAGE,
            ),
            (
                BufferKey::TransformA,
                "river flow transform A",
                scratch_bytes,
                wgpu::BufferUsages::STORAGE,
            ),
            (
                BufferKey::TransformB,
                "river flow transform B",
                scratch_bytes,
                wgpu::BufferUsages::STORAGE,
            ),
            (
                BufferKey::TransformFinal,
                "river flow final transform",
                scratch_bytes,
                wgpu::BufferUsages::STORAGE,
            ),
        ];
        let bytes = entries
            .iter()
            .map(|&(_, name, size, _)| (name.into(), size))
            .collect();
        let bindings: BTreeMap<_, _> = entries
            .into_iter()
            .map(|(key, name, size, usage)| {
                (
                    key,
                    device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some(name),
                        size,
                        usage,
                        mapped_at_creation: false,
                    }),
                )
            })
            .collect();
        let mut modes = vec![Bin::zeroed(); (BANDS * PIXELS) as usize];
        let mut params = Params::zeroed();
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "validated modes fit the GPU lattice and loop clock"
        )]
        for (band, length) in profile.patches.into_iter().enumerate() {
            params.lengths[band] = length as f32;
            for (component, omega) in bands.components(band) {
                let cycles = (omega * LOOP_SECONDS / TAU).round() as u32;
                let (index, mode) = lattice_mode(component, length, cycles);
                modes[band * PIXELS as usize + index] = mode;
            }
        }
        queue.write_buffer(
            &bindings[&BufferKey::Modes],
            0,
            bytemuck::cast_slice(&modes),
        );

        let transforms = specs
            .iter()
            .filter(|s| s.key != PassKey::Long)
            .map(|s| {
                let pipeline = pipelines[&s.key].pipeline.clone();
                let entries: Vec<_> = s
                    .bindings
                    .iter()
                    .map(|b| wgpu::BindGroupEntry {
                        binding: b.slot,
                        resource: bindings
                            .get(&b.buffer)
                            .unwrap_or_else(|| &wind[&b.buffer])
                            .as_entire_binding(),
                    })
                    .collect();
                let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(PASS),
                    layout: &pipeline.get_bind_group_layout(0),
                    entries: &entries,
                });
                let counts = match s.key {
                    PassKey::Rows | PassKey::Columns => {
                        [TRANSFORM_SIZE, BANDS * PACKED_CHANNELS, 1]
                    }
                    _ => [(BANDS * PIXELS).div_ceil(64), 1, 1],
                };
                Dispatch {
                    pipeline,
                    group,
                    counts,
                }
            })
            .collect();
        let first = first_layer(rain);
        let mip = if rain {
            u32::from(crate::rain::WIND_BASE_MIP)
        } else {
            0
        };
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            base_mip_level: mip,
            ..Default::default()
        });
        let pyramid = pyramid(device, texture, &bindings[&BufferKey::Cascades], first, mip);
        Self {
            bindings,
            bytes,
            transforms,
            pyramid,
            view,
            first,
            layers: texture.depth_or_array_layers(),
            mip,
            params,
        }
    }

    pub(super) fn upload(&mut self, queue: &wgpu::Queue, time: f64) {
        self.params.time[0] = loop_fraction(time);
        queue.write_buffer(
            &self.bindings[&BufferKey::Params],
            0,
            bytemuck::bytes_of(&self.params),
        );
    }

    pub(super) fn encode(&self, index: usize, pass: &mut wgpu::ComputePass<'_>) {
        let dispatch = if index < self.transforms.len() {
            &self.transforms[index]
        } else {
            &self.pyramid[index - self.transforms.len()]
        };
        pass.set_pipeline(&dispatch.pipeline);
        pass.set_bind_group(0, &dispatch.group, &[]);
        let [x, y, z] = dispatch.counts;
        pass.dispatch_workgroups(x, y, z);
    }

    pub(super) fn bytes(&self) -> impl Iterator<Item = (String, u64)> + '_ {
        self.bytes.iter().cloned()
    }

    pub(super) fn reads(&self, texture: usize) -> impl Iterator<Item = EffectAccess> + '_ {
        (0..MIPS).map(move |mip| self.access(texture, mip, AccessKind::SampledRead))
    }

    fn access(&self, resource: usize, mip: u32, kind: AccessKind) -> EffectAccess {
        let start = u64::from((mip + self.mip) * self.layers + self.first);
        EffectAccess {
            resource,
            range: start..start + u64::from(LAYERS),
            kind,
            view: Some(wgpu::TextureViewDimension::D2Array),
            history: None,
        }
    }

    pub(super) fn append_graph(&self, graph: &mut EffectGraph, specs: &[PassSpec], texture: usize) {
        let persistent = |key: BufferKey| ResourceScope::Persistent {
            owner: 0x666c_6f00 + key as u64,
            epoch: 0,
            initialized: true,
        };
        let mut ids: BTreeMap<_, _> = spec::resources()
            .into_iter()
            .enumerate()
            .map(|(index, resource)| (resource.key, index))
            .collect();
        for (key, buffer) in &self.bindings {
            ids.insert(*key, graph.resources.len());
            graph.resources.push((
                ResourceDescriptor::Buffer {
                    bytes: buffer.size(),
                    usage: buffer.usage(),
                },
                if matches!(*key, BufferKey::Params | BufferKey::Modes) {
                    ResourceScope::Imported { initialized: true }
                } else {
                    persistent(*key)
                },
            ));
        }
        let buffer_access = |key, kind| {
            let resource = ids[&key];
            let ResourceDescriptor::Buffer { bytes, .. } = graph.resources[resource].0 else {
                unreachable!("transform buffer");
            };
            EffectAccess {
                resource,
                range: 0..bytes,
                kind,
                view: None,
                history: None,
            }
        };
        let pass = |name, accesses| EffectPass {
            name,
            cadence: EffectCadence::Skippable,
            async_eligible: false,
            accesses,
        };
        for spec in specs.iter().filter(|spec| spec.key != PassKey::Long) {
            graph.passes.push(pass(
                match spec.key {
                    PassKey::Evolve => "river flow bands evolve".into(),
                    PassKey::Rows => "river flow bands rows".into(),
                    PassKey::Columns => "river flow bands columns".into(),
                    PassKey::Unpack => "river flow bands unpack".into(),
                    PassKey::Long => unreachable!("flow bands have no long cache"),
                },
                spec.bindings
                    .iter()
                    .map(|b| buffer_access(b.buffer, b.access))
                    .collect(),
            ));
        }
        graph.passes.push(pass(
            format!("{PASS} pack"),
            vec![
                buffer_access(BufferKey::Cascades, AccessKind::StorageRead),
                self.access(texture, 0, AccessKind::StorageWrite),
            ],
        ));
        for mip in 1..MIPS {
            graph.passes.push(pass(
                format!("{PASS} mip {mip}"),
                vec![
                    self.access(texture, mip - 1, AccessKind::SampledRead),
                    self.access(texture, mip, AccessKind::StorageWrite),
                ],
            ));
        }
    }
}

fn pyramid(
    device: &DeviceSite,
    texture: &wgpu::Texture,
    data: &wgpu::Buffer,
    first: u32,
    base_mip: u32,
) -> Vec<Dispatch> {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(PASS),
        source: wgpu::ShaderSource::Wgsl(
            (crate::filtered::SHADER.to_owned() + include_str!("flow.wgsl")).into(),
        ),
    });
    let storage = |slot| {
        binding::storage_texture(
            slot,
            wgpu::ShaderStages::COMPUTE,
            wgpu::StorageTextureAccess::WriteOnly,
            FORMAT,
            wgpu::TextureViewDimension::D2Array,
        )
    };
    let pipeline = |entry, entries: &[wgpu::BindGroupLayoutEntry]| {
        let group = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(PASS),
            entries,
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(PASS),
            bind_group_layouts: &[Some(&group)],
            immediate_size: 0,
        });
        device.compute(&wgpu::ComputePipelineDescriptor {
            label: Some(PASS),
            layout: Some(&layout),
            module: &shader,
            entry_point: Some(entry),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            cache: None,
        })
    };
    let pack = pipeline(
        "pack_flow",
        &[
            binding::storage(0, wgpu::ShaderStages::COMPUTE, true, 0),
            storage(2),
        ],
    );
    let reduce = pipeline(
        "reduce_texture",
        &[
            binding::texture(
                4,
                wgpu::ShaderStages::COMPUTE,
                wgpu::TextureSampleType::Float { filterable: false },
                wgpu::TextureViewDimension::D2Array,
            ),
            storage(5),
        ],
    );
    let at = |mip| {
        texture.create_view(&wgpu::TextureViewDescriptor {
            base_mip_level: mip + base_mip,
            mip_level_count: Some(1),
            base_array_layer: first,
            array_layer_count: Some(LAYERS),
            ..Default::default()
        })
    };
    (0..MIPS)
        .map(|mip| {
            let dest = at(mip);
            let parent = mip.checked_sub(1).map(at);
            let (pipeline, entries) = if let Some(parent) = &parent {
                (
                    &reduce,
                    [
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: wgpu::BindingResource::TextureView(parent),
                        },
                        wgpu::BindGroupEntry {
                            binding: 5,
                            resource: wgpu::BindingResource::TextureView(&dest),
                        },
                    ],
                )
            } else {
                (
                    &pack,
                    [
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: data.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(&dest),
                        },
                    ],
                )
            };
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(PASS),
                layout: &pipeline.get_bind_group_layout(0),
                entries: &entries,
            });
            let groups = (TRANSFORM_SIZE >> mip).div_ceil(8);
            Dispatch {
                pipeline: pipeline.clone(),
                group,
                counts: [groups, groups, LAYERS],
            }
        })
        .collect()
}
