//! Camera culling glue between the prepared synthetic scene and the private
//! GPU visibility module: graph declarations, the compute records, the opaque
//! draws and the forward view-group binding of the compacted lists.
//!
//! Occlusion declares four passes before `synthetic forward opaque`: the
//! prepass list, `camera depth prepass` (last frame's visible set, depth
//! only, into a private target), the pyramid of that depth, and the final
//! cull. The forward pass is unchanged from Frustum's except that it draws
//! the final list, so atmosphere, the list in slot order into cleared depth,
//! then any contribution draw exactly as in Direct.

use std::collections::BTreeSet;

use super::mesh_arena::MeshArena;
use super::mesh_route::BucketRoute;
use super::{
    CompiledPass, DEPTH_FORMAT, GraphColorAttachment, GraphDepthAttachment, OffscreenTarget,
    PreparedSynthetic, RenderError, SYNTHETIC_REVISION, SceneContext, VIEW_BYTES, history_gpu,
    storage_layout,
};
use crate::graph::{
    Access, AccessKind, FrameGraph, GraphError, Pass, PassScope, PipelineKey, ResourceDescriptor,
    ResourceId, ResourceScope,
};
pub use crate::visibility::CullMode;
use crate::visibility::{
    self, MeshRecord, OcclusionTarget, VisibilityIds, VisibilityPrepared, VisibilityTarget,
};
pub(super) use stats::CullingStats;
pub(super) use stats::STATS_PASS;
pub use stats::{CountedFrame, CullingCounts};

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
/// Graph and timing name of Occlusion's depth-only prepass.
pub(crate) const DEPTH_PREPASS_PASS: &str = "camera depth prepass";
/// Pipeline role of the depth prepass inside the synthetic closure.
const DEPTH_PREPASS_ROLE: u32 = 0x4450;

/// One frame slot's prepass list, its pyramid, the private depth the
/// prepass draws, and the view group that reads the prepass list.
pub(super) struct PrepassCulling {
    target: OcclusionTarget,
    view_bind_group: wgpu::BindGroup,
    depth: wgpu::TextureView,
}

/// What the scene graph needs back from camera culling's declarations.
pub(super) struct CameraGraph {
    /// Argument buffers whose counts the stats copy reads: the drawn list
    /// first, then Occlusion's prepass list.
    pub(super) stats: Vec<ResourceId>,
    /// Occlusion only: the private prepass depth target.
    pub(super) prepass_depth: Option<ResourceId>,
}

/// Visibility pipelines for this scene's slot capacity: over the cube mesh
/// row for the legacy route, or member-indexed over the arena's rows for
/// the bucket route.
pub(super) fn prepare(
    device: &crate::runtime::DeviceSite,
    queue: &wgpu::Queue,
    generation: u64,
    mode: CullMode,
    (capacity, active): (u32, u32),
    route: Option<(&BucketRoute, &MeshArena)>,
) -> Result<Option<VisibilityPrepared>, RenderError> {
    if mode == CullMode::Direct {
        return Ok(None);
    }
    let key = PipelineKey {
        device_generation: generation,
        revision: SYNTHETIC_REVISION,
        role: visibility::ROLE,
    };
    let Some((route, arena)) = route else {
        return Ok(Some(VisibilityPrepared::new(
            device,
            queue,
            key,
            capacity,
            &[CUBE_MESH],
            active,
            mode,
        )));
    };
    let meshes = mesh_records(arena)?;
    let mut prepared = VisibilityPrepared::new_members(
        device,
        queue,
        key,
        (capacity, active),
        &meshes,
        route.shape()?,
        mode,
    );
    prepared.set_member_groups(route.groups());
    Ok(Some(prepared))
}

/// The arena's rows as the compaction's mesh table: index range and bounds.
pub(super) fn mesh_records(arena: &MeshArena) -> Result<Vec<MeshRecord>, RenderError> {
    arena
        .rows()
        .iter()
        .map(|row| {
            Ok(MeshRecord {
                first_index: row.first_index,
                index_count: row.index_count,
                base_vertex: row
                    .draw_base_vertex()
                    .map_err(|_| RenderError::SceneCapacity)?,
                bounds_min: row.bounds[0],
                bounds_max: row.bounds[1],
            })
        })
        .collect()
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

/// Occlusion only: the prepass view group, the forward entries with the
/// prepass list in place of the final one.
pub(super) fn prepass_view_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    entries: &[wgpu::BindGroupEntry<'_>],
    target: Option<OcclusionTarget>,
    depth: Option<(ResourceId, wgpu::TextureView)>,
) -> Option<PrepassCulling> {
    let (target, (_, depth)) = target.zip(depth)?;
    let entries: Vec<_> = entries
        .iter()
        .map(|entry| match entry.binding {
            VISIBLE_BINDING => wgpu::BindGroupEntry {
                binding: VISIBLE_BINDING,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: target.prepass().visible(),
                    offset: 0,
                    size: match &entry.resource {
                        wgpu::BindingResource::Buffer(binding) => binding.size,
                        _ => None,
                    },
                }),
            },
            _ => entry.clone(),
        })
        .collect();
    let view_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("synthetic prepass view and target aux"),
        layout,
        entries: &entries,
    });
    Some(PrepassCulling {
        target,
        view_bind_group,
        depth,
    })
}

impl PreparedSynthetic {
    /// Culled modes: declare the visibility passes and the forward reads of
    /// the drawn list (storage) and its arguments (indirect). Frustum
    /// declares one compute pass. Occlusion declares the prepass list, the
    /// depth prepass into a private `extent` target, the pyramid and the
    /// final cull, whose list the forward pass reads.
    pub(super) fn append_camera_visibility(
        &self,
        graph: &mut FrameGraph,
        (view, instance, mesh, route): (ResourceId, &ResourceId, &[Access], &[Access]),
        extent: [u32; 2],
        accesses: &mut Vec<Access>,
        keys: &mut BTreeSet<PipelineKey>,
    ) -> Result<CameraGraph, GraphError> {
        let Some(prepared) = &self.visibility else {
            return Ok(CameraGraph {
                stats: Vec::new(),
                prepass_depth: None,
            });
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
        let mut list_inputs = reads.clone();
        list_inputs.extend_from_slice(route);
        let first = prepared.append_graph(graph, list_inputs)?;
        keys.insert(prepared.key());
        if prepared.mode() != CullMode::Occlusion {
            accesses.extend(list_reads(prepared, &first));
            return Ok(CameraGraph {
                stats: vec![first.args],
                prepass_depth: None,
            });
        }
        let depth = graph.resource(
            ResourceDescriptor::Texture {
                width: extent[0],
                height: extent[1],
                layers: 1,
                mips: 1,
                samples: 1,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
            },
            ResourceScope::Transient,
        )?;
        let mut prepass = reads.clone();
        prepass.extend(list_reads(prepared, &first));
        prepass.extend_from_slice(mesh);
        prepass.push(Access {
            resource: depth.clone(),
            range: 0..1,
            kind: AccessKind::AttachmentClear,
            store: true,
        });
        graph.pass(Pass {
            name: DEPTH_PREPASS_PASS.into(),
            scope: PassScope::View(1),
            pipeline: Some(self.depth_prepass_key()),
            accesses: prepass,
            external_side_effect: false,
        });
        let mut final_reads = reads;
        final_reads.extend_from_slice(route);
        let last = prepared.append_final_graph(graph, final_reads, &first, &depth, extent)?;
        keys.extend([
            prepared.final_key(),
            prepared.pyramid_key(),
            self.depth_prepass_key(),
        ]);
        accesses.extend(list_reads(prepared, &last));
        Ok(CameraGraph {
            stats: vec![last.args, first.args],
            prepass_depth: Some(depth),
        })
    }

    pub(super) fn depth_prepass_key(&self) -> PipelineKey {
        PipelineKey {
            role: DEPTH_PREPASS_ROLE,
            ..self.key
        }
    }

    pub(super) fn draw_camera(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        target: &OffscreenTarget,
    ) -> Result<(), RenderError> {
        if let Some(route) = &self.bucket_route {
            let view_offset = self.view_stride * u32::from(target.frame_revision & 1 != 0);
            let view = self.pending_view.as_ref().unwrap_or(&self.view_sample);
            let orthographic = matches!(view.projection(), super::Projection::OrthographicFixture);
            let indirect = target
                .visibility
                .as_ref()
                .filter(|_| self.visibility.is_some())
                .map(VisibilityTarget::args);
            return route.draw(
                pass,
                (&target.view_bind_group, view_offset),
                &self.mesh_arena,
                orthographic,
                indirect,
                false,
            );
        }
        match &target.visibility {
            Some(culled) if self.visibility.is_some() => {
                pass.draw_indexed_indirect(culled.args(), 0);
            }
            _ => pass.draw_indexed(0..36, 0, 0..self.instance_upload.active_count()),
        }
        Ok(())
    }

    /// The drawn list's target (frustum, or Occlusion's final cull), and
    /// Occlusion's prepass list over the pyramid of the private
    /// `prepass_depth`.
    pub(super) fn camera_visibility_target(
        &self,
        device: &wgpu::Device,
        prepass_depth: Option<&wgpu::TextureView>,
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
        let members = self.bucket_route.as_ref().map(BucketRoute::member_inputs);
        let occlusion = prepass_depth.and_then(|depth| {
            prepared.occlusion_target(device, inputs.0, inputs.1, inputs.2, depth, extent, members)
        });
        if let Some((last, prepass)) = occlusion {
            return (Some(last), Some(prepass));
        }
        let members = self.bucket_route.as_ref().map(BucketRoute::member_inputs);
        (Some(prepared.member_target(device, inputs, members)), None)
    }

    /// Occlusion only: the prepass list draws depth only into the private
    /// target, with the forward vertex entry, primitive state, two-sided
    /// choice, bindings and indices, so its depth equals what the forward
    /// pass writes for the same slots.
    fn encode_depth_prepass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &OffscreenTarget,
        compiled: &CompiledPass,
        timings: Option<&mut crate::timing::GpuTimings>,
        exposure_side: usize,
    ) -> Result<(), RenderError> {
        let prepass = target
            .occlusion
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?;
        let no_color: [Option<GraphColorAttachment<'_>>; 0] = [];
        let mut pass = compiled
            .begin_render(
                encoder,
                no_color,
                Some(GraphDepthAttachment {
                    resource: target
                        .ids
                        .prepass_depth
                        .as_ref()
                        .ok_or(RenderError::GraphPassMismatch)?,
                    range: 0..1,
                    view: &prepass.depth,
                    clear: 0.0,
                }),
                self.key.device_generation,
                timings,
            )?
            .ok_or(RenderError::GraphPassMismatch)?;
        self.bind_forward(
            &mut pass,
            target,
            &prepass.view_bind_group,
            exposure_side,
            true,
        )?;
        if let Some(route) = &self.bucket_route {
            let view_offset = self.view_stride * u32::from(target.frame_revision & 1 != 0);
            let view = self.pending_view.as_ref().unwrap_or(&self.view_sample);
            route.draw(
                &mut pass,
                (&prepass.view_bind_group, view_offset),
                &self.mesh_arena,
                matches!(view.projection(), super::Projection::OrthographicFixture),
                Some(prepass.target.prepass().args()),
                true,
            )?;
        } else {
            pass.draw_indexed_indirect(prepass.target.prepass().args(), 0);
        }
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
            range: 0..prepared.args_bytes(),
            kind: AccessKind::IndirectRead,
            store: true,
        },
    ]
}

impl SceneContext<'_> {
    /// The frustum list, or Occlusion's prepass list.
    pub(super) fn visibility(&mut self, pass: &CompiledPass) -> Result<(), RenderError> {
        let frame_target = self.target;
        let target = match &frame_target.occlusion {
            Some(prepass) => prepass.target.prepass(),
            None => frame_target
                .visibility
                .as_ref()
                .ok_or(RenderError::GraphPassMismatch)?,
        };
        self.record_phase(target, pass)
    }

    /// Occlusion's final cull, whose list the forward pass draws.
    pub(super) fn final_visibility(&mut self, pass: &CompiledPass) -> Result<(), RenderError> {
        let frame_target = self.target;
        if frame_target.occlusion.is_none() {
            return Err(RenderError::GraphPassMismatch);
        }
        let target = frame_target
            .visibility
            .as_ref()
            .ok_or(RenderError::GraphPassMismatch)?;
        self.record_phase(target, pass)
    }

    fn record_phase(
        &mut self,
        target: &VisibilityTarget,
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
            self.exposure_side,
            (view_offset, self.generation),
            pass,
            self.timings.as_deref_mut(),
        )?;
        encoded.then_some(()).ok_or(RenderError::GraphPassMismatch)
    }

    /// The pyramid of the prepass depth, which only the final cull reads.
    pub(super) fn depth_pyramid(&mut self, pass: &CompiledPass) -> Result<(), RenderError> {
        let (Some(prepared), Some(prepass)) = (&self.prepared.visibility, &self.target.occlusion)
        else {
            return Err(RenderError::GraphPassMismatch);
        };
        let encoded = prepared.encode_pyramid(
            self.encoder,
            &prepass.target,
            self.generation,
            pass,
            self.timings.as_deref_mut(),
        )?;
        encoded.then_some(()).ok_or(RenderError::GraphPassMismatch)
    }

    pub(super) fn depth_prepass(&mut self, pass: &CompiledPass) -> Result<(), RenderError> {
        self.prepared.encode_depth_prepass(
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
