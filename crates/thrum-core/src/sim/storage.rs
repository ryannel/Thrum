//! Private narrow SoA candidate, replaceable by A0 storage adapters.
use std::{collections::hash_map::RandomState, hash::BuildHasher};

use crate::{Handle, StableId, WorldEpoch};

use super::{ChangeTicks, ComponentEntity, ComponentView, Ownership, Position, SimError, Velocity};

struct Slot {
    generation: u64,
    row: Option<usize>,
}

/// Fixed-capacity stable-ID index. Empty buckets are restored by backward-shift
/// deletion, so long-running unique-ID churn cannot accumulate tombstones or
/// trigger a surprise rehash. At most half the buckets are occupied.
struct IdIndex {
    buckets: Vec<Option<(StableId, u32)>>,
    mask: usize,
    hash: RandomState,
}

impl IdIndex {
    fn new(max_live: usize) -> Self {
        let bucket_count = max_live
            .checked_mul(2)
            .and_then(usize::checked_next_power_of_two)
            .unwrap_or_else(|| panic!("entity capacity exceeds fixed ID index address space"))
            .max(1);
        Self {
            buckets: vec![None; bucket_count],
            mask: bucket_count - 1,
            hash: RandomState::new(),
        }
    }

    fn home(&self, id: StableId) -> usize {
        usize::try_from(
            self.hash.hash_one(id) & u64::try_from(self.mask).expect("index mask fits u64"),
        )
        .expect("masked hash fits usize")
    }

    fn find(&self, id: StableId) -> Option<usize> {
        let mut bucket = self.home(id);
        loop {
            match self.buckets[bucket] {
                Some((found, _)) if found == id => return Some(bucket),
                None => return None,
                _ => bucket = (bucket + 1) & self.mask,
            }
        }
    }

    fn get(&self, id: StableId) -> Option<u32> {
        self.find(id)
            .map(|bucket| self.buckets[bucket].expect("found occupied bucket").1)
    }

    fn contains(&self, id: StableId) -> bool {
        self.find(id).is_some()
    }

    fn insert(&mut self, id: StableId, slot: u32) {
        let mut bucket = self.home(id);
        loop {
            match self.buckets[bucket] {
                None => {
                    self.buckets[bucket] = Some((id, slot));
                    return;
                }
                Some((found, _)) => debug_assert_ne!(found, id, "prevalidated unique stable ID"),
            }
            bucket = (bucket + 1) & self.mask;
        }
    }

    fn remove(&mut self, id: StableId) {
        let mut hole = self.find(id).expect("prevalidated live stable ID");
        let mut next = (hole + 1) & self.mask;
        while let Some((key, slot)) = self.buckets[next] {
            let home = self.home(key);
            let next_distance = (next.wrapping_sub(home)) & self.mask;
            let hole_distance = (hole.wrapping_sub(home)) & self.mask;
            if hole_distance < next_distance {
                self.buckets[hole] = Some((key, slot));
                hole = next;
            }
            next = (next + 1) & self.mask;
        }
        self.buckets[hole] = None;
    }
}

pub(super) struct Store {
    epoch: WorldEpoch,
    slots: Vec<Slot>,
    free: Vec<u32>,
    ids: IdIndex,
    stable_ids: Vec<StableId>,
    pub(super) row_slots: Vec<u32>,
    pub(super) positions: Vec<Position>,
    pub(super) velocities: Vec<Velocity>,
    pub(super) health: Vec<u16>,
    pub(super) ownership: Vec<Option<Ownership>>,
    pub(super) visuals: Vec<Option<u64>>,
    pub(super) changes: Vec<ChangeTicks>,
}

impl Store {
    pub(super) fn new(epoch: WorldEpoch, capacity: usize) -> Self {
        Self {
            epoch,
            slots: (0..capacity)
                .map(|_| Slot {
                    generation: 1,
                    row: None,
                })
                .collect(),
            free: (0..u32::try_from(capacity).expect("validated entity capacity"))
                .rev()
                .collect(),
            ids: IdIndex::new(capacity),
            stable_ids: Vec::with_capacity(capacity),
            row_slots: Vec::with_capacity(capacity),
            positions: Vec::with_capacity(capacity),
            velocities: Vec::with_capacity(capacity),
            health: Vec::with_capacity(capacity),
            ownership: Vec::with_capacity(capacity),
            visuals: Vec::with_capacity(capacity),
            changes: Vec::with_capacity(capacity),
        }
    }

    pub(super) fn len(&self) -> usize {
        self.row_slots.len()
    }
    pub(super) fn free_count(&self) -> usize {
        self.free.len()
    }
    pub(super) fn reusable_after_despawn(&self, handle: Handle) -> bool {
        self.slots[handle.slot as usize].generation != u64::MAX
    }
    pub(super) fn contains(&self, id: StableId) -> bool {
        self.ids.contains(id)
    }

    pub(super) fn handle(&self, id: StableId) -> Option<Handle> {
        self.ids.get(id).map(|slot| self.slot_handle(slot))
    }

    fn slot_handle(&self, slot: u32) -> Handle {
        Handle {
            epoch: self.epoch,
            slot,
            generation: self.slots[slot as usize].generation,
        }
    }

    pub(super) fn row(&self, handle: Handle) -> Result<usize, SimError> {
        if handle.epoch != self.epoch {
            return Err(SimError::InvalidHandle);
        }
        let slot = self
            .slots
            .get(handle.slot as usize)
            .ok_or(SimError::InvalidHandle)?;
        if slot.generation != handle.generation {
            return Err(SimError::InvalidHandle);
        }
        slot.row.ok_or(SimError::InvalidHandle)
    }

    pub(super) fn view(&self, row: usize) -> ComponentView<'_> {
        let slot = self.row_slots[row];
        ComponentView {
            handle: self.slot_handle(slot),
            id: self.id_at(row),
            position: &self.positions[row],
            velocity: &self.velocities[row],
            health: self.health[row],
            ownership: self.ownership[row],
            visual: self.visuals[row],
            changes: self.changes[row],
        }
    }

    // ID is stored beside its private slot, separately from hot numeric columns.
    pub(super) fn id_at(&self, row: usize) -> StableId {
        self.stable_ids[row]
    }

    pub(super) fn spawn(&mut self, entity: ComponentEntity, tick: u64) -> Handle {
        let slot = self.free.pop().expect("prevalidated physical capacity");
        let row = self.len();
        self.slots[slot as usize].row = Some(row);
        self.ids.insert(entity.id, slot);
        self.row_slots.push(slot);
        self.stable_ids.push(entity.id);
        self.positions.push(entity.position);
        self.velocities.push(entity.velocity);
        self.health.push(entity.health);
        self.ownership.push(entity.ownership);
        self.visuals.push(entity.visual);
        self.changes.push(ChangeTicks {
            position: tick,
            velocity: tick,
            health: tick,
            structural: tick,
        });
        self.slot_handle(slot)
    }

    pub(super) fn despawn(&mut self, handle: Handle) {
        let row = self.row(handle).expect("prevalidated handle");
        self.ids.remove(self.stable_ids[row]);
        self.row_slots.swap_remove(row);
        self.stable_ids.swap_remove(row);
        self.positions.swap_remove(row);
        self.velocities.swap_remove(row);
        self.health.swap_remove(row);
        self.ownership.swap_remove(row);
        self.visuals.swap_remove(row);
        self.changes.swap_remove(row);
        if row < self.len() {
            self.slots[self.row_slots[row] as usize].row = Some(row);
        }
        let slot = &mut self.slots[handle.slot as usize];
        slot.row = None;
        // Generation overflow retires the slot permanently.
        if let Some(next) = slot.generation.checked_add(1) {
            slot.generation = next;
            self.free.push(handle.slot);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{IdIndex, StableId};

    #[test]
    fn backward_shift_keeps_colliding_ids_reachable_after_head_and_middle_removal() {
        let mut index = IdIndex::new(8);
        // Start at the last bucket so the probe run wraps across index zero.
        let bucket = index.mask;
        let ids: Vec<_> = (0..10_000u128)
            .map(StableId)
            .filter(|&id| index.home(id) == bucket)
            .take(5)
            .collect();
        assert_eq!(ids.len(), 5);
        for (slot, &id) in ids[..4].iter().enumerate() {
            index.insert(id, u32::try_from(slot).expect("small fixture slot"));
        }
        index.remove(ids[0]);
        assert_eq!(index.get(ids[0]), None);
        for (slot, &id) in ids[1..4].iter().enumerate() {
            assert_eq!(
                index.get(id),
                Some(u32::try_from(slot + 1).expect("small fixture slot"))
            );
        }
        index.insert(ids[4], 4);
        index.remove(ids[2]);
        assert_eq!(index.get(ids[2]), None);
        assert_eq!(index.get(ids[1]), Some(1));
        assert_eq!(index.get(ids[3]), Some(3));
        assert_eq!(index.get(ids[4]), Some(4));
    }
}
