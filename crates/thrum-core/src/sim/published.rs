//! Immutable committed pages shared by presentation, replication and history.
//! Physical slots are indices; stable IDs are values and never indices.

use std::{mem::size_of, sync::Arc};

use crate::{Handle, StableId, WorldEpoch};

use super::{ChangeTicks, Command, Ownership, Position, Velocity, storage::Store};

const PAGE_COUNT: usize = 4;

/// Identity of one complete committed endpoint.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnapshotId {
    /// Constructed world generation.
    pub epoch: WorldEpoch,
    /// Committed fixed tick.
    pub tick: u64,
}

/// Structural events retain order even when a despawn and spawn reuse a slot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StructuralChange {
    /// Retired checked handle.
    Despawn(Handle),
    /// Newly live checked handle.
    Spawn(Handle),
}

/// Component dirty slots for one committed tick. Entries are unique per column.
#[derive(Default)]
pub struct DirtySlots {
    position: Vec<u32>,
    velocity: Vec<u32>,
    health: Vec<u32>,
    structural: Vec<u32>,
}

impl DirtySlots {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            position: Vec::with_capacity(capacity),
            velocity: Vec::with_capacity(capacity),
            health: Vec::with_capacity(capacity),
            structural: Vec::with_capacity(capacity),
        }
    }

    fn clear(&mut self) {
        self.position.clear();
        self.velocity.clear();
        self.health.clear();
        self.structural.clear();
    }

    /// Slots whose position changed.
    pub fn position(&self) -> &[u32] {
        &self.position
    }
    /// Slots whose velocity changed.
    pub fn velocity(&self) -> &[u32] {
        &self.velocity
    }
    /// Slots whose health changed.
    pub fn health(&self) -> &[u32] {
        &self.health
    }
    /// Slots with a spawn or despawn.
    pub fn structural(&self) -> &[u32] {
        &self.structural
    }
}

struct Page {
    id: SnapshotId,
    ids: Vec<Option<StableId>>,
    generations: Vec<u64>,
    positions: Vec<Position>,
    velocities: Vec<Velocity>,
    health: Vec<u16>,
    ownership: Vec<Option<Ownership>>,
    visuals: Vec<Option<u64>>,
    dirty: DirtySlots,
    events: Vec<StructuralChange>,
    live: usize,
    initialized: bool,
}

impl Page {
    fn with_capacity(epoch: WorldEpoch, capacity: usize) -> Self {
        Self {
            id: SnapshotId { epoch, tick: 0 },
            ids: vec![None; capacity],
            generations: vec![0; capacity],
            positions: vec![Position::default(); capacity],
            velocities: vec![Velocity::default(); capacity],
            health: vec![0; capacity],
            ownership: vec![None; capacity],
            visuals: vec![None; capacity],
            dirty: DirtySlots::with_capacity(capacity),
            events: Vec::with_capacity(capacity.saturating_mul(2)),
            live: 0,
            initialized: false,
        }
    }

    fn reserved_bytes(&self) -> usize {
        fn bytes<T>(values: &Vec<T>) -> usize {
            values.capacity() * size_of::<T>()
        }
        size_of::<Self>()
            + bytes(&self.ids)
            + bytes(&self.generations)
            + bytes(&self.positions)
            + bytes(&self.velocities)
            + bytes(&self.health)
            + bytes(&self.ownership)
            + bytes(&self.visuals)
            + bytes(&self.dirty.position)
            + bytes(&self.dirty.velocity)
            + bytes(&self.dirty.health)
            + bytes(&self.dirty.structural)
            + bytes(&self.events)
    }

    fn capture(
        &mut self,
        store: &Store,
        epoch: WorldEpoch,
        tick: u64,
        commands: &[Command],
        first: bool,
    ) {
        let since = self.initialized.then_some(self.id.tick);
        self.id = SnapshotId { epoch, tick };
        self.dirty.clear();
        self.events.clear();
        self.live = store.len();
        store.visit_changed_since(since, |view| {
            let index = view.handle.slot as usize;
            let structural = since.is_none_or(|old| view.changes.structural > old);
            if structural {
                self.ids[index] = Some(view.id);
                self.generations[index] = view.handle.generation;
                self.ownership[index] = view.ownership;
                self.visuals[index] = view.visual;
            }
            if structural || since.is_none_or(|old| view.changes.position > old) {
                self.positions[index] = view.position;
            }
            if structural || since.is_none_or(|old| view.changes.velocity > old) {
                self.velocities[index] = view.velocity;
            }
            if structural || since.is_none_or(|old| view.changes.health > old) {
                self.health[index] = view.health;
            }
            let ChangeTicks {
                position,
                velocity,
                health,
                structural,
            } = view.changes;
            if first || position == tick {
                self.dirty.position.push(view.handle.slot);
            }
            if first || velocity == tick {
                self.dirty.velocity.push(view.handle.slot);
            }
            if first || health == tick {
                self.dirty.health.push(view.handle.slot);
            }
            if first || structural == tick {
                self.dirty.structural.push(view.handle.slot);
            }
        });
        // An old page can predate several intervening removals. Removed slots
        // have no live change tick, so clear only their stale liveness bits.
        for (slot, id) in self.ids.iter_mut().enumerate() {
            if id.is_some() && !store.slot_live(slot) {
                *id = None;
            }
        }
        if first {
            for (index, id) in self.ids.iter().enumerate() {
                if id.is_some() {
                    let slot = u32::try_from(index).expect("validated physical slot capacity");
                    self.events.push(StructuralChange::Spawn(Handle {
                        epoch,
                        slot,
                        generation: self.generations[index],
                    }));
                }
            }
        } else {
            for command in commands {
                if let Command::Despawn(handle) = command {
                    self.events.push(StructuralChange::Despawn(*handle));
                    if self.ids[handle.slot as usize].is_none() {
                        self.dirty.structural.push(handle.slot);
                    }
                }
            }
            for command in commands {
                if let Command::Spawn(entity) = command {
                    let handle = store
                        .handle(entity.id)
                        .expect("committed spawn has a handle");
                    self.events.push(StructuralChange::Spawn(handle));
                }
            }
        }
        self.dirty.structural.sort_unstable();
        self.dirty.structural.dedup();
        self.initialized = true;
    }
}

/// Shared immutable lease. It survives later ticks and physical-slot reuse.
#[derive(Clone)]
pub struct PublishedPage(Arc<Page>);

impl PublishedPage {
    /// Complete source identity.
    pub fn id(&self) -> SnapshotId {
        self.0.id
    }
    /// Number of live components.
    pub fn len(&self) -> usize {
        self.0.live
    }
    /// Whether the page contains no live components.
    pub fn is_empty(&self) -> bool {
        self.0.live == 0
    }
    /// Number of physical slots in the world.
    pub fn slot_capacity(&self) -> usize {
        self.0.ids.len()
    }
    /// Stable identity at a physical slot, if live.
    pub fn stable_id(&self, slot: u32) -> Option<StableId> {
        self.0.ids.get(slot as usize).copied().flatten()
    }
    /// Checked current handle at a physical slot, if live.
    pub fn handle(&self, slot: u32) -> Option<Handle> {
        self.stable_id(slot).map(|_| Handle {
            epoch: self.0.id.epoch,
            slot,
            generation: self.0.generations[slot as usize],
        })
    }
    /// Position at a matching live handle.
    pub fn position(&self, handle: Handle) -> Option<Position> {
        self.matches(handle)
            .then(|| self.0.positions[handle.slot as usize])
    }
    /// Velocity at a matching live handle.
    pub fn velocity(&self, handle: Handle) -> Option<Velocity> {
        self.matches(handle)
            .then(|| self.0.velocities[handle.slot as usize])
    }
    /// Health at a matching live handle.
    pub fn health(&self, handle: Handle) -> Option<u16> {
        self.matches(handle)
            .then(|| self.0.health[handle.slot as usize])
    }
    /// Ownership at a matching live handle; absent for unowned or stale slots.
    pub fn ownership(&self, handle: Handle) -> Option<Ownership> {
        self.matches(handle)
            .then(|| self.0.ownership[handle.slot as usize])
            .flatten()
    }
    /// Visual revision at a matching live handle; absent for invisible or stale slots.
    pub fn visual(&self, handle: Handle) -> Option<u64> {
        self.matches(handle)
            .then(|| self.0.visuals[handle.slot as usize])
            .flatten()
    }
    /// Per-component changes in this tick.
    pub fn dirty(&self) -> &DirtySlots {
        &self.0.dirty
    }
    /// Ordered structural changes in this tick.
    pub fn structural_events(&self) -> &[StructuralChange] {
        &self.0.events
    }
    /// Visit live slots without allocation or stable-ID lookup.
    /// # Panics
    /// Panics only if an internally constructed page exceeds the validated slot limit.
    pub fn visit(&self, mut visit: impl FnMut(Handle, StableId, Position, Option<u64>)) {
        for (index, id) in self.0.ids.iter().enumerate() {
            if let Some(id) = id {
                let slot = u32::try_from(index).expect("validated physical slot capacity");
                let handle = Handle {
                    epoch: self.0.id.epoch,
                    slot,
                    generation: self.0.generations[index],
                };
                visit(handle, *id, self.0.positions[index], self.0.visuals[index]);
            }
        }
    }
    fn matches(&self, handle: Handle) -> bool {
        handle.epoch == self.0.id.epoch
            && self
                .0
                .ids
                .get(handle.slot as usize)
                .is_some_and(Option::is_some)
            && self.0.generations[handle.slot as usize] == handle.generation
    }
}

pub(super) struct PublicationRing {
    pages: Vec<Arc<Page>>,
    cursor: usize,
    latest: Option<PublishedPage>,
    previous: Option<PublishedPage>,
}

impl PublicationRing {
    pub(super) fn new(epoch: WorldEpoch, capacity: usize) -> Self {
        Self {
            pages: (0..PAGE_COUNT)
                .map(|_| Arc::new(Page::with_capacity(epoch, capacity)))
                .collect(),
            cursor: 0,
            latest: None,
            previous: None,
        }
    }

    pub(super) fn can_capture(&self) -> bool {
        self.pages.iter().any(|page| Arc::strong_count(page) == 1)
    }

    pub(super) fn capture(
        &mut self,
        store: &Store,
        epoch: WorldEpoch,
        tick: u64,
        commands: &[Command],
    ) {
        let index = (0..self.pages.len())
            .map(|offset| (self.cursor + offset) % self.pages.len())
            .find(|&index| Arc::strong_count(&self.pages[index]) == 1)
            .expect("page reserved before world commit");
        let first = self.latest.is_none();
        Arc::get_mut(&mut self.pages[index])
            .expect("unleased page")
            .capture(store, epoch, tick, commands, first);
        self.cursor = (index + 1) % self.pages.len();
        self.previous = self.latest.take();
        self.latest = Some(PublishedPage(Arc::clone(&self.pages[index])));
    }

    pub(super) fn latest(&self) -> Option<PublishedPage> {
        self.latest.clone()
    }
    pub(super) fn previous(&self) -> Option<PublishedPage> {
        self.previous.clone()
    }

    pub(super) fn reserved_bytes(&self) -> usize {
        size_of::<Self>()
            + self.pages.capacity() * size_of::<Arc<Page>>()
            + self
                .pages
                .iter()
                .map(|page| page.reserved_bytes() + 2 * size_of::<usize>())
                .sum::<usize>()
    }
}
