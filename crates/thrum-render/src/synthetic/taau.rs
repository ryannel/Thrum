//! Prepared portable temporal resolve. One retained pair is shared by frame slots;
//! the submitted transform cursor also selects its read and publication sides.
use super::{OffscreenTarget, PreparedSynthetic, RenderError, SceneContext, SyntheticView};
use crate::graph::{
    Access, AccessKind, FrameGraph, Pass, PassScope, PipelineKey, ResourceDescriptor, ResourceId,
    ResourceScope,
};
use std::collections::BTreeSet;

const PARAM_BYTES: u64 = 224;

pub(super) struct Taau {
    pipeline: wgpu::ComputePipeline,
    reference: bool,
}

pub(super) struct TaauResources {
    pub(super) output: ResourceId,
    pub(super) pass: usize,
}

pub(super) struct TaauTarget {
    pub(super) output: wgpu::TextureView,
    history: [wgpu::TextureView; 2],
    depth: [wgpu::TextureView; 2],
    params: wgpu::Buffer,
    groups: [wgpu::BindGroup; 2],
}

impl Taau {
    pub(super) fn selection(&self) -> super::Reconstruction {
        if self.reference {
            super::Reconstruction::Reference
        } else {
            super::Reconstruction::Taau
        }
    }
    pub(super) fn new(device: &crate::runtime::DeviceSite, reference: bool) -> Self {
        let entries: Vec<_> = (0..=10)
            .filter(|binding| !reference || [0, 4, 6, 7, 9].contains(binding))
            .map(|binding| {
                let ty = match binding {
                    6..=8 => wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: match binding {
                            8 => wgpu::TextureFormat::R32Float,
                            7 if reference => wgpu::TextureFormat::Rgba32Float,
                            _ => wgpu::TextureFormat::Rgba16Float,
                        },
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    9 | 10 => wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(if binding == 9 {
                            PARAM_BYTES
                        } else {
                            super::exposure::SAMPLE_BYTES
                        }),
                    },
                    _ => wgpu::BindingType::Texture {
                        sample_type: if binding == 1 {
                            wgpu::TextureSampleType::Depth
                        } else {
                            wgpu::TextureSampleType::Float { filterable: false }
                        },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                };
                wgpu::BindGroupLayoutEntry {
                    binding,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty,
                    count: None,
                }
            })
            .collect();
        let bindings = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("portable reconstruction inputs"),
            entries: &entries,
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("portable reconstruction"),
            bind_group_layouts: &[Some(&bindings)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("portable TAAU"),
            source: wgpu::ShaderSource::Wgsl(
                if reference {
                    include_str!("reference.wgsl")
                } else {
                    include_str!("taau.wgsl")
                }
                .into(),
            ),
        });
        Self {
            pipeline: device.compute(&wgpu::ComputePipelineDescriptor {
                label: Some("portable TAAU"),
                layout: Some(&layout),
                module: &shader,
                entry_point: Some(if reference {
                    "accumulate"
                } else {
                    "reconstruct"
                }),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            }),
            reference,
        }
    }

    pub(super) fn target(
        &self,
        device: &wgpu::Device,
        output: wgpu::TextureView,
        inputs: [&wgpu::TextureView; 4],
        exposure: &[wgpu::Buffer; 2],
    ) -> TaauTarget {
        let texture = |format| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("submitted TAAU history"),
                    size: output.texture().size(),
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::STORAGE_BINDING,
                    view_formats: &[],
                })
                .create_view(&wgpu::TextureViewDescriptor::default())
        };
        let history = std::array::from_fn(|_| {
            texture(if self.reference {
                wgpu::TextureFormat::Rgba32Float
            } else {
                wgpu::TextureFormat::Rgba16Float
            })
        });
        let depth = std::array::from_fn(|_| texture(wgpu::TextureFormat::R32Float));
        let params = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("TAAU view parameters"),
            size: PARAM_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let groups = self.groups(
            device,
            inputs,
            (&output, &history, &depth),
            &params,
            exposure,
        );
        TaauTarget {
            output,
            history,
            depth,
            params,
            groups,
        }
    }

    fn groups(
        &self,
        device: &wgpu::Device,
        inputs: [&wgpu::TextureView; 4],
        (output, history, depth): (
            &wgpu::TextureView,
            &[wgpu::TextureView; 2],
            &[wgpu::TextureView; 2],
        ),
        params: &wgpu::Buffer,
        exposure: &[wgpu::Buffer; 2],
    ) -> [wgpu::BindGroup; 2] {
        std::array::from_fn(|write| {
            let views = [
                inputs[0],
                inputs[1],
                inputs[2],
                inputs[3],
                &history[1 - write],
                &depth[1 - write],
                output,
                &history[write],
                &depth[write],
            ];
            let mut entries: Vec<_> = views
                .iter()
                .enumerate()
                .filter(|(binding, _)| !self.reference || [0, 4, 6, 7].contains(binding))
                .map(|(binding, view)| wgpu::BindGroupEntry {
                    binding: u32::try_from(binding).expect("nine textures"),
                    resource: wgpu::BindingResource::TextureView(view),
                })
                .collect();
            entries.push(wgpu::BindGroupEntry {
                binding: 9,
                resource: params.as_entire_binding(),
            });
            if !self.reference {
                entries.push(wgpu::BindGroupEntry {
                    binding: 10,
                    resource: exposure[write].as_entire_binding(),
                });
            }
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("TAAU submitted sides"),
                layout: &self.pipeline.get_bind_group_layout(0),
                entries: &entries,
            })
        })
    }
}

impl PreparedSynthetic {
    /// The provider actually prepared for this scene generation.
    pub fn reconstruction(&self) -> super::Reconstruction {
        self.taau
            .as_ref()
            .map_or(super::Reconstruction::Off, Taau::selection)
    }
    pub(super) fn prepare_taau_target(
        &self,
        device: &wgpu::Device,
        textures: &crate::graph::GraphTextures,
        ids: &super::TargetResources,
        inputs: [&wgpu::TextureView; 4],
    ) -> Result<Option<TaauTarget>, RenderError> {
        ids.taau
            .as_ref()
            .map(|ids| {
                let output = textures
                    .texture(&ids.output)
                    .ok_or(RenderError::GraphPassMismatch)?
                    .create_view(&wgpu::TextureViewDescriptor::default());
                Ok(self
                    .taau
                    .as_ref()
                    .ok_or(RenderError::GraphPassMismatch)?
                    .target(device, output, inputs, &self.exposure.samples))
            })
            .transpose()
    }
    pub(super) fn taau_key(&self) -> PipelineKey {
        PipelineKey {
            role: 0x5450,
            ..self.key
        }
    }

    pub(super) fn append_taau(
        &self,
        graph: &mut FrameGraph,
        keys: &mut BTreeSet<PipelineKey>,
        inputs: [&ResourceId; 4],
        exposure: &ResourceId,
        extent: [u32; 2],
    ) -> Result<Option<TaauResources>, RenderError> {
        let Some(taau) = &self.taau else {
            return Ok(None);
        };
        let descriptor = |format| ResourceDescriptor::Texture {
            dimension: crate::graph::TextureKind::D2,
            width: extent[0],
            height: extent[1],
            layers: 1,
            mips: 1,
            samples: 1,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING,
        };
        let output = graph.resource(
            descriptor(wgpu::TextureFormat::Rgba16Float),
            ResourceScope::Transient,
        )?;
        let mut accesses: Vec<_> = inputs
            .into_iter()
            .take(if taau.reference { 1 } else { 4 })
            .map(|id| Access::new(id.clone(), 0..1, AccessKind::SampledRead))
            .collect();
        accesses.push(Access::new(output.clone(), 0..1, AccessKind::StorageWrite));
        for (owner, format) in [
            (
                0x5450,
                if taau.reference {
                    wgpu::TextureFormat::Rgba32Float
                } else {
                    wgpu::TextureFormat::Rgba16Float
                },
            ),
            (0x5451, wgpu::TextureFormat::R32Float),
        ] {
            if taau.reference && owner == 0x5451 {
                continue;
            }
            for kind in [AccessKind::SampledRead, AccessKind::StorageWrite] {
                let id = graph.resource(
                    descriptor(format),
                    ResourceScope::History {
                        owner,
                        epoch: self.key.device_generation,
                        initialized: true,
                    },
                )?;
                accesses.push(Access::new(id, 0..1, kind));
            }
        }
        let params = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: PARAM_BYTES,
                usage: wgpu::BufferUsages::UNIFORM,
            },
            ResourceScope::Imported { initialized: true },
        )?;
        accesses.push(Access::new(params, 0..PARAM_BYTES, AccessKind::UniformRead));
        if !taau.reference {
            accesses.push(Access::new(
                exposure.clone(),
                0..super::exposure::SAMPLE_BYTES,
                AccessKind::UniformRead,
            ));
        }
        keys.insert(self.taau_key());
        let pass = graph.pass(Pass {
            name: "portable TAAU".into(),
            scope: PassScope::View(1),
            pipeline: Some(self.taau_key()),
            accesses,
            external_side_effect: true,
        });
        Ok(Some(TaauResources { output, pass }))
    }
}

impl TaauTarget {
    pub(super) fn update(
        &self,
        queue: &wgpu::Queue,
        view: &SyntheticView,
        previous: &SyntheticView,
        valid: bool,
        extent: [u32; 2],
    ) {
        let mut words = [0.0_f32; 56];
        let inverse = glam::Mat4::from_cols_array_2d(&view.view_proj_columns()).inverse();
        words[..16].copy_from_slice(&inverse.to_cols_array());
        let matrix = |v: &SyntheticView| match v.projection {
            super::view::Projection::Perspective { view, .. } => view,
            super::view::Projection::OrthographicFixture => glam::Mat4::IDENTITY,
        };
        words[16..32].copy_from_slice(&matrix(previous).to_cols_array());
        words[32..48].copy_from_slice(&matrix(view).to_cols_array());
        words[48..52].copy_from_slice(&[
            f32::from(u8::from(valid)),
            super::PRE_EXPOSURE,
            extent[0] as f32,
            extent[1] as f32,
        ]);
        let packed = view.pack_view(previous, 1.0);
        words[52..56].copy_from_slice(&[
            f32::from_le_bytes(packed[152..156].try_into().expect("jitter x")),
            f32::from_le_bytes(packed[156..160].try_into().expect("jitter y")),
            u32::from_le_bytes(packed[228..232].try_into().expect("jitter phase")) as f32,
            0.0,
        ]);
        let bytes: [u8; 224] = std::array::from_fn(|i| words[i / 4].to_le_bytes()[i % 4]);
        queue.write_buffer(&self.params, 0, &bytes);
    }
}

impl OffscreenTarget {
    pub(crate) fn share_reconstruction_history(
        &mut self,
        prepared: &PreparedSynthetic,
        first: &Self,
    ) {
        if let (Some(taau), Some(target), Some(source)) =
            (&prepared.taau, &mut self.taau, &first.taau)
        {
            target.history.clone_from(&source.history);
            target.depth.clone_from(&source.depth);
            target.groups = taau.groups(
                &self.device,
                [
                    &self.scene_view,
                    &self.depth,
                    &self.motion_view,
                    &self.reactive_view,
                ],
                (&target.output, &target.history, &target.depth),
                &target.params,
                &prepared.exposure.samples,
            );
        }
    }
}

impl SceneContext<'_> {
    pub(super) fn taau(
        &mut self,
        compiled: &crate::graph::CompiledPass,
    ) -> Result<(), RenderError> {
        let taau = self
            .prepared
            .taau
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?;
        let target = self
            .target
            .taau
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?;
        let mut pass = compiled
            .begin_compute(self.encoder, self.generation, self.timings.as_deref_mut())?
            .ok_or(RenderError::GraphPassMismatch)?;
        pass.set_pipeline(&taau.pipeline);
        pass.set_bind_group(0, &target.groups[self.exposure_side], &[]);
        pass.dispatch_workgroups(
            self.target.width.div_ceil(8),
            self.target.height.div_ceil(8),
            1,
        );
        Ok(())
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests;
