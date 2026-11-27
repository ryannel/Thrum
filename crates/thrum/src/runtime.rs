use std::{
    any::{Any, TypeId, type_name},
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
    fn encode_value(&self, digest: &mut Digest<'_>) {
        self.write(&mut DataWriter(digest));
    }
}

pub(crate) type ValueRef = Arc<dyn Value>;

#[derive(Default)]
pub(crate) struct Resources(Vec<(Box<dyn Value>, bool)>);

impl Clone for Resources {
    fn clone(&self) -> Self {
        Self(
            self.0
                .iter()
                .map(|(value, local)| (value.copy_box(), *local))
                .collect(),
        )
    }

    fn clone_from(&mut self, source: &Self) {
        self.0.retain(|(value, scope)| {
            source
                .0
                .iter()
                .any(|(other, local)| other.id() == value.id() && local == scope)
        });
        for (value, local) in &source.0 {
            self.install(value.as_ref(), *local);
        }
    }
}

impl Resources {
    pub(crate) fn get<T: Data>(&self) -> Option<&T> {
        self.0
            .iter()
            .find(|(value, _)| value.id() == TypeId::of::<T>())
            .and_then(|(value, _)| value.any().downcast_ref())
    }

    pub(crate) fn replace<T: Data>(&mut self, value: T) {
        if let Some((current, _)) = self
            .0
            .iter_mut()
            .find(|(current, _)| current.id() == TypeId::of::<T>())
        {
            *current
                .any_mut()
                .downcast_mut::<T>()
                .expect("same resource type") = value;
        }
    }

    pub(crate) fn install(&mut self, value: &dyn Value, local: bool) {
        if let Some((current, scope)) = self
            .0
            .iter_mut()
            .find(|(current, scope)| current.id() == value.id() && *scope == local)
        {
            value.copy_to(current.as_mut());
            *scope = local;
        } else {
            self.0.push((value.copy_box(), local));
            self.0
                .sort_unstable_by_key(|(value, local)| (value.name(), !*local));
        }
    }

    pub(crate) fn types(&self) -> Vec<TypeId> {
        self.0.iter().map(|(value, _)| value.id()).collect()
    }

    pub(crate) fn remove_scene(&mut self) {
        self.0.retain(|(_, local)| !local);
    }

    fn encode(&self, digest: &mut Digest<'_>) {
        (self.0.len() as u64).encode(digest);
        for (value, local) in &self.0 {
            (value.name().len() as u64).encode(digest);
            digest.bytes(value.name().as_bytes());
            local.encode(digest);
            value.encode_value(digest);
        }
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

#[derive(Default)]
pub(crate) struct State {
    pub(crate) resources: Resources,
    pub(crate) scene_value: Option<ValueRef>,
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
    pub(crate) fn clock(&self, tick: Tick) -> PhaseClock {
        let elapsed = tick.number.0.saturating_sub(self.phase_entered);
        let remaining = self.phase_end.map(|end| end.saturating_sub(tick.number.0));
        PhaseClock {
            elapsed: self.phase_offset + Seconds::from_ticks(elapsed, tick.rate),
            remaining: remaining
                .filter(|&left| left > 0)
                .map(|left| Seconds::from_ticks(left, tick.rate)),
            expired: self.phase_expired,
        }
    }
}

impl StateEncode for State {
    fn encode(&self, digest: &mut Digest<'_>) {
        (self.scene as u64).encode(digest);
        if let Some(value) = &self.scene_value {
            value.encode_value(digest);
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
    pub(crate) spawns: Vec<Box<dyn crate::entity::SpawnValue>>,
    pub(crate) changes: Vec<Box<dyn crate::entity::QueuedComponent>>,
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
            next: candidate.state.held.take(),
            ..Requests::default()
        };
        candidate.transition = None;
        candidate.resumed = false;
        candidate.recycle_edits();
        candidate.entities_valid = false;
    }

    fn after_rule_edits(
        &self,
        world: &WorldRead<'_>,
        candidate: &mut Candidate,
        tick: u64,
    ) -> Result {
        collect_entities(world, &mut candidate.entities);
        candidate.entities_valid = true;
        candidate.recycle_edits();
        self.program.validate_queries(world, candidate, tick)
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

pub(crate) fn check_value(value: &impl Data, name: &str) -> Result {
    if crate::data::Stored(value.clone()).is_finite() {
        Ok(())
    } else {
        Err(crate::Error::refused(
            "T0308",
            name,
            "the value contains a non-finite Num",
            "Keep the result of the rule finite.",
        ))
    }
}
