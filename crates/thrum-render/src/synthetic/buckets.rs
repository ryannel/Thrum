//! Draw buckets for mixed meshes (G3c M3b, private until root reviews the
//! wiring).
//!
//! A bucket is one (mesh row, pipeline class) pair, fixed when the scene is
//! prepared, so an empty bucket keeps its segment and draws nothing. The CPU
//! is the only authority for a slot's key: the mesh row comes from the
//! renderer-owned per-slot mesh metadata, and the pipeline class from the
//! upload's f64 determinant. No shader recomputes a key.
//!
//! Each bucket owns a segment of the member list starting at a multiple of
//! [`SEGMENT_ALIGN`] slots: the compaction workgroup width, and WebGPU's
//! 256-byte dynamic-offset alignment for the visible list. A segment holds its
//! active members' physical rows in logical-slot order (the bucket-ordered
//! oracle's order), then [`PADDING`] up to the next segment.

use std::collections::BTreeMap;

use super::mesh_arena::MeshId;
use super::upload::{DEGENERATE_DETERMINANT, determinant};
use crate::abi::InstanceRecord;

/// Slots per alignment step; each segment starts at a multiple of this.
pub(super) const SEGMENT_ALIGN: u32 = 256;
/// A member-list entry that names no instance.
pub(super) const PADDING: u32 = u32::MAX;

/// How a bucket rasterizes.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) enum PipelineClass {
    /// Positive determinant: counter-clockwise front faces, back faces culled.
    Ordinary,
    /// Negative determinant: clockwise front faces, back faces culled.
    Mirrored,
    /// Near-zero determinant: no culling, the existing two-sided fallback.
    TwoSided,
}

impl PipelineClass {
    pub(super) fn of(instance: &InstanceRecord) -> Self {
        let determinant = determinant(instance);
        if determinant > DEGENERATE_DETERMINANT {
            Self::Ordinary
        } else if determinant < -DEGENERATE_DETERMINANT {
            Self::Mirrored
        } else {
            Self::TwoSided
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) struct BucketKey {
    pub(super) mesh: MeshId,
    pub(super) class: PipelineClass,
}

/// One bucket's place in the member list.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Segment {
    pub(super) key: BucketKey,
    /// First member, a multiple of `SEGMENT_ALIGN`.
    pub(super) start: u32,
    /// Active members.
    pub(super) len: u32,
    /// Slots reserved, `len` rounded up to `SEGMENT_ALIGN` (at least one step).
    pub(super) capacity: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BucketError {
    /// A slot's key is not one of the buckets fixed at prepare.
    UnknownBucket,
    /// No buckets, or more than the shader's `MAX_BUCKETS`.
    BucketCount,
    /// A slot index, member offset or byte size does not fit.
    Capacity,
    /// The worst-case member list, scratch or dispatch exceeds a device limit.
    DeviceLimit,
    /// The physical mapping does not match the slot count or names a row past it.
    Mapping,
}

/// The shader's bucket-table length (`MAX_BUCKETS` in `visibility.wgsl`).
pub(super) const MAX_BUCKETS: usize = 64;
/// Mask words per workgroup, stored after the counts in the scratch buffer.
const WORDS: u64 = SEGMENT_ALIGN as u64 / 32;

/// The device limits a bucket layout must fit, checked at prepare.
#[derive(Clone, Copy, Debug)]
pub(super) struct BucketLimits {
    pub(super) max_storage_binding: u64,
    pub(super) max_workgroups: u32,
}

impl BucketLimits {
    pub(super) fn of(limits: &wgpu::Limits) -> Self {
        Self {
            max_storage_binding: limits.max_storage_buffer_binding_size,
            max_workgroups: limits.max_compute_workgroups_per_dimension,
        }
    }
}

/// The largest member list any layout of `slots` slots in `buckets` buckets
/// can need: every slot, plus at most one alignment step of padding per
/// bucket (an empty bucket keeps one step).
fn worst_members(slots: usize, buckets: usize) -> Option<u64> {
    u64::try_from(slots).ok()?.checked_add(
        u64::try_from(buckets)
            .ok()?
            .checked_mul(u64::from(SEGMENT_ALIGN))?,
    )
}

/// Per-slot bucket keys and the member list they lay out.
pub(super) struct Buckets {
    /// Bucket index per key, in key order.
    index: BTreeMap<BucketKey, usize>,
    keys: Vec<BucketKey>,
    /// Each logical slot's bucket, or `None` while inactive.
    slot_bucket: Vec<Option<usize>>,
    /// A key changed since the last layout.
    dirty: bool,
    /// The physical mapping the live layout was built from.
    laid_out: Vec<u32>,
    segments: Vec<Segment>,
    members: Vec<u32>,
    /// Reused scratch: per-bucket counts, then write cursors.
    counts: Vec<u32>,
    next_segments: Vec<Segment>,
    next_members: Vec<u32>,
    /// The admitted worst-case member list, in bytes.
    worst_member_bytes: u64,
}

impl Buckets {
    /// Fix the bucket set for `slots` slots and admit its worst case against
    /// `limits`. Duplicate keys collapse; order is key order.
    pub(super) fn new(
        keys: impl IntoIterator<Item = BucketKey>,
        slots: usize,
        limits: BucketLimits,
    ) -> Result<Self, BucketError> {
        let mut keys: Vec<_> = keys.into_iter().collect();
        keys.sort_unstable();
        keys.dedup();
        if keys.is_empty() || keys.len() > MAX_BUCKETS {
            return Err(BucketError::BucketCount);
        }
        u32::try_from(slots).map_err(|_| BucketError::Capacity)?;
        let members = worst_members(slots, keys.len()).ok_or(BucketError::Capacity)?;
        let groups = members / u64::from(SEGMENT_ALIGN);
        let member_bytes = members.checked_mul(4).ok_or(BucketError::Capacity)?;
        let scratch_bytes = groups
            .checked_mul(1 + WORDS)
            .and_then(|words| words.checked_mul(4))
            .ok_or(BucketError::Capacity)?;
        if member_bytes.max(scratch_bytes) > limits.max_storage_binding
            || groups > u64::from(limits.max_workgroups)
        {
            return Err(BucketError::DeviceLimit);
        }
        let index = keys.iter().enumerate().map(|(i, &key)| (key, i)).collect();
        let capacity = usize::try_from(members).map_err(|_| BucketError::Capacity)?;
        Ok(Self {
            index,
            counts: vec![0; keys.len()],
            next_segments: Vec::with_capacity(keys.len()),
            segments: Vec::with_capacity(keys.len()),
            keys,
            slot_bucket: vec![None; slots],
            dirty: true,
            laid_out: Vec::with_capacity(slots),
            members: Vec::with_capacity(capacity),
            next_members: Vec::with_capacity(capacity),
            worst_member_bytes: member_bytes,
        })
    }

    /// Set a slot's key, or `None` for an inactive slot. Refused keys leave
    /// the slot unchanged.
    pub(super) fn set(&mut self, slot: u32, key: Option<BucketKey>) -> Result<(), BucketError> {
        let bucket = key
            .map(|key| {
                self.index
                    .get(&key)
                    .copied()
                    .ok_or(BucketError::UnknownBucket)
            })
            .transpose()?;
        let entry = self
            .slot_bucket
            .get_mut(slot as usize)
            .ok_or(BucketError::Capacity)?;
        if *entry != bucket {
            *entry = bucket;
            self.dirty = true;
        }
        Ok(())
    }

    /// Lay out every segment from the upload's current physical rows, and
    /// return whether the live layout changed. Nothing changes on an error, or
    /// when no key changed and the mapping is the one last laid out.
    pub(super) fn layout(&mut self, slot_to_physical: &[u32]) -> Result<bool, BucketError> {
        let slots = self.slot_bucket.len();
        if slot_to_physical.len() != slots
            || slot_to_physical.iter().any(|&row| row as usize >= slots)
        {
            return Err(BucketError::Mapping);
        }
        if !self.dirty && self.laid_out == slot_to_physical {
            return Ok(false);
        }
        self.counts.fill(0);
        for bucket in self.slot_bucket.iter().flatten() {
            self.counts[*bucket] += 1;
        }
        // Admit every range before touching the live layout.
        self.next_segments.clear();
        let mut end = 0_u32;
        for (key, count) in self.keys.iter().zip(&mut self.counts) {
            let capacity = (*count)
                .max(1)
                .checked_next_multiple_of(SEGMENT_ALIGN)
                .ok_or(BucketError::Capacity)?;
            self.next_segments.push(Segment {
                key: *key,
                start: end,
                len: *count,
                capacity,
            });
            // From here the count is this bucket's write cursor.
            *count = end;
            end = end.checked_add(capacity).ok_or(BucketError::Capacity)?;
        }
        if u64::from(end) > worst_members(slots, self.keys.len()).ok_or(BucketError::Capacity)? {
            return Err(BucketError::Capacity);
        }
        self.next_members.clear();
        self.next_members.resize(end as usize, PADDING);
        for (slot, bucket) in self.slot_bucket.iter().enumerate() {
            if let Some(bucket) = bucket {
                let cursor = &mut self.counts[*bucket];
                self.next_members[*cursor as usize] = slot_to_physical[slot];
                *cursor += 1;
            }
        }
        std::mem::swap(&mut self.segments, &mut self.next_segments);
        std::mem::swap(&mut self.members, &mut self.next_members);
        self.laid_out.clear();
        self.laid_out.extend_from_slice(slot_to_physical);
        self.dirty = false;
        Ok(true)
    }

    /// The largest member list any layout can need, in bytes.
    pub(super) fn worst_member_bytes(&self) -> u64 {
        self.worst_member_bytes
    }

    pub(super) fn segments(&self) -> &[Segment] {
        &self.segments
    }

    /// The concatenated, padded member list the compaction kernel reads.
    pub(super) fn members(&self) -> &[u32] {
        &self.members
    }
}

#[cfg(test)]
mod tests;
