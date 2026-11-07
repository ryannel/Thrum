//! Camera culling glue between the prepared synthetic scene and the private
//! GPU visibility module: graph declarations, the compute record, the opaque
//! draw and the forward view-group binding of the compacted list.

use std::collections::BTreeSet;

use super::{
    OffscreenTarget, PreparedSynthetic, RenderError, SYNTHETIC_REVISION, SceneContext, VIEW_BYTES,
};
use crate::graph::{
    Access, AccessKind, CompiledPass, FrameGraph, GraphError, PipelineKey, ResourceId,
};
pub use crate::visibility::CullMode;
use crate::visibility::{self, MeshRecord, VisibilityPrepared, VisibilityTarget};

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
    capacity: u32,
    active: u32,
) -> VisibilityPrepared {
    let key = PipelineKey {
        device_generation: generation,
        revision: SYNTHETIC_REVISION,
        role: visibility::ROLE,
    };
    VisibilityPrepared::new(device, queue, key, capacity, &[CUBE_MESH], active)
}

/// Forward vertex entry: Direct reads the physical slot with no list fetch.
pub(super) fn vertex_entry(mode: CullMode) -> &'static str {
    match mode {
        CullMode::Direct => "vertex",
        CullMode::Frustum => "compacted_vertex",
    }
}

impl PreparedSynthetic {
    /// Frustum only: declare the compute pass and the forward reads of its
    /// visible list (storage) and arguments (indirect).
    pub(super) fn append_camera_visibility(
        &self,
        graph: &mut FrameGraph,
        view: ResourceId,
        instance: &ResourceId,
        accesses: &mut Vec<Access>,
        keys: &mut BTreeSet<PipelineKey>,
    ) -> Result<(), GraphError> {
        if self.camera_culling != CullMode::Frustum {
            return Ok(());
        }
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
        ];
        let outputs = self.visibility.append_graph(graph, reads)?;
        keys.insert(self.visibility.key());
        accesses.extend([
            Access {
                resource: outputs.visible,
                range: 0..self.visibility.visible_bytes(),
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
        match self.camera_culling {
            CullMode::Direct => {
                pass.draw_indexed(0..36, 0, 0..self.instance_upload.active_count());
            }
            CullMode::Frustum => pass.draw_indexed_indirect(target.visibility.args(), 0),
        }
    }

    pub(super) fn camera_visibility_target(&self, device: &wgpu::Device) -> VisibilityTarget {
        self.visibility.target(
            device,
            &self.view_buffer,
            self.instance_upload.buffer(),
            self.history.serial_buffer(),
        )
    }
}

impl SceneContext<'_> {
    pub(super) fn visibility(&mut self, pass: &CompiledPass) -> Result<(), RenderError> {
        let view_offset =
            self.prepared.view_stride * u32::from(self.target.frame_revision & 1 != 0);
        let encoded = self.prepared.visibility.encode(
            self.encoder,
            &self.target.visibility,
            view_offset,
            self.generation,
            pass,
            self.timings.as_deref_mut(),
        )?;
        encoded.then_some(()).ok_or(RenderError::GraphPassMismatch)
    }
}
