use std::collections::BTreeSet;

use crate::graph::{
    Access, AccessKind, FrameGraph, GraphError, Pass, PassScope, PipelineKey, ResourceDescriptor,
    ResourceScope,
};

use super::gpu::{AERIAL_EXTENT, AtmospherePrepared, EXTENTS};

const LUT_OWNER_BASE: u64 = 0x4154_4d4f;
const AERIAL_OWNER_BASE: u64 = 0x4154_4d60;
const _: () = assert!(AERIAL_OWNER_BASE >= LUT_OWNER_BASE + 3);

impl AtmospherePrepared {
    pub(crate) fn append_graph(
        &self,
        graph: &mut FrameGraph,
        keys: &mut BTreeSet<PipelineKey>,
        forward_accesses: &mut Vec<Access>,
        directional: crate::graph::ResourceId,
    ) -> Result<(), GraphError> {
        let mut resources: Vec<crate::graph::ResourceId> = Vec::with_capacity(3);
        let uniform = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: 128,
                usage: wgpu::BufferUsages::UNIFORM,
            },
            ResourceScope::Imported { initialized: true },
        )?;
        for (index, [width, height]) in EXTENTS.into_iter().enumerate() {
            let resource = graph.resource(
                ResourceDescriptor::Texture {
                    width,
                    height,
                    layers: 1,
                    mips: 1,
                    samples: 1,
                    format: self.textures[index].format(),
                    usage: self.textures[index].usage(),
                },
                ResourceScope::Persistent {
                    owner: LUT_OWNER_BASE + index as u64,
                    epoch: self.epoch,
                    initialized: false,
                },
            )?;
            let mut accesses = vec![Access {
                resource: uniform.clone(),
                range: 0..128,
                kind: AccessKind::UniformRead,
                store: true,
            }];
            for input in &resources {
                accesses.push(Access {
                    resource: input.clone(),
                    range: 0..1,
                    kind: AccessKind::SampledRead,
                    store: true,
                });
            }
            accesses.push(Access {
                resource: resource.clone(),
                range: 0..1,
                kind: AccessKind::StorageWrite,
                store: true,
            });
            let key = self.key(u32::try_from(index).expect("three atmosphere LUTs"));
            keys.insert(key);
            graph.pass(Pass {
                name: [
                    "atmosphere transmittance",
                    "atmosphere multiple scattering",
                    "atmosphere sky view",
                ][index]
                    .into(),
                scope: PassScope::View(1),
                pipeline: Some(key),
                accesses,
                external_side_effect: false,
            });
            resources.push(resource);
        }
        let diffuse_key = self.key(3);
        keys.insert(diffuse_key);
        graph.pass(Pass {
            name: "atmosphere diffuse irradiance".into(),
            scope: PassScope::View(1),
            pipeline: Some(diffuse_key),
            accesses: vec![
                Access {
                    resource: uniform.clone(),
                    range: 0..128,
                    kind: AccessKind::UniformRead,
                    store: true,
                },
                Access {
                    resource: resources[2].clone(),
                    range: 0..1,
                    kind: AccessKind::SampledRead,
                    store: true,
                },
                Access {
                    resource: directional,
                    range: 48..64,
                    kind: AccessKind::StorageWrite,
                    store: true,
                },
            ],
            external_side_effect: false,
        });
        let aerial = self
            .aerial_textures
            .iter()
            .enumerate()
            .map(|(index, texture)| {
                graph.resource(
                    ResourceDescriptor::Texture {
                        width: AERIAL_EXTENT[0],
                        height: AERIAL_EXTENT[1],
                        layers: AERIAL_EXTENT[2],
                        mips: 1,
                        samples: 1,
                        format: texture.format(),
                        usage: texture.usage(),
                    },
                    ResourceScope::Persistent {
                        owner: AERIAL_OWNER_BASE + index as u64,
                        epoch: self.epoch,
                        initialized: false,
                    },
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let aerial_key = self.key(4);
        keys.insert(aerial_key);
        graph.pass(Pass {
            name: "atmosphere aerial perspective".into(),
            scope: PassScope::View(1),
            pipeline: Some(aerial_key),
            accesses: [
                Access {
                    resource: uniform.clone(),
                    range: 0..128,
                    kind: AccessKind::UniformRead,
                    store: true,
                },
                Access {
                    resource: resources[0].clone(),
                    range: 0..1,
                    kind: AccessKind::SampledRead,
                    store: true,
                },
                Access {
                    resource: resources[1].clone(),
                    range: 0..1,
                    kind: AccessKind::SampledRead,
                    store: true,
                },
                Access {
                    resource: aerial[0].clone(),
                    range: 0..u64::from(AERIAL_EXTENT[2]),
                    kind: AccessKind::StorageWrite,
                    store: true,
                },
                Access {
                    resource: aerial[1].clone(),
                    range: 0..u64::from(AERIAL_EXTENT[2]),
                    kind: AccessKind::StorageWrite,
                    store: true,
                },
            ]
            .to_vec(),
            external_side_effect: false,
        });
        forward_accesses.push(Access {
            resource: uniform,
            range: 0..128,
            kind: AccessKind::UniformRead,
            store: true,
        });
        for index in [0, 2] {
            forward_accesses.push(Access {
                resource: resources[index].clone(),
                range: 0..1,
                kind: AccessKind::SampledRead,
                store: true,
            });
        }
        for resource in aerial {
            forward_accesses.push(Access {
                resource,
                range: 0..u64::from(AERIAL_EXTENT[2]),
                kind: AccessKind::SampledRead,
                store: true,
            });
        }
        Ok(())
    }
}
