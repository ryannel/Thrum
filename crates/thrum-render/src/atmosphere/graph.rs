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

pub(crate) fn overlay_reads(
    graph: &FrameGraph,
    transmittance: &ResourceId,
    uniform: &ResourceId,
) -> Result<[Access; 3], crate::error::RenderError> {
    let sky = graph
        .persistent(LUT_OWNER_BASE + Chart::SkyView.owner_offset())
        .ok_or(crate::error::RenderError::GraphPassMismatch)?;
    Ok([
        Access::new(sky, 0..1, AccessKind::SampledRead),
        Access::new(transmittance.clone(), 0..1, AccessKind::SampledRead),
        Access::new(uniform.clone(), 0..UNIFORM_BYTES, AccessKind::UniformRead),
    ])
}

impl AtmospherePrepared {
    pub(crate) fn terminal_read(&self, graph: &FrameGraph) -> Option<Access> {
        self.view_volume.as_ref()?;
        let resource = graph.persistent(AERIAL_OWNER_BASE)?;
        let end = u64::from(self.aerial_textures[0].depth_or_array_layers());
        Some(Access::new(resource, end - 1..end, AccessKind::SampledRead))
    }

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
        let aerosol_read = self
            .aerosol
            .as_ref()
            .map(|buffer| {
                graph
                    .resource(
                        ResourceDescriptor::Buffer {
                            bytes: buffer.size(),
                            usage: wgpu::BufferUsages::UNIFORM,
                        },
                        ResourceScope::Imported { initialized: true },
                    )
                    .map(|resource| {
                        Access::new(resource, 0..buffer.size(), AccessKind::UniformRead)
                    })
            })
            .transpose()?;
        let resources = Charts::try_from_fn(|chart| {
            let texture = self.textures.get(chart);
            graph.resource(
                ResourceDescriptor::of_texture(texture),
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
            accesses.extend(aerosol_read.iter().cloned());
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
        if self.overlay.is_none() {
            self.append_diffuse(
                graph,
                keys,
                &uniform,
                &resources,
                environment_light,
                None,
                Vec::new(),
            );
        }
        let aerial = self
            .aerial_textures
            .iter()
            .enumerate()
            .map(|(index, texture)| {
                graph.resource(
                    ResourceDescriptor::Texture {
                        dimension: crate::graph::TextureKind::D2,
                        width: texture.width(),
                        height: texture.height(),
                        layers: texture.depth_or_array_layers(),
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
        .chain(aerosol_read)
        .collect();
        if self.view_volume.is_none() {
            self.declare_stage(graph, keys, stage, accesses);
        }
        if self.overlay.is_none() {
            self.append_reflection(graph, keys, &uniform, &resources, forward_accesses, None)?;
        }
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
        for (resource, texture) in aerial.into_iter().zip(self.aerial_textures.iter()) {
            forward_accesses.push(Access::new(
                resource,
                0..u64::from(texture.depth_or_array_layers()),
                AccessKind::SampledRead,
            ));
        }
        Ok((resources.transmittance.clone(), uniform))
    }

    pub(crate) fn append_overlay_lighting(
        &self,
        graph: &mut FrameGraph,
        keys: &mut BTreeSet<PipelineKey>,
        forward: &mut Vec<Access>,
        uniform: &ResourceId,
        environment_light: &ResourceId,
        produced: Option<Access>,
        sun_reads: Vec<Access>,
    ) -> Result<(), crate::error::RenderError> {
        let overlay = self
            .overlay
            .as_ref()
            .ok_or(crate::error::RenderError::GraphPassMismatch)?;
        let charts = Charts::try_from_fn(|chart: Chart| {
            graph
                .persistent(LUT_OWNER_BASE + chart.owner_offset())
                .ok_or(crate::error::RenderError::GraphPassMismatch)
        })?;
        let read = if let Some(produced) = produced {
            produced
        } else {
            let resource = graph.resource(
                ResourceDescriptor::of_texture(&overlay.texture),
                ResourceScope::Imported { initialized: true },
            )?;
            Access::new(resource, 0..6, AccessKind::SampledRead)
        };
        self.append_diffuse(
            graph,
            keys,
            uniform,
            &charts,
            environment_light.clone(),
            Some(read.clone()),
            sun_reads,
        );
        self.append_reflection(graph, keys, uniform, &charts, forward, Some(&read))?;
        Ok(())
    }

    fn append_diffuse(
        &self,
        graph: &mut FrameGraph,
        keys: &mut BTreeSet<PipelineKey>,
        uniform: &ResourceId,
        charts: &Charts<ResourceId>,
        environment_light: ResourceId,
        overlay: Option<Access>,
        mut sun_reads: Vec<Access>,
    ) {
        let diffuse = AtmosphereStage::Diffuse;
        let mut accesses = vec![Access::new(
            uniform.clone(),
            0..UNIFORM_BYTES,
            AccessKind::UniformRead,
        )];
        accesses.extend(chart_reads(diffuse, charts));
        accesses.extend(overlay);
        accesses.append(&mut sun_reads);
        accesses.push(Access::new(
            environment_light,
            0..crate::light::ENVIRONMENT_BYTES,
            AccessKind::StorageWrite,
        ));
        self.declare_stage(graph, keys, diffuse, accesses);
    }

    pub(crate) fn append_view_volume(
        &self,
        graph: &mut FrameGraph,
        keys: &mut BTreeSet<PipelineKey>,
        uniform: &ResourceId,
        shadow: Option<(&ResourceId, u32, u64)>,
        cloud: Option<&super::CloudPrepared>,
        sun_path_reads: Option<&[Access; 2]>,
        medium_read: Option<Access>,
        ground_band_read: Option<&Access>,
    ) -> Result<(), crate::error::RenderError> {
        use crate::error::RenderError;
        let Some(volume) = &self.view_volume else {
            return Ok(());
        };
        let stage = AtmosphereStage::Aerial;
        let mut accesses = stage_reads(graph, stage, &[])?;
        accesses.push(Access::new(
            uniform.clone(),
            0..UNIFORM_BYTES,
            AccessKind::UniformRead,
        ));
        accesses.extend(self.night.graph_reads(graph)?);
        let aerosol = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: super::gpu::aerosol::BYTES,
                usage: wgpu::BufferUsages::UNIFORM,
            },
            ResourceScope::Imported { initialized: true },
        )?;
        accesses.push(Access::new(
            aerosol,
            0..super::gpu::aerosol::BYTES,
            AccessKind::UniformRead,
        ));
        accesses.extend(volume.graph_reads_and_writes(graph)?);
        accesses.extend(volume.medium_read(graph, medium_read)?);
        for (index, texture) in self.aerial_textures.iter().enumerate() {
            let resource = graph
                .persistent(AERIAL_OWNER_BASE + index as u64)
                .ok_or(RenderError::GraphPassMismatch)?;
            accesses.push(Access::new(
                resource,
                0..u64::from(texture.depth_or_array_layers()),
                AccessKind::StorageWrite,
            ));
        }
        if let Some((maps, layers, bytes)) = shadow {
            let samples = graph.resource(
                ResourceDescriptor::Buffer {
                    bytes,
                    usage: wgpu::BufferUsages::UNIFORM,
                },
                ResourceScope::Imported { initialized: true },
            )?;
            accesses.push(Access::new(samples, 0..bytes, AccessKind::UniformRead));
            accesses.push(Access::new(
                maps.clone(),
                0..u64::from(layers),
                AccessKind::SampledRead,
            ));
        }
        if let Some(cloud) = cloud {
            cloud.append_reads(graph, &mut accesses, sun_path_reads, ground_band_read)?;
        }
        self.declare_stage(graph, keys, stage, accesses);
        Ok(())
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
        overlay: Option<&Access>,
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
            .chain(overlay.cloned())
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
) -> Result<Vec<Access>, crate::error::RenderError> {
    // The atmosphere declares its charts before any view that samples them.
    let charts = Charts::try_from_fn(|chart| {
        graph
            .persistent(LUT_OWNER_BASE + chart.owner_offset())
            .ok_or(crate::error::RenderError::GraphPassMismatch)
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
