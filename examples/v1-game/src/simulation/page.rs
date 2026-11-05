//! Game-owned V1 page columns in the engine's generic immutable lease ring.

use std::{mem::size_of, sync::Arc};

use thrum_core::{
    Handle, StableId, WorldEpoch,
    sim::{Read2, ReadPermit, RuntimeUndo, SnapshotId, StructuralChange, WorldRead},
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
    refresh_marks: Vec<u64>,
    refresh_generation: u64,
    dirty_flags: Vec<u8>,
    dirty_words: Vec<u64>,
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
    patch_source: Option<(SnapshotId, u64)>,
}

impl Page {
    fn mark_dirty(&mut self, slot: usize, flags: u8) {
        self.dirty_flags[slot] |= flags;
        self.dirty_words[slot / 64] |= 1_u64 << (slot % 64);
    }

    fn refresh_position(&mut self, slot: usize, position: Position, dirty: bool) {
        self.positions[slot] = position;
        if dirty {
            self.mark_dirty(slot, POSITION_DIRTY);
        }
    }

    pub(crate) fn new(epoch: WorldEpoch, capacity: usize) -> Self {
        Self {
            id: SnapshotId { epoch, tick: 0 },
            timeline: 0,
            ids: vec![None; capacity],
            generations: vec![0; capacity],
            present_marks: vec![u64::MAX; capacity],
            refresh_marks: vec![0; capacity],
            refresh_generation: 0,
            dirty_flags: vec![0; capacity],
            dirty_words: vec![0; capacity.div_ceil(64)],
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
            patch_source: None,
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
        let since =
            (self.initialized && !first && self.timeline == timeline).then_some(self.id.tick);
        let full_refresh = since.is_none_or(|old| !world.has_changes_since(old, tick));
        self.id.tick = tick;
        self.timeline = timeline;
        self.baseline = first;
        self.patch_source = None;
        self.dirty.clear();
        self.events.clear();
        if full_refresh {
            self.live = 0;
            self.present_marks.fill(u64::MAX);
        }
        if !first {
            self.events.extend_from_slice(events);
            for event in events {
                let slot = match event {
                    StructuralChange::Despawn(handle) | StructuralChange::Spawn(handle) => {
                        handle.slot
                    }
                };
                self.mark_dirty(slot as usize, STRUCTURAL_DIRTY);
            }
        }
        if full_refresh {
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
                        self.ownership[slot] =
                            world.with(ownership_read, handle, |value| *value).ok();
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
                        self.mark_dirty(slot, POSITION_DIRTY);
                    }
                    if first || structural_tick == tick || velocity_tick == tick {
                        self.mark_dirty(slot, VELOCITY_DIRTY);
                    }
                    if first || structural_tick == tick || health_tick == tick {
                        self.mark_dirty(slot, HEALTH_DIRTY);
                    }
                    if first || structural_tick == tick {
                        self.mark_dirty(slot, STRUCTURAL_DIRTY);
                    }
                },
            );
        } else if let Some(old) = since {
            let positions = world.column(position_read).expect("declared position read");
            let healths = world.column(health_read).expect("declared health read");
            let velocities = world.column(velocity_read).expect("declared velocity read");
            let owners = world
                .column(ownership_read)
                .expect("declared ownership read");
            let visuals = world.column(visual_read).expect("declared visual read");
            self.refresh_generation = self.refresh_generation.wrapping_add(1);
            if self.refresh_generation == 0 {
                self.refresh_marks.fill(0);
                self.refresh_generation = 1;
            }
            let retained = world.visit_changes_since(old, tick, |key| {
                let slot = key as usize;
                if self.refresh_marks[slot] == self.refresh_generation {
                    return;
                }
                self.refresh_marks[slot] = self.refresh_generation;
                let old_present = self.ids[slot].is_some();
                let Some(live) = world.live_at_slot(key) else {
                    if old_present {
                        self.ids[slot] = None;
                        self.live -= 1;
                    }
                    return;
                };
                let [structural_tick, position_tick, velocity_tick, health_tick] = world
                    .change_ticks3(live, (position_read, velocity_read, health_read))
                    .expect("same runtime live slot");
                if old_present
                    && structural_tick <= old
                    && velocity_tick <= old
                    && health_tick <= old
                {
                    if position_tick <= old {
                        return;
                    }
                    if let Some(position) = positions.get(live).expect("same runtime live slot") {
                        self.refresh_position(slot, *position, position_tick == tick);
                        return;
                    }
                }
                let (Some(position), Some(health)) = (
                    positions.get(live).expect("same runtime live slot"),
                    healths.get(live).expect("same runtime live slot"),
                ) else {
                    if old_present {
                        self.ids[slot] = None;
                        self.live -= 1;
                    }
                    return;
                };
                if !old_present {
                    self.live += 1;
                }
                let structural = structural_tick > old
                    || self.ids[slot] != Some(live.stable_id())
                    || self.generations[slot] != live.handle().generation;
                if structural {
                    self.ids[slot] = Some(live.stable_id());
                    self.generations[slot] = live.handle().generation;
                    self.ownership[slot] =
                        owners.get(live).expect("same runtime live slot").copied();
                    self.visuals[slot] = visuals
                        .get(live)
                        .expect("same runtime live slot")
                        .map(|value| value.0);
                }
                if structural || position_tick > old {
                    self.positions[slot] = *position;
                }
                if structural || velocity_tick > old {
                    self.velocities[slot] = velocities
                        .get(live)
                        .expect("same runtime live slot")
                        .copied()
                        .unwrap_or_default();
                }
                if structural || health_tick > old {
                    self.health[slot] = health.0;
                }
                if structural_tick == tick || position_tick == tick {
                    self.mark_dirty(slot, POSITION_DIRTY);
                }
                if structural_tick == tick || velocity_tick == tick {
                    self.mark_dirty(slot, VELOCITY_DIRTY);
                }
                if structural_tick == tick || health_tick == tick {
                    self.mark_dirty(slot, HEALTH_DIRTY);
                }
                if structural_tick == tick {
                    self.mark_dirty(slot, STRUCTURAL_DIRTY);
                }
            });
            debug_assert!(retained, "availability was checked before page refresh");
        }
        if full_refresh {
            for (slot, id) in self.ids.iter_mut().enumerate() {
                if id.is_some() && self.present_marks[slot] != tick {
                    *id = None;
                }
                if first && id.is_some() {
                    self.events.push(StructuralChange::Spawn(Handle {
                        epoch: self.id.epoch,
                        slot: u32::try_from(slot).expect("validated slot capacity"),
                        generation: self.generations[slot],
                    }));
                }
            }
        }
        for (word, value) in self.dirty_words.iter_mut().enumerate() {
            let mut marked = std::mem::take(value);
            while marked != 0 {
                let bit = marked.trailing_zeros() as usize;
                marked &= marked - 1;
                let slot = word * 64 + bit;
                let flags = std::mem::take(&mut self.dirty_flags[slot]);
                let key = u32::try_from(slot).expect("validated slot capacity");
                if flags & POSITION_DIRTY != 0 {
                    self.dirty.position.push(key);
                }
                if flags & VELOCITY_DIRTY != 0 {
                    self.dirty.velocity.push(key);
                }
                if flags & HEALTH_DIRTY != 0 {
                    self.dirty.health.push(key);
                }
                if flags & STRUCTURAL_DIRTY != 0 {
                    self.dirty.structural.push(key);
                }
            }
        }
        self.initialized = true;
    }

    pub(crate) fn source_tick(&self, old_timeline: u64, target: u64, latest: u64) -> Option<u64> {
        (self.initialized
            && self.timeline == old_timeline
            && (target..=latest).contains(&self.id.tick))
        .then_some(self.id.tick)
    }

    /// Turn an unleased old-branch endpoint into a target-branch change page.
    /// Membership and component differences are compared only at slots named
    /// by retained undo records, then emitted in physical slot order.
    #[expect(
        clippy::too_many_arguments,
        reason = "the V1 patch consumes its declared component permits and branch endpoints"
    )]
    pub(crate) fn patch_rewind(
        &mut self,
        world: &WorldRead<'_>,
        position_read: &ReadPermit<Position>,
        velocity_read: &ReadPermit<Velocity>,
        health_read: &ReadPermit<Health>,
        ownership_read: &ReadPermit<Ownership>,
        visual_read: &ReadPermit<Visual>,
        steps: &[RuntimeUndo],
        source: u64,
        target: u64,
        timeline: u64,
    ) {
        let positions = world.column(position_read).expect("declared position read");
        let velocities = world.column(velocity_read).expect("declared velocity read");
        let healths = world.column(health_read).expect("declared health read");
        let owners = world
            .column(ownership_read)
            .expect("declared ownership read");
        let visuals = world.column(visual_read).expect("declared visual read");
        let old_id = self.id;
        let old_timeline = self.timeline;
        self.dirty.clear();
        self.events.clear();
        for step in steps.iter().take_while(|step| step.tick() <= source) {
            step.visit_changed_slots(|slot| self.mark_dirty(slot as usize, POSITION_DIRTY));
        }
        for (word, value) in self.dirty_words.iter_mut().enumerate() {
            let mut marked = std::mem::take(value);
            while marked != 0 {
                let bit = marked.trailing_zeros() as usize;
                marked &= marked - 1;
                let slot = word * 64 + bit;
                self.dirty_flags[slot] = 0;
                let key = u32::try_from(slot).expect("validated slot capacity");
                let previous = self.ids[slot].map(|id| (id, self.generations[slot]));
                let live = world.live_at_slot(key);
                let current = live.map(|row| (row.stable_id(), row.handle().generation));
                if previous != current {
                    if previous.is_some() {
                        self.events.push(StructuralChange::Despawn(Handle {
                            epoch: old_id.epoch,
                            slot: key,
                            generation: self.generations[slot],
                        }));
                        self.live -= 1;
                    }
                    if let Some(row) = live {
                        self.events.push(StructuralChange::Spawn(row.handle()));
                        self.live += 1;
                    }
                    self.dirty.structural.push(key);
                }
                let Some(row) = live else {
                    self.ids[slot] = None;
                    continue;
                };
                let position = *positions
                    .get(row)
                    .expect("same runtime slot")
                    .expect("V1 position");
                let velocity = velocities
                    .get(row)
                    .expect("same runtime slot")
                    .copied()
                    .unwrap_or_default();
                let health = healths
                    .get(row)
                    .expect("same runtime slot")
                    .expect("V1 health")
                    .0;
                if previous != current || self.positions[slot] != position {
                    self.positions[slot] = position;
                    self.dirty.position.push(key);
                }
                if previous != current || self.velocities[slot] != velocity {
                    self.velocities[slot] = velocity;
                    self.dirty.velocity.push(key);
                }
                if previous != current || self.health[slot] != health {
                    self.health[slot] = health;
                    self.dirty.health.push(key);
                }
                if previous != current {
                    self.ids[slot] = Some(row.stable_id());
                    self.generations[slot] = row.handle().generation;
                    self.ownership[slot] = owners.get(row).expect("same runtime slot").copied();
                    self.visuals[slot] = visuals
                        .get(row)
                        .expect("same runtime slot")
                        .map(|value| value.0);
                }
            }
        }
        self.id.tick = target;
        self.timeline = timeline;
        self.baseline = false;
        self.patch_source = Some((old_id, old_timeline));
    }

    pub(crate) fn reserved_bytes(&self) -> usize {
        fn bytes<T>(values: &Vec<T>) -> usize {
            values.capacity() * size_of::<T>()
        }
        size_of::<Self>()
            + bytes(&self.ids)
            + bytes(&self.generations)
            + bytes(&self.present_marks)
            + bytes(&self.refresh_marks)
            + bytes(&self.dirty_flags)
            + bytes(&self.dirty_words)
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
    /// Old branch endpoint this inverse patch can update. Consumers with a
    /// different source must request a complete baseline.
    pub fn patch_source(&self) -> Option<(SnapshotId, u64)> {
        self.0.patch_source
    }
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
