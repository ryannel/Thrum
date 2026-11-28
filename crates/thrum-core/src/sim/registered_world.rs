//! Game-registered component storage with engine-owned identity. This is the
//! G2 replacement seam for the fixed A1 store; only checked handles escape.

#[cfg(test)]
mod digest_tests;
mod image;

use std::{
    any::Any,
    collections::hash_map::RandomState,
    hash::{BuildHasher, Hash, Hasher},
    marker::PhantomData,
    mem::size_of,
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
};

use crate::{Handle, StableId, WorldEpoch};

use super::{
    ComponentKey, ComponentRegistry, ComponentSchemaId,
    column_store::{EntityBundle, StorageColumn, TypedStore},
    dense_directory::{DenseDirectory, RowSwap},
    schedule::SnapshotColumn,
    undo::{TypedUndo, UndoColumn},
};

fn tick_index(component: usize, slot: usize, capacity: usize) -> usize {
    component * capacity + slot
}

/// Run word-aligned kernel chunks in storage order on the caller.
fn run_serial_chunks<T, O>(
    values: &mut [Option<T>],
    mut active: Option<&mut [u64]>,
    outputs: &mut [O],
    rows: usize,
    run: &impl Fn(usize, &mut [Option<T>], &mut [u64], &mut O),
) {
    for (chunk, (values, output)) in values.chunks_mut(rows).zip(outputs).enumerate() {
        let words = match active.as_deref_mut() {
            Some(active) => {
                let start = chunk * KERNEL_CHUNK_WORDS;
                let end = (start + KERNEL_CHUNK_WORDS).min(active.len());
                &mut active[start..end]
            }
            None => &mut [],
        };
        run(chunk, values, words, output);
    }
}

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
    id_index_slots: Vec<Option<u32>>,
    directory: Option<DenseDirectory>,
}

pub(super) struct WorldUndo {
    pub(super) rule_boundaries: bool,
    slots: Vec<(u32, Slot)>,
    structural_ticks: Vec<u64>,
    columns: Vec<Box<dyn UndoColumn>>,
    layout_swaps: Vec<RowSwap>,
}

pub(super) trait TentativeDirty {
    fn is_marked(&self, component: usize, slot: u32) -> bool;
    fn mark_first(&mut self, component: usize, slot: u32) -> bool;
}

pub(super) struct BatchUndoWriter<'a, T> {
    column: &'a mut TypedUndo<T>,
}

impl<T> BatchUndoWriter<'_, T> {
    pub(super) fn push(&mut self, slot: u32, prior: T, tick: u64) {
        self.column.push_component(slot, Some(prior), tick);
    }
}

impl WorldUndo {
    pub(super) fn visit_changed_components(&self, mut visit: impl FnMut(usize, u32)) {
        for (component, column) in self.columns.iter().enumerate() {
            column.visit_changed_slots(&mut |slot| visit(component, slot));
        }
    }

    pub(super) fn visit_first_preimages<T: Send + Sync + 'static>(
        &self,
        component: usize,
        seen_words: &mut [u64],
        visit: impl FnMut(u32, Option<&T>),
    ) {
        self.columns[component]
            .as_any()
            .downcast_ref::<TypedUndo<T>>()
            .expect("registered undo column type")
            .visit_first_preimages(seen_words, visit);
    }

    pub(super) fn batch_writer<T: Send + Sync + 'static>(
        &mut self,
        component: usize,
    ) -> BatchUndoWriter<'_, T> {
        BatchUndoWriter {
            column: self.columns[component]
                .as_any_mut()
                .downcast_mut::<TypedUndo<T>>()
                .expect("registered undo column type"),
        }
    }

    pub(super) fn changes_only(&self, component: usize) -> bool {
        self.slots.is_empty()
            && self
                .columns
                .iter()
                .enumerate()
                .all(|(index, column)| index == component || !column.has_rows())
    }

    pub(super) fn can_coalesce_without_drop_order(&self, component: usize) -> bool {
        self.columns[component].can_coalesce_without_drop_order()
    }

    pub(super) fn has_component_rows(&self, component: usize) -> bool {
        self.columns[component].has_rows()
    }

    pub(super) fn has_layout_swaps(&self) -> bool {
        !self.layout_swaps.is_empty()
    }
    pub(super) fn reserve_for(&mut self, counts: &[usize], structural: usize, despawns: usize) {
        self.slots.reserve(structural);
        self.structural_ticks
            .reserve(structural * self.columns.len());
        for (column, &count) in self.columns.iter_mut().zip(counts) {
            column.reserve(count + despawns);
        }
        self.layout_swaps
            .reserve(structural + counts.iter().sum::<usize>());
    }

    pub(super) fn visit_changed_slots(&self, mut visit: impl FnMut(u32)) {
        for column in &self.columns {
            column.visit_changed_slots(&mut visit);
        }
        for &(slot, _) in &self.slots {
            visit(slot);
        }
    }

    pub(super) fn clear(&mut self) {
        self.slots.clear();
        self.structural_ticks.clear();
        self.layout_swaps.clear();
        for column in &mut self.columns {
            column.clear();
        }
    }

    /// Roll back component writes made before commit metadata is published.
    /// Committed change marks are untouched because tentative writes did not
    /// call `begin_tick` or mark a committed change.
    pub(super) fn abort_tentative(&mut self, world: &mut RegisteredWorld) {
        assert!(self.slots.is_empty(), "structural edits are not tentative");
        assert!(
            self.layout_swaps.is_empty(),
            "layout edits are not tentative"
        );
        let capacity = world.slots.len();
        for (component, (column, storage)) in
            self.columns.iter_mut().zip(&mut world.columns).enumerate()
        {
            column.restore_into(
                storage.as_mut(),
                &mut world.change_ticks,
                component,
                capacity,
                world.directory.as_ref(),
            );
        }
        self.clear();
    }

    pub(super) fn reserved_bytes(&self) -> usize {
        size_of::<Self>()
            + self.slots.capacity() * size_of::<(u32, Slot)>()
            + self.structural_ticks.capacity() * size_of::<u64>()
            + self.layout_swaps.capacity() * size_of::<RowSwap>()
            + self
                .columns
                .iter()
                .map(|column| column.reserved_bytes())
                .sum::<usize>()
    }

    pub(super) fn record_component<T: Send + Sync + 'static>(
        &mut self,
        component: usize,
        slot: u32,
        tick: u64,
        prior: Option<T>,
    ) {
        self.columns[component]
            .as_any_mut()
            .downcast_mut::<TypedUndo<T>>()
            .expect("registered undo column type")
            .push_component(slot, prior, tick);
    }

    pub(super) fn record_structural(&mut self, world: &RegisteredWorld, slot: u32) {
        self.slots.push((slot, world.slots[slot as usize]));
        for component in 0..self.columns.len() {
            self.structural_ticks
                .push(world.change_ticks[tick_index(component, slot as usize, world.capacity())]);
        }
    }

    pub(super) fn record_spawn(&mut self, world: &RegisteredWorld, slot: u32) {
        self.record_structural(world, slot);
        if self.rule_boundaries {
            // A subsequent rule may edit this life or retire it and reuse its
            // slot. Record absence in the same sequence as component writes.
            for (component, column) in self.columns.iter_mut().enumerate() {
                column.record_absent(slot, world.change_tick_at(component, slot));
            }
        }
    }

    pub(super) fn restore(
        &mut self,
        world: &mut RegisteredWorld,
        events: &[super::StructuralChange],
    ) {
        if let Some(directory) = &mut world.directory {
            for &swap in self.layout_swaps.iter().rev() {
                directory.undo(swap);
                for column in &mut world.columns {
                    column.swap_rows(swap.row, swap.boundary_row);
                }
            }
        }
        assert_eq!(
            events.len(),
            self.slots.len(),
            "one preimage per structural edit"
        );
        let count = self.columns.len();
        let capacity = world.slots.len();
        for (index, event) in events.iter().enumerate().rev() {
            let (recorded_slot, before) = self.slots[index];
            match event {
                super::StructuralChange::Spawn(handle) => {
                    assert_eq!(handle.slot, recorded_slot, "recorded spawn slot");
                    let slot = handle.slot as usize;
                    let id = world.slots[slot].stable.expect("committed spawned ID");
                    world.ids.remove(id);
                    if !self.rule_boundaries {
                        let row = world.row_of_slot(handle.slot);
                        for column in &mut world.columns {
                            column.clear_slot(row);
                        }
                    }
                    world.slots[slot].stable = None;
                    world.live -= 1;
                    world.free.push(handle.slot);
                }
                super::StructuralChange::Despawn(handle) => {
                    assert_eq!(handle.slot, recorded_slot, "recorded despawn slot");
                    if before.generation != u64::MAX {
                        assert_eq!(world.free.pop(), Some(handle.slot), "reverse free order");
                    }
                    world
                        .ids
                        .insert(before.stable.expect("prior live ID"), handle.slot);
                    world.live += 1;
                }
            }
            world.slots[recorded_slot as usize] = before;
            for component in 0..count {
                world.change_ticks[tick_index(component, recorded_slot as usize, capacity)] =
                    self.structural_ticks[index * count + component];
            }
        }
        for (component, (column, storage)) in
            self.columns.iter_mut().zip(&mut world.columns).enumerate()
        {
            column.restore_into(
                storage.as_mut(),
                &mut world.change_ticks,
                component,
                capacity,
                world.directory.as_ref(),
            );
        }
        world.begin_tick();
    }
}

impl WorldCapture {
    pub(super) fn hash_metadata(&self, hash: &mut impl Hasher) {
        self.epoch.hash(hash);
        self.schema.hash(hash);
        self.types.hash(hash);
        self.slots.len().hash(hash);
        for slot in &self.slots {
            slot.generation.hash(hash);
            slot.stable.hash(hash);
            slot.structural_tick.hash(hash);
        }
        self.free.hash(hash);
        self.change_ticks.hash(hash);
        self.columns.len().hash(hash);
        for column in &self.columns {
            column.hash_presence(hash);
        }
        self.id_index_slots.hash(hash);
        if let Some(directory) = &self.directory {
            directory.hash(hash);
        }
    }

    pub(super) fn reserved_bytes(&self) -> usize {
        size_of::<Self>()
            + self.schema.capacity() * size_of::<ComponentSchemaId>()
            + self.types.capacity() * size_of::<ComponentKey>()
            + self.slots.capacity() * size_of::<CapturedSlot>()
            + self.free.capacity() * size_of::<u32>()
            + self.change_ticks.capacity() * size_of::<u64>()
            + self.id_index_slots.capacity() * size_of::<Option<u32>>()
            + self
                .directory
                .as_ref()
                .map_or(0, DenseDirectory::reserved_bytes)
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
            id_index_slots: self.id_index_slots.clone(),
            directory: self.directory.clone(),
        }
    }

    pub(super) fn copy_from(&mut self, source: &Self) {
        assert_eq!(
            self.columns.len(),
            source.columns.len(),
            "same snapshot schema"
        );
        self.slots.clone_from(&source.slots);
        self.free.clone_from(&source.free);
        self.change_ticks.clone_from(&source.change_ticks);
        self.id_index_slots.clone_from(&source.id_index_slots);
        self.directory.clone_from(&source.directory);
        for (target, source) in self.columns.iter_mut().zip(&source.columns) {
            target.copy_from(source.as_ref());
        }
    }
}

#[derive(Clone, Copy)]
struct Slot {
    generation: u64,
    stable: Option<StableId>,
    structural_tick: u64,
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

    fn clear(&mut self) {
        self.buckets.fill(None);
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
    /// A staged profile authority resource would store a non-finite number.
    NonFiniteResource,
    /// A profile component would store a non-finite number.
    NonFinite {
        /// Durable component identity.
        schema: ComponentSchemaId,
        /// Logical slot of the invalid value.
        slot: u32,
    },
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

/// Human-readable refusal at the registered-world boundary.
impl std::fmt::Display for RegisteredWorldError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFiniteResource => {
                f.write_str("authority resources would store a non-finite profile number")
            }
            Self::NonFinite { schema, slot } => write!(
                f,
                "component {} at slot {slot} would store a non-finite profile number",
                schema.0
            ),
            Self::Capacity => {
                f.write_str("Capacity is outside the slot address range or no free slot remains")
            }
            Self::DuplicateId => f.write_str("Stable identity already belongs to a live entity"),
            Self::InvalidHandle => {
                f.write_str("A handle is stale, retired or from another world epoch")
            }
            Self::UnregisteredComponent => {
                f.write_str("A game attempted to use a type absent from setup registration")
            }
            Self::DuplicateComponent => {
                f.write_str("A spawn attempted to add a component type twice")
            }
            Self::MissingComponent => {
                f.write_str("The checked entity does not contain this registered component")
            }
            Self::SnapshotMismatch => {
                f.write_str("Capture and receiver have different generations, capacity or schema")
            }
            Self::WrongRuntime => {
                f.write_str("A typed access token belongs to another compiled runtime")
            }
            Self::WrongSystem => {
                f.write_str("A prepared read belongs to another compiled system callback")
            }
        }
    }
}

impl std::error::Error for RegisteredWorldError {}

/// Reusable typed bundle writer. It never exposes storage rows or a mutable world.
pub struct EntityWriter<'a> {
    slot: u32,
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
        if self.registry.entries[index]
            .profile
            .as_ref()
            .is_some_and(|profile| !(profile.valid)(&value))
        {
            return Err(RegisteredWorldError::NonFinite {
                schema: self.registry.entries[index].schema,
                slot: self.slot,
            });
        }
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
    directory: Option<DenseDirectory>,
    activity_component: Option<usize>,
    free: Vec<u32>,
    ids: IdIndex,
    columns: Vec<Box<dyn StorageColumn>>,
    builder: EntityBundle,
    registry: ComponentRegistry,
    change_ticks: Vec<u64>,
    changed: Vec<Vec<u32>>,
    pending_slots: Vec<u32>,
    pending_marks: Vec<bool>,
    live: usize,
    /// Per-component reusable chunk outputs for chunked in-place kernels.
    chunk_scratch: Vec<Option<Box<dyn Any + Send + Sync>>>,
}

/// Storage words per chunk of a word-aligned in-place kernel (4,096 rows),
/// sized so one Rayon task outweighs its dispatch on small row kernels.
pub(super) const KERNEL_CHUNK_WORDS: usize = 64;

/// Outcome of one chunked in-place visit.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ChunkedVisit {
    /// Rows whose visit reported a change.
    pub changed: usize,
    /// Chunks dispatched to the Rayon pool; zero when the visit ran serially.
    pub dispatched_chunks: usize,
}

/// One chunk's private kernel output, merged on the caller in chunk order.
struct ChunkOutput<T> {
    /// First-write preimages: logical slot, prior value and prior change tick.
    preimages: Vec<(u32, T, u64)>,
    changed: usize,
}

impl<T> Default for ChunkOutput<T> {
    fn default() -> Self {
        Self {
            preimages: Vec::new(),
            changed: 0,
        }
    }
}

impl RegisteredWorld {
    pub(super) fn copy_free_slots(&self, output: &mut Vec<u32>) {
        output.clone_from(&self.free);
    }
    pub(super) fn validate_profile_value(
        &self,
        component: usize,
        value: &dyn Any,
        slot: u32,
    ) -> Result<(), RegisteredWorldError> {
        let entry = &self.registry.entries[component];
        if entry
            .profile
            .as_ref()
            .is_some_and(|profile| !(profile.valid)(value))
        {
            return Err(RegisteredWorldError::NonFinite {
                schema: entry.schema,
                slot,
            });
        }
        Ok(())
    }

    pub(super) fn validate_profile(&self) -> Result<(), RegisteredWorldError> {
        for (index, entry) in self.registry.entries.iter().enumerate() {
            if let Some(profile) = &entry.profile
                && let Some(slot) = (profile.validate)(self, index)
            {
                return Err(RegisteredWorldError::NonFinite {
                    schema: entry.schema,
                    slot,
                });
            }
        }
        Ok(())
    }

    pub(super) fn digest(
        &self,
        digest: &mut crate::profile::Digest<'_>,
        partial: &mut impl FnMut(ComponentSchemaId, u64),
    ) -> Result<(), ComponentSchemaId> {
        use crate::profile::{Digest, StateEncode};
        self.encode_metadata(digest);
        for component in self.schema_order() {
            let entry = &self.registry.entries[component];
            let mut column = Digest::default();
            self.encode_column(component, &mut column)?;
            let digest_value = column.finish();
            partial(entry.schema, digest_value);
            digest_value.encode(digest);
        }
        Ok(())
    }

    fn encode_metadata(&self, digest: &mut crate::profile::Digest<'_>) {
        use crate::profile::StateEncode;
        u64::try_from(self.slots.len())
            .expect("capacity fits u64")
            .encode(digest);
        for slot in &self.slots {
            slot.stable.is_some().encode(digest);
            slot.generation.encode(digest);
            slot.stable.map_or(0, |id| id.0).encode(digest);
            slot.structural_tick.encode(digest);
        }
        self.encode_free_list(digest);
        // The identity index is canonicalized by logical slot, independently of
        // randomized bucket layout. Encode actual lookups to catch a broken index.
        for slot in &self.slots {
            let found = slot.stable.and_then(|id| self.ids.get(id));
            found.is_some().encode(digest);
            found.unwrap_or(0).encode(digest);
        }
        self.directory.is_some().encode(digest);
        if let Some(directory) = &self.directory {
            directory.encode(digest);
        }
    }

    pub(super) fn encode_free_list(&self, digest: &mut crate::profile::Digest<'_>) {
        use crate::profile::StateEncode;
        u64::try_from(self.free.len())
            .expect("free length fits u64")
            .encode(digest);
        for slot in &self.free {
            slot.encode(digest);
        }
    }

    fn schema_order(&self) -> Vec<usize> {
        let mut order: Vec<_> = (0..self.component_count()).collect();
        order.sort_unstable_by_key(|&index| self.registry.entries[index].schema.0);
        order
    }

    fn encode_column(
        &self,
        component: usize,
        sink: &mut crate::profile::Digest<'_>,
    ) -> Result<(), ComponentSchemaId> {
        use crate::profile::StateEncode;
        let entry = &self.registry.entries[component];
        let profile = entry.profile.as_ref().ok_or(entry.schema)?;
        entry.schema.0.encode(sink);
        for slot in 0..self.capacity() {
            self.change_ticks[tick_index(component, slot, self.capacity())].encode(sink);
        }
        (profile.encode)(self, component, sink);
        Ok(())
    }
    pub(super) fn capacity(&self) -> usize {
        self.slots.len()
    }

    pub(super) fn row_of_slot(&self, slot: u32) -> usize {
        self.directory
            .as_ref()
            .map_or(slot as usize, |directory| directory.row_of(slot as usize))
    }

    pub(super) fn directory(&self) -> Option<&DenseDirectory> {
        self.directory.as_ref()
    }

    pub(super) fn activity_component(&self) -> Option<usize> {
        self.activity_component
    }

    pub(super) fn active_at_slot(&self, slot: u32) -> bool {
        let Some(component) = self.activity_component else {
            return false;
        };
        let row = self.row_of_slot(slot);
        let words = self.columns[component]
            .active_presence()
            .expect("selected activity component has an index");
        words[row / 64] & (1_u64 << (row % 64)) != 0
    }

    pub(super) fn slot_at_row(&self, row: usize) -> usize {
        self.directory
            .as_ref()
            .map_or(row, |directory| directory.slot_at(row))
    }

    pub(super) fn component_count(&self) -> usize {
        self.columns.len()
    }

    pub(super) fn visit_changed_components(&self, mut visit: impl FnMut(usize, u32)) {
        for (component, slots) in self.changed.iter().enumerate() {
            for &slot in slots {
                visit(component, slot);
            }
        }
    }

    pub(super) fn restore_first_seen(&mut self, undo: &mut WorldUndo, component: usize) {
        let capacity = self.slots.len();
        undo.columns[component].restore_first_seen_into(
            self.columns[component].as_mut(),
            &mut self.change_ticks,
            component,
            capacity,
            self.directory.as_ref(),
            &mut self.pending_marks,
            &mut self.pending_slots,
        );
    }

    pub(super) fn empty_undo(&self) -> WorldUndo {
        WorldUndo {
            rule_boundaries: false,
            slots: Vec::new(),
            structural_ticks: Vec::new(),
            layout_swaps: Vec::with_capacity(256),
            columns: self
                .registry
                .entries
                .iter()
                .map(|entry| (entry.undo)())
                .collect(),
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
            id_index_slots: self
                .slots
                .iter()
                .map(|slot| slot.stable.and_then(|id| self.ids.get(id)))
                .collect(),
            directory: self.directory.clone(),
        }
    }

    pub(super) fn restore(&mut self, capture: &WorldCapture) -> Result<(), RegisteredWorldError> {
        if capture.epoch != self.epoch
            || capture.slots.len() != self.slots.len()
            || capture.schema.len() != self.registry.entries.len()
            || capture.types.len() != self.registry.entries.len()
            || capture.columns.len() != self.registry.entries.len()
            || capture.change_ticks.len() != self.change_ticks.len()
            || capture.id_index_slots.len() != self.slots.len()
            || capture.directory.is_some() != self.directory.is_some()
            || capture
                .slots
                .iter()
                .zip(&capture.id_index_slots)
                .enumerate()
                .any(|(slot, (state, mapped))| {
                    *mapped
                        != state
                            .stable
                            .map(|_| u32::try_from(slot).expect("slot fits u32"))
                })
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
        self.directory.clone_from(&capture.directory);
        for column in &mut self.columns {
            column.clear_all();
        }
        for (snapshot, column) in capture.columns.iter().zip(&mut self.columns) {
            snapshot.restore_into(column.as_mut(), self.directory.as_ref());
        }
        self.ids.clear();
        let mut live = 0;
        for (index, saved) in capture.slots.iter().enumerate() {
            if let Some(id) = saved.stable {
                let key = u32::try_from(index).expect("validated slot capacity");
                self.ids.insert(id, key);
                live += 1;
            }
            self.slots[index] = Slot {
                generation: saved.generation,
                stable: saved.stable,
                structural_tick: saved.structural_tick,
            };
        }
        self.builder.clear();
        self.free.clone_from(&capture.free);
        self.change_ticks.clone_from(&capture.change_ticks);
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
        let directory = registry
            .activity_partition
            .map(|_| DenseDirectory::new(capacity));
        let activity_component = registry
            .activity_partition
            .and_then(|key| registry.entries.iter().position(|entry| entry.key == key));
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
            directory,
            activity_component,
            free: (0..slot_count).rev().collect(),
            ids: IdIndex::new(capacity),
            columns,
            builder,
            registry,
            change_ticks: vec![0; change_count],
            changed: (0..component_count).map(|_| Vec::new()).collect(),
            pending_slots: Vec::with_capacity(capacity),
            pending_marks: vec![false; capacity],
            live: 0,
            chunk_scratch: (0..component_count).map(|_| None).collect(),
        })
    }

    /// Number of live entities.
    pub(super) fn len(&self) -> usize {
        self.live
    }

    pub(super) fn free_count(&self) -> usize {
        self.free.len()
    }

    pub(super) fn next_free_slot(&self) -> u32 {
        *self.free.last().expect("prevalidated spawn capacity")
    }

    pub(super) fn component_index(&self, key: ComponentKey) -> Option<usize> {
        self.registry
            .entries
            .iter()
            .position(|entry| entry.key == key)
    }

    pub(super) fn has_active_index(&self, component: usize) -> bool {
        self.columns[component].active_presence().is_some()
    }

    pub(super) fn registry(&self) -> &ComponentRegistry {
        &self.registry
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

    pub(super) fn mark_changed(&mut self, component: usize, handle: Handle, tick: u64) {
        self.change_ticks[tick_index(component, handle.slot as usize, self.slots.len())] = tick;
        self.changed[component].push(handle.slot);
        self.mark_pending(handle.slot);
    }

    pub(super) fn change_tick_at(&self, component: usize, slot: u32) -> u64 {
        self.change_ticks[tick_index(component, slot as usize, self.capacity())]
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

    pub(super) fn live_at_slot(&self, slot: u32) -> Option<(Handle, StableId)> {
        let stable = self.slots.get(slot as usize)?.stable?;
        Some((self.handle_at(slot), stable))
    }

    pub(super) fn prepare_bundle(
        &self,
        builder: &mut EntityBundle,
        slot: u32,
        fill: impl FnOnce(&mut EntityWriter<'_>) -> Result<(), RegisteredWorldError>,
    ) -> Result<(), RegisteredWorldError> {
        builder.clear();
        let result = fill(&mut EntityWriter {
            slot,
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
        let row = self.row_of_slot(slot);
        builder.commit(&mut self.columns, row);
        self.free.pop();
        self.slots[slot as usize].stable = Some(id);
        self.slots[slot as usize].structural_tick = tick;
        self.ids.insert(id, slot);
        self.live += 1;
        Ok(self.handle_at(slot))
    }

    fn reconcile_activity(&mut self, slot: u32, mut undo: Option<&mut WorldUndo>) {
        let Some(component) = self.activity_component else {
            return;
        };
        let row = self.row_of_slot(slot);
        let words = self.columns[component]
            .active_presence()
            .expect("selected activity component has an index");
        let active = words[row / 64] & (1_u64 << (row % 64)) != 0;
        let Some(swap) = self
            .directory
            .as_mut()
            .expect("activity partition has a directory")
            .transition(slot as usize, active)
        else {
            return;
        };
        for column in &mut self.columns {
            column.swap_rows(swap.row, swap.boundary_row);
        }
        if let Some(record) = &mut undo {
            record.layout_swaps.push(swap);
        }
    }

    pub(super) fn finalize_activity(
        &mut self,
        events: &[super::StructuralChange],
        undo: &mut WorldUndo,
    ) {
        let Some(component) = self.activity_component else {
            return;
        };
        let changed = std::mem::take(&mut self.changed[component]);
        for &slot in &changed {
            self.reconcile_activity(slot, Some(&mut *undo));
        }
        debug_assert!(
            self.directory
                .as_ref()
                .is_none_or(|directory| directory.active_len() <= self.live),
            "active rows must belong to live entities"
        );
        self.changed[component] = changed;
        for event in events {
            let slot = match event {
                super::StructuralChange::Spawn(handle)
                | super::StructuralChange::Despawn(handle) => handle.slot,
            };
            self.reconcile_activity(slot, Some(&mut *undo));
        }
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
            slot,
            builder: &mut self.builder,
            registry: &self.registry,
        });
        if let Err(error) = result {
            self.builder.clear();
            return Err(error);
        }
        let row = self.row_of_slot(slot);
        self.builder.commit(&mut self.columns, row);
        self.free.pop();
        self.slots[slot as usize].stable = Some(id);
        self.ids.insert(id, slot);
        self.live += 1;
        self.reconcile_activity(slot, None);
        Ok(self.handle_at(slot))
    }

    /// Remove a checked entity and retire its generation.
    ///
    /// # Errors
    ///
    /// The handle is stale or from another world.
    pub(super) fn despawn(&mut self, handle: Handle) -> Result<(), RegisteredWorldError> {
        self.despawn_impl(handle, None)
    }

    pub(super) fn despawn_into_undo(
        &mut self,
        handle: Handle,
        undo: &mut WorldUndo,
    ) -> Result<(), RegisteredWorldError> {
        self.despawn_impl(handle, Some(undo))
    }

    fn despawn_impl(
        &mut self,
        handle: Handle,
        undo: Option<&mut WorldUndo>,
    ) -> Result<(), RegisteredWorldError> {
        let recorded = undo.is_some();
        self.entity(handle)?;
        let id = self.slots[handle.slot as usize]
            .stable
            .ok_or(RegisteredWorldError::InvalidHandle)?;
        if let Some(undo) = undo {
            undo.record_structural(self, handle.slot);
            let row = self.row_of_slot(handle.slot);
            for (column, preimage) in self.columns.iter_mut().zip(&mut undo.columns) {
                column.take_into_undo(handle.slot, row, preimage.as_mut());
            }
        } else {
            let row = self.row_of_slot(handle.slot);
            for column in &mut self.columns {
                column.clear_slot(row);
            }
        }
        let slot = &mut self.slots[handle.slot as usize];
        self.ids.remove(id);
        slot.stable = None;
        self.live -= 1;
        if let Some(next) = slot.generation.checked_add(1) {
            slot.generation = next;
            self.free.push(handle.slot);
        }
        if !recorded {
            self.reconcile_activity(handle.slot, None);
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
            .get(self.row_of_slot(handle.slot))
            .ok_or(RegisteredWorldError::MissingComponent)?;
        Ok(read(value))
    }

    pub(super) fn has_component(&self, handle: Handle, key: ComponentKey) -> bool {
        self.entity(handle).is_ok()
            && self
                .component_index(key)
                .is_some_and(|index| self.columns[index].present(self.row_of_slot(handle.slot)))
    }

    pub(super) fn present_at<T: Send + Sync + 'static>(&self, component: usize, slot: u32) -> bool {
        self.column_at::<T>(component)
            .get(self.row_of_slot(slot))
            .is_some()
    }

    pub(super) fn replace_prevalidated<T: Send + Sync + 'static>(
        &mut self,
        component: usize,
        slot: u32,
        value: T,
    ) -> T {
        let row = self.row_of_slot(slot);
        self.column_mut_index::<T>(component)
            .insert(row, value)
            .expect("prevalidated replacement component")
    }

    pub(super) fn put_tentative<T: Clone + PartialEq + Send + Sync + 'static>(
        &mut self,
        component: usize,
        handle: Handle,
        value: T,
        undo: &mut WorldUndo,
        dirty: &mut impl TentativeDirty,
    ) -> Result<bool, RegisteredWorldError> {
        self.entity(handle)?;
        self.validate_profile_value(component, &value, handle.slot)?;
        let slot = handle.slot;
        let row = self.row_of_slot(slot);
        let prior_tick = self.change_tick_at(component, slot);
        let column = self.column_mut_index::<T>(component);
        let prior = column.get(row);
        if prior == Some(&value) {
            return Ok(false);
        }
        let first_write = dirty.mark_first(component, slot);
        if first_write {
            undo.record_component(component, slot, prior_tick, prior.cloned());
        }
        column.insert(row, value);
        Ok(true)
    }

    pub(super) fn rule_column_address<T: Send + Sync + 'static>(
        &mut self,
        component: usize,
    ) -> (std::ptr::NonNull<()>, usize) {
        self.column_mut_index::<T>(component).rule_address()
    }

    pub(super) fn journal_rule_column<T: PartialEq + Send + Sync + 'static>(
        &mut self,
        component: usize,
        values: impl IntoIterator<Item = (Handle, T)>,
        undo: &mut WorldUndo,
        dirty: &mut impl TentativeDirty,
        finite: impl Fn(&T) -> bool,
    ) -> Result<(), RegisteredWorldError> {
        let schema = self.registry.entries[component].schema;
        let capacity = self.slots.len();
        let directory = self.directory.as_ref();
        let ticks = &self.change_ticks;
        let column = self.columns[component]
            .as_any_mut()
            .downcast_mut::<TypedStore<T>>()
            .expect("registered rule column type");
        let mut undo = undo.batch_writer(component);
        let mut result = Ok(());
        for (handle, prior) in values {
            let slot = handle.slot as usize;
            let row = directory.map_or(slot, |directory| directory.row_of(slot));
            let current = column.get(row).expect("loaned row is present");
            if current == &prior {
                continue;
            }
            if result.is_ok() && !finite(current) {
                result = Err(RegisteredWorldError::NonFinite {
                    schema,
                    slot: handle.slot,
                });
            }
            if dirty.mark_first(component, handle.slot) {
                let previous_tick = ticks[tick_index(component, slot, capacity)];
                undo.push(handle.slot, prior, previous_tick);
            }
            column.refresh_active(row);
        }
        result
    }

    pub(super) fn replace_batch_prevalidated<T: Clone + Send + Sync + 'static>(
        &mut self,
        component: usize,
        rows: &[(Handle, T)],
        tick: u64,
        mut save_old: impl FnMut(u32, T, u64),
    ) {
        let capacity = self.slots.len();
        let directory = self.directory.as_ref();
        let column = self.columns[component]
            .as_any_mut()
            .downcast_mut::<TypedStore<T>>()
            .expect("registered storage column type");
        let changed = &mut self.changed[component];
        for (handle, value) in rows {
            let slot = handle.slot as usize;
            let row = directory.map_or(slot, |directory| directory.row_of(slot));
            let prior = column
                .insert(row, value.clone())
                .expect("prevalidated replacement component");
            let old_tick = self.change_ticks[tick_index(component, slot, capacity)];
            save_old(handle.slot, prior, old_tick);
            self.change_ticks[tick_index(component, slot, capacity)] = tick;
            changed.push(handle.slot);
            if !self.pending_marks[slot] {
                self.pending_marks[slot] = true;
                self.pending_slots.push(handle.slot);
            }
        }
    }

    pub(super) fn insert_prevalidated<T: Send + Sync + 'static>(
        &mut self,
        component: usize,
        slot: u32,
        value: T,
    ) {
        let row = self.row_of_slot(slot);
        assert!(
            self.column_mut_index::<T>(component)
                .insert(row, value)
                .is_none(),
            "prevalidated insertion slot is empty"
        );
    }

    pub(super) fn remove_prevalidated<T: Send + Sync + 'static>(
        &mut self,
        component: usize,
        slot: u32,
    ) -> T {
        let row = self.row_of_slot(slot);
        self.column_mut_index::<T>(component)
            .remove(row)
            .expect("prevalidated removal component")
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

    /// Visit independent row kernels over an indexed shared component in
    /// storage chunks. Each chunk of [`KERNEL_CHUNK_WORDS`]
    /// storage words owns its values and active words; first-write preimages
    /// stay private until the caller merges them in storage order. Every
    /// chunk's preimages are kept on refusal so the transaction abort restores
    /// rows any chunk already wrote; the earliest chunk's error is returned.
    /// Serial and parallel dispatch run the same chunk code. Returns the number
    /// of rows whose visit reported a change.
    pub(super) fn visit_mut2_chunked<
        T: Clone + Send + Sync + 'static,
        U: Send + Sync + 'static,
        C: Sync,
        E: Send,
    >(
        &mut self,
        first: usize,
        second: usize,
        excluded_words: &[u64],
        parallel: bool,
        undo: &mut WorldUndo,
        dirty: &mut (impl TentativeDirty + Sync),
        context: &C,
        select: impl Fn(&C, Handle, StableId, &T, &U) -> bool + Sync,
        visit: impl Fn(&C, Handle, StableId, &mut T, &U) -> Result<bool, E> + Sync,
    ) -> Result<ChunkedVisit, E> {
        assert_ne!(first, second, "mutable query components must be distinct");
        let mut scratch = self.chunk_scratch[first]
            .take()
            .unwrap_or_else(|| Box::new(Vec::<ChunkOutput<T>>::new()));
        let outputs = scratch
            .downcast_mut::<Vec<ChunkOutput<T>>>()
            .expect("chunk scratch matches its component type");
        let capacity = self.slots.len();
        let words = capacity.div_ceil(64);
        // Activity partitions keep active rows dense, so only chunks up to the
        // last active word carry work.
        let active_words = self.columns[second]
            .active_presence()
            .expect("indexed shared component");
        let last = active_words.iter().rposition(|word| *word != 0);
        let chunks = last.map_or(0, |last| (last + 1).div_ceil(KERNEL_CHUNK_WORDS));
        // Chunks run on the pool only under a parallel executor with more than one.
        let dispatched_chunks = if parallel && chunks > 1 && cfg!(not(target_arch = "wasm32")) {
            chunks
        } else {
            0
        };
        if outputs.len() < chunks {
            outputs.resize_with(chunks, ChunkOutput::default);
        }
        let epoch = self.epoch;
        let directory = self.directory.as_ref();
        let slots = self.slots.as_slice();
        let change_ticks = self.change_ticks.as_slice();
        let (write, read) = if first < second {
            let (left, right) = self.columns.split_at_mut(second);
            (&mut left[first], &right[0])
        } else {
            let (left, right) = self.columns.split_at_mut(first);
            (&mut right[0], &left[second])
        };
        let write = write
            .as_any_mut()
            .downcast_mut::<TypedStore<T>>()
            .expect("prepared mutable component type");
        let read = read
            .as_any()
            .downcast_ref::<TypedStore<U>>()
            .expect("prepared shared component type");
        let read_active = read.active_presence().expect("indexed shared component");
        let read_presence = read.presence();
        let parts = write.split_for_chunks();
        let presence = parts.presence;
        let predicate = parts.active.as_ref().map(|(_, predicate)| *predicate);
        let refusal = std::sync::Mutex::new(None::<(usize, E)>);
        let marks: &(dyn TentativeDirty + Sync) = &*dirty;
        let run = |chunk: usize,
                   values: &mut [Option<T>],
                   active: &mut [u64],
                   output: &mut ChunkOutput<T>| {
            output.preimages.clear();
            output.changed = 0;
            let start = chunk * KERNEL_CHUNK_WORDS;
            for word in start..(start + KERNEL_CHUNK_WORDS).min(words) {
                let mut matching = presence[word]
                    & read_presence[word]
                    & read_active[word]
                    & !excluded_words[word];
                while matching != 0 {
                    let bit = matching.trailing_zeros() as usize;
                    matching &= matching - 1;
                    let index = word * 64 + bit;
                    let local = index - start * 64;
                    let logical = directory.map_or(index, |directory| directory.slot_at(index));
                    let key = u32::try_from(logical).expect("validated slot capacity");
                    let slot = slots[logical];
                    let id = slot
                        .stable
                        .expect("stored component belongs to a live slot");
                    let handle = Handle {
                        epoch,
                        slot: key,
                        generation: slot.generation,
                    };
                    let shared = read.get(index).expect("presence bit has value");
                    let value = values[local].as_mut().expect("presence bit has value");
                    if !select(context, handle, id, value, shared) {
                        continue;
                    }
                    let first_write = !marks.is_marked(first, key);
                    if first_write {
                        output.preimages.push((
                            key,
                            value.clone(),
                            change_ticks[tick_index(first, logical, capacity)],
                        ));
                    }
                    let result = visit(context, handle, id, value, shared);
                    if let Some(predicate) = predicate {
                        let mask = 1_u64 << bit;
                        let target = &mut active[word - start];
                        if values[local].as_ref().is_some_and(predicate) {
                            *target |= mask;
                        } else {
                            *target &= !mask;
                        }
                    }
                    match result {
                        Ok(true) => output.changed += 1,
                        Ok(false) => {
                            if first_write {
                                output.preimages.pop();
                            }
                        }
                        Err(error) => {
                            let mut refusal = refusal.lock().expect("refusal lock");
                            if refusal
                                .as_ref()
                                .is_none_or(|(earliest, _)| chunk < *earliest)
                            {
                                *refusal = Some((chunk, error));
                            }
                            return;
                        }
                    }
                }
            }
        };
        let rows = KERNEL_CHUNK_WORDS * 64;
        let used = &mut outputs[..chunks];
        let values = &mut parts.values[..(chunks * rows).min(capacity)];
        // A panicking callback must not strand other chunks' writes: catch it,
        // merge every chunk's preimages so the tick abort restores them, then
        // resume. Each chunk records a preimage before visiting its row.
        let dispatched = catch_unwind(AssertUnwindSafe(|| match parts.active {
            Some((active, _)) => {
                let active = &mut active[..(chunks * KERNEL_CHUNK_WORDS).min(words)];
                #[cfg(not(target_arch = "wasm32"))]
                if parallel && chunks > 1 {
                    use rayon::prelude::*;
                    values
                        .par_chunks_mut(rows)
                        .zip(active.par_chunks_mut(KERNEL_CHUNK_WORDS))
                        .zip(used.par_iter_mut())
                        .enumerate()
                        .for_each(|(chunk, ((values, active), output))| {
                            run(chunk, values, active, output);
                        });
                } else {
                    run_serial_chunks(values, Some(active), used, rows, &run);
                }
                #[cfg(target_arch = "wasm32")]
                run_serial_chunks(values, Some(active), used, rows, &run);
            }
            None => {
                #[cfg(not(target_arch = "wasm32"))]
                if parallel && chunks > 1 {
                    use rayon::prelude::*;
                    values
                        .par_chunks_mut(rows)
                        .zip(used.par_iter_mut())
                        .enumerate()
                        .for_each(|(chunk, (values, output))| {
                            run(chunk, values, &mut [], output);
                        });
                } else {
                    run_serial_chunks(values, None, used, rows, &run);
                }
                #[cfg(target_arch = "wasm32")]
                run_serial_chunks(values, None, used, rows, &run);
            }
        }));
        #[cfg(target_arch = "wasm32")]
        let _ = parallel;
        let undo_column = undo.columns[first]
            .as_any_mut()
            .downcast_mut::<TypedUndo<T>>()
            .expect("registered undo column type");
        let mut changed = 0;
        for output in &mut outputs[..chunks] {
            changed += output.changed;
            for (key, prior, tick) in output.preimages.drain(..) {
                let marked = dirty.mark_first(first, key);
                debug_assert!(marked, "chunk preimage is the first write");
                undo_column.push_component(key, Some(prior), tick);
            }
        }
        self.chunk_scratch[first] = Some(scratch);
        if let Err(payload) = dispatched {
            resume_unwind(payload);
        }
        match refusal.into_inner().expect("refusal lock") {
            Some((_, error)) => Err(error),
            None => Ok(ChunkedVisit {
                changed,
                dispatched_chunks,
            }),
        }
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
            let row = self
                .directory
                .as_ref()
                .map_or(index, |directory| directory.row_of(index));
            let (Some(a), Some(b)) = (first.get(row), second.get(row)) else {
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
                let logical = world.slot_at_row(index);
                let key = u32::try_from(logical).expect("validated slot capacity");
                let id = world.slots[logical]
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

    use super::{
        ComponentRegistry, IdIndex, RegisteredWorld, RegisteredWorldError, TentativeDirty,
    };
    use crate::sim::ComponentSchemaId;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Height(f64);
    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Drift(f64);
    struct Unregistered;

    struct TestDirty([bool; 4]);

    impl TentativeDirty for TestDirty {
        fn is_marked(&self, component: usize, slot: u32) -> bool {
            self.0[component * 2 + slot as usize]
        }

        fn mark_first(&mut self, component: usize, slot: u32) -> bool {
            let mark = &mut self.0[component * 2 + slot as usize];
            let first = !*mark;
            *mark = true;
            first
        }
    }

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
    fn tentative_upsert_restores_absence_and_first_value_after_repeated_writes() {
        let mut world = world();
        let handle = world
            .spawn(StableId(1), |entity| {
                entity.add(Height(2.0))?;
                Ok(())
            })
            .expect("height-only entity");
        let drift = world
            .component_index(super::ComponentKey::of::<Drift>())
            .expect("registered drift");
        let mut undo = world.empty_undo();
        let mut marks = TestDirty([false; 4]);
        assert_eq!(
            world.put_tentative(drift, handle, Drift(3.0), &mut undo, &mut marks),
            Ok(true)
        );
        assert_eq!(
            world.put_tentative(drift, handle, Drift(4.0), &mut undo, &mut marks),
            Ok(true)
        );
        assert_eq!(
            world.with::<Drift, _>(handle, |value| *value),
            Ok(Drift(4.0))
        );
        undo.abort_tentative(&mut world);
        assert_eq!(
            world.with::<Drift, _>(handle, |value| *value),
            Err(RegisteredWorldError::MissingComponent)
        );
        marks.0.fill(false);
        assert_eq!(
            world.put_tentative(drift, handle, Drift(5.0), &mut undo, &mut marks),
            Ok(true)
        );
        undo.abort_tentative(&mut world);
        world.despawn(handle).expect("retire checked handle");
        assert_eq!(
            world.put_tentative(drift, handle, Drift(6.0), &mut undo, &mut marks),
            Err(RegisteredWorldError::InvalidHandle)
        );
    }

    #[test]
    fn component_ticks_survive_tentative_abort_committed_undo_and_slot_reuse() {
        let mut world = world();
        let first = world
            .spawn(StableId(1), |entity| {
                entity.add(Height(1.0))?;
                entity.add(Drift(2.0))?;
                Ok(())
            })
            .expect("first entity");
        let second = world
            .spawn(StableId(2), |entity| {
                entity.add(Height(3.0))?;
                entity.add(Drift(4.0))?;
                Ok(())
            })
            .expect("second entity");
        let height = world
            .component_index(super::ComponentKey::of::<Height>())
            .expect("height component");
        let drift = world
            .component_index(super::ComponentKey::of::<Drift>())
            .expect("drift component");
        world.mark_changed(height, first, 3);
        world.mark_changed(drift, second, 5);
        let expected = [(3, 0), (0, 5)];
        let ticks = |world: &RegisteredWorld| {
            [first, second].map(|handle| {
                (
                    world.change_tick_at(height, handle.slot),
                    world.change_tick_at(drift, handle.slot),
                )
            })
        };
        assert_eq!(ticks(&world), expected);

        let mut tentative = world.empty_undo();
        let mut marks = TestDirty([false; 4]);
        world
            .put_tentative(height, first, Height(9.0), &mut tentative, &mut marks)
            .expect("tentative write");
        tentative.abort_tentative(&mut world);
        assert_eq!(ticks(&world), expected);
        assert_eq!(world.with::<Height, _>(first, |value| value.0), Ok(1.0));

        let mut committed = world.empty_undo();
        world.begin_tick();
        let prior = world.replace_prevalidated(height, first.slot, Height(10.0));
        committed.record_component(height, first.slot, 3, Some(prior));
        world.mark_changed(height, first, 8);
        committed.restore(&mut world, &[]);
        assert_eq!(ticks(&world), expected);

        let mut structural = world.empty_undo();
        world.begin_tick();
        world
            .despawn_into_undo(second, &mut structural)
            .expect("retire second entity");
        structural.record_structural(&world, world.next_free_slot());
        let replacement = world
            .spawn(StableId(3), |entity| {
                entity.add(Height(5.0))?;
                entity.add(Drift(6.0))?;
                Ok(())
            })
            .expect("reuse second slot");
        assert_eq!(replacement.slot, second.slot);
        structural.restore(
            &mut world,
            &[
                crate::sim::StructuralChange::Despawn(second),
                crate::sim::StructuralChange::Spawn(replacement),
            ],
        );
        assert_eq!(world.handle(StableId(2)), Some(second));
        assert_eq!(ticks(&world), expected);
        assert_eq!(world.with::<Drift, _>(second, |value| value.0), Ok(4.0));
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
        world.replace_prevalidated(height, first.slot, Height(6.0));
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

    #[test]
    fn dense_activity_keeps_handles_and_replays_layout_through_capture_and_undo() {
        use std::{collections::hash_map::DefaultHasher, hash::Hasher};

        let mut registry = ComponentRegistry::default();
        registry
            .register::<Height>(ComponentSchemaId(1))
            .expect("height");
        registry
            .register_indexed::<Drift>(ComponentSchemaId(2), |value| value.0 != 0.0)
            .expect("indexed drift");
        registry
            .select_activity_partition::<Drift>()
            .expect("partition");
        let mut world = RegisteredWorld::new(WorldEpoch(8), 6, registry).expect("world");
        let mut handles = Vec::new();
        for slot in 0..4_u128 {
            let speed = if slot % 2 == 0 { 0.0 } else { 1.0 };
            handles.push(
                world
                    .spawn(StableId(slot), |entity| {
                        entity.add(Height(f64::from(u32::try_from(slot).expect("small slot"))))?;
                        entity.add(Drift(speed))?;
                        Ok(())
                    })
                    .expect("interleaved seed"),
            );
        }
        assert_eq!(world.directory().expect("dense").active_len(), 2);
        assert!(world.row_of_slot(handles[1].slot) < 2);
        assert!(world.row_of_slot(handles[3].slot) < 2);
        assert_eq!(
            world.with::<Height, _>(handles[2], |height| height.0),
            Ok(2.0)
        );
        let saved = world.capture();
        let mut undo = world.empty_undo();
        let drift = world
            .component_index(super::ComponentKey::of::<Drift>())
            .expect("registered drift");
        world.begin_tick();
        let prior = world.replace_prevalidated(drift, handles[0].slot, Drift(3.0));
        undo.record_component(drift, handles[0].slot, 0, Some(prior));
        world.mark_changed(drift, handles[0], 1);
        world.finalize_activity(&[], &mut undo);
        let future = world.capture();
        assert!(world.row_of_slot(handles[0].slot) < 3);
        world
            .restore(&saved)
            .expect("capture restores earlier layout");
        assert_eq!(world.directory().expect("dense").active_len(), 2);
        assert_eq!(world.with::<Drift, _>(handles[0], |value| value.0), Ok(0.0));
        world
            .restore(&future)
            .expect("capture restores active layout");
        let mut expected_hash = DefaultHasher::new();
        future.hash_metadata(&mut expected_hash);
        let mut actual_hash = DefaultHasher::new();
        world.capture().hash_metadata(&mut actual_hash);
        assert_eq!(actual_hash.finish(), expected_hash.finish());
        assert!(world.row_of_slot(handles[0].slot) < 3);
        assert_eq!(world.with::<Drift, _>(handles[0], |value| value.0), Ok(3.0));
        undo.restore(&mut world, &[]);
        assert_eq!(world.directory().expect("dense").active_len(), 2);
        assert_eq!(world.with::<Drift, _>(handles[0], |value| value.0), Ok(0.0));
        assert_eq!(
            world.with::<Height, _>(handles[2], |value| value.0),
            Ok(2.0)
        );

        let mut structural_undo = world.empty_undo();
        world.begin_tick();
        world
            .despawn_into_undo(handles[1], &mut structural_undo)
            .expect("active boundary deletion");
        world.mark_structural(handles[1].slot);
        let events = [crate::sim::StructuralChange::Despawn(handles[1])];
        world.finalize_activity(&events, &mut structural_undo);
        assert_eq!(world.directory().expect("dense").active_len(), 1);
        assert_eq!(world.handle(StableId(1)), None);
        assert_eq!(
            world.with::<Height, _>(handles[3], |value| value.0),
            Ok(3.0)
        );
        structural_undo.restore(&mut world, &events);
        assert_eq!(world.directory().expect("dense").active_len(), 2);
        assert_eq!(world.handle(StableId(1)), Some(handles[1]));
        assert_eq!(
            world.with::<Height, _>(handles[1], |value| value.0),
            Ok(1.0)
        );
    }

    #[test]
    fn refusing_visit_refreshes_the_written_rows_active_bit() {
        // Ultra review #2: a refusing callback may already have changed its
        // row, so the written component's active index must still match it.
        // The chunked visit needs an indexed read column; the written column
        // is indexed too, so its active bit is what this test watches.
        let mut registry = ComponentRegistry::default();
        registry
            .register_indexed::<Height>(ComponentSchemaId(1), |height| height.0 != 0.0)
            .expect("indexed height registers");
        registry
            .register_indexed::<Drift>(ComponentSchemaId(2), |drift| drift.0 != 0.0)
            .expect("indexed drift registers");
        let mut world = RegisteredWorld::new(WorldEpoch(8), 2, registry).expect("capacity fits");
        world
            .spawn(StableId(1), |entity| {
                entity.add(Height(1.0))?;
                entity.add(Drift(1.0))?;
                Ok(())
            })
            .expect("entity");
        let height = world
            .component_index(super::ComponentKey::of::<Height>())
            .expect("height");
        let drift = world
            .component_index(super::ComponentKey::of::<Drift>())
            .expect("drift");
        let mut undo = world.empty_undo();
        let mut marks = TestDirty([false; 4]);
        let refused = world.visit_mut2_chunked::<Height, Drift, (), ()>(
            height,
            drift,
            &[0],
            false,
            &mut undo,
            &mut marks,
            &(),
            |(), _, _, _, _| true,
            |(), _, _, value, _| {
                value.0 = 0.0;
                Err(())
            },
        );
        assert!(matches!(refused, Err(())));
        assert_eq!(
            world.column::<Height>().active_presence().expect("index")[0],
            0,
            "the refused row's zero height is inactive"
        );
        undo.abort_tentative(&mut world);
        assert_eq!(
            world.column::<Height>().active_presence().expect("index")[0],
            1,
            "abort restores the value and its active bit"
        );
    }
}
