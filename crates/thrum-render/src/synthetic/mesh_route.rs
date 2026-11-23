//! The bucket route: one draw per (mesh row, pipeline class) bucket, for
//! scenes that select meshes from the arena (G3c M3b, experimental).
//!
//! Scenes that select no meshes keep the legacy route: one scene-wide
//! pipeline over the cube, two-sided whenever any instance needs it, in
//! physical-slot order. The bucket route draws each bucket's members in
//! logical-slot order with that bucket's class pipeline, buckets in key
//! order (mesh row, then class). That order is the bucket-ordered Direct
//! oracle; the culled modes compact each bucket in place and keep it.
//!
//! A bucket's members start at a multiple of `SEGMENT_ALIGN` slots, so the
//! forward view group binds the member (or visible) list at a 256-byte
//! aligned dynamic offset per bucket and every draw starts at instance 0, as
//! WebGPU requires without `indirect-first-instance`.

use super::buckets::{
    BucketError, BucketKey, BucketLimits, Buckets, PipelineClass, SEGMENT_ALIGN, Segment,
};
use super::mesh_arena::{ArenaLimits, MeshArena, MeshId};
use super::shadow::CasterDraw;
use super::upload::InstanceUpload;
use super::{
    MESH_COORDINATE_LIMIT, RenderError, SyntheticMesh, SyntheticMeshChoice, SyntheticScene,
};
use crate::abi::InstanceRecord;
use crate::visibility::{MemberInputs, MemberShape};

/// `Buckets` in `visibility.wgsl`: four words, then 64 four-word segments.
const TABLE_BYTES: u64 = size_of::<crate::records::Buckets>() as u64;

/// Each logical slot's admitted mesh row. Every slot names a row the arena
/// holds.
#[derive(Clone)]
pub(super) struct MeshSelection {
    pub(super) slots: Vec<MeshId>,
    pub(super) aloft: Vec<MeshId>,
}

/// Build the arena and check every slot's choice: the admitted selection
/// names only rows the arena holds. CPU-only, before any GPU work.
pub(super) fn admit_meshes(
    scene: &SyntheticScene,
    mut limits: ArenaLimits,
) -> Result<(MeshArena, Option<MeshSelection>), RenderError> {
    if let Some(binding) = scene
        .meshes
        .as_ref()
        .and_then(|meshes| meshes.updates.as_ref())
        .and_then(|updates| updates.0.lock().ok().and_then(|state| state.binding_limit))
    {
        let selected = ArenaLimits::for_binding(binding);
        limits.vertices = limits.vertices.min(selected.vertices);
        limits.indices = limits.indices.min(selected.indices);
    }
    let mut arena = MeshArena::new(limits).map_err(RenderError::Arena)?;
    let Some(meshes) = scene.meshes.as_deref() else {
        if !scene.aloft_mesh_rows.is_empty() {
            return Err(RenderError::InvalidAloftMesh);
        }
        return Ok((arena, None));
    };
    if meshes.slots.len() != scene.instances.len() {
        return Err(RenderError::MeshSlotCount);
    }
    let current = meshes
        .updates
        .as_ref()
        .and_then(|updates| updates.0.lock().ok().map(|state| state.current.clone()));
    let geometry: Vec<&SyntheticMesh> = current.as_ref().map_or_else(
        || meshes.meshes.iter().collect(),
        |rows| rows.iter().map(AsRef::as_ref).collect(),
    );
    if meshes.updates.is_some() {
        arena.enable_live();
    }
    let rows = geometry
        .iter()
        .map(|mesh| {
            let id = arena
                .insert(&mesh.vertices, &mesh.indices)
                .map_err(RenderError::Arena)?;
            let bounded = arena.row(id).is_some_and(|row| {
                row.bounds
                    .iter()
                    .flatten()
                    .all(|value| value.abs() <= MESH_COORDINATE_LIMIT)
            });
            if bounded {
                Ok(id)
            } else {
                Err(RenderError::Arena(
                    crate::error::ArenaError::CoordinateLimit,
                ))
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    let aloft = scene
        .aloft_mesh_rows
        .iter()
        .map(|&index| rows.get(usize::try_from(index).ok()?).copied())
        .collect::<Option<Vec<_>>>()
        .ok_or(RenderError::InvalidAloftMesh)?;
    let slots = meshes
        .slots
        .iter()
        .map(|choice| match *choice {
            SyntheticMeshChoice::Cube => Some(MeshId::CUBE),
            SyntheticMeshChoice::Mesh(index) => rows.get(usize::try_from(index).ok()?).copied(),
        })
        .collect::<Option<Vec<_>>>()
        .ok_or(RenderError::InvalidMeshSlot)?;
    arena.retain_geometry(current.unwrap_or_default());
    Ok((arena, Some(MeshSelection { slots, aloft })))
}

/// The classes in pipeline-array order.
const CLASSES: [PipelineClass; 3] = [
    PipelineClass::Ordinary,
    PipelineClass::Mirrored,
    PipelineClass::TwoSided,
];

impl PipelineClass {
    fn index(self) -> usize {
        match self {
            Self::Ordinary => 0,
            Self::Mirrored => 1,
            Self::TwoSided => 2,
        }
    }

    /// Rasterizer state: back faces culled with the class's front face, or
    /// none for the two-sided fallback. `flipped` reverses the front face for
    /// a view that mirrors the screen: the orthographic fixture keeps world XY
    /// as clip XY while its camera looks from -Z.
    pub(super) fn primitive(self, flipped: bool) -> wgpu::PrimitiveState {
        let (mirrored, cull_mode) = match self {
            Self::Ordinary => (false, Some(wgpu::Face::Back)),
            Self::Mirrored => (true, Some(wgpu::Face::Back)),
            Self::TwoSided => (false, None),
        };
        wgpu::PrimitiveState {
            front_face: if mirrored == flipped {
                wgpu::FrontFace::Ccw
            } else {
                wgpu::FrontFace::Cw
            },
            cull_mode,
            ..Default::default()
        }
    }
}

pub(super) struct TargetMembers {
    members: wgpu::Buffer,
    table: wgpu::Buffer,
    pub(super) segment_bytes: u64,
}

impl TargetMembers {
    pub(super) fn member_inputs(&self) -> MemberInputs<'_> {
        MemberInputs {
            members: &self.members,
            table: &self.table,
        }
    }

    pub(super) fn members_entry(&self, binding: u32) -> wgpu::BindGroupEntry<'_> {
        members_entry(&self.members, self.segment_bytes, binding)
    }
}

pub(super) struct BucketRoute {
    buckets: Buckets,
    slot_meshes: Vec<MeshId>,
    /// The laid-out member list Direct draws from: physical rows, padded.
    members: wgpu::Buffer,
    /// The compaction's uniform bucket table: count, mask base, then each
    /// segment's first workgroup, workgroups and mesh row.
    table: wgpu::Buffer,
    /// Bytes bound at each bucket's dynamic offset: the largest segment.
    segment_bytes: u64,
    /// Forward pipelines by class, then unflipped and flipped.
    forward: [[wgpu::RenderPipeline; 2]; 3],
    depth: Option<[[wgpu::RenderPipeline; 2]; 3]>,
    /// Each segment's checked index range and base vertex, from the arena's
    /// fixed rows: checked once at preparation.
    ranges: Vec<(std::ops::Range<u32>, i32)>,
    /// The Sun caster draws for the current layout: every bucket's active
    /// members, never the camera-visible list. Rebuilt when members change.
    caster_draws: Vec<CasterDraw>,
    /// Scene-owned horizon membership, keyed by the stable mesh row.
    aloft: Vec<MeshId>,
    /// The horizon reuses the cascade draws when there are no aloft rows.
    height_caster_draws: Option<Vec<CasterDraw>>,
}

impl BucketRoute {
    pub(super) fn target_members(&self) -> TargetMembers {
        TargetMembers {
            members: self.members.clone(),
            table: self.table.clone(),
            segment_bytes: self.segment_bytes,
        }
    }
    pub(super) fn mesh_for_slot(&self, slot: u32) -> MeshId {
        self.slot_meshes[slot as usize]
    }

    pub(super) fn refresh_rows(
        &mut self,
        arena: &MeshArena,
    ) -> Result<Vec<super::shadow::CasterBox>, RenderError> {
        for (key, range) in self.buckets.keys().iter().zip(&mut self.ranges) {
            let row = arena.row(key.mesh).ok_or(RenderError::MissingMeshRow)?;
            *range = (
                row.first_index..row.first_index + row.index_count,
                row.draw_base_vertex().map_err(RenderError::Arena)?,
            );
        }
        self.lay_out_casters();
        self.slot_meshes
            .iter()
            .map(|&mesh| {
                arena
                    .row(mesh)
                    .map(|row| super::shadow::CasterBox::from_bounds(row.bounds))
                    .ok_or(RenderError::MissingMeshRow)
            })
            .collect()
    }

    /// Check the selection against the arena and the scene, fix its buckets
    /// (every class of every selected mesh, so a class change never needs a
    /// new segment) and lay out the first frame's members.
    pub(super) fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        selection: &MeshSelection,
        arena: &MeshArena,
        upload: &InstanceUpload,
        instances: &[InstanceRecord],
        forward: impl Fn(wgpu::PrimitiveState, bool) -> wgpu::RenderPipeline,
        occlusion: bool,
    ) -> Result<Self, RenderError> {
        if selection.slots.len() != instances.len()
            || selection
                .slots
                .iter()
                .any(|&mesh| arena.row(mesh).is_none())
        {
            return Err(RenderError::MissingMeshRow);
        }
        let limits = BucketLimits::of(&device.limits());
        let meshes: std::collections::BTreeSet<MeshId> = selection.slots.iter().copied().collect();
        let keys = meshes
            .into_iter()
            .flat_map(|mesh| CLASSES.map(|class| BucketKey { mesh, class }));
        let mut buckets = Buckets::new(keys, instances.len(), limits).map_err(bucket_error)?;
        let slots =
            u64::try_from(instances.len()).map_err(|_| RenderError::BucketMemberCapacity)?;
        let segment_bytes = slots
            .max(1)
            .next_multiple_of(u64::from(SEGMENT_ALIGN))
            .checked_mul(4)
            .ok_or(RenderError::BucketMemberCapacity)?;
        let member_bytes = buckets
            .worst_member_bytes()
            .checked_add(segment_bytes)
            .ok_or(RenderError::BucketMemberCapacity)?;
        if segment_bytes > limits.max_storage_binding {
            return Err(RenderError::BucketMemberCapacity);
        }
        let members = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("synthetic bucket members"),
            size: member_bytes,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let table = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("synthetic bucket table"),
            size: TABLE_BYTES,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        for (slot, instance) in (0..).zip(instances) {
            buckets
                .set(slot, Self::key(selection.slots[slot as usize], instance))
                .map_err(bucket_error)?;
        }
        let ranges = buckets
            .keys()
            .iter()
            .map(|key| {
                let row = arena.row(key.mesh).ok_or(RenderError::MissingMeshRow)?;
                let base = row.draw_base_vertex().map_err(RenderError::Arena)?;
                let end = row
                    .first_index
                    .checked_add(row.index_count)
                    .ok_or(RenderError::MeshIndexRange)?;
                Ok((row.first_index..end, base))
            })
            .collect::<Result<Vec<_>, RenderError>>()?;
        let mut route = Self {
            buckets,
            slot_meshes: selection.slots.clone(),
            members,
            table,
            segment_bytes,
            forward: CLASSES
                .map(|class| [false, true].map(|flipped| forward(class.primitive(flipped), false))),
            depth: occlusion.then(|| {
                CLASSES.map(|class| {
                    [false, true].map(|flipped| forward(class.primitive(flipped), true))
                })
            }),
            ranges,
            caster_draws: Vec::new(),
            aloft: selection.aloft.clone(),
            height_caster_draws: (!selection.aloft.is_empty()).then(Vec::new),
        };
        route.lay_out_casters();
        route.publish(queue, upload)?;
        Ok(route)
    }

    fn key(mesh: MeshId, instance: &InstanceRecord) -> Option<BucketKey> {
        instance.is_active().then(|| BucketKey {
            mesh,
            class: PipelineClass::of(instance),
        })
    }

    /// Follow an instance upload: re-key the changed slots and lay out the
    /// members from the upload's current physical rows. Writes the member
    /// list only when the layout changed; queue order keeps submitted frames
    /// on the list they recorded.
    pub(super) fn update(
        &mut self,
        queue: &wgpu::Queue,
        upload: &InstanceUpload,
        instances: &[InstanceRecord],
        changed_slots: &[u32],
    ) -> Result<(), RenderError> {
        for &slot in changed_slots {
            let (mesh, instance) = self
                .slot_meshes
                .get(slot as usize)
                .zip(instances.get(slot as usize))
                .ok_or(RenderError::InvalidInstanceUpdate)?;
            self.buckets
                .set(slot, Self::key(*mesh, instance))
                .map_err(bucket_error)?;
        }
        self.publish(queue, upload)
    }

    fn publish(&mut self, queue: &wgpu::Queue, upload: &InstanceUpload) -> Result<(), RenderError> {
        if self
            .buckets
            .layout(upload.slot_to_physical())
            .map_err(bucket_error)?
        {
            let bytes: Vec<u8> = self
                .buckets
                .members()
                .iter()
                .flat_map(|member| bytemuck::cast::<u32, [u8; 4]>(*member))
                .collect();
            queue.write_buffer(&self.members, 0, &bytes);
            queue.write_buffer(&self.table, 0, &self.table_bytes());
            self.lay_out_casters();
        }
        Ok(())
    }

    fn lay_out_casters(&mut self) {
        self.caster_draws.clear();
        for (bucket, (segment, (indices, base_vertex))) in
            (0..).zip(self.buckets.segments().iter().zip(&self.ranges))
        {
            if segment.len != 0 {
                self.caster_draws.push(CasterDraw {
                    bucket,
                    member_offset: segment.start * 4,
                    indices: indices.clone(),
                    base_vertex: *base_vertex,
                    instances: segment.len,
                });
            }
        }
        if let Some(grounded) = &mut self.height_caster_draws {
            grounded.clear();
            grounded.extend(
                self.caster_draws
                    .iter()
                    .filter(|draw| {
                        let mesh = self.buckets.keys()[draw.bucket as usize].mesh;
                        !self.aloft.contains(&mesh)
                    })
                    .cloned(),
            );
        }
    }

    /// The uniform table `visibility.wgsl`'s `Buckets` declares.
    fn table_bytes(&self) -> Vec<u8> {
        let segments = self.segments();
        let mut table = crate::records::Buckets {
            count: u32::try_from(segments.len()).unwrap_or(u32::MAX),
            mask_base: self.groups(),
            ..bytemuck::Zeroable::zeroed()
        };
        for (record, segment) in table.segments.iter_mut().zip(segments) {
            *record = crate::records::Segment {
                first_group: segment.start / SEGMENT_ALIGN,
                groups: segment.capacity / SEGMENT_ALIGN,
                mesh: segment.key.mesh.row(),
                padding: 0,
            };
        }
        bytemuck::bytes_of(&table).to_vec()
    }

    /// Workgroups in the current member list.
    pub(super) fn groups(&self) -> u32 {
        u32::try_from(self.buckets.members().len()).unwrap_or(u32::MAX) / SEGMENT_ALIGN
    }

    /// The compaction's fixed shape.
    pub(super) fn shape(&self) -> Result<MemberShape, RenderError> {
        Ok(MemberShape {
            buckets: u32::try_from(self.segments().len())
                .map_err(|_| RenderError::BucketMemberCapacity)?,
            worst_members: u32::try_from(self.buckets.worst_member_bytes() / 4)
                .map_err(|_| RenderError::BucketMemberCapacity)?,
            segment_bytes: self.segment_bytes,
        })
    }

    /// The member list and table the compaction binds.
    pub(super) fn member_inputs(&self) -> MemberInputs<'_> {
        MemberInputs {
            members: &self.members,
            table: &self.table,
        }
    }

    pub(super) fn segments(&self) -> &[Segment] {
        self.buckets.segments()
    }

    /// The forward view-group entry the route binds per bucket.
    pub(super) fn view_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: true,
                min_binding_size: wgpu::BufferSize::new(4),
            },
            count: None,
        }
    }

    /// Direct: the member list, bound one segment wide.
    pub(super) fn members_entry(&self, binding: u32) -> wgpu::BindGroupEntry<'_> {
        members_entry(&self.members, self.segment_bytes, binding)
    }

    /// Draw every bucket with its class pipeline. `view` is the view group
    /// and its view-record offset; each bucket adds its segment's offset.
    /// Direct draws each segment's active members; `indirect` draws the
    /// culled count each bucket's argument record holds.
    pub(super) fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        (view_group, view_offset): (&wgpu::BindGroup, u32),
        arena: &MeshArena,
        orthographic: bool,
        indirect: Option<&wgpu::Buffer>,
        depth_only: bool,
    ) -> Result<(), RenderError> {
        let buffers = arena.buffers().ok_or(RenderError::GraphPassMismatch)?;
        pass.set_index_buffer(buffers.indices.slice(..), wgpu::IndexFormat::Uint32);
        for ((bucket, segment), (indices, base)) in (0_u64..).zip(self.segments()).zip(&self.ranges)
        {
            if indirect.is_none() && segment.len == 0 {
                continue;
            }
            let pipelines = if depth_only {
                self.depth.as_ref().ok_or(RenderError::GraphPassMismatch)?
            } else {
                &self.forward
            };
            let pipelines = &pipelines[segment.key.class.index()];
            pass.set_pipeline(&pipelines[usize::from(orthographic)]);
            pass.set_bind_group(0, view_group, &[view_offset, segment.start * 4]);
            if let Some(args) = indirect {
                pass.draw_indexed_indirect(args, bucket * crate::visibility::ARGS_BYTES);
            } else {
                pass.draw_indexed(indices.clone(), *base, 0..segment.len);
            }
        }
        Ok(())
    }
}

fn members_entry(buffer: &wgpu::Buffer, bytes: u64, binding: u32) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
            buffer,
            offset: 0,
            size: wgpu::BufferSize::new(bytes),
        }),
    }
}

impl BucketRoute {
    pub(super) fn draw_secondary(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        view: &wgpu::BindGroup,
        arena: &MeshArena,
        pipelines: &[wgpu::RenderPipeline; 3],
        indirect: Option<&wgpu::Buffer>,
    ) -> Result<(), RenderError> {
        let buffers = arena.buffers().ok_or(RenderError::GraphPassMismatch)?;
        pass.set_index_buffer(buffers.indices.slice(..), wgpu::IndexFormat::Uint32);
        for ((bucket, segment), (indices, base)) in (0_u64..).zip(self.segments()).zip(&self.ranges)
        {
            if segment.len == 0 {
                continue;
            }
            pass.set_pipeline(&pipelines[segment.key.class.index()]);
            pass.set_bind_group(0, view, &[0, segment.start * 4]);
            if let Some(args) = indirect {
                pass.draw_indexed_indirect(args, bucket * crate::visibility::ARGS_BYTES);
            } else {
                pass.draw_indexed(indices.clone(), *base, 0..segment.len);
            }
        }
        Ok(())
    }

    /// The Sun caster draws for the current member layout, each bound at its
    /// segment offset.
    pub(super) fn caster_draws(&self) -> &[CasterDraw] {
        &self.caster_draws
    }

    pub(super) fn height_caster_draws(&self) -> &[CasterDraw] {
        self.height_caster_draws
            .as_deref()
            .unwrap_or(&self.caster_draws)
    }
}

fn bucket_error(error: BucketError) -> RenderError {
    match error {
        BucketError::UnknownBucket | BucketError::Mapping => RenderError::InvalidInstanceUpdate,
        BucketError::BucketCount => RenderError::BucketCount,
        BucketError::Capacity => RenderError::BucketMemberCapacity,
        BucketError::StorageLimit => RenderError::BucketStorageLimit,
        BucketError::DispatchLimit => RenderError::BucketDispatchLimit,
    }
}

#[cfg(test)]
mod tests;
