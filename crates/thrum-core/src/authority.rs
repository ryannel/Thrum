use std::{
    alloc::Layout,
    sync::{Arc, Weak},
};

use crate::replica::{ChargedStateGroup, Replica, StateGroup};
use crate::{
    ClosedPublicationValue, ClosedValueError, ClosedValueLimits, EntityChange, PreparedEdits,
    PublicationBytes, PublicationCapacity, PublicationCharge, PublicationPool,
    PublicationReservation, Snapshot, World, WorldPublicationError, WorldWork,
    publication_pool::PublicationColumn,
    world::{ActorId, AuthorityId, MAX_SNAPSHOT_LEASES, SnapshotLease, SnapshotLeaseFloor},
};

const MAX_PENDING: usize = 8;
pub(crate) const MAX_RESULTS: usize = 256;

struct SnapshotLeaseDirectory {
    slots: [Option<Weak<SnapshotLease>>; MAX_SNAPSHOT_LEASES],
    len: usize,
}

impl SnapshotLeaseDirectory {
    fn new(first: Weak<SnapshotLease>) -> Self {
        let mut slots = std::array::from_fn(|_| None);
        slots[0] = Some(first);
        Self { slots, len: 1 }
    }

    fn retain_live(&mut self) {
        let mut live = 0;
        for index in 0..self.len {
            let lease = self.slots[index].take().expect("occupied lease slot");
            if lease.strong_count() != 0 {
                self.slots[live] = Some(lease);
                live += 1;
            }
        }
        self.len = live;
    }

    fn is_full(&self) -> bool {
        self.len == MAX_SNAPSHOT_LEASES
    }

    fn push(&mut self, lease: Weak<SnapshotLease>) {
        assert!(!self.is_full(), "snapshot lease admission precedes commit");
        self.slots[self.len] = Some(lease);
        self.len += 1;
    }
}

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
    /// A charged peer's bounded pending storage could not be reserved.
    PublicationCapacity,
    /// The required local replica lacks conservative object headroom for a
    /// new action; no input sequence was consumed.
    ReplicaCapacity,
    /// A qualified input carries a payload charged to another pool.
    WrongPool,
    /// A qualified input exceeds its selected per-value shape or byte limit.
    ValueProfile,
}
/// Authority tick cannot publish while required readers retain the bounded pool.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StepError {
    /// This entry path requires a sealed, charged authority configuration.
    Unqualified,
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

/// Read-only charged baseline cannot be produced from this authority state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChargedBaselineError {
    /// The authority has no coherent committed state after a terminal fault.
    Faulted,
    /// The authority did not select the sealed closed-value entry path.
    Unqualified,
    /// The shared pool cannot reserve the complete baseline column owners.
    Capacity,
}

/// A sealed authority cannot activate protected fixed-slot execution credits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionCreditSetupError {
    /// The authority has no closed, charged output profile.
    Unqualified,
    /// Pending inputs already exist without protected credits.
    PendingInputs,
    /// This first credit profile excludes structural growth or large batches.
    UnsupportedEdits,
    /// Tick-created variable payloads need a credit-aware rule context.
    VariablePayloads,
    /// Existing peers exceed the declared maximum.
    PeerCapacity,
    /// Checked layout or aggregate credit arithmetic exceeded a finite bound.
    Capacity,
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

/// Per-value bounds for retained game values on a closed authority path.
/// Temporary change work needs a separate game-work policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClosedGameLimits {
    /// Submitted action payload limits.
    pub action: ClosedValueLimits,
    /// Authoritative entity record limits.
    pub entity: ClosedValueLimits,
    /// Immediate event payload limits.
    pub event: ClosedValueLimits,
    /// Game rejection payload limits.
    pub rejection: ClosedValueLimits,
}

type ClosedValueCheck<T> =
    fn(&T, &PublicationPool, ClosedValueLimits) -> Result<(), ClosedValueError>;

struct ClosedValueGate<T> {
    limits: ClosedValueLimits,
    check: ClosedValueCheck<T>,
}

struct ClosedGameGates<A, T, E, R> {
    action: ClosedValueGate<A>,
    entity: ClosedValueGate<T>,
    event: ClosedValueGate<E>,
    rejection: ClosedValueGate<R>,
}

struct SnapshotAdmission<A, T, E, R> {
    pool: PublicationPool,
    lease_floor: Arc<SnapshotLeaseFloor>,
    bounds: SnapshotWriteBounds,
    closed: Option<ClosedGameGates<A, T, E, R>>,
}

/// A rule tried to create a payload outside its admitted per-action allowance.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RulePayloadError {
    /// This authority has no protected output-payload profile.
    Unqualified,
    /// The cumulative allowance was exceeded, including owner overhead.
    Limit,
}

/// Engine-owned action budget. Callback code only borrows a view of it, so it
/// cannot replace the sticky fault state or drop the reserved credit.
struct RuleBudget {
    payload_credit: Option<PublicationReservation>,
    payload_remaining: usize,
    exceeded: bool,
}

impl RuleBudget {
    fn unqualified() -> Self {
        Self {
            payload_credit: None,
            payload_remaining: 0,
            exceeded: false,
        }
    }

    fn with_payload_credit(credit: PublicationReservation, allowance: usize) -> Self {
        debug_assert_eq!(credit.remaining_bytes(), allowance);
        Self {
            payload_credit: Some(credit),
            payload_remaining: allowance,
            exceeded: false,
        }
    }
}

/// One action's publication-payload allowance, shared by its decision and
/// preparation callbacks. It does not bound arbitrary game-private work.
/// In a protected output profile, every newly created retained rule payload
/// must use this context; pre-existing charged owners may be shared directly.
pub struct RuleContext<'a> {
    budget: &'a mut RuleBudget,
}

impl RuleContext<'_> {
    /// Copy one immutable output from preadmitted credit. Both callbacks spend
    /// the same non-refunding allowance, even if an owner is dropped early.
    pub fn try_copy_payload(
        &mut self,
        source: &[u8],
    ) -> Result<PublicationBytes, RulePayloadError> {
        let Some(credit) = self.budget.payload_credit.as_mut() else {
            return Err(RulePayloadError::Unqualified);
        };
        let required = PublicationBytes::required_charge(source.len()).map_err(|_| {
            self.budget.exceeded = true;
            RulePayloadError::Limit
        })?;
        if required > self.budget.payload_remaining {
            self.budget.exceeded = true;
            return Err(RulePayloadError::Limit);
        }
        self.budget.payload_remaining -= required;
        PublicationBytes::try_copy_from_reservation(credit, source).map_err(|_| {
            self.budget.exceeded = true;
            RulePayloadError::Limit
        })
    }
}

/// Game-owned rule boundary. `decide` and `prepare` are read-only; preparation
/// must be infallible for a valid decision and have no external irreversible
/// side effects. An invalid prepared batch is a terminal game-contract fault.
/// Both local and multiplayer authority paths call these same methods. The
/// protected output profile requires new payloads to use `RuleContext`;
/// allocating them from a captured pool bypasses the per-action liveness
/// contract even though the shared pool still enforces its global limit.
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
        context: &mut RuleContext<'_>,
    ) -> Result<Self::Change, Self::Rejection>;
    /// Build complete edits and an optional event from the current state.
    /// The authority applies them after validation, in canonical input order.
    fn prepare(
        &self,
        world: &World<Self::Entity>,
        change: Self::Change,
        context: &mut RuleContext<'_>,
    ) -> PreparedCommit<Self::Entity, Self::Event>;
}

struct Pending<A> {
    input: Input<A>,
    due_tick: u64,
    timing_error: Option<TimingError>,
    credit: Option<PublicationReservation>,
}

#[derive(Clone, Copy)]
struct FixedCreditProfile {
    per_input_bytes: usize,
    max_peers: usize,
    output_allowance: Option<usize>,
}

struct PendingQueue<A> {
    values: Vec<(u64, Pending<A>)>,
    _charge: Option<PublicationCharge>,
}

struct DueScratch {
    values: Vec<(PeerId, u64)>,
    limit: usize,
    _charge: Option<PublicationCharge>,
}

impl DueScratch {
    fn new() -> Self {
        Self {
            values: Vec::new(),
            limit: 0,
            _charge: None,
        }
    }

    fn try_grow(
        &mut self,
        peer_count: usize,
        pool: Option<&PublicationPool>,
    ) -> Result<(), PublicationCapacity> {
        let limit = peer_count
            .checked_mul(MAX_PENDING)
            .ok_or(PublicationCapacity)?;
        if limit <= self.limit {
            return Ok(());
        }
        assert!(self.values.is_empty(), "peer joins outside a step");
        let bytes = Layout::array::<(PeerId, u64)>(limit)
            .map_err(|_| PublicationCapacity)?
            .size();
        let charge = if let Some(pool) = pool {
            let mut reservation = pool.try_reserve(bytes)?;
            Some(reservation.charge(bytes)?)
        } else {
            None
        };
        let values = Vec::with_capacity(limit);
        self.values = values;
        self._charge = charge;
        self.limit = limit;
        Ok(())
    }

    fn push(&mut self, key: (PeerId, u64)) {
        assert!(self.values.len() < self.limit, "due-key admission cap");
        self.values.push(key);
    }
}

impl<A> PendingQueue<A> {
    fn try_new(pool: Option<&PublicationPool>) -> Result<Self, PublicationCapacity> {
        let Some(pool) = pool else {
            return Ok(Self {
                values: Vec::new(),
                _charge: None,
            });
        };
        let bytes = Layout::array::<(u64, Pending<A>)>(MAX_PENDING)
            .map_err(|_| PublicationCapacity)?
            .size();
        let mut reservation = pool.try_reserve(bytes)?;
        let charge = reservation.charge(bytes)?;
        Ok(Self {
            values: Vec::with_capacity(MAX_PENDING),
            _charge: Some(charge),
        })
    }

    fn get(&self, sequence: &u64) -> Option<&Pending<A>> {
        self.values
            .binary_search_by_key(sequence, |(key, _)| *key)
            .ok()
            .map(|index| &self.values[index].1)
    }

    fn get_mut(&mut self, sequence: &u64) -> Option<&mut Pending<A>> {
        self.values
            .binary_search_by_key(sequence, |(key, _)| *key)
            .ok()
            .map(|index| &mut self.values[index].1)
    }

    fn insert(&mut self, sequence: u64, pending: Pending<A>) {
        assert!(self.values.len() < MAX_PENDING, "pending admission cap");
        if self.values.capacity() == 0 {
            self.values.reserve_exact(MAX_PENDING);
        }
        assert!(
            self.values.capacity() >= MAX_PENDING,
            "pending storage was preallocated"
        );
        assert!(
            self.values.last().is_none_or(|(last, _)| *last < sequence),
            "admitted sequences append in order"
        );
        self.values.push((sequence, pending));
    }

    fn remove(&mut self, sequence: &u64) -> Option<Pending<A>> {
        let index = self
            .values
            .binary_search_by_key(sequence, |(key, _)| *key)
            .ok()?;
        Some(self.values.remove(index).1)
    }

    fn iter(&self) -> impl Iterator<Item = (&u64, &Pending<A>)> {
        self.values
            .iter()
            .map(|(sequence, pending)| (sequence, pending))
    }

    fn len(&self) -> usize {
        self.values.len()
    }

    fn clear(&mut self) {
        self.values.clear();
    }
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
    pending: PendingQueue<A>,
    completed: Vec<Completed<A, R, E>>,
    completed_limit: usize,
    _completed_charge: Option<PublicationCharge>,
    resolved_through: u64,
    out_of_order_resolved: [u64; MAX_RESULTS / 64],
}

impl<A, R, E> Peer<A, R, E> {
    fn new(actor: ActorId, epoch: u64, pending: PendingQueue<A>) -> Self {
        Self {
            actor,
            epoch,
            next_sequence: 1,
            pending,
            completed: Vec::new(),
            completed_limit: 0,
            _completed_charge: None,
            resolved_through: 0,
            out_of_order_resolved: [0; MAX_RESULTS / 64],
        }
    }
    fn try_grow_completed(
        &mut self,
        needed: usize,
        pool: Option<&PublicationPool>,
    ) -> Result<(), PublicationCapacity> {
        if needed <= self.completed_limit {
            return Ok(());
        }
        let Some(pool) = pool else {
            self.completed
                .try_reserve(needed.saturating_sub(self.completed.len()))
                .map_err(|_| PublicationCapacity)?;
            self.completed_limit = self.completed.capacity();
            return Ok(());
        };
        let limit = needed
            .checked_next_power_of_two()
            .ok_or(PublicationCapacity)?;
        if limit > MAX_RESULTS {
            return Err(PublicationCapacity);
        }
        let bytes = Layout::array::<Completed<A, R, E>>(limit)
            .map_err(|_| PublicationCapacity)?
            .size();
        let mut reservation = pool.try_reserve(bytes)?;
        let charge = reservation.charge(bytes)?;
        let mut values = Vec::with_capacity(limit);
        values.append(&mut self.completed);
        let old_values = std::mem::replace(&mut self.completed, values);
        let old_charge = self._completed_charge.replace(charge);
        self.completed_limit = limit;
        drop(old_values);
        drop(old_charge);
        Ok(())
    }
    fn reconnect(&mut self, actor: ActorId, epoch: u64) {
        self.actor = actor;
        self.epoch = epoch;
        self.next_sequence = 1;
        self.pending.clear();
        self.completed.clear();
        self.resolved_through = 0;
        self.out_of_order_resolved = [0; MAX_RESULTS / 64];
    }
    fn mark_resolved(&mut self, sequence: u64) {
        let offset = usize::try_from(
            sequence
                .checked_sub(self.resolved_through + 1)
                .expect("resolved sequence is above watermark"),
        )
        .expect("admitted outcome offset fits usize");
        assert!(
            offset < MAX_RESULTS,
            "admitted outcome exceeds result window"
        );
        if offset == 0 && self.out_of_order_resolved == [0; MAX_RESULTS / 64] {
            self.resolved_through += 1;
            return;
        }
        self.out_of_order_resolved[offset / 64] |= 1_u64 << (offset % 64);
        while self.out_of_order_resolved[0] & 1 != 0 {
            for word in 0..self.out_of_order_resolved.len() - 1 {
                self.out_of_order_resolved[word] = (self.out_of_order_resolved[word] >> 1)
                    | (self.out_of_order_resolved[word + 1] << 63);
            }
            let last = self.out_of_order_resolved.len() - 1;
            self.out_of_order_resolved[last] >>= 1;
            self.resolved_through += 1;
        }
    }
}

struct PeerRegistry<A, R, E> {
    entries: Vec<(PeerId, Peer<A, R, E>)>,
    limit: usize,
    _charge: Option<PublicationCharge>,
}

struct PreparedPeerGrowth<A, R, E> {
    entries: Vec<(PeerId, Peer<A, R, E>)>,
    charge: PublicationCharge,
    limit: usize,
}

impl<A, R, E> PeerRegistry<A, R, E> {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            limit: 0,
            _charge: None,
        }
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn index(&self, id: PeerId) -> Result<usize, usize> {
        self.entries.binary_search_by_key(&id, |(key, _)| *key)
    }

    fn get(&self, id: &PeerId) -> Option<&Peer<A, R, E>> {
        self.index(*id).ok().map(|index| &self.entries[index].1)
    }

    fn get_mut(&mut self, id: &PeerId) -> Option<&mut Peer<A, R, E>> {
        self.index(*id).ok().map(|index| &mut self.entries[index].1)
    }

    fn iter(&self) -> impl Iterator<Item = (&PeerId, &Peer<A, R, E>)> {
        self.entries.iter().map(|(id, peer)| (id, peer))
    }

    fn values(&self) -> impl Iterator<Item = &Peer<A, R, E>> {
        self.entries.iter().map(|(_, peer)| peer)
    }

    fn prepare_growth(
        &self,
        pool: Option<&PublicationPool>,
    ) -> Result<Option<PreparedPeerGrowth<A, R, E>>, PublicationCapacity> {
        let Some(pool) = pool else {
            return Ok(None);
        };
        if self.entries.len() < self.limit {
            return Ok(None);
        }
        let limit = self
            .limit
            .max(1)
            .checked_mul(2)
            .ok_or(PublicationCapacity)?;
        let bytes = Layout::array::<(PeerId, Peer<A, R, E>)>(limit)
            .map_err(|_| PublicationCapacity)?
            .size();
        let mut reservation = pool.try_reserve(bytes)?;
        let charge = reservation.charge(bytes)?;
        Ok(Some(PreparedPeerGrowth {
            entries: Vec::with_capacity(limit),
            charge,
            limit,
        }))
    }

    fn install_growth(&mut self, candidate: Option<PreparedPeerGrowth<A, R, E>>) {
        let Some(mut candidate) = candidate else {
            return;
        };
        candidate.entries.append(&mut self.entries);
        let old_entries = std::mem::replace(&mut self.entries, candidate.entries);
        let old_charge = self._charge.replace(candidate.charge);
        self.limit = candidate.limit;
        drop(old_entries);
        drop(old_charge);
    }

    fn insert(&mut self, id: PeerId, peer: Peer<A, R, E>) {
        assert!(self._charge.is_none() || self.entries.len() < self.limit);
        let index = self.index(id).expect_err("new peer is absent");
        self.entries.insert(index, (id, peer));
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

/// A tick whose group columns keep their charges through the final reader.
pub struct ChargedTickPublication<T, E, R> {
    /// Immutable committed world state.
    pub snapshot: Snapshot<T>,
    /// Full-state group with shared charged columns.
    pub group: ChargedStateGroup<T, E, R>,
    /// Counted world publication work.
    pub world_work: WorldWork,
}

/// The committed tick output of one concrete game rule set.
pub type GameTick<G> =
    TickPublication<<G as GameRules>::Entity, <G as GameRules>::Event, <G as GameRules>::Rejection>;
/// Charged tick output for one concrete game rule set.
pub type ChargedGameTick<G> = ChargedTickPublication<
    <G as GameRules>::Entity,
    <G as GameRules>::Event,
    <G as GameRules>::Rejection,
>;
/// Complete state group for one concrete game rule set.
pub type GameStateGroup<G> =
    StateGroup<<G as GameRules>::Entity, <G as GameRules>::Event, <G as GameRules>::Rejection>;
/// Charged, shared-column state group for one concrete game rule set.
pub type ChargedGameStateGroup<G> = ChargedStateGroup<
    <G as GameRules>::Entity,
    <G as GameRules>::Event,
    <G as GameRules>::Rejection,
>;
type GameClosedGates<G> = ClosedGameGates<
    <G as GameRules>::Action,
    <G as GameRules>::Entity,
    <G as GameRules>::Event,
    <G as GameRules>::Rejection,
>;
type GameSnapshotAdmission<G> = SnapshotAdmission<
    <G as GameRules>::Action,
    <G as GameRules>::Entity,
    <G as GameRules>::Event,
    <G as GameRules>::Rejection,
>;

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
    due_scratch: DueScratch,
    cached_watermarks: Option<PublicationColumn<(PeerId, u64)>>,
    snapshot_leases: SnapshotLeaseDirectory,
    snapshot_admission: Option<GameSnapshotAdmission<G>>,
    fixed_credits: Option<FixedCreditProfile>,
    faulted: bool,
}

type PeerMap<G> =
    PeerRegistry<<G as GameRules>::Action, <G as GameRules>::Rejection, <G as GameRules>::Event>;

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
        world: World<G::Entity>,
        pool: PublicationPool,
        bounds: SnapshotWriteBounds,
    ) -> Result<Self, WorldPublicationError> {
        Self::try_new_snapshot_charged_with_check(id, rules, world, pool, bounds, None)
    }

    /// Start a charged authority whose closed world states are checked against
    /// the selected pool before tick-zero publication and before every edit.
    /// Snapshot pages/roots remain the only pre-edit byte reservation here.
    pub fn try_new_closed_snapshot_charged(
        id: AuthorityId,
        rules: G,
        world: World<G::Entity>,
        pool: PublicationPool,
        bounds: SnapshotWriteBounds,
        value_limits: ClosedGameLimits,
    ) -> Result<Self, WorldPublicationError>
    where
        G::Entity: ClosedPublicationValue,
        G::Action: ClosedPublicationValue,
        G::Event: ClosedPublicationValue,
        G::Rejection: ClosedPublicationValue,
    {
        <G::Action as ClosedPublicationValue>::validate_shape(value_limits.action)
            .map_err(|_| WorldPublicationError::ValueProfile)?;
        <G::Entity as ClosedPublicationValue>::validate_shape(value_limits.entity)
            .map_err(|_| WorldPublicationError::ValueProfile)?;
        <G::Event as ClosedPublicationValue>::validate_shape(value_limits.event)
            .map_err(|_| WorldPublicationError::ValueProfile)?;
        <G::Rejection as ClosedPublicationValue>::validate_shape(value_limits.rejection)
            .map_err(|_| WorldPublicationError::ValueProfile)?;
        Self::try_new_snapshot_charged_with_check(
            id,
            rules,
            world,
            pool,
            bounds,
            Some(ClosedGameGates {
                action: ClosedValueGate {
                    limits: value_limits.action,
                    check: <G::Action as ClosedPublicationValue>::validate_for_pool,
                },
                entity: ClosedValueGate {
                    limits: value_limits.entity,
                    check: <G::Entity as ClosedPublicationValue>::validate_for_pool,
                },
                event: ClosedValueGate {
                    limits: value_limits.event,
                    check: <G::Event as ClosedPublicationValue>::validate_for_pool,
                },
                rejection: ClosedValueGate {
                    limits: value_limits.rejection,
                    check: <G::Rejection as ClosedPublicationValue>::validate_for_pool,
                },
            }),
        )
    }

    fn try_new_snapshot_charged_with_check(
        id: AuthorityId,
        rules: G,
        mut world: World<G::Entity>,
        pool: PublicationPool,
        bounds: SnapshotWriteBounds,
        closed: Option<GameClosedGates<G>>,
    ) -> Result<Self, WorldPublicationError> {
        if let Some(gates) = &closed {
            let gate = &gates.entity;
            world
                .validate_states(|state| (gate.check)(state, &pool, gate.limits))
                .map_err(|error| match error {
                    ClosedValueError::WrongPool => WorldPublicationError::WrongPool,
                    _ => WorldPublicationError::ValueProfile,
                })?;
        }
        let peak = world
            .conservative_charged_publication_peak(0, 0)?
            .checked_add(
                SnapshotLeaseFloor::required_charge()
                    .map_err(|_| WorldPublicationError::Capacity)?,
            )
            .ok_or(WorldPublicationError::Capacity)?;
        let mut reservation = pool
            .try_reserve(peak)
            .map_err(|_| WorldPublicationError::Capacity)?;
        let lease_floor = SnapshotLeaseFloor::try_new(&mut reservation)
            .map_err(|_| WorldPublicationError::Capacity)?;
        let snapshot = world
            .try_publish_charged_with_lease_floor(0, &mut reservation, &lease_floor)?
            .snapshot;
        Ok(Self::from_initial_snapshot(
            id,
            rules,
            world,
            snapshot,
            Some(SnapshotAdmission {
                pool,
                lease_floor,
                bounds,
                closed,
            }),
        ))
    }

    fn from_initial_snapshot(
        id: AuthorityId,
        rules: G,
        world: World<G::Entity>,
        snapshot: Snapshot<G::Entity>,
        snapshot_admission: Option<GameSnapshotAdmission<G>>,
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
            peers: PeerRegistry::new(),
            due_scratch: DueScratch::new(),
            cached_watermarks: None,
            snapshot_leases: SnapshotLeaseDirectory::new(initial_lease),
            snapshot_admission,
            fixed_credits: None,
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

    fn reserve_snapshot_tick(
        &mut self,
        due: usize,
        charged_group: bool,
    ) -> Result<Option<PublicationReservation>, StepError> {
        if self.fixed_credits.is_some() && !charged_group {
            return Err(StepError::Unqualified);
        }
        let Some(admission) = &self.snapshot_admission else {
            return Ok(None);
        };
        let Some(max_edits) = due.checked_mul(admission.bounds.max_edits_per_action) else {
            self.faulted = true;
            return Err(StepError::TerminalGameContract);
        };
        let Some(max_spawns) = due.checked_mul(admission.bounds.max_spawns_per_action) else {
            self.faulted = true;
            return Err(StepError::TerminalGameContract);
        };
        let snapshot_peak = match self
            .world
            .conservative_charged_publication_peak(max_edits, max_spawns)
        {
            Ok(peak) => peak,
            Err(_) => {
                self.faulted = true;
                return Err(StepError::TerminalGameContract);
            }
        };
        let group_peak = if charged_group {
            match self.charged_group_peak(
                due,
                max_edits,
                self.peers.len(),
                due == 0 && self.cached_watermarks.is_some(),
            ) {
                Ok(peak) => peak,
                Err(_) => {
                    self.faulted = true;
                    return Err(StepError::TerminalGameContract);
                }
            }
        } else {
            0
        };
        let Some(peak) = snapshot_peak.checked_add(group_peak) else {
            self.faulted = true;
            return Err(StepError::TerminalGameContract);
        };
        if let Some(profile) = self.fixed_credits.filter(|_| due != 0) {
            let Some(expected_credit) = profile
                .per_input_bytes
                .checked_add(profile.output_allowance.unwrap_or(0))
            else {
                self.faulted = true;
                return Err(StepError::TerminalGameContract);
            };
            let mut protected = 0_usize;
            for (peer_id, sequence) in &self.due_scratch.values {
                let Some(credit) = self
                    .peers
                    .get(peer_id)
                    .and_then(|peer| peer.pending.get(sequence))
                    .and_then(|pending| pending.credit.as_ref())
                else {
                    self.faulted = true;
                    return Err(StepError::TerminalGameContract);
                };
                if credit.remaining_bytes() != expected_credit {
                    self.faulted = true;
                    return Err(StepError::TerminalGameContract);
                }
                let Some(next) = protected.checked_add(profile.per_input_bytes) else {
                    self.faulted = true;
                    return Err(StepError::TerminalGameContract);
                };
                protected = next;
            }
            if protected < peak {
                self.faulted = true;
                return Err(StepError::TerminalGameContract);
            }
            let mut due_reservation = admission
                .pool
                .try_reserve(0)
                .expect("zero-byte reservation");
            for (peer_id, sequence) in &self.due_scratch.values {
                let credit = self
                    .peers
                    .get_mut(peer_id)
                    .and_then(|peer| peer.pending.get_mut(sequence))
                    .and_then(|pending| pending.credit.as_mut())
                    .expect("validated due credit");
                credit
                    .transfer_to(&mut due_reservation, profile.per_input_bytes)
                    .expect("same-pool checked credit transfer");
            }
            return Ok(Some(due_reservation));
        }
        admission
            .pool
            .try_reserve(peak)
            .map(Some)
            .map_err(|_| StepError::PublicationBackpressure)
    }
    /// Current controlled-peer count used by sealed host profile admission.
    pub(crate) fn registered_peer_count(&self) -> usize {
        self.peers.len()
    }

    pub(crate) fn has_peer(&self, id: PeerId) -> bool {
        self.peers.get(&id).is_some()
    }

    fn charged_group_peak(
        &self,
        due: usize,
        max_edits: usize,
        peer_count: usize,
        reuse_watermarks: bool,
    ) -> Result<usize, PublicationCapacity> {
        let changes = self.world.conservative_change_count(max_edits)?;
        let change_bytes = PublicationColumn::<EntityChange<G::Entity>>::required_charge(changes)?;
        let result_bytes = PublicationColumn::<ActionResult<G::Rejection>>::required_charge(due)?;
        let event_bytes = PublicationColumn::<CommittedEvent<G::Event>>::required_charge(due)?;
        let watermark_bytes = if reuse_watermarks {
            0
        } else {
            PublicationColumn::<(PeerId, u64)>::required_charge(peer_count)?
        };
        change_bytes
            .checked_add(result_bytes)
            .and_then(|bytes| bytes.checked_add(event_bytes))
            .and_then(|bytes| bytes.checked_add(watermark_bytes))
            .ok_or(PublicationCapacity)
    }

    /// Activate a narrow fixed-slot, inline-output execution-credit profile.
    /// The host must keep peer membership within `max_peers` after activation.
    pub(crate) fn enable_fixed_inline_credits(
        &mut self,
        max_peers: usize,
    ) -> Result<(), ExecutionCreditSetupError> {
        self.enable_fixed_credits(max_peers, None)
    }

    pub(crate) fn enable_fixed_output_credits(
        &mut self,
        max_peers: usize,
        output_allowance: usize,
    ) -> Result<(), ExecutionCreditSetupError> {
        self.enable_fixed_credits(max_peers, Some(output_allowance))
    }

    fn enable_fixed_credits(
        &mut self,
        max_peers: usize,
        output_allowance: Option<usize>,
    ) -> Result<(), ExecutionCreditSetupError> {
        let admission = self
            .snapshot_admission
            .as_ref()
            .filter(|admission| admission.closed.is_some())
            .ok_or(ExecutionCreditSetupError::Unqualified)?;
        if self.peers.values().any(|peer| peer.pending.len() != 0) {
            return Err(ExecutionCreditSetupError::PendingInputs);
        }
        if self.peers.len() > max_peers {
            return Err(ExecutionCreditSetupError::PeerCapacity);
        }
        if admission.bounds.max_spawns_per_action != 0 || admission.bounds.max_edits_per_action > 2
        {
            return Err(ExecutionCreditSetupError::UnsupportedEdits);
        }
        let closed = admission.closed.as_ref().expect("closed profile checked");
        // Action payloads already own their pool charge before submission;
        // only payloads created by rules after admission need output credit.
        if output_allowance.is_none()
            && [
                closed.entity.limits,
                closed.event.limits,
                closed.rejection.limits,
            ]
            .iter()
            .any(|limits| limits.max_payload_slots != 0 || limits.max_payload_bytes != 0)
        {
            return Err(ExecutionCreditSetupError::VariablePayloads);
        }
        let max_due = max_peers
            .checked_mul(MAX_PENDING)
            .ok_or(ExecutionCreditSetupError::Capacity)?;
        let max_edits = max_due
            .checked_mul(admission.bounds.max_edits_per_action)
            .ok_or(ExecutionCreditSetupError::Capacity)?;
        let one_snapshot = self
            .world
            .conservative_charged_publication_peak(admission.bounds.max_edits_per_action, 0)
            .map_err(|_| ExecutionCreditSetupError::Capacity)?;
        let one_group = self
            .charged_group_peak(1, admission.bounds.max_edits_per_action, max_peers, false)
            .map_err(|_| ExecutionCreditSetupError::Capacity)?;
        let per_input_bytes = one_snapshot
            .checked_add(one_group)
            .ok_or(ExecutionCreditSetupError::Capacity)?;
        let combined_input_bytes = per_input_bytes
            .checked_add(output_allowance.unwrap_or(0))
            .ok_or(ExecutionCreditSetupError::Capacity)?;
        max_due
            .checked_mul(combined_input_bytes)
            .ok_or(ExecutionCreditSetupError::Capacity)?;
        let aggregate_snapshot = self
            .world
            .conservative_charged_publication_peak(max_edits, 0)
            .map_err(|_| ExecutionCreditSetupError::Capacity)?;
        let aggregate_group = self
            .charged_group_peak(max_due, max_edits, max_peers, false)
            .map_err(|_| ExecutionCreditSetupError::Capacity)?;
        let aggregate = aggregate_snapshot
            .checked_add(aggregate_group)
            .ok_or(ExecutionCreditSetupError::Capacity)?;
        if max_due
            .checked_mul(per_input_bytes)
            .is_none_or(|credit| credit < aggregate)
        {
            return Err(ExecutionCreditSetupError::Capacity);
        }
        self.world
            .activate_fixed_slot_journal(&admission.pool, max_edits)
            .map_err(|_| ExecutionCreditSetupError::Capacity)?;
        self.fixed_credits = Some(FixedCreditProfile {
            per_input_bytes,
            max_peers,
            output_allowance,
        });
        Ok(())
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
        if let Some(peer) = self.peers.get_mut(&id) {
            peer.reconnect(actor, epoch);
            self.cached_watermarks = None;
            return Ok(());
        }
        let pending = PendingQueue::try_new(
            self.snapshot_admission
                .as_ref()
                .map(|admission| &admission.pool),
        )
        .map_err(|_| AdmissionError::PublicationCapacity)?;
        let candidate = self
            .peers
            .prepare_growth(
                self.snapshot_admission
                    .as_ref()
                    .map(|admission| &admission.pool),
            )
            .map_err(|_| AdmissionError::PublicationCapacity)?;
        self.due_scratch
            .try_grow(
                self.peers
                    .len()
                    .checked_add(1)
                    .ok_or(AdmissionError::PublicationCapacity)?,
                self.snapshot_admission
                    .as_ref()
                    .map(|admission| &admission.pool),
            )
            .map_err(|_| AdmissionError::PublicationCapacity)?;
        self.peers.install_growth(candidate);
        self.peers.insert(id, Peer::new(actor, epoch, pending));
        self.cached_watermarks = None;
        Ok(())
    }

    /// A sealed host prepares the replacement watermark owner before adding
    /// a new peer. Reconnect requires a fresh replica baseline and is not
    /// admitted by this join path.
    pub(crate) fn register_peer_for_charged_host(
        &mut self,
        id: PeerId,
        epoch: u64,
        actor: ActorId,
    ) -> Result<(), AdmissionError> {
        if self.faulted {
            return Err(AdmissionError::Faulted);
        }
        if self.has_peer(id) {
            return Err(AdmissionError::SequenceConflict);
        }
        let admission = self
            .snapshot_admission
            .as_ref()
            .filter(|admission| admission.closed.is_some())
            .ok_or(AdmissionError::PublicationCapacity)?;
        let count = self
            .peers
            .len()
            .checked_add(1)
            .ok_or(AdmissionError::PublicationCapacity)?;
        if self
            .fixed_credits
            .is_some_and(|profile| count > profile.max_peers)
        {
            return Err(AdmissionError::ReplicaCapacity);
        }
        let bytes = PublicationColumn::<(PeerId, u64)>::required_charge(count)
            .map_err(|_| AdmissionError::PublicationCapacity)?;
        let mut reservation = admission
            .pool
            .try_reserve(bytes)
            .map_err(|_| AdmissionError::PublicationCapacity)?;
        let mut replacement = PublicationColumn::try_with_capacity(&mut reservation, count)
            .map_err(|_| AdmissionError::PublicationCapacity)?;
        let mut inserted = false;
        for (other_id, peer) in self.peers.iter() {
            if !inserted && id < *other_id {
                replacement.push((id, 0));
                inserted = true;
            }
            replacement.push((*other_id, peer.resolved_through));
        }
        if !inserted {
            replacement.push((id, 0));
        }
        self.register_peer(id, epoch, actor)?;
        self.cached_watermarks = Some(replacement);
        Ok(())
    }

    /// A sealed host primes one shared watermark owner before activation, so
    /// its first clean tick needs no new group buffer under pool contention.
    pub(crate) fn prime_charged_watermarks(&mut self) -> Result<(), ChargedBaselineError> {
        if self.cached_watermarks.is_some() {
            return Ok(());
        }
        let admission = self
            .snapshot_admission
            .as_ref()
            .filter(|admission| admission.closed.is_some())
            .ok_or(ChargedBaselineError::Unqualified)?;
        let count = self.peers.len();
        if count == 0 {
            self.cached_watermarks = Some(PublicationColumn::empty());
            return Ok(());
        }
        let bytes = PublicationColumn::<(PeerId, u64)>::required_charge(count)
            .map_err(|_| ChargedBaselineError::Capacity)?;
        let mut reservation = admission
            .pool
            .try_reserve(bytes)
            .map_err(|_| ChargedBaselineError::Capacity)?;
        let mut column = PublicationColumn::try_with_capacity(&mut reservation, count)
            .map_err(|_| ChargedBaselineError::Capacity)?;
        for (id, peer) in self.peers.iter() {
            column.push((*id, peer.resolved_through));
        }
        self.cached_watermarks = Some(column);
        Ok(())
    }
    /// Admit a typed one-shot input; duplicate detection precedes scheduling.
    pub fn submit(
        &mut self,
        input: Input<G::Action>,
    ) -> Result<Admission<G::Rejection>, AdmissionError> {
        self.submit_inner(input, None)
    }

    pub(crate) fn submit_for_replica(
        &mut self,
        input: Input<G::Action>,
        replica: &Replica<G::Entity>,
    ) -> Result<Admission<G::Rejection>, AdmissionError> {
        self.submit_inner(input, Some(replica))
    }

    pub(crate) fn pending_fits_replica(&self, replica: &Replica<G::Entity>) -> bool {
        let Some(admission) = &self.snapshot_admission else {
            return false;
        };
        if admission.bounds.max_spawns_per_action == 0 {
            return replica.can_admit_charged_spawns(0) == Ok(true);
        }
        let Some(pending) = self
            .peers
            .values()
            .try_fold(0_usize, |total, peer| total.checked_add(peer.pending.len()))
        else {
            return false;
        };
        pending
            .checked_mul(admission.bounds.max_spawns_per_action)
            .is_some_and(|spawns| replica.can_admit_charged_spawns(spawns) == Ok(true))
    }

    fn submit_inner(
        &mut self,
        input: Input<G::Action>,
        required_replica: Option<&Replica<G::Entity>>,
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
        if let Some(gates) = self
            .snapshot_admission
            .as_ref()
            .and_then(|admission| admission.closed.as_ref().map(|closed| (admission, closed)))
        {
            let (admission, closed) = gates;
            (closed.action.check)(&input.action, &admission.pool, closed.action.limits).map_err(
                |error| match error {
                    ClosedValueError::WrongPool => AdmissionError::WrongPool,
                    _ => AdmissionError::ValueProfile,
                },
            )?;
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
        if let Some(replica) = required_replica {
            let bound = self
                .snapshot_admission
                .as_ref()
                .ok_or(AdmissionError::ReplicaCapacity)?
                .bounds
                .max_spawns_per_action;
            if bound != 0 {
                let pending = self
                    .peers
                    .values()
                    .try_fold(1_usize, |total, peer| total.checked_add(peer.pending.len()))
                    .ok_or(AdmissionError::ReplicaCapacity)?;
                let max_spawns = pending
                    .checked_mul(bound)
                    .ok_or(AdmissionError::ReplicaCapacity)?;
                if replica.can_admit_charged_spawns(max_spawns) != Ok(true) {
                    return Err(AdmissionError::ReplicaCapacity);
                }
            }
        }
        let credit = self
            .fixed_credits
            .map(|profile| {
                let output_bytes = profile.output_allowance.unwrap_or(0);
                let combined = profile
                    .per_input_bytes
                    .checked_add(output_bytes)
                    .ok_or(AdmissionError::PublicationCapacity)?;
                self.snapshot_admission
                    .as_ref()
                    .expect("protected credits have a pool")
                    .pool
                    .try_reserve(combined)
                    .map_err(|_| AdmissionError::PublicationCapacity)
            })
            .transpose()?;
        let peer = self
            .peers
            .get_mut(&input.origin.peer)
            .expect("validated peer remains registered");
        peer.try_grow_completed(
            peer.completed.len() + peer.pending.len() + 1,
            self.snapshot_admission
                .as_ref()
                .map(|admission| &admission.pool),
        )
        .map_err(|_| AdmissionError::PublicationCapacity)?;
        peer.pending.insert(
            seq,
            Pending {
                input,
                due_tick,
                timing_error,
                credit,
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
    /// Prototype baseline at the current committed tick. Use
    /// `try_baseline_charged` for a protected sealed authority.
    pub fn baseline(&self) -> GameStateGroup<G> {
        self.try_baseline()
            .expect("prototype baseline requires a coherent unprotected authority")
    }
    /// Fallible prototype baseline. Protected sealed authorities return
    /// `Unqualified` so callers cannot bypass charged baseline admission.
    pub fn try_baseline(&self) -> Result<GameStateGroup<G>, StepError> {
        if self.faulted {
            return Err(StepError::TerminalGameContract);
        }
        if self.fixed_credits.is_some() {
            return Err(StepError::Unqualified);
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
    /// Build a read-only baseline with shared charged column owners. The
    /// complete column peak is reserved before copying records or outcomes.
    /// Live tick and replica storage are not admitted by this method.
    pub fn try_baseline_charged(&self) -> Result<ChargedGameStateGroup<G>, ChargedBaselineError>
    where
        G::Action: ClosedPublicationValue,
        G::Entity: ClosedPublicationValue,
        G::Event: ClosedPublicationValue,
        G::Rejection: ClosedPublicationValue,
    {
        if self.faulted {
            return Err(ChargedBaselineError::Faulted);
        }
        let admission = self
            .snapshot_admission
            .as_ref()
            .filter(|admission| admission.closed.is_some())
            .ok_or(ChargedBaselineError::Unqualified)?;
        let change_count = self.snapshot.records().count();
        let mut result_count = 0_usize;
        let mut event_count = 0_usize;
        for peer in self.peers.values() {
            result_count = result_count
                .checked_add(peer.completed.len())
                .ok_or(ChargedBaselineError::Capacity)?;
            for completed in &peer.completed {
                event_count = event_count
                    .checked_add(usize::from(completed.event.is_some()))
                    .ok_or(ChargedBaselineError::Capacity)?;
            }
        }
        let charge = PublicationColumn::<EntityChange<G::Entity>>::required_charge(change_count)
            .and_then(|changes| {
                changes
                    .checked_add(
                        PublicationColumn::<ActionResult<G::Rejection>>::required_charge(
                            result_count,
                        )?,
                    )
                    .ok_or(PublicationCapacity)
            })
            .and_then(|bytes| {
                bytes
                    .checked_add(
                        PublicationColumn::<CommittedEvent<G::Event>>::required_charge(
                            event_count,
                        )?,
                    )
                    .ok_or(PublicationCapacity)
            })
            .and_then(|bytes| {
                bytes
                    .checked_add(if self.cached_watermarks.is_some() {
                        0
                    } else {
                        PublicationColumn::<(PeerId, u64)>::required_charge(self.peers.len())?
                    })
                    .ok_or(PublicationCapacity)
            })
            .map_err(|_| ChargedBaselineError::Capacity)?;
        let mut reservation = admission
            .pool
            .try_reserve(charge)
            .map_err(|_| ChargedBaselineError::Capacity)?;
        let mut changes = PublicationColumn::try_with_capacity(&mut reservation, change_count)
            .map_err(|_| ChargedBaselineError::Capacity)?;
        let mut results = PublicationColumn::try_with_capacity(&mut reservation, result_count)
            .map_err(|_| ChargedBaselineError::Capacity)?;
        let mut events = PublicationColumn::try_with_capacity(&mut reservation, event_count)
            .map_err(|_| ChargedBaselineError::Capacity)?;
        let mut resolved_through = match &self.cached_watermarks {
            Some(owner) => owner.clone(),
            None => PublicationColumn::try_with_capacity(&mut reservation, self.peers.len())
                .map_err(|_| ChargedBaselineError::Capacity)?,
        };
        for record in self.snapshot.records() {
            changes.push(EntityChange::Spawn(record.clone()));
        }
        for peer in self.peers.values() {
            for completed in &peer.completed {
                results.push(completed.result.clone());
                if let Some(event) = &completed.event {
                    events.push(event.clone());
                }
            }
        }
        events.sort_unstable_by_key(|event: &CommittedEvent<G::Event>| event.id);
        if self.cached_watermarks.is_none() {
            for (id, peer) in self.peers.iter() {
                resolved_through.push((*id, peer.resolved_through));
            }
        }
        Ok(ChargedStateGroup {
            pool: admission.pool.clone(),
            authority: self.id,
            group_id: self.group_id,
            commit_tick: self.tick,
            event_cursor: self.next_event_id - 1,
            changes,
            results,
            events,
            resolved_through,
        })
    }
    /// Execute exactly one fixed 1/60-second authority step and publish after all decisions.
    pub fn step(&mut self) -> Result<GameTick<G>, StepError> {
        self.cached_watermarks = None;
        self.step_inner(false, None).map(|(tick, _)| tick)
    }

    /// Publish one sealed tick with shared charged group columns. This admits
    /// the group's requested storage before edits; replica and host storage
    /// still require their own precommit admission.
    pub fn step_charged(&mut self) -> Result<ChargedGameTick<G>, StepError>
    where
        G::Action: ClosedPublicationValue,
        G::Entity: ClosedPublicationValue,
        G::Event: ClosedPublicationValue,
        G::Rejection: ClosedPublicationValue,
    {
        self.step_charged_inner(None)
    }

    /// The required local replica is checked against the declared spawn peak
    /// after the one due-input scan and before any rule edit. This bounds its
    /// already charged object table; progress and host delivery remain open.
    pub(crate) fn step_charged_for_replica(
        &mut self,
        replica: &Replica<G::Entity>,
    ) -> Result<ChargedGameTick<G>, StepError>
    where
        G::Action: ClosedPublicationValue,
        G::Entity: ClosedPublicationValue,
        G::Event: ClosedPublicationValue,
        G::Rejection: ClosedPublicationValue,
    {
        if !replica.ready_for(self.id, self.tick, self.group_id) {
            return Err(StepError::Unqualified);
        }
        self.step_charged_inner(Some(replica))
    }

    fn step_charged_inner(
        &mut self,
        required_replica: Option<&Replica<G::Entity>>,
    ) -> Result<ChargedGameTick<G>, StepError>
    where
        G::Action: ClosedPublicationValue,
        G::Entity: ClosedPublicationValue,
        G::Event: ClosedPublicationValue,
        G::Rejection: ClosedPublicationValue,
    {
        if self
            .snapshot_admission
            .as_ref()
            .is_none_or(|admission| admission.closed.is_none())
        {
            return Err(StepError::Unqualified);
        }
        let (tick, mut reservation) = self.step_inner(true, required_replica)?;
        let reservation = reservation.as_mut().expect("charged group reservation");
        let StateGroup {
            authority,
            group_id,
            commit_tick,
            event_cursor,
            changes,
            results,
            events,
            resolved_through,
        } = tick.group;
        // Neutral structural journal entries can leave spare requested slots
        // in this vector; its retained allocation follows the column owner.
        let changes_limit = changes.capacity();
        let results_limit = results.len();
        // Events requested one slot per due input before any rule callback.
        let events_limit = results.len();
        let watermarks_limit = resolved_through.len();
        let watermarks = if results.is_empty() {
            self.cached_watermarks.clone()
        } else {
            None
        };
        let watermarks = match watermarks {
            Some(watermarks) => watermarks,
            None => PublicationColumn::try_from_reserved_vec(
                reservation,
                resolved_through,
                watermarks_limit,
            )
            .expect("preflight group watermarks"),
        };
        self.cached_watermarks = Some(watermarks.clone());
        let group = ChargedStateGroup {
            pool: reservation.pool(),
            authority,
            group_id,
            commit_tick,
            event_cursor,
            changes: PublicationColumn::try_from_reserved_vec(reservation, changes, changes_limit)
                .expect("preflight group changes"),
            results: PublicationColumn::try_from_reserved_vec(reservation, results, results_limit)
                .expect("preflight group results"),
            events: PublicationColumn::try_from_reserved_vec(reservation, events, events_limit)
                .expect("preflight group events"),
            resolved_through: watermarks,
        };
        self.faulted = false;
        Ok(ChargedTickPublication {
            snapshot: tick.snapshot,
            group,
            world_work: tick.world_work,
        })
    }

    fn step_inner(
        &mut self,
        charged_group: bool,
        required_replica: Option<&Replica<G::Entity>>,
    ) -> Result<(GameTick<G>, Option<PublicationReservation>), StepError> {
        if self.faulted {
            return Err(StepError::TerminalGameContract);
        }
        self.snapshot_leases.retain_live();
        if self.snapshot_leases.is_full() {
            return Err(StepError::SnapshotBackpressure);
        }
        let due_tick = self.tick.checked_add(1).expect("world tick exhausted");
        self.due_scratch.values.clear();
        for (peer_id, peer) in self.peers.iter() {
            for (sequence, pending) in peer.pending.iter() {
                if pending.due_tick <= due_tick {
                    self.due_scratch.push((*peer_id, *sequence));
                }
            }
        }
        if let Some(replica) = required_replica {
            let max_spawns = self.snapshot_admission.as_ref().and_then(|admission| {
                self.due_scratch
                    .values
                    .len()
                    .checked_mul(admission.bounds.max_spawns_per_action)
            });
            let Some(max_spawns) = max_spawns else {
                self.faulted = true;
                self.due_scratch.values.clear();
                return Err(StepError::TerminalGameContract);
            };
            if replica.can_admit_charged_spawns(max_spawns) != Ok(true) {
                // A hosted charged action receives object headroom before its
                // sequence is consumed. Failure here violates that host
                // invariant and must not enter an impossible retry loop.
                self.faulted = true;
                self.due_scratch.values.clear();
                return Err(StepError::TerminalGameContract);
            }
        }
        let mut snapshot_reservation =
            match self.reserve_snapshot_tick(self.due_scratch.values.len(), charged_group) {
                Ok(reservation) => reservation,
                Err(error) => {
                    self.due_scratch.values.clear();
                    return Err(error);
                }
            };
        // Poison before any tick mutation. If game code unwinds and its caller
        // catches the panic, the partially processed tick cannot be replayed.
        self.faulted = true;
        self.tick = self.tick.checked_add(1).expect("world tick exhausted");
        let mut results = Vec::with_capacity(self.due_scratch.values.len());
        let mut events = if charged_group {
            Vec::with_capacity(self.due_scratch.values.len())
        } else {
            Vec::new()
        };
        for index in 0..self.due_scratch.values.len() {
            let (peer_id, sequence) = self.due_scratch.values[index];
            let mut pending = self
                .peers
                .get_mut(&peer_id)
                .expect("admitted peer")
                .pending
                .remove(&sequence)
                .expect("pending key");
            let origin = pending.input.origin;
            let mut rule_budget = if let Some(allowance) = self
                .fixed_credits
                .and_then(|profile| profile.output_allowance)
            {
                let credit = pending
                    .credit
                    .take()
                    .ok_or(StepError::TerminalGameContract)?;
                if credit.remaining_bytes() != allowance {
                    return Err(StepError::TerminalGameContract);
                }
                RuleBudget::with_payload_credit(credit, allowance)
            } else {
                RuleBudget::unqualified()
            };
            let decision = match pending.timing_error {
                Some(TimingError::StaleObservation) => Err(ActionRejection::StaleObservation),
                Some(TimingError::Late) => Err(ActionRejection::Late),
                Some(TimingError::TooEarly) => Err(ActionRejection::TooEarly),
                None => {
                    let mut context = RuleContext {
                        budget: &mut rule_budget,
                    };
                    self.rules
                        .decide(
                            &self.world,
                            pending.input.actor,
                            &pending.input.action,
                            &mut context,
                        )
                        .map_err(ActionRejection::Game)
                }
            };
            if rule_budget.exceeded {
                return Err(StepError::TerminalGameContract);
            }
            if let Err(ActionRejection::Game(reason)) = &decision
                && self.snapshot_admission.as_ref().is_some_and(|admission| {
                    admission.closed.as_ref().is_some_and(|gates| {
                        (gates.rejection.check)(reason, &admission.pool, gates.rejection.limits)
                            .is_err()
                    })
                })
            {
                return Err(StepError::TerminalGameContract);
            }
            let (disposition, event_id, retained_event) = match decision {
                Ok(change) => {
                    let prepared = self.rules.prepare(
                        &self.world,
                        change,
                        &mut RuleContext {
                            budget: &mut rule_budget,
                        },
                    );
                    if rule_budget.exceeded {
                        return Err(StepError::TerminalGameContract);
                    }
                    if self.fixed_credits.is_some() && !prepared.edits.is_inline() {
                        return Err(StepError::TerminalGameContract);
                    }
                    if self.snapshot_admission.as_ref().is_some_and(|admission| {
                        prepared.edits.len() > admission.bounds.max_edits_per_action
                            || prepared.edits.spawns() > admission.bounds.max_spawns_per_action
                            || admission.closed.as_ref().is_some_and(|gates| {
                                prepared
                                    .edits
                                    .validate_states(|state| {
                                        (gates.entity.check)(
                                            state,
                                            &admission.pool,
                                            gates.entity.limits,
                                        )
                                    })
                                    .is_err()
                                    || prepared.event.as_ref().is_some_and(|event| {
                                        (gates.event.check)(
                                            event,
                                            &admission.pool,
                                            gates.event.limits,
                                        )
                                        .is_err()
                                    })
                            })
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
            peer.completed.push(Completed {
                input: pending.input,
                result: result.clone(),
                event: retained_event,
            });
            results.push(result);
        }
        self.due_scratch.values.clear();
        let published = if let Some(reservation) = &mut snapshot_reservation {
            let floor = &self
                .snapshot_admission
                .as_ref()
                .expect("charged snapshot admission")
                .lease_floor;
            self.world
                .try_publish_charged_with_lease_floor(self.tick, reservation, floor)
                .map_err(|_| StepError::TerminalGameContract)?
        } else {
            self.world.publish(self.tick)
        };
        self.snapshot = published.snapshot.clone();
        self.snapshot_leases.push(published.snapshot.lease_token());
        self.group_id = self.group_id.checked_add(1).expect("group ID exhausted");
        let resolved_through =
            if charged_group && results.is_empty() && self.cached_watermarks.is_some() {
                Vec::new()
            } else {
                let mut values = Vec::with_capacity(self.peers.len());
                values.extend(
                    self.peers
                        .iter()
                        .map(|(id, peer)| (*id, peer.resolved_through)),
                );
                values
            };
        let group = StateGroup {
            authority: self.id,
            group_id: self.group_id,
            commit_tick: self.tick,
            event_cursor: self.next_event_id - 1,
            changes: published.changes,
            results,
            events,
            resolved_through,
        };
        let tick = TickPublication {
            snapshot: published.snapshot,
            group,
            world_work: published.work,
        };
        if !charged_group {
            self.faulted = false;
        }
        Ok((tick, snapshot_reservation))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_payload_allowance_counts_zero_length_owners_without_refunding() {
        let owner_bytes = PublicationBytes::required_charge(0).unwrap();
        let pool = PublicationPool::new(owner_bytes * 2);
        let reservation = pool.try_reserve(owner_bytes * 2).unwrap();
        let mut budget = RuleBudget::with_payload_credit(reservation, owner_bytes * 2);
        let occupied = {
            let mut context = RuleContext {
                budget: &mut budget,
            };
            let first = context.try_copy_payload(&[]).unwrap();
            drop(first);
            let occupied = pool.try_reserve(owner_bytes).unwrap();
            let second = context.try_copy_payload(&[]).unwrap();
            assert_eq!(context.try_copy_payload(&[]), Err(RulePayloadError::Limit));
            drop(second);
            occupied
        };
        assert!(budget.exceeded);
        drop(budget);
        drop(occupied);
        assert_eq!(pool.used_bytes(), 0);
        let mut unqualified = RuleBudget::unqualified();
        assert_eq!(
            RuleContext {
                budget: &mut unqualified
            }
            .try_copy_payload(&[]),
            Err(RulePayloadError::Unqualified)
        );
    }

    #[test]
    fn swapping_callback_views_cannot_erase_an_exceeded_budget() {
        let pool = PublicationPool::new(0);
        let mut protected = RuleBudget::with_payload_credit(pool.try_reserve(0).unwrap(), 0);
        let mut ordinary = RuleBudget::unqualified();
        {
            let mut protected_view = RuleContext {
                budget: &mut protected,
            };
            let mut ordinary_view = RuleContext {
                budget: &mut ordinary,
            };
            assert_eq!(
                protected_view.try_copy_payload(&[]),
                Err(RulePayloadError::Limit)
            );
            std::mem::swap(&mut protected_view, &mut ordinary_view);
        }
        assert!(protected.exceeded);
        assert!(!ordinary.exceeded);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn combined_input_credit_refusal_preserves_sequence_for_retry() {
        use crate::{MatchId, PublicationRecord, WorldEpoch};

        struct RejectWithPayload(PublicationPool);
        impl GameRules for RejectWithPayload {
            type Entity = u8;
            type Action = u8;
            type Change = ();
            type Rejection = PublicationRecord<0, 1>;
            type Event = ();

            fn decide(
                &self,
                _world: &World<u8>,
                _actor: ActorId,
                _action: &u8,
                context: &mut RuleContext<'_>,
            ) -> Result<(), Self::Rejection> {
                let payload = context.try_copy_payload(&[1; 8]).unwrap();
                Err(PublicationRecord::try_new(&self.0, [], [Some(payload)]).unwrap())
            }

            fn prepare(
                &self,
                _world: &World<u8>,
                _change: (),
                _context: &mut RuleContext<'_>,
            ) -> PreparedCommit<u8, ()> {
                unreachable!()
            }
        }

        let pool = PublicationPool::new(1_000_000);
        let inline = |bytes| ClosedValueLimits {
            max_inline_bytes: bytes,
            max_payload_slots: 0,
            max_payload_bytes: 0,
        };
        let mut authority = Authority::try_new_closed_snapshot_charged(
            AuthorityId::Match(MatchId(261)),
            RejectWithPayload(pool.clone()),
            World::new(WorldEpoch(261)),
            pool.clone(),
            SnapshotWriteBounds {
                max_edits_per_action: 0,
                max_spawns_per_action: 0,
            },
            ClosedGameLimits {
                action: inline(1),
                entity: inline(1),
                event: inline(0),
                rejection: ClosedValueLimits {
                    max_inline_bytes: 0,
                    max_payload_slots: 1,
                    max_payload_bytes: 8,
                },
            },
        )
        .unwrap();
        authority.register_peer(PeerId(1), 1, ActorId(1)).unwrap();
        let allowance = PublicationBytes::required_charge(8).unwrap();
        authority.enable_fixed_output_credits(1, allowance).unwrap();
        authority
            .peers
            .get_mut(&PeerId(1))
            .unwrap()
            .try_grow_completed(1, Some(&pool))
            .unwrap();
        let snapshot_credit = authority.fixed_credits.unwrap().per_input_bytes;
        let available = snapshot_credit + allowance - 1;
        let before = pool.used_bytes();
        let occupied = pool
            .try_reserve(pool.limit_bytes() - before - available)
            .unwrap();
        let input = Input {
            origin: Origin {
                peer: PeerId(1),
                connection_epoch: 1,
                sequence: 1,
            },
            actor: ActorId(1),
            schedule: InputSchedule::NextTick { observed_tick: 0 },
            action: 1,
        };
        assert_eq!(
            authority.submit(input.clone()),
            Err(AdmissionError::PublicationCapacity)
        );
        assert_eq!(pool.used_bytes(), pool.limit_bytes() - available);
        assert_eq!(authority.peers.get(&PeerId(1)).unwrap().next_sequence, 1);
        drop(occupied);
        assert_eq!(authority.submit(input).unwrap(), Admission::Queued);
        let tick = authority.step_charged().unwrap();
        assert_eq!(tick.group.results().len(), 1);
        drop(tick);
        authority.acknowledge_results(PeerId(1), 1).unwrap();
        drop(authority);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn completed_window_closes_full_out_of_order_gap() {
        let mut peer = Peer::<u8, (), ()>::new(ActorId(1), 1, PendingQueue::try_new(None).unwrap());
        for sequence in 2..=MAX_RESULTS as u64 {
            peer.mark_resolved(sequence);
            assert_eq!(peer.resolved_through, 0);
        }
        peer.mark_resolved(1);
        assert_eq!(peer.resolved_through, MAX_RESULTS as u64);
        assert_eq!(peer.out_of_order_resolved, [0; MAX_RESULTS / 64]);
        peer.mark_resolved(MAX_RESULTS as u64 + 1);
        assert_eq!(peer.resolved_through, MAX_RESULTS as u64 + 1);
    }

    #[test]
    fn completed_buffer_growth_reserves_overlap_and_reconnect_reuses_it() {
        let pool = PublicationPool::new(10_000);
        let mut peer =
            Peer::<u8, (), ()>::new(ActorId(1), 1, PendingQueue::try_new(Some(&pool)).unwrap());
        peer.try_grow_completed(1, Some(&pool)).unwrap();
        let first = pool.used_bytes();
        let occupied = pool.try_reserve(pool.limit_bytes() - first).unwrap();
        assert_eq!(
            peer.try_grow_completed(2, Some(&pool)),
            Err(PublicationCapacity)
        );
        assert_eq!(pool.used_bytes(), pool.limit_bytes());
        drop(occupied);
        peer.try_grow_completed(2, Some(&pool)).unwrap();
        let grown = pool.used_bytes();
        assert!(grown > first);
        peer.reconnect(ActorId(1), 2);
        peer.try_grow_completed(2, Some(&pool)).unwrap();
        assert_eq!(pool.used_bytes(), grown);
        drop(peer);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn charged_peer_registry_grows_with_old_buffer_live_and_keeps_canonical_order() {
        let pool = PublicationPool::new(10_000);
        let mut peers = PeerRegistry::<u8, (), ()>::new();
        let first = peers.prepare_growth(Some(&pool)).unwrap();
        peers.install_growth(first);
        for id in [PeerId(8), PeerId(3)] {
            peers.insert(
                id,
                Peer::new(ActorId(id.0), 1, PendingQueue::try_new(None).unwrap()),
            );
        }
        assert_eq!(
            peers.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            [PeerId(3), PeerId(8)]
        );
        let before = pool.used_bytes();
        let occupied = pool.try_reserve(pool.limit_bytes() - before).unwrap();
        assert!(matches!(
            peers.prepare_growth(Some(&pool)),
            Err(PublicationCapacity)
        ));
        assert_eq!(peers.len(), 2);
        drop(occupied);
        let next = peers.prepare_growth(Some(&pool)).unwrap();
        assert!(pool.used_bytes() > before);
        peers.install_growth(next);
        peers.insert(
            PeerId(5),
            Peer::new(ActorId(5), 1, PendingQueue::try_new(None).unwrap()),
        );
        assert_eq!(
            peers.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
            [PeerId(3), PeerId(5), PeerId(8)]
        );
        assert_eq!(peers.get(&PeerId(5)).unwrap().actor, ActorId(5));
        drop(peers);
        assert_eq!(pool.used_bytes(), 0);
    }
}
