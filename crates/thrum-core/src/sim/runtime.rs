//! Generic fixed-tick authority boundary. Games own component types, candidate
//! work and rules; the engine owns registered storage and complete publication.

use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    marker::PhantomData,
    mem::size_of,
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

static NEXT_RUNTIME: AtomicU64 = AtomicU64::new(1);

#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

use crate::{Handle, StableId, WorldEpoch};

use super::{
    AccessMode, ComponentKey, ComponentRegistry, Schedule, ScheduleError, SnapshotRing,
    StructuralChange, SystemId, SystemSpec,
    column_store::TypedStore,
    registered_world::{
        EntityWriter, PreparedRead2, RegisteredWorld, RegisteredWorldError, TentativeDirty,
        WorldCapture, WorldDelta, WorldUndo,
    },
    transaction::{Edit, EditError, EditScratch},
};

/// Cold-path registered state capture for replay. Component values remain
/// opaque to the engine and are cloned only when this capture is requested.
pub struct RuntimeCapture {
    world: WorldCapture,
    tick: u64,
    nonce: u64,
    revision: u64,
}

/// Reusable changed-component and structural record for one committed tick.
pub struct RuntimeDelta {
    world: WorldDelta,
    from: u64,
    to: u64,
    nonce: u64,
    from_revision: u64,
    to_revision: u64,
}

/// Reusable sparse preimages for reversing one committed tick on this runtime.
pub struct RuntimeUndo {
    world: WorldUndo,
    events: Vec<StructuralChange>,
    from: u64,
    to: u64,
    nonce: u64,
    from_revision: u64,
    to_revision: u64,
    spent: bool,
}

/// Opaque identity of one committed runtime revision. Equal numeric ticks on
/// different replay branches have distinct endpoints.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeEndpoint {
    nonce: u64,
    tick: u64,
    revision: u64,
    timeline: u64,
}

impl RuntimeUndo {
    /// Committed tick reversed by this record.
    pub fn tick(&self) -> u64 {
        self.to
    }

    /// Visit physical slots edited by this tick, including structural edits.
    /// A slot may appear more than once; consumers deduplicate as needed.
    pub fn visit_changed_slots(&self, visit: impl FnMut(u32)) {
        self.world.visit_changed_slots(visit);
    }

    /// Physical slots changed by this tick, in a fixed-capacity bitset when
    /// one is beneficial at this world size. The words survive rewind.
    pub fn changed_words(&self) -> Option<&[u64]> {
        self.world.changed_words()
    }

    /// Whether this tick changed only one permitted component and no entity
    /// membership. Consumers may skip unrelated page columns in that case.
    ///
    /// # Errors
    ///
    /// The permit belongs to another runtime.
    pub fn changes_only<T: Send + Sync + 'static>(
        &self,
        permit: &ReadPermit<T>,
    ) -> Result<bool, RegisteredWorldError> {
        if self.nonce != permit.nonce {
            return Err(RegisteredWorldError::WrongRuntime);
        }
        Ok(self.world.changes_only(permit.component))
    }

    /// Reserved sparse storage retained for reuse.
    pub fn reserved_bytes(&self) -> usize {
        self.world.reserved_bytes() + self.events.capacity() * size_of::<StructuralChange>()
    }
}

/// Worker selection for one deterministic fixed-tick candidate. Platform
/// policy can select a pool for its core classes without changing game rules.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionMode {
    /// Run each job on the tick caller.
    Serial,
    /// Permit Rayon dispatch under the game's measured work policy.
    Rayon,
    /// Force native Rayon dispatch for correctness and crossover qualification.
    RayonForced,
}

/// Whether a committed tick also refreshes an optional presentation page.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicationMode {
    /// Capture this committed endpoint for presentation.
    Capture,
    /// Keep authority advancing; the next capture rebuilds any missed view.
    Defer,
}

/// Engine-owned dispatch over independent mutable candidate jobs. Both modes
/// use the same job callback; games apply results in canonical order afterward.
#[derive(Clone, Copy, Debug)]
pub struct Executor {
    mode: ExecutionMode,
}

impl Executor {
    /// Active dispatch mode.
    pub fn mode(self) -> ExecutionMode {
        self.mode
    }

    /// Execute independent candidate jobs, never direct committed-world edits.
    #[cfg_attr(
        target_arch = "wasm32",
        expect(
            clippy::unused_self,
            reason = "the executor mode selects native Rayon; browser builds deliberately run every mode serially"
        )
    )]
    pub fn for_each_mut<T: Send>(self, jobs: &mut [T], run: impl Fn(&mut T) + Send + Sync) {
        #[cfg(not(target_arch = "wasm32"))]
        if self.mode != ExecutionMode::Serial && jobs.len() > 1 {
            jobs.par_iter_mut().for_each(run);
            return;
        }
        jobs.iter_mut().for_each(run);
    }
}

impl RuntimeCapture {
    /// Hash cold authority metadata for replay diagnostics. The caller also
    /// hashes its game component and resource values to cover the full state.
    pub fn hash_authority_metadata(&self, hash: &mut impl Hasher) {
        self.tick.hash(hash);
        self.world.hash_metadata(hash);
    }

    /// Whether a one-tick delta follows this exact committed endpoint.
    pub fn accepts_delta(&self, delta: &RuntimeDelta) -> bool {
        delta.nonce == self.nonce && delta.from == self.tick && delta.from_revision == self.revision
    }

    /// Committed tick represented by this capture.
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Retained full baseline storage including typed component columns.
    pub fn reserved_bytes(&self) -> usize {
        self.world.reserved_bytes()
    }

    /// Independently copy a registered baseline, including authored component values.
    #[must_use]
    pub fn clone_independent(&self) -> Self {
        Self {
            world: self.world.clone_independent(),
            tick: self.tick,
            nonce: self.nonce,
            revision: self.revision,
        }
    }

    /// Reuse a compatible capture's storage while copying its complete state.
    ///
    /// # Errors
    ///
    /// The source belongs to another runtime generation.
    pub fn copy_from(&mut self, source: &Self) -> Result<(), RuntimeError> {
        if self.nonce != source.nonce {
            return Err(RuntimeError::WrongCapture);
        }
        self.world.copy_from(&source.world);
        self.tick = source.tick;
        self.revision = source.revision;
        Ok(())
    }

    /// Advance a full baseline with one contiguous delta from the same runtime.
    ///
    /// # Errors
    ///
    /// The delta belongs to another runtime or does not follow this capture.
    ///
    /// # Panics
    ///
    /// Only if an engine-produced delta contradicts its recorded slot order or schema.
    pub fn advance(&mut self, delta: &RuntimeDelta) -> Result<(), RuntimeError> {
        if !self.accepts_delta(delta) {
            return Err(RuntimeError::WrongCapture);
        }
        self.world.advance(&delta.world);
        self.tick = delta.to;
        self.revision = delta.to_revision;
        Ok(())
    }
}

impl RuntimeDelta {
    /// Whether this delta immediately follows another on the same branch.
    pub fn follows(&self, previous: &Self) -> bool {
        self.nonce == previous.nonce
            && self.from == previous.to
            && self.from_revision == previous.to_revision
    }

    /// Committed tick represented by this one-tick delta.
    pub fn tick(&self) -> u64 {
        self.to
    }

    /// Retained backing storage, including typed component columns.
    pub fn reserved_bytes(&self) -> usize {
        self.world.reserved_bytes()
    }
}

/// Setup, access or checked-world refusal before a game tick commits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeError {
    /// A component or identity operation failed without changing committed state.
    World(RegisteredWorldError),
    /// System declarations could not form a valid schedule.
    Schedule(ScheduleError),
    /// A query or edit was requested without declared component access.
    UndeclaredAccess,
    /// Initial seeding was attempted after publication or the first tick.
    AlreadyStarted,
    /// This process exhausted unique runtime access-token generations.
    RuntimeExhausted,
    /// No committed tick exists for a one-tick delta.
    NoCommittedTick,
    /// The replay capture belongs to another runtime instance.
    WrongCapture,
}

impl From<RegisteredWorldError> for RuntimeError {
    fn from(error: RegisteredWorldError) -> Self {
        Self::World(error)
    }
}

/// A due tick is retained on refusal; the caller may retry after correcting it.
#[derive(Debug, Eq, PartialEq)]
pub enum TickError<E> {
    /// The tick counter cannot advance without wrapping.
    TickExhausted,
    /// The committed endpoint revision cannot advance without wrapping.
    RevisionExhausted,
    /// Tick-zero seed membership must be published before ticking.
    BaselineRequired,
    /// Game-owned validation refused the candidate before any commit.
    Game(E),
    /// Engine preflight rejected a typed candidate edit batch.
    Edit(EditError),
}

/// Game-owned rules for one registered authority. Preparation receives only
/// immutable game state and mutable candidate scratch. `apply` and `capture`
/// run only after complete candidate validation and must be infallible.
pub trait TickGame<P> {
    /// Expected game-rule or input error.
    type Error: Send;
    /// Reusable game-owned candidate and query scratch, never committed state.
    type Candidate;
    /// Private reusable output for one conflict-free parallel system callback.
    type ParallelOutput: Default + Send;

    /// Reset reusable candidate work before the first registered callback.
    fn reset_candidate(&self, candidate: &mut Self::Candidate);

    /// Clear one private parallel buffer before every callback, retaining capacity.
    fn reset_parallel_output(&self, output: &mut Self::ParallelOutput);

    /// Merge one completed private output in compiled system order.
    fn merge_parallel(
        &self,
        system: SystemId,
        output: &mut Self::ParallelOutput,
        candidate: &mut Self::Candidate,
        tick: u64,
    );

    /// Typed edits prepared in reusable game-owned candidate storage. The
    /// engine validates the entire list before applying any of its members.
    fn edits<'a>(&self, _candidate: &'a Self::Candidate) -> &'a [Edit<Self::Candidate>] {
        &[]
    }

    /// Apply the fully validated candidate. All target handles and registered
    /// component presence must have been checked during `prepare`.
    fn apply(&mut self, world: &WorldRead<'_>, tick: u64, candidate: &Self::Candidate);

    /// Copy one complete committed endpoint into an unleased reusable page.
    fn capture(
        &mut self,
        world: &WorldRead<'_>,
        candidate: &mut Self::Candidate,
        events: &[StructuralChange],
        page: &mut P,
        first: bool,
        tick: u64,
        timeline: u64,
    );
}

/// One game callback bound to its access declaration before schedule compile.
pub struct RegisteredSystem<P, G: TickGame<P>> {
    spec: SystemSpec,
    callback: Callback<P, G>,
}

type SystemCallback<P, G> = fn(
    &G,
    &WorldRead<'_>,
    Executor,
    u64,
    &mut <G as TickGame<P>>::Candidate,
) -> Result<(), <G as TickGame<P>>::Error>;

type ParallelCallback<P, G> = fn(
    &G,
    &WorldRead<'_>,
    Executor,
    u64,
    &mut <G as TickGame<P>>::ParallelOutput,
) -> Result<(), <G as TickGame<P>>::Error>;

type DirectCallback<P, G> = fn(
    &G,
    &mut WorldTxn<'_>,
    Executor,
    u64,
    &mut <G as TickGame<P>>::Candidate,
) -> Result<(), <G as TickGame<P>>::Error>;

enum Callback<P, G: TickGame<P>> {
    Serial(SystemCallback<P, G>),
    Parallel(ParallelCallback<P, G>),
    Direct(DirectCallback<P, G>),
}

struct BoundSystem<P, G: TickGame<P>> {
    id: SystemId,
    callback: Callback<P, G>,
    output: G::ParallelOutput,
    #[cfg(not(target_arch = "wasm32"))]
    result: Option<Result<(), G::Error>>,
}

impl<P, G: TickGame<P>> RegisteredSystem<P, G> {
    /// Bind one declared system to the game callback the engine will invoke.
    pub fn new(spec: SystemSpec, callback: SystemCallback<P, G>) -> Self {
        Self {
            spec,
            callback: Callback::Serial(callback),
        }
    }

    /// Bind a conflict-free callback that writes only to its private output.
    pub fn parallel(spec: SystemSpec, callback: ParallelCallback<P, G>) -> Self {
        Self {
            spec,
            callback: Callback::Parallel(callback),
        }
    }

    /// Bind a typed in-place system. Tentative component writes roll back if
    /// this or any later precommit callback refuses the tick.
    pub fn direct(spec: SystemSpec, callback: DirectCallback<P, G>) -> Self {
        Self {
            spec,
            callback: Callback::Direct(callback),
        }
    }
}

fn check_edit_system<C, E>(
    edits: &[Edit<C>],
    before: usize,
    system: SystemId,
) -> Result<(), TickError<E>> {
    if edits.len() < before || edits[before..].iter().any(|edit| edit.system() != system) {
        return Err(TickError::Edit(EditError::WrongSystem));
    }
    Ok(())
}

struct DirectDirty {
    words: Vec<Vec<u64>>,
    slots: Vec<(usize, u32)>,
}

impl DirectDirty {
    fn new(components: usize, capacity: usize) -> Self {
        Self {
            words: vec![vec![0; capacity.div_ceil(64)]; components],
            slots: Vec::with_capacity(capacity),
        }
    }

    fn clear(&mut self) {
        for &(component, slot) in &self.slots {
            self.words[component][slot as usize / 64] &= !(1_u64 << (slot % 64));
        }
        self.slots.clear();
    }
}

impl TentativeDirty for DirectDirty {
    fn mark_first(&mut self, component: usize, slot: u32) -> bool {
        let word = &mut self.words[component][slot as usize / 64];
        let bit = 1_u64 << (slot % 64);
        if *word & bit != 0 {
            return false;
        }
        *word |= bit;
        self.slots.push((component, slot));
        true
    }

    fn unmark_last(&mut self, component: usize, slot: u32) {
        assert_eq!(self.slots.pop(), Some((component, slot)));
        self.words[component][slot as usize / 64] &= !(1_u64 << (slot % 64));
    }
}

/// One engine-owned registered world, compiled schedule and leased page ring.
/// The concrete game supplies no engine-side component or action type.
pub struct GameRuntime<P, G: TickGame<P>> {
    world: RegisteredWorld,
    schedule: Schedule,
    waves: Vec<Vec<BoundSystem<P, G>>>,
    pages: SnapshotRing<P>,
    initial_published: bool,
    publication_skipped: bool,
    rewind_patch_target: Option<u64>,
    tick: u64,
    timeline: u64,
    revision: u64,
    next_revision: u64,
    delta_from_revision: u64,
    delta_ready: bool,
    executor: Executor,
    nonce: u64,
    edit_scratch: EditScratch,
    direct_dirty: DirectDirty,
    abort_undo: WorldUndo,
    structural_journal: Vec<StructuralChange>,
}

impl<P, G: TickGame<P>> GameRuntime<P, G> {
    /// Identity of the current committed authority endpoint.
    pub fn endpoint(&self) -> RuntimeEndpoint {
        RuntimeEndpoint {
            nonce: self.nonce,
            tick: self.tick,
            revision: self.revision,
            timeline: self.timeline,
        }
    }

    /// Allocate one reusable sparse undo record for this registered schema.
    pub fn new_undo(&self) -> RuntimeUndo {
        RuntimeUndo {
            world: self.world.empty_undo(),
            events: Vec::new(),
            from: 0,
            to: 0,
            nonce: self.nonce,
            from_revision: 0,
            to_revision: 0,
            spent: false,
        }
    }

    /// Whether a retained capture belongs to this exact runtime generation.
    pub fn matches_capture(&self, capture: &RuntimeCapture) -> bool {
        capture.nonce == self.nonce
    }

    /// Allocate an empty reusable delta for this registered schema.
    pub fn new_delta(&self) -> RuntimeDelta {
        RuntimeDelta {
            world: self.world.empty_delta(),
            from: 0,
            to: 0,
            nonce: self.nonce,
            from_revision: 0,
            to_revision: 0,
        }
    }

    /// Fill a reusable delta immediately after its committed tick.
    ///
    /// # Errors
    ///
    /// No tick has committed, or the delta belongs to another runtime.
    pub fn capture_delta_into(&self, delta: &mut RuntimeDelta) -> Result<(), RuntimeError> {
        if delta.nonce != self.nonce {
            return Err(RuntimeError::WrongCapture);
        }
        if !self.delta_ready {
            return Err(RuntimeError::NoCommittedTick);
        }
        let from = self
            .tick
            .checked_sub(1)
            .ok_or(RuntimeError::NoCommittedTick)?;
        self.world
            .capture_delta_into(&self.structural_journal, &mut delta.world);
        delta.from = from;
        delta.to = self.tick;
        delta.from_revision = self.delta_from_revision;
        delta.to_revision = self.revision;
        Ok(())
    }

    /// Rebuild a complete endpoint from one baseline and contiguous deltas.
    ///
    /// # Errors
    ///
    /// A capture is foreign or the delta sequence has a gap.
    ///
    /// # Panics
    ///
    /// Only if an engine-produced delta contradicts its recorded slot order or schema.
    pub fn restore_through(
        &mut self,
        base: &RuntimeCapture,
        deltas: &[RuntimeDelta],
        make_page: impl FnMut() -> P,
    ) -> Result<(), RuntimeError> {
        if base.nonce != self.nonce {
            return Err(RuntimeError::WrongCapture);
        }
        let mut merged = base.clone_independent();
        for delta in deltas {
            merged.advance(delta)?;
        }
        self.restore(&merged, make_page)
    }

    /// Capture registered components, identities, slot generations and free
    /// order after a complete committed tick. This is a cold replay path.
    pub fn capture(&self) -> RuntimeCapture {
        RuntimeCapture {
            world: self.world.capture(),
            tick: self.tick,
            nonce: self.nonce,
            revision: self.revision,
        }
    }

    /// Restore a compatible capture and invalidate the active page ring. The
    /// game restores its own resource state, then calls `publish_initial` to
    /// seal one complete replay baseline before the next tick.
    ///
    /// # Errors
    ///
    /// The capture belongs to a different epoch, capacity or component schema.
    pub fn restore(
        &mut self,
        capture: &RuntimeCapture,
        make_page: impl FnMut() -> P,
    ) -> Result<(), RuntimeError> {
        if capture.nonce != self.nonce {
            return Err(RuntimeError::WrongCapture);
        }
        let following_timeline = self
            .timeline
            .checked_add(1)
            .ok_or(RuntimeError::RuntimeExhausted)?;
        self.world.restore(&capture.world)?;
        self.tick = capture.tick;
        self.timeline = following_timeline;
        self.revision = capture.revision;
        self.delta_ready = false;
        self.world.begin_tick(None);
        self.structural_journal.clear();
        self.pages.reset_timeline();
        self.initial_published = false;
        if !self.pages.can_capture() {
            self.pages = SnapshotRing::new(make_page);
        }
        self.publication_skipped = false;
        self.rewind_patch_target = None;
        Ok(())
    }

    /// Reverse contiguous committed ticks without copying unaffected rows or
    /// allocating a new page ring. Publication remains invalid until a later
    /// page capture; held old pages are never changed.
    ///
    /// # Errors
    ///
    /// The records are foreign, out of order or the timeline is exhausted.
    ///
    /// # Panics
    ///
    /// Only if an engine-produced undo contradicts its committed free order.
    pub fn rewind_many(&mut self, steps: &mut [RuntimeUndo]) -> Result<(), RuntimeError> {
        if steps.is_empty() {
            return Ok(());
        }
        let timeline = self
            .timeline
            .checked_add(1)
            .ok_or(RuntimeError::RuntimeExhausted)?;
        let mut tick = self.tick;
        let mut revision = self.revision;
        for step in steps.iter().rev() {
            if step.nonce != self.nonce
                || step.spent
                || step.to != tick
                || step.to_revision != revision
                || step.from.checked_add(1) != Some(step.to)
            {
                return Err(RuntimeError::WrongCapture);
            }
            tick = step.from;
            revision = step.from_revision;
        }
        let coalesced_component = (steps.len() > 1
            && steps.iter().all(|step| step.events.is_empty()))
        .then(|| {
            steps
                .iter()
                .find_map(|step| step.world.first_changed_component())
        })
        .flatten()
        .filter(|&component| {
            steps.iter().all(|step| {
                step.world.changes_only(component)
                    && step.world.can_coalesce_without_drop_order(component)
            })
        });
        if let Some(component) = coalesced_component {
            self.world.begin_tick(None);
            for step in steps.iter_mut() {
                self.world.restore_first_seen(&mut step.world, component);
                step.spent = true;
            }
            self.world.begin_tick(None);
            self.world.invalidate_change_frames();
        } else {
            for step in steps.iter_mut().rev() {
                step.world.restore(&mut self.world, &step.events);
                step.spent = true;
            }
        }
        self.tick = tick;
        self.revision = revision;
        self.timeline = timeline;
        self.delta_ready = false;
        self.structural_journal.clear();
        self.pages.reset_timeline();
        self.publication_skipped = true;
        self.rewind_patch_target = Some(tick);
        Ok(())
    }

    /// Publish an inverse patch from an unleased endpoint of the retired
    /// branch. The game names eligible source pages and updates only slots
    /// touched between that source and the restored authority tick.
    pub fn patch_rewound_page(
        &mut self,
        steps: &[RuntimeUndo],
        source_tick: impl Fn(&P) -> Option<u64>,
        patch: impl FnOnce(&WorldRead<'_>, &mut P, u64, u64, u64, &[RuntimeUndo]),
    ) -> bool {
        if !self.publication_skipped
            || self.rewind_patch_target != Some(self.tick)
            || steps.first().is_none_or(|first| first.from != self.tick)
        {
            return false;
        }
        let world = WorldRead {
            world: &self.world,
            nonce: self.nonce,
            system: None,
        };
        if self
            .pages
            .patch_rewind(source_tick, |page, source| {
                patch(&world, page, source, self.tick, self.timeline, steps);
            })
            .is_err()
        {
            return false;
        }
        self.publication_skipped = false;
        self.rewind_patch_target = None;
        true
    }

    /// Current presentation branch generation.
    pub fn timeline(&self) -> u64 {
        self.timeline
    }

    /// Compile declarations and allocate the world and four complete pages.
    ///
    /// # Errors
    ///
    /// Capacity, registration or schedule validation failed.
    ///
    /// # Panics
    ///
    /// Only if the compiled schedule refers to a registration absent from
    /// its own validated input list.
    ///
    pub fn new(
        epoch: WorldEpoch,
        capacity: usize,
        registry: ComponentRegistry,
        systems: Vec<RegisteredSystem<P, G>>,
        make_page: impl FnMut() -> P,
    ) -> Result<Self, RuntimeError> {
        let schedule = Schedule::new(
            &registry,
            systems.iter().map(|system| system.spec.clone()).collect(),
        )
        .map_err(RuntimeError::Schedule)?;
        let mut callbacks_by_id: HashMap<_, _> = systems
            .into_iter()
            .map(|system| (system.spec.id, system.callback))
            .collect();
        let waves = schedule
            .batches()
            .map(|batch| {
                batch
                    .iter()
                    .map(|id| BoundSystem {
                        id: *id,
                        callback: callbacks_by_id.remove(id).expect("compiled registration"),
                        output: G::ParallelOutput::default(),
                        #[cfg(not(target_arch = "wasm32"))]
                        result: None,
                    })
                    .collect()
            })
            .collect();
        let component_count = registry.entries.len();
        let world = RegisteredWorld::new(epoch, capacity, registry)?;
        let nonce = NEXT_RUNTIME
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1)
            })
            .map_err(|_| RuntimeError::RuntimeExhausted)?;
        let abort_undo = world.empty_undo();
        Ok(Self {
            world,
            schedule,
            waves,
            pages: SnapshotRing::new(make_page),
            initial_published: false,
            publication_skipped: false,
            rewind_patch_target: None,
            tick: 0,
            timeline: 0,
            revision: 0,
            next_revision: 1,
            delta_from_revision: 0,
            delta_ready: false,
            executor: Executor {
                mode: ExecutionMode::Serial,
            },
            nonce,
            edit_scratch: EditScratch::new(capacity, component_count),
            direct_dirty: DirectDirty::new(component_count, capacity),
            abort_undo,
            structural_journal: Vec::with_capacity(128),
        })
    }

    /// Last complete committed tick.
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// Select serial or native parallel candidate work before the next tick.
    pub fn set_execution_mode(&mut self, mode: ExecutionMode) {
        self.executor.mode = mode;
    }

    /// Current engine dispatch policy.
    pub fn execution_mode(&self) -> ExecutionMode {
        self.executor.mode
    }

    /// Generation shared by checked handles and pages.
    pub fn epoch(&self) -> WorldEpoch {
        self.world.epoch()
    }

    /// Live entity count.
    pub fn len(&self) -> usize {
        self.world.len()
    }

    /// Free physical slots available before structural edits.
    pub fn free_count(&self) -> usize {
        self.world.free_count()
    }

    /// Whether a validated despawn returns its slot to the free list.
    ///
    /// # Errors
    ///
    /// The handle is stale or from another world epoch.
    pub fn reusable_after_despawn(&self, handle: Handle) -> Result<bool, RegisteredWorldError> {
        self.world.entity(handle)?;
        Ok(self.world.reusable_after_despawn(handle))
    }

    /// Whether this authority has no live entities.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Resolve one stable identity to a checked current handle.
    pub fn handle(&self, id: StableId) -> Option<Handle> {
        self.world.handle(id)
    }

    /// Read committed game components through checked identity.
    pub fn read(&self) -> WorldRead<'_> {
        WorldRead {
            world: &self.world,
            nonce: self.nonce,
            system: None,
        }
    }

    /// Compiled game system graph.
    pub fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    /// Seed a registered bundle before the first publication or tick.
    ///
    /// # Errors
    ///
    /// The game has started, identity repeats, capacity is full, or the bundle
    /// contains an unregistered or duplicate component.
    pub fn seed(
        &mut self,
        id: StableId,
        fill: impl FnOnce(&mut EntityWriter<'_>) -> Result<(), RegisteredWorldError>,
    ) -> Result<Handle, RuntimeError> {
        if self.tick != 0 || self.initial_published {
            return Err(RuntimeError::AlreadyStarted);
        }
        self.world.spawn(id, fill).map_err(Into::into)
    }

    /// Prepare a storage-order read cursor once for a declared system.
    ///
    /// # Errors
    ///
    /// The system lacks read access or either type is unregistered.
    pub fn read2<T: Send + Sync + 'static, U: Send + Sync + 'static>(
        &self,
        system: SystemId,
    ) -> Result<Read2<T, U>, RuntimeError> {
        if !self
            .schedule
            .permits(system, ComponentKey::of::<T>(), AccessMode::Read)
            || !self
                .schedule
                .permits(system, ComponentKey::of::<U>(), AccessMode::Read)
        {
            return Err(RuntimeError::UndeclaredAccess);
        }
        Ok(Read2 {
            query: self.world.read2::<T, U>()?,
            nonce: self.nonce,
            system,
        })
    }

    /// Prepare a storage-order query whose access is `&mut T` and `&U`.
    /// The distinct columns and schedule permissions are checked once.
    ///
    /// # Errors
    ///
    /// The types are identical, absent or undeclared for this system.
    pub fn write_read2<T: Clone + Send + Sync + 'static, U: Send + Sync + 'static>(
        &self,
        system: SystemId,
    ) -> Result<WriteRead2<T, U>, RuntimeError> {
        let first = self
            .world
            .component_index(ComponentKey::of::<T>())
            .ok_or(RuntimeError::UndeclaredAccess)?;
        let second = self
            .world
            .component_index(ComponentKey::of::<U>())
            .ok_or(RuntimeError::UndeclaredAccess)?;
        if first == second
            || !self
                .schedule
                .permits(system, ComponentKey::of::<T>(), AccessMode::Write)
            || !self
                .schedule
                .permits(system, ComponentKey::of::<U>(), AccessMode::Read)
        {
            return Err(RuntimeError::UndeclaredAccess);
        }
        Ok(WriteRead2 {
            first,
            second,
            nonce: self.nonce,
            system,
            marker: PhantomData,
        })
    }

    /// Grant a typed sparse read only when declared in the compiled schedule.
    ///
    /// # Errors
    ///
    /// The system did not declare access to this registered component.
    pub fn read_permit<T: Send + Sync + 'static>(
        &self,
        system: SystemId,
    ) -> Result<ReadPermit<T>, RuntimeError> {
        if !self
            .schedule
            .permits(system, ComponentKey::of::<T>(), AccessMode::Read)
        {
            return Err(RuntimeError::UndeclaredAccess);
        }
        let component = self
            .world
            .component_index(ComponentKey::of::<T>())
            .ok_or(RuntimeError::UndeclaredAccess)?;
        Ok(ReadPermit {
            marker: PhantomData,
            nonce: self.nonce,
            system,
            component,
        })
    }

    /// Grant a typed sparse edit only when it appears in the compiled schedule.
    ///
    /// # Errors
    ///
    /// The system did not declare a write to this registered component.
    pub fn write_permit<T: Send + Sync + 'static>(
        &self,
        system: SystemId,
    ) -> Result<WritePermit<T>, RuntimeError> {
        if !self
            .schedule
            .permits(system, ComponentKey::of::<T>(), AccessMode::Write)
        {
            return Err(RuntimeError::UndeclaredAccess);
        }
        let component = self
            .world
            .component_index(ComponentKey::of::<T>())
            .ok_or(RuntimeError::UndeclaredAccess)?;
        Ok(WritePermit {
            marker: PhantomData,
            nonce: self.nonce,
            system,
            component,
        })
    }

    /// Grant structural edits only to a system with an explicit external write.
    ///
    /// # Errors
    ///
    /// The system did not declare this structural resource write.
    pub fn structural_permit(
        &self,
        system: SystemId,
        resource: u64,
    ) -> Result<StructuralPermit, RuntimeError> {
        if !self
            .schedule
            .permits_external(system, resource, AccessMode::Write)
        {
            return Err(RuntimeError::UndeclaredAccess);
        }
        Ok(StructuralPermit {
            resource,
            nonce: self.nonce,
            system,
        })
    }

    /// Lease the latest complete game-authored page, if one exists.
    pub fn latest(&self) -> Option<Arc<P>> {
        self.pages.latest()
    }

    /// Lease the immediately preceding complete page, if one exists.
    pub fn previous(&self) -> Option<Arc<P>> {
        self.pages.previous()
    }

    /// Whether a tick can publish without blocking on a held reader.
    pub fn can_publish_next(&self) -> bool {
        self.pages.can_capture()
    }

    /// Whether the latest immutable page represents the current authority tick.
    pub fn has_current_publication(&self) -> bool {
        !self.publication_skipped && self.pages.has_latest()
    }

    /// Seal a full current baseline after optional publication was skipped.
    /// Returns false while all reusable pages remain leased. This never runs
    /// game systems or advances the committed tick.
    pub fn publish_current_if_available(
        &mut self,
        game: &mut G,
        candidate: &mut G::Candidate,
    ) -> bool {
        if !self.publication_skipped {
            return self.pages.has_latest();
        }
        if !self.pages.can_capture() {
            return false;
        }
        let world = WorldRead {
            world: &self.world,
            nonce: self.nonce,
            system: None,
        };
        if self
            .pages
            .capture(true, |page, first| {
                game.capture(
                    &world,
                    candidate,
                    &self.structural_journal,
                    page,
                    first,
                    self.tick,
                    self.timeline,
                );
            })
            .is_err()
        {
            return false;
        }
        self.publication_skipped = false;
        self.rewind_patch_target = None;
        true
    }

    /// Reserved page-ring bytes including game-owned nested page columns.
    pub fn publication_reserved_bytes(&self, page_bytes: impl Fn(&P) -> usize) -> usize {
        self.pages.reserved_bytes(page_bytes)
    }

    /// Seal tick zero before any mutation, preserving seeded structural state.
    ///
    /// # Panics
    ///
    /// Panics only if an unpublished ring has lost all four private pages or
    /// the game capture callback panics.
    pub fn publish_initial(&mut self, game: &mut G, candidate: &mut G::Candidate) {
        if !self.initial_published {
            let world = WorldRead {
                world: &self.world,
                nonce: self.nonce,
                system: None,
            };
            self.pages
                .capture(false, |page, first| {
                    game.capture(
                        &world,
                        candidate,
                        &[],
                        page,
                        first,
                        self.tick,
                        self.timeline,
                    );
                })
                .expect("unpublished ring has a free page");
            self.initial_published = true;
        }
    }

    /// Prepare, validate and commit exactly one due tick. Publish a page when
    /// one is free; otherwise the next publication becomes a full baseline.
    /// Game rejection leaves world, tick and published endpoints unchanged.
    ///
    /// # Errors
    ///
    /// Missing tick-zero baseline, tick overflow or game validation refusal.
    ///
    /// # Panics
    ///
    /// Panics only if the reserved page becomes unavailable inside the
    /// exclusive tick, or a game apply/capture callback panics.
    pub fn step(
        &mut self,
        game: &mut G,
        candidate: &mut G::Candidate,
    ) -> Result<u64, TickError<G::Error>>
    where
        G: Sync,
    {
        self.step_inner(game, candidate, None, PublicationMode::Capture)
    }

    /// Commit one tick and retain moved preimages for a prediction branch.
    ///
    /// # Errors
    ///
    /// The undo record is foreign or ordinary tick validation fails.
    pub fn step_recording(
        &mut self,
        game: &mut G,
        candidate: &mut G::Candidate,
        undo: &mut RuntimeUndo,
        publication: PublicationMode,
    ) -> Result<u64, TickError<G::Error>>
    where
        G: Sync,
    {
        if undo.nonce != self.nonce {
            return Err(TickError::Edit(EditError::WrongRuntime));
        }
        self.step_inner(game, candidate, Some(undo), publication)
    }

    fn step_inner(
        &mut self,
        game: &mut G,
        candidate: &mut G::Candidate,
        mut undo: Option<&mut RuntimeUndo>,
        publication: PublicationMode,
    ) -> Result<u64, TickError<G::Error>>
    where
        G: Sync,
    {
        let next = self.tick.checked_add(1).ok_or(TickError::TickExhausted)?;
        let following_revision = self
            .next_revision
            .checked_add(1)
            .ok_or(TickError::RevisionExhausted)?;
        if !self.initial_published {
            return Err(TickError::BaselineRequired);
        }
        self.direct_dirty.clear();
        let txn_undo = match undo.as_deref_mut() {
            Some(record) => &mut record.world,
            None => &mut self.abort_undo,
        };
        txn_undo.clear();
        game.reset_candidate(candidate);
        let preparation = catch_unwind(AssertUnwindSafe(|| -> Result<(), TickError<G::Error>> {
            for wave in &mut self.waves {
                #[cfg(not(target_arch = "wasm32"))]
                // Two tiny input callbacks stay on the tick caller to avoid pool scheduling.
                if self.executor.mode != ExecutionMode::Serial
                    && (wave.len() > 3 || self.executor.mode == ExecutionMode::RayonForced)
                    && wave
                        .iter()
                        .all(|bound| matches!(&bound.callback, Callback::Parallel(_)))
                {
                    let world = &self.world;
                    let nonce = self.nonce;
                    let executor = self.executor;
                    let game_ref: &G = game;
                    wave.par_iter_mut().for_each(|bound| {
                        let Callback::Parallel(callback) = &bound.callback else {
                            unreachable!("wave checked parallel")
                        };
                        game_ref.reset_parallel_output(&mut bound.output);
                        bound.result = Some(callback(
                            game_ref,
                            &WorldRead {
                                world,
                                nonce,
                                system: Some(bound.id),
                            },
                            executor,
                            next,
                            &mut bound.output,
                        ));
                    });
                    for bound in wave {
                        bound
                            .result
                            .take()
                            .expect("callback ran")
                            .map_err(TickError::Game)?;
                        let before = game.edits(candidate).len();
                        game.merge_parallel(bound.id, &mut bound.output, candidate, next);
                        check_edit_system(game.edits(candidate), before, bound.id)?;
                    }
                    continue;
                }
                for bound in wave {
                    let before = game.edits(candidate).len();
                    match &bound.callback {
                        Callback::Serial(callback) => callback(
                            game,
                            &WorldRead {
                                world: &self.world,
                                nonce: self.nonce,
                                system: Some(bound.id),
                            },
                            self.executor,
                            next,
                            candidate,
                        ),
                        Callback::Parallel(callback) => {
                            game.reset_parallel_output(&mut bound.output);
                            callback(
                                game,
                                &WorldRead {
                                    world: &self.world,
                                    nonce: self.nonce,
                                    system: Some(bound.id),
                                },
                                self.executor,
                                next,
                                &mut bound.output,
                            )
                        }
                        Callback::Direct(callback) => callback(
                            game,
                            &mut WorldTxn {
                                world: &mut self.world,
                                undo: txn_undo,
                                dirty: &mut self.direct_dirty,
                                nonce: self.nonce,
                                system: bound.id,
                            },
                            self.executor,
                            next,
                            candidate,
                        ),
                    }
                    .map_err(TickError::Game)?;
                    if matches!(&bound.callback, Callback::Parallel(_)) {
                        game.merge_parallel(bound.id, &mut bound.output, candidate, next);
                    }
                    check_edit_system(game.edits(candidate), before, bound.id)?;
                }
            }
            let edits = game.edits(candidate);
            self.edit_scratch
                .validate(&self.world, self.nonce, edits, candidate)
                .map_err(TickError::Edit)?;
            Ok(())
        }));
        match preparation {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                txn_undo.abort_tentative(&mut self.world);
                self.direct_dirty.clear();
                return Err(error);
            }
            Err(payload) => {
                txn_undo.abort_tentative(&mut self.world);
                self.direct_dirty.clear();
                resume_unwind(payload);
            }
        }
        let edits = game.edits(candidate);
        if let Some(record) = undo.as_deref_mut() {
            record.events.reserve(edits.len());
        }
        self.edit_scratch.apply(
            &mut self.world,
            edits,
            candidate,
            next,
            &mut self.structural_journal,
            &self.direct_dirty.slots,
            undo.as_deref_mut().map(|record| &mut record.world),
        );
        self.direct_dirty.clear();
        game.apply(
            &WorldRead {
                world: &self.world,
                nonce: self.nonce,
                system: None,
            },
            next,
            candidate,
        );
        if let Some(record) = undo {
            record.events.clear();
            record.events.extend_from_slice(&self.structural_journal);
            record.from = self.tick;
            record.to = next;
            record.from_revision = self.revision;
            record.to_revision = self.next_revision;
            record.spent = false;
        }
        self.tick = next;
        self.rewind_patch_target = None;
        self.delta_from_revision = self.revision;
        self.revision = self.next_revision;
        self.next_revision = following_revision;
        self.delta_ready = true;
        let world = WorldRead {
            world: &self.world,
            nonce: self.nonce,
            system: None,
        };
        if publication == PublicationMode::Defer || !self.pages.can_capture() {
            self.publication_skipped = true;
            return Ok(next);
        }
        self.pages
            .capture(self.publication_skipped, |page, first| {
                game.capture(
                    &world,
                    candidate,
                    &self.structural_journal,
                    page,
                    first,
                    next,
                    self.timeline,
                );
            })
            .expect("unleased page remained available during exclusive tick");
        self.publication_skipped = false;
        Ok(next)
    }
}

/// Checked immutable access to committed game components.
pub struct WorldRead<'a> {
    world: &'a RegisteredWorld,
    nonce: u64,
    system: Option<SystemId>,
}

/// Tentative typed writes for one declared system. The runtime retains
/// preimages and publishes change ticks only after the whole tick succeeds.
pub struct WorldTxn<'a> {
    world: &'a mut RegisteredWorld,
    undo: &'a mut WorldUndo,
    dirty: &'a mut DirectDirty,
    nonce: u64,
    system: SystemId,
}

/// Prepared storage-order access to distinct `&mut T` and `&U` columns.
pub struct WriteRead2<T: Clone + Send + Sync + 'static, U: Send + Sync + 'static> {
    first: usize,
    second: usize,
    nonce: u64,
    system: SystemId,
    marker: PhantomData<(T, U)>,
}

impl<T: Clone + Send + Sync + 'static, U: Send + Sync + 'static> Copy for WriteRead2<T, U> {}

impl<T: Clone + Send + Sync + 'static, U: Send + Sync + 'static> Clone for WriteRead2<T, U> {
    fn clone(&self) -> Self {
        *self
    }
}

impl WorldTxn<'_> {
    /// Visit matching rows in deterministic slot order. Return `true` for a
    /// changed value and `false` for an unchanged value. A callback refusal
    /// restores all tentative writes before returning.
    ///
    /// # Errors
    ///
    /// The game callback refuses one row.
    ///
    /// # Panics
    ///
    /// The query belongs to a different runtime or system callback.
    pub fn visit<T: Clone + Send + Sync + 'static, U: Send + Sync + 'static, E>(
        &mut self,
        query: &WriteRead2<T, U>,
        visit: impl FnMut(Handle, StableId, &mut T, &U) -> Result<bool, E>,
    ) -> Result<(), E> {
        assert_eq!(query.nonce, self.nonce, "query belongs to another runtime");
        assert_eq!(query.system, self.system, "query belongs to another system");
        self.world
            .visit_mut2(query.first, query.second, self.undo, self.dirty, visit)
    }
}

/// Checked live slot borrowed from one immutable authority view.
#[derive(Clone, Copy)]
pub struct LiveSlot<'a> {
    handle: Handle,
    id: StableId,
    nonce: u64,
    guard: PhantomData<&'a RegisteredWorld>,
}

impl LiveSlot<'_> {
    /// Current checked handle.
    pub fn handle(self) -> Handle {
        self.handle
    }

    /// Stable identity at this live slot.
    pub fn stable_id(self) -> StableId {
        self.id
    }
}

/// Read-only registered column bound once to a checked authority view.
pub struct ColumnRead<'a, T: Send + Sync + 'static> {
    column: &'a TypedStore<T>,
    nonce: u64,
}

impl<T: Send + Sync + 'static> ColumnRead<'_, T> {
    /// Read a component at a live slot from the same runtime.
    ///
    /// # Errors
    ///
    /// The slot belongs to another runtime.
    pub fn get(&self, slot: LiveSlot<'_>) -> Result<Option<&T>, RegisteredWorldError> {
        if slot.nonce != self.nonce {
            return Err(RegisteredWorldError::WrongRuntime);
        }
        Ok(self.column.get(slot.handle.slot as usize))
    }
}

impl WorldRead<'_> {
    /// Whether a page's changed-slot interval is still in the bounded journal.
    pub fn has_changes_since(&self, since: u64, current: u64) -> bool {
        self.world.has_changes_since(since, current)
    }

    /// Visit the union of changed slots once each, in physical slot order.
    /// Returns false and visits nothing if the interval is no longer retained.
    pub fn visit_changes_since(&self, since: u64, current: u64, visit: impl FnMut(u32)) -> bool {
        self.world.visit_changes_since(since, current, visit)
    }

    /// Visit changed slots with a live row and three component stamps. Permit
    /// checks happen once before the scan rather than once per changed row.
    /// Returns false without visiting when the interval has expired.
    ///
    /// # Panics
    ///
    /// A permit belongs to another runtime or system callback.
    pub fn visit_changes_with_ticks3<
        A: Send + Sync + 'static,
        B: Send + Sync + 'static,
        C: Send + Sync + 'static,
    >(
        &self,
        since: u64,
        current: u64,
        permits: (&ReadPermit<A>, &ReadPermit<B>, &ReadPermit<C>),
        mut visit: impl FnMut(u32, Option<(LiveSlot<'_>, [u64; 4])>),
    ) -> bool {
        for permit in [permits.0.nonce, permits.1.nonce, permits.2.nonce] {
            assert_eq!(
                permit, self.nonce,
                "change permit belongs to another runtime"
            );
        }
        assert!(self.system.is_none_or(|system| {
            system == permits.0.system && system == permits.1.system && system == permits.2.system
        }));
        let components = [
            permits.0.component,
            permits.1.component,
            permits.2.component,
        ];
        self.world.visit_changes_since(since, current, |key| {
            let live = self.world.live_at_slot(key).map(|(handle, id)| {
                (
                    LiveSlot {
                        handle,
                        id,
                        nonce: self.nonce,
                        guard: PhantomData,
                    },
                    self.world.change_ticks3_at(key, components),
                )
            });
            visit(key, live);
        })
    }

    /// Visit slots with structural changes in a retained interval once each,
    /// in physical slot order. Returns false if the interval is not retained.
    pub fn visit_structural_since(&self, since: u64, current: u64, visit: impl FnMut(u32)) -> bool {
        self.world.visit_structural_since(since, current, visit)
    }

    /// Whether every earlier changed slot in a retained interval also changed
    /// in its last tick. This lets a game refresh a leased page from its
    /// current validated edit batch when that batch covers the whole interval.
    /// Structural slots may be omitted if the caller refreshes them separately.
    pub fn prior_changes_covered_by_current(
        &self,
        since: u64,
        current: u64,
        ignore_structural: bool,
    ) -> bool {
        self.world
            .prior_changes_covered_by_current(since, current, ignore_structural)
    }

    /// Whether a retained interval contains only writes to one declared
    /// component, optionally including structural edits.
    ///
    /// # Errors
    ///
    /// The read permit belongs to another runtime or callback.
    pub fn only_component_changes_since<T: Send + Sync + 'static>(
        &self,
        permit: &ReadPermit<T>,
        since: u64,
        current: u64,
        include_structural: bool,
    ) -> Result<bool, RegisteredWorldError> {
        if permit.nonce != self.nonce {
            return Err(RegisteredWorldError::WrongRuntime);
        }
        if self.system.is_some_and(|system| system != permit.system) {
            return Err(RegisteredWorldError::WrongSystem);
        }
        Ok(self.world.only_component_changes_since(
            since,
            current,
            permit.component,
            include_structural,
        ))
    }

    /// Resolve one live physical slot without exposing its storage row.
    pub fn live_at_slot(&self, slot: u32) -> Option<LiveSlot<'_>> {
        let (handle, id) = self.world.live_at_slot(slot)?;
        Some(LiveSlot {
            handle,
            id,
            nonce: self.nonce,
            guard: PhantomData,
        })
    }

    /// Bind one declared component column for repeated checked reads.
    ///
    /// # Errors
    ///
    /// The permit belongs to another runtime or callback.
    pub fn column<T: Send + Sync + 'static>(
        &self,
        permit: &ReadPermit<T>,
    ) -> Result<ColumnRead<'_, T>, RegisteredWorldError> {
        if permit.nonce != self.nonce {
            return Err(RegisteredWorldError::WrongRuntime);
        }
        if self.system.is_some_and(|system| system != permit.system) {
            return Err(RegisteredWorldError::WrongSystem);
        }
        Ok(ColumnRead {
            column: self.world.column_at(permit.component),
            nonce: self.nonce,
        })
    }

    /// Resolve a stable identity at this committed endpoint.
    pub fn handle(&self, id: StableId) -> Option<Handle> {
        self.world.handle(id)
    }

    /// Resolve one checked handle to its stable identity.
    ///
    /// # Errors
    ///
    /// The handle is stale or from another world epoch.
    pub fn stable_id(&self, handle: Handle) -> Result<StableId, RegisteredWorldError> {
        self.world.stable_id(handle)
    }

    /// Read a registered component through a checked handle.
    ///
    /// # Errors
    ///
    /// The handle is stale, the component is absent or its type is unregistered.
    pub fn with<T: Send + Sync + 'static, R>(
        &self,
        permit: &ReadPermit<T>,
        handle: Handle,
        read: impl FnOnce(&T) -> R,
    ) -> Result<R, RegisteredWorldError> {
        if permit.nonce != self.nonce {
            return Err(RegisteredWorldError::WrongRuntime);
        }
        if self.system.is_some_and(|system| system != permit.system) {
            return Err(RegisteredWorldError::WrongSystem);
        }
        self.world.with_index(permit.component, handle, read)
    }
}

/// Compile-checked permission to read one registered component.
pub struct ReadPermit<T: Send + Sync + 'static> {
    marker: PhantomData<T>,
    nonce: u64,
    system: SystemId,
    component: usize,
}

/// Reusable storage-order read of two registered components.
pub struct Read2<T: Send + Sync + 'static, U: Send + Sync + 'static> {
    query: PreparedRead2<T, U>,
    nonce: u64,
    system: SystemId,
}

impl<T: Send + Sync + 'static, U: Send + Sync + 'static> Read2<T, U> {
    /// Visit matching entities with checked handles and stable identities.
    ///
    /// # Panics
    ///
    /// The query was prepared for another runtime instance.
    pub fn visit(&mut self, world: &WorldRead<'_>, visit: impl FnMut(Handle, StableId, &T, &U)) {
        assert_eq!(self.nonce, world.nonce, "query belongs to another runtime");
        assert!(world.system.is_none_or(|system| system == self.system));
        self.query.visit(world.world, visit);
    }

    /// Visit query rows with three registered change stamps and the structural
    /// stamp. Permits are checked once before entering storage-order traversal.
    ///
    /// # Panics
    ///
    /// A query or permit belongs to another runtime or system callback.
    pub fn visit_with_changes<
        A: Send + Sync + 'static,
        B: Send + Sync + 'static,
        C: Send + Sync + 'static,
    >(
        &mut self,
        world: &WorldRead<'_>,
        permits: (&ReadPermit<A>, &ReadPermit<B>, &ReadPermit<C>),
        mut visit: impl FnMut(Handle, StableId, &T, &U, [u64; 4]),
    ) {
        assert_eq!(self.nonce, world.nonce, "query belongs to another runtime");
        assert!(world.system.is_none_or(|system| system == self.system));
        for permit in [permits.0.nonce, permits.1.nonce, permits.2.nonce] {
            assert_eq!(
                permit, world.nonce,
                "change permit belongs to another runtime"
            );
        }
        assert!(world.system.is_none_or(|system| {
            system == permits.0.system && system == permits.1.system && system == permits.2.system
        }));
        let components = [
            permits.0.component,
            permits.1.component,
            permits.2.component,
        ];
        self.query
            .visit_rows(world.world, |handle, id, a, b, slot| {
                let stamps = world.world.change_ticks3_at(slot, components);
                visit(handle, id, a, b, stamps);
            });
    }

    /// Visit matching entities in stable physical-slot order. Use this for
    /// order-sensitive event emission or accumulation that must replay exactly.
    ///
    /// # Panics
    ///
    /// The query was prepared for another runtime instance.
    pub fn visit_canonical(
        &self,
        world: &WorldRead<'_>,
        visit: impl FnMut(Handle, StableId, &T, &U),
    ) {
        assert_eq!(self.nonce, world.nonce, "query belongs to another runtime");
        assert!(world.system.is_none_or(|system| system == self.system));
        world.world.visit_canonical(visit);
    }
}

/// Compile-checked permission to edit one registered component.
pub struct WritePermit<T: Send + Sync + 'static> {
    marker: PhantomData<T>,
    pub(super) nonce: u64,
    pub(super) system: SystemId,
    pub(super) component: usize,
}

/// Compile-checked permission to spawn or despawn under one external resource.
pub struct StructuralPermit {
    resource: u64,
    pub(super) nonce: u64,
    pub(super) system: SystemId,
}

impl StructuralPermit {
    /// Owner-defined structural resource identity used at schedule closure.
    pub fn resource(&self) -> u64 {
        self.resource
    }
}
