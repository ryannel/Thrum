//! Game-registered component storage with engine-owned identity. This is the
//! G2 replacement seam for the fixed A1 store; only checked handles escape.

use std::{
    collections::hash_map::RandomState, hash::BuildHasher, marker::PhantomData, mem::size_of,
};

use crate::{Handle, StableId, WorldEpoch};

use super::{
    ComponentKey, ComponentRegistry, ComponentSchemaId,
    column_store::{EntityBundle, StorageColumn, TypedStore},
    schedule::SnapshotColumn,
};

#[derive(Clone, Copy)]
struct CapturedSlot {
    generation: u64,
    stable: Option<StableId>,
    structural_tick: u64,
}

pub(super) struct WorldCapture {
    epoch: WorldEpoch,
    schema: Vec<ComponentSchemaId>,
    types: Vec<ComponentKey>,
    slots: Vec<CapturedSlot>,
    columns: Vec<Box<dyn SnapshotColumn>>,
    free: Vec<u32>,
    change_ticks: Vec<u64>,
}

pub(super) struct WorldDelta {
    slots: Vec<(u32, CapturedSlot)>,
    events: Vec<super::StructuralChange>,
    changed: Vec<Vec<(u32, u64)>>,
    columns: Vec<Box<dyn SnapshotColumn>>,
}

impl WorldDelta {
    pub(super) fn reserved_bytes(&self) -> usize {
        size_of::<Self>()
            + self.slots.capacity() * size_of::<(u32, CapturedSlot)>()
            + self.events.capacity() * size_of::<super::StructuralChange>()
            + self.changed.capacity() * size_of::<Vec<(u32, u64)>>()
            + self
                .changed
                .iter()
                .map(|column| column.capacity() * size_of::<(u32, u64)>())
                .sum::<usize>()
            + self.columns.capacity() * size_of::<Box<dyn SnapshotColumn>>()
            + self
                .columns
                .iter()
                .map(|column| column.reserved_bytes())
                .sum::<usize>()
    }
}

impl WorldCapture {
    pub(super) fn reserved_bytes(&self) -> usize {
        size_of::<Self>()
            + self.schema.capacity() * size_of::<ComponentSchemaId>()
            + self.types.capacity() * size_of::<ComponentKey>()
            + self.slots.capacity() * size_of::<CapturedSlot>()
            + self.free.capacity() * size_of::<u32>()
            + self.change_ticks.capacity() * size_of::<u64>()
            + self.columns.capacity() * size_of::<Box<dyn SnapshotColumn>>()
            + self
                .columns
                .iter()
                .map(|column| column.reserved_bytes())
                .sum::<usize>()
    }

    pub(super) fn clone_independent(&self) -> Self {
        Self {
            epoch: self.epoch,
            schema: self.schema.clone(),
            types: self.types.clone(),
            slots: self.slots.clone(),
            columns: self
                .columns
                .iter()
                .map(|column| column.clone_box())
                .collect(),
            free: self.free.clone(),
            change_ticks: self.change_ticks.clone(),
        }
    }

    pub(super) fn advance(&mut self, delta: &WorldDelta) {
        for event in &delta.events {
            match event {
                super::StructuralChange::Despawn(handle) => {
                    if self.slots[handle.slot as usize].generation != u64::MAX {
                        self.free.push(handle.slot);
                    }
                }
                super::StructuralChange::Spawn(handle) => {
                    assert_eq!(self.free.pop(), Some(handle.slot), "recorded slot order");
                }
            }
        }
        for &(slot, state) in &delta.slots {
            self.slots[slot as usize] = state;
        }
        let component_count = self.columns.len();
        for (index, (column, changed)) in self.columns.iter_mut().zip(&delta.changed).enumerate() {
            column.overlay(&*delta.columns[index], changed);
            for &(slot, tick) in changed {
                self.change_ticks[slot as usize * component_count + index] = tick;
            }
        }
    }
}

#[derive(Clone, Copy)]
struct Slot {
    generation: u64,
    stable: Option<StableId>,
    structural_tick: u64,
}

const CHANGE_FRAME_COUNT: usize = 4;

fn change_frame_index(tick: u64) -> usize {
    usize::try_from(tick % 4).expect("four frame indices fit usize")
}

struct ChangeFrame {
    tick: u64,
    slots: Vec<u32>,
}

struct IdIndex {
    buckets: Vec<Option<(StableId, u32)>>,
    mask: usize,
    hash: RandomState,
}

impl IdIndex {
    fn new(capacity: usize) -> Self {
        let count = capacity
            .checked_mul(2)
            .and_then(usize::checked_next_power_of_two)
            .expect("validated world capacity fits identity index")
            .max(1);
        Self {
            buckets: vec![None; count],
            mask: count - 1,
            hash: RandomState::new(),
        }
    }

    fn home(&self, id: StableId) -> usize {
        usize::try_from(self.hash.hash_one(id) & u64::try_from(self.mask).expect("mask fits u64"))
            .expect("masked hash fits usize")
    }

    fn get(&self, id: StableId) -> Option<u32> {
        let mut bucket = self.home(id);
        loop {
            match self.buckets[bucket] {
                Some((found, slot)) if found == id => return Some(slot),
                None => return None,
                _ => bucket = (bucket + 1) & self.mask,
            }
        }
    }

    fn insert(&mut self, id: StableId, slot: u32) {
        let mut bucket = self.home(id);
        while self.buckets[bucket].is_some() {
            bucket = (bucket + 1) & self.mask;
        }
        self.buckets[bucket] = Some((id, slot));
    }

    fn remove(&mut self, id: StableId) {
        let mut hole = self.home(id);
        while self.buckets[hole].is_some_and(|(key, _)| key != id) {
            hole = (hole + 1) & self.mask;
        }
        let mut next = (hole + 1) & self.mask;
        while let Some((key, slot)) = self.buckets[next] {
            let home = self.home(key);
            if ((hole.wrapping_sub(home)) & self.mask) < ((next.wrapping_sub(home)) & self.mask) {
                self.buckets[hole] = Some((key, slot));
                hole = next;
            }
            next = (next + 1) & self.mask;
        }
        self.buckets[hole] = None;
    }
}

/// Expected setup, identity or component failure. A rejected spawn changes no
/// world state; callers validate a whole command batch before applying it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegisteredWorldError {
    /// Capacity is outside the slot address range or no free slot remains.
    Capacity,
    /// Stable identity already belongs to a live entity.
    DuplicateId,
    /// A handle is stale, retired or from another world epoch.
    InvalidHandle,
    /// A game attempted to use a type absent from setup registration.
    UnregisteredComponent,
    /// A spawn attempted to add a component type twice.
    DuplicateComponent,
    /// The checked entity does not contain this registered component.
    MissingComponent,
    /// Capture and receiver have different generations, capacity or schema.
    SnapshotMismatch,
    /// A typed access token belongs to another compiled runtime.
    WrongRuntime,
    /// A prepared read belongs to another compiled system callback.
    WrongSystem,
}

/// Reusable typed bundle writer. It never exposes storage rows or a mutable world.
pub struct EntityWriter<'a> {
    builder: &'a mut EntityBundle,
    registry: &'a ComponentRegistry,
}

impl EntityWriter<'_> {
    /// Add one registered game component to the pending entity.
    ///
    /// # Errors
    ///
    /// The type is unregistered or already present in this entity bundle.
    pub fn add<T: Send + Sync + 'static>(
        &mut self,
        value: T,
    ) -> Result<&mut Self, RegisteredWorldError> {
        let Some(index) = self
            .registry
            .entries
            .iter()
            .position(|entry| entry.key == ComponentKey::of::<T>())
        else {
            return Err(RegisteredWorldError::UnregisteredComponent);
        };
        if !self.builder.add(index, value) {
            return Err(RegisteredWorldError::DuplicateComponent);
        }
        Ok(self)
    }
}

/// Generic component authority for one world epoch. Identity slots and typed
/// columns are private; the public handle is checked on every sparse access.
pub(super) struct RegisteredWorld {
    epoch: WorldEpoch,
    slots: Vec<Slot>,
    free: Vec<u32>,
    ids: IdIndex,
    columns: Vec<Box<dyn StorageColumn>>,
    builder: EntityBundle,
    registry: ComponentRegistry,
    change_ticks: Vec<u64>,
    changed: Vec<Vec<u32>>,
    change_frames: [ChangeFrame; CHANGE_FRAME_COUNT],
    pending_slots: Vec<u32>,
    pending_marks: Vec<bool>,
    live: usize,
}

impl RegisteredWorld {
    pub(super) fn empty_delta(&self) -> WorldDelta {
        WorldDelta {
            slots: Vec::new(),
            events: Vec::new(),
            changed: (0..self.registry.entries.len())
                .map(|_| Vec::new())
                .collect(),
            columns: self
                .registry
                .entries
                .iter()
                .map(|entry| (entry.empty)())
                .collect(),
        }
    }

    pub(super) fn capture_delta_into(
        &self,
        events: &[super::StructuralChange],
        delta: &mut WorldDelta,
    ) {
        delta.events.clear();
        delta.events.extend_from_slice(events);
        delta.slots.clear();
        for event in events {
            let slot = match event {
                super::StructuralChange::Despawn(handle)
                | super::StructuralChange::Spawn(handle) => handle.slot,
            };
            let state = self.slots[slot as usize];
            delta.slots.push((
                slot,
                CapturedSlot {
                    generation: state.generation,
                    stable: state.stable,
                    structural_tick: state.structural_tick,
                },
            ));
        }
        delta.slots.sort_unstable_by_key(|(slot, _)| *slot);
        delta.slots.dedup_by_key(|(slot, _)| *slot);
        for (index, (changed, column)) in
            delta.changed.iter_mut().zip(&mut delta.columns).enumerate()
        {
            changed.clear();
            changed.extend(self.changed[index].iter().map(|slot| {
                (
                    *slot,
                    self.change_ticks[*slot as usize * self.registry.entries.len() + index],
                )
            }));
            changed.sort_unstable_by_key(|(slot, _)| *slot);
            changed.dedup_by_key(|(slot, _)| *slot);
            column.clear();
            for &(slot, _) in changed.iter() {
                column.capture_slot(self, slot);
            }
        }
    }
    pub(super) fn capture(&self) -> WorldCapture {
        let slots = self
            .slots
            .iter()
            .map(|slot| CapturedSlot {
                generation: slot.generation,
                stable: slot.stable,
                structural_tick: slot.structural_tick,
            })
            .collect();
        WorldCapture {
            epoch: self.epoch,
            schema: self
                .registry
                .entries
                .iter()
                .map(|entry| entry.schema)
                .collect(),
            types: self
                .registry
                .entries
                .iter()
                .map(|entry| entry.key)
                .collect(),
            slots,
            columns: self
                .registry
                .entries
                .iter()
                .map(|entry| (entry.capture)(self))
                .collect(),
            free: self.free.clone(),
            change_ticks: self.change_ticks.clone(),
        }
    }

    pub(super) fn restore(&mut self, capture: &WorldCapture) -> Result<(), RegisteredWorldError> {
        if capture.epoch != self.epoch
            || capture.slots.len() != self.slots.len()
            || capture.schema.len() != self.registry.entries.len()
            || capture.types.len() != self.registry.entries.len()
            || capture.columns.len() != self.registry.entries.len()
            || capture.change_ticks.len() != self.change_ticks.len()
            || capture
                .schema
                .iter()
                .zip(&self.registry.entries)
                .any(|(schema, entry)| *schema != entry.schema)
            || capture
                .types
                .iter()
                .zip(&self.registry.entries)
                .any(|(key, entry)| *key != entry.key)
        {
            return Err(RegisteredWorldError::SnapshotMismatch);
        }
        let mut columns: Vec<_> = self
            .registry
            .entries
            .iter()
            .map(|entry| (entry.storage)(capture.slots.len()))
            .collect();
        for (snapshot, column) in capture.columns.iter().zip(&mut columns) {
            snapshot.restore_into(column.as_mut());
        }
        let mut slots = Vec::with_capacity(capture.slots.len());
        let mut ids = IdIndex::new(capture.slots.len());
        let mut live = 0;
        for (index, saved) in capture.slots.iter().enumerate() {
            if let Some(id) = saved.stable {
                let key = u32::try_from(index).expect("validated slot capacity");
                ids.insert(id, key);
                live += 1;
            }
            slots.push(Slot {
                generation: saved.generation,
                stable: saved.stable,
                structural_tick: saved.structural_tick,
            });
        }
        self.columns = columns;
        self.builder.clear();
        self.slots = slots;
        self.ids = ids;
        self.free.clone_from(&capture.free);
        self.change_ticks.clone_from(&capture.change_ticks);
        for frame in &mut self.change_frames {
            frame.tick = 0;
            frame.slots.clear();
        }
        for slot in self.pending_slots.drain(..) {
            self.pending_marks[slot as usize] = false;
        }
        self.live = live;
        Ok(())
    }

    pub(super) fn epoch(&self) -> WorldEpoch {
        self.epoch
    }

    /// Reserve physical slots and adopt the game's setup-time component registry.
    ///
    /// # Errors
    ///
    /// Capacity cannot be represented by the physical slot index.
    pub(super) fn new(
        epoch: WorldEpoch,
        capacity: usize,
        registry: ComponentRegistry,
    ) -> Result<Self, RegisteredWorldError> {
        if capacity > u32::MAX as usize
            || capacity
                .checked_mul(2)
                .and_then(usize::checked_next_power_of_two)
                .is_none()
        {
            return Err(RegisteredWorldError::Capacity);
        }
        let slot_count = u32::try_from(capacity).map_err(|_| RegisteredWorldError::Capacity)?;
        let component_count = registry.entries.len();
        let change_count = capacity
            .checked_mul(component_count)
            .ok_or(RegisteredWorldError::Capacity)?;
        let builder = EntityBundle::new(&registry);
        let columns = registry
            .entries
            .iter()
            .map(|entry| (entry.storage)(capacity))
            .collect();
        Ok(Self {
            epoch,
            slots: vec![
                Slot {
                    generation: 1,
                    stable: None,
                    structural_tick: 0,
                };
                capacity
            ],
            free: (0..slot_count).rev().collect(),
            ids: IdIndex::new(capacity),
            columns,
            builder,
            registry,
            change_ticks: vec![0; change_count],
            changed: (0..component_count).map(|_| Vec::new()).collect(),
            change_frames: std::array::from_fn(|_| ChangeFrame {
                tick: 0,
                slots: Vec::with_capacity(capacity),
            }),
            pending_slots: Vec::with_capacity(capacity),
            pending_marks: vec![false; capacity],
            live: 0,
        })
    }

    /// Number of live entities.
    pub(super) fn len(&self) -> usize {
        self.live
    }

    pub(super) fn free_count(&self) -> usize {
        self.free.len()
    }

    pub(super) fn component_index(&self, key: ComponentKey) -> Option<usize> {
        self.registry
            .entries
            .iter()
            .position(|entry| entry.key == key)
    }

    pub(super) fn new_bundle(&self) -> EntityBundle {
        EntityBundle::new(&self.registry)
    }

    pub(super) fn column<T: Send + Sync + 'static>(&self) -> &TypedStore<T> {
        let index = self
            .component_index(ComponentKey::of::<T>())
            .expect("registered component type");
        self.column_at(index)
    }

    pub(super) fn column_at<T: Send + Sync + 'static>(&self, index: usize) -> &TypedStore<T> {
        self.columns[index]
            .as_any()
            .downcast_ref::<TypedStore<T>>()
            .expect("registered storage column type")
    }

    fn column_mut_index<T: Send + Sync + 'static>(&mut self, index: usize) -> &mut TypedStore<T> {
        self.columns[index]
            .as_any_mut()
            .downcast_mut::<TypedStore<T>>()
            .expect("registered storage column type")
    }

    pub(super) fn change_ticks3_at(&self, slot: u32, components: [usize; 3]) -> [u64; 4] {
        let row = slot as usize * self.registry.entries.len();
        [
            self.slots[slot as usize].structural_tick,
            self.change_ticks[row + components[0]],
            self.change_ticks[row + components[1]],
            self.change_ticks[row + components[2]],
        ]
    }

    pub(super) fn mark_changed(&mut self, component: usize, handle: Handle, tick: u64) {
        self.change_ticks[handle.slot as usize * self.registry.entries.len() + component] = tick;
        self.changed[component].push(handle.slot);
        self.mark_pending(handle.slot);
    }

    pub(super) fn begin_tick(&mut self) {
        for slot in self.pending_slots.drain(..) {
            self.pending_marks[slot as usize] = false;
        }
        for changed in &mut self.changed {
            changed.clear();
        }
    }

    pub(super) fn mark_structural(&mut self, slot: u32) {
        for changed in &mut self.changed {
            changed.push(slot);
        }
        self.mark_pending(slot);
    }

    fn mark_pending(&mut self, slot: u32) {
        if !self.pending_marks[slot as usize] {
            self.pending_marks[slot as usize] = true;
            self.pending_slots.push(slot);
        }
    }

    pub(super) fn finish_tick(&mut self, tick: u64) {
        let frame = &mut self.change_frames[change_frame_index(tick)];
        frame.tick = tick;
        frame.slots.clear();
        frame.slots.extend_from_slice(&self.pending_slots);
    }

    pub(super) fn has_changes_since(&self, since: u64, current: u64) -> bool {
        if since > current || current - since > CHANGE_FRAME_COUNT as u64 {
            return false;
        }
        if since == current {
            return true;
        }
        (since + 1..=current).all(|tick| self.change_frames[change_frame_index(tick)].tick == tick)
    }

    pub(super) fn visit_changes_since(
        &self,
        since: u64,
        current: u64,
        mut visit: impl FnMut(u32),
    ) -> bool {
        if !self.has_changes_since(since, current) {
            return false;
        }
        if since == current {
            return true;
        }
        for tick in since + 1..=current {
            for &slot in &self.change_frames[change_frame_index(tick)].slots {
                visit(slot);
            }
        }
        true
    }

    pub(super) fn live_at_slot(&self, slot: u32) -> Option<(Handle, StableId)> {
        let stable = self.slots.get(slot as usize)?.stable?;
        Some((self.handle_at(slot), stable))
    }

    pub(super) fn prepare_bundle(
        &self,
        builder: &mut EntityBundle,
        fill: impl FnOnce(&mut EntityWriter<'_>) -> Result<(), RegisteredWorldError>,
    ) -> Result<(), RegisteredWorldError> {
        builder.clear();
        let result = fill(&mut EntityWriter {
            builder,
            registry: &self.registry,
        });
        if result.is_err() {
            builder.clear();
        }
        result
    }

    pub(super) fn spawn_prepared(
        &mut self,
        id: StableId,
        builder: &mut EntityBundle,
        tick: u64,
    ) -> Result<Handle, RegisteredWorldError> {
        if self.ids.get(id).is_some() {
            return Err(RegisteredWorldError::DuplicateId);
        }
        let slot = *self.free.last().ok_or(RegisteredWorldError::Capacity)?;
        builder.commit(&mut self.columns, slot as usize);
        self.free.pop();
        self.slots[slot as usize].stable = Some(id);
        self.slots[slot as usize].structural_tick = tick;
        self.ids.insert(id, slot);
        self.live += 1;
        Ok(self.handle_at(slot))
    }

    pub(super) fn reusable_after_despawn(&self, handle: Handle) -> bool {
        self.slots[handle.slot as usize].generation != u64::MAX
    }

    /// Resolve a persistent identity to its current checked physical handle.
    pub(super) fn handle(&self, id: StableId) -> Option<Handle> {
        self.ids.get(id).map(|slot| self.handle_at(slot))
    }

    pub(super) fn handle_at(&self, slot: u32) -> Handle {
        Handle {
            epoch: self.epoch,
            slot,
            generation: self.slots[slot as usize].generation,
        }
    }

    pub(super) fn entity(&self, handle: Handle) -> Result<(), RegisteredWorldError> {
        if handle.epoch != self.epoch {
            return Err(RegisteredWorldError::InvalidHandle);
        }
        let slot = self
            .slots
            .get(handle.slot as usize)
            .ok_or(RegisteredWorldError::InvalidHandle)?;
        if slot.generation != handle.generation {
            return Err(RegisteredWorldError::InvalidHandle);
        }
        slot.stable
            .map(|_| ())
            .ok_or(RegisteredWorldError::InvalidHandle)
    }

    pub(super) fn stable_id(&self, handle: Handle) -> Result<StableId, RegisteredWorldError> {
        self.entity(handle)?;
        self.slots[handle.slot as usize]
            .stable
            .ok_or(RegisteredWorldError::InvalidHandle)
    }

    /// Spawn one registered game bundle under a new stable identity.
    ///
    /// # Errors
    ///
    /// The ID repeats, capacity is full, or the bundle contains an unregistered
    /// or repeated component. No failure changes committed state.
    pub(super) fn spawn(
        &mut self,
        id: StableId,
        fill: impl FnOnce(&mut EntityWriter<'_>) -> Result<(), RegisteredWorldError>,
    ) -> Result<Handle, RegisteredWorldError> {
        if self.ids.get(id).is_some() {
            return Err(RegisteredWorldError::DuplicateId);
        }
        let slot = *self.free.last().ok_or(RegisteredWorldError::Capacity)?;
        let result = fill(&mut EntityWriter {
            builder: &mut self.builder,
            registry: &self.registry,
        });
        if let Err(error) = result {
            self.builder.clear();
            return Err(error);
        }
        self.builder.commit(&mut self.columns, slot as usize);
        self.free.pop();
        self.slots[slot as usize].stable = Some(id);
        self.ids.insert(id, slot);
        self.live += 1;
        Ok(self.handle_at(slot))
    }

    /// Remove a checked entity and retire its generation.
    ///
    /// # Errors
    ///
    /// The handle is stale or from another world.
    pub(super) fn despawn(&mut self, handle: Handle) -> Result<(), RegisteredWorldError> {
        self.entity(handle)?;
        let id = self.slots[handle.slot as usize]
            .stable
            .ok_or(RegisteredWorldError::InvalidHandle)?;
        for column in &mut self.columns {
            column.clear_slot(handle.slot as usize);
        }
        let slot = &mut self.slots[handle.slot as usize];
        self.ids.remove(id);
        slot.stable = None;
        self.live -= 1;
        if let Some(next) = slot.generation.checked_add(1) {
            slot.generation = next;
            self.free.push(handle.slot);
        }
        Ok(())
    }

    /// Read one registered component through a checked handle without exposing
    /// a backend borrow or backend entity ID.
    ///
    /// # Errors
    ///
    /// The type is unregistered, the handle is stale, or the entity lacks it.
    pub(super) fn with<T: Send + Sync + 'static, R>(
        &self,
        handle: Handle,
        read: impl FnOnce(&T) -> R,
    ) -> Result<R, RegisteredWorldError> {
        let index = self
            .component_index(ComponentKey::of::<T>())
            .ok_or(RegisteredWorldError::UnregisteredComponent)?;
        self.with_index(index, handle, read)
    }

    pub(super) fn with_index<T: Send + Sync + 'static, R>(
        &self,
        index: usize,
        handle: Handle,
        read: impl FnOnce(&T) -> R,
    ) -> Result<R, RegisteredWorldError> {
        self.entity(handle)?;
        let value = self.columns[index]
            .as_any()
            .downcast_ref::<TypedStore<T>>()
            .expect("registered read permit type")
            .get(handle.slot as usize)
            .ok_or(RegisteredWorldError::MissingComponent)?;
        Ok(read(value))
    }

    pub(super) fn replace<T: Send + Sync + 'static>(
        &mut self,
        component: usize,
        handle: Handle,
        value: T,
    ) -> Result<T, RegisteredWorldError> {
        self.entity(handle)?;
        let current = self
            .column_mut_index::<T>(component)
            .get_mut(handle.slot as usize)
            .ok_or(RegisteredWorldError::MissingComponent)?;
        Ok(std::mem::replace(current, value))
    }

    pub(super) fn insert<T: Send + Sync + 'static>(
        &mut self,
        component: usize,
        handle: Handle,
        value: T,
    ) -> Result<(), RegisteredWorldError> {
        self.entity(handle)?;
        let column = self.column_mut_index::<T>(component);
        if column.get(handle.slot as usize).is_some() {
            return Err(RegisteredWorldError::DuplicateComponent);
        }
        column.insert(handle.slot as usize, value);
        Ok(())
    }

    pub(super) fn remove<T: Send + Sync + 'static>(
        &mut self,
        component: usize,
        handle: Handle,
    ) -> Result<T, RegisteredWorldError> {
        self.entity(handle)?;
        self.column_mut_index::<T>(component)
            .remove(handle.slot as usize)
            .ok_or(RegisteredWorldError::MissingComponent)
    }

    /// Prepare one two-component storage-order query for repeated ticks.
    ///
    /// # Errors
    ///
    /// Either type is absent from setup registration.
    pub(super) fn read2<T: Send + Sync + 'static, U: Send + Sync + 'static>(
        &self,
    ) -> Result<PreparedRead2<T, U>, RegisteredWorldError> {
        let first = self
            .component_index(ComponentKey::of::<T>())
            .ok_or(RegisteredWorldError::UnregisteredComponent)?;
        let second = self
            .component_index(ComponentKey::of::<U>())
            .ok_or(RegisteredWorldError::UnregisteredComponent)?;
        Ok(PreparedRead2 {
            first,
            second,
            marker: PhantomData,
        })
    }

    /// Visit matching components in canonical physical-slot order for
    /// order-sensitive reductions and events. Independent kernels should use
    /// the prepared storage-order cursor instead.
    pub(super) fn visit_canonical<T: Send + Sync + 'static, U: Send + Sync + 'static>(
        &self,
        mut visit: impl FnMut(Handle, StableId, &T, &U),
    ) {
        let first = self.column::<T>();
        let second = self.column::<U>();
        for (index, slot) in self.slots.iter().enumerate() {
            let Some(id) = slot.stable else { continue };
            let (Some(a), Some(b)) = (first.get(index), second.get(index)) else {
                continue;
            };
            let key = u32::try_from(index).expect("validated slot capacity");
            visit(self.handle_at(key), id, a, b);
        }
    }
}

/// Prepared read of two game-owned component types in canonical slot order.
/// The callback receives checked physical and stable identity.
pub(super) struct PreparedRead2<T: Send + Sync + 'static, U: Send + Sync + 'static> {
    first: usize,
    second: usize,
    marker: PhantomData<(T, U)>,
}

impl<T: Send + Sync + 'static, U: Send + Sync + 'static> PreparedRead2<T, U> {
    /// Visit matching entities with no per-entity allocation or ID lookup.
    ///
    /// # Panics
    ///
    /// Panics only if the private slot-to-entity invariant has been broken.
    pub(super) fn visit(
        &mut self,
        world: &RegisteredWorld,
        mut visit: impl FnMut(Handle, StableId, &T, &U),
    ) {
        self.visit_rows(world, |handle, id, a, b, _| visit(handle, id, a, b));
    }

    pub(super) fn visit_rows(
        &mut self,
        world: &RegisteredWorld,
        mut visit: impl FnMut(Handle, StableId, &T, &U, u32),
    ) {
        let first = world.columns[self.first]
            .as_any()
            .downcast_ref::<TypedStore<T>>()
            .expect("prepared first component type");
        let second = world.columns[self.second]
            .as_any()
            .downcast_ref::<TypedStore<U>>()
            .expect("prepared second component type");
        for (word, (&a, &b)) in first.presence().iter().zip(second.presence()).enumerate() {
            let mut matching = a & b;
            while matching != 0 {
                let bit = matching.trailing_zeros() as usize;
                matching &= matching - 1;
                let index = word * 64 + bit;
                let key = u32::try_from(index).expect("validated slot capacity");
                let id = world.slots[index]
                    .stable
                    .expect("stored component belongs to a live slot");
                visit(
                    world.handle_at(key),
                    id,
                    first.get(index).expect("presence bit has value"),
                    second.get(index).expect("presence bit has value"),
                    key,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{StableId, WorldEpoch};

    use super::{ComponentRegistry, IdIndex, RegisteredWorld, RegisteredWorldError};
    use crate::sim::ComponentSchemaId;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Height(f64);
    #[derive(Clone, Copy)]
    struct Drift(f64);
    struct Unregistered;

    fn world() -> RegisteredWorld {
        let mut registry = ComponentRegistry::default();
        registry
            .register::<Height>(ComponentSchemaId(1))
            .expect("position registers");
        registry
            .register::<Drift>(ComponentSchemaId(2))
            .expect("velocity registers");
        RegisteredWorld::new(WorldEpoch(7), 2, registry).expect("capacity fits")
    }

    #[test]
    fn checked_identity_and_registered_components_survive_churn() {
        let mut world = world();
        assert_eq!(
            world.spawn(StableId(1), |entity| {
                entity.add(Unregistered)?;
                Ok(())
            }),
            Err(RegisteredWorldError::UnregisteredComponent)
        );
        assert_eq!(world.len(), 0);
        let first = world
            .spawn(StableId(1), |entity| {
                entity.add(Height(2.0))?;
                entity.add(Drift(3.0))?;
                Ok(())
            })
            .expect("registered bundle spawns");
        assert_eq!(world.handle(StableId(1)), Some(first));
        assert_eq!(
            world.with::<Height, _>(first, |value| *value),
            Ok(Height(2.0))
        );
        assert_eq!(
            world.spawn(StableId(1), |_| Ok(())),
            Err(RegisteredWorldError::DuplicateId)
        );
        let mut query = world.read2::<Height, Drift>().expect("query registered");
        let mut visited = Vec::new();
        query.visit(&world, |handle, id, position, velocity| {
            visited.push((handle, id, position.0, velocity.0));
        });
        assert_eq!(visited, [(first, StableId(1), 2.0, 3.0)]);
        let height = world
            .component_index(super::ComponentKey::of::<Height>())
            .expect("registered height");
        world
            .replace(height, first, Height(6.0))
            .expect("registered value");
        assert_eq!(world.with::<Height, _>(first, |value| value.0), Ok(6.0));
        world.despawn(first).expect("first entity live");
        assert_eq!(
            world.with::<Height, _>(first, |value| *value),
            Err(RegisteredWorldError::InvalidHandle)
        );
        let replacement = world
            .spawn(StableId(2), |entity| {
                entity.add(Height(5.0))?;
                entity.add(Drift(7.0))?;
                Ok(())
            })
            .expect("retired slot reused");
        assert_eq!(replacement.slot, first.slot);
        assert_ne!(replacement.generation, first.generation);
        visited.clear();
        query.visit(&world, |handle, id, position, velocity| {
            visited.push((handle, id, position.0, velocity.0));
        });
        assert_eq!(visited, [(replacement, StableId(2), 5.0, 7.0)]);
    }

    #[test]
    fn duplicate_component_refuses_without_consume_or_partial_spawn() {
        let mut world = world();
        assert_eq!(
            world.spawn(StableId(1), |entity| {
                entity.add(Height(1.0))?;
                entity.add(Height(2.0))?;
                Ok(())
            }),
            Err(RegisteredWorldError::DuplicateComponent)
        );
        assert_eq!(world.len(), 0);
        let handle = world
            .spawn(StableId(1), |entity| {
                entity.add(Height(3.0))?;
                Ok(())
            })
            .expect("failed builder was cleared");
        assert_eq!(handle.slot, 0);
        assert_eq!(
            world.with::<Height, _>(handle, |value| *value),
            Ok(Height(3.0))
        );
    }

    #[test]
    fn backward_shift_keeps_colliding_ids_reachable_after_removal() {
        let mut index = IdIndex::new(8);
        let bucket = index.mask;
        let ids: Vec<_> = (0..10_000u128)
            .map(StableId)
            .filter(|&id| index.home(id) == bucket)
            .take(5)
            .collect();
        assert_eq!(ids.len(), 5);
        for (slot, &id) in ids[..4].iter().enumerate() {
            index.insert(id, u32::try_from(slot).expect("small slot"));
        }
        index.remove(ids[0]);
        assert_eq!(index.get(ids[0]), None);
        for (slot, &id) in ids[1..4].iter().enumerate() {
            assert_eq!(
                index.get(id),
                Some(u32::try_from(slot + 1).expect("small slot"))
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
