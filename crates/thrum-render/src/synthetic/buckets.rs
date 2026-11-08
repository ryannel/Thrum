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
#![cfg_attr(
    not(test),
    expect(dead_code, reason = "M3b buckets; wiring waits for root's review")
)]

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
    /// Slot counts or member offsets do not fit in u32.
    Capacity,
}

/// Per-slot bucket keys and the member list they lay out.
pub(super) struct Buckets {
    /// Bucket index per key, in key order.
    index: BTreeMap<BucketKey, usize>,
    keys: Vec<BucketKey>,
    /// Each logical slot's bucket, or `None` while inactive.
    slot_bucket: Vec<Option<usize>>,
    segments: Vec<Segment>,
    members: Vec<u32>,
}

impl Buckets {
    /// Fix the bucket set. Duplicate keys collapse; order is key order.
    pub(super) fn new(keys: impl IntoIterator<Item = BucketKey>, slots: usize) -> Self {
        let mut keys: Vec<_> = keys.into_iter().collect();
        keys.sort_unstable();
        keys.dedup();
        let index = keys.iter().enumerate().map(|(i, &key)| (key, i)).collect();
        Self {
            index,
            keys,
            slot_bucket: vec![None; slots],
            segments: Vec::new(),
            members: Vec::new(),
        }
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
        *entry = bucket;
        Ok(())
    }

    /// Lay out every segment from the upload's current physical rows.
    pub(super) fn layout(&mut self, slot_to_physical: &[u32]) -> Result<(), BucketError> {
        let mut lists = vec![Vec::new(); self.keys.len()];
        for (slot, bucket) in self.slot_bucket.iter().enumerate() {
            if let Some(bucket) = bucket {
                lists[*bucket].push(slot_to_physical[slot]);
            }
        }
        self.segments.clear();
        self.members.clear();
        for (key, list) in self.keys.iter().zip(lists) {
            let start = u32::try_from(self.members.len()).map_err(|_| BucketError::Capacity)?;
            let len = u32::try_from(list.len()).map_err(|_| BucketError::Capacity)?;
            let capacity = len
                .max(1)
                .checked_next_multiple_of(SEGMENT_ALIGN)
                .ok_or(BucketError::Capacity)?;
            self.members.extend(list);
            self.members.resize((start + capacity) as usize, PADDING);
            self.segments.push(Segment {
                key: *key,
                start,
                len,
                capacity,
            });
        }
        Ok(())
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
