//! Late effects share one refraction source and independently request scene depth.
use super::{RenderError, SceneContext, SyntheticTargetInputs, TargetResources};
use crate::contribution::AfterOpaqueImages;
use crate::graph::PassBody;
use crate::graph::{
    Access, AccessKind, CompiledPass, FrameGraph, GraphColorAttachment, GraphDepthAttachment,
    PassScope, PipelineKey, ResourceDescriptor, ResourceId, ResourceScope,
};
use crate::pass_key::{PassKey, PipelineRole};
use std::{collections::BTreeSet, sync::Arc};

pub(super) struct TargetEffects {
    pub(super) groups: Vec<wgpu::BindGroup>,
    pub(super) names: Vec<Arc<str>>,
}
pub(super) struct LateGraph {
    pub(super) copy: Option<ResourceId>,
    pub(super) images: Vec<Vec<ResourceId>>,
    pub(super) first: Option<usize>,
    pub(super) last: usize,
    pub(super) colour: usize,
}
impl LateGraph {
    pub(super) fn order_consumers(
        &self,
        graph: &mut FrameGraph,
        passes: impl IntoIterator<Item = Option<usize>>,
    ) -> Result<(), RenderError> {
        if self.first.is_some() {
            for pass in passes.into_iter().flatten() {
                graph.dependency(self.last, pass)?;
                if self.colour != self.last {
                    graph.dependency(self.colour, pass)?;
                }
            }
        }
        Ok(())
    }
}

impl super::OffscreenTarget {
    pub(crate) fn first_late_pass(&self) -> Option<&str> {
        if self.effects.groups.is_empty() {
            return None;
        }
        self.graph(false)
            .plan()
            .pass_bindings()
            .find(|(_, pass)| pass.key == Some(PassKey::AfterOpaque(0)))
            .map(|(_, pass)| pass.name.as_ref())
    }
}

impl SyntheticTargetInputs {
    pub(super) fn late_key(&self, index: usize) -> PipelineKey {
        PipelineRole::AfterOpaque(crate::pass_key::pass_index(index)).key(self.key)
    }

    pub(super) fn append_after_opaque(
        &self,
        graph: &mut FrameGraph,
        keys: &mut BTreeSet<PipelineKey>,
        targets: [&ResourceId; 4],
        extent: [u32; 2],
        resources: &crate::contribution::EffectResources,
        uniforms: &[Access],
        forward: usize,
    ) -> Result<LateGraph, RenderError> {
        let [depth, scene, motion, reactive] = targets;
        let refracts = self
            .effect_graph
            .after_opaque
            .iter()
            .any(|pass| pass.refraction);
        let mut first = None;
        let mut last = forward;
        let mut previous_render = forward;
        let copy = if refracts {
            let id = graph.resource(
                ResourceDescriptor::Texture {
                    dimension: crate::graph::TextureKind::D2,
                    width: extent[0],
                    height: extent[1],
                    layers: 1,
                    mips: 1,
                    samples: 1,
                    format: super::SCENE_FORMAT,
                    usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
                },
                ResourceScope::Transient,
            )?;
            let pass = graph.declare(
                PassKey::AfterOpaqueCopy,
                PassBody {
                    scope: PassScope::View(1),
                    pipeline: None,
                    accesses: vec![
                        Access::new(scene.clone(), 0..1, AccessKind::CopyRead),
                        Access::new(id.clone(), 0..1, AccessKind::CopyWrite),
                    ],
                    external_side_effect: false,
                },
            );
            graph.dependency(last, pass)?;
            first = Some(pass);
            last = pass;
            Some(id)
        } else {
            None
        };
        let mut images: Vec<Vec<ResourceId>> = Vec::new();
        for (index, declaration) in self.effect_graph.after_opaque.iter().enumerate() {
            if declaration.kind == crate::contribution::AfterOpaqueKind::PreTone {
                images.push(Vec::new());
                continue;
            }
            let key = self.late_key(index);
            keys.insert(key);
            let mut accesses = uniforms.to_vec();
            accesses.extend(resources.accesses(&declaration.accesses));
            if declaration.kind == crate::contribution::AfterOpaqueKind::Render {
                accesses.extend(
                    [scene, motion, reactive]
                        .map(|id| Access::new(id.clone(), 0..1, AccessKind::AttachmentLoad)),
                );
            }
            if declaration.depth {
                accesses.push(Access::new(depth.clone(), 0..1, AccessKind::SampledRead));
            }
            if declaration.write_depth {
                accesses.push(Access::new(depth.clone(), 0..1, AccessKind::AttachmentLoad));
            }
            if declaration.refraction {
                accesses.push(Access::new(
                    copy.clone().ok_or(RenderError::GraphPassMismatch)?,
                    0..1,
                    AccessKind::SampledRead,
                ));
            }
            let writes = match declaration.images {
                AfterOpaqueImages::None => Vec::new(),
                AfterOpaqueImages::Capture(formats) => {
                    accesses.push(Access::new(scene.clone(), 0..1, AccessKind::SampledRead));
                    capture_images(graph, formats, extent, &mut accesses)?
                }
                AfterOpaqueImages::Sample { producer, .. } => {
                    accesses.extend(
                        images[producer]
                            .iter()
                            .cloned()
                            .map(|id| Access::new(id, 0..1, AccessKind::SampledRead)),
                    );
                    Vec::new()
                }
            };
            images.push(writes);
            let pass = graph.declare_named(
                PassKey::AfterOpaque(crate::pass_key::pass_index(index)),
                format!("effect after {index}: {}", declaration.name),
                PassBody {
                    scope: PassScope::View(1),
                    pipeline: Some(key),
                    accesses,
                    external_side_effect: true,
                },
            );
            graph.dependency(last, pass)?;
            if declaration.kind == crate::contribution::AfterOpaqueKind::Render
                && last != previous_render
            {
                graph.dependency(previous_render, pass)?;
            }
            first.get_or_insert(pass);
            last = pass;
            if declaration.kind == crate::contribution::AfterOpaqueKind::Render {
                previous_render = pass;
            }
        }
        Ok(LateGraph {
            copy,
            images,
            first,
            last,
            colour: previous_render,
        })
    }

    pub(super) fn append_pre_tone(
        &self,
        graph: &mut FrameGraph,
        keys: &mut BTreeSet<PipelineKey>,
        colour: &ResourceId,
        resources: &crate::contribution::EffectResources,
        uniforms: &[Access],
        mut producer: usize,
    ) -> Result<usize, RenderError> {
        for (index, declaration) in self.effect_graph.after_opaque.iter().enumerate() {
            if declaration.kind != crate::contribution::AfterOpaqueKind::PreTone {
                continue;
            }
            let key = self.late_key(index);
            keys.insert(key);
            let pass = graph.declare_named(
                PassKey::AfterOpaque(crate::pass_key::pass_index(index)),
                format!("effect before tone {index}: {}", declaration.name),
                PassBody {
                    scope: PassScope::View(1),
                    pipeline: Some(key),
                    accesses: uniforms
                        .iter()
                        .cloned()
                        .chain(resources.accesses(&declaration.accesses))
                        .chain([Access::new(
                            colour.clone(),
                            0..1,
                            AccessKind::AttachmentLoad,
                        )])
                        .collect(),
                    external_side_effect: true,
                },
            );
            graph.dependency(producer, pass)?;
            producer = pass;
        }
        Ok(producer)
    }

    pub(super) fn late_bindings(
        &self,
        device: &wgpu::Device,
        textures: &crate::graph::GraphTextures,
        ids: &TargetResources,
        depth: &wgpu::TextureView,
    ) -> Result<TargetEffects, RenderError> {
        let copy = ids
            .refraction
            .as_ref()
            .map(|id| {
                textures
                    .texture(id)
                    .ok_or(RenderError::GraphPassMismatch)
                    .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()))
            })
            .transpose()?;
        let groups = self
            .effect_graph
            .after_opaque
            .iter()
            .enumerate()
            .map(|(index, declaration)| {
                let layout = declaration.input_layout(device);
                let (source, images) = match declaration.images {
                    AfterOpaqueImages::None => (None, &[][..]),
                    AfterOpaqueImages::Capture(_) => (
                        Some(texture_view(textures, &ids.scene)?),
                        ids.after_opaque_images[index].as_slice(),
                    ),
                    AfterOpaqueImages::Sample { producer, .. } => {
                        (None, ids.after_opaque_images[producer].as_slice())
                    }
                };
                let views = images
                    .iter()
                    .map(|id| texture_view(textures, id))
                    .collect::<Result<Vec<_>, _>>()?;
                let mut entries = Vec::new();
                if declaration.depth {
                    entries.push(wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(depth),
                    });
                }
                if declaration.refraction {
                    entries.push(wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(
                            copy.as_ref().ok_or(RenderError::GraphPassMismatch)?,
                        ),
                    });
                }
                if let Some(view) = &source {
                    entries.push(wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(view),
                    });
                }
                entries.extend((AfterOpaqueImages::FIRST_IMAGE..).zip(&views).map(
                    |(binding, view)| wgpu::BindGroupEntry {
                        binding,
                        resource: wgpu::BindingResource::TextureView(view),
                    },
                ));
                Ok(device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(&declaration.name),
                    layout: &layout,
                    entries: &entries,
                }))
            })
            .collect::<Result<_, RenderError>>()?;
        let names = self
            .effect_graph
            .passes
            .iter()
            .enumerate()
            .map(|(index, pass)| format!("effect {index}: {}", pass.name))
            .chain(
                self.effect_graph
                    .after_opaque
                    .iter()
                    .enumerate()
                    .filter(|(_, pass)| pass.kind == crate::contribution::AfterOpaqueKind::Compute)
                    .map(|(index, pass)| format!("effect after {index}: {}", pass.name)),
            )
            .map(Arc::from)
            .collect();
        Ok(TargetEffects { groups, names })
    }
}

fn texture_view(
    textures: &crate::graph::GraphTextures,
    id: &ResourceId,
) -> Result<wgpu::TextureView, RenderError> {
    textures
        .texture(id)
        .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()))
        .ok_or(RenderError::GraphPassMismatch)
}

fn capture_images(
    graph: &mut FrameGraph,
    formats: &[wgpu::TextureFormat],
    [width, height]: [u32; 2],
    accesses: &mut Vec<Access>,
) -> Result<Vec<ResourceId>, RenderError> {
    formats
        .iter()
        .map(|&format| {
            let id = graph.resource(
                ResourceDescriptor::Texture {
                    dimension: crate::graph::TextureKind::D2,
                    width,
                    height,
                    layers: 1,
                    mips: 1,
                    samples: 1,
                    format,
                    usage: wgpu::TextureUsages::TEXTURE_BINDING
                        | wgpu::TextureUsages::STORAGE_BINDING,
                },
                ResourceScope::Transient,
            )?;
            accesses.push(Access::new(id.clone(), 0..1, AccessKind::StorageWrite));
            Ok(id)
        })
        .collect()
}

impl SceneContext<'_> {
    pub(super) fn refraction_copy(&mut self) -> Result<(), RenderError> {
        let source = self
            .target
            .textures
            .texture(&self.target.ids.scene)
            .ok_or(RenderError::GraphPassMismatch)?;
        let dest = self
            .target
            .ids
            .refraction
            .as_ref()
            .and_then(|id| self.target.textures.texture(id))
            .ok_or(RenderError::GraphPassMismatch)?;
        self.encoder.copy_texture_to_texture(
            source.as_image_copy(),
            dest.as_image_copy(),
            source.size(),
        );
        Ok(())
    }
    pub(super) fn after_opaque(
        &mut self,
        index: usize,
        compiled: &CompiledPass,
    ) -> Result<(), RenderError> {
        use crate::contribution::AfterOpaqueKind;
        let declaration = &self.prepared.effect_graph.after_opaque[index];
        let pre_tone = match declaration.kind {
            AfterOpaqueKind::Compute => return self.after_opaque_compute(index, compiled),
            AfterOpaqueKind::Render => false,
            AfterOpaqueKind::PreTone => true,
        };
        if pre_tone
            && !self
                .contribution
                .as_deref()
                .is_some_and(|effect| effect.after_opaque_runs(index, &self.contribution_frame))
        {
            return Ok(());
        }
        let colour = if pre_tone {
            &self.target.ids.pre_tone.colour
        } else {
            &self.target.ids.scene
        };
        let attachment = |resource, view| {
            Some(GraphColorAttachment {
                resource,
                view,
                range: 0..1,
                clear: wgpu::Color::BLACK,
            })
        };
        let mut pass = if pre_tone {
            compiled.begin_render(
                self.encoder,
                [attachment(colour, &self.target.display.colour)],
                None,
                self.generation,
                self.timings.as_deref_mut(),
            )?
        } else {
            compiled.begin_render(
                self.encoder,
                [
                    attachment(colour, &self.target.scene_view),
                    attachment(&self.target.ids.motion, &self.target.motion_view),
                    attachment(&self.target.ids.reactive, &self.target.reactive_view),
                ],
                declaration.write_depth.then_some(GraphDepthAttachment {
                    resource: &self.target.ids.depth,
                    view: &self.target.depth,
                    range: 0..1,
                    clear: 0.0,
                }),
                self.generation,
                self.timings.as_deref_mut(),
            )?
        }
        .ok_or(RenderError::GraphPassMismatch)?;
        if colour == &self.target.ids.scene {
            self.target.set_drawn_viewport(&mut pass);
        }
        let view_offset =
            self.prepared.view_stride * u32::from(self.target.frame_revision & 1 != 0);
        let offsets = [view_offset, 0];
        let count = if self.prepared.bucket_route.is_some() {
            2
        } else {
            1
        };
        pass.set_bind_group(0, &self.target.view_bind_group, &offsets[..count]);
        pass.set_bind_group(1, &self.target.effects.groups[index], &[]);
        if let Some(contribution) = self.contribution.as_deref() {
            contribution.draw_after_opaque(index, &mut pass, &self.contribution_frame);
        }
        Ok(())
    }
    fn after_opaque_compute(
        &mut self,
        index: usize,
        compiled: &CompiledPass,
    ) -> Result<(), RenderError> {
        if let Some(contribution) = self.contribution.as_deref_mut()
            && contribution.after_opaque_runs(index, &self.contribution_frame)
        {
            let mut pass = compiled
                .begin_compute(self.encoder, self.generation, self.timings.as_deref_mut())?
                .ok_or(RenderError::GraphPassMismatch)?;
            let offset = self.prepared.view_stride * u32::from(self.target.frame_revision & 1 != 0);
            let offsets = [offset, 0];
            let count = if self.prepared.bucket_route.is_some() {
                2
            } else {
                1
            };
            pass.set_bind_group(0, &self.target.view_bind_group, &offsets[..count]);
            pass.set_bind_group(1, &self.target.effects.groups[index], &[]);
            contribution.encode_after_opaque(index, &mut pass, &self.contribution_frame);
        }
        Ok(())
    }
}
