//! Local-light world records are shared; the view owns only its projected
//! bounds, cluster table and dispatch. The same builder serves the main view.

use super::{
    Access, AccessKind, BTreeSet, CompiledPass, FrameGraph, PassScope, PipelineKey,
    PreparedSynthetic, RenderError, ResourceDescriptor, ResourceScope, SecondaryViewError,
    SyntheticView,
};
use crate::SharedGpu;
use crate::graph::PassBody;
use crate::pass_key::{PassKey, SecondaryStage};
use crate::synthetic::{SyntheticPointLight, cluster::ClusterTable, cluster_gpu::ClusterCompute};

pub(super) const PASS: &str = "secondary view clusters";

#[derive(Clone)]
pub(super) struct Lighting {
    pub(super) groups: [wgpu::BindGroup; 2],
    state: SharedGpu<ClusterState>,
    pub(super) gpu: bool,
}

struct ClusterState {
    table: ClusterTable,
    heads: wgpu::Buffer,
    indices: wgpu::Buffer,
    compute: Option<ClusterCompute>,
}

impl Lighting {
    pub(super) fn new(
        prepared: &PreparedSynthetic,
        queue: &wgpu::Queue,
        sources: &[SyntheticPointLight],
        aerial: Option<crate::synthetic::forward::AerialBindings<'_>>,
    ) -> Result<Self, SecondaryViewError> {
        let device = prepared.capabilities.device();
        let gpu = (1..=crate::synthetic::cluster::GPU_MAX_SOURCES).contains(&sources.len());
        let table = if gpu {
            ClusterTable::build_gpu(sources, &prepared.view_sample)
        } else {
            ClusterTable::build(sources, &prepared.view_sample)
        }
        .map_err(|_| SecondaryViewError::LightingLimit)?;
        let [_, head_bytes, index_bytes] = table.capacities();
        if head_bytes
            > prepared
                .capabilities
                .limits()
                .max_storage_buffer_binding_size
            || index_bytes
                > prepared
                    .capabilities
                    .limits()
                    .max_storage_buffer_binding_size
        {
            return Err(SecondaryViewError::LightingLimit);
        }
        let heads = crate::synthetic::storage_buffer(device, queue, PASS, &table.heads, head_bytes);
        let indices =
            crate::synthetic::storage_buffer(device, queue, PASS, &table.indices, index_bytes);
        let compute = gpu.then(|| {
            let bounds = crate::synthetic::storage_buffer(
                device,
                queue,
                PASS,
                &table.bounds,
                table.bounds.len() as u64,
            );
            ClusterCompute::new(device, bounds, &heads, &indices)
        });
        let groups = crate::synthetic::forward::LightBindings {
            directional: &prepared.directional_buffer,
            environment_light: &prepared.environment_light,
            local: &prepared.local_light_buffer,
            heads: &heads,
            indices: &indices,
            atmosphere: prepared.atmosphere.as_ref(),
            cloud: prepared.cloud.as_ref(),
            aerial,
        }
        .bind(
            device,
            &prepared.map_light_layout,
            &prepared.exposure.samples,
            None,
            None,
            None,
            None,
        );
        Ok(Self {
            groups,
            state: SharedGpu::new(ClusterState {
                table,
                heads,
                indices,
                compute,
            }),
            gpu,
        })
    }

    pub(super) fn update(
        &self,
        queue: &wgpu::Queue,
        sources: &[SyntheticPointLight],
        view: &SyntheticView,
    ) -> Result<(), RenderError> {
        let mut state = self.state.write();
        if self.gpu {
            state.table.repack_gpu(sources, view)?;
        } else {
            state.table.rebuild(sources, view)?;
        }
        if let Some(compute) = &state.compute {
            queue.write_buffer(&compute.bounds, 0, &state.table.bounds);
        } else {
            queue.write_buffer(&state.heads, 0, &state.table.heads);
            queue.write_buffer(&state.indices, 0, &state.table.indices);
        }
        Ok(())
    }

    pub(super) fn append(
        &self,
        graph: &mut FrameGraph,
        keys: &mut BTreeSet<PipelineKey>,
        key: PipelineKey,
        index: usize,
        shared: &[Access],
        opaque: &mut Vec<Access>,
    ) -> Result<(), RenderError> {
        let state = self.state.write();
        let mut writes = Vec::new();
        for buffer in [&state.heads, &state.indices] {
            let id = graph.resource(
                ResourceDescriptor::Buffer {
                    bytes: buffer.size(),
                    usage: buffer.usage(),
                },
                ResourceScope::Imported { initialized: true },
            )?;
            opaque.push(Access::new(
                id.clone(),
                0..buffer.size(),
                AccessKind::StorageRead,
            ));
            writes.push(Access::new(id, 0..buffer.size(), AccessKind::StorageWrite));
        }
        if let Some(compute) = &state.compute {
            let bounds = graph.resource(
                ResourceDescriptor::Buffer {
                    bytes: compute.bounds.size(),
                    usage: compute.bounds.usage(),
                },
                ResourceScope::Imported { initialized: true },
            )?;
            writes.push(Access::new(
                bounds,
                0..compute.bounds.size(),
                AccessKind::StorageRead,
            ));
            // Uploaded scene inputs and simulate products precede the view's work.
            writes.extend_from_slice(shared);
            keys.insert(key);
            graph.declare(
                PassKey::Secondary {
                    view: crate::pass_key::pass_index(index),
                    stage: SecondaryStage::Clusters,
                },
                PassBody {
                    scope: PassScope::View(index as u64 + 2),
                    pipeline: Some(key),
                    accesses: writes,
                    external_side_effect: false,
                },
            );
        }
        Ok(())
    }

    pub(super) fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        generation: u64,
        pass: &CompiledPass,
        timings: Option<&mut crate::timing::GpuTimings>,
    ) -> Result<(), RenderError> {
        let state = self.state.write();
        state
            .compute
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?
            .encode(encoder, generation, pass, timings)
    }
}
