//! Generic fixed-tick authority boundary. Games own component types, candidate
//! work and rules; the engine owns registered storage and complete publication.

use std::{
    collections::HashMap,
    marker::PhantomData,
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
        EntityWriter, PreparedRead2, RegisteredWorld, RegisteredWorldError, WorldCapture,
        WorldDelta,
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

/// Worker selection for one deterministic fixed-tick candidate. Platform
/// policy can select a pool for its core classes without changing game rules.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionMode {
    /// Run each job on the tick caller.
    Serial,
    /// Run independent jobs through the native Rayon pool.
    Rayon,
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
    pub fn for_each_mut<T: Send>(self, jobs: &mut [T], run: impl Fn(&mut T) + Send + Sync) {
        #[cfg(not(target_arch = "wasm32"))]
        if self.mode == ExecutionMode::Rayon {
            jobs.par_iter_mut().for_each(run);
            return;
        }
        jobs.iter_mut().for_each(run);
    }
}

impl RuntimeCapture {
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

enum Callback<P, G: TickGame<P>> {
    Serial(SystemCallback<P, G>),
    Parallel(ParallelCallback<P, G>),
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

/// One engine-owned registered world, compiled schedule and leased page ring.
/// The concrete game supplies no engine-side component or action type.
pub struct GameRuntime<P, G: TickGame<P>> {
    world: RegisteredWorld,
    schedule: Schedule,
    waves: Vec<Vec<BoundSystem<P, G>>>,
    pages: SnapshotRing<P>,
    publication_skipped: bool,
    tick: u64,
    timeline: u64,
    revision: u64,
    next_revision: u64,
    delta_from_revision: u64,
    delta_ready: bool,
    executor: Executor,
    nonce: u64,
    edit_scratch: EditScratch,
    structural_journal: Vec<StructuralChange>,
}

impl<P, G: TickGame<P>> GameRuntime<P, G> {
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
        self.world.begin_tick();
        self.structural_journal.clear();
        self.pages.reset_timeline();
        if !self.pages.can_capture() {
            self.pages = SnapshotRing::new(make_page);
        }
        self.publication_skipped = false;
        Ok(())
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
        Ok(Self {
            world,
            schedule,
            waves,
            pages: SnapshotRing::new(make_page),
            publication_skipped: false,
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
        if self.tick != 0 || self.pages.latest().is_some() {
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
        if self.pages.latest().is_none() {
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
        let next = self.tick.checked_add(1).ok_or(TickError::TickExhausted)?;
        let following_revision = self
            .next_revision
            .checked_add(1)
            .ok_or(TickError::RevisionExhausted)?;
        if self.pages.latest().is_none() {
            return Err(TickError::BaselineRequired);
        }
        game.reset_candidate(candidate);
        for wave in &mut self.waves {
            #[cfg(not(target_arch = "wasm32"))]
            if self.executor.mode == ExecutionMode::Rayon
                && wave.len() > 1
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
                let world = WorldRead {
                    world: &self.world,
                    nonce: self.nonce,
                    system: Some(bound.id),
                };
                match &bound.callback {
                    Callback::Serial(callback) => {
                        callback(game, &world, self.executor, next, candidate)
                    }
                    Callback::Parallel(callback) => {
                        game.reset_parallel_output(&mut bound.output);
                        callback(game, &world, self.executor, next, &mut bound.output)
                    }
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
        self.edit_scratch.apply(
            &mut self.world,
            edits,
            candidate,
            next,
            &mut self.structural_journal,
        );
        game.apply(
            &WorldRead {
                world: &self.world,
                nonce: self.nonce,
                system: None,
            },
            next,
            candidate,
        );
        self.tick = next;
        self.delta_from_revision = self.revision;
        self.revision = self.next_revision;
        self.next_revision = following_revision;
        self.delta_ready = true;
        let world = WorldRead {
            world: &self.world,
            nonce: self.nonce,
            system: None,
        };
        if !self.pages.can_capture() {
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

    /// Visit changed slots in tick order. A slot may appear in several ticks.
    /// Returns false and visits nothing if the interval is no longer retained.
    pub fn visit_changes_since(&self, since: u64, current: u64, visit: impl FnMut(u32)) -> bool {
        self.world.visit_changes_since(since, current, visit)
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

    /// Read structural and three registered component change ticks at a live slot.
    ///
    /// # Errors
    ///
    /// The slot belongs to another runtime.
    ///
    /// # Panics
    ///
    /// A permit belongs to another runtime or system callback.
    pub fn change_ticks3<
        A: Send + Sync + 'static,
        B: Send + Sync + 'static,
        C: Send + Sync + 'static,
    >(
        &self,
        slot: LiveSlot<'_>,
        permits: (&ReadPermit<A>, &ReadPermit<B>, &ReadPermit<C>),
    ) -> Result<[u64; 4], RegisteredWorldError> {
        for permit in [permits.0.nonce, permits.1.nonce, permits.2.nonce] {
            assert_eq!(
                permit, self.nonce,
                "change permit belongs to another runtime"
            );
        }
        assert!(self.system.is_none_or(|system| {
            system == permits.0.system && system == permits.1.system && system == permits.2.system
        }));
        if slot.nonce != self.nonce {
            return Err(RegisteredWorldError::WrongRuntime);
        }
        Ok(self.world.change_ticks3_at(
            slot.handle.slot,
            [
                permits.0.component,
                permits.1.component,
                permits.2.component,
            ],
        ))
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
