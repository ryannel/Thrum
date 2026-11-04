//! Generic fixed-tick authority boundary. Games own component types, candidate
//! work and rules; the engine owns registered storage and complete publication.

use std::{
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
    AccessMode, ComponentKey, ComponentRegistry, Schedule, ScheduleError, SnapshotRing, SystemId,
    SystemSpec,
    registered_world::{
        EntityWriter, PreparedRead2, PreparedWrite1, RegisteredWorld, RegisteredWorldError,
        WorldCapture,
    },
};

/// Cold-path registered state capture for replay. Component values remain
/// opaque to the engine and are cloned only when this capture is requested.
pub struct RuntimeCapture {
    world: WorldCapture,
    tick: u64,
    nonce: u64,
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
    /// Committed tick represented by this capture.
    pub fn tick(&self) -> u64 {
        self.tick
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
    /// Tick-zero seed membership must be published before ticking.
    BaselineRequired,
    /// All preallocated publication pages are leased.
    PublicationBusy,
    /// Game-owned validation refused the candidate before any commit.
    Game(E),
}

/// Game-owned rules for one registered authority. Preparation receives only
/// immutable game state and mutable candidate scratch. `apply` and `capture`
/// run only after complete candidate validation and must be infallible.
pub trait TickGame<P> {
    /// Expected game-rule or input error.
    type Error;
    /// Reusable game-owned candidate and query scratch, never committed state.
    type Candidate;

    /// Read committed components and prepare/validate one candidate tick.
    ///
    /// # Errors
    ///
    /// Refuses invalid input or candidate state without mutating the world.
    fn prepare(
        &self,
        world: &WorldRead<'_>,
        schedule: &Schedule,
        executor: Executor,
        tick: u64,
        candidate: &mut Self::Candidate,
    ) -> Result<(), Self::Error>;

    /// Apply the fully validated candidate. All target handles and registered
    /// component presence must have been checked during `prepare`.
    fn apply(&mut self, world: &mut WorldCommit<'_>, tick: u64, candidate: &Self::Candidate);

    /// Copy one complete committed endpoint into an unleased reusable page.
    fn capture(&mut self, world: &WorldRead<'_>, page: &mut P, first: bool, tick: u64);
}

/// One engine-owned registered world, compiled schedule and leased page ring.
/// The concrete game supplies no engine-side component or action type.
pub struct GameRuntime<P> {
    world: RegisteredWorld,
    schedule: Schedule,
    pages: SnapshotRing<P>,
    tick: u64,
    executor: Executor,
    nonce: u64,
}

impl<P> GameRuntime<P> {
    /// Capture registered components, identities, slot generations and free
    /// order after a complete committed tick. This is a cold replay path.
    pub fn capture(&self) -> RuntimeCapture {
        RuntimeCapture {
            world: self.world.capture(),
            tick: self.tick,
            nonce: self.nonce,
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
        self.world.restore(&capture.world)?;
        self.tick = capture.tick;
        self.pages = SnapshotRing::new(make_page);
        Ok(())
    }

    /// Compile declarations and allocate the world and four complete pages.
    ///
    /// # Errors
    ///
    /// Capacity, registration or schedule validation failed.
    pub fn new(
        epoch: WorldEpoch,
        capacity: usize,
        registry: ComponentRegistry,
        systems: Vec<SystemSpec>,
        make_page: impl FnMut() -> P,
    ) -> Result<Self, RuntimeError> {
        let schedule = Schedule::new(&registry, systems).map_err(RuntimeError::Schedule)?;
        let world = RegisteredWorld::new(epoch, capacity, registry)?;
        let nonce = NEXT_RUNTIME
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                current.checked_add(1)
            })
            .map_err(|_| RuntimeError::RuntimeExhausted)?;
        Ok(Self {
            world,
            schedule,
            pages: SnapshotRing::new(make_page),
            tick: 0,
            executor: Executor {
                mode: ExecutionMode::Serial,
            },
            nonce,
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
        Ok(ReadPermit {
            marker: PhantomData,
            nonce: self.nonce,
        })
    }

    /// Prepare a storage-order write cursor once for a declared system.
    ///
    /// # Errors
    ///
    /// The system lacks write access or the type is unregistered.
    pub fn write1<T: Send + Sync + 'static>(
        &self,
        system: SystemId,
    ) -> Result<Write1<T>, RuntimeError> {
        if !self
            .schedule
            .permits(system, ComponentKey::of::<T>(), AccessMode::Write)
        {
            return Err(RuntimeError::UndeclaredAccess);
        }
        Ok(Write1 {
            query: self.world.write1::<T>()?,
            nonce: self.nonce,
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
        Ok(WritePermit {
            marker: PhantomData,
            nonce: self.nonce,
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
    pub fn publish_initial<G: TickGame<P>>(&mut self, game: &mut G) {
        if self.pages.latest().is_none() {
            let world = WorldRead {
                world: &self.world,
                nonce: self.nonce,
            };
            self.pages
                .capture(|page, first| game.capture(&world, page, first, self.tick))
                .expect("unpublished ring has a free page");
        }
    }

    /// Prepare, validate and commit exactly one due tick, then seal one page.
    /// Game rejection leaves world, tick and published endpoints unchanged.
    ///
    /// # Errors
    ///
    /// Missing tick-zero baseline, tick overflow, publication pressure or game
    /// validation refusal.
    ///
    /// # Panics
    ///
    /// Panics only if the reserved page becomes unavailable inside the
    /// exclusive tick, or a game apply/capture callback panics.
    pub fn step<G: TickGame<P>>(
        &mut self,
        game: &mut G,
        candidate: &mut G::Candidate,
    ) -> Result<u64, TickError<G::Error>> {
        let next = self.tick.checked_add(1).ok_or(TickError::TickExhausted)?;
        if self.pages.latest().is_none() {
            return Err(TickError::BaselineRequired);
        }
        if !self.pages.can_capture() {
            return Err(TickError::PublicationBusy);
        }
        game.prepare(&self.read(), &self.schedule, self.executor, next, candidate)
            .map_err(TickError::Game)?;
        game.apply(
            &mut WorldCommit {
                world: &mut self.world,
                nonce: self.nonce,
            },
            next,
            candidate,
        );
        self.tick = next;
        let world = WorldRead {
            world: &self.world,
            nonce: self.nonce,
        };
        self.pages
            .capture(|page, first| game.capture(&world, page, first, next))
            .expect("capture reserved before candidate commit");
        Ok(next)
    }
}

/// Checked immutable access to committed game components.
pub struct WorldRead<'a> {
    world: &'a RegisteredWorld,
    nonce: u64,
}

impl WorldRead<'_> {
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
        self.world.with(handle, read)
    }
}

/// Compile-checked permission to read one registered component.
pub struct ReadPermit<T: Send + Sync + 'static> {
    marker: PhantomData<T>,
    nonce: u64,
}

/// Reusable storage-order read of two registered components.
pub struct Read2<T: Send + Sync + 'static, U: Send + Sync + 'static> {
    query: PreparedRead2<T, U>,
    nonce: u64,
}

impl<T: Send + Sync + 'static, U: Send + Sync + 'static> Read2<T, U> {
    /// Visit matching entities with checked handles and stable identities.
    ///
    /// # Panics
    ///
    /// The query was prepared for another runtime instance.
    pub fn visit(&mut self, world: &WorldRead<'_>, visit: impl FnMut(Handle, StableId, &T, &U)) {
        assert_eq!(self.nonce, world.nonce, "query belongs to another runtime");
        self.query.visit(world.world, visit);
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
        world.world.visit_canonical(visit);
    }
}

/// Reusable storage-order write of one registered component.
pub struct Write1<T: Send + Sync + 'static> {
    query: PreparedWrite1<T>,
    nonce: u64,
}

impl<T: Send + Sync + 'static> Write1<T> {
    /// Apply previously validated edits to matching entities.
    ///
    /// # Panics
    ///
    /// The query was prepared for another runtime instance.
    pub fn visit(
        &mut self,
        world: &mut WorldCommit<'_>,
        visit: impl FnMut(Handle, StableId, &mut T),
    ) {
        assert_eq!(self.nonce, world.nonce, "query belongs to another runtime");
        self.query.visit(world.world, visit);
    }
}

/// Compile-checked permission to edit one registered component.
pub struct WritePermit<T: Send + Sync + 'static> {
    marker: PhantomData<T>,
    nonce: u64,
}

/// Compile-checked permission to spawn or despawn under one external resource.
pub struct StructuralPermit {
    resource: u64,
    nonce: u64,
}

impl StructuralPermit {
    /// Owner-defined structural resource identity used at schedule closure.
    pub fn resource(&self) -> u64 {
        self.resource
    }
}

/// Commit-only typed world access; never given to game preparation callbacks.
pub struct WorldCommit<'a> {
    world: &'a mut RegisteredWorld,
    nonce: u64,
}

impl WorldCommit<'_> {
    /// Replace one present component through a checked handle.
    ///
    /// # Errors
    ///
    /// Stale identity, missing component or unregistered type. Game code must
    /// prevalidate before calling this infallible commit phase.
    pub fn replace<T: Send + Sync + 'static>(
        &mut self,
        permit: &WritePermit<T>,
        handle: Handle,
        value: T,
    ) -> Result<T, RegisteredWorldError> {
        if permit.nonce != self.nonce {
            return Err(RegisteredWorldError::WrongRuntime);
        }
        self.world.replace(handle, value)
    }

    /// Add a registered component to one checked entity.
    ///
    /// # Errors
    ///
    /// Stale identity, duplicate component or unregistered type.
    pub fn insert<T: Send + Sync + 'static>(
        &mut self,
        permit: &WritePermit<T>,
        handle: Handle,
        value: T,
    ) -> Result<(), RegisteredWorldError> {
        if permit.nonce != self.nonce {
            return Err(RegisteredWorldError::WrongRuntime);
        }
        self.world.insert(handle, value)
    }

    /// Remove one registered component from a checked entity.
    ///
    /// # Errors
    ///
    /// Stale identity, missing component or unregistered type.
    pub fn remove<T: Send + Sync + 'static>(
        &mut self,
        permit: &WritePermit<T>,
        handle: Handle,
    ) -> Result<T, RegisteredWorldError> {
        if permit.nonce != self.nonce {
            return Err(RegisteredWorldError::WrongRuntime);
        }
        self.world.remove(handle)
    }

    /// Spawn a fully validated registered bundle.
    ///
    /// # Errors
    ///
    /// Duplicate identity, capacity or invalid bundle.
    pub fn spawn(
        &mut self,
        permit: &StructuralPermit,
        id: StableId,
        fill: impl FnOnce(&mut EntityWriter<'_>) -> Result<(), RegisteredWorldError>,
    ) -> Result<Handle, RegisteredWorldError> {
        if permit.nonce != self.nonce {
            return Err(RegisteredWorldError::WrongRuntime);
        }
        self.world.spawn(id, fill)
    }

    /// Despawn a checked entity and retire its handle generation.
    ///
    /// # Errors
    ///
    /// The handle is stale or belongs to another world.
    pub fn despawn(
        &mut self,
        permit: &StructuralPermit,
        handle: Handle,
    ) -> Result<(), RegisteredWorldError> {
        if permit.nonce != self.nonce {
            return Err(RegisteredWorldError::WrongRuntime);
        }
        self.world.despawn(handle)
    }
}
