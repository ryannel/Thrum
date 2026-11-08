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
use super::mesh_arena::{MeshArena, MeshId, MeshVertex};
use super::shadow::CasterDraw;
use super::upload::InstanceUpload;
use super::{RenderError, storage_layout};
use crate::abi::InstanceRecord;
use crate::visibility::{MemberInputs, MemberShape};

/// `Buckets` in `visibility.wgsl`: four words, then 64 four-word segments.
const TABLE_WORDS: usize = 4 + super::buckets::MAX_BUCKETS * 4;
const TABLE_BYTES: u64 = TABLE_WORDS as u64 * 4;

/// Meshes added to the arena at preparation (rows 2 onward, in order) and
/// each logical slot's mesh row. Every slot names a row the arena holds.
pub(super) struct MeshSelection {
    pub(super) meshes: Vec<(Vec<MeshVertex>, Vec<u32>)>,
    pub(super) slots: Vec<MeshId>,
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
}

impl BucketRoute {
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
        forward: impl Fn(wgpu::PrimitiveState) -> wgpu::RenderPipeline,
    ) -> Result<Self, RenderError> {
        if selection.slots.len() != instances.len()
            || selection
                .slots
                .iter()
                .any(|&mesh| arena.row(mesh).is_none())
        {
            return Err(RenderError::SceneCapacity);
        }
        let limits = BucketLimits::of(&device.limits());
        let meshes: std::collections::BTreeSet<MeshId> = selection.slots.iter().copied().collect();
        let keys = meshes
            .into_iter()
            .flat_map(|mesh| CLASSES.map(|class| BucketKey { mesh, class }));
        let mut buckets =
            Buckets::new(keys, instances.len(), limits).map_err(|_| RenderError::SceneCapacity)?;
        let slots = u64::try_from(instances.len()).map_err(|_| RenderError::SceneCapacity)?;
        let segment_bytes = slots
            .max(1)
            .next_multiple_of(u64::from(SEGMENT_ALIGN))
            .checked_mul(4)
            .ok_or(RenderError::SceneCapacity)?;
        let member_bytes = buckets
            .worst_member_bytes()
            .checked_add(segment_bytes)
            .ok_or(RenderError::SceneCapacity)?;
        if segment_bytes > limits.max_storage_binding {
            return Err(RenderError::SceneCapacity);
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
        let mut route = Self {
            buckets,
            slot_meshes: selection.slots.clone(),
            members,
            table,
            segment_bytes,
            forward: CLASSES
                .map(|class| [false, true].map(|flipped| forward(class.primitive(flipped)))),
        };
        route.publish(queue, upload)?;
        Ok(route)
    }

    fn key(mesh: MeshId, instance: &InstanceRecord) -> Option<BucketKey> {
        (instance.ids[3] != 0).then(|| BucketKey {
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
                .flat_map(|member| member.to_le_bytes())
                .collect();
            queue.write_buffer(&self.members, 0, &bytes);
            queue.write_buffer(&self.table, 0, &self.table_bytes());
        }
        Ok(())
    }

    /// The uniform table `visibility.wgsl`'s `Buckets` declares.
    fn table_bytes(&self) -> Vec<u8> {
        let segments = self.segments();
        let mut words = [0_u32; TABLE_WORDS];
        words[0] = u32::try_from(segments.len()).unwrap_or(u32::MAX);
        words[1] = self.groups();
        for (record, segment) in words[4..].as_chunks_mut::<4>().0.iter_mut().zip(segments) {
            record[0] = segment.start / SEGMENT_ALIGN;
            record[1] = segment.capacity / SEGMENT_ALIGN;
            record[2] = segment.key.mesh.row();
        }
        words.iter().flat_map(|word| word.to_le_bytes()).collect()
    }

    /// Workgroups in the current member list.
    pub(super) fn groups(&self) -> u32 {
        u32::try_from(self.buckets.members().len()).unwrap_or(u32::MAX) / SEGMENT_ALIGN
    }

    /// The compaction's fixed shape.
    pub(super) fn shape(&self) -> Result<MemberShape, RenderError> {
        Ok(MemberShape {
            buckets: u32::try_from(self.segments().len())
                .map_err(|_| RenderError::SceneCapacity)?,
            worst_members: u32::try_from(self.buckets.worst_member_bytes() / 4)
                .map_err(|_| RenderError::SceneCapacity)?,
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
        let mut entry = storage_layout(binding, wgpu::ShaderStages::VERTEX, 4);
        if let wgpu::BindingType::Buffer {
            has_dynamic_offset, ..
        } = &mut entry.ty
        {
            *has_dynamic_offset = true;
        }
        entry
    }

    /// Direct: the member list, bound one segment wide.
    pub(super) fn members_entry(&self, binding: u32) -> wgpu::BindGroupEntry<'_> {
        wgpu::BindGroupEntry {
            binding,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: &self.members,
                offset: 0,
                size: wgpu::BufferSize::new(self.segment_bytes),
            }),
        }
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
    ) -> Result<(), RenderError> {
        let buffers = arena.buffers().ok_or(RenderError::GraphPassMismatch)?;
        pass.set_index_buffer(buffers.indices.slice(..), wgpu::IndexFormat::Uint32);
        for (bucket, segment) in (0_u64..).zip(self.segments()) {
            if indirect.is_none() && segment.len == 0 {
                continue;
            }
            let pipelines = &self.forward[segment.key.class.index()];
            pass.set_pipeline(&pipelines[usize::from(orthographic)]);
            pass.set_bind_group(0, view_group, &[view_offset, segment.start * 4]);
            if let Some(args) = indirect {
                pass.draw_indexed_indirect(args, bucket * crate::visibility::ARGS_BYTES);
            } else {
                let row = arena
                    .row(segment.key.mesh)
                    .ok_or(RenderError::GraphPassMismatch)?;
                let base = row
                    .draw_base_vertex()
                    .map_err(|_| RenderError::SceneCapacity)?;
                let first = row.first_index;
                pass.draw_indexed(first..first + row.index_count, base, 0..segment.len);
            }
        }
        Ok(())
    }
}

impl BucketRoute {
    /// The Sun caster draws: every bucket's active members, never the
    /// camera-visible list, each bound at its segment offset.
    pub(super) fn caster_draws(&self, arena: &MeshArena) -> Result<Vec<CasterDraw>, RenderError> {
        self.segments()
            .iter()
            .filter(|segment| segment.len != 0)
            .map(|segment| {
                let row = arena
                    .row(segment.key.mesh)
                    .ok_or(RenderError::GraphPassMismatch)?;
                Ok(CasterDraw {
                    member_offset: segment.start * 4,
                    indices: row.first_index..row.first_index + row.index_count,
                    base_vertex: row
                        .draw_base_vertex()
                        .map_err(|_| RenderError::SceneCapacity)?,
                    instances: segment.len,
                })
            })
            .collect()
    }
}

fn bucket_error(error: BucketError) -> RenderError {
    match error {
        BucketError::UnknownBucket | BucketError::Mapping => RenderError::InvalidInstanceUpdate,
        BucketError::BucketCount | BucketError::Capacity | BucketError::DeviceLimit => {
            RenderError::SceneCapacity
        }
    }
}

#[cfg(test)]
mod tests;
