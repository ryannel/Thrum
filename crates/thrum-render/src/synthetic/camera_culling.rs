//! Camera culling glue between the prepared synthetic scene and the private
//! GPU visibility module: graph declarations, the compute record, the opaque
//! draw and the forward view-group binding of the compacted list.

use std::collections::BTreeSet;

use super::{
    OffscreenTarget, PreparedSynthetic, RenderError, SYNTHETIC_REVISION, SceneContext, VIEW_BYTES,
    history_gpu, storage_layout,
};
use crate::graph::{
    Access, AccessKind, CompiledPass, FrameGraph, GraphError, PipelineKey, ResourceDescriptor,
    ResourceId, ResourceScope,
};
pub use crate::visibility::CullMode;
use crate::visibility::{self, MeshRecord, VisibilityPrepared, VisibilityTarget};
pub use stats::CullingCounts;
pub(super) use stats::{CullingStats, STATS_PASS};

mod stats;

/// Stable mesh row 0: the diagnostic cube's index range and local bounds.
const CUBE_MESH: MeshRecord = MeshRecord {
    first_index: 0,
    index_count: 36,
    base_vertex: 0,
    bounds_min: [-1.0; 3],
    bounds_max: [1.0; 3],
};
/// Forward view-group binding of the camera's visible slot list.
pub(super) const VISIBLE_BINDING: u32 = 5;

/// Visibility pipelines for this scene's slot capacity and the cube mesh row.
pub(super) fn prepare(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    generation: u64,
    mode: CullMode,
    capacity: u32,
    active: u32,
) -> Option<VisibilityPrepared> {
    if mode == CullMode::Direct {
        return None;
    }
    let key = PipelineKey {
        device_generation: generation,
        revision: SYNTHETIC_REVISION,
        role: visibility::ROLE,
    };
    Some(VisibilityPrepared::new(
        device,
        queue,
        key,
        capacity,
        &[CUBE_MESH],
        active,
    ))
}

/// Frustum only: the forward view-group entry for the compacted slot list.
pub(super) fn view_layout_entry(mode: CullMode) -> Option<wgpu::BindGroupLayoutEntry> {
    (mode == CullMode::Frustum)
        .then(|| storage_layout(VISIBLE_BINDING, wgpu::ShaderStages::VERTEX, 4))
}

pub(super) fn visible_entry(target: Option<&VisibilityTarget>) -> Option<wgpu::BindGroupEntry<'_>> {
    target.map(|target| wgpu::BindGroupEntry {
        binding: VISIBLE_BINDING,
        resource: target.visible().as_entire_binding(),
    })
}

/// Forward vertex entry: Direct reads the physical slot with no list fetch.
pub(super) fn vertex_entry(mode: CullMode) -> &'static str {
    match mode {
        CullMode::Direct => "vertex",
        CullMode::Frustum => "compacted_vertex",
    }
}

impl PreparedSynthetic {
    /// Frustum only: declare the compute pass, the untimed copy of its drawn
    /// count and the forward reads of its visible list (storage) and
    /// arguments (indirect).
    pub(super) fn append_camera_visibility(
        &self,
        graph: &mut FrameGraph,
        view: ResourceId,
        instance: &ResourceId,
        accesses: &mut Vec<Access>,
        keys: &mut BTreeSet<PipelineKey>,
    ) -> Result<(), GraphError> {
        let Some(prepared) = &self.visibility else {
            return Ok(());
        };
        // The cull reads the same submitted-history serials as the forward
        // vertex stage (presentation alpha) through its own uniform binding.
        let serials = graph.resource(
            ResourceDescriptor::Buffer {
                bytes: history_gpu::SERIAL_BYTES,
                usage: wgpu::BufferUsages::UNIFORM,
            },
            ResourceScope::Imported { initialized: true },
        )?;
        let reads = vec![
            Access {
                resource: view,
                range: 0..VIEW_BYTES as u64,
                kind: AccessKind::UniformRead,
                store: true,
            },
            Access {
                resource: instance.clone(),
                range: 0..self.instance_bytes,
                kind: AccessKind::StorageRead,
                store: true,
            },
            Access {
                resource: serials,
                range: 0..history_gpu::SERIAL_BYTES,
                kind: AccessKind::UniformRead,
                store: true,
            },
        ];
        let outputs = prepared.append_graph(graph, reads)?;
        keys.insert(prepared.key());
        Self::append_camera_stats(graph, &outputs.args)?;
        accesses.extend([
            Access {
                resource: outputs.visible,
                range: 0..prepared.visible_bytes(),
                kind: AccessKind::StorageRead,
                store: true,
            },
            Access {
                resource: outputs.args,
                range: 0..visibility::ARGS_BYTES,
                kind: AccessKind::IndirectRead,
                store: true,
            },
        ]);
        Ok(())
    }

    pub(super) fn draw_camera(&self, pass: &mut wgpu::RenderPass<'_>, target: &OffscreenTarget) {
        match &target.visibility {
            Some(culled) if self.visibility.is_some() => {
                pass.draw_indexed_indirect(culled.args(), 0);
            }
            _ => pass.draw_indexed(0..36, 0, 0..self.instance_upload.active_count()),
        }
    }

    pub(super) fn camera_visibility_target(
        &self,
        device: &wgpu::Device,
    ) -> Option<VisibilityTarget> {
        self.visibility.as_ref().map(|prepared| {
            prepared.target(
                device,
                &self.view_buffer,
                self.instance_upload.buffer(),
                self.history.serial_buffer(),
            )
        })
    }
}

impl SceneContext<'_> {
    pub(super) fn visibility(&mut self, pass: &CompiledPass) -> Result<(), RenderError> {
        let view_offset =
            self.prepared.view_stride * u32::from(self.target.frame_revision & 1 != 0);
        let (Some(prepared), Some(target)) = (&self.prepared.visibility, &self.target.visibility)
        else {
            return Err(RenderError::GraphPassMismatch);
        };
        let encoded = prepared.encode(
            self.encoder,
            target,
            view_offset,
            self.generation,
            pass,
            self.timings.as_deref_mut(),
        )?;
        encoded.then_some(()).ok_or(RenderError::GraphPassMismatch)
    }
}
