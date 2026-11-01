use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Weak},
};

const PAGE_SIZE: usize = 32;

/// A stable game/content object identity, distinct from a runtime handle.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StableId(pub u128);
/// The identity of a controlled actor within the admitted game.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActorId(pub u64);
/// A fresh local play session identity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SessionId(pub u128);
/// A shared authoritative network match identity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct MatchId(pub u128);
/// The local identity of one constructed world, supplied fresh by the host.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WorldEpoch(pub u128);
/// Distinguishes standalone and match authority without conflating their IDs.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AuthorityId {
    /// Local standalone authority.
    Standalone(SessionId),
    /// Shared match authority.
    Match(MatchId),
}
/// Checked transient runtime reference. A stale generation never resolves.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Handle {
    /// Constructed world identity.
    pub epoch: WorldEpoch,
    /// Slot within this world.
    pub slot: u32,
    /// Generation of that slot.
    pub generation: u64,
}
/// A full object record in an immutable read model or replica publication.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityRecord<T> {
    /// Stable object identity.
    pub id: StableId,
    /// Current game-owned object state.
    pub state: T,
}
/// One complete object lifecycle/update at a commit boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EntityChange<T> {
    /// New object and all required first state.
    Spawn(EntityRecord<T>),
    /// Complete replacement state for an existing object.
    Update(EntityRecord<T>),
    /// Committed destruction.
    Despawn(StableId),
}
/// One complete, game-prepared world edit. A batch is checked in order before
/// any edit is applied, so ordinary invalid references cannot partly mutate
/// the authoritative world.
pub enum WorldEdit<T> {
    /// Insert a new stable object identity.
    Spawn(StableId, T),
    /// Replace the full state through a checked existing handle.
    Replace(Handle, T),
    /// Retire an existing object through a checked handle.
    Despawn(Handle),
    /// Retire an object spawned earlier in this same batch.
    DespawnNew(StableId),
}
/// World insertion or lookup failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorldError {
    /// A stable object identity is already active.
    DuplicateId,
    /// No free slot can be represented by a `u32`.
    SlotExhausted,
    /// A handle is stale or foreign to this world.
    InvalidHandle,
}

struct Slot<T> {
    generation: u64,
    entry: Option<EntityRecord<T>>,
}

#[derive(Clone)]
struct SnapshotEntry<T> {
    generation: u64,
    record: EntityRecord<T>,
}

type Page<T> = Arc<Vec<Option<SnapshotEntry<T>>>>;
type PageRoot<T> = Arc<[Page<T>]>;

#[derive(Clone, Copy)]
enum ChangeKind {
    Spawn,
    Update,
    Despawn,
}

#[derive(Clone, Copy)]
struct VirtualObject {
    alive: bool,
    change: Option<ChangeKind>,
    spawned_in_batch: bool,
    original_handle_alive: bool,
}

/// Exclusive authoritative object store for the finite I1 slice.
///
/// Each mutation is visible to readers only after `publish`. `get_mut` marks a
/// page dirty even if the caller ultimately makes no change.
pub struct World<T: Clone> {
    epoch: WorldEpoch,
    slots: Vec<Slot<T>>,
    free: Vec<u32>,
    by_id: HashMap<StableId, u32>,
    dirty: Vec<bool>,
    dirty_count: usize,
    changed: BTreeMap<StableId, ChangeKind>,
    pages: PageRoot<T>,
    published_tick: u64,
}

/// Counted work at a world publication boundary.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WorldWork {
    /// Immutable page references copied into the new read model.
    pub page_references: usize,
    /// Pages rebuilt because at least one slot was touched.
    pub copied_pages: usize,
    /// Live object records cloned into those pages.
    pub copied_objects: usize,
    /// Full object records copied for replica changes.
    pub group_objects: usize,
}

/// Immutable committed read model; clones retain page leases.
#[derive(Clone)]
pub struct Snapshot<T> {
    epoch: WorldEpoch,
    tick: u64,
    pages: PageRoot<T>,
    lease: Arc<()>,
}

impl<T> Snapshot<T> {
    pub(crate) fn lease_token(&self) -> Weak<()> {
        Arc::downgrade(&self.lease)
    }
    /// Local world epoch represented by this snapshot.
    pub fn epoch(&self) -> WorldEpoch {
        self.epoch
    }
    /// Last committed authority tick represented by this snapshot.
    pub fn tick(&self) -> u64 {
        self.tick
    }
    /// Resolve a checked handle against an immutable publication.
    pub fn get(&self, handle: Handle) -> Option<&EntityRecord<T>> {
        if handle.epoch != self.epoch {
            return None;
        }
        let index = handle.slot as usize;
        self.pages
            .get(index / PAGE_SIZE)?
            .get(index % PAGE_SIZE)?
            .as_ref()
            .filter(|entry| entry.generation == handle.generation)
            .map(|entry| &entry.record)
    }
    /// Enumerate complete records in slot order for a baseline or inspection.
    pub fn records(&self) -> impl Iterator<Item = &EntityRecord<T>> {
        self.pages.iter().flat_map(|page| {
            page.iter()
                .filter_map(|entry| entry.as_ref().map(|e| &e.record))
        })
    }
    /// Approximate retained payload bytes, excluding allocator and shared ownership overhead.
    pub fn logical_bytes(&self) -> usize {
        self.pages
            .iter()
            .map(|p| p.len() * std::mem::size_of::<Option<SnapshotEntry<T>>>())
            .sum()
    }
}

/// A complete immutable read model and its in-memory lifecycle projection.
pub struct WorldPublication<T> {
    /// Committed read model.
    pub snapshot: Snapshot<T>,
    /// Full state/lifecycle changes at this barrier.
    pub changes: Vec<EntityChange<T>>,
    /// Counted publication work.
    pub work: WorldWork,
}

impl<T: Clone> World<T> {
    /// Construct an empty world with a fresh host-supplied epoch.
    pub fn new(epoch: WorldEpoch) -> Self {
        Self {
            epoch,
            slots: Vec::new(),
            free: Vec::new(),
            by_id: HashMap::new(),
            dirty: Vec::new(),
            dirty_count: 0,
            changed: BTreeMap::new(),
            pages: Arc::from(Vec::new()),
            published_tick: 0,
        }
    }
    /// Local identity of this constructed world.
    pub fn epoch(&self) -> WorldEpoch {
        self.epoch
    }
    /// Number of currently live objects.
    pub fn len(&self) -> usize {
        self.by_id.len()
    }
    /// Whether the world contains no objects.
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
    /// Resolve stable identity into a checked transient handle.
    pub fn handle_for(&self, id: StableId) -> Option<Handle> {
        let slot = *self.by_id.get(&id)?;
        Some(Handle {
            epoch: self.epoch,
            slot,
            generation: self.slots[slot as usize].generation,
        })
    }
    /// Read a live object through a checked handle.
    pub fn get(&self, handle: Handle) -> Option<&EntityRecord<T>> {
        if handle.epoch != self.epoch {
            return None;
        }
        let slot = self.slots.get(handle.slot as usize)?;
        (slot.generation == handle.generation)
            .then_some(slot.entry.as_ref())
            .flatten()
    }
    /// Mutate a live object during the exclusive authority step.
    pub fn get_mut(&mut self, handle: Handle) -> Option<&mut T> {
        if handle.epoch != self.epoch {
            return None;
        }
        let index = handle.slot as usize;
        let slot = self.slots.get(index)?;
        if slot.generation != handle.generation {
            return None;
        }
        let id = slot.entry.as_ref()?.id;
        self.mark_dirty(index / PAGE_SIZE);
        self.changed.entry(id).or_insert(ChangeKind::Update);
        self.slots[index]
            .entry
            .as_mut()
            .map(|record| &mut record.state)
    }
    /// Apply one prepared edit without allocating a validation collection.
    pub fn apply_one(&mut self, edit: WorldEdit<T>) -> Result<(), WorldError> {
        match edit {
            WorldEdit::Spawn(id, state) => self.spawn(id, state).map(|_| ()),
            WorldEdit::Replace(handle, state) => {
                *self.get_mut(handle).ok_or(WorldError::InvalidHandle)? = state;
                Ok(())
            }
            WorldEdit::Despawn(handle) => self.despawn(handle).map(|_| ()),
            WorldEdit::DespawnNew(_) => Err(WorldError::InvalidHandle),
        }
    }
    /// Validate a complete prepared batch before applying its ordered edits.
    /// A structural peak may exceed final live objects, so admission callers
    /// must reserve for every possible new slot before invoking this method.
    pub fn apply_prepared(&mut self, mut edits: Vec<WorldEdit<T>>) -> Result<(), WorldError> {
        if edits.len() == 1 {
            return self.apply_one(edits.pop().expect("one prepared edit"));
        }
        let mut virtual_objects = HashMap::with_capacity(edits.len());
        for edit in &edits {
            let id = match edit {
                WorldEdit::Spawn(id, _) | WorldEdit::DespawnNew(id) => *id,
                WorldEdit::Replace(handle, _) | WorldEdit::Despawn(handle) => {
                    self.get(*handle).ok_or(WorldError::InvalidHandle)?.id
                }
            };
            let object = virtual_objects.entry(id).or_insert_with(|| {
                let alive = self.by_id.contains_key(&id);
                VirtualObject {
                    alive,
                    change: self.changed.get(&id).copied(),
                    spawned_in_batch: false,
                    original_handle_alive: alive,
                }
            });
            match edit {
                WorldEdit::Spawn(..) => {
                    if object.alive || matches!(object.change, Some(ChangeKind::Despawn)) {
                        return Err(WorldError::DuplicateId);
                    }
                    object.alive = true;
                    object.change = Some(ChangeKind::Spawn);
                    object.spawned_in_batch = true;
                }
                WorldEdit::Replace(..) => {
                    if !object.alive || !object.original_handle_alive {
                        return Err(WorldError::InvalidHandle);
                    }
                    if !matches!(object.change, Some(ChangeKind::Spawn)) {
                        object.change = Some(ChangeKind::Update);
                    }
                }
                WorldEdit::Despawn(..) => {
                    if !object.alive || !object.original_handle_alive {
                        return Err(WorldError::InvalidHandle);
                    }
                    object.alive = false;
                    object.original_handle_alive = false;
                    object.change = if matches!(object.change, Some(ChangeKind::Spawn)) {
                        None
                    } else {
                        Some(ChangeKind::Despawn)
                    };
                }
                WorldEdit::DespawnNew(..) => {
                    if !object.alive || !object.spawned_in_batch {
                        return Err(WorldError::InvalidHandle);
                    }
                    object.alive = false;
                    object.change = if matches!(object.change, Some(ChangeKind::Spawn)) {
                        None
                    } else {
                        Some(ChangeKind::Despawn)
                    };
                }
            }
        }
        self.validate_prepared_slot_capacity(&edits)?;
        for edit in edits {
            match edit {
                WorldEdit::Spawn(id, state) => {
                    self.spawn(id, state).expect("validated prepared spawn");
                }
                WorldEdit::Replace(handle, state) => {
                    *self.get_mut(handle).expect("validated prepared handle") = state;
                }
                WorldEdit::Despawn(handle) => {
                    self.despawn(handle).expect("validated prepared handle");
                }
                WorldEdit::DespawnNew(id) => {
                    let handle = self.handle_for(id).expect("validated new object");
                    self.despawn(handle).expect("validated new object handle");
                }
            }
        }
        Ok(())
    }
    fn validate_prepared_slot_capacity(&self, edits: &[WorldEdit<T>]) -> Result<(), WorldError> {
        let spawns = edits
            .iter()
            .filter(|edit| matches!(edit, WorldEdit::Spawn(..)))
            .count();
        if spawns == 0
            || self
                .slots
                .len()
                .checked_add(spawns - 1)
                .is_some_and(|last| u32::try_from(last).is_ok())
        {
            return Ok(());
        }
        // Only a world near u32 slot exhaustion needs to simulate reuse. The
        // ordinary path above is O(1) and never clones the free list.
        let mut free = self.free.clone();
        let mut generations = HashMap::new();
        let mut spawned = HashMap::new();
        let mut appended = 0_usize;
        for edit in edits {
            match edit {
                WorldEdit::Spawn(id, _) => {
                    let slot = if let Some(slot) = free.pop() {
                        slot
                    } else {
                        let index = self
                            .slots
                            .len()
                            .checked_add(appended)
                            .ok_or(WorldError::SlotExhausted)?;
                        let slot = u32::try_from(index).map_err(|_| WorldError::SlotExhausted)?;
                        appended += 1;
                        slot
                    };
                    let generation = generations.get(&slot).copied().unwrap_or_else(|| {
                        self.slots
                            .get(slot as usize)
                            .map_or(0, |old| old.generation)
                    });
                    spawned.insert(*id, (slot, generation));
                }
                WorldEdit::Despawn(handle) => {
                    if let Some(next) = handle.generation.checked_add(1) {
                        generations.insert(handle.slot, next);
                        free.push(handle.slot);
                    }
                }
                WorldEdit::DespawnNew(id) => {
                    let (slot, generation) = spawned.remove(id).expect("validated new object");
                    if let Some(next) = generation.checked_add(1) {
                        generations.insert(slot, next);
                        free.push(slot);
                    }
                }
                WorldEdit::Replace(..) => {}
            }
        }
        Ok(())
    }
    /// Insert a complete game-owned object; stable IDs cannot alias.
    pub fn spawn(&mut self, id: StableId, state: T) -> Result<Handle, WorldError> {
        if self.by_id.contains_key(&id) {
            return Err(WorldError::DuplicateId);
        }
        if matches!(self.changed.get(&id), Some(ChangeKind::Despawn)) {
            return Err(WorldError::DuplicateId);
        }
        let slot = if let Some(slot) = self.free.pop() {
            slot
        } else {
            let slot = u32::try_from(self.slots.len()).map_err(|_| WorldError::SlotExhausted)?;
            self.slots.push(Slot {
                generation: 0,
                entry: None,
            });
            if self.slots.len() % PAGE_SIZE == 1 {
                self.dirty.push(false);
            }
            slot
        };
        let index = slot as usize;
        self.slots[index].entry = Some(EntityRecord { id, state });
        self.by_id.insert(id, slot);
        self.mark_dirty(index / PAGE_SIZE);
        self.changed.insert(id, ChangeKind::Spawn);
        Ok(Handle {
            epoch: self.epoch,
            slot,
            generation: self.slots[index].generation,
        })
    }
    /// Destroy an object and invalidate its runtime handle.
    pub fn despawn(&mut self, handle: Handle) -> Result<EntityRecord<T>, WorldError> {
        let record = self.get(handle).ok_or(WorldError::InvalidHandle)?.clone();
        let index = handle.slot as usize;
        self.mark_dirty(index / PAGE_SIZE);
        let slot = &mut self.slots[index];
        slot.entry = None;
        self.by_id.remove(&record.id);
        if matches!(self.changed.get(&record.id), Some(ChangeKind::Spawn)) {
            self.changed.remove(&record.id);
        } else {
            self.changed.insert(record.id, ChangeKind::Despawn);
        }
        if let Some(next) = slot.generation.checked_add(1) {
            slot.generation = next;
            self.free.push(handle.slot);
        }
        Ok(record)
    }
    fn mark_dirty(&mut self, page_index: usize) {
        if !self.dirty[page_index] {
            self.dirty[page_index] = true;
            self.dirty_count += 1;
        }
    }

    /// Build the next immutable state and complete changes at the tick barrier.
    pub fn publish(&mut self, tick: u64) -> WorldPublication<T> {
        assert!(tick >= self.published_tick, "tick cannot move backwards");
        let count = self.slots.len().div_ceil(PAGE_SIZE);
        let mut work = WorldWork {
            ..WorldWork::default()
        };
        if self.dirty_count != 0 {
            let mut pages: Vec<_> = self.pages.iter().cloned().collect();
            work.page_references = count;
            for page_index in 0..count {
                if !self.dirty[page_index] {
                    continue;
                }
                let start = page_index * PAGE_SIZE;
                let end = (start + PAGE_SIZE).min(self.slots.len());
                let mut entries = Vec::with_capacity(end - start);
                for slot in &self.slots[start..end] {
                    if slot.entry.is_some() {
                        work.copied_objects += 1;
                    }
                    entries.push(slot.entry.as_ref().map(|record| SnapshotEntry {
                        generation: slot.generation,
                        record: record.clone(),
                    }));
                }
                let page = Arc::new(entries);
                if page_index == pages.len() {
                    pages.push(page);
                } else {
                    pages[page_index] = page;
                }
                self.dirty[page_index] = false;
                work.copied_pages += 1;
            }
            self.pages = Arc::from(pages);
            self.dirty_count = 0;
        }
        let mut changes = Vec::with_capacity(self.changed.len());
        for (id, kind) in std::mem::take(&mut self.changed) {
            match kind {
                ChangeKind::Spawn | ChangeKind::Update => {
                    let slot = self.by_id[&id] as usize;
                    let record = self.slots[slot]
                        .entry
                        .as_ref()
                        .expect("live indexed object")
                        .clone();
                    work.group_objects += 1;
                    changes.push(if matches!(kind, ChangeKind::Spawn) {
                        EntityChange::Spawn(record)
                    } else {
                        EntityChange::Update(record)
                    });
                }
                ChangeKind::Despawn => changes.push(EntityChange::Despawn(id)),
            }
        }
        self.published_tick = tick;
        WorldPublication {
            snapshot: Snapshot {
                epoch: self.epoch,
                tick,
                pages: self.pages.clone(),
                lease: Arc::new(()),
            },
            changes,
            work,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepared_batch_does_not_reuse_a_retired_generation() {
        let mut world = World::new(WorldEpoch(1));
        let original = world.spawn(StableId(1), 10_u64).unwrap();
        world.publish(0);
        world.slots[original.slot as usize].generation = u64::MAX;
        let retiring = Handle {
            generation: u64::MAX,
            ..original
        };

        world
            .apply_prepared(vec![
                WorldEdit::Despawn(retiring),
                WorldEdit::Spawn(StableId(2), 20),
            ])
            .unwrap();

        let new_handle = world.handle_for(StableId(2)).unwrap();
        assert_ne!(new_handle.slot, retiring.slot);
        assert_eq!(world.get(retiring), None);
        assert_eq!(world.get(new_handle).unwrap().state, 20);
        assert!(world.free.is_empty());
    }
}
