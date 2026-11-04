//! Generic fixed-tick authority boundary. Games own component types, candidate
//! work and rules; the engine owns registered storage and complete publication.

use std::{marker::PhantomData, sync::Arc};

use crate::{Handle, StableId, WorldEpoch};

use super::{
    AccessMode, ComponentKey, ComponentRegistry, Schedule, ScheduleError, SnapshotRing, SystemId,
    SystemSpec,
    registered_world::{
        EntityWriter, PreparedRead2, PreparedWrite1, RegisteredWorld, RegisteredWorldError,
    },
};

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
}

impl<P> GameRuntime<P> {
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
        Ok(Self {
            world,
            schedule,
            pages: SnapshotRing::new(make_page),
            tick: 0,
        })
    }

    /// Last complete committed tick.
    pub fn tick(&self) -> u64 {
        self.tick
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
        WorldRead { world: &self.world }
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
        Ok(StructuralPermit { resource })
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

    /// Seal tick zero before any mutation, preserving seeded structural state.
    ///
    /// # Panics
    ///
    /// Panics only if an unpublished ring has lost all four private pages or
    /// the game capture callback panics.
    pub fn publish_initial<G: TickGame<P>>(&mut self, game: &mut G) {
        if self.pages.latest().is_none() {
            let world = WorldRead { world: &self.world };
            self.pages
                .capture(|page, first| game.capture(&world, page, first, 0))
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
        game.prepare(&self.read(), &self.schedule, next, candidate)
            .map_err(TickError::Game)?;
        game.apply(
            &mut WorldCommit {
                world: &mut self.world,
            },
            next,
            candidate,
        );
        self.tick = next;
        let world = WorldRead { world: &self.world };
        self.pages
            .capture(|page, first| game.capture(&world, page, first, next))
            .expect("capture reserved before candidate commit");
        Ok(next)
    }
}

/// Checked immutable access to committed game components.
pub struct WorldRead<'a> {
    world: &'a RegisteredWorld,
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
        _permit: &ReadPermit<T>,
        handle: Handle,
        read: impl FnOnce(&T) -> R,
    ) -> Result<R, RegisteredWorldError> {
        self.world.with(handle, read)
    }
}

/// Compile-checked permission to read one registered component.
pub struct ReadPermit<T: Send + Sync + 'static> {
    marker: PhantomData<T>,
}

/// Reusable storage-order read of two registered components.
pub struct Read2<T: Send + Sync + 'static, U: Send + Sync + 'static> {
    query: PreparedRead2<T, U>,
}

impl<T: Send + Sync + 'static, U: Send + Sync + 'static> Read2<T, U> {
    /// Visit matching entities with checked handles and stable identities.
    pub fn visit(&mut self, world: &WorldRead<'_>, visit: impl FnMut(Handle, StableId, &T, &U)) {
        self.query.visit(world.world, visit);
    }
}

/// Reusable storage-order write of one registered component.
pub struct Write1<T: Send + Sync + 'static> {
    query: PreparedWrite1<T>,
}

impl<T: Send + Sync + 'static> Write1<T> {
    /// Apply previously validated edits to matching entities.
    pub fn visit(
        &mut self,
        world: &mut WorldCommit<'_>,
        visit: impl FnMut(Handle, StableId, &mut T),
    ) {
        self.query.visit(world.world, visit);
    }
}

/// Compile-checked permission to edit one registered component.
pub struct WritePermit<T: Send + Sync + 'static> {
    marker: PhantomData<T>,
}

/// Compile-checked permission to spawn or despawn under one external resource.
pub struct StructuralPermit {
    resource: u64,
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
        _permit: &WritePermit<T>,
        handle: Handle,
        value: T,
    ) -> Result<T, RegisteredWorldError> {
        self.world.replace(handle, value)
    }

    /// Add a registered component to one checked entity.
    ///
    /// # Errors
    ///
    /// Stale identity, duplicate component or unregistered type.
    pub fn insert<T: Send + Sync + 'static>(
        &mut self,
        _permit: &WritePermit<T>,
        handle: Handle,
        value: T,
    ) -> Result<(), RegisteredWorldError> {
        self.world.insert(handle, value)
    }

    /// Remove one registered component from a checked entity.
    ///
    /// # Errors
    ///
    /// Stale identity, missing component or unregistered type.
    pub fn remove<T: Send + Sync + 'static>(
        &mut self,
        _permit: &WritePermit<T>,
        handle: Handle,
    ) -> Result<T, RegisteredWorldError> {
        self.world.remove(handle)
    }

    /// Spawn a fully validated registered bundle.
    ///
    /// # Errors
    ///
    /// Duplicate identity, capacity or invalid bundle.
    pub fn spawn(
        &mut self,
        _permit: &StructuralPermit,
        id: StableId,
        fill: impl FnOnce(&mut EntityWriter<'_>) -> Result<(), RegisteredWorldError>,
    ) -> Result<Handle, RegisteredWorldError> {
        self.world.spawn(id, fill)
    }

    /// Despawn a checked entity and retire its handle generation.
    ///
    /// # Errors
    ///
    /// The handle is stale or belongs to another world.
    pub fn despawn(
        &mut self,
        _permit: &StructuralPermit,
        handle: Handle,
    ) -> Result<(), RegisteredWorldError> {
        self.world.despawn(handle)
    }
}
