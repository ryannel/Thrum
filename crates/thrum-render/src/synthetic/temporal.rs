//! GPU exposure and submitted temporal-input diagnostics. No CPU metering.
use super::{OffscreenTarget, PreparedSynthetic, RenderError, SceneContext};
use crate::graph::{
    Access, AccessKind, CompiledPass, FrameGraph, GraphColorAttachment, Pass, PassScope,
    PipelineKey, ResourceDescriptor, ResourceId, ResourceScope,
};
use std::collections::BTreeSet;

pub(super) struct TemporalInputs {
    pipelines: [wgpu::RenderPipeline; 2],
}
pub(super) struct TemporalTarget {
    exposure: wgpu::TextureView,
    exposure_groups: [wgpu::BindGroup; 2],
    pub(super) witness: Option<(wgpu::TextureView, [wgpu::BindGroup; 2])>,
}
pub(super) struct TemporalResources {
    pub(super) exposure: ResourceId,
    pub(super) witness: Option<ResourceId>,
    pub(super) witness_pass: Option<usize>,
}

impl TemporalInputs {
    pub(super) fn new(device: &crate::runtime::DeviceSite) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("temporal inputs"),
            source: wgpu::ShaderSource::Wgsl(include_str!("temporal.wgsl").into()),
        });
        let pipelines = std::array::from_fn(|i| {
            device.render(&wgpu::RenderPipelineDescriptor {
                label: Some("temporal inputs"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vertex"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(if i == 0 { "resolve" } else { "witness" }),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: if i == 0 {
                            wgpu::TextureFormat::R16Float
                        } else {
                            wgpu::TextureFormat::Rgba16Float
                        },
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        });
        Self { pipelines }
    }

    pub(super) fn target(
        &self,
        device: &wgpu::Device,
        textures: &crate::graph::GraphTextures,
        ids: &TemporalResources,
        inputs: [&wgpu::TextureView; 3],
        samples: &[wgpu::Buffer; 2],
    ) -> Result<TemporalTarget, RenderError> {
        let view = |id| {
            textures
                .texture(id)
                .ok_or(RenderError::GraphPassMismatch)
                .map(|t| t.create_view(&wgpu::TextureViewDescriptor::default()))
        };
        let groups =
            |witness: bool| {
                std::array::from_fn(|side| {
                    let mut entries = if witness {
                        Vec::new()
                    } else {
                        vec![wgpu::BindGroupEntry {
                            binding: 0,
                            resource: samples[side].as_entire_binding(),
                        }]
                    };
                    if witness {
                        entries.extend(inputs.iter().enumerate().map(|(i, v)| {
                            wgpu::BindGroupEntry {
                                binding: u32::try_from(i + 1).expect("three inputs"),
                                resource: wgpu::BindingResource::TextureView(v),
                            }
                        }));
                    }
                    device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("temporal inputs"),
                        layout: &self.pipelines[usize::from(witness)].get_bind_group_layout(0),
                        entries: &entries,
                    })
                })
            };
        Ok(TemporalTarget {
            exposure: view(&ids.exposure)?,
            exposure_groups: groups(false),
            witness: ids
                .witness
                .as_ref()
                .map(|id| Ok::<_, RenderError>((view(id)?, groups(true))))
                .transpose()?,
        })
    }
}

impl PreparedSynthetic {
    pub(super) fn temporal_key(&self, witness: bool) -> PipelineKey {
        PipelineKey {
            role: 0x5440 + u32::from(witness),
            ..self.key
        }
    }

    pub(super) fn append_temporal(
        &self,
        graph: &mut FrameGraph,
        keys: &mut BTreeSet<PipelineKey>,
        inputs: [&ResourceId; 3],
        exposure: &ResourceId,
        extent: [u32; 2],
    ) -> Result<TemporalResources, RenderError> {
        let mut append = |witness: bool| {
            let key = self.temporal_key(witness);
            keys.insert(key);
            let [width, height] = if witness { extent } else { [1, 1] };
            let output = graph.resource(
                ResourceDescriptor::Texture {
                    dimension: crate::graph::TextureKind::D2,
                    width,
                    height,
                    layers: 1,
                    mips: 1,
                    samples: 1,
                    format: if witness {
                        wgpu::TextureFormat::Rgba16Float
                    } else {
                        wgpu::TextureFormat::R16Float
                    },
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::COPY_SRC,
                },
                ResourceScope::Transient,
            )?;
            let mut accesses = vec![Access {
                view: None,
                resource: output.clone(),
                range: 0..1,
                kind: AccessKind::AttachmentClear,
                store: true,
            }];
            if !witness {
                accesses.push(Access {
                    view: None,
                    resource: exposure.clone(),
                    range: 0..super::exposure::SAMPLE_BYTES,
                    kind: AccessKind::UniformRead,
                    store: true,
                });
            }
            if witness {
                accesses.extend(inputs.iter().map(|id| Access {
                    view: None,
                    resource: (*id).clone(),
                    range: 0..1,
                    kind: AccessKind::SampledRead,
                    store: true,
                }));
            }
            let pass = graph.pass(Pass {
                name: if witness {
                    "temporal input witness"
                } else {
                    "temporal exposure texture"
                }
                .into(),
                scope: PassScope::View(1),
                pipeline: Some(key),
                accesses,
                external_side_effect: true,
            });
            Ok::<_, RenderError>((output, pass))
        };
        let (exposure, _) = append(false)?;
        let witness = self.temporal_inputs.then(|| append(true)).transpose()?;
        let (witness, witness_pass) =
            witness.map_or((None, None), |(id, pass)| (Some(id), Some(pass)));
        Ok(TemporalResources {
            exposure,
            witness,
            witness_pass,
        })
    }
}

impl SceneContext<'_> {
    pub(super) fn temporal(
        &mut self,
        pass: &CompiledPass,
        witness: bool,
    ) -> Result<(), RenderError> {
        let TemporalTarget {
            exposure,
            exposure_groups,
            witness: target,
        } = &self.target.temporal;
        let (view, groups, id) = if witness {
            let (view, groups) = target.as_ref().ok_or(RenderError::GraphPassMismatch)?;
            (
                view,
                groups,
                self.target
                    .ids
                    .temporal
                    .witness
                    .as_ref()
                    .ok_or(RenderError::GraphPassMismatch)?,
            )
        } else {
            (
                exposure,
                exposure_groups,
                &self.target.ids.temporal.exposure,
            )
        };
        let mut render = pass
            .begin_render(
                self.encoder,
                [Some(GraphColorAttachment {
                    resource: id,
                    range: 0..1,
                    view,
                    clear: wgpu::Color::TRANSPARENT,
                })],
                None,
                self.generation,
                self.timings.as_deref_mut(),
            )?
            .ok_or(RenderError::GraphPassMismatch)?;
        render.set_pipeline(&self.prepared.temporal.pipelines[usize::from(witness)]);
        render.set_bind_group(0, &groups[self.exposure_side], &[]);
        render.draw(0..3, 0..1);
        Ok(())
    }
}

impl OffscreenTarget {
    /// Submitted reconstruction diagnostic, prepared only for a declared reader.
    pub fn temporal_witness(&self) -> Option<&wgpu::Texture> {
        self.textures.texture(self.ids.temporal.witness.as_ref()?)
    }
}

#[cfg(all(test, target_os = "macos"))]
pub(super) mod tests;
