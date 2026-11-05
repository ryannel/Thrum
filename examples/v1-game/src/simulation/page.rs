//! Game-owned V1 page columns in the engine's generic immutable lease ring.

use std::{mem::size_of, sync::Arc};

use thrum_core::{
    Handle, StableId, WorldEpoch,
    sim::{Read2, ReadPermit, SnapshotId, StructuralChange, WorldRead},
};

use super::types::{Health, Ownership, Position, Velocity, Visual};

const POSITION_DIRTY: u8 = 1;
const VELOCITY_DIRTY: u8 = 2;
const HEALTH_DIRTY: u8 = 4;
const STRUCTURAL_DIRTY: u8 = 8;

/// Changed physical slots in one complete V1 game tick.
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

    /// Slots with a position write this tick.
    pub fn position(&self) -> &[u32] {
        &self.position
    }
    /// Slots with a velocity write this tick.
    pub fn velocity(&self) -> &[u32] {
        &self.velocity
    }
    /// Slots with a health write this tick.
    pub fn health(&self) -> &[u32] {
        &self.health
    }
    /// Slots with a spawn or despawn this tick.
    pub fn structural(&self) -> &[u32] {
        &self.structural
    }
}

pub(crate) struct Page {
    id: SnapshotId,
    timeline: u64,
    ids: Vec<Option<StableId>>,
    generations: Vec<u64>,
    present_marks: Vec<u64>,
    dirty_flags: Vec<u8>,
    positions: Vec<Position>,
    velocities: Vec<Velocity>,
    health: Vec<u16>,
    ownership: Vec<Option<Ownership>>,
    visuals: Vec<Option<u64>>,
    dirty: DirtySlots,
    events: Vec<StructuralChange>,
    live: usize,
    initialized: bool,
    baseline: bool,
}

impl Page {
    pub(crate) fn new(epoch: WorldEpoch, capacity: usize) -> Self {
        Self {
            id: SnapshotId { epoch, tick: 0 },
            timeline: 0,
            ids: vec![None; capacity],
            generations: vec![0; capacity],
            present_marks: vec![u64::MAX; capacity],
            dirty_flags: vec![0; capacity],
            positions: vec![Position::default(); capacity],
            velocities: vec![Velocity::default(); capacity],
            health: vec![0; capacity],
            ownership: vec![None; capacity],
            visuals: vec![None; capacity],
            dirty: DirtySlots::with_capacity(capacity),
            events: Vec::with_capacity(capacity.saturating_mul(2)),
            live: 0,
            initialized: false,
            baseline: false,
        }
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "one page capture consumes the declared V1 query and sparse read permits"
    )]
    pub(crate) fn capture(
        &mut self,
        world: &WorldRead<'_>,
        query: &mut Read2<Position, Health>,
        position_read: &ReadPermit<Position>,
        velocity_read: &ReadPermit<Velocity>,
        health_read: &ReadPermit<Health>,
        ownership_read: &ReadPermit<Ownership>,
        visual_read: &ReadPermit<Visual>,
        events: &[StructuralChange],
        tick: u64,
        timeline: u64,
        first: bool,
    ) {
        let since = (self.initialized && !first).then_some(self.id.tick);
        self.id.tick = tick;
        self.timeline = timeline;
        self.baseline = first;
        self.dirty.clear();
        self.events.clear();
        self.live = 0;
        if !first {
            self.events.extend_from_slice(events);
            for event in events {
                let slot = match event {
                    StructuralChange::Despawn(handle) | StructuralChange::Spawn(handle) => {
                        handle.slot
                    }
                };
                self.dirty_flags[slot as usize] |= STRUCTURAL_DIRTY;
            }
        }
        query.visit_with_changes(
            world,
            (position_read, velocity_read, health_read),
            |handle, id, position, health, stamps| {
                let slot = handle.slot as usize;
                self.present_marks[slot] = tick;
                self.live += 1;
                let [structural_tick, position_tick, velocity_tick, health_tick] = stamps;
                let structural = since.is_none_or(|old| structural_tick > old);
                if structural {
                    self.ids[slot] = Some(id);
                    self.generations[slot] = handle.generation;
                    self.ownership[slot] = world.with(ownership_read, handle, |value| *value).ok();
                    self.visuals[slot] = world.with(visual_read, handle, |value| value.0).ok();
                }
                if structural || since.is_none_or(|old| position_tick > old) {
                    self.positions[slot] = *position;
                }
                if structural || since.is_none_or(|old| velocity_tick > old) {
                    self.velocities[slot] = world
                        .with(velocity_read, handle, |value| *value)
                        .unwrap_or_default();
                }
                if structural || since.is_none_or(|old| health_tick > old) {
                    self.health[slot] = health.0;
                }
                if first || structural_tick == tick || position_tick == tick {
                    self.dirty_flags[slot] |= POSITION_DIRTY;
                }
                if first || structural_tick == tick || velocity_tick == tick {
                    self.dirty_flags[slot] |= VELOCITY_DIRTY;
                }
                if first || structural_tick == tick || health_tick == tick {
                    self.dirty_flags[slot] |= HEALTH_DIRTY;
                }
                if first || structural_tick == tick {
                    self.dirty_flags[slot] |= STRUCTURAL_DIRTY;
                }
            },
        );
        for (slot, id) in self.ids.iter_mut().enumerate() {
            if id.is_some() && self.present_marks[slot] != tick {
                *id = None;
            }
            let flags = std::mem::take(&mut self.dirty_flags[slot]);
            if flags != 0 {
                let slot = u32::try_from(slot).expect("validated slot capacity");
                if flags & POSITION_DIRTY != 0 {
                    self.dirty.position.push(slot);
                }
                if flags & VELOCITY_DIRTY != 0 {
                    self.dirty.velocity.push(slot);
                }
                if flags & HEALTH_DIRTY != 0 {
                    self.dirty.health.push(slot);
                }
                if flags & STRUCTURAL_DIRTY != 0 {
                    self.dirty.structural.push(slot);
                }
            }
            if first && id.is_some() {
                self.events.push(StructuralChange::Spawn(Handle {
                    epoch: self.id.epoch,
                    slot: u32::try_from(slot).expect("validated slot capacity"),
                    generation: self.generations[slot],
                }));
            }
        }
        self.initialized = true;
    }

    pub(crate) fn reserved_bytes(&self) -> usize {
        fn bytes<T>(values: &Vec<T>) -> usize {
            values.capacity() * size_of::<T>()
        }
        size_of::<Self>()
            + bytes(&self.ids)
            + bytes(&self.generations)
            + bytes(&self.present_marks)
            + bytes(&self.dirty_flags)
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
}

/// Shared immutable lease of one complete V1 game endpoint.
#[derive(Clone)]
pub struct PublishedPage(pub(crate) Arc<Page>);

impl PublishedPage {
    /// Whether this page starts a new complete membership stream, including
    /// a restored replay endpoint at a nonzero tick.
    pub fn is_baseline(&self) -> bool {
        self.0.baseline
    }
    /// Committed world generation and fixed tick.
    pub fn id(&self) -> SnapshotId {
        self.0.id
    }
    /// Replay branch generation for temporal presentation history.
    pub fn timeline(&self) -> u64 {
        self.0.timeline
    }
    /// Live entity count.
    pub fn len(&self) -> usize {
        self.0.live
    }
    /// Whether this endpoint has no live entities.
    pub fn is_empty(&self) -> bool {
        self.0.live == 0
    }
    /// Physical slot capacity of this authority.
    pub fn slot_capacity(&self) -> usize {
        self.0.ids.len()
    }
    /// Stable identity at one live physical slot.
    pub fn stable_id(&self, slot: u32) -> Option<StableId> {
        self.0.ids.get(slot as usize).copied().flatten()
    }
    /// Checked handle at one live physical slot.
    pub fn handle(&self, slot: u32) -> Option<Handle> {
        self.stable_id(slot).map(|_| Handle {
            epoch: self.0.id.epoch,
            slot,
            generation: self.0.generations[slot as usize],
        })
    }
    /// Position at one matching live handle.
    pub fn position(&self, handle: Handle) -> Option<Position> {
        self.matches(handle)
            .then(|| self.0.positions[handle.slot as usize])
    }
    /// Velocity at one matching live handle; zero when its component is absent.
    pub fn velocity(&self, handle: Handle) -> Option<Velocity> {
        self.matches(handle)
            .then(|| self.0.velocities[handle.slot as usize])
    }
    /// Health at one matching live handle.
    pub fn health(&self, handle: Handle) -> Option<u16> {
        self.matches(handle)
            .then(|| self.0.health[handle.slot as usize])
    }
    /// Ownership at one matching live handle.
    pub fn ownership(&self, handle: Handle) -> Option<Ownership> {
        self.matches(handle)
            .then(|| self.0.ownership[handle.slot as usize])
            .flatten()
    }
    /// Visual selector at one matching live handle.
    pub fn visual(&self, handle: Handle) -> Option<u64> {
        self.matches(handle)
            .then(|| self.0.visuals[handle.slot as usize])
            .flatten()
    }
    /// This tick's component changes.
    pub fn dirty(&self) -> &DirtySlots {
        &self.0.dirty
    }
    /// This tick's ordered structural journal.
    pub fn structural_events(&self) -> &[StructuralChange] {
        &self.0.events
    }
    /// Visit live visual rows without allocation or stable-ID lookup.
    ///
    /// # Panics
    ///
    /// Panics only if an internally validated physical slot exceeds u32.
    pub fn visit(&self, mut visit: impl FnMut(Handle, StableId, Position, Option<u64>)) {
        for (slot, id) in self.0.ids.iter().enumerate() {
            if let Some(id) = id {
                visit(
                    Handle {
                        epoch: self.0.id.epoch,
                        slot: u32::try_from(slot).expect("validated slot capacity"),
                        generation: self.0.generations[slot],
                    },
                    *id,
                    self.0.positions[slot],
                    self.0.visuals[slot],
                );
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
