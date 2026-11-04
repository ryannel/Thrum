//! Temporary fixed-schema adapter over the G2 registered world. The live V1
//! authority uses this one store while game fields and rules move outward.

use crate::{Handle, StableId, WorldEpoch};

use super::{
    ChangeTicks, ComponentEntity, ComponentRegistry, ComponentView, Ownership, Position, SimError,
    Velocity,
    registered_world::{PreparedRead2, RegisteredWorld, SlotKey},
};

#[derive(Clone, Copy)]
pub(super) struct Health(pub(super) u16);
#[derive(Clone, Copy)]
pub(super) struct Visual(u64);

pub(super) struct Store {
    world: RegisteredWorld,
    motion: PreparedRead2<Position, Velocity>,
    pub(super) changes: Vec<ChangeTicks>,
}

impl Store {
    pub(super) fn new(epoch: WorldEpoch, capacity: usize, registry: ComponentRegistry) -> Self {
        let world = RegisteredWorld::new(epoch, capacity, registry)
            .expect("validated fixture capacity fits registered world");
        let motion = world
            .read2::<Position, Velocity>()
            .expect("fixture registers position and velocity");
        Self {
            world,
            motion,
            changes: vec![ChangeTicks::default(); capacity],
        }
    }

    pub(super) fn len(&self) -> usize {
        self.world.len()
    }

    pub(super) fn free_count(&self) -> usize {
        self.world.free_count()
    }

    pub(super) fn reusable_after_despawn(&self, handle: Handle) -> bool {
        self.world.reusable_after_despawn(handle)
    }

    pub(super) fn contains(&self, id: StableId) -> bool {
        self.world.handle(id).is_some()
    }

    pub(super) fn handle(&self, id: StableId) -> Option<Handle> {
        self.world.handle(id)
    }

    pub(super) fn row(&self, handle: Handle) -> Result<usize, SimError> {
        self.world
            .entity(handle)
            .map_err(|_| SimError::InvalidHandle)?;
        Ok(handle.slot as usize)
    }

    pub(super) fn view(&self, row: usize) -> ComponentView<'_> {
        let slot = u32::try_from(row).expect("physical row fits slot");
        ComponentView {
            handle: self.world.handle_at(slot),
            id: self.id_at(row),
            position: self.position(row),
            velocity: self.velocity(row),
            health: self.health(row),
            ownership: self.ownership(row),
            visual: self.visual(row),
            changes: self.changes[row],
            world_borrow: std::marker::PhantomData,
        }
    }

    pub(super) fn visit_components(&self, mut visit: impl FnMut(ComponentView<'_>)) {
        let mut query = self.world.world.query::<(
            &SlotKey,
            &Position,
            Option<&Velocity>,
            &Health,
            Option<&Ownership>,
            Option<&Visual>,
        )>();
        for (key, position, velocity, health, ownership, visual) in &mut query {
            let row = key.0 as usize;
            visit(ComponentView {
                handle: self.world.handle_at(key.0),
                id: self.id_at(row),
                position: *position,
                velocity: velocity.copied().unwrap_or_default(),
                health: health.0,
                ownership: ownership.copied(),
                visual: visual.map(|value| value.0),
                changes: self.changes[row],
                world_borrow: std::marker::PhantomData,
            });
        }
    }

    pub(super) fn visit_changed_since(
        &self,
        since: Option<u64>,
        mut visit: impl FnMut(ComponentView<'_>),
    ) {
        let mut query = self.world.world.query::<(
            &SlotKey,
            &Position,
            Option<&Velocity>,
            &Health,
            Option<&Ownership>,
            Option<&Visual>,
        )>();
        for (key, position, velocity, health, ownership, visual) in &mut query {
            let row = key.0 as usize;
            let changes = self.changes[row];
            if since.is_none_or(|old| {
                changes.position > old
                    || changes.velocity > old
                    || changes.health > old
                    || changes.structural > old
            }) {
                visit(ComponentView {
                    handle: self.world.handle_at(key.0),
                    id: self.id_at(row),
                    position: *position,
                    velocity: velocity.copied().unwrap_or_default(),
                    health: health.0,
                    ownership: ownership.copied(),
                    visual: visual.map(|value| value.0),
                    changes,
                    world_borrow: std::marker::PhantomData,
                });
            }
        }
    }

    pub(super) fn slot_live(&self, slot: usize) -> bool {
        self.world.live_slot(slot)
    }

    pub(super) fn id_at(&self, row: usize) -> StableId {
        self.world.id_at_slot(row)
    }

    fn entity_at(&self, row: usize) -> hecs::Entity {
        self.world.entity_at_slot(row)
    }

    pub(super) fn position(&self, row: usize) -> Position {
        *self
            .world
            .world
            .get::<&Position>(self.entity_at(row))
            .expect("live position")
    }

    pub(super) fn velocity(&self, row: usize) -> Velocity {
        self.world
            .world
            .get::<&Velocity>(self.entity_at(row))
            .map_or(Velocity::default(), |velocity| *velocity)
    }

    pub(super) fn health(&self, row: usize) -> u16 {
        self.world
            .world
            .get::<&Health>(self.entity_at(row))
            .expect("live health")
            .0
    }

    pub(super) fn ownership(&self, row: usize) -> Option<Ownership> {
        self.world
            .world
            .get::<&Ownership>(self.entity_at(row))
            .ok()
            .map(|value| *value)
    }

    pub(super) fn visual(&self, row: usize) -> Option<u64> {
        self.world
            .world
            .get::<&Visual>(self.entity_at(row))
            .ok()
            .map(|value| value.0)
    }

    pub(super) fn set_velocity(&mut self, row: usize, value: Velocity) {
        let entity = self.entity_at(row);
        if value == Velocity::default() && self.world.world.get::<&Ownership>(entity).is_err() {
            if self.world.world.get::<&Velocity>(entity).is_ok() {
                self.world
                    .world
                    .remove_one::<Velocity>(entity)
                    .expect("live mover accepts static transition");
            }
        } else if let Ok(mut velocity) = self.world.world.get::<&mut Velocity>(entity) {
            *velocity = value;
        } else {
            self.world
                .world
                .insert_one(entity, value)
                .expect("live entity accepts velocity transition");
        }
    }

    pub(super) fn set_health(&mut self, row: usize, value: u16) {
        self.world
            .world
            .get::<&mut Health>(self.entity_at(row))
            .expect("live health")
            .0 = value;
    }

    pub(super) fn publish_positions(
        &mut self,
        candidates: &[Position],
        marks: &[u64],
        tick: u64,
    ) -> usize {
        let mut changed = 0;
        for (key, position) in self.world.world.query_mut::<(&SlotKey, &mut Position)>() {
            let row = key.0 as usize;
            if marks[row] == tick && *position != candidates[row] {
                *position = candidates[row];
                self.changes[row].position = tick;
                changed += 1;
            }
        }
        changed
    }

    pub(super) fn for_each_motion(&mut self, mut visit: impl FnMut(usize, Position, Velocity)) {
        self.motion
            .visit(&self.world, |handle, _, position, velocity| {
                visit(handle.slot as usize, *position, *velocity);
            });
    }

    pub(super) fn spawn(&mut self, entity: ComponentEntity, tick: u64) -> Handle {
        let moving = entity.velocity != Velocity::default() || entity.ownership.is_some();
        let handle = self
            .world
            .spawn(entity.id, |writer| {
                writer.add(entity.position)?;
                writer.add(Health(entity.health))?;
                if moving {
                    writer.add(entity.velocity)?;
                }
                if let Some(ownership) = entity.ownership {
                    writer.add(ownership)?;
                }
                if let Some(visual) = entity.visual {
                    writer.add(Visual(visual))?;
                }
                Ok(())
            })
            .expect("prevalidated fixture spawn");
        self.changes[handle.slot as usize] = ChangeTicks {
            position: tick,
            velocity: tick,
            health: tick,
            structural: tick,
        };
        handle
    }

    pub(super) fn despawn(&mut self, handle: Handle) {
        self.world
            .despawn(handle)
            .expect("prevalidated fixture handle");
        self.changes[handle.slot as usize] = ChangeTicks::default();
    }
}
