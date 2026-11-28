//! Independent cascade targets and fits, with the world's height layer read
//! through a separate binding. No camera-visible list supplies these draws.
use super::{
    Access, AccessKind, BTreeSet, CompiledPass, FrameGraph, PassScope, PipelineKey, RenderError,
    ResourceDescriptor, ResourceId, ResourceScope, SceneContext,
};
use crate::graph::PassBody;
use crate::pass_key::{PassKey, PipelineRole, SecondaryStage};
use crate::synthetic::{history_gpu, shadow};
use crate::visibility::casters::{CasterCulling, CasterInputs};

pub(super) const PASS: &str = "secondary view Sun cascade";
const CULL_PASS: &str = "secondary view Sun caster cull";
#[derive(Clone)]
pub(super) struct Sun {
    pub(super) prepared: shadow::ShadowPrepared,
    pub(super) target: shadow::ShadowTarget,
    texture: wgpu::Texture,
    pub(super) culling: Option<(CasterCulling, Vec<wgpu::BindGroup>)>,
}

impl Sun {
    pub(super) fn new(
        world: &shadow::ShadowPrepared,
        scene: &super::PreparedSynthetic,
        queue: &wgpu::Queue,
    ) -> Result<Self, RenderError> {
        let device = scene.capabilities.device();
        let prepared = world.secondary(device);
        let culling = match (&scene.bucket_route, &scene.visibility) {
            (Some(route), Some(_)) => {
                let mut culling = CasterCulling::new(
                    device,
                    queue,
                    &super::super::camera_culling::mesh_records(&scene.mesh_arena)?,
                    route.shape()?,
                    &CasterInputs {
                        instances: scene.instance_upload.buffer(),
                        serials: scene.history.serial_buffer(),
                        members: route.member_inputs(),
                        views: prepared
                            .caster_views()
                            .into_iter()
                            .take(shadow::SECONDARY_CASCADES as usize)
                            .collect(),
                    },
                );
                culling.set_member_groups(route.groups());
                let groups = culling
                    .lists()
                    .map(|(list, _)| {
                        device.create_bind_group(&wgpu::BindGroupDescriptor {
                            label: Some(CULL_PASS),
                            layout: prepared.member_layout().expect("bucket caster layout"),
                            entries: &[wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                    buffer: list,
                                    offset: 0,
                                    size: wgpu::BufferSize::new(culling.segment_bytes()),
                                }),
                            }],
                        })
                    })
                    .collect();
                Some((culling, groups))
            }
            _ => None,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(PASS),
            size: wgpu::Extent3d {
                width: shadow::SIZE,
                height: shadow::SIZE,
                depth_or_array_layers: shadow::SECONDARY_LAYERS,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: shadow::FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        Ok(Self {
            prepared,
            target: shadow::ShadowTarget::new(&texture),
            texture,
            culling,
        })
    }

    pub(super) fn append(
        &self,
        graph: &mut FrameGraph,
        keys: &mut BTreeSet<PipelineKey>,
        key: PipelineKey,
        index: usize,
        shared: &[Access],
        opaque: &mut Vec<Access>,
    ) -> Result<ResourceId, RenderError> {
        let imported = ResourceScope::Imported { initialized: true };
        let maps = graph.resource(
            ResourceDescriptor::Texture {
                dimension: crate::graph::TextureKind::D2,
                width: shadow::SIZE,
                height: shadow::SIZE,
                layers: shadow::SECONDARY_LAYERS,
                mips: 1,
                samples: 1,
                format: shadow::FORMAT,
                usage: self.texture.usage(),
            },
            imported,
        )?;
        let matrices = (0..shadow::SECONDARY_LAYERS)
            .map(|_| {
                graph.resource(
                    ResourceDescriptor::Buffer {
                        bytes: shadow::CASTER_VIEW_RANGE,
                        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    },
                    imported,
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let lists = if let Some((culling, _)) = &self.culling {
            let mut accesses = shared.to_vec();
            accesses.extend(
                matrices
                    .iter()
                    .take(shadow::SECONDARY_CASCADES as usize)
                    .map(|matrix| {
                        Access::new(
                            matrix.clone(),
                            0..shadow::CASTER_VIEW_RANGE,
                            AccessKind::UniformRead,
                        )
                    }),
            );
            let serials = graph.resource(
                ResourceDescriptor::Buffer {
                    bytes: history_gpu::SERIAL_BYTES,
                    usage: wgpu::BufferUsages::UNIFORM,
                },
                imported,
            )?;
            accesses.push(Access::new(
                serials,
                0..history_gpu::SERIAL_BYTES,
                AccessKind::UniformRead,
            ));
            culling.append_mesh_read(graph, &mut accesses)?;
            let ids = culling.append_outputs(graph, &mut accesses)?;
            let cull_key = PipelineRole::Secondary {
                view: crate::pass_key::pass_index(index),
                stage: SecondaryStage::Casters,
            }
            .key(key);
            keys.insert(cull_key);
            graph.declare(
                PassKey::Secondary {
                    view: crate::pass_key::pass_index(index),
                    stage: SecondaryStage::Casters,
                },
                PassBody {
                    scope: PassScope::View(index as u64 + 2),
                    pipeline: Some(cull_key),
                    accesses,
                    external_side_effect: false,
                },
            );
            Some(ids)
        } else {
            None
        };
        for (layer, matrix) in (0..shadow::SECONDARY_LAYERS).zip(matrices) {
            let (cascade, _) =
                shadow::layer_sample(layer as usize, shadow::SECONDARY_CASCADES as usize);
            let stage = SecondaryStage::Sun(layer);
            let key = PipelineRole::Secondary {
                view: crate::pass_key::pass_index(index),
                stage,
            }
            .key(key);
            keys.insert(key);
            let mut accesses = shared.to_vec();
            if let Some(ids) = &lists {
                let (culling, _) = self.culling.as_ref().expect("declared cull lists");
                accesses.extend(culling.list_reads(&ids.lists[cascade]));
            }
            accesses.extend([
                Access::new(
                    matrix,
                    0..shadow::CASTER_VIEW_RANGE,
                    AccessKind::UniformRead,
                ),
                Access::new(
                    maps.clone(),
                    u64::from(layer)..u64::from(layer + 1),
                    AccessKind::AttachmentClear,
                ),
            ]);
            graph.declare(
                PassKey::Secondary {
                    view: crate::pass_key::pass_index(index),
                    stage,
                },
                PassBody {
                    scope: PassScope::View(index as u64 + 2),
                    pipeline: Some(key),
                    accesses,
                    external_side_effect: false,
                },
            );
        }
        let samples = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: shadow::SAMPLE_BYTES as u64,
                usage: self.prepared.sample_buffer.usage(),
            },
            imported,
        )?;
        opaque.extend([
            Access::new(
                maps.clone(),
                0..shadow::SECONDARY_LAYERS as u64,
                AccessKind::SampledRead,
            ),
            Access::new(
                samples,
                0..shadow::SAMPLE_BYTES as u64,
                AccessKind::UniformRead,
            ),
        ]);
        Ok(maps)
    }
}

impl SceneContext<'_> {
    pub(super) fn secondary_casters(
        &mut self,
        index: usize,
        pass: &CompiledPass,
    ) -> Result<(), RenderError> {
        let scene = self.prepared;
        let slot = &scene.secondary[index];
        let Some(request) = slot.handle.asked().filter(|request| request.cascades != 0) else {
            return Ok(());
        };
        let sun = slot.sun.as_ref().ok_or(RenderError::GraphPassMismatch)?;
        sun.prepared.write_secondary(
            scene
                .shadow
                .as_ref()
                .ok_or(RenderError::GraphPassMismatch)?,
            &request.camera,
            (request.cascades, request.shadow_reach),
            self.contribution
                .as_deref()
                .and_then(crate::contribution::SceneContribution::shadow_bounds),
        )?;
        sun.culling
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?
            .0
            .encode_layers(
                self.encoder,
                self.generation,
                pass,
                self.timings.as_deref_mut(),
                0..request.cascades as usize,
            )?
            .then_some(())
            .ok_or(RenderError::GraphPassMismatch)
    }

    pub(super) fn secondary_sun(
        &mut self,
        index: usize,
        layer: usize,
        pass: &CompiledPass,
    ) -> Result<(), RenderError> {
        let prepared = self.prepared;
        let slot = &prepared.secondary[index];
        let Some(request) = slot.handle.asked() else {
            return Ok(());
        };
        let (cascade, sample) = shadow::layer_sample(layer, shadow::SECONDARY_CASCADES as usize);
        if cascade >= request.cascades as usize
            || (sample != 0
                && cascade
                    >= shadow::outline_count(request.cascades, request.shadow_reach) as usize)
        {
            return Ok(());
        }
        let sun = slot.sun.as_ref().ok_or(RenderError::GraphPassMismatch)?;
        let world = prepared
            .shadow
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?;
        let matrices = sun.prepared.write_secondary(
            world,
            &request.camera,
            (request.cascades, request.shadow_reach),
            self.contribution
                .as_deref()
                .and_then(crate::contribution::SceneContribution::shadow_bounds),
        )?;
        let arena = prepared
            .mesh_arena
            .buffers()
            .ok_or(RenderError::GraphPassMismatch)?;
        let (indices, base_vertex) = prepared.cube_draw.clone();
        let cube = [shadow::CasterDraw {
            bucket: 0,
            member_offset: 0,
            indices,
            base_vertex,
            instances: prepared.instance_upload.active_count(),
        }];
        let casters = shadow::CasterDraws {
            indices: &arena.indices,
            two_sided: prepared.instance_upload.two_sided_count() != 0,
            members: prepared.shadow_members.as_ref(),
            culled: match &sun.culling {
                Some((culling, groups)) => Some((
                    &groups[cascade],
                    culling
                        .lists()
                        .nth(cascade)
                        .ok_or(RenderError::GraphPassMismatch)?
                        .1,
                )),
                None => None,
            },
            draws: prepared
                .bucket_route
                .as_ref()
                .map_or(&cube, super::super::BucketRoute::caster_draws),
        };
        sun.prepared.encode_cascade(
            self.encoder,
            &sun.target,
            prepared.history.read_group(),
            &casters,
            self.generation,
            layer,
            pass,
            self.target.ids.secondary[index]
                .sun
                .as_ref()
                .ok_or(RenderError::GraphPassMismatch)?,
            self.timings.as_deref_mut(),
            self.contribution
                .as_deref()
                .filter(|entry| entry.secondary_views_enabled())
                .map(|entry| {
                    let cascade =
                        crate::contribution::SecondaryShadowCascade::new(matrices[cascade]);
                    shadow::CasterContribution::Secondary(entry, index, cascade)
                }),
        )
    }
}
