use std::{
    any::{Any, TypeId, type_name},
    collections::HashMap,
    sync::{Arc, Mutex},
};

use thrum_core::{
    Handle, StableId,
    profile::{Digest, StateEncode},
    sim::{Edit, EditWriter, SystemId, TickGame, WorldRead},
};

use crate::{Data, Entity, Event, PhaseClock, Result, Seconds, Tick, data::DataWriter};

pub(crate) trait Value: Send + Sync {
    fn id(&self) -> TypeId;
    fn name(&self) -> &'static str;
    fn any(&self) -> &dyn Any;
    fn any_mut(&mut self) -> &mut dyn Any;
    fn copy_box(&self) -> Box<dyn Value>;
    fn copy_to(&self, target: &mut dyn Value);
    fn encode_value(&self, digest: &mut Digest<'_>);
    fn equals(&self, other: &dyn Value) -> bool;
}

impl<T: Data> Value for T {
    fn id(&self) -> TypeId {
        TypeId::of::<T>()
    }
    fn name(&self) -> &'static str {
        type_name::<T>()
    }
    fn any(&self) -> &dyn Any {
        self
    }
    fn any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn copy_box(&self) -> Box<dyn Value> {
        Box::new(self.clone())
    }
    fn copy_to(&self, target: &mut dyn Value) {
        target
            .any_mut()
            .downcast_mut::<T>()
            .expect("same resource type")
            .clone_from(self);
    }
    fn equals(&self, other: &dyn Value) -> bool {
        other.any().downcast_ref::<T>() == Some(self)
    }
    fn encode_value(&self, digest: &mut Digest<'_>) {
        self.write(&mut DataWriter(digest));
    }
}

impl StateEncode for dyn Value + '_ {
    fn encode(&self, digest: &mut Digest<'_>) {
        self.encode_value(digest);
    }
}

pub(crate) fn check_value(value: &dyn Value) -> Result {
    if value.is_finite() {
        Ok(())
    } else {
        Err(crate::Error::refused(
            "T0308",
            value.name(),
            "the value contains a non-finite Num",
            "Supply a finite world value.",
        ))
    }
}

pub(crate) type ValueRef = Arc<dyn Value>;

#[derive(Default)]
pub(crate) struct ValuePool(HashMap<TypeId, SceneValues>);

#[derive(Default)]
struct SceneValues {
    entries: Vec<ValueRef>,
    next: usize,
}

impl ValuePool {
    pub(crate) fn store<T: Data>(&mut self, value: T) -> ValueRef {
        let pool = self.0.entry(TypeId::of::<T>()).or_default();
        for _ in 0..pool.entries.len() {
            let index = pool.next;
            pool.next = (index + 1) % pool.entries.len();
            let entry = &mut pool.entries[index];
            if let Some(stored) = Arc::get_mut(entry) {
                *stored
                    .any_mut()
                    .downcast_mut::<T>()
                    .expect("typed scene value pool") = value;
                return Arc::clone(entry);
            }
        }
        let value: ValueRef = Arc::new(value);
        pool.entries.push(Arc::clone(&value));
        value
    }
}

struct ResourceSlot {
    value: Box<dyn Value>,
    local: bool,
    active: bool,
}

#[derive(Default)]
pub(crate) struct Resources(Vec<ResourceSlot>);

impl Clone for Resources {
    fn clone(&self) -> Self {
        let mut result = Self::default();
        result.clone_from(self);
        result
    }

    fn clone_from(&mut self, source: &Self) {
        for slot in &mut self.0 {
            slot.active = false;
        }
        for slot in source.active() {
            self.install(slot.value.as_ref(), slot.local);
        }
    }
}

impl Resources {
    fn active(&self) -> impl Iterator<Item = &ResourceSlot> {
        self.0.iter().filter(|slot| slot.active)
    }

    pub(crate) fn get<T: Data>(&self) -> Option<&T> {
        self.active()
            .find_map(|slot| slot.value.any().downcast_ref())
    }

    pub(crate) fn replace<T: Data>(&mut self, value: T) {
        if let Some(current) = self
            .0
            .iter_mut()
            .filter(|slot| slot.active)
            .find_map(|slot| slot.value.any_mut().downcast_mut::<T>())
        {
            *current = value;
        }
    }

    pub(crate) fn install(&mut self, value: &dyn Value, local: bool) {
        if let Some(slot) = self
            .0
            .iter_mut()
            .find(|slot| slot.value.id() == value.id() && slot.local == local)
        {
            value.copy_to(slot.value.as_mut());
            slot.active = true;
        } else {
            self.0.push(ResourceSlot {
                value: value.copy_box(),
                local,
                active: true,
            });
            self.0
                .sort_unstable_by_key(|slot| (slot.value.name(), !slot.local));
        }
    }

    pub(crate) fn types(&self) -> Vec<TypeId> {
        self.active().map(|slot| slot.value.id()).collect()
    }

    pub(crate) fn remove_scene(&mut self) {
        for slot in &mut self.0 {
            slot.active &= !slot.local;
        }
    }

    fn encode(&self, digest: &mut Digest<'_>) {
        (self.active().count() as u64).encode(digest);
        for slot in self.active() {
            (slot.value.name().len() as u64).encode(digest);
            digest.bytes(slot.value.name().as_bytes());
            slot.local.encode(digest);
            slot.value.encode_value(digest);
        }
    }
}

impl PartialEq for Resources {
    fn eq(&self, other: &Self) -> bool {
        let mut other = other.active();
        self.active().all(|slot| {
            other
                .next()
                .is_some_and(|rhs| slot.local == rhs.local && slot.value.equals(rhs.value.as_ref()))
        }) && other.next().is_none()
    }
}

trait EventsBuffer: Send + Sync {
    fn clear(&mut self);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn append(&mut self, other: &mut dyn EventsBuffer);
}

impl<E: Event> EventsBuffer for Vec<E> {
    fn clear(&mut self) {
        self.clear();
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn append(&mut self, other: &mut dyn EventsBuffer) {
        self.append(
            other
                .as_any_mut()
                .downcast_mut::<Self>()
                .expect("same event type"),
        );
    }
}

#[derive(Default)]
pub(crate) struct EventBuffers(Vec<(TypeId, Box<dyn EventsBuffer>)>);

impl EventBuffers {
    pub(crate) fn read<E: Event>(&self) -> &[E] {
        self.0
            .iter()
            .find(|(id, _)| *id == TypeId::of::<E>())
            .and_then(|(_, events)| events.as_any().downcast_ref::<Vec<E>>())
            .map_or(&[], Vec::as_slice)
    }

    pub(crate) fn extend<E: Event>(&mut self, events: impl Iterator<Item = E>) {
        let id = TypeId::of::<E>();
        if let Some((_, buffer)) = self.0.iter_mut().find(|(known, _)| *known == id) {
            buffer
                .as_any_mut()
                .downcast_mut::<Vec<E>>()
                .expect("typed event buffer")
                .extend(events);
        } else {
            self.0.push((id, Box::new(events.collect::<Vec<E>>())));
        }
    }

    pub(crate) fn collect(&mut self, source: &mut Self) {
        let mut index = 0;
        while index < source.0.len() {
            let type_id = source.0[index].0;
            if let Some((_, buffer)) = self.0.iter_mut().find(|(id, _)| *id == type_id) {
                buffer.append(source.0[index].1.as_mut());
                index += 1;
            } else {
                self.0.push(source.0.swap_remove(index));
            }
        }
    }

    pub(crate) fn take<E: Event>(&mut self) -> Vec<E> {
        self.0
            .iter_mut()
            .find(|(id, _)| *id == TypeId::of::<E>())
            .and_then(|(_, buffer)| buffer.as_any_mut().downcast_mut::<Vec<E>>())
            .map_or_else(Vec::new, std::mem::take)
    }

    fn clear(&mut self) {
        for (_, buffer) in &mut self.0 {
            buffer.clear();
        }
    }
}

#[derive(Clone)]
pub(crate) enum Transition {
    Scene(usize, ValueRef),
    Phase(usize),
}

impl PartialEq for Transition {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Scene(index, value), Self::Scene(rhs, other)) => {
                index == rhs && value.equals(other.as_ref())
            }
            (Self::Phase(index), Self::Phase(rhs)) => index == rhs,
            _ => false,
        }
    }
}

impl Transition {
    fn encode(&self, digest: &mut Digest<'_>) {
        match self {
            Self::Scene(index, value) => {
                0_u8.encode(digest);
                (*index as u64).encode(digest);
                value.encode_value(digest);
            }
            Self::Phase(index) => {
                1_u8.encode(digest);
                (*index as u64).encode(digest);
            }
        }
    }
}

#[derive(Clone)]
pub(crate) struct SceneValue(pub(crate) ValueRef);

impl PartialEq for SceneValue {
    fn eq(&self, other: &Self) -> bool {
        self.0.equals(other.0.as_ref())
    }
}

#[derive(Default, PartialEq)]
pub(crate) struct State {
    pub(crate) resources: Resources,
    pub(crate) scene_value: Option<SceneValue>,
    pub(crate) scene: usize,
    pub(crate) scene_entered: u64,
    pub(crate) phase: Option<usize>,
    pub(crate) phase_entered: u64,
    pub(crate) phase_offset: Seconds,
    pub(crate) phase_end: Option<u64>,
    pub(crate) phase_expired: bool,
    pub(crate) held: Option<Transition>,
    pub(crate) next_entity: u128,
    pub(crate) seed: u64,
}

impl Clone for State {
    fn clone(&self) -> Self {
        let mut state = Self::default();
        state.clone_from(self);
        state
    }

    fn clone_from(&mut self, source: &Self) {
        self.resources.clone_from(&source.resources);
        self.scene_value.clone_from(&source.scene_value);
        self.scene = source.scene;
        self.scene_entered = source.scene_entered;
        self.phase = source.phase;
        self.phase_entered = source.phase_entered;
        self.phase_offset = source.phase_offset;
        self.phase_end = source.phase_end;
        self.phase_expired = source.phase_expired;
        self.held.clone_from(&source.held);
        self.next_entity = source.next_entity;
        self.seed = source.seed;
    }
}

impl State {
    pub(crate) fn resource<T: Data>(&self) -> Option<&T> {
        self.scene_value
            .as_ref()
            .and_then(|scene| scene.0.any().downcast_ref())
            .or_else(|| self.resources.get::<T>())
    }

    pub(crate) fn clock(&self, tick: Tick) -> PhaseClock {
        let elapsed = tick.number.0.saturating_sub(self.phase_entered);
        let remaining = self.phase_end.map(|end| end.saturating_sub(tick.number.0));
        PhaseClock {
            elapsed: self.phase_offset + Seconds::from_ticks(elapsed, tick.rate),
            remaining: remaining.map(|left| Seconds::from_ticks(left, tick.rate)),
            expired: self.phase_expired,
        }
    }
}

impl StateEncode for State {
    fn encode(&self, digest: &mut Digest<'_>) {
        (self.scene as u64).encode(digest);
        if let Some(value) = &self.scene_value {
            value.0.encode_value(digest);
        }
        self.scene_entered.encode(digest);
        self.phase
            .map_or(u64::MAX, |phase| phase as u64)
            .encode(digest);
        self.phase_entered.encode(digest);
        self.phase_offset.write(&mut DataWriter(digest));
        self.phase_end.unwrap_or(u64::MAX).encode(digest);
        self.phase_expired.encode(digest);
        self.held.is_some().encode(digest);
        if let Some(request) = &self.held {
            request.encode(digest);
        }
        self.next_entity.encode(digest);
        self.seed.encode(digest);
        self.resources.encode(digest);
    }
}

#[derive(Default)]
pub(crate) struct Requests {
    pub(crate) next: Option<Transition>,
    pub(crate) resume: Option<(usize, PhaseClock)>,
    pub(crate) error: Option<crate::Error>,
}

#[derive(Default)]
pub(crate) struct Candidate {
    pub(crate) attempt: u64,
    pub(crate) state: State,
    pub(crate) events: EventBuffers,
    pub(crate) requests: Arc<Mutex<Requests>>,
    pub(crate) transition: Option<Transition>,
    pub(crate) resumed: bool,
    pub(crate) entities: Vec<(Entity, Handle)>,
    pub(crate) entities_valid: bool,
    pub(crate) changed_entities: Vec<Entity>,
    pub(crate) spawned_entities: Vec<Entity>,
    pub(crate) retired_entities: Vec<Entity>,
    pub(crate) spawns: Vec<Box<dyn crate::entity::SpawnValue>>,
    pub(crate) changes: Vec<Box<dyn crate::entity::QueuedComponent>>,
    pub(crate) values: Arc<Mutex<ValuePool>>,
    pub(crate) edit_pool: Arc<Mutex<crate::entity::EditPool>>,
}

impl Candidate {
    fn recycle_edits(&mut self) {
        if !self.spawns.is_empty() || !self.changes.is_empty() {
            let mut pool = self.edit_pool.lock().expect("structural edit pool");
            pool.births.append(&mut self.spawns);
            pool.components.append(&mut self.changes);
        }
    }
}

pub(crate) enum Operation {
    Tick,
    Start(usize, ValueRef),
    Transition(Transition),
    Manual(crate::rule::Invocation<()>),
}

pub(crate) struct Authority {
    pub(crate) state: State,
    pub(crate) events: EventBuffers,
    pub(crate) operation: Mutex<Operation>,
    pub(crate) program: crate::scene::Program,
}

impl TickGame for Authority {
    type Error = crate::Error;
    type Candidate = Candidate;
    type ParallelOutput = ();

    fn reset_candidate(&self, candidate: &mut Candidate) {
        candidate.attempt = candidate.attempt.wrapping_add(1);
        candidate.state.clone_from(&self.state);
        candidate.events.clear();
        *candidate.requests.lock().expect("requests lock") = Requests {
            next: if matches!(*self.operation.lock().expect("operation"), Operation::Tick) {
                candidate.state.held.take()
            } else {
                None
            },
            ..Requests::default()
        };
        candidate.transition = None;
        candidate.resumed = false;
        candidate.recycle_edits();
        candidate.entities_valid = false;
        candidate.changed_entities.clear();
        candidate.spawned_entities.clear();
        candidate.retired_entities.clear();
    }

    fn profile_resource_encoding<'a>(
        &self,
        candidate: &'a Candidate,
    ) -> Option<&'a dyn StateEncode> {
        Some(&candidate.state)
    }

    fn after_rule_edits(
        &self,
        world: &WorldRead<'_>,
        candidate: &mut Candidate,
        tick: u64,
    ) -> Result {
        if candidate.entities_valid {
            for entity in candidate.retired_entities.drain(..) {
                if let Ok(index) = candidate
                    .entities
                    .binary_search_by_key(&entity, |row| row.0)
                {
                    candidate.entities.remove(index);
                }
            }
            for &entity in &candidate.spawned_entities {
                let handle = world
                    .handle(entity.stable())
                    .expect("new life in tentative world");
                candidate.entities.push((entity, handle));
            }
        }
        candidate.retired_entities.clear();
        candidate.spawned_entities.clear();
        candidate.recycle_edits();
        let result = self.program.validate_queries(
            world,
            candidate.state.scene,
            &candidate.changed_entities,
            tick,
        );
        candidate.changed_entities.clear();
        result
    }

    fn reset_parallel_output(&self, (): &mut ()) {}
    fn merge_parallel(
        &self,
        _: SystemId,
        (): &mut (),
        _: &mut Candidate,
        _: u64,
        _: &mut EditWriter<'_, Candidate>,
    ) {
    }

    fn apply(&mut self, _: &WorldRead<'_>, _: u64, candidate: &Candidate) {
        self.state.clone_from(&candidate.state);
    }
}

pub(crate) fn collect_entities(world: &WorldRead<'_>, entities: &mut Vec<(Entity, Handle)>) {
    entities.clear();
    for slot in 0..world.capacity() {
        if let Some(slot) =
            world.live_at_slot(u32::try_from(slot).expect("world capacity is checked"))
        {
            entities.push((Entity(slot.stable_id().0), slot.handle()));
        }
    }
    entities.sort_unstable_by_key(|&(entity, _)| entity);
}

pub(crate) fn despawn_all(
    permit: &thrum_core::sim::StructuralPermit,
    edits: &mut EditWriter<'_, Candidate>,
    world: &WorldRead<'_>,
) {
    for slot in 0..world.capacity() {
        if let Some(slot) =
            world.live_at_slot(u32::try_from(slot).expect("world capacity is checked"))
        {
            edits.push(Edit::despawn(permit, slot.handle()));
        }
    }
}

pub(crate) fn spawn_edit(
    permit: &thrum_core::sim::StructuralPermit,
    edits: &mut EditWriter<'_, Candidate>,
    candidate: &mut Candidate,
    id: StableId,
    value: Box<dyn crate::entity::SpawnValue>,
) {
    let index = candidate.spawns.len();
    candidate.spawns.push(value);
    edits.push(Edit::spawn(
        permit,
        id,
        index,
        |candidate, index, writer| candidate.spawns[index].fill(writer),
    ));
}

pub(crate) fn operation_result(error: thrum_core::sim::TickError<crate::Error>) -> crate::Error {
    match error {
        thrum_core::sim::TickError::Game(error) => error,
        error => crate::Error::refused(
            "T0308",
            "world transaction",
            error.to_string(),
            "Correct the rule or increase Game::max_entities(..).",
        ),
    }
}
