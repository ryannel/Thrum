//! Camera culling glue between the prepared synthetic scene and the private
//! GPU visibility module: graph declarations, the compute records, the opaque
//! draws and the forward view-group binding of the compacted lists.
//!
//! Occlusion adds, after `synthetic forward opaque` (which draws the early
//! list), the depth pyramid, the late phase and `synthetic forward late`,
//! which loads the opaque attachments and draws the late list. History
//! publication and tone read what the late pass wrote, so they follow it even
//! when its list is empty. While a scene contribution draws, the early phase
//! takes the Frustum decision and the late phase lists nothing, so the
//! contribution's blended output is never overdrawn.

use std::collections::BTreeSet;

use super::{
    OffscreenTarget, PreparedSynthetic, RenderError, SYNTHETIC_REVISION, SceneContext, VIEW_BYTES,
    history_gpu, storage_layout,
};
use crate::graph::{
    Access, AccessKind, CompiledPass, FrameGraph, GraphError, Pass, PassScope, PipelineKey,
    ResourceDescriptor, ResourceId, ResourceScope,
};
pub use crate::visibility::CullMode;
use crate::visibility::{
    self, MeshRecord, OcclusionTarget, Phase, VisibilityIds, VisibilityPrepared, VisibilityTarget,
};
pub use stats::CullingCounts;
pub(super) use stats::CullingStats;
pub(super) use stats::STATS_PASS;

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
/// Graph and timing name of the late opaque pass.
pub(crate) const FORWARD_LATE_PASS: &str = "synthetic forward late";
/// Pipeline role of the late opaque pass inside the synthetic closure.
const FORWARD_LATE_ROLE: u32 = 0x464C;

/// One frame slot's late phase and the forward view group that reads its list.
pub(super) struct LateCulling {
    target: OcclusionTarget,
    view_bind_group: wgpu::BindGroup,
}

/// The early phase's graph outputs and the reads the late phase repeats.
pub(super) struct EarlyGraph {
    ids: VisibilityIds,
    reads: Vec<Access>,
}

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
        mode,
    ))
}

/// Culled modes: the forward view-group entry for the compacted slot list.
pub(super) fn view_layout_entry(mode: CullMode) -> Option<wgpu::BindGroupLayoutEntry> {
    (mode != CullMode::Direct)
        .then(|| storage_layout(VISIBLE_BINDING, wgpu::ShaderStages::VERTEX, 4))
}

/// Timed scene passes camera culling adds.
pub(super) fn pass_count(mode: CullMode) -> u32 {
    match mode {
        CullMode::Direct => 0,
        CullMode::Frustum => 1,
        CullMode::Occlusion => 4,
    }
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
        CullMode::Frustum | CullMode::Occlusion => "compacted_vertex",
    }
}

/// Occlusion only: the late forward view group, the same entries with the
/// late visible list.
pub(super) fn late_view_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    entries: &[wgpu::BindGroupEntry<'_>],
    target: Option<OcclusionTarget>,
) -> Option<LateCulling> {
    let target = target?;
    let entries: Vec<_> = entries
        .iter()
        .map(|entry| match entry.binding {
            VISIBLE_BINDING => wgpu::BindGroupEntry {
                binding: VISIBLE_BINDING,
                resource: target.late().visible().as_entire_binding(),
            },
            _ => entry.clone(),
        })
        .collect();
    let view_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("synthetic late view and target aux"),
        layout,
        entries: &entries,
    });
    Some(LateCulling {
        target,
        view_bind_group,
    })
}

impl PreparedSynthetic {
    /// Culled modes: declare the frustum or early compute pass and the
    /// forward reads of its visible list (storage) and arguments (indirect).
    pub(super) fn append_camera_visibility(
        &self,
        graph: &mut FrameGraph,
        view: ResourceId,
        instance: &ResourceId,
        accesses: &mut Vec<Access>,
        keys: &mut BTreeSet<PipelineKey>,
    ) -> Result<Option<EarlyGraph>, GraphError> {
        let Some(prepared) = &self.visibility else {
            return Ok(None);
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
        let ids = prepared.append_graph(graph, reads.clone())?;
        keys.insert(prepared.key());
        accesses.extend(list_reads(prepared, &ids));
        Ok(Some(EarlyGraph { ids, reads }))
    }

    /// After the early forward pass, declare each culled mode's untimed
    /// count copy and, for Occlusion only, the pyramid, the late phase and
    /// the late forward pass. The late pass makes the early pass's accesses
    /// with its attachments loaded and the late list in place of the early
    /// one. Returns the late pass, which history publication follows.
    pub(super) fn append_camera_late(
        &self,
        graph: &mut FrameGraph,
        early: Option<EarlyGraph>,
        (depth, extent): (&ResourceId, [u32; 2]),
        forward_pass: usize,
        keys: &mut BTreeSet<PipelineKey>,
    ) -> Result<Option<usize>, GraphError> {
        let (Some(prepared), Some(early)) = (&self.visibility, early) else {
            return Ok(None);
        };
        if prepared.mode() != CullMode::Occlusion {
            Self::append_camera_stats(graph, &[early.ids.args])?;
            return Ok(None);
        }
        let pyramid_pass = graph.passes().len();
        let late = prepared.append_late_graph(graph, early.reads, &early.ids, depth, extent)?;
        // Two passes write the attachments; the pyramid reads the early one.
        graph.dependency(forward_pass, pyramid_pass)?;
        keys.extend([
            prepared.late_key(),
            prepared.pyramid_key(),
            self.forward_late_key(),
        ]);
        let swapped = list_reads(prepared, &late);
        // The early forward pass's own declaration, with the attachments
        // loaded and the late list and arguments in place of the early ones.
        let accesses = graph
            .passes()
            .get(forward_pass)
            .ok_or(GraphError::InvalidDependency)?
            .accesses
            .iter()
            .map(|access| {
                let mut access = access.clone();
                if access.kind == AccessKind::AttachmentClear {
                    access.kind = AccessKind::AttachmentLoad;
                }
                if access.resource == early.ids.visible {
                    access.resource = swapped[0].resource.clone();
                } else if access.resource == early.ids.args {
                    access.resource = swapped[1].resource.clone();
                }
                access
            })
            .collect();
        let pass = graph.pass(Pass {
            name: FORWARD_LATE_PASS.into(),
            scope: PassScope::View(1),
            pipeline: Some(self.forward_late_key()),
            accesses,
            external_side_effect: true,
        });
        Self::append_camera_stats(graph, &[early.ids.args, swapped[1].resource.clone()])?;
        Ok(Some(pass))
    }

    /// Occlusion only: every pass after the late forward pass that reads an
    /// attachment both forward passes write reads the late pass's output.
    pub(super) fn order_after_camera_late(
        graph: &mut FrameGraph,
        late: Option<usize>,
        attachments: [&ResourceId; 3],
    ) -> Result<(), GraphError> {
        let Some(late) = late else {
            return Ok(());
        };
        let readers: Vec<usize> = graph
            .passes()
            .iter()
            .enumerate()
            .skip(late + 1)
            .filter(|(_, pass)| {
                pass.accesses.iter().any(|access| {
                    attachments.contains(&&access.resource)
                        && matches!(
                            access.kind,
                            AccessKind::SampledRead
                                | AccessKind::CopyRead
                                | AccessKind::StorageRead
                                | AccessKind::StorageReadWrite
                                | AccessKind::AttachmentLoad
                        )
                })
            })
            .map(|(index, _)| index)
            .collect();
        for reader in readers {
            graph.dependency(late, reader)?;
        }
        Ok(())
    }

    pub(super) fn forward_late_key(&self) -> PipelineKey {
        PipelineKey {
            role: FORWARD_LATE_ROLE,
            ..self.key
        }
    }

    pub(super) fn draw_camera(&self, pass: &mut wgpu::RenderPass<'_>, target: &OffscreenTarget) {
        match &target.visibility {
            Some(culled) if self.visibility.is_some() => {
                pass.draw_indexed_indirect(culled.args(), 0);
            }
            _ => pass.draw_indexed(0..36, 0, 0..self.instance_upload.active_count()),
        }
    }

    /// The frustum or early target, and Occlusion's late phase over the
    /// pyramid of `depth`.
    pub(super) fn camera_visibility_target(
        &self,
        device: &wgpu::Device,
        depth: &wgpu::TextureView,
        extent: [u32; 2],
    ) -> (Option<VisibilityTarget>, Option<OcclusionTarget>) {
        let Some(prepared) = &self.visibility else {
            return (None, None);
        };
        let inputs = (
            &self.view_buffer,
            self.instance_upload.buffer(),
            self.history.serial_buffer(),
        );
        match prepared.occlusion_target(device, inputs.0, inputs.1, inputs.2, depth, extent) {
            Some((early, late)) => (Some(early), Some(late)),
            None => (
                Some(prepared.target(device, inputs.0, inputs.1, inputs.2)),
                None,
            ),
        }
    }

    /// Occlusion only: the late opaque pass, loading the early pass's
    /// attachments and drawing the late list with the same pipeline,
    /// bindings and indices. No atmosphere and no contribution draw here.
    pub(super) fn encode_opaque_late(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &OffscreenTarget,
        compiled: &CompiledPass,
        timings: Option<&mut crate::timing::GpuTimings>,
        exposure_side: usize,
    ) -> Result<(), RenderError> {
        let late = target
            .occlusion
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?;
        let mut pass = self.begin_forward(encoder, target, compiled, timings)?;
        self.bind_forward(&mut pass, target, &late.view_bind_group, exposure_side);
        pass.draw_indexed_indirect(late.target.late().args(), 0);
        Ok(())
    }
}

fn list_reads(prepared: &VisibilityPrepared, ids: &VisibilityIds) -> [Access; 2] {
    [
        Access {
            resource: ids.visible.clone(),
            range: 0..prepared.visible_bytes(),
            kind: AccessKind::StorageRead,
            store: true,
        },
        Access {
            resource: ids.args.clone(),
            range: 0..visibility::ARGS_BYTES,
            kind: AccessKind::IndirectRead,
            store: true,
        },
    ]
}

impl SceneContext<'_> {
    /// The frustum or early phase. With a contribution drawing, the early
    /// phase takes the Frustum decision.
    pub(super) fn visibility(&mut self, pass: &CompiledPass) -> Result<(), RenderError> {
        let frame_target = self.target;
        let target = frame_target
            .visibility
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?;
        let phase = if self.contribution.is_some() {
            Phase::Frustum
        } else {
            target.phase()
        };
        self.record_phase(target, phase, pass)
    }

    /// Occlusion's late phase; Skip while a contribution draws.
    pub(super) fn late_visibility(&mut self, pass: &CompiledPass) -> Result<(), RenderError> {
        let frame_target = self.target;
        let late = frame_target
            .occlusion
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?
            .target
            .late();
        let phase = if self.contribution.is_some() {
            Phase::Skip
        } else {
            Phase::Late
        };
        self.record_phase(late, phase, pass)
    }

    fn record_phase(
        &mut self,
        target: &VisibilityTarget,
        phase: Phase,
        pass: &CompiledPass,
    ) -> Result<(), RenderError> {
        let prepared = self
            .prepared
            .visibility
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?;
        let view_offset =
            self.prepared.view_stride * u32::from(self.target.frame_revision & 1 != 0);
        let encoded = prepared.encode_phase(
            self.encoder,
            target,
            // The transform history's write side: only a committed display
            // frame flips it, so captures leave the displayed bits alone.
            (phase, self.exposure_side),
            (view_offset, self.generation),
            pass,
            self.timings.as_deref_mut(),
        )?;
        encoded.then_some(()).ok_or(RenderError::GraphPassMismatch)
    }

    /// The pyramid, which only the late classifier reads. While a
    /// contribution draws, the late phase is Skip and nothing is recorded:
    /// Metal does not sample a compute pass without a dispatch, so the frame's
    /// timing layout leaves the pass out too.
    pub(super) fn depth_pyramid(&mut self, pass: &CompiledPass) -> Result<(), RenderError> {
        let (Some(prepared), Some(late)) = (&self.prepared.visibility, &self.target.occlusion)
        else {
            return Err(RenderError::GraphPassMismatch);
        };
        if self.contribution.is_some() {
            return Ok(());
        }
        let encoded = prepared.encode_pyramid(
            self.encoder,
            &late.target,
            self.generation,
            pass,
            self.timings.as_deref_mut(),
        )?;
        encoded.then_some(()).ok_or(RenderError::GraphPassMismatch)
    }

    pub(super) fn opaque_late(&mut self, pass: &CompiledPass) -> Result<(), RenderError> {
        self.prepared.encode_opaque_late(
            self.encoder,
            self.target,
            pass,
            self.timings.as_deref_mut(),
            self.exposure_side,
        )
    }
}

mod stats;
#[cfg(test)]
pub(crate) mod tests;
