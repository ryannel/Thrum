//! Game-registered component storage with engine-owned identity. This is the
//! G2 replacement seam for the fixed A1 store; only checked handles escape.

use std::{collections::hash_map::RandomState, hash::BuildHasher};

use crate::{Handle, StableId, WorldEpoch};

use super::{ComponentKey, ComponentRegistry};

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
pub struct RegisteredWorld {
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
    /// Reserve physical slots and adopt the game's setup-time component registry.
    ///
    /// # Errors
    ///
    /// Capacity cannot be represented by the physical slot index.
    pub fn new(
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

    /// World epoch for checked handles and committed read models.
    pub fn epoch(&self) -> WorldEpoch {
        self.epoch
    }

    /// Number of live entities.
    pub fn len(&self) -> usize {
        self.live
    }

    /// Whether this world currently has no entities.
    pub fn is_empty(&self) -> bool {
        self.live == 0
    }

    /// Physical capacity selected when this world was built.
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    pub(super) fn free_count(&self) -> usize {
        self.free.len()
    }

    pub(super) fn reusable_after_despawn(&self, handle: Handle) -> bool {
        self.slots[handle.slot as usize].generation != u64::MAX
    }

    pub(super) fn live_slot(&self, slot: usize) -> bool {
        self.slots[slot].entity.is_some()
    }

    pub(super) fn id_at_slot(&self, slot: usize) -> StableId {
        self.slots[slot]
            .stable
            .expect("live physical slot has stable identity")
    }

    pub(super) fn entity_at_slot(&self, slot: usize) -> hecs::Entity {
        self.slots[slot]
            .entity
            .expect("live physical slot has private entity")
    }

    /// Resolve a persistent identity to its current checked physical handle.
    pub fn handle(&self, id: StableId) -> Option<Handle> {
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

    /// Spawn one registered game bundle under a new stable identity.
    ///
    /// # Errors
    ///
    /// The ID repeats, capacity is full, or the bundle contains an unregistered
    /// or repeated component. No failure changes committed state.
    pub fn spawn(
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
    pub fn despawn(&mut self, handle: Handle) -> Result<(), RegisteredWorldError> {
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
    pub fn with<T: Send + Sync + 'static, R>(
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

    /// Prepare one two-component storage-order query for repeated ticks.
    ///
    /// # Errors
    ///
    /// Either type is absent from setup registration.
    pub fn read2<T: Send + Sync + 'static, U: Send + Sync + 'static>(
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
}

/// Prepared read of two game-owned component types in backend storage order.
/// The callback receives checked physical and stable identity, never a hecs ID.
pub struct PreparedRead2<T: Send + Sync + 'static, U: Send + Sync + 'static> {
    query: hecs::PreparedQuery<(&'static SlotKey, &'static T, &'static U)>,
}

impl<T: Send + Sync + 'static, U: Send + Sync + 'static> PreparedRead2<T, U> {
    /// Visit matching entities with no per-entity allocation or ID lookup.
    ///
    /// # Panics
    ///
    /// Panics only if the private slot-to-entity invariant has been broken.
    pub fn visit(
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

#[cfg(test)]
mod tests {
    use crate::{StableId, WorldEpoch};

    use super::{ComponentRegistry, IdIndex, RegisteredWorld, RegisteredWorldError};
    use crate::sim::ComponentSchemaId;

    #[derive(Clone, Copy, Debug, PartialEq)]
    struct Position(f64);
    #[derive(Clone, Copy)]
    struct Velocity(f64);
    struct Unregistered;

    fn world() -> RegisteredWorld {
        let mut registry = ComponentRegistry::default();
        registry
            .register::<Position>(ComponentSchemaId(1))
            .expect("position registers");
        registry
            .register::<Velocity>(ComponentSchemaId(2))
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
        assert!(world.is_empty());
        let first = world
            .spawn(StableId(1), |entity| {
                entity.add(Position(2.0))?;
                entity.add(Velocity(3.0))?;
                Ok(())
            })
            .expect("registered bundle spawns");
        assert_eq!(world.handle(StableId(1)), Some(first));
        assert_eq!(
            world.with::<Position, _>(first, |value| *value),
            Ok(Position(2.0))
        );
        assert_eq!(
            world.spawn(StableId(1), |_| Ok(())),
            Err(RegisteredWorldError::DuplicateId)
        );
        let mut query = world
            .read2::<Position, Velocity>()
            .expect("query registered");
        let mut visited = Vec::new();
        query.visit(&world, |handle, id, position, velocity| {
            visited.push((handle, id, position.0, velocity.0));
        });
        assert_eq!(visited, [(first, StableId(1), 2.0, 3.0)]);
        world.despawn(first).expect("first entity live");
        assert_eq!(
            world.with::<Position, _>(first, |value| *value),
            Err(RegisteredWorldError::InvalidHandle)
        );
        let replacement = world
            .spawn(StableId(2), |entity| {
                entity.add(Position(5.0))?;
                entity.add(Velocity(7.0))?;
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
                entity.add(Position(1.0))?;
                entity.add(Position(2.0))?;
                Ok(())
            }),
            Err(RegisteredWorldError::DuplicateComponent)
        );
        assert!(world.is_empty());
        let handle = world
            .spawn(StableId(1), |entity| {
                entity.add(Position(3.0))?;
                Ok(())
            })
            .expect("failed builder was cleared");
        assert_eq!(handle.slot, 0);
        assert_eq!(
            world.with::<Position, _>(handle, |value| *value),
            Ok(Position(3.0))
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
