use std::collections::BTreeSet;

use crate::graph::{
    Access, AccessKind, FrameGraph, GraphError, PassBody, PassScope, PipelineKey,
    ResourceDescriptor, ResourceId, ResourceScope,
};
use crate::pass_key::{AtmosphereStage, PassKey};

use super::{
    gpu::{AERIAL_EXTENT, AtmospherePrepared, SKY_MOTION_BYTES, UNIFORM_BYTES},
    reflection::{CUBE_SIZE, DFG_SIZE, MIP_COUNT, SAMPLE_BYTES},
    stage::{Chart, Charts},
};

const LUT_OWNER_BASE: u64 = 0x4154_4d4f;
const AERIAL_OWNER_BASE: u64 = 0x4154_4d60;
const REFLECTION_OWNER_BASE: u64 = 0x4154_4d70;
const _: () = assert!(AERIAL_OWNER_BASE >= LUT_OWNER_BASE + 4);

/// The chart whose persistent graph resource has `owner`.
pub(crate) fn chart_of_owner(owner: u64) -> Option<Chart> {
    [
        Chart::Transmittance,
        Chart::MultipleScattering,
        Chart::SkyView,
        Chart::Ground,
    ]
    .into_iter()
    .find(|chart| LUT_OWNER_BASE + chart.owner_offset() == owner)
}

impl AtmospherePrepared {
    pub(crate) fn append_graph(
        &self,
        graph: &mut FrameGraph,
        keys: &mut BTreeSet<PipelineKey>,
        forward_accesses: &mut Vec<Access>,
        environment_light: ResourceId,
    ) -> Result<(ResourceId, ResourceId), GraphError> {
        let uniform = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: UNIFORM_BYTES,
                usage: wgpu::BufferUsages::UNIFORM,
            },
            ResourceScope::Imported { initialized: true },
        )?;
        append_motion_read(graph, forward_accesses)?;
        let night_reads = self.night.graph_reads(graph)?;
        let resources = Charts::try_from_fn(|chart| {
            let texture = self.textures.get(chart);
            graph.resource(
                ResourceDescriptor::Texture {
                    dimension: crate::graph::TextureKind::D2,
                    width: texture.width(),
                    height: texture.height(),
                    layers: 1,
                    mips: 1,
                    samples: 1,
                    format: texture.format(),
                    usage: texture.usage(),
                },
                ResourceScope::Persistent {
                    owner: LUT_OWNER_BASE + chart.owner_offset(),
                    epoch: self.epoch,
                    initialized: false,
                },
            )
        })?;
        // The four chart stages, in graph order: ground runs before the sky view,
        // which samples it, and has no dependency on the camera sky.
        for stage in AtmosphereStage::ORDER {
            let Some(output) = stage.chart_output() else {
                continue;
            };
            let mut accesses = vec![Access::new(
                uniform.clone(),
                0..UNIFORM_BYTES,
                AccessKind::UniformRead,
            )];
            accesses.extend(chart_reads(stage, &resources));
            if stage == AtmosphereStage::SkyView {
                accesses.extend(night_reads.iter().cloned());
            }
            accesses.push(Access::new(
                resources.get(output).clone(),
                0..1,
                AccessKind::StorageWrite,
            ));
            self.declare_stage(graph, keys, stage, accesses);
        }
        let diffuse = AtmosphereStage::Diffuse;
        let mut accesses = vec![Access::new(
            uniform.clone(),
            0..UNIFORM_BYTES,
            AccessKind::UniformRead,
        )];
        accesses.extend(chart_reads(diffuse, &resources));
        accesses.push(Access::new(
            environment_light,
            0..crate::light::ENVIRONMENT_BYTES,
            AccessKind::StorageWrite,
        ));
        self.declare_stage(graph, keys, diffuse, accesses);
        let aerial = self
            .aerial_textures
            .iter()
            .enumerate()
            .map(|(index, texture)| {
                graph.resource(
                    ResourceDescriptor::Texture {
                        dimension: crate::graph::TextureKind::D2,
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
        let stage = AtmosphereStage::Aerial;
        let accesses = [Access::new(
            uniform.clone(),
            0..UNIFORM_BYTES,
            AccessKind::UniformRead,
        )]
        .into_iter()
        .chain(chart_reads(stage, &resources))
        .chain([
            Access::new(
                aerial[0].clone(),
                0..u64::from(AERIAL_EXTENT[2]),
                AccessKind::StorageWrite,
            ),
            Access::new(
                aerial[1].clone(),
                0..u64::from(AERIAL_EXTENT[2]),
                AccessKind::StorageWrite,
            ),
        ])
        .chain(night_reads)
        .collect();
        self.declare_stage(graph, keys, stage, accesses);
        self.append_reflection(graph, keys, &uniform, &resources, forward_accesses)?;
        forward_accesses.push(Access::new(
            uniform.clone(),
            0..UNIFORM_BYTES,
            AccessKind::UniformRead,
        ));
        for chart in [Chart::Transmittance, Chart::SkyView] {
            forward_accesses.push(Access::new(
                resources.get(chart).clone(),
                0..1,
                AccessKind::SampledRead,
            ));
        }
        for resource in aerial {
            forward_accesses.push(Access::new(
                resource,
                0..u64::from(AERIAL_EXTENT[2]),
                AccessKind::SampledRead,
            ));
        }
        Ok((resources.transmittance.clone(), uniform))
    }

    fn declare_stage(
        &self,
        graph: &mut FrameGraph,
        keys: &mut BTreeSet<PipelineKey>,
        stage: AtmosphereStage,
        accesses: Vec<Access>,
    ) {
        let key = self.key(stage);
        keys.insert(key);
        graph.declare(
            PassKey::Atmosphere(stage),
            PassBody {
                scope: PassScope::View(1),
                pipeline: Some(key),
                accesses,
                external_side_effect: false,
            },
        );
    }

    fn append_reflection(
        &self,
        graph: &mut FrameGraph,
        keys: &mut BTreeSet<PipelineKey>,
        uniform: &ResourceId,
        charts: &Charts<ResourceId>,
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
                    dimension: crate::graph::TextureKind::D2,
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
        let dfg = AtmosphereStage::Dfg;
        let accesses = chart_reads(dfg, charts)
            .chain([
                Access::new(resources[0].clone(), 0..1, AccessKind::StorageWrite),
                Access::new(samples.clone(), 0..SAMPLE_BYTES, AccessKind::StorageWrite),
            ])
            .collect();
        self.declare_stage(graph, keys, dfg, accesses);
        for mip in 0..MIP_COUNT {
            let stage = AtmosphereStage::Reflection(mip);
            let accesses = [
                Access::new(samples.clone(), 0..SAMPLE_BYTES, AccessKind::StorageRead),
                Access::new(uniform.clone(), 0..UNIFORM_BYTES, AccessKind::UniformRead),
            ]
            .into_iter()
            .chain(chart_reads(stage, charts))
            .chain([Access::new(
                resources[1].clone(),
                u64::from(mip * 6)..u64::from((mip + 1) * 6),
                AccessKind::StorageWrite,
            )])
            .collect();
            self.declare_stage(graph, keys, stage, accesses);
        }
        for (resource, end) in [
            (resources[0].clone(), 1),
            (resources[1].clone(), u64::from(MIP_COUNT * 6)),
        ] {
            forward.push(Access::new(resource, 0..end, AccessKind::SampledRead));
        }
        Ok(())
    }
}

fn append_motion_read(
    graph: &mut FrameGraph,
    forward_accesses: &mut Vec<Access>,
) -> Result<(), GraphError> {
    let motion = graph.resource(
        ResourceDescriptor::Buffer {
            bytes: SKY_MOTION_BYTES,
            usage: wgpu::BufferUsages::UNIFORM,
        },
        ResourceScope::Imported { initialized: true },
    )?;
    forward_accesses.push(Access::new(
        motion,
        0..SKY_MOTION_BYTES,
        AccessKind::UniformRead,
    ));
    Ok(())
}

/// Graph reads of the charts `stage` samples, for a pass declared outside the
/// atmosphere's own graph section, such as a secondary view's aerial volume.
/// Accesses in `inherited` that touch any chart are replaced by the table's
/// reads, so the pass reads exactly the charts its bind group samples.
pub(crate) fn stage_reads(
    graph: &FrameGraph,
    stage: AtmosphereStage,
    inherited: &[Access],
) -> Result<Vec<Access>, crate::synthetic::RenderError> {
    // The atmosphere declares its charts before any view that samples them.
    let charts = Charts::try_from_fn(|chart| {
        graph
            .persistent(LUT_OWNER_BASE + chart.owner_offset())
            .ok_or(crate::synthetic::RenderError::GraphPassMismatch)
    })?;
    let is_chart = |access: &Access| {
        [
            &charts.transmittance,
            &charts.multiple_scattering,
            &charts.sky_view,
            &charts.ground,
        ]
        .contains(&&access.resource)
    };
    Ok(inherited
        .iter()
        .filter(|access| !is_chart(access))
        .cloned()
        .chain(chart_reads(stage, &charts))
        .collect())
}

/// Graph reads of the charts `stage` samples, from the same table its bind
/// group is built from.
fn chart_reads(
    stage: AtmosphereStage,
    charts: &Charts<ResourceId>,
) -> impl Iterator<Item = Access> + '_ {
    stage.chart_reads().iter().map(|read| {
        Access::new(
            charts.get(read.chart).clone(),
            0..1,
            AccessKind::SampledRead,
        )
    })
}
