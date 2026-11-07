use std::collections::BTreeSet;

use crate::graph::{
    Access, AccessKind, FrameGraph, GraphError, Pass, PassScope, PipelineKey, ResourceDescriptor,
    ResourceScope,
};

use super::{
    STAGE_NAMES,
    gpu::{AERIAL_EXTENT, AtmospherePrepared, UNIFORM_BYTES},
    reflection::{CUBE_SIZE, DFG_SIZE, MIP_COUNT, SAMPLE_BYTES},
};

const LUT_OWNER_BASE: u64 = 0x4154_4d4f;
const AERIAL_OWNER_BASE: u64 = 0x4154_4d60;
const REFLECTION_OWNER_BASE: u64 = 0x4154_4d70;
const _: () = assert!(AERIAL_OWNER_BASE >= LUT_OWNER_BASE + 4);

impl AtmospherePrepared {
    pub(crate) fn append_graph(
        &self,
        graph: &mut FrameGraph,
        keys: &mut BTreeSet<PipelineKey>,
        forward_accesses: &mut Vec<Access>,
        directional: crate::graph::ResourceId,
    ) -> Result<(), GraphError> {
        let uniform = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: UNIFORM_BYTES,
                usage: wgpu::BufferUsages::UNIFORM,
            },
            ResourceScope::Imported { initialized: true },
        )?;
        let resources = self
            .textures
            .iter()
            .enumerate()
            .map(|(index, texture)| {
                graph.resource(
                    ResourceDescriptor::Texture {
                        width: texture.width(),
                        height: texture.height(),
                        layers: 1,
                        mips: 1,
                        samples: 1,
                        format: texture.format(),
                        usage: texture.usage(),
                    },
                    ResourceScope::Persistent {
                        owner: LUT_OWNER_BASE + index as u64,
                        epoch: self.epoch,
                        initialized: false,
                    },
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        // Resource indices remain T=0,M=1,Sky=2; GroundE=3 is appended. Its
        // producer runs before Sky and has no dependency on the camera sky.
        for index in [0_usize, 1, 3, 2] {
            let mut accesses = vec![Access {
                resource: uniform.clone(),
                range: 0..UNIFORM_BYTES,
                kind: AccessKind::UniformRead,
                store: true,
            }];
            let inputs: &[usize] = match index {
                0 => &[],
                1 => &[0],
                2 => &[0, 1, 3],
                _ => &[0, 1],
            };
            for &input in inputs {
                accesses.push(Access {
                    resource: resources[input].clone(),
                    range: 0..1,
                    kind: AccessKind::SampledRead,
                    store: true,
                });
            }
            accesses.push(Access {
                resource: resources[index].clone(),
                range: 0..1,
                kind: AccessKind::StorageWrite,
                store: true,
            });
            let role = if index == 3 {
                5
            } else {
                u32::try_from(index).expect("three original atmosphere LUTs")
            };
            let key = self.key(role);
            keys.insert(key);
            graph.pass(Pass {
                name: STAGE_NAMES[usize::try_from(role).expect("prepared atmosphere stage")].into(),
                scope: PassScope::View(1),
                pipeline: Some(key),
                accesses,
                external_side_effect: false,
            });
        }
        let diffuse_key = self.key(3);
        keys.insert(diffuse_key);
        graph.pass(Pass {
            name: STAGE_NAMES[3].into(),
            scope: PassScope::View(1),
            pipeline: Some(diffuse_key),
            accesses: vec![
                Access {
                    resource: uniform.clone(),
                    range: 0..UNIFORM_BYTES,
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
                    range: 48..crate::synthetic::LIGHT_BYTES as u64,
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
            name: STAGE_NAMES[4].into(),
            scope: PassScope::View(1),
            pipeline: Some(aerial_key),
            accesses: [
                Access {
                    resource: uniform.clone(),
                    range: 0..UNIFORM_BYTES,
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
        self.append_reflection(graph, keys, &uniform, &resources[2], forward_accesses)?;
        forward_accesses.push(Access {
            resource: uniform,
            range: 0..UNIFORM_BYTES,
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

    fn append_reflection(
        &self,
        graph: &mut FrameGraph,
        keys: &mut BTreeSet<PipelineKey>,
        uniform: &crate::graph::ResourceId,
        sky: &crate::graph::ResourceId,
        forward: &mut Vec<Access>,
    ) -> Result<(), GraphError> {
        let resources = [
            (&self.reflection.dfg, DFG_SIZE, 1, 1, self.generation),
            (&self.reflection.cube, CUBE_SIZE, 6, MIP_COUNT, self.epoch),
        ]
        .into_iter()
        .enumerate()
        .map(|(index, (texture, size, layers, mips, epoch))| {
            graph.resource(
                ResourceDescriptor::Texture {
                    width: size,
                    height: size,
                    layers,
                    mips,
                    samples: 1,
                    format: texture.format(),
                    usage: texture.usage(),
                },
                ResourceScope::Persistent {
                    owner: REFLECTION_OWNER_BASE + index as u64,
                    epoch,
                    initialized: false,
                },
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
        let dfg_key = self.key(6);
        let samples = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: SAMPLE_BYTES,
                usage: wgpu::BufferUsages::STORAGE,
            },
            ResourceScope::Persistent {
                owner: REFLECTION_OWNER_BASE + 2,
                epoch: self.generation,
                initialized: false,
            },
        )?;
        keys.insert(dfg_key);
        graph.pass(Pass {
            name: STAGE_NAMES[6].into(),
            scope: PassScope::View(1),
            pipeline: Some(dfg_key),
            accesses: vec![
                Access {
                    resource: resources[0].clone(),
                    range: 0..1,
                    kind: AccessKind::StorageWrite,
                    store: true,
                },
                Access {
                    resource: samples.clone(),
                    range: 0..SAMPLE_BYTES,
                    kind: AccessKind::StorageWrite,
                    store: true,
                },
            ],
            external_side_effect: false,
        });
        for mip in 0..MIP_COUNT {
            let stage = 7 + mip;
            let key = self.key(stage);
            keys.insert(key);
            graph.pass(Pass {
                name: STAGE_NAMES[usize::try_from(stage).expect("prepared reflection stage")]
                    .into(),
                scope: PassScope::View(1),
                pipeline: Some(key),
                accesses: vec![
                    Access {
                        resource: samples.clone(),
                        range: 0..SAMPLE_BYTES,
                        kind: AccessKind::StorageRead,
                        store: true,
                    },
                    Access {
                        resource: uniform.clone(),
                        range: 0..UNIFORM_BYTES,
                        kind: AccessKind::UniformRead,
                        store: true,
                    },
                    Access {
                        resource: sky.clone(),
                        range: 0..1,
                        kind: AccessKind::SampledRead,
                        store: true,
                    },
                    Access {
                        resource: resources[1].clone(),
                        range: u64::from(mip * 6)..u64::from((mip + 1) * 6),
                        kind: AccessKind::StorageWrite,
                        store: true,
                    },
                ],
                external_side_effect: false,
            });
        }
        for (resource, end) in [
            (resources[0].clone(), 1),
            (resources[1].clone(), u64::from(MIP_COUNT * 6)),
        ] {
            forward.push(Access {
                resource,
                range: 0..end,
                kind: AccessKind::SampledRead,
                store: true,
            });
        }
        Ok(())
    }
}
