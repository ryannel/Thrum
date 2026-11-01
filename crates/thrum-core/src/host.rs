//! Native/headless callback seam for the bounded I1 authority and replica.
//! A platform event loop calls `on_callback` with monotonic `Instant` values.

use std::{
    alloc::Layout,
    ops::Deref,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::{Arc, Weak},
    time::{Duration, Instant},
};

use crate::{
    Admission, AdmissionError, AdmittedGroup, Authority, ChargedAdmittedGroup,
    ChargedBaselineError, ChargedReplicaSetupError, ClockConfig, ClockError, ClockReport,
    ClockState, ClosedPublicationValue, ExecutionCreditSetupError, GameRules, Input, PeerId,
    PublicationCharge, PublicationPool, Replica, ReplicaError, Snapshot, StepError, TickClock,
    TickPublication, WorldWork, world::MAX_SNAPSHOT_LEASES,
};

struct CallbackBufferFloor {
    _charge: PublicationCharge,
}

impl CallbackBufferFloor {
    fn required_charge(buffer_bytes: usize) -> Result<usize, crate::PublicationCapacity> {
        let arc_header = 2 * size_of::<usize>();
        let token = size_of::<CallbackBufferLease>()
            .checked_add(arc_header)
            .ok_or(crate::PublicationCapacity)?;
        let slot = buffer_bytes
            .checked_add(token)
            .ok_or(crate::PublicationCapacity)?;
        MAX_SNAPSHOT_LEASES
            .checked_mul(slot)
            .and_then(|bytes| bytes.checked_add(size_of::<Self>()))
            .and_then(|bytes| bytes.checked_add(arc_header))
            .ok_or(crate::PublicationCapacity)
    }

    fn try_new(
        pool: &PublicationPool,
        buffer_bytes: usize,
    ) -> Result<Arc<Self>, crate::PublicationCapacity> {
        let bytes = Self::required_charge(buffer_bytes)?;
        let mut reservation = pool.try_reserve(bytes)?;
        let charge = reservation.charge(bytes)?;
        Ok(Arc::new(Self { _charge: charge }))
    }
}

struct CallbackBufferLease {
    _floor: Arc<CallbackBufferFloor>,
}

struct CallbackLeaseDirectory {
    slots: [Option<Weak<CallbackBufferLease>>; MAX_SNAPSHOT_LEASES],
    len: usize,
}

impl CallbackLeaseDirectory {
    fn new() -> Self {
        Self {
            slots: std::array::from_fn(|_| None),
            len: 0,
        }
    }

    fn acquire(
        &mut self,
        floor: &Arc<CallbackBufferFloor>,
    ) -> Result<Arc<CallbackBufferLease>, crate::PublicationCapacity> {
        let mut live = 0;
        for index in 0..self.len {
            let old = self.slots[index].take().expect("callback lease slot");
            if old.strong_count() != 0 {
                self.slots[live] = Some(old);
                live += 1;
            }
        }
        self.len = live;
        if live == MAX_SNAPSHOT_LEASES {
            return Err(crate::PublicationCapacity);
        }
        let lease = Arc::new(CallbackBufferLease {
            _floor: floor.clone(),
        });
        self.slots[live] = Some(Arc::downgrade(&lease));
        self.len += 1;
        Ok(lease)
    }
}

/// CPU admission policy for one host callback. A cost bound is an input from
/// measurement and reserve planning, not a guarantee about actual execution.
#[derive(Clone, Copy, Debug)]
pub struct HostBudget {
    /// CPU time available before this host callback's deadline.
    pub callback_cpu: Duration,
    /// Time reserved for other callback work outside world steps.
    pub other_work: Duration,
    /// Conservative cost admitted for one complete world/replica step.
    pub tick_cost_bound: Duration,
}

impl HostBudget {
    fn valid(self) -> bool {
        !self.callback_cpu.is_zero()
            && !self.tick_cost_bound.is_zero()
            && self.other_work < self.callback_cpu
    }

    fn admitted_ticks(self) -> u32 {
        let count = self.available_step_cpu().as_nanos() / self.tick_cost_bound.as_nanos();
        u32::try_from(count).unwrap_or(u32::MAX)
    }

    fn available_step_cpu(self) -> Duration {
        self.callback_cpu - self.other_work
    }
}

/// Host input or peer-registration admission failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostAdmissionError {
    /// The host is paused, overloaded or terminally faulted.
    Unavailable,
    /// An existing peer needs a new replica baseline before reconnecting.
    RequiresBaseline,
    /// Authority rejected the input envelope.
    Authority(AdmissionError),
}

/// Host callback or setup failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostError {
    /// CPU admission parameters cannot service a tick.
    InvalidBudget,
    /// The selected callback method does not match this host's storage path.
    WrongMode,
    /// Charged baseline could not be published during host construction.
    ChargedBaseline(ChargedBaselineError),
    /// Charged replica could not admit the initial baseline.
    ChargedReplicaSetup(ChargedReplicaSetupError),
    /// The shared pool cannot protect the sealed host's finite callback floor.
    ChargedDeliverySetup(crate::PublicationCapacity),
    /// The requested fixed-slot execution-credit profile is not available.
    ExecutionCreditSetup(ExecutionCreditSetupError),
    /// Host supplied a timestamp before its origin or the clock rejected it.
    Clock(ClockError),
    /// The authority encountered a terminal game-contract fault. The host
    /// cannot expose the partial tick or retry it.
    Authority(StepError),
    /// The local replica rejected a committed authority group; this host is
    /// terminally faulted and may not replay that tick.
    Replica(ReplicaError),
    /// A result acknowledgement was inconsistent with the committed prefix.
    Acknowledgement(AdmissionError),
    /// A previous post-commit fault already stopped this host.
    Faulted,
}

/// One committed tick delivered to the host caller. The local path retains
/// its complete group; the in-memory replica path returns admitted outcomes
/// and the committed snapshot. The caller decides when to acknowledge them.
pub enum HostPublication<T, E, R> {
    /// Authority-only publication, including full state changes and results.
    Local(TickPublication<T, E, R>),
    /// State admitted by the replica, with results/events still observable.
    Replicated {
        /// Immutable authority state at this tick.
        snapshot: Snapshot<T>,
        /// Atomically admitted outcomes and events.
        admitted: AdmittedGroup<E, R>,
        /// Counted world publication work.
        world_work: WorldWork,
    },
    /// A sealed charged group was admitted by the finite local replica.
    ReplicatedCharged {
        /// Immutable authority state at this tick.
        snapshot: Snapshot<T>,
        /// Shared charged outcomes and events.
        admitted: ChargedAdmittedGroup<E, R>,
        /// Counted world publication work.
        world_work: WorldWork,
    },
}

/// Callback publications keep their requested vector storage and one floor
/// permit together, including when the collection is moved out of a report.
pub struct HostPublications<T, E, R> {
    values: Vec<HostPublication<T, E, R>>,
    limit: usize,
    _lease: Option<Arc<CallbackBufferLease>>,
}

impl<T, E, R> HostPublications<T, E, R> {
    fn new() -> Self {
        Self {
            values: Vec::new(),
            limit: usize::MAX,
            _lease: None,
        }
    }

    fn push(&mut self, publication: HostPublication<T, E, R>) {
        assert!(self.values.len() < self.limit, "callback publication bound");
        self.values.push(publication);
    }

    fn is_charged(&self) -> bool {
        self._lease.is_some()
    }

    fn try_reserve_from_floor(
        &mut self,
        lease: Arc<CallbackBufferLease>,
        capacity: usize,
    ) -> Result<(), StepError> {
        assert!(!self.is_charged() && self.values.is_empty());
        self.values
            .try_reserve_exact(capacity)
            .map_err(|_| StepError::PublicationBackpressure)?;
        self._lease = Some(lease);
        self.limit = capacity;
        Ok(())
    }
}

impl<T, E, R> Deref for HostPublications<T, E, R> {
    type Target = [HostPublication<T, E, R>];

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

/// Consuming iterator whose buffer permit retires only after its last owner.
pub struct HostPublicationsIntoIter<T, E, R> {
    values: std::vec::IntoIter<HostPublication<T, E, R>>,
    _lease: Option<Arc<CallbackBufferLease>>,
}

impl<T, E, R> Iterator for HostPublicationsIntoIter<T, E, R> {
    type Item = HostPublication<T, E, R>;

    fn next(&mut self) -> Option<Self::Item> {
        self.values.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.values.size_hint()
    }
}

impl<T, E, R> ExactSizeIterator for HostPublicationsIntoIter<T, E, R> {}

impl<T, E, R> IntoIterator for HostPublications<T, E, R> {
    type Item = HostPublication<T, E, R>;
    type IntoIter = HostPublicationsIntoIter<T, E, R>;

    fn into_iter(self) -> Self::IntoIter {
        HostPublicationsIntoIter {
            values: self.values.into_iter(),
            _lease: self._lease,
        }
    }
}

impl<'a, T, E, R> IntoIterator for &'a HostPublications<T, E, R> {
    type Item = &'a HostPublication<T, E, R>;
    type IntoIter = std::slice::Iter<'a, HostPublication<T, E, R>>;

    fn into_iter(self) -> Self::IntoIter {
        self.values.iter()
    }
}

/// Observed work and delivered publications for one real host callback.
pub struct HostCallback<T, E, R> {
    /// Fixed-step admission, committed work and pending/overload state.
    pub clock: ClockReport<StepError>,
    /// Terminal fault encountered after earlier ticks committed in this
    /// callback. Those earlier publications remain deliverable exactly once.
    pub terminal_fault: Option<HostError>,
    /// Measured wall time spent in this callback, including stage work.
    pub observed_elapsed: Duration,
    /// World-step work exceeded `callback_cpu - other_work` and entered overload.
    pub step_budget_overrun: bool,
    /// Counted world copy work over successfully committed ticks.
    pub world_work: WorldWork,
    /// Full groups admitted by the in-memory replica.
    pub replica_groups: u32,
    /// Committed ticks and their observable outputs, in tick order.
    pub publications: HostPublications<T, E, R>,
}

/// Initial admitted outcomes when a game host starts its replica.
pub type InitialOutcomes<G> = AdmittedGroup<<G as GameRules>::Event, <G as GameRules>::Rejection>;

/// Charged initial outcomes when a sealed host starts its finite replica.
pub type InitialChargedOutcomes<G> =
    ChargedAdmittedGroup<<G as GameRules>::Event, <G as GameRules>::Rejection>;

/// Callback output for one concrete game rule set.
pub type GameHostCallback<G> =
    HostCallback<<G as GameRules>::Entity, <G as GameRules>::Event, <G as GameRules>::Rejection>;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Stage {
    AuthorityStep,
    ReplicaAdmission,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Phase {
    Commit,
    Observe,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Role {
    Authority,
    Replica,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Resource {
    World,
    Group,
    Replica,
}

struct StageSpec {
    stage: Stage,
    phase: Phase,
    role: Role,
    reads: &'static [Resource],
    writes: &'static [Resource],
    after: &'static [Stage],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScheduleError {
    Duplicate,
    TooMany,
    MissingDependency,
    BackwardDependency,
    WrongOwner,
    UnorderedConflict,
}

/// The I1 host has at most two built-in stages. Validation fills this inline
/// order once; callback traversal never acquires scheduler storage.
struct StageOrder {
    values: [Stage; 2],
    len: usize,
}

impl StageOrder {
    fn new() -> Self {
        Self {
            values: [Stage::AuthorityStep; 2],
            len: 0,
        }
    }

    fn contains(&self, stage: Stage) -> bool {
        self.values[..self.len].contains(&stage)
    }

    fn push(&mut self, stage: Stage) {
        self.values[self.len] = stage;
        self.len += 1;
    }

    fn iter(&self) -> impl Iterator<Item = &Stage> {
        self.values[..self.len].iter()
    }
}

/// Validate access once, then retain only the prepared stage order on the hot
/// path. The finite I1 registration uses typed resources rather than an
/// unqualified dynamic schema or a speculative general scheduler.
fn prepare_schedule(specs: &[StageSpec]) -> Result<StageOrder, ScheduleError> {
    if specs.len() > 2 {
        return Err(ScheduleError::TooMany);
    }
    for (index, spec) in specs.iter().enumerate() {
        if specs[..index].iter().any(|prior| prior.stage == spec.stage) {
            return Err(ScheduleError::Duplicate);
        }
        if spec.writes.iter().any(|resource| {
            matches!(
                (spec.role, resource),
                (Role::Authority, Resource::Replica)
                    | (Role::Replica, Resource::World | Resource::Group)
            )
        }) || spec.reads.iter().any(|resource| {
            matches!(
                (spec.role, resource),
                (Role::Authority, Resource::Replica) | (Role::Replica, Resource::World)
            )
        }) {
            return Err(ScheduleError::WrongOwner);
        }
    }
    for spec in specs {
        for dependency in spec.after {
            let Some(prior) = specs
                .iter()
                .find(|candidate| candidate.stage == *dependency)
            else {
                return Err(ScheduleError::MissingDependency);
            };
            if prior.phase > spec.phase || prior.stage == spec.stage {
                return Err(ScheduleError::BackwardDependency);
            }
        }
    }
    for (index, left) in specs.iter().enumerate() {
        for right in &specs[index + 1..] {
            if left.phase != right.phase {
                continue;
            }
            let conflict =
                left.writes.iter().any(|resource| {
                    right.writes.contains(resource) || right.reads.contains(resource)
                }) || right
                    .writes
                    .iter()
                    .any(|resource| left.reads.contains(resource));
            if conflict
                && !ordered_by_dependency(specs, left.stage, right.stage)
                && !ordered_by_dependency(specs, right.stage, left.stage)
            {
                return Err(ScheduleError::UnorderedConflict);
            }
        }
    }
    let mut order = StageOrder::new();
    while order.len < specs.len() {
        let next = specs
            .iter()
            .filter(|spec| !order.contains(spec.stage))
            .filter(|spec| {
                spec.after
                    .iter()
                    .all(|dependency| order.contains(*dependency))
            })
            .filter(|spec| {
                !specs
                    .iter()
                    .any(|other| other.phase < spec.phase && !order.contains(other.stage))
            })
            .min_by_key(|spec| (spec.phase, spec.stage));
        let Some(next) = next else {
            return Err(ScheduleError::BackwardDependency);
        };
        order.push(next.stage);
    }
    Ok(order)
}

fn ordered_by_dependency(specs: &[StageSpec], first: Stage, later: Stage) -> bool {
    fn reaches(specs: &[StageSpec], first: Stage, later: Stage, visited: &mut u8) -> bool {
        let Some(index) = specs.iter().position(|spec| spec.stage == later) else {
            return false;
        };
        let bit = 1_u8 << index;
        if *visited & bit != 0 {
            return false;
        }
        *visited |= bit;
        specs[index]
            .after
            .iter()
            .any(|dependency| *dependency == first || reaches(specs, first, *dependency, visited))
    }
    reaches(specs, first, later, &mut 0)
}

fn headless_schedule(with_replica: bool) -> StageOrder {
    let specs = [
        StageSpec {
            stage: Stage::AuthorityStep,
            phase: Phase::Commit,
            role: Role::Authority,
            reads: &[],
            writes: &[Resource::World, Resource::Group],
            after: &[],
        },
        StageSpec {
            stage: Stage::ReplicaAdmission,
            phase: Phase::Observe,
            role: Role::Replica,
            reads: &[Resource::Group],
            writes: &[Resource::Replica],
            after: &[Stage::AuthorityStep],
        },
    ];
    let selected = if with_replica {
        &specs[..]
    } else {
        &specs[..1]
    };
    prepare_schedule(selected).expect("built-in I1 stage declarations must validate")
}

/// One bounded headless local or in-memory match host. It owns the authority
/// and optionally its confirmed replica; no second rule implementation exists.
pub struct HeadlessHost<G: GameRules> {
    origin: Instant,
    clock: TickClock,
    authority: Authority<G>,
    replica: Option<Replica<G::Entity>>,
    schedule: StageOrder,
    budget: HostBudget,
    admitted_ticks_per_callback: u32,
    callback_publication_capacity: usize,
    callback_publication_bytes: usize,
    callback_leases: CallbackLeaseDirectory,
    callback_floor: Option<Arc<CallbackBufferFloor>>,
    faulted: bool,
}

impl<G: GameRules> HeadlessHost<G> {
    /// Construct a local headless host at the monotonic start instant.
    pub fn local(
        origin: Instant,
        clock: ClockConfig,
        budget: HostBudget,
        authority: Authority<G>,
    ) -> Result<Self, HostError> {
        Self::build(origin, clock, budget, authority, None)
    }

    /// Construct a match host with a complete initial in-memory replica.
    pub fn with_replica(
        origin: Instant,
        clock: ClockConfig,
        budget: HostBudget,
        authority: Authority<G>,
    ) -> Result<(Self, InitialOutcomes<G>), HostError> {
        if authority.is_faulted() {
            return Err(HostError::Authority(StepError::TerminalGameContract));
        }
        let (replica, initial_outcomes) =
            Replica::from_baseline(authority.try_baseline().map_err(HostError::Authority)?)
                .map_err(HostError::Replica)?;
        Ok((
            Self::build(origin, clock, budget, authority, Some(replica))?,
            initial_outcomes,
        ))
    }

    /// Construct a sealed match host with a finite, precharged local replica.
    /// Its peer limit is the baseline peer count. Use the explicit-limit
    /// constructor when this host will register more peers after activation.
    pub fn with_charged_replica(
        origin: Instant,
        clock: ClockConfig,
        budget: HostBudget,
        authority: Authority<G>,
        pool: &PublicationPool,
        max_objects: usize,
    ) -> Result<(Self, InitialChargedOutcomes<G>), HostError>
    where
        G::Action: ClosedPublicationValue,
        G::Entity: ClosedPublicationValue,
        G::Event: ClosedPublicationValue,
        G::Rejection: ClosedPublicationValue,
    {
        let max_peers = authority.registered_peer_count();
        Self::with_charged_replica_with_peer_limit(
            origin,
            clock,
            budget,
            authority,
            pool,
            max_objects,
            max_peers,
        )
    }

    /// Construct a sealed host with precharged room for `max_peers` confirmed
    /// and candidate progress rows. Later joins use `register_peer`.
    pub fn with_charged_replica_with_peer_limit(
        origin: Instant,
        clock: ClockConfig,
        budget: HostBudget,
        mut authority: Authority<G>,
        pool: &PublicationPool,
        max_objects: usize,
        max_peers: usize,
    ) -> Result<(Self, InitialChargedOutcomes<G>), HostError>
    where
        G::Action: ClosedPublicationValue,
        G::Entity: ClosedPublicationValue,
        G::Event: ClosedPublicationValue,
        G::Rejection: ClosedPublicationValue,
    {
        if authority.is_faulted() {
            return Err(HostError::Authority(StepError::TerminalGameContract));
        }
        authority
            .prime_charged_watermarks()
            .map_err(HostError::ChargedBaseline)?;
        let baseline = authority
            .try_baseline_charged()
            .map_err(HostError::ChargedBaseline)?;
        let (replica, initial_outcomes) =
            Replica::from_charged_baseline_with_peer_limit(baseline, pool, max_objects, max_peers)
                .map_err(HostError::ChargedReplicaSetup)?;
        if !authority.pending_fits_replica(&replica) {
            return Err(HostError::ChargedReplicaSetup(
                ChargedReplicaSetupError::Capacity,
            ));
        }
        let mut host = Self::build(origin, clock, budget, authority, Some(replica))?;
        host.callback_floor = Some(
            CallbackBufferFloor::try_new(pool, host.callback_publication_bytes)
                .map_err(HostError::ChargedDeliverySetup)?,
        );
        Ok((host, initial_outcomes))
    }

    /// Construct a sealed fixed-slot host whose accepted inputs protect their
    /// eventual inline snapshot and outcome publication before sequencing.
    pub fn with_protected_inline_replica(
        origin: Instant,
        clock: ClockConfig,
        budget: HostBudget,
        mut authority: Authority<G>,
        pool: &PublicationPool,
        max_objects: usize,
        max_peers: usize,
    ) -> Result<(Self, InitialChargedOutcomes<G>), HostError>
    where
        G::Action: ClosedPublicationValue,
        G::Entity: ClosedPublicationValue,
        G::Event: ClosedPublicationValue,
        G::Rejection: ClosedPublicationValue,
    {
        authority
            .enable_fixed_inline_credits(max_peers)
            .map_err(HostError::ExecutionCreditSetup)?;
        Self::with_charged_replica_with_peer_limit(
            origin,
            clock,
            budget,
            authority,
            pool,
            max_objects,
            max_peers,
        )
    }

    /// Construct a fixed-slot host with a separate cumulative payload
    /// allowance for each accepted action's rule callbacks. Rules must create
    /// new retained payloads through `RuleContext`; direct pool allocation
    /// during a callback violates the accepted-input progress contract.
    pub fn with_protected_output_replica(
        origin: Instant,
        clock: ClockConfig,
        budget: HostBudget,
        mut authority: Authority<G>,
        pool: &PublicationPool,
        max_objects: usize,
        max_peers: usize,
        output_payload_bytes_per_action: usize,
    ) -> Result<(Self, InitialChargedOutcomes<G>), HostError>
    where
        G::Action: ClosedPublicationValue,
        G::Entity: ClosedPublicationValue,
        G::Event: ClosedPublicationValue,
        G::Rejection: ClosedPublicationValue,
    {
        authority
            .enable_fixed_output_credits(max_peers, output_payload_bytes_per_action)
            .map_err(HostError::ExecutionCreditSetup)?;
        Self::with_charged_replica_with_peer_limit(
            origin,
            clock,
            budget,
            authority,
            pool,
            max_objects,
            max_peers,
        )
    }

    /// Construct a sealed structural host with finite physical slots, live
    /// objects and per-action output credit. The game rule classifies each
    /// action's maximum spawns before its input sequence is consumed.
    pub fn with_protected_structural_replica(
        origin: Instant,
        clock: ClockConfig,
        budget: HostBudget,
        mut authority: Authority<G>,
        pool: &PublicationPool,
        max_objects: usize,
        max_peers: usize,
        max_slots: usize,
        max_live: usize,
        output_payload_bytes_per_action: usize,
    ) -> Result<(Self, InitialChargedOutcomes<G>), HostError>
    where
        G::Action: ClosedPublicationValue,
        G::Entity: ClosedPublicationValue,
        G::Event: ClosedPublicationValue,
        G::Rejection: ClosedPublicationValue,
    {
        if max_objects < max_live {
            return Err(HostError::ExecutionCreditSetup(
                ExecutionCreditSetupError::Capacity,
            ));
        }
        authority
            .enable_structural_credits(
                max_peers,
                max_slots,
                max_live,
                Some(output_payload_bytes_per_action),
            )
            .map_err(HostError::ExecutionCreditSetup)?;
        Self::with_charged_replica_with_peer_limit(
            origin,
            clock,
            budget,
            authority,
            pool,
            max_objects,
            max_peers,
        )
    }

    fn build(
        origin: Instant,
        clock: ClockConfig,
        budget: HostBudget,
        authority: Authority<G>,
        replica: Option<Replica<G::Entity>>,
    ) -> Result<Self, HostError> {
        if authority.is_faulted() {
            return Err(HostError::Authority(StepError::TerminalGameContract));
        }
        if !budget.valid() || budget.admitted_ticks() == 0 {
            return Err(HostError::InvalidBudget);
        }
        let admitted_ticks_per_callback = budget.admitted_ticks();
        let callback_publication_capacity = clock
            .max_ticks_per_call
            .get()
            .min(admitted_ticks_per_callback)
            .min(MAX_SNAPSHOT_LEASES as u32) as usize;
        let callback_publication_bytes = Layout::array::<
            HostPublication<G::Entity, G::Event, G::Rejection>,
        >(callback_publication_capacity)
        .map_err(|_| HostError::InvalidBudget)?
        .size();
        let schedule = headless_schedule(replica.is_some());
        Ok(Self {
            origin,
            clock: TickClock::new(clock, Duration::ZERO).map_err(HostError::Clock)?,
            authority,
            replica,
            schedule,
            budget,
            admitted_ticks_per_callback,
            callback_publication_capacity,
            callback_publication_bytes,
            callback_leases: CallbackLeaseDirectory::new(),
            callback_floor: None,
            faulted: false,
        })
    }

    /// Read the committed authority for inspection or baseline creation.
    pub fn authority(&self) -> &Authority<G> {
        &self.authority
    }

    /// Read confirmed state when this host has an in-memory replica.
    pub fn replica(&self) -> Option<&Replica<G::Entity>> {
        self.replica.as_ref()
    }

    /// Register a new controlled peer. A reconnect needs a fresh replica
    /// baseline, so an existing peer is refused on replicated hosts. Sealed
    /// hosts preflight replica headroom and the next idle watermark owner.
    pub fn register_peer(
        &mut self,
        id: PeerId,
        epoch: u64,
        actor: crate::ActorId,
    ) -> Result<(), HostAdmissionError> {
        if self.faulted || self.authority.is_faulted() {
            return Err(HostAdmissionError::Unavailable);
        }
        if self.replica.is_some() && self.authority.has_peer(id) {
            return Err(HostAdmissionError::RequiresBaseline);
        }
        if self.callback_floor.is_some() {
            let replica = self.replica.as_ref().expect("sealed host has replica");
            let next_count = self.authority.registered_peer_count().checked_add(1);
            if !next_count.is_some_and(|count| replica.can_admit_charged_peer_count(count)) {
                return Err(HostAdmissionError::Authority(
                    AdmissionError::ReplicaCapacity,
                ));
            }
            self.authority
                .register_peer_for_charged_host(id, epoch, actor)
        } else {
            self.authority.register_peer(id, epoch, actor)
        }
        .map_err(HostAdmissionError::Authority)
    }

    /// Current local clock state.
    pub fn clock_state(&self) -> ClockState {
        self.clock.state()
    }

    /// Admit input only while this host is active.
    pub fn submit(
        &mut self,
        input: Input<G::Action>,
    ) -> Result<Admission<G::Rejection>, HostAdmissionError> {
        if self.faulted || self.authority.is_faulted() || self.clock.state() != ClockState::Active {
            return Err(HostAdmissionError::Unavailable);
        }
        match &self.replica {
            Some(replica) if replica.charged_object_bytes().is_some() => {
                self.authority.submit_for_replica(input, replica)
            }
            _ => self.authority.submit(input),
        }
        .map_err(HostAdmissionError::Authority)
    }

    /// Retire results only after the caller has received and handled their
    /// group. This works for both local and replicated host paths.
    pub fn acknowledge_results(&mut self, peer: PeerId, through: u64) -> Result<(), HostError> {
        if self.faulted || self.authority.is_faulted() {
            return Err(HostError::Faulted);
        }
        self.authority
            .acknowledge_results(peer, through)
            .map_err(HostError::Acknowledgement)
    }

    /// Stop accumulating local world time. Already admitted work may drain.
    pub fn pause(&mut self) {
        self.clock.pause();
    }

    /// Resume after a pause or overload from a new monotonic origin.
    pub fn resume(&mut self, now: Instant) -> Result<(), HostError> {
        if self.faulted || self.authority.is_faulted() {
            return Err(HostError::Faulted);
        }
        let elapsed = now
            .checked_duration_since(self.origin)
            .ok_or(HostError::Clock(ClockError::NonMonotonicTime))?;
        self.clock.resume(elapsed).map_err(HostError::Clock)
    }

    /// Service one actual platform callback. A callback that exceeds the CPU
    /// budget visibly overloads this local host after its committed ticks;
    /// it cannot retroactively preempt a slow game rule.
    pub fn on_callback(&mut self, now: Instant) -> Result<GameHostCallback<G>, HostError> {
        if self
            .replica
            .as_ref()
            .is_some_and(|replica| replica.charged_object_bytes().is_some())
        {
            return Err(HostError::WrongMode);
        }
        if self.faulted || self.authority.is_faulted() {
            return Err(HostError::Faulted);
        }
        let elapsed = now
            .checked_duration_since(self.origin)
            .ok_or(HostError::Clock(ClockError::NonMonotonicTime))?;
        let started = Instant::now();
        let mut work = WorldWork::default();
        let mut replica_groups = 0;
        let mut publications = HostPublications::new();
        let mut fatal = None;
        let authority = &mut self.authority;
        let replica = &mut self.replica;
        let schedule = &self.schedule;
        let step_cpu_budget = self.budget.available_step_cpu();
        let mut report = self
            .clock
            .advance_controlled(elapsed, self.admitted_ticks_per_callback, || {
                let mut publication = None;
                for stage in schedule.iter() {
                    match stage {
                        Stage::AuthorityStep => {
                            let committed = catch_unwind(AssertUnwindSafe(|| authority.step()))
                                .map_err(|_| StepError::TerminalGameContract)??;
                            work.page_references += committed.world_work.page_references;
                            work.copied_pages += committed.world_work.copied_pages;
                            work.copied_objects += committed.world_work.copied_objects;
                            work.group_objects += committed.world_work.group_objects;
                            publication = Some(committed);
                        }
                        Stage::ReplicaAdmission => {
                            let committed = publication.take().expect("validated producer order");
                            let Some(replica) = replica else {
                                unreachable!("replica stage requires a replica")
                            };
                            match replica.apply(committed.group) {
                                Ok(admitted) => {
                                    replica_groups += 1;
                                    publications.push(HostPublication::Replicated {
                                        snapshot: committed.snapshot,
                                        admitted,
                                        world_work: committed.world_work,
                                    });
                                }
                                Err(error) => fatal = Some(HostError::Replica(error)),
                            }
                        }
                    }
                    if fatal.is_some() {
                        break;
                    }
                }
                if let Some(committed) = publication {
                    publications.push(HostPublication::Local(committed));
                }
                // A bound based on prior measurement is only an admission
                // estimate. Stop catch-up after an actual slow committed tick.
                Ok(fatal.is_none() && started.elapsed() < step_cpu_budget)
            })
            .map_err(HostError::Clock)?;
        let observed_elapsed = started.elapsed();
        let terminal_fault = if report.step_error == Some(StepError::TerminalGameContract) {
            Some(HostError::Authority(StepError::TerminalGameContract))
        } else {
            fatal
        };
        if let Some(fault) = terminal_fault {
            self.faulted = true;
            if publications.is_empty() {
                return Err(fault);
            }
        }
        let step_budget_overrun = observed_elapsed > step_cpu_budget;
        if step_budget_overrun && terminal_fault.is_none() {
            self.clock.report_cpu_overload();
            report.state = ClockState::Overloaded;
        }
        Ok(HostCallback {
            clock: report,
            terminal_fault,
            observed_elapsed,
            step_budget_overrun,
            world_work: work,
            replica_groups,
            publications,
        })
    }

    /// Service the same validated authority → replica stages with sealed,
    /// shared charged groups. A finite precharged callback buffer permit is
    /// acquired before the first step; the authority checks required replica
    /// headroom before rule edits. Replica progress scratch remains uncharged.
    pub fn on_callback_charged(&mut self, now: Instant) -> Result<GameHostCallback<G>, HostError>
    where
        G::Action: ClosedPublicationValue,
        G::Entity: ClosedPublicationValue,
        G::Event: ClosedPublicationValue,
        G::Rejection: ClosedPublicationValue,
    {
        if self
            .replica
            .as_ref()
            .is_none_or(|replica| replica.charged_object_bytes().is_none())
        {
            return Err(HostError::WrongMode);
        }
        if self.faulted || self.authority.is_faulted() {
            return Err(HostError::Faulted);
        }
        let elapsed = now
            .checked_duration_since(self.origin)
            .ok_or(HostError::Clock(ClockError::NonMonotonicTime))?;
        let started = Instant::now();
        let mut work = WorldWork::default();
        let mut replica_groups = 0;
        let mut publications = HostPublications::new();
        let mut fatal = None;
        let authority = &mut self.authority;
        let replica = self.replica.as_mut().expect("charged replica mode");
        let callback_floor = self.callback_floor.as_ref().expect("charged host floor");
        let callback_leases = &mut self.callback_leases;
        let schedule = &self.schedule;
        let step_cpu_budget = self.budget.available_step_cpu();
        let callback_publication_capacity = self.callback_publication_capacity;
        let mut report = self
            .clock
            .advance_controlled(elapsed, self.admitted_ticks_per_callback, || {
                let mut publication = None;
                for stage in schedule.iter() {
                    match stage {
                        Stage::AuthorityStep => {
                            if !publications.is_charged() {
                                let lease = callback_leases
                                    .acquire(callback_floor)
                                    .map_err(|_| StepError::PublicationBackpressure)?;
                                publications
                                    .try_reserve_from_floor(lease, callback_publication_capacity)?;
                            }
                            let committed = catch_unwind(AssertUnwindSafe(|| {
                                authority.step_charged_for_replica(replica)
                            }))
                            .map_err(|_| StepError::TerminalGameContract)??;
                            work.page_references += committed.world_work.page_references;
                            work.copied_pages += committed.world_work.copied_pages;
                            work.copied_objects += committed.world_work.copied_objects;
                            work.group_objects += committed.world_work.group_objects;
                            publication = Some(committed);
                        }
                        Stage::ReplicaAdmission => {
                            let committed = publication.take().expect("validated producer order");
                            match replica.apply_charged(committed.group) {
                                Ok(admitted) => {
                                    replica_groups += 1;
                                    publications.push(HostPublication::ReplicatedCharged {
                                        snapshot: committed.snapshot,
                                        admitted,
                                        world_work: committed.world_work,
                                    });
                                }
                                Err(error) => fatal = Some(HostError::Replica(error)),
                            }
                        }
                    }
                    if fatal.is_some() {
                        break;
                    }
                }
                assert!(publication.is_none(), "charged host requires replica stage");
                Ok(fatal.is_none() && started.elapsed() < step_cpu_budget)
            })
            .map_err(HostError::Clock)?;
        let observed_elapsed = started.elapsed();
        let terminal_fault = if report.step_error == Some(StepError::TerminalGameContract) {
            Some(HostError::Authority(StepError::TerminalGameContract))
        } else {
            fatal
        };
        if let Some(fault) = terminal_fault {
            self.faulted = true;
            if publications.is_empty() {
                return Err(fault);
            }
        }
        let step_budget_overrun = observed_elapsed > step_cpu_budget;
        if step_budget_overrun && terminal_fault.is_none() {
            self.clock.report_cpu_overload();
            report.state = ClockState::Overloaded;
        }
        if publications.is_empty() {
            publications = HostPublications::new();
        }
        Ok(HostCallback {
            clock: report,
            terminal_fault,
            observed_elapsed,
            step_budget_overrun,
            world_work: work,
            replica_groups,
            publications,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ActorId, AuthorityId, ClosedGameLimits, ClosedValueLimits, Handle, MatchId, Origin,
        PreparedCommit, PreparedEdits, PublicationBytes, PublicationRecord, SnapshotWriteBounds,
        StableId, StateGroup, World, WorldEdit, WorldEpoch,
    };
    use std::num::NonZeroU32;

    #[test]
    fn moved_callback_vector_and_consuming_iterator_keep_its_floor_charge() {
        let bytes = Layout::array::<HostPublication<u8, (), ()>>(2)
            .unwrap()
            .size();
        let floor_bytes = CallbackBufferFloor::required_charge(bytes).unwrap();
        let pool = PublicationPool::new(floor_bytes);
        let floor = CallbackBufferFloor::try_new(&pool, bytes).unwrap();
        let mut leases = CallbackLeaseDirectory::new();
        let mut publications: HostPublications<u8, (), ()> = HostPublications::new();
        publications
            .try_reserve_from_floor(leases.acquire(&floor).unwrap(), 2)
            .unwrap();
        drop(floor);
        let mut world = World::<u8>::new(WorldEpoch(247));
        let published = world.publish(0);
        publications.push(HostPublication::Local(TickPublication {
            snapshot: published.snapshot,
            group: StateGroup {
                authority: AuthorityId::Match(MatchId(247)),
                group_id: 0,
                commit_tick: 0,
                event_cursor: 0,
                changes: vec![],
                results: vec![],
                events: vec![],
                resolved_through: vec![],
            },
            world_work: published.work,
        }));
        let moved = {
            let report = HostCallback {
                clock: ClockReport::<StepError> {
                    state: ClockState::Active,
                    completed: 1,
                    queued: 0,
                    admitted: 1,
                    refused_on_overload: 0,
                    step_error: None,
                },
                terminal_fault: None,
                observed_elapsed: Duration::ZERO,
                step_budget_overrun: false,
                world_work: WorldWork::default(),
                replica_groups: 0,
                publications,
            };
            report.publications
        };
        assert_eq!(pool.used_bytes(), floor_bytes);
        let mut iterator = moved.into_iter();
        assert!(matches!(iterator.next(), Some(HostPublication::Local(_))));
        assert_eq!(pool.used_bytes(), floor_bytes);
        drop(iterator);
        assert_eq!(pool.used_bytes(), 0);
    }

    struct SpawnRule;

    struct StructuralRule;

    struct StructuralChurnRule;

    struct ReplaceRule;

    struct TwoSpawnRule;

    impl GameRules for TwoSpawnRule {
        type Entity = u8;
        type Action = u8;
        type Change = u8;
        type Rejection = ();
        type Event = ();

        fn spawn_bound(&self, _action: &u8, _configured_max: usize) -> usize {
            2
        }

        fn decide(
            &self,
            _world: &World<u8>,
            _actor: ActorId,
            action: &u8,
            _context: &mut crate::RuleContext<'_>,
        ) -> Result<u8, ()> {
            Ok(*action)
        }

        fn prepare(
            &self,
            _world: &World<u8>,
            action: u8,
            _context: &mut crate::RuleContext<'_>,
        ) -> PreparedCommit<u8, ()> {
            let first = 33 + u128::from(action) * 2;
            PreparedCommit {
                edits: PreparedEdits::Two(
                    WorldEdit::Spawn(StableId(first), first as u8),
                    WorldEdit::Spawn(StableId(first + 1), (first + 1) as u8),
                ),
                event: None,
            }
        }
    }

    #[test]
    fn structural_maximum_two_peer_batch_publishes_thirty_two_spawns_under_contention() {
        let pool = PublicationPool::new(2_000_000);
        let mut world = World::new(WorldEpoch(301));
        for id in 1..=32_u128 {
            world.spawn(StableId(id), id as u8).unwrap();
        }
        let inline = |bytes| ClosedValueLimits {
            max_inline_bytes: bytes,
            max_payload_slots: 0,
            max_payload_bytes: 0,
        };
        let mut authority = Authority::try_new_closed_snapshot_charged(
            AuthorityId::Match(MatchId(301)),
            TwoSpawnRule,
            world,
            pool.clone(),
            SnapshotWriteBounds {
                max_edits_per_action: 2,
                max_spawns_per_action: 2,
            },
            ClosedGameLimits {
                action: inline(1),
                entity: inline(1),
                event: inline(0),
                rejection: inline(0),
            },
        )
        .unwrap();
        authority.register_peer(PeerId(1), 1, ActorId(1)).unwrap();
        authority.register_peer(PeerId(2), 1, ActorId(2)).unwrap();
        let origin = Instant::now();
        let (mut host, initial) = HeadlessHost::with_protected_structural_replica(
            origin,
            ClockConfig {
                max_ticks_per_call: NonZeroU32::new(1).unwrap(),
                ..ClockConfig::default()
            },
            HostBudget {
                callback_cpu: Duration::from_secs(1),
                other_work: Duration::from_millis(1),
                tick_cost_bound: Duration::from_millis(1),
            },
            authority,
            &pool,
            64,
            2,
            64,
            64,
            0,
        )
        .unwrap();
        let old_snapshot = host.authority().snapshot().clone();
        for peer in 1..=2_u64 {
            for sequence in 1..=8_u64 {
                host.submit(Input {
                    origin: Origin {
                        peer: PeerId(peer),
                        connection_epoch: 1,
                        sequence,
                    },
                    actor: ActorId(peer),
                    schedule: crate::InputSchedule::NextTick { observed_tick: 0 },
                    action: ((peer - 1) * 8 + sequence - 1) as u8,
                })
                .unwrap();
            }
        }
        let occupied = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes())
            .unwrap();
        let report = host
            .on_callback_charged(origin + Duration::from_millis(17))
            .unwrap();
        assert_eq!(report.clock.completed, 1);
        let HostPublication::ReplicatedCharged {
            snapshot, admitted, ..
        } = &report.publications[0]
        else {
            panic!("expected charged replica delivery")
        };
        assert_eq!(admitted.results().len(), 16);
        assert!(
            admitted
                .results()
                .iter()
                .all(|result| matches!(&result.disposition, crate::ResultDisposition::Committed))
        );
        assert_eq!(
            admitted.resolved_through(),
            &[(PeerId(1), 8), (PeerId(2), 8)]
        );
        assert_eq!(host.authority().world().slot_high_water(), 64);
        assert_eq!(host.authority().world().len(), 64);
        assert_eq!(snapshot.records().count(), 64);
        assert_eq!(host.replica().unwrap().len(), 64);
        assert_eq!(old_snapshot.records().count(), 32);
        for id in 1..=32_u128 {
            assert_eq!(host.replica().unwrap().get(StableId(id)), Some(&(id as u8)));
        }
        for id in 33..=64_u128 {
            assert_eq!(host.replica().unwrap().get(StableId(id)), Some(&(id as u8)));
            let handle = host.authority().world().handle_for(StableId(id)).unwrap();
            assert_eq!(snapshot.get(handle).unwrap().state, id as u8);
        }
        drop(report);
        drop(occupied);
        drop(old_snapshot);
        drop(initial);
        drop(host);
        assert_eq!(pool.used_bytes(), 0);
    }

    impl GameRules for ReplaceRule {
        type Entity = u8;
        type Action = u8;
        type Change = Handle;
        type Rejection = ();
        type Event = ();

        fn spawn_bound(&self, _action: &u8, _configured_max: usize) -> usize {
            0
        }

        fn decide(
            &self,
            world: &World<u8>,
            _actor: ActorId,
            _action: &u8,
            _context: &mut crate::RuleContext<'_>,
        ) -> Result<Handle, ()> {
            world.handle_for(StableId(1)).ok_or(())
        }

        fn prepare(
            &self,
            _world: &World<u8>,
            handle: Handle,
            _context: &mut crate::RuleContext<'_>,
        ) -> PreparedCommit<u8, ()> {
            PreparedCommit {
                edits: PreparedEdits::One(WorldEdit::Replace(handle, 8)),
                event: None,
            }
        }
    }

    #[test]
    #[ignore = "local paired complete-path development cost fixture"]
    fn compare_fixed_and_structural_protected_callbacks() {
        fn run(objects: usize, structural: bool) -> (u128, u128, usize) {
            let pool = PublicationPool::new(64_000_000);
            let mut world = World::new(WorldEpoch(300));
            for id in 1..=objects as u128 {
                world.spawn(StableId(id), 7_u8).unwrap();
            }
            let inline = |bytes| ClosedValueLimits {
                max_inline_bytes: bytes,
                max_payload_slots: 0,
                max_payload_bytes: 0,
            };
            let mut authority = Authority::try_new_closed_snapshot_charged(
                AuthorityId::Match(MatchId(300)),
                ReplaceRule,
                world,
                pool.clone(),
                SnapshotWriteBounds {
                    max_edits_per_action: 1,
                    max_spawns_per_action: 0,
                },
                ClosedGameLimits {
                    action: inline(1),
                    entity: inline(1),
                    event: inline(0),
                    rejection: inline(0),
                },
            )
            .unwrap();
            authority.register_peer(PeerId(1), 1, ActorId(1)).unwrap();
            let origin = Instant::now();
            let clock = ClockConfig {
                max_ticks_per_call: NonZeroU32::new(1).unwrap(),
                ..ClockConfig::default()
            };
            let budget = HostBudget {
                callback_cpu: Duration::from_secs(1),
                other_work: Duration::from_millis(1),
                tick_cost_bound: Duration::from_millis(1),
            };
            let (mut host, initial) = if structural {
                HeadlessHost::with_protected_structural_replica(
                    origin, clock, budget, authority, &pool, objects, 1, objects, objects, 0,
                )
            } else {
                HeadlessHost::with_protected_output_replica(
                    origin, clock, budget, authority, &pool, objects, 1, 0,
                )
            }
            .unwrap();
            let retained_at_start = pool.used_bytes();
            let mut samples = Vec::with_capacity(500);
            for tick in 1..=600_u64 {
                let start = Instant::now();
                host.submit(Input {
                    origin: Origin {
                        peer: PeerId(1),
                        connection_epoch: 1,
                        sequence: tick,
                    },
                    actor: ActorId(1),
                    schedule: crate::InputSchedule::NextTick {
                        observed_tick: tick - 1,
                    },
                    action: 1,
                })
                .unwrap();
                let report = host
                    .on_callback_charged(
                        origin + Duration::from_nanos((tick * 1_000_000_000).div_ceil(60)),
                    )
                    .unwrap();
                assert_eq!(report.clock.completed, 1);
                let HostPublication::ReplicatedCharged { admitted, .. } = &report.publications[0]
                else {
                    panic!("expected required replica delivery")
                };
                assert_eq!(admitted.results().len(), 1);
                assert!(matches!(
                    &admitted.results()[0].disposition,
                    crate::ResultDisposition::Committed
                ));
                assert_eq!(host.replica().unwrap().get(StableId(1)), Some(&8));
                drop(report);
                host.acknowledge_results(PeerId(1), tick).unwrap();
                if tick > 100 {
                    samples.push(start.elapsed().as_nanos());
                }
            }
            samples.sort_unstable();
            let result = (samples[250], samples[474], retained_at_start);
            drop(initial);
            drop(host);
            assert_eq!(pool.used_bytes(), 0);
            result
        }
        for objects in [4_096, 16_384] {
            let fixed = run(objects, false);
            let structural = run(objects, true);
            eprintln!(
                "paired protected one-change {objects}: fixed p50={}ns p95={}ns retained={}B; structural p50={}ns p95={}ns retained={}B",
                fixed.0, fixed.1, fixed.2, structural.0, structural.1, structural.2
            );
        }
    }

    impl GameRules for StructuralChurnRule {
        type Entity = u8;
        type Action = u64;
        type Change = u64;
        type Rejection = ();
        type Event = ();

        fn spawn_bound(&self, action: &u64, _configured_max: usize) -> usize {
            (*action & 1) as usize
        }

        fn decide(
            &self,
            world: &World<u8>,
            _actor: ActorId,
            action: &u64,
            _context: &mut crate::RuleContext<'_>,
        ) -> Result<u64, ()> {
            let id = StableId((action >> 1) as u128);
            if (*action & 1 == 0 && world.handle_for(id).is_none())
                || (*action & 1 == 1 && world.handle_for(id).is_some())
            {
                return Err(());
            }
            Ok(*action)
        }

        fn prepare(
            &self,
            world: &World<u8>,
            action: u64,
            _context: &mut crate::RuleContext<'_>,
        ) -> PreparedCommit<u8, ()> {
            let id = StableId((action >> 1) as u128);
            let edit = if action & 1 == 1 {
                WorldEdit::Spawn(id, 7)
            } else {
                WorldEdit::Despawn(world.handle_for(id).expect("decided live identity"))
            };
            PreparedCommit {
                edits: PreparedEdits::One(edit),
                event: None,
            }
        }
    }

    #[test]
    #[ignore = "local complete-path development cost fixture"]
    fn measure_structural_churn_complete_callback_cost() {
        fn run(objects: usize) -> (u128, u128, u128, u128) {
            let pool = PublicationPool::new(64_000_000);
            let mut world = World::new(WorldEpoch(297));
            for id in 1..=objects as u128 {
                world.spawn(StableId(id), 7_u8).unwrap();
            }
            let inline = |bytes| ClosedValueLimits {
                max_inline_bytes: bytes,
                max_payload_slots: 0,
                max_payload_bytes: 0,
            };
            let mut authority = Authority::try_new_closed_snapshot_charged(
                AuthorityId::Match(MatchId(297)),
                StructuralChurnRule,
                world,
                pool.clone(),
                SnapshotWriteBounds {
                    max_edits_per_action: 1,
                    max_spawns_per_action: 1,
                },
                ClosedGameLimits {
                    action: inline(8),
                    entity: inline(1),
                    event: inline(0),
                    rejection: inline(0),
                },
            )
            .unwrap();
            authority.register_peer(PeerId(1), 1, ActorId(1)).unwrap();
            let origin = Instant::now();
            let (mut host, initial) = HeadlessHost::with_protected_structural_replica(
                origin,
                ClockConfig {
                    max_ticks_per_call: NonZeroU32::new(1).unwrap(),
                    ..ClockConfig::default()
                },
                HostBudget {
                    callback_cpu: Duration::from_secs(1),
                    other_work: Duration::from_millis(1),
                    tick_cost_bound: Duration::from_millis(1),
                },
                authority,
                &pool,
                objects,
                1,
                objects,
                objects,
                0,
            )
            .unwrap();
            let mut despawn_cost = Vec::with_capacity(500);
            let mut spawn_cost = Vec::with_capacity(500);
            let mut current = 1_u64;
            for cycle in 0..600_u64 {
                let next = objects as u64 + cycle + 1;
                for (id, spawning) in [(current, false), (next, true)] {
                    let tick = cycle * 2 + u64::from(spawning) + 1;
                    let start = Instant::now();
                    host.submit(Input {
                        origin: Origin {
                            peer: PeerId(1),
                            connection_epoch: 1,
                            sequence: tick,
                        },
                        actor: ActorId(1),
                        schedule: crate::InputSchedule::NextTick {
                            observed_tick: tick - 1,
                        },
                        action: (id << 1) | u64::from(spawning),
                    })
                    .unwrap();
                    let report = host
                        .on_callback_charged(
                            origin + Duration::from_nanos((tick * 1_000_000_000).div_ceil(60)),
                        )
                        .unwrap();
                    assert_eq!(report.clock.completed, 1);
                    drop(report);
                    host.acknowledge_results(PeerId(1), tick).unwrap();
                    if cycle >= 100 {
                        if spawning {
                            &mut spawn_cost
                        } else {
                            &mut despawn_cost
                        }
                        .push(start.elapsed().as_nanos());
                    }
                }
                current = next;
            }
            despawn_cost.sort_unstable();
            spawn_cost.sort_unstable();
            let sample = (
                despawn_cost[250],
                despawn_cost[474],
                spawn_cost[250],
                spawn_cost[474],
            );
            drop(initial);
            drop(host);
            assert_eq!(pool.used_bytes(), 0);
            sample
        }
        for objects in [4_096, 16_384] {
            let (despawn_p50, despawn_p95, spawn_p50, spawn_p95) = run(objects);
            eprintln!(
                "structural complete path {objects} objects: despawn p50={despawn_p50}ns p95={despawn_p95}ns; spawn p50={spawn_p50}ns p95={spawn_p95}ns"
            );
        }
    }

    #[test]
    #[ignore = "local isolated admission cost fixture"]
    fn measure_structural_credit_upgrade_cost() {
        let pool = PublicationPool::new(1_000_000);
        let mut world = World::new(WorldEpoch(298));
        for id in 1..=32_u128 {
            world.spawn(StableId(id), id as u8).unwrap();
        }
        let inline = |bytes| ClosedValueLimits {
            max_inline_bytes: bytes,
            max_payload_slots: 0,
            max_payload_bytes: 0,
        };
        let mut authority = Authority::try_new_closed_snapshot_charged(
            AuthorityId::Match(MatchId(298)),
            StructuralRule,
            world,
            pool.clone(),
            SnapshotWriteBounds {
                max_edits_per_action: 1,
                max_spawns_per_action: 1,
            },
            ClosedGameLimits {
                action: inline(1),
                entity: inline(1),
                event: inline(0),
                rejection: inline(0),
            },
        )
        .unwrap();
        authority.register_peer(PeerId(1), 1, ActorId(1)).unwrap();
        authority.register_peer(PeerId(2), 1, ActorId(2)).unwrap();
        authority
            .enable_structural_credits(2, 33, 33, Some(0))
            .unwrap();
        let mut ordinary = Vec::with_capacity(200);
        let mut upgraded = Vec::with_capacity(200);
        for cycle in 0..600_u64 {
            let epoch = cycle + 1;
            if cycle != 0 {
                authority
                    .register_peer(PeerId(1), epoch, ActorId(1))
                    .unwrap();
                authority
                    .register_peer(PeerId(2), epoch, ActorId(2))
                    .unwrap();
            }
            let input = |peer, sequence, action| Input {
                origin: Origin {
                    peer: PeerId(peer),
                    connection_epoch: epoch,
                    sequence,
                },
                actor: ActorId(peer),
                schedule: crate::InputSchedule::Timed { intended_tick: 3 },
                action,
            };
            for sequence in 1..=8 {
                authority.submit(input(1, sequence, 0)).unwrap();
            }
            for sequence in 1..=7 {
                authority.submit(input(2, sequence, 0)).unwrap();
            }
            let upgrade = cycle % 2 == 1;
            let older_before = authority.pending_credit_bytes(PeerId(1), 1).unwrap();
            let start = Instant::now();
            let admission = authority
                .submit(input(2, 8, if upgrade { 2 } else { 0 }))
                .unwrap();
            std::hint::black_box(admission);
            let older_after = authority.pending_credit_bytes(PeerId(1), 1).unwrap();
            assert_eq!(older_after > older_before, upgrade);
            std::hint::black_box(older_after);
            std::hint::black_box(&authority);
            if cycle >= 200 {
                if upgrade {
                    &mut upgraded
                } else {
                    &mut ordinary
                }
                .push(start.elapsed().as_nanos());
            }
        }
        ordinary.sort_unstable();
        upgraded.sort_unstable();
        let ordinary_mean: u128 = ordinary.iter().sum::<u128>() / ordinary.len() as u128;
        let upgraded_mean: u128 = upgraded.iter().sum::<u128>() / upgraded.len() as u128;
        eprintln!(
            "structural admission with 15 older pending inputs: ordinary mean={ordinary_mean}ns p50={}ns p95={}ns; page-horizon upgrade mean={upgraded_mean}ns p50={}ns p95={}ns",
            ordinary[100], ordinary[189], upgraded[100], upgraded[189]
        );
        drop(authority);
        assert_eq!(pool.used_bytes(), 0);
    }

    impl GameRules for StructuralRule {
        type Entity = u8;
        type Action = u8;
        type Change = u8;
        type Rejection = ();
        type Event = ();

        fn spawn_bound(&self, action: &u8, _configured_max: usize) -> usize {
            usize::from(matches!(*action, 2 | 3))
        }

        fn decide(
            &self,
            world: &World<u8>,
            _actor: ActorId,
            action: &u8,
            _context: &mut crate::RuleContext<'_>,
        ) -> Result<u8, ()> {
            if *action == 1 && world.handle_for(StableId(1)).is_none() {
                return Err(());
            }
            Ok(*action)
        }

        fn prepare(
            &self,
            world: &World<u8>,
            action: u8,
            _context: &mut crate::RuleContext<'_>,
        ) -> PreparedCommit<u8, ()> {
            let edits = match action {
                0 => PreparedEdits::None,
                1 => PreparedEdits::One(WorldEdit::Despawn(
                    world.handle_for(StableId(1)).expect("decided live object"),
                )),
                2 => PreparedEdits::One(WorldEdit::Spawn(StableId(33), 33)),
                3 => PreparedEdits::One(WorldEdit::Spawn(StableId(34), 34)),
                4 => PreparedEdits::One(WorldEdit::Spawn(StableId(35), 35)),
                _ => unreachable!(),
            };
            PreparedCommit { edits, event: None }
        }
    }

    #[test]
    fn structural_host_upgrades_delayed_credit_before_same_tick_reuse() {
        let pool = PublicationPool::new(1_000_000);
        let mut world = World::new(WorldEpoch(295));
        for id in 1..=32_u128 {
            world.spawn(StableId(id), id as u8).unwrap();
        }
        let inline = |bytes| ClosedValueLimits {
            max_inline_bytes: bytes,
            max_payload_slots: 0,
            max_payload_bytes: 0,
        };
        let mut authority = Authority::try_new_closed_snapshot_charged(
            AuthorityId::Match(MatchId(295)),
            StructuralRule,
            world,
            pool.clone(),
            SnapshotWriteBounds {
                max_edits_per_action: 1,
                max_spawns_per_action: 1,
            },
            ClosedGameLimits {
                action: inline(1),
                entity: inline(1),
                event: inline(0),
                rejection: inline(0),
            },
        )
        .unwrap();
        authority.register_peer(PeerId(1), 1, ActorId(1)).unwrap();
        authority.register_peer(PeerId(2), 1, ActorId(2)).unwrap();
        let origin = Instant::now();
        let (mut host, initial) = HeadlessHost::with_protected_structural_replica(
            origin,
            ClockConfig {
                max_ticks_per_call: NonZeroU32::new(1).unwrap(),
                ..ClockConfig::default()
            },
            HostBudget {
                callback_cpu: Duration::from_secs(1),
                other_work: Duration::from_millis(1),
                tick_cost_bound: Duration::from_millis(1),
            },
            authority,
            &pool,
            34,
            2,
            34,
            34,
            0,
        )
        .unwrap();
        let input = |peer, sequence, schedule, action| Input {
            origin: Origin {
                peer: PeerId(peer),
                connection_epoch: 1,
                sequence,
            },
            actor: ActorId(peer),
            schedule,
            action,
        };
        host.submit(input(
            1,
            1,
            crate::InputSchedule::Timed { intended_tick: 3 },
            0,
        ))
        .unwrap();
        let before = host.authority.pending_credit_bytes(PeerId(1), 1).unwrap();
        host.submit(input(
            1,
            2,
            crate::InputSchedule::NextTick { observed_tick: 0 },
            1,
        ))
        .unwrap();
        let before_despawn = host.authority.pending_credit_bytes(PeerId(1), 2).unwrap();
        let spawn = input(2, 1, crate::InputSchedule::NextTick { observed_tick: 0 }, 2);
        let (candidate_bytes, combined_bytes) = host.authority.structural_credit_quote(1).unwrap();
        assert!(
            candidate_bytes < combined_bytes,
            "older inputs require a top-up"
        );
        let denied = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes() - candidate_bytes)
            .unwrap();
        assert_eq!(
            host.submit(spawn.clone()),
            Err(HostAdmissionError::Authority(
                AdmissionError::PublicationCapacity,
            ))
        );
        assert_eq!(
            host.authority.pending_credit_bytes(PeerId(1), 1),
            Some(before)
        );
        assert_eq!(
            host.authority.pending_credit_bytes(PeerId(1), 2),
            Some(before_despawn)
        );
        drop(denied);
        host.submit(spawn).unwrap();
        let after = host.authority.pending_credit_bytes(PeerId(1), 1).unwrap();
        assert!(
            after > before,
            "older delayed credit must gain a second-page horizon"
        );
        host.submit(input(
            2,
            2,
            crate::InputSchedule::Timed { intended_tick: 4 },
            3,
        ))
        .unwrap();
        let occupied = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes())
            .unwrap();
        for (tick, expected) in [(1, 2), (2, 0), (3, 1), (4, 1)] {
            let report = host
                .on_callback_charged(origin + Duration::from_millis(17 * tick))
                .unwrap();
            assert_eq!(report.clock.completed, 1);
            let HostPublication::ReplicatedCharged { admitted, .. } = &report.publications[0]
            else {
                panic!("expected charged replica delivery")
            };
            assert_eq!(admitted.results().len(), expected);
            if tick == 1 {
                assert_eq!(host.replica().unwrap().get(StableId(1)), None);
                assert_eq!(host.replica().unwrap().get(StableId(33)), Some(&33));
                assert_eq!(host.authority.world().slot_high_water(), 32);
            }
            if tick == 4 {
                assert_eq!(host.replica().unwrap().get(StableId(34)), Some(&34));
                assert_eq!(host.authority.world().slot_high_water(), 33);
            }
        }
        drop(occupied);
        let baseline = host.authority.try_baseline_charged().unwrap();
        let (reconnected, outcomes) =
            Replica::from_charged_baseline_with_peer_limit(baseline, &pool, 34, 2).unwrap();
        assert_eq!(reconnected.get(StableId(1)), None);
        assert_eq!(reconnected.get(StableId(33)), Some(&33));
        assert_eq!(reconnected.get(StableId(34)), Some(&34));
        drop(outcomes);
        drop(reconnected);
        host.authority
            .submit(input(
                1,
                3,
                crate::InputSchedule::Timed { intended_tick: 12 },
                3,
            ))
            .unwrap();
        let before_reconnect = pool.used_bytes();
        host.authority
            .register_peer(PeerId(1), 2, ActorId(1))
            .unwrap();
        assert!(pool.used_bytes() < before_reconnect);
        assert!(host.authority.pending_credit_bytes(PeerId(1), 3).is_none());
        assert_eq!(
            host.authority.submit(Input {
                origin: Origin {
                    peer: PeerId(1),
                    connection_epoch: 2,
                    sequence: 1
                },
                actor: ActorId(1),
                schedule: crate::InputSchedule::NextTick { observed_tick: 4 },
                action: 3,
            }),
            Ok(Admission::Queued)
        );
        drop(initial);
        drop(host);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn structural_zero_spawn_reclamation_is_admitted_at_both_caps() {
        let pool = PublicationPool::new(300_000);
        let mut world = World::new(WorldEpoch(296));
        for id in 1..=32_u128 {
            world.spawn(StableId(id), id as u8).unwrap();
        }
        let inline = |bytes| ClosedValueLimits {
            max_inline_bytes: bytes,
            max_payload_slots: 0,
            max_payload_bytes: 0,
        };
        let mut authority = Authority::try_new_closed_snapshot_charged(
            AuthorityId::Match(MatchId(296)),
            StructuralRule,
            world,
            pool.clone(),
            SnapshotWriteBounds {
                max_edits_per_action: 1,
                max_spawns_per_action: 1,
            },
            ClosedGameLimits {
                action: inline(1),
                entity: inline(1),
                event: inline(0),
                rejection: inline(0),
            },
        )
        .unwrap();
        authority.register_peer(PeerId(1), 1, ActorId(1)).unwrap();
        authority
            .enable_structural_credits(1, 32, 32, Some(0))
            .unwrap();
        let input = |sequence, action| Input {
            origin: Origin {
                peer: PeerId(1),
                connection_epoch: 1,
                sequence,
            },
            actor: ActorId(1),
            schedule: crate::InputSchedule::NextTick {
                observed_tick: sequence - 1,
            },
            action,
        };
        assert_eq!(
            authority.submit(input(1, 2)),
            Err(AdmissionError::PublicationCapacity)
        );
        assert_eq!(authority.submit(input(1, 1)), Ok(Admission::Queued));
        let first = authority.step_charged().unwrap();
        assert_eq!(first.group.results().len(), 1);
        drop(first);
        assert_eq!(authority.world().len(), 31);
        assert_eq!(authority.submit(input(2, 2)), Ok(Admission::Queued));
        let second = authority.step_charged().unwrap();
        assert_eq!(second.group.results().len(), 1);
        drop(second);
        assert_eq!(authority.world().slot_high_water(), 32);
        assert!(authority.world().handle_for(StableId(33)).is_some());
        assert_eq!(authority.submit(input(3, 4)), Ok(Admission::Queued));
        assert_eq!(
            authority.step_charged().err(),
            Some(StepError::TerminalGameContract)
        );
        assert_eq!(authority.tick_index(), 2);
        assert!(authority.world().handle_for(StableId(35)).is_none());
        assert!(authority.is_faulted());
        drop(authority);
        assert_eq!(pool.used_bytes(), 0);
    }

    struct FaultRule;

    impl GameRules for FaultRule {
        type Entity = u8;
        type Action = u8;
        type Change = u8;
        type Rejection = ();
        type Event = ();

        fn decide(
            &self,
            _world: &World<u8>,
            _actor: ActorId,
            action: &u8,
            _context: &mut crate::RuleContext<'_>,
        ) -> Result<u8, ()> {
            Ok(*action)
        }

        fn prepare(
            &self,
            _world: &World<u8>,
            action: u8,
            _context: &mut crate::RuleContext<'_>,
        ) -> PreparedCommit<u8, ()> {
            assert_eq!(action, 1, "injected second-tick game fault");
            PreparedCommit {
                edits: PreparedEdits::None,
                event: None,
            }
        }
    }

    fn idle_charged_host(
        max_ticks_per_call: u32,
    ) -> (
        HeadlessHost<FaultRule>,
        InitialChargedOutcomes<FaultRule>,
        PublicationPool,
        Instant,
    ) {
        idle_charged_host_with_peer_limit(max_ticks_per_call, 2)
    }

    fn idle_charged_host_with_peer_limit(
        max_ticks_per_call: u32,
        max_peers: usize,
    ) -> (
        HeadlessHost<FaultRule>,
        InitialChargedOutcomes<FaultRule>,
        PublicationPool,
        Instant,
    ) {
        charged_host_with_peer_limit(max_ticks_per_call, max_peers, false)
    }

    fn charged_host_with_peer_limit(
        max_ticks_per_call: u32,
        max_peers: usize,
        protected: bool,
    ) -> (
        HeadlessHost<FaultRule>,
        InitialChargedOutcomes<FaultRule>,
        PublicationPool,
        Instant,
    ) {
        let pool = PublicationPool::new(100_000);
        let mut authority = Authority::try_new_closed_snapshot_charged(
            AuthorityId::Match(MatchId(248)),
            FaultRule,
            World::new(WorldEpoch(248)),
            pool.clone(),
            SnapshotWriteBounds {
                max_edits_per_action: 0,
                max_spawns_per_action: 0,
            },
            ClosedGameLimits {
                action: ClosedValueLimits {
                    max_inline_bytes: 1,
                    max_payload_slots: 0,
                    max_payload_bytes: 0,
                },
                entity: ClosedValueLimits {
                    max_inline_bytes: 1,
                    max_payload_slots: 0,
                    max_payload_bytes: 0,
                },
                event: ClosedValueLimits {
                    max_inline_bytes: 0,
                    max_payload_slots: 0,
                    max_payload_bytes: 0,
                },
                rejection: ClosedValueLimits {
                    max_inline_bytes: 0,
                    max_payload_slots: 0,
                    max_payload_bytes: 0,
                },
            },
        )
        .unwrap();
        authority.register_peer(PeerId(1), 1, ActorId(1)).unwrap();
        authority.register_peer(PeerId(2), 1, ActorId(2)).unwrap();
        let origin = Instant::now();
        let clock = ClockConfig {
            max_ticks_per_call: NonZeroU32::new(max_ticks_per_call).unwrap(),
            max_queued_ticks: NonZeroU32::new(max_ticks_per_call.max(8)).unwrap(),
            ..ClockConfig::default()
        };
        let constructor = if protected {
            HeadlessHost::with_protected_inline_replica
        } else {
            HeadlessHost::with_charged_replica_with_peer_limit
        };
        let (host, initial) = constructor(
            origin,
            clock,
            HostBudget {
                callback_cpu: Duration::from_secs(1),
                other_work: Duration::from_millis(1),
                tick_cost_bound: Duration::from_millis(1),
            },
            authority,
            &pool,
            0,
            max_peers,
        )
        .unwrap();
        (host, initial, pool, origin)
    }

    #[test]
    fn protected_future_inputs_publish_under_competing_pool_contention() {
        let (mut host, initial, pool, origin) = charged_host_with_peer_limit(1, 2, true);
        let input = |peer, sequence, schedule| Input {
            origin: Origin {
                peer: PeerId(peer),
                connection_epoch: 1,
                sequence,
            },
            actor: ActorId(peer),
            schedule,
            action: 1,
        };
        let future = input(1, 1, crate::InputSchedule::Timed { intended_tick: 3 });
        let early = input(1, 2, crate::InputSchedule::NextTick { observed_tick: 0 });
        let other = input(2, 1, crate::InputSchedule::NextTick { observed_tick: 0 });
        let rejected = input(2, 2, crate::InputSchedule::Timed { intended_tick: 100 });
        let occupied = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes())
            .unwrap();
        assert_eq!(
            host.submit(future.clone()),
            Err(HostAdmissionError::Authority(
                AdmissionError::PublicationCapacity
            ))
        );
        drop(occupied);
        host.submit(future.clone()).unwrap();
        let credited = pool.used_bytes();
        assert_eq!(host.submit(future).unwrap(), Admission::PendingDuplicate);
        assert_eq!(pool.used_bytes(), credited);
        host.submit(early).unwrap();
        host.submit(other).unwrap();
        host.submit(rejected).unwrap();
        let occupied = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes())
            .unwrap();
        for (tick, expected) in [(1, 3), (2, 0), (3, 1)] {
            let report = host
                .on_callback_charged(origin + Duration::from_millis(17 * tick))
                .unwrap();
            assert_eq!(report.clock.completed, 1);
            assert_eq!(report.clock.step_error, None);
            let HostPublication::ReplicatedCharged { admitted, .. } = &report.publications[0]
            else {
                panic!("expected charged publication")
            };
            assert_eq!(admitted.results().len(), expected);
            if tick == 1 {
                assert_eq!(admitted.resolved_through()[0], (PeerId(1), 0));
                assert!(admitted.results().iter().any(|result| matches!(
                    result.disposition,
                    crate::ResultDisposition::Rejected(crate::ActionRejection::TooEarly)
                )));
            }
            if tick == 3 {
                assert_eq!(admitted.resolved_through()[0], (PeerId(1), 2));
            }
        }
        drop(occupied);
        drop(initial);
        drop(host);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn protected_credit_survives_snapshot_reader_backpressure() {
        let (mut host, initial, pool, origin) = charged_host_with_peer_limit(1, 2, true);
        let mut readers = Vec::new();
        for tick in 1..=8 {
            let report = host
                .on_callback_charged(origin + Duration::from_millis(17 * tick))
                .unwrap();
            assert_eq!(report.clock.completed, 1);
            let HostPublication::ReplicatedCharged { snapshot, .. } = &report.publications[0]
            else {
                panic!("expected charged publication")
            };
            readers.push(snapshot.clone());
        }
        host.submit(Input {
            origin: Origin {
                peer: PeerId(1),
                connection_epoch: 1,
                sequence: 1,
            },
            actor: ActorId(1),
            schedule: crate::InputSchedule::NextTick { observed_tick: 8 },
            action: 1,
        })
        .unwrap();
        let credited = pool.used_bytes();
        let occupied = pool.try_reserve(pool.limit_bytes() - credited).unwrap();
        let blocked = host
            .on_callback_charged(origin + Duration::from_millis(153))
            .unwrap();
        assert_eq!(blocked.clock.completed, 0);
        assert_eq!(
            blocked.clock.step_error,
            Some(StepError::SnapshotBackpressure)
        );
        assert_eq!(pool.used_bytes(), pool.limit_bytes());
        drop(readers.swap_remove(0));
        let retried = host
            .on_callback_charged(origin + Duration::from_millis(153))
            .unwrap();
        assert_eq!(retried.clock.completed, 1);
        let HostPublication::ReplicatedCharged { admitted, .. } = &retried.publications[0] else {
            panic!("expected charged publication")
        };
        assert_eq!(admitted.results().len(), 1);
        drop(retried);
        drop(blocked);
        drop(readers);
        drop(occupied);
        drop(initial);
        drop(host);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn protected_pending_credit_survives_a_later_peer_join() {
        let (mut host, initial, pool, origin) = charged_host_with_peer_limit(1, 3, true);
        host.submit(Input {
            origin: Origin {
                peer: PeerId(1),
                connection_epoch: 1,
                sequence: 1,
            },
            actor: ActorId(1),
            schedule: crate::InputSchedule::Timed { intended_tick: 2 },
            action: 1,
        })
        .unwrap();
        host.register_peer(PeerId(3), 1, ActorId(3)).unwrap();
        let occupied = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes())
            .unwrap();
        for tick in 1..=2 {
            let report = host
                .on_callback_charged(origin + Duration::from_millis(17 * tick))
                .unwrap();
            assert_eq!(report.clock.completed, 1);
            let HostPublication::ReplicatedCharged { admitted, .. } = &report.publications[0]
            else {
                panic!("expected charged publication")
            };
            assert_eq!(admitted.resolved_through().len(), 3);
            assert_eq!(admitted.results().len(), usize::from(tick == 2));
        }
        drop(occupied);
        drop(initial);
        drop(host);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn protected_profile_refuses_uncredited_pending_and_unsupported_bounds() {
        let pool = PublicationPool::new(100_000);
        let make = |bounds, payload_slots| {
            let inline = |bytes| ClosedValueLimits {
                max_inline_bytes: bytes,
                max_payload_slots: payload_slots,
                max_payload_bytes: 0,
            };
            let mut authority = Authority::try_new_closed_snapshot_charged(
                AuthorityId::Match(MatchId(250)),
                FaultRule,
                World::new(WorldEpoch(250)),
                pool.clone(),
                bounds,
                ClosedGameLimits {
                    action: inline(1),
                    entity: inline(1),
                    event: inline(0),
                    rejection: inline(0),
                },
            )
            .unwrap();
            authority.register_peer(PeerId(1), 1, ActorId(1)).unwrap();
            authority
        };
        let fixed = SnapshotWriteBounds {
            max_edits_per_action: 0,
            max_spawns_per_action: 0,
        };
        let mut pending = make(fixed, 0);
        pending
            .submit(Input {
                origin: Origin {
                    peer: PeerId(1),
                    connection_epoch: 1,
                    sequence: 1,
                },
                actor: ActorId(1),
                schedule: crate::InputSchedule::NextTick { observed_tick: 0 },
                action: 1,
            })
            .unwrap();
        assert_eq!(
            pending.enable_fixed_inline_credits(1),
            Err(ExecutionCreditSetupError::PendingInputs)
        );
        assert_eq!(
            make(fixed, 0).enable_fixed_inline_credits(0),
            Err(ExecutionCreditSetupError::PeerCapacity)
        );
        assert_eq!(
            make(
                SnapshotWriteBounds {
                    max_edits_per_action: 1,
                    max_spawns_per_action: 1,
                },
                0,
            )
            .enable_fixed_inline_credits(1),
            Err(ExecutionCreditSetupError::UnsupportedEdits)
        );
        assert_eq!(
            make(fixed, 1).enable_fixed_inline_credits(1),
            Err(ExecutionCreditSetupError::VariablePayloads)
        );
        drop(pending);
        assert_eq!(pool.used_bytes(), 0);
    }

    struct ChargedActionRule;

    impl GameRules for ChargedActionRule {
        type Entity = u8;
        type Action = PublicationRecord<0, 1>;
        type Change = u8;
        type Rejection = ();
        type Event = ();

        fn decide(
            &self,
            _world: &World<u8>,
            _actor: ActorId,
            action: &Self::Action,
            _context: &mut crate::RuleContext<'_>,
        ) -> Result<u8, ()> {
            Ok(action.payload_owner(0).unwrap().as_slice()[0])
        }

        fn prepare(
            &self,
            world: &World<u8>,
            value: u8,
            _context: &mut crate::RuleContext<'_>,
        ) -> PreparedCommit<u8, ()> {
            PreparedCommit {
                edits: PreparedEdits::One(WorldEdit::Replace(
                    world.handle_for(StableId(1)).unwrap(),
                    value,
                )),
                event: None,
            }
        }
    }

    #[test]
    fn protected_host_accepts_precharged_action_payload_under_pool_contention() {
        let pool = PublicationPool::new(200_000);
        let foreign = PublicationPool::new(1_000);
        let mut world = World::new(WorldEpoch(252));
        world.spawn(StableId(1), 1_u8).unwrap();
        let inline = |bytes| ClosedValueLimits {
            max_inline_bytes: bytes,
            max_payload_slots: 0,
            max_payload_bytes: 0,
        };
        let mut authority = Authority::try_new_closed_snapshot_charged(
            AuthorityId::Match(MatchId(252)),
            ChargedActionRule,
            world,
            pool.clone(),
            SnapshotWriteBounds {
                max_edits_per_action: 1,
                max_spawns_per_action: 0,
            },
            ClosedGameLimits {
                action: ClosedValueLimits {
                    max_inline_bytes: 0,
                    max_payload_slots: 1,
                    max_payload_bytes: 1_024,
                },
                entity: inline(1),
                event: inline(0),
                rejection: inline(0),
            },
        )
        .unwrap();
        authority.register_peer(PeerId(1), 1, ActorId(1)).unwrap();
        let origin = Instant::now();
        let (mut host, initial) = HeadlessHost::with_protected_inline_replica(
            origin,
            ClockConfig::default(),
            HostBudget {
                callback_cpu: Duration::from_millis(5),
                other_work: Duration::from_millis(1),
                tick_cost_bound: Duration::from_millis(1),
            },
            authority,
            &pool,
            1,
            1,
        )
        .unwrap();
        let input = |pool: &PublicationPool, sequence, len, schedule| Input {
            origin: Origin {
                peer: PeerId(1),
                connection_epoch: 1,
                sequence,
            },
            actor: ActorId(1),
            schedule,
            action: PublicationRecord::try_new(
                pool,
                [],
                [Some(
                    PublicationBytes::try_copy_from_slice(pool, &vec![7; len]).unwrap(),
                )],
            )
            .unwrap(),
        };
        assert_eq!(
            host.submit(input(
                &foreign,
                1,
                1,
                crate::InputSchedule::Timed { intended_tick: 3 },
            )),
            Err(HostAdmissionError::Authority(AdmissionError::WrongPool))
        );
        assert_eq!(foreign.used_bytes(), 0);
        assert_eq!(
            host.submit(input(
                &pool,
                1,
                1_025,
                crate::InputSchedule::Timed { intended_tick: 3 },
            )),
            Err(HostAdmissionError::Authority(AdmissionError::ValueProfile))
        );
        let payload_charge = PublicationBytes::required_charge(1_024).unwrap();
        let before = pool.used_bytes();
        let accepted = input(
            &pool,
            1,
            1_024,
            crate::InputSchedule::Timed { intended_tick: 3 },
        );
        let duplicate = accepted.clone();
        let occupied_before_admission = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes())
            .unwrap();
        assert_eq!(
            host.submit(accepted.clone()),
            Err(HostAdmissionError::Authority(
                AdmissionError::PublicationCapacity
            ))
        );
        drop(occupied_before_admission);
        assert_eq!(host.submit(accepted).unwrap(), crate::Admission::Queued);
        assert_eq!(
            host.submit(duplicate.clone()).unwrap(),
            crate::Admission::PendingDuplicate
        );
        host.submit(input(
            &pool,
            2,
            1_024,
            crate::InputSchedule::NextTick { observed_tick: 0 },
        ))
        .unwrap();
        assert!(pool.used_bytes() >= before + 2 * payload_charge);
        let occupied = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes())
            .unwrap();
        for (tick, expected_results) in [(1, 1), (2, 0), (3, 1)] {
            let report = host
                .on_callback_charged(origin + Duration::from_millis(17 * tick))
                .unwrap();
            assert_eq!(report.clock.completed, 1);
            let HostPublication::ReplicatedCharged { admitted, .. } = &report.publications[0]
            else {
                panic!("expected charged publication")
            };
            assert_eq!(admitted.results().len(), expected_results);
            if tick == 1 {
                assert_eq!(admitted.resolved_through(), &[(PeerId(1), 0)]);
                assert_eq!(
                    host.submit(duplicate.clone()).unwrap(),
                    crate::Admission::PendingDuplicate
                );
            }
            if tick == 3 {
                assert_eq!(admitted.resolved_through(), &[(PeerId(1), 2)]);
            }
        }
        assert_eq!(host.replica().unwrap().get(StableId(1)), Some(&7));
        assert!(matches!(
            host.submit(duplicate.clone()).unwrap(),
            crate::Admission::ResolvedDuplicate(_)
        ));
        let used_with_action = pool.used_bytes();
        host.acknowledge_results(PeerId(1), 2).unwrap();
        assert_eq!(pool.used_bytes(), used_with_action - payload_charge);
        drop(duplicate);
        assert_eq!(pool.used_bytes(), used_with_action - 2 * payload_charge);
        drop(occupied);
        drop(initial);
        drop(host);
        assert_eq!(pool.used_bytes(), 0);
    }

    struct InlineUpdateRule;

    impl GameRules for InlineUpdateRule {
        type Entity = u8;
        type Action = u8;
        type Change = u8;
        type Rejection = ();
        type Event = ();

        fn decide(
            &self,
            _world: &World<u8>,
            _actor: ActorId,
            action: &u8,
            _context: &mut crate::RuleContext<'_>,
        ) -> Result<u8, ()> {
            Ok(*action)
        }

        fn prepare(
            &self,
            world: &World<u8>,
            change: u8,
            _context: &mut crate::RuleContext<'_>,
        ) -> PreparedCommit<u8, ()> {
            let handle = world.handle_for(StableId(1)).unwrap();
            PreparedCommit {
                edits: PreparedEdits::One(if change == 0 {
                    WorldEdit::Despawn(handle)
                } else {
                    WorldEdit::Replace(handle, change)
                }),
                event: None,
            }
        }
    }

    #[test]
    fn protected_credit_covers_dirty_snapshot_page_and_change_column() {
        let pool = PublicationPool::new(100_000);
        let mut world = World::new(WorldEpoch(249));
        world.spawn(StableId(1), 1_u8).unwrap();
        let handle = world.handle_for(StableId(1)).unwrap();
        let inline = |max_inline_bytes| ClosedValueLimits {
            max_inline_bytes,
            max_payload_slots: 0,
            max_payload_bytes: 0,
        };
        let mut authority = Authority::try_new_closed_snapshot_charged(
            AuthorityId::Match(MatchId(249)),
            InlineUpdateRule,
            world,
            pool.clone(),
            SnapshotWriteBounds {
                max_edits_per_action: 1,
                max_spawns_per_action: 0,
            },
            ClosedGameLimits {
                action: inline(1),
                entity: inline(1),
                event: inline(0),
                rejection: inline(0),
            },
        )
        .unwrap();
        authority.register_peer(PeerId(1), 1, ActorId(1)).unwrap();
        let origin = Instant::now();
        let (mut host, initial) = HeadlessHost::with_protected_inline_replica(
            origin,
            ClockConfig::default(),
            HostBudget {
                callback_cpu: Duration::from_millis(5),
                other_work: Duration::from_millis(1),
                tick_cost_bound: Duration::from_millis(1),
            },
            authority,
            &pool,
            1,
            1,
        )
        .unwrap();
        host.submit(Input {
            origin: Origin {
                peer: PeerId(1),
                connection_epoch: 1,
                sequence: 1,
            },
            actor: ActorId(1),
            schedule: crate::InputSchedule::NextTick { observed_tick: 0 },
            action: 7,
        })
        .unwrap();
        let occupied = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes())
            .unwrap();
        let report = host
            .on_callback_charged(origin + Duration::from_millis(17))
            .unwrap();
        assert_eq!(report.clock.completed, 1);
        let HostPublication::ReplicatedCharged { snapshot, .. } = &report.publications[0] else {
            panic!("expected charged publication")
        };
        assert_eq!(snapshot.get(handle).unwrap().state, 7);
        assert_eq!(*host.replica().unwrap().get(StableId(1)).unwrap(), 7);
        drop(report);
        drop(occupied);
        host.submit(Input {
            origin: Origin {
                peer: PeerId(1),
                connection_epoch: 1,
                sequence: 2,
            },
            actor: ActorId(1),
            schedule: crate::InputSchedule::NextTick { observed_tick: 1 },
            action: 0,
        })
        .unwrap();
        let occupied = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes())
            .unwrap();
        let report = host
            .on_callback_charged(origin + Duration::from_millis(34))
            .unwrap();
        assert_eq!(report.clock.completed, 1);
        assert!(host.replica().unwrap().get(StableId(1)).is_none());
        drop(report);
        drop(occupied);
        drop(initial);
        drop(host);
        assert_eq!(pool.used_bytes(), 0);
    }

    struct VectorEditRule;

    impl GameRules for VectorEditRule {
        type Entity = u8;
        type Action = u8;
        type Change = u8;
        type Rejection = ();
        type Event = ();

        fn decide(
            &self,
            _world: &World<u8>,
            _actor: ActorId,
            action: &u8,
            _context: &mut crate::RuleContext<'_>,
        ) -> Result<u8, ()> {
            Ok(*action)
        }

        fn prepare(
            &self,
            world: &World<u8>,
            action: u8,
            _context: &mut crate::RuleContext<'_>,
        ) -> PreparedCommit<u8, ()> {
            PreparedCommit {
                edits: PreparedEdits::Many(vec![WorldEdit::Replace(
                    world.handle_for(StableId(1)).unwrap(),
                    action,
                )]),
                event: None,
            }
        }
    }

    #[test]
    fn protected_host_rejects_vector_edits_and_prototype_baseline() {
        let pool = PublicationPool::new(100_000);
        let mut world = World::new(WorldEpoch(256));
        world.spawn(StableId(1), 1_u8).unwrap();
        let limits = |bytes| ClosedValueLimits {
            max_inline_bytes: bytes,
            max_payload_slots: 0,
            max_payload_bytes: 0,
        };
        let mut authority = Authority::try_new_closed_snapshot_charged(
            AuthorityId::Match(MatchId(256)),
            VectorEditRule,
            world,
            pool.clone(),
            SnapshotWriteBounds {
                max_edits_per_action: 1,
                max_spawns_per_action: 0,
            },
            ClosedGameLimits {
                action: limits(1),
                entity: limits(1),
                event: limits(0),
                rejection: limits(0),
            },
        )
        .unwrap();
        authority.register_peer(PeerId(1), 1, ActorId(1)).unwrap();
        let origin = Instant::now();
        let (mut host, initial) = HeadlessHost::with_protected_inline_replica(
            origin,
            ClockConfig::default(),
            HostBudget {
                callback_cpu: Duration::from_millis(5),
                other_work: Duration::from_millis(1),
                tick_cost_bound: Duration::from_millis(1),
            },
            authority,
            &pool,
            1,
            1,
        )
        .unwrap();
        assert!(matches!(
            host.authority().try_baseline(),
            Err(StepError::Unqualified)
        ));
        assert!(host.authority().try_baseline_charged().is_ok());
        host.submit(Input {
            origin: Origin {
                peer: PeerId(1),
                connection_epoch: 1,
                sequence: 1,
            },
            actor: ActorId(1),
            schedule: crate::InputSchedule::NextTick { observed_tick: 0 },
            action: 9,
        })
        .unwrap();
        assert!(matches!(
            host.on_callback_charged(origin + Duration::from_millis(17)),
            Err(HostError::Authority(StepError::TerminalGameContract))
        ));
        assert_eq!(*host.replica().unwrap().get(StableId(1)).unwrap(), 1);
        assert_eq!(host.authority().tick_index(), 0);
        drop(initial);
        drop(host);
        assert_eq!(pool.used_bytes(), 0);
    }

    struct PageBatchRule;

    impl GameRules for PageBatchRule {
        type Entity = u8;
        type Action = StableId;
        type Change = StableId;
        type Rejection = ();
        type Event = ();

        fn decide(
            &self,
            _world: &World<u8>,
            _actor: ActorId,
            action: &StableId,
            _context: &mut crate::RuleContext<'_>,
        ) -> Result<StableId, ()> {
            Ok(*action)
        }

        fn prepare(
            &self,
            world: &World<u8>,
            id: StableId,
            _context: &mut crate::RuleContext<'_>,
        ) -> PreparedCommit<u8, ()> {
            PreparedCommit {
                edits: PreparedEdits::One(WorldEdit::Replace(world.handle_for(id).unwrap(), 9)),
                event: None,
            }
        }
    }

    #[test]
    fn protected_maximum_due_batch_publishes_sixteen_dirty_pages_under_contention() {
        let pool = PublicationPool::new(2_000_000);
        let mut world = World::new(WorldEpoch(257));
        for id in 1..=512 {
            world.spawn(StableId(id), 0_u8).unwrap();
        }
        let limits = |bytes| ClosedValueLimits {
            max_inline_bytes: bytes,
            max_payload_slots: 0,
            max_payload_bytes: 0,
        };
        let mut authority = Authority::try_new_closed_snapshot_charged(
            AuthorityId::Match(MatchId(257)),
            PageBatchRule,
            world,
            pool.clone(),
            SnapshotWriteBounds {
                max_edits_per_action: 1,
                max_spawns_per_action: 0,
            },
            ClosedGameLimits {
                action: limits(16),
                entity: limits(1),
                event: limits(0),
                rejection: limits(0),
            },
        )
        .unwrap();
        for peer in 1..=2 {
            authority
                .register_peer(PeerId(peer), 1, ActorId(peer))
                .unwrap();
        }
        let origin = Instant::now();
        let (mut host, initial) = HeadlessHost::with_protected_inline_replica(
            origin,
            ClockConfig::default(),
            HostBudget {
                callback_cpu: Duration::from_millis(5),
                other_work: Duration::from_millis(1),
                tick_cost_bound: Duration::from_millis(1),
            },
            authority,
            &pool,
            512,
            2,
        )
        .unwrap();
        for peer in 1..=2 {
            for sequence in 1..=8 {
                let page = (peer - 1) * 8 + sequence - 1;
                host.submit(Input {
                    origin: Origin {
                        peer: PeerId(peer),
                        connection_epoch: 1,
                        sequence,
                    },
                    actor: ActorId(peer),
                    schedule: crate::InputSchedule::NextTick { observed_tick: 0 },
                    action: StableId(page as u128 * 32 + 1),
                })
                .unwrap();
            }
        }
        let occupied = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes())
            .unwrap();
        let report = host
            .on_callback_charged(origin + Duration::from_millis(17))
            .unwrap();
        assert_eq!(report.clock.completed, 1);
        assert_eq!(report.world_work.copied_pages, 16);
        let HostPublication::ReplicatedCharged { admitted, .. } = &report.publications[0] else {
            panic!("expected charged publication")
        };
        assert_eq!(admitted.results().len(), 16);
        assert_eq!(
            admitted.resolved_through(),
            &[(PeerId(1), 8), (PeerId(2), 8)]
        );
        for page in 0..16 {
            assert_eq!(
                host.replica().unwrap().get(StableId(page * 32 + 1)),
                Some(&9)
            );
        }
        drop(report);
        host.acknowledge_results(PeerId(1), 8).unwrap();
        host.acknowledge_results(PeerId(2), 8).unwrap();
        drop(occupied);
        drop(initial);
        drop(host);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn sealed_host_peer_join_preflights_replica_and_keeps_idle_floor() {
        let (mut tight, initial, pool, _) = idle_charged_host(1);
        let before = pool.used_bytes();
        assert_eq!(
            tight.register_peer(PeerId(3), 1, ActorId(3)),
            Err(HostAdmissionError::Authority(
                AdmissionError::ReplicaCapacity
            ))
        );
        assert_eq!(
            tight.register_peer(PeerId(1), 2, ActorId(1)),
            Err(HostAdmissionError::RequiresBaseline)
        );
        assert_eq!(pool.used_bytes(), before);
        assert_eq!(
            tight
                .authority()
                .try_baseline_charged()
                .unwrap()
                .resolved_through()
                .len(),
            2
        );
        drop(initial);
        drop(tight);
        assert_eq!(pool.used_bytes(), 0);

        let (mut host, initial, pool, origin) = idle_charged_host_with_peer_limit(1, 3);
        let occupied = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes())
            .unwrap();
        assert_eq!(
            host.register_peer(PeerId(3), 1, ActorId(3)),
            Err(HostAdmissionError::Authority(
                AdmissionError::PublicationCapacity
            ))
        );
        drop(occupied);
        assert_eq!(
            host.authority()
                .try_baseline_charged()
                .unwrap()
                .resolved_through()
                .len(),
            2
        );
        host.register_peer(PeerId(3), 1, ActorId(3)).unwrap();
        let joined_bytes = pool.used_bytes();
        assert_eq!(
            host.register_peer(PeerId(4), 1, ActorId(4)),
            Err(HostAdmissionError::Authority(
                AdmissionError::ReplicaCapacity
            ))
        );
        assert_eq!(pool.used_bytes(), joined_bytes);
        assert_eq!(
            host.authority()
                .try_baseline_charged()
                .unwrap()
                .resolved_through()
                .len(),
            3
        );
        let occupied = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes())
            .unwrap();
        let report = host
            .on_callback_charged(origin + Duration::from_millis(17))
            .unwrap();
        assert_eq!(report.clock.completed, 1);
        let HostPublication::ReplicatedCharged { admitted, .. } = &report.publications[0] else {
            panic!("expected charged replica admission")
        };
        assert_eq!(
            admitted.resolved_through(),
            &[(PeerId(1), 0), (PeerId(2), 0), (PeerId(3), 0)]
        );
        drop(report);
        drop(occupied);
        host.submit(Input {
            origin: Origin {
                peer: PeerId(3),
                connection_epoch: 1,
                sequence: 1,
            },
            actor: ActorId(3),
            schedule: crate::InputSchedule::NextTick { observed_tick: 1 },
            action: 1,
        })
        .unwrap();
        let report = host
            .on_callback_charged(origin + Duration::from_millis(34))
            .unwrap();
        assert_eq!(report.clock.completed, 1);
        let HostPublication::ReplicatedCharged { admitted, .. } = &report.publications[0] else {
            panic!("expected new-peer result")
        };
        assert_eq!(admitted.results()[0].origin.peer, PeerId(3));
        assert_eq!(admitted.resolved_through()[2], (PeerId(3), 1));
        drop(report);
        host.acknowledge_results(PeerId(3), 1).unwrap();
        drop(initial);
        drop(host);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn sealed_host_preserves_unqualified_baseline_error() {
        let pool = PublicationPool::new(100_000);
        let authority = Authority::new(
            AuthorityId::Match(MatchId(249)),
            FaultRule,
            World::new(WorldEpoch(249)),
        );
        let result = HeadlessHost::with_charged_replica(
            Instant::now(),
            ClockConfig::default(),
            HostBudget {
                callback_cpu: Duration::from_millis(5),
                other_work: Duration::from_millis(1),
                tick_cost_bound: Duration::from_millis(1),
            },
            authority,
            &pool,
            0,
        );
        assert!(matches!(
            result,
            Err(HostError::ChargedBaseline(
                ChargedBaselineError::Unqualified
            ))
        ));
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn sealed_host_idle_floor_survives_full_competing_pool_and_old_reader() {
        for limit in [1, 16] {
            let (mut host, initial, pool, origin) = idle_charged_host(limit);
            assert_eq!(
                initial.resolved_through(),
                &[(PeerId(1), 0), (PeerId(2), 0)]
            );
            let occupied = pool
                .try_reserve(pool.limit_bytes() - pool.used_bytes())
                .unwrap();
            let old = host
                .on_callback_charged(origin + Duration::from_millis(17))
                .unwrap();
            assert_eq!(old.clock.completed, 1);
            assert_eq!(old.publications.len(), 1);
            for tick in 2..=16 {
                let report = host
                    .on_callback_charged(origin + Duration::from_millis(17 * tick))
                    .unwrap();
                assert_eq!(report.clock.completed, 1);
                assert_eq!(report.clock.step_error, None);
                assert_eq!(report.replica_groups, 1);
                assert_eq!(pool.used_bytes(), pool.limit_bytes());
            }
            assert_eq!(host.authority().tick_index(), 16);
            assert_eq!(host.replica().unwrap().tick(), 16);
            drop(old);
            drop(occupied);
            drop(initial);
            drop(host);
            assert_eq!(pool.used_bytes(), 0);
        }
    }

    #[test]
    fn exhausted_callback_iterators_hold_eight_delivery_permits_and_clock_retries() {
        let (mut host, initial, pool, origin) = idle_charged_host(16);
        let occupied = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes())
            .unwrap();
        let mut held = Vec::new();
        for tick in 1..=8 {
            let report = host
                .on_callback_charged(origin + Duration::from_millis(17 * tick))
                .unwrap();
            assert_eq!(report.clock.completed, 1);
            let mut iter = report.publications.into_iter();
            assert!(matches!(
                iter.next(),
                Some(HostPublication::ReplicatedCharged { .. })
            ));
            assert!(iter.next().is_none());
            held.push(iter);
        }
        let blocked = host
            .on_callback_charged(origin + Duration::from_millis(153))
            .unwrap();
        assert_eq!(blocked.clock.completed, 0);
        assert_eq!(
            blocked.clock.step_error,
            Some(StepError::PublicationBackpressure)
        );
        assert!(blocked.publications.is_empty());
        assert_eq!(host.authority().tick_index(), 8);
        assert_eq!(pool.used_bytes(), pool.limit_bytes());
        drop(blocked);
        drop(held.swap_remove(3));
        let retried = host
            .on_callback_charged(origin + Duration::from_millis(153))
            .unwrap();
        assert_eq!(retried.clock.completed, 1);
        assert_eq!(retried.clock.step_error, None);
        assert_eq!(host.authority().tick_index(), 9);
        drop(initial);
        drop(occupied);
        drop(host);
        assert!(pool.used_bytes() > 0);
        drop(retried);
        assert!(pool.used_bytes() > 0);
        drop(held);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn high_callback_limit_stops_at_eight_live_snapshots_before_buffer_limit() {
        let (mut host, initial, pool, origin) = idle_charged_host(16);
        let report = host
            .on_callback_charged(origin + Duration::from_millis(153))
            .unwrap();
        assert_eq!(report.clock.completed, 8);
        assert_eq!(report.publications.len(), 8);
        assert_eq!(
            report.clock.step_error,
            Some(StepError::SnapshotBackpressure)
        );
        assert_eq!(host.authority().tick_index(), 8);
        drop(report);
        let retried = host
            .on_callback_charged(origin + Duration::from_millis(153))
            .unwrap();
        assert_eq!(retried.clock.completed, 1);
        assert_eq!(host.authority().tick_index(), 9);
        drop(retried);
        drop(initial);
        drop(host);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn charged_host_delivers_first_tick_before_later_terminal_fault() {
        let pool = PublicationPool::new(100_000);
        let mut authority = Authority::try_new_closed_snapshot_charged(
            AuthorityId::Match(MatchId(246)),
            FaultRule,
            World::new(WorldEpoch(246)),
            pool.clone(),
            SnapshotWriteBounds {
                max_edits_per_action: 0,
                max_spawns_per_action: 0,
            },
            ClosedGameLimits {
                action: ClosedValueLimits {
                    max_inline_bytes: 1,
                    max_payload_slots: 0,
                    max_payload_bytes: 0,
                },
                entity: ClosedValueLimits {
                    max_inline_bytes: 1,
                    max_payload_slots: 0,
                    max_payload_bytes: 0,
                },
                event: ClosedValueLimits {
                    max_inline_bytes: 0,
                    max_payload_slots: 0,
                    max_payload_bytes: 0,
                },
                rejection: ClosedValueLimits {
                    max_inline_bytes: 0,
                    max_payload_slots: 0,
                    max_payload_bytes: 0,
                },
            },
        )
        .unwrap();
        authority.register_peer(PeerId(1), 1, ActorId(1)).unwrap();
        let origin = Instant::now();
        let (mut host, initial) = HeadlessHost::with_charged_replica(
            origin,
            ClockConfig::default(),
            HostBudget {
                callback_cpu: Duration::from_millis(5),
                other_work: Duration::from_millis(1),
                tick_cost_bound: Duration::from_millis(1),
            },
            authority,
            &pool,
            0,
        )
        .unwrap();
        for (sequence, action, schedule) in [
            (1, 1, crate::InputSchedule::NextTick { observed_tick: 0 }),
            (2, 2, crate::InputSchedule::Timed { intended_tick: 2 }),
        ] {
            host.submit(Input {
                origin: Origin {
                    peer: PeerId(1),
                    connection_epoch: 1,
                    sequence,
                },
                actor: ActorId(1),
                schedule,
                action,
            })
            .unwrap();
        }
        let report = host
            .on_callback_charged(origin + Duration::from_millis(34))
            .unwrap();
        assert_eq!(report.clock.completed, 1);
        assert_eq!(
            report.clock.step_error,
            Some(StepError::TerminalGameContract)
        );
        assert_eq!(
            report.terminal_fault,
            Some(HostError::Authority(StepError::TerminalGameContract))
        );
        assert_eq!(report.publications.len(), 1);
        let HostPublication::ReplicatedCharged { admitted, .. } = &report.publications[0] else {
            panic!("expected earlier charged group")
        };
        assert_eq!(admitted.results().len(), 1);
        assert_eq!(host.replica().unwrap().tick(), 1);
        assert!(matches!(
            host.on_callback_charged(origin + Duration::from_millis(51)),
            Err(HostError::Faulted)
        ));
        drop(initial);
        drop(host);
        drop(report);
        assert_eq!(pool.used_bytes(), 0);
    }

    impl GameRules for SpawnRule {
        type Entity = u8;
        type Action = u8;
        type Change = ();
        type Rejection = ();
        type Event = ();

        fn decide(
            &self,
            _world: &World<u8>,
            _actor: ActorId,
            action: &u8,
            _context: &mut crate::RuleContext<'_>,
        ) -> Result<(), ()> {
            if *action == 0 { Err(()) } else { Ok(()) }
        }

        fn prepare(
            &self,
            _world: &World<u8>,
            _change: (),
            _context: &mut crate::RuleContext<'_>,
        ) -> PreparedCommit<u8, ()> {
            PreparedCommit {
                edits: PreparedEdits::One(WorldEdit::Spawn(StableId(2), 2)),
                event: None,
            }
        }
    }

    #[test]
    fn required_replica_refuses_input_before_sequence_and_keeps_idle_clock_live() {
        let pool = PublicationPool::new(100_000);
        let mut world = World::new(WorldEpoch(245));
        world.spawn(StableId(1), 1_u8).unwrap();
        let mut authority = Authority::try_new_closed_snapshot_charged(
            AuthorityId::Match(MatchId(245)),
            SpawnRule,
            world,
            pool.clone(),
            SnapshotWriteBounds {
                max_edits_per_action: 1,
                max_spawns_per_action: 1,
            },
            ClosedGameLimits {
                action: ClosedValueLimits {
                    max_inline_bytes: 1,
                    max_payload_slots: 0,
                    max_payload_bytes: 0,
                },
                entity: ClosedValueLimits {
                    max_inline_bytes: 1,
                    max_payload_slots: 0,
                    max_payload_bytes: 0,
                },
                event: ClosedValueLimits {
                    max_inline_bytes: 0,
                    max_payload_slots: 0,
                    max_payload_bytes: 0,
                },
                rejection: ClosedValueLimits {
                    max_inline_bytes: 0,
                    max_payload_slots: 0,
                    max_payload_bytes: 0,
                },
            },
        )
        .unwrap();
        authority.register_peer(PeerId(1), 1, ActorId(1)).unwrap();
        authority.register_peer(PeerId(2), 1, ActorId(2)).unwrap();
        let origin = Instant::now();
        let (mut host, initial) = HeadlessHost::with_charged_replica(
            origin,
            ClockConfig::default(),
            HostBudget {
                callback_cpu: Duration::from_millis(5),
                other_work: Duration::from_millis(1),
                tick_cost_bound: Duration::from_millis(1),
            },
            authority,
            &pool,
            2,
        )
        .unwrap();
        let input = Input {
            origin: Origin {
                peer: PeerId(1),
                connection_epoch: 1,
                sequence: 1,
            },
            actor: ActorId(1),
            schedule: crate::InputSchedule::NextTick { observed_tick: 0 },
            action: 0,
        };
        assert_eq!(host.submit(input.clone()), Ok(Admission::Queued));
        assert_eq!(host.submit(input.clone()), Ok(Admission::PendingDuplicate));
        let later = Input {
            origin: Origin {
                peer: PeerId(2),
                sequence: 1,
                ..input.origin
            },
            actor: ActorId(2),
            schedule: crate::InputSchedule::Timed { intended_tick: 3 },
            action: 1,
        };
        assert_eq!(
            host.submit(later.clone()).err(),
            Some(HostAdmissionError::Authority(
                AdmissionError::ReplicaCapacity
            ))
        );
        let report = host
            .on_callback_charged(origin + Duration::from_millis(17))
            .unwrap();
        assert_eq!(report.clock.step_error, None);
        assert_eq!(report.clock.completed, 1);
        assert_eq!(host.authority().tick_index(), 1);
        assert!(host.authority().world().handle_for(StableId(2)).is_none());
        assert_eq!(host.replica().unwrap().tick(), 1);
        assert_eq!(host.replica().unwrap().len(), 1);
        assert!(matches!(
            host.submit(input),
            Ok(Admission::ResolvedDuplicate(_))
        ));
        assert_eq!(host.submit(later.clone()), Ok(Admission::Queued));
        let another = Input {
            origin: Origin {
                peer: PeerId(1),
                connection_epoch: 1,
                sequence: 2,
            },
            actor: ActorId(1),
            schedule: crate::InputSchedule::NextTick { observed_tick: 1 },
            action: 1,
        };
        assert_eq!(
            host.submit(another.clone()).err(),
            Some(HostAdmissionError::Authority(
                AdmissionError::ReplicaCapacity
            ))
        );
        let idle = host
            .on_callback_charged(origin + Duration::from_millis(34))
            .unwrap();
        assert_eq!(idle.clock.completed, 1);
        assert_eq!(host.replica().unwrap().tick(), 2);
        assert_eq!(host.replica().unwrap().len(), 1);
        let committed = host
            .on_callback_charged(origin + Duration::from_millis(51))
            .unwrap();
        assert_eq!(committed.clock.completed, 1);
        assert_eq!(host.replica().unwrap().len(), 2);
        assert!(host.authority().world().handle_for(StableId(2)).is_some());
        assert_eq!(
            host.submit(another).err(),
            Some(HostAdmissionError::Authority(
                AdmissionError::ReplicaCapacity
            ))
        );
        let later_idle = host
            .on_callback_charged(origin + Duration::from_millis(68))
            .unwrap();
        assert_eq!(later_idle.clock.completed, 1);
        assert_eq!(host.replica().unwrap().tick(), 4);
        drop(later_idle);
        drop(committed);
        drop(idle);
        drop(report);
        drop(initial);
        drop(host);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn admission_and_observation_share_the_reserved_step_budget() {
        let budget = HostBudget {
            callback_cpu: Duration::from_millis(10),
            other_work: Duration::from_millis(8),
            tick_cost_bound: Duration::from_millis(1),
        };
        assert_eq!(budget.available_step_cpu(), Duration::from_millis(2));
        assert_eq!(budget.admitted_ticks(), 2);
    }

    #[test]
    fn schedule_rejects_unordered_conflict_and_wrong_owner() {
        let conflict = [
            StageSpec {
                stage: Stage::AuthorityStep,
                phase: Phase::Commit,
                role: Role::Authority,
                reads: &[],
                writes: &[Resource::World],
                after: &[],
            },
            StageSpec {
                stage: Stage::ReplicaAdmission,
                phase: Phase::Commit,
                role: Role::Authority,
                reads: &[Resource::World],
                writes: &[],
                after: &[],
            },
        ];
        assert_eq!(
            prepare_schedule(&conflict).err(),
            Some(ScheduleError::UnorderedConflict)
        );
        let wrong_owner = [StageSpec {
            stage: Stage::ReplicaAdmission,
            phase: Phase::Observe,
            role: Role::Replica,
            reads: &[],
            writes: &[Resource::World],
            after: &[],
        }];
        assert_eq!(
            prepare_schedule(&wrong_owner).err(),
            Some(ScheduleError::WrongOwner)
        );
        let forbidden_read = [StageSpec {
            stage: Stage::ReplicaAdmission,
            phase: Phase::Observe,
            role: Role::Replica,
            reads: &[Resource::World],
            writes: &[],
            after: &[],
        }];
        assert_eq!(
            prepare_schedule(&forbidden_read).err(),
            Some(ScheduleError::WrongOwner)
        );
    }

    #[test]
    fn schedule_rejects_missing_and_backward_dependencies() {
        let missing = [StageSpec {
            stage: Stage::ReplicaAdmission,
            phase: Phase::Observe,
            role: Role::Replica,
            reads: &[],
            writes: &[Resource::Replica],
            after: &[Stage::AuthorityStep],
        }];
        assert_eq!(
            prepare_schedule(&missing).err(),
            Some(ScheduleError::MissingDependency)
        );
        let backwards = [
            StageSpec {
                stage: Stage::AuthorityStep,
                phase: Phase::Commit,
                role: Role::Authority,
                reads: &[],
                writes: &[Resource::World],
                after: &[Stage::ReplicaAdmission],
            },
            StageSpec {
                stage: Stage::ReplicaAdmission,
                phase: Phase::Observe,
                role: Role::Replica,
                reads: &[],
                writes: &[Resource::Replica],
                after: &[],
            },
        ];
        assert_eq!(
            prepare_schedule(&backwards).err(),
            Some(ScheduleError::BackwardDependency)
        );
        let cycle = [
            StageSpec {
                stage: Stage::AuthorityStep,
                phase: Phase::Commit,
                role: Role::Authority,
                reads: &[],
                writes: &[Resource::World],
                after: &[Stage::ReplicaAdmission],
            },
            StageSpec {
                stage: Stage::ReplicaAdmission,
                phase: Phase::Commit,
                role: Role::Authority,
                reads: &[],
                writes: &[Resource::Group],
                after: &[Stage::AuthorityStep],
            },
        ];
        assert_eq!(
            prepare_schedule(&cycle).err(),
            Some(ScheduleError::BackwardDependency)
        );
    }

    #[test]
    fn built_in_schedule_is_validated_inline_and_bounded() {
        assert_eq!(
            headless_schedule(false).iter().copied().collect::<Vec<_>>(),
            [Stage::AuthorityStep]
        );
        assert_eq!(
            headless_schedule(true).iter().copied().collect::<Vec<_>>(),
            [Stage::AuthorityStep, Stage::ReplicaAdmission]
        );
        let extra = [
            StageSpec {
                stage: Stage::AuthorityStep,
                phase: Phase::Commit,
                role: Role::Authority,
                reads: &[],
                writes: &[],
                after: &[],
            },
            StageSpec {
                stage: Stage::ReplicaAdmission,
                phase: Phase::Observe,
                role: Role::Replica,
                reads: &[],
                writes: &[],
                after: &[Stage::AuthorityStep],
            },
            StageSpec {
                stage: Stage::AuthorityStep,
                phase: Phase::Commit,
                role: Role::Authority,
                reads: &[],
                writes: &[],
                after: &[],
            },
        ];
        assert_eq!(prepare_schedule(&extra).err(), Some(ScheduleError::TooMany));
    }
}
