//! Independent cascade targets and fits, with the world's height layer read
//! through a separate binding. No camera-visible list supplies these draws.
use super::{
    Access, AccessKind, BTreeSet, CompiledPass, FrameGraph, Pass, PassScope, PipelineKey,
    RenderError, ResourceDescriptor, ResourceId, ResourceScope, SceneContext,
};
use crate::synthetic::shadow;

pub(super) const PASS: &str = "secondary view Sun cascade";
pub(super) struct Sun {
    pub(super) prepared: shadow::ShadowPrepared,
    pub(super) target: shadow::ShadowTarget,
    texture: wgpu::Texture,
}

impl Sun {
    pub(super) fn new(world: &shadow::ShadowPrepared, device: &crate::runtime::DeviceSite) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(PASS),
            size: wgpu::Extent3d {
                width: shadow::SIZE,
                height: shadow::SIZE,
                depth_or_array_layers: shadow::LAYERS,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: shadow::FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        Self {
            prepared: world.secondary(device),
            target: shadow::ShadowTarget::new(&texture),
            texture,
        }
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
                layers: shadow::LAYERS,
                mips: 1,
                samples: 1,
                format: shadow::FORMAT,
                usage: self.texture.usage(),
            },
            imported,
        )?;
        for layer in 0..2_u32 {
            let matrix = graph.resource(
                ResourceDescriptor::Buffer {
                    bytes: shadow::CASTER_VIEW_RANGE,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                },
                imported,
            )?;
            let key = PipelineKey {
                role: key.role + 3 + layer,
                ..key
            };
            keys.insert(key);
            let mut accesses = shared.to_vec();
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
            graph.pass(Pass {
                name: format!("{PASS} {index} {layer}"),
                scope: PassScope::View(index as u64 + 2),
                pipeline: Some(key),
                accesses,
                external_side_effect: false,
            });
        }
        let samples = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: shadow::SAMPLE_BYTES as u64,
                usage: self.prepared.sample_buffer.usage(),
            },
            imported,
        )?;
        opaque.extend([
            Access::new(maps.clone(), 0..2, AccessKind::SampledRead),
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
        // A single cascade uses the whole-reach far fit. Zero uses only
        // the scene's height layer, so neither cascade is recorded.
        if request.cascades == 0 || (request.cascades == 1 && layer == 0) {
            return Ok(());
        }
        let sun = slot.sun.as_ref().ok_or(RenderError::GraphPassMismatch)?;
        let world = prepared
            .shadow
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?;
        sun.prepared.write_secondary(
            world,
            &request.camera,
            (request.cascades, request.shadow_reach),
            None,
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
            culled: None,
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
            None,
        )
    }
}
