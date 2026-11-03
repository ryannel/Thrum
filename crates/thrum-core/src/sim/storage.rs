//! Private hecs component backend with engine-owned stable identity and public
//! handle generations. Hecs entity values never leave this module.
use std::{collections::hash_map::RandomState, hash::BuildHasher};

use crate::{Handle, StableId, WorldEpoch};

use super::{ChangeTicks, ComponentEntity, ComponentView, Ownership, Position, SimError, Velocity};

struct Slot {
    generation: u64,
    row: Option<usize>,
    entity: Option<hecs::Entity>,
}

#[derive(Clone, Copy)]
struct SlotKey(u32);
#[derive(Clone, Copy)]
struct Health(u16);
#[derive(Clone, Copy)]
struct Visual(u64);

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
    row_slots: Vec<u32>,
    world: hecs::World,
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
                    entity: None,
                })
                .collect(),
            free: (0..u32::try_from(capacity).expect("validated entity capacity"))
                .rev()
                .collect(),
            ids: IdIndex::new(capacity),
            stable_ids: Vec::with_capacity(capacity),
            row_slots: Vec::with_capacity(capacity),
            world: hecs::World::new(),
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

    pub(super) fn view(&self, row: usize) -> ComponentView {
        let slot = self.row_slots[row];
        ComponentView {
            handle: self.slot_handle(slot),
            id: self.id_at(row),
            position: self.position(row),
            velocity: self.velocity(row),
            health: self.health(row),
            ownership: self.ownership(row),
            visual: self.visual(row),
            changes: self.changes[row],
        }
    }

    // ID is stored beside its private slot, separately from hot numeric columns.
    pub(super) fn id_at(&self, row: usize) -> StableId {
        self.stable_ids[row]
    }

    fn entity_at(&self, row: usize) -> hecs::Entity {
        self.slots[self.row_slots[row] as usize]
            .entity
            .expect("live row has a private entity")
    }

    pub(super) fn position(&self, row: usize) -> Position {
        *self
            .world
            .get::<&Position>(self.entity_at(row))
            .expect("live position")
    }

    pub(super) fn velocity(&self, row: usize) -> Velocity {
        *self
            .world
            .get::<&Velocity>(self.entity_at(row))
            .expect("live velocity")
    }

    pub(super) fn health(&self, row: usize) -> u16 {
        self.world
            .get::<&Health>(self.entity_at(row))
            .expect("live health")
            .0
    }

    pub(super) fn ownership(&self, row: usize) -> Option<Ownership> {
        self.world
            .get::<&Ownership>(self.entity_at(row))
            .ok()
            .map(|v| *v)
    }

    pub(super) fn visual(&self, row: usize) -> Option<u64> {
        self.world
            .get::<&Visual>(self.entity_at(row))
            .ok()
            .map(|v| v.0)
    }

    pub(super) fn set_position(&mut self, row: usize, value: Position) {
        *self
            .world
            .get::<&mut Position>(self.entity_at(row))
            .expect("live position") = value;
    }

    pub(super) fn set_velocity(&mut self, row: usize, value: Velocity) {
        *self
            .world
            .get::<&mut Velocity>(self.entity_at(row))
            .expect("live velocity") = value;
    }

    pub(super) fn set_health(&mut self, row: usize, value: u16) {
        self.world
            .get::<&mut Health>(self.entity_at(row))
            .expect("live health")
            .0 = value;
    }

    pub(super) fn for_each_motion(&self, mut visit: impl FnMut(usize, Position, Velocity)) {
        for (key, position, velocity) in self
            .world
            .query::<(&SlotKey, &Position, &Velocity)>()
            .iter()
        {
            let row = self.slots[key.0 as usize].row.expect("live slot has a row");
            visit(row, *position, *velocity);
        }
    }

    pub(super) fn copy_snapshot_into(&self, output: &mut Vec<ComponentEntity>) {
        output.clear();
        for (key, position, velocity, health, ownership, visual) in self
            .world
            .query::<(
                &SlotKey,
                &Position,
                &Velocity,
                &Health,
                Option<&Ownership>,
                Option<&Visual>,
            )>()
            .iter()
        {
            let row = self.slots[key.0 as usize].row.expect("live slot has a row");
            output.push(ComponentEntity {
                id: self.id_at(row),
                position: *position,
                velocity: *velocity,
                health: health.0,
                ownership: ownership.copied(),
                visual: visual.map(|v| v.0),
            });
        }
    }

    pub(super) fn spawn(&mut self, entity: ComponentEntity, tick: u64) -> Handle {
        let slot = self.free.pop().expect("prevalidated physical capacity");
        let row = self.len();
        self.slots[slot as usize].row = Some(row);
        self.ids.insert(entity.id, slot);
        self.row_slots.push(slot);
        self.stable_ids.push(entity.id);
        let base = (
            SlotKey(slot),
            entity.position,
            entity.velocity,
            Health(entity.health),
        );
        let private = match (entity.ownership, entity.visual) {
            (Some(owner), Some(visual)) => {
                self.world
                    .spawn((base.0, base.1, base.2, base.3, owner, Visual(visual)))
            }
            (Some(owner), None) => self.world.spawn((base.0, base.1, base.2, base.3, owner)),
            (None, Some(visual)) => {
                self.world
                    .spawn((base.0, base.1, base.2, base.3, Visual(visual)))
            }
            (None, None) => self.world.spawn(base),
        };
        self.slots[slot as usize].entity = Some(private);
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
        self.world
            .despawn(self.entity_at(row))
            .expect("live private entity");
        self.ids.remove(self.stable_ids[row]);
        self.row_slots.swap_remove(row);
        self.stable_ids.swap_remove(row);
        self.changes.swap_remove(row);
        if row < self.len() {
            self.slots[self.row_slots[row] as usize].row = Some(row);
        }
        let slot = &mut self.slots[handle.slot as usize];
        slot.row = None;
        slot.entity = None;
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
