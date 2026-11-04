//! Game-registered component storage with engine-owned identity. This is the
//! G2 replacement seam for the fixed A1 store; only checked handles escape.

use std::{collections::hash_map::RandomState, hash::BuildHasher};

use crate::{Handle, StableId, WorldEpoch};

use super::{ComponentKey, ComponentRegistry, ComponentSchemaId, schedule::SnapshotColumn};

struct CapturedSlot {
    generation: u64,
    stable: Option<StableId>,
}

pub(super) struct WorldCapture {
    epoch: WorldEpoch,
    schema: Vec<ComponentSchemaId>,
    types: Vec<ComponentKey>,
    slots: Vec<CapturedSlot>,
    columns: Vec<Box<dyn SnapshotColumn>>,
    free: Vec<u32>,
}

#[derive(Clone, Copy)]
struct Slot {
    generation: u64,
    entity: Option<hecs::Entity>,
    stable: Option<StableId>,
}

#[derive(Clone, Copy)]
pub(super) struct SlotKey(pub(super) u32);

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
}

/// Reusable typed bundle writer. It never exposes hecs IDs or a mutable world.
pub struct EntityWriter<'a> {
    builder: &'a mut hecs::EntityBuilder,
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
        if self.registry.schema(ComponentKey::of::<T>()).is_none() {
            return Err(RegisteredWorldError::UnregisteredComponent);
        }
        if self.builder.has::<T>() {
            return Err(RegisteredWorldError::DuplicateComponent);
        }
        self.builder.add(value);
        Ok(self)
    }
}

/// Generic component authority for one world epoch. Identity slots and hecs
/// entities are private; the public handle is checked on every sparse access.
pub(super) struct RegisteredWorld {
    epoch: WorldEpoch,
    slots: Vec<Slot>,
    free: Vec<u32>,
    ids: IdIndex,
    pub(super) world: hecs::World,
    builder: hecs::EntityBuilder,
    registry: ComponentRegistry,
    live: usize,
}

impl RegisteredWorld {
    pub(super) fn capture(&self) -> WorldCapture {
        let slots = self
            .slots
            .iter()
            .map(|slot| CapturedSlot {
                generation: slot.generation,
                stable: slot.stable,
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
                .map(|entry| (entry.capture)(&self.world))
                .collect(),
            free: self.free.clone(),
        }
    }

    pub(super) fn restore(&mut self, capture: &WorldCapture) -> Result<(), RegisteredWorldError> {
        if capture.epoch != self.epoch
            || capture.slots.len() != self.slots.len()
            || capture.schema.len() != self.registry.entries.len()
            || capture.types.len() != self.registry.entries.len()
            || capture.columns.len() != self.registry.entries.len()
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
        let mut world = hecs::World::new();
        let mut builder = hecs::EntityBuilder::new();
        let mut slots = Vec::with_capacity(capture.slots.len());
        let mut ids = IdIndex::new(capture.slots.len());
        let mut live = 0;
        let mut cursors = vec![0; capture.columns.len()];
        for (index, saved) in capture.slots.iter().enumerate() {
            let entity = saved.stable.map(|id| {
                let key = u32::try_from(index).expect("validated slot capacity");
                for (column, cursor) in capture.columns.iter().zip(&mut cursors) {
                    column.build_at(key, cursor, &mut builder);
                }
                builder.add(SlotKey(key));
                let entity = world.spawn(builder.build());
                ids.insert(id, key);
                live += 1;
                entity
            });
            slots.push(Slot {
                generation: saved.generation,
                entity,
                stable: saved.stable,
            });
        }
        self.world = world;
        self.builder = builder;
        self.slots = slots;
        self.ids = ids;
        self.free.clone_from(&capture.free);
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
        Ok(Self {
            epoch,
            slots: vec![
                Slot {
                    generation: 1,
                    entity: None,
                    stable: None,
                };
                capacity
            ],
            free: (0..slot_count).rev().collect(),
            ids: IdIndex::new(capacity),
            world: hecs::World::new(),
            builder: hecs::EntityBuilder::new(),
            registry,
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

    pub(super) fn entity(&self, handle: Handle) -> Result<hecs::Entity, RegisteredWorldError> {
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
        slot.entity.ok_or(RegisteredWorldError::InvalidHandle)
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
        self.builder.add(SlotKey(slot));
        let entity = self.world.spawn(self.builder.build());
        self.free.pop();
        self.slots[slot as usize].entity = Some(entity);
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
        let entity = self.entity(handle)?;
        let id = self.slots[handle.slot as usize]
            .stable
            .ok_or(RegisteredWorldError::InvalidHandle)?;
        self.world
            .despawn(entity)
            .map_err(|_| RegisteredWorldError::InvalidHandle)?;
        let slot = &mut self.slots[handle.slot as usize];
        self.ids.remove(id);
        slot.entity = None;
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
        if self.registry.schema(ComponentKey::of::<T>()).is_none() {
            return Err(RegisteredWorldError::UnregisteredComponent);
        }
        let entity = self.entity(handle)?;
        let value = self
            .world
            .get::<&T>(entity)
            .map_err(|_| RegisteredWorldError::MissingComponent)?;
        Ok(read(&value))
    }

    pub(super) fn replace<T: Send + Sync + 'static>(
        &mut self,
        handle: Handle,
        value: T,
    ) -> Result<T, RegisteredWorldError> {
        if self.registry.schema(ComponentKey::of::<T>()).is_none() {
            return Err(RegisteredWorldError::UnregisteredComponent);
        }
        let entity = self.entity(handle)?;
        let mut current = self
            .world
            .get::<&mut T>(entity)
            .map_err(|_| RegisteredWorldError::MissingComponent)?;
        Ok(std::mem::replace(&mut current, value))
    }

    pub(super) fn insert<T: Send + Sync + 'static>(
        &mut self,
        handle: Handle,
        value: T,
    ) -> Result<(), RegisteredWorldError> {
        if self.registry.schema(ComponentKey::of::<T>()).is_none() {
            return Err(RegisteredWorldError::UnregisteredComponent);
        }
        let entity = self.entity(handle)?;
        if self.world.get::<&T>(entity).is_ok() {
            return Err(RegisteredWorldError::DuplicateComponent);
        }
        self.world
            .insert_one(entity, value)
            .map_err(|_| RegisteredWorldError::InvalidHandle)
    }

    pub(super) fn remove<T: Send + Sync + 'static>(
        &mut self,
        handle: Handle,
    ) -> Result<T, RegisteredWorldError> {
        if self.registry.schema(ComponentKey::of::<T>()).is_none() {
            return Err(RegisteredWorldError::UnregisteredComponent);
        }
        let entity = self.entity(handle)?;
        self.world
            .remove_one::<T>(entity)
            .map_err(|_| RegisteredWorldError::MissingComponent)
    }

    /// Prepare one two-component storage-order query for repeated ticks.
    ///
    /// # Errors
    ///
    /// Either type is absent from setup registration.
    pub(super) fn read2<T: Send + Sync + 'static, U: Send + Sync + 'static>(
        &self,
    ) -> Result<PreparedRead2<T, U>, RegisteredWorldError> {
        if self.registry.schema(ComponentKey::of::<T>()).is_none()
            || self.registry.schema(ComponentKey::of::<U>()).is_none()
        {
            return Err(RegisteredWorldError::UnregisteredComponent);
        }
        Ok(PreparedRead2 {
            query: hecs::PreparedQuery::new(),
        })
    }

    /// Visit matching components in canonical physical-slot order for
    /// order-sensitive reductions and events. Independent kernels should use
    /// the prepared storage-order cursor instead.
    pub(super) fn visit_canonical<T: Send + Sync + 'static, U: Send + Sync + 'static>(
        &self,
        mut visit: impl FnMut(Handle, StableId, &T, &U),
    ) {
        for (index, slot) in self.slots.iter().enumerate() {
            let Some(entity) = slot.entity else { continue };
            let (Ok(a), Ok(b)) = (self.world.get::<&T>(entity), self.world.get::<&U>(entity))
            else {
                continue;
            };
            let key = u32::try_from(index).expect("validated slot capacity");
            visit(
                self.handle_at(key),
                slot.stable.expect("live slot has stable identity"),
                &a,
                &b,
            );
        }
    }

    /// Prepare one mutable component column for repeated validated commits.
    ///
    /// # Errors
    ///
    /// The type is absent from setup registration.
    pub(super) fn write1<T: Send + Sync + 'static>(
        &self,
    ) -> Result<PreparedWrite1<T>, RegisteredWorldError> {
        if self.registry.schema(ComponentKey::of::<T>()).is_none() {
            return Err(RegisteredWorldError::UnregisteredComponent);
        }
        Ok(PreparedWrite1 {
            query: hecs::PreparedQuery::new(),
        })
    }
}

/// Prepared read of two game-owned component types in backend storage order.
/// The callback receives checked physical and stable identity, never a hecs ID.
pub(super) struct PreparedRead2<T: Send + Sync + 'static, U: Send + Sync + 'static> {
    query: hecs::PreparedQuery<(&'static SlotKey, &'static T, &'static U)>,
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
        let mut query = self.query.query(&world.world);
        for (key, a, b) in query.iter() {
            let slot = &world.slots[key.0 as usize];
            visit(
                world.handle_at(key.0),
                slot.stable.expect("queried entity remains live"),
                a,
                b,
            );
        }
    }
}

/// Prepared mutable visit of one registered game component. A caller validates
/// its full candidate transaction before invoking this commit-time cursor.
pub(super) struct PreparedWrite1<T: Send + Sync + 'static> {
    query: hecs::PreparedQuery<(&'static SlotKey, &'static mut T)>,
}

impl<T: Send + Sync + 'static> PreparedWrite1<T> {
    /// Visit matching entities in backend storage order with checked identity.
    ///
    /// # Panics
    ///
    /// Panics only if the private slot-to-entity invariant has been broken.
    pub(super) fn visit(
        &mut self,
        world: &mut RegisteredWorld,
        mut visit: impl FnMut(Handle, StableId, &mut T),
    ) {
        let slots = &world.slots;
        let epoch = world.epoch;
        for (key, component) in self.query.query_mut(&mut world.world) {
            let slot = &slots[key.0 as usize];
            visit(
                Handle {
                    epoch,
                    slot: key.0,
                    generation: slot.generation,
                },
                slot.stable.expect("queried entity remains live"),
                component,
            );
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
        let mut write = world.write1::<Height>().expect("position registered");
        write.visit(&mut world, |handle, id, position| {
            assert_eq!((handle, id), (first, StableId(1)));
            position.0 += 4.0;
        });
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
