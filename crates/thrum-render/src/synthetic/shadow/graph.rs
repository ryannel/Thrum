//! Graph declarations for the Sun's two cascades and top-down caster raster.

use std::collections::BTreeSet;

use crate::graph::{
    Access, AccessKind, FrameGraph, Pass, PassScope, PipelineKey, ResourceDescriptor, ResourceId,
    ResourceScope,
};

use super::super::{CUBE_INDEX_BYTES, PreparedSynthetic, RenderError, shadow};

impl PreparedSynthetic {
    pub(in crate::synthetic) fn append_shadow_passes(
        &self,
        graph: &mut FrameGraph,
        instance: &ResourceId,
        cube_indices: &ResourceId,
        accesses: &mut Vec<Access>,
        prepared_keys: &mut BTreeSet<PipelineKey>,
        contribution_reads: &[Access],
    ) -> Result<Option<ResourceId>, RenderError> {
        if self.shadow.is_none() {
            return Ok(None);
        }

        let maps = graph.resource(
            ResourceDescriptor::Texture {
                width: shadow::SIZE,
                height: shadow::SIZE,
                layers: shadow::LAYERS,
                mips: 1,
                samples: 1,
                format: shadow::FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
            },
            ResourceScope::Transient,
        )?;
        for index in 0..shadow::LAYERS {
            let matrix = graph.resource(
                ResourceDescriptor::Buffer {
                    bytes: 64,
                    usage: wgpu::BufferUsages::UNIFORM,
                },
                ResourceScope::Imported { initialized: true },
            )?;
            let key = self.shadow_key(index);
            prepared_keys.insert(key);
            graph.pass(Pass {
                name: match index {
                    0 => "synthetic Sun shadow near",
                    1 => "synthetic Sun shadow far",
                    _ => "synthetic Sun horizon heights",
                }
                .into(),
                scope: PassScope::View(1),
                pipeline: Some(key),
                accesses: [
                    Access {
                        resource: instance.clone(),
                        range: 0..self.instance_bytes,
                        kind: AccessKind::StorageRead,
                        store: true,
                    },
                    Access {
                        resource: cube_indices.clone(),
                        range: 0..CUBE_INDEX_BYTES,
                        kind: AccessKind::IndexRead,
                        store: true,
                    },
                    Access {
                        resource: matrix,
                        range: 0..64,
                        kind: AccessKind::UniformRead,
                        store: true,
                    },
                    Access {
                        resource: maps.clone(),
                        range: u64::from(index)..u64::from(index + 1),
                        kind: AccessKind::AttachmentClear,
                        store: true,
                    },
                ]
                .into_iter()
                .chain(contribution_reads.iter().cloned())
                .collect(),
                external_side_effect: false,
            });
        }
        let sample = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: shadow::SAMPLE_BYTES as u64,
                usage: wgpu::BufferUsages::UNIFORM,
            },
            ResourceScope::Imported { initialized: true },
        )?;
        accesses.push(Access {
            resource: sample,
            range: 0..shadow::SAMPLE_BYTES as u64,
            kind: AccessKind::UniformRead,
            store: true,
        });
        accesses.push(Access {
            resource: maps.clone(),
            range: 0..u64::from(shadow::LAYERS),
            kind: AccessKind::SampledRead,
            store: true,
        });
        Ok(Some(maps))
    }
}
