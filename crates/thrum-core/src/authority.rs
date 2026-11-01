use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::Weak,
};

use crate::replica::StateGroup;
use crate::{
    EntityChange, Snapshot, World, WorldWork,
    world::{ActorId, AuthorityId},
};

const MAX_PENDING: usize = 8;
const MAX_RESULTS: usize = 256;
const MAX_SNAPSHOT_LEASES: usize = 8;

/// One local or remote input origin within an authority.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PeerId(pub u64);
/// Exact origin of an admitted input and its result/event.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Origin {
    /// Controlled connection identity in this authority.
    pub peer: PeerId,
    /// Connection generation; reconnect creates a new value.
    pub connection_epoch: u64,
    /// Monotonic input sequence within the connection epoch.
    pub sequence: u64,
}
/// Selected scheduling intent for an I1 one-shot action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputSchedule {
    /// Execute at the next authority tick; observed tick bounds age.
    NextTick {
        /// Latest authority tick observed by the sender.
        observed_tick: u64,
    },
    /// Execute at the intended authority tick, or next tick if tolerably late.
    Timed {
        /// Intended authority execution tick.
        intended_tick: u64,
    },
}
/// Typed game action and its checked authority envelope.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Input<A> {
    /// Connection-scoped origin.
    pub origin: Origin,
    /// Controlled actor named by the action.
    pub actor: ActorId,
    /// Execution-time policy.
    pub schedule: InputSchedule,
    /// Game-owned typed payload.
    pub action: A,
}
/// Input refusal before a valid sequence is consumed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmissionError {
    /// Peer was not registered.
    UnknownPeer,
    /// Reconnected peer sent an old connection epoch.
    StaleEpoch,
    /// Actor is not controlled by this peer.
    UnauthorizedActor,
    /// Sequence zero, a gap, or a changed duplicate payload.
    SequenceConflict,
    /// Duplicate result is no longer retained.
    Expired,
    /// Bounded pending or retained-result capacity is full.
    Busy,
}
/// Authority tick cannot publish while required readers retain the bounded pool.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StepError {
    /// Eight immutable tick bundles remain leased; caller must release readers
    /// or enter its explicit overload policy before advancing.
    SnapshotBackpressure,
}
/// Submission status; success here is never gameplay commitment.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Admission<R> {
    /// The input awaits its tick decision.
    Queued,
    /// Identical pending duplicate; no second execution was scheduled.
    PendingDuplicate,
    /// Identical resolved duplicate returned its original result.
    ResolvedDuplicate(ActionResult<R>),
}
/// Engine timing or game-owned legality rejection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActionRejection<R> {
    /// `nextTick` observation fell outside the age window.
    StaleObservation,
    /// Intended timed action is over two ticks late.
    Late,
    /// Intended tick exceeds the future admission window.
    TooEarly,
    /// Game-owned rule rejected the authoritative state/action.
    Game(R),
}
/// Committed effect or rejected attempt at an exact authority tick.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResultDisposition<R> {
    /// The game state change committed at the stated tick.
    Committed,
    /// The action did not change game state.
    Rejected(ActionRejection<R>),
}
/// One durable-within-this-connection action decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionResult<R> {
    /// Exact admitted input origin.
    pub origin: Origin,
    /// Tick at which the decision was made.
    pub resolved_tick: u64,
    /// Gameplay disposition.
    pub disposition: ResultDisposition<R>,
    /// Immediate event identity, if the committed rule emitted an event.
    pub event_id: Option<u64>,
}
/// An authority event emitted once by a committed action.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommittedEvent<E> {
    /// Monotonic identity scoped to the authority.
    pub id: u64,
    /// Tick of commitment.
    pub tick: u64,
    /// Action that caused the event.
    pub origin: Origin,
    /// Game-owned event payload.
    pub payload: E,
}

/// Game-owned rule boundary. `decide` is read-only; `commit` must be infallible
/// for a valid decision and must have no external irreversible side effects.
/// A panic after mutation is a terminal world fault, not a rollback.
pub trait GameRules {
    /// Complete I1 object state.
    type Entity: Clone;
    /// Typed action payload.
    type Action: Clone + Eq;
    /// Prepared, validated mutation.
    type Change;
    /// Rejection reason owned by the game.
    type Rejection: Clone + Eq;
    /// Immediate event payload.
    type Event: Clone + Eq;

    /// Validate against the current authoritative world without mutating it.
    fn decide(
        &self,
        world: &World<Self::Entity>,
        actor: ActorId,
        action: &Self::Action,
    ) -> Result<Self::Change, Self::Rejection>;
    /// Apply one valid decision under exclusive authority access.
    fn commit(&self, world: &mut World<Self::Entity>, change: Self::Change) -> Option<Self::Event>;
}

struct Pending<A> {
    input: Input<A>,
    due_tick: u64,
    timing_error: Option<TimingError>,
}
struct Completed<A, R, E> {
    input: Input<A>,
    result: ActionResult<R>,
    event: Option<CommittedEvent<E>>,
}
#[derive(Clone, Copy)]
enum TimingError {
    StaleObservation,
    Late,
    TooEarly,
}

struct Peer<A, R, E> {
    actor: ActorId,
    epoch: u64,
    next_sequence: u64,
    pending: BTreeMap<u64, Pending<A>>,
    completed: VecDeque<Completed<A, R, E>>,
    resolved_through: u64,
    out_of_order_resolved: BTreeSet<u64>,
}

impl<A, R, E> Peer<A, R, E> {
    fn new(actor: ActorId, epoch: u64) -> Self {
        Self {
            actor,
            epoch,
            next_sequence: 1,
            pending: BTreeMap::new(),
            completed: VecDeque::new(),
            resolved_through: 0,
            out_of_order_resolved: BTreeSet::new(),
        }
    }
    fn mark_resolved(&mut self, sequence: u64) {
        self.out_of_order_resolved.insert(sequence);
        while self
            .out_of_order_resolved
            .remove(&(self.resolved_through + 1))
        {
            self.resolved_through += 1;
        }
    }
}

/// One tick's immutable state and in-memory replica group.
pub struct TickPublication<T, E, R> {
    /// Immutable committed world state.
    pub snapshot: Snapshot<T>,
    /// Atomic in-memory group for the replica.
    pub group: StateGroup<T, E, R>,
    /// Counted world publication work.
    pub world_work: WorldWork,
}

/// The committed tick output of one concrete game rule set.
pub type GameTick<G> =
    TickPublication<<G as GameRules>::Entity, <G as GameRules>::Event, <G as GameRules>::Rejection>;

/// Fixed-tick standalone or match authority using one game-owned rule path.
pub struct Authority<G: GameRules> {
    id: AuthorityId,
    rules: G,
    world: World<G::Entity>,
    snapshot: Snapshot<G::Entity>,
    tick: u64,
    group_id: u64,
    next_event_id: u64,
    peers: PeerMap<G>,
    snapshot_leases: VecDeque<Weak<()>>,
}

type PeerMap<G> = BTreeMap<
    PeerId,
    Peer<<G as GameRules>::Action, <G as GameRules>::Rejection, <G as GameRules>::Event>,
>;

impl<G: GameRules> Authority<G> {
    /// Start from a fully preloaded world; publishes its complete tick-zero state.
    pub fn new(id: AuthorityId, rules: G, mut world: World<G::Entity>) -> Self {
        let snapshot = world.publish(0).snapshot;
        let initial_lease = snapshot.lease_token();
        Self {
            id,
            rules,
            world,
            snapshot,
            tick: 0,
            group_id: 0,
            next_event_id: 1,
            peers: BTreeMap::new(),
            snapshot_leases: VecDeque::from([initial_lease]),
        }
    }
    /// Current authority tick.
    pub fn tick_index(&self) -> u64 {
        self.tick
    }
    /// Read current authoritative world without a mutable escape hatch.
    pub fn world(&self) -> &World<G::Entity> {
        &self.world
    }
    /// Register or reconnect one controlled actor. Reconnect requires a newer
    /// connection epoch and loses old pending input.
    pub fn register_peer(
        &mut self,
        id: PeerId,
        epoch: u64,
        actor: ActorId,
    ) -> Result<(), AdmissionError> {
        if self.peers.get(&id).is_some_and(|old| epoch <= old.epoch) {
            return Err(AdmissionError::StaleEpoch);
        }
        self.peers.insert(id, Peer::new(actor, epoch));
        Ok(())
    }
    /// Admit a typed one-shot input; duplicate detection precedes scheduling.
    pub fn submit(
        &mut self,
        input: Input<G::Action>,
    ) -> Result<Admission<G::Rejection>, AdmissionError> {
        let peer = self
            .peers
            .get_mut(&input.origin.peer)
            .ok_or(AdmissionError::UnknownPeer)?;
        if input.origin.connection_epoch != peer.epoch {
            return Err(AdmissionError::StaleEpoch);
        }
        if input.actor != peer.actor {
            return Err(AdmissionError::UnauthorizedActor);
        }
        let seq = input.origin.sequence;
        if seq == 0 || seq == u64::MAX {
            return Err(AdmissionError::SequenceConflict);
        }
        if seq < peer.next_sequence {
            if let Some(old) = peer.pending.get(&seq) {
                return if old.input == input {
                    Ok(Admission::PendingDuplicate)
                } else {
                    Err(AdmissionError::SequenceConflict)
                };
            }
            if let Some(completed) = peer
                .completed
                .iter()
                .find(|completed| completed.input.origin.sequence == seq)
            {
                return if completed.input == input {
                    Ok(Admission::ResolvedDuplicate(completed.result.clone()))
                } else {
                    Err(AdmissionError::SequenceConflict)
                };
            }
            return Err(AdmissionError::Expired);
        }
        if seq != peer.next_sequence {
            return Err(AdmissionError::SequenceConflict);
        }
        if peer.pending.len() >= MAX_PENDING
            || peer.pending.len() + peer.completed.len() >= MAX_RESULTS
        {
            return Err(AdmissionError::Busy);
        }
        let (due_tick, timing_error) = match input.schedule {
            InputSchedule::NextTick { observed_tick } => {
                let invalid = observed_tick.saturating_add(18) < self.tick
                    || observed_tick > self.tick.saturating_add(2);
                (
                    self.tick + 1,
                    invalid.then_some(TimingError::StaleObservation),
                )
            }
            InputSchedule::Timed { intended_tick } => {
                let error = if intended_tick.saturating_add(2) < self.tick {
                    Some(TimingError::Late)
                } else if intended_tick > self.tick.saturating_add(12) {
                    Some(TimingError::TooEarly)
                } else {
                    None
                };
                (
                    if error.is_some() {
                        self.tick + 1
                    } else {
                        intended_tick.max(self.tick + 1)
                    },
                    error,
                )
            }
        };
        peer.pending.insert(
            seq,
            Pending {
                input,
                due_tick,
                timing_error,
            },
        );
        peer.next_sequence = peer
            .next_sequence
            .checked_add(1)
            .expect("input sequence exhausted");
        Ok(Admission::Queued)
    }
    /// Release retained results only after the caller has admitted their group.
    pub fn acknowledge_results(
        &mut self,
        peer_id: PeerId,
        through: u64,
    ) -> Result<(), AdmissionError> {
        let peer = self
            .peers
            .get_mut(&peer_id)
            .ok_or(AdmissionError::UnknownPeer)?;
        if through > peer.resolved_through {
            return Err(AdmissionError::SequenceConflict);
        }
        // Timed inputs can resolve after a higher sequence. Completion order
        // is therefore not sequence order, even when `through` is contiguous.
        peer.completed
            .retain(|completed| completed.result.origin.sequence > through);
        Ok(())
    }
    /// Complete baseline for an in-memory replica at the current committed tick.
    pub fn baseline(&self) -> StateGroup<G::Entity, G::Event, G::Rejection> {
        let mut results = Vec::new();
        let mut events = Vec::new();
        for peer in self.peers.values() {
            for completed in &peer.completed {
                results.push(completed.result.clone());
                if let Some(event) = &completed.event {
                    events.push(event.clone());
                }
            }
        }
        events.sort_by_key(|event| event.id);
        StateGroup {
            authority: self.id,
            group_id: self.group_id,
            commit_tick: self.tick,
            event_cursor: self.next_event_id - 1,
            changes: self
                .snapshot
                .records()
                .cloned()
                .map(EntityChange::Spawn)
                .collect(),
            results,
            events,
            resolved_through: self
                .peers
                .iter()
                .map(|(id, p)| (*id, p.resolved_through))
                .collect(),
        }
    }
    /// Execute exactly one fixed 1/60-second authority step and publish after all decisions.
    pub fn step(&mut self) -> Result<GameTick<G>, StepError> {
        self.snapshot_leases
            .retain(|lease| lease.strong_count() != 0);
        if self.snapshot_leases.len() >= MAX_SNAPSHOT_LEASES {
            return Err(StepError::SnapshotBackpressure);
        }
        self.tick = self.tick.checked_add(1).expect("world tick exhausted");
        let mut due = Vec::new();
        for (peer_id, peer) in &mut self.peers {
            let keys: Vec<_> = peer
                .pending
                .iter()
                .filter(|(_, p)| p.due_tick <= self.tick)
                .map(|(seq, _)| *seq)
                .collect();
            for seq in keys {
                due.push((*peer_id, peer.pending.remove(&seq).expect("pending key")));
            }
        }
        let mut results = Vec::with_capacity(due.len());
        let mut events = Vec::new();
        for (peer_id, pending) in due {
            let origin = pending.input.origin;
            let decision = match pending.timing_error {
                Some(TimingError::StaleObservation) => Err(ActionRejection::StaleObservation),
                Some(TimingError::Late) => Err(ActionRejection::Late),
                Some(TimingError::TooEarly) => Err(ActionRejection::TooEarly),
                None => self
                    .rules
                    .decide(&self.world, pending.input.actor, &pending.input.action)
                    .map_err(ActionRejection::Game),
            };
            let (disposition, event_id, retained_event) = match decision {
                Ok(change) => {
                    let event = self.rules.commit(&mut self.world, change);
                    let event = event.map(|payload| {
                        let id = self.next_event_id;
                        self.next_event_id = self
                            .next_event_id
                            .checked_add(1)
                            .expect("event ID exhausted");
                        CommittedEvent {
                            id,
                            tick: self.tick,
                            origin,
                            payload,
                        }
                    });
                    let event_id = event.as_ref().map(|event| event.id);
                    if let Some(event) = &event {
                        events.push(event.clone());
                    }
                    (ResultDisposition::Committed, event_id, event)
                }
                Err(reason) => (ResultDisposition::Rejected(reason), None, None),
            };
            let result = ActionResult {
                origin,
                resolved_tick: self.tick,
                disposition,
                event_id,
            };
            let peer = self.peers.get_mut(&peer_id).expect("admitted peer");
            peer.mark_resolved(origin.sequence);
            peer.completed.push_back(Completed {
                input: pending.input,
                result: result.clone(),
                event: retained_event,
            });
            results.push(result);
        }
        let published = self.world.publish(self.tick);
        self.snapshot = published.snapshot.clone();
        self.snapshot_leases
            .push_back(published.snapshot.lease_token());
        self.group_id = self.group_id.checked_add(1).expect("group ID exhausted");
        let group = StateGroup {
            authority: self.id,
            group_id: self.group_id,
            commit_tick: self.tick,
            event_cursor: self.next_event_id - 1,
            changes: published.changes,
            results,
            events,
            resolved_through: self
                .peers
                .iter()
                .map(|(id, p)| (*id, p.resolved_through))
                .collect(),
        };
        Ok(TickPublication {
            snapshot: published.snapshot,
            group,
            world_work: published.work,
        })
    }
}
