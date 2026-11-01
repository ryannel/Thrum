use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::Weak,
};

use crate::replica::StateGroup;
use crate::{
    EntityChange, PreparedEdits, PublicationPool, PublicationReservation, Snapshot, World,
    WorldPublicationError, WorldWork,
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
    /// The authority suffered a terminal game-contract fault.
    Faulted,
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
    /// The shared pool cannot reserve the declared snapshot page/root peak;
    /// no tick mutation occurred and the caller may retry after readers drain.
    PublicationBackpressure,
    /// Game preparation violated the world-edit contract. This authority
    /// cannot publish or retry its partially processed tick.
    TerminalGameContract,
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

/// One fully prepared game decision before the authority applies its edits.
pub struct PreparedCommit<T, E> {
    /// Complete ordered world mutations for this decision.
    pub edits: PreparedEdits<T>,
    /// Immediate event payload, emitted only after valid edits are applied.
    pub event: Option<E>,
}

/// Declared per-action edit counts for the partial charged-snapshot path.
/// These bounds do not account for game payloads or replica/output memory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SnapshotWriteBounds {
    /// Maximum ordered world edits produced by one successful action.
    pub max_edits_per_action: usize,
    /// Maximum spawn edits within those ordered edits.
    pub max_spawns_per_action: usize,
}

struct SnapshotAdmission {
    pool: PublicationPool,
    bounds: SnapshotWriteBounds,
}

/// Game-owned rule boundary. `decide` and `prepare` are read-only; preparation
/// must be infallible for a valid decision and have no external irreversible
/// side effects. An invalid prepared batch is a terminal game-contract fault.
pub trait GameRules {
    /// Complete I1 object state.
    type Entity: Clone;
    /// Typed action payload.
    type Action: Clone + Eq;
    /// Read-only decision material used to prepare complete edits.
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
    /// Build complete edits and an optional event from the current state.
    /// The authority applies them after validation, in canonical input order.
    fn prepare(
        &self,
        world: &World<Self::Entity>,
        change: Self::Change,
    ) -> PreparedCommit<Self::Entity, Self::Event>;
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
/// Complete state group for one concrete game rule set.
pub type GameStateGroup<G> =
    StateGroup<<G as GameRules>::Entity, <G as GameRules>::Event, <G as GameRules>::Rejection>;

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
    snapshot_admission: Option<SnapshotAdmission>,
    faulted: bool,
}

type PeerMap<G> = BTreeMap<
    PeerId,
    Peer<<G as GameRules>::Action, <G as GameRules>::Rejection, <G as GameRules>::Event>,
>;

impl<G: GameRules> Authority<G> {
    /// Start from a fully preloaded world; publishes its complete tick-zero state.
    pub fn new(id: AuthorityId, rules: G, mut world: World<G::Entity>) -> Self {
        let snapshot = world.publish(0).snapshot;
        Self::from_initial_snapshot(id, rules, world, snapshot, None)
    }

    /// Start with pool-charged immutable snapshot pages and roots. The pool
    /// guards only snapshot allocations; this is not a whole-process byte cap.
    pub fn try_new_snapshot_charged(
        id: AuthorityId,
        rules: G,
        mut world: World<G::Entity>,
        pool: PublicationPool,
        bounds: SnapshotWriteBounds,
    ) -> Result<Self, WorldPublicationError> {
        let peak = world.conservative_charged_publication_peak(0, 0)?;
        let mut reservation = pool
            .try_reserve(peak)
            .map_err(|_| WorldPublicationError::Capacity)?;
        let snapshot = world.try_publish_charged(0, &mut reservation)?.snapshot;
        Ok(Self::from_initial_snapshot(
            id,
            rules,
            world,
            snapshot,
            Some(SnapshotAdmission { pool, bounds }),
        ))
    }

    fn from_initial_snapshot(
        id: AuthorityId,
        rules: G,
        world: World<G::Entity>,
        snapshot: Snapshot<G::Entity>,
        snapshot_admission: Option<SnapshotAdmission>,
    ) -> Self {
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
            snapshot_admission,
            faulted: false,
        }
    }
    /// Last fully published authority tick. A terminal attempted tick is not
    /// reported as committed.
    pub fn tick_index(&self) -> u64 {
        self.snapshot.tick()
    }
    /// Whether a failed or unwound tick permanently disabled this authority.
    pub fn is_faulted(&self) -> bool {
        self.faulted
    }
    /// Read the world without a mutable escape hatch. After a terminal fault
    /// it may contain unpublished effects and is only suitable for diagnosis.
    pub fn world(&self) -> &World<G::Entity> {
        &self.world
    }
    /// Last complete immutable world view; clones retain its page owners.
    pub fn snapshot(&self) -> &Snapshot<G::Entity> {
        &self.snapshot
    }

    fn reserve_snapshot_tick(&mut self) -> Result<Option<PublicationReservation>, StepError> {
        let Some(admission) = &self.snapshot_admission else {
            return Ok(None);
        };
        let due_tick = self.tick.checked_add(1).expect("world tick exhausted");
        let due = self
            .peers
            .values()
            .flat_map(|peer| peer.pending.values())
            .filter(|pending| pending.due_tick <= due_tick)
            .count();
        let Some(max_edits) = due.checked_mul(admission.bounds.max_edits_per_action) else {
            self.faulted = true;
            return Err(StepError::TerminalGameContract);
        };
        let Some(max_spawns) = due.checked_mul(admission.bounds.max_spawns_per_action) else {
            self.faulted = true;
            return Err(StepError::TerminalGameContract);
        };
        let peak = match self
            .world
            .conservative_charged_publication_peak(max_edits, max_spawns)
        {
            Ok(peak) => peak,
            Err(_) => {
                self.faulted = true;
                return Err(StepError::TerminalGameContract);
            }
        };
        admission
            .pool
            .try_reserve(peak)
            .map(Some)
            .map_err(|_| StepError::PublicationBackpressure)
    }
    /// Register or reconnect one controlled actor. Reconnect requires a newer
    /// connection epoch and loses old pending input.
    pub fn register_peer(
        &mut self,
        id: PeerId,
        epoch: u64,
        actor: ActorId,
    ) -> Result<(), AdmissionError> {
        if self.faulted {
            return Err(AdmissionError::Faulted);
        }
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
        if self.faulted {
            return Err(AdmissionError::Faulted);
        }
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
        if self.faulted {
            return Err(AdmissionError::Faulted);
        }
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
    pub fn baseline(&self) -> GameStateGroup<G> {
        self.try_baseline()
            .expect("faulted authority has no coherent baseline")
    }
    /// Fallible baseline for callers that may inspect a terminal authority.
    pub fn try_baseline(&self) -> Result<GameStateGroup<G>, StepError> {
        if self.faulted {
            return Err(StepError::TerminalGameContract);
        }
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
        Ok(StateGroup {
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
        })
    }
    /// Execute exactly one fixed 1/60-second authority step and publish after all decisions.
    pub fn step(&mut self) -> Result<GameTick<G>, StepError> {
        if self.faulted {
            return Err(StepError::TerminalGameContract);
        }
        self.snapshot_leases
            .retain(|lease| lease.strong_count() != 0);
        if self.snapshot_leases.len() >= MAX_SNAPSHOT_LEASES {
            return Err(StepError::SnapshotBackpressure);
        }
        let mut snapshot_reservation = self.reserve_snapshot_tick()?;
        // Poison before any tick mutation. If game code unwinds and its caller
        // catches the panic, the partially processed tick cannot be replayed.
        self.faulted = true;
        self.tick = self.tick.checked_add(1).expect("world tick exhausted");
        let mut due_keys = Vec::new();
        for (peer_id, peer) in &self.peers {
            due_keys.extend(
                peer.pending
                    .iter()
                    .filter(|(_, p)| p.due_tick <= self.tick)
                    .map(|(seq, _)| (*peer_id, *seq)),
            );
        }
        let mut results = Vec::with_capacity(due_keys.len());
        let mut events = Vec::new();
        for (peer_id, sequence) in due_keys {
            let pending = self
                .peers
                .get_mut(&peer_id)
                .expect("admitted peer")
                .pending
                .remove(&sequence)
                .expect("pending key");
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
                    let prepared = self.rules.prepare(&self.world, change);
                    if self.snapshot_admission.as_ref().is_some_and(|admission| {
                        prepared.edits.len() > admission.bounds.max_edits_per_action
                            || prepared.edits.spawns() > admission.bounds.max_spawns_per_action
                    }) {
                        return Err(StepError::TerminalGameContract);
                    }
                    self.world
                        .apply_edits(prepared.edits)
                        .map_err(|_| StepError::TerminalGameContract)?;
                    let event = prepared.event;
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
        let published = if let Some(reservation) = &mut snapshot_reservation {
            self.world
                .try_publish_charged(self.tick, reservation)
                .map_err(|_| StepError::TerminalGameContract)?
        } else {
            self.world.publish(self.tick)
        };
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
        let tick = TickPublication {
            snapshot: published.snapshot,
            group,
            world_work: published.work,
        };
        self.faulted = false;
        Ok(tick)
    }
}
