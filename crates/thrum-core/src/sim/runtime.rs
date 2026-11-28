//! Generic fixed-tick authority boundary. Games own component types, candidate
//! work and rules; the engine owns registered storage and complete publication.

mod image;
pub use image::{RuleSetIdentity, WorldImageError};

use std::{
    collections::HashMap,
    error::Error,
    hash::{Hash, Hasher},
    marker::PhantomData,
    mem::size_of,
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

#[cfg(not(target_arch = "wasm32"))]
mod scoped_workers;

static NEXT_RUNTIME: AtomicU64 = AtomicU64::new(1);

#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

use crate::{Handle, StableId, WorldEpoch};

use super::{
    AccessMode, ComponentKey, ComponentRegistry, DirtyPage, Schedule, ScheduleError, SnapshotRing,
    StructuralChange, SystemId, SystemSpec,
    column_store::TypedStore,
    publication::{PublicationCapture, PublicationPage},
    registered_world::{
        ChunkedVisit, EntityWriter, PreparedRead2, RegisteredWorld, RegisteredWorldError,
        TentativeDirty, WorldCapture, WorldUndo,
    },
    snapshot_ring::{PAGE_COUNT, PublicationBusy},
    transaction::{Edit, EditError, EditScratch, EditWriter},
};

/// Cold-path registered state capture for replay. Component values remain
/// opaque to the engine and are cloned only when this capture is requested.
pub struct RuntimeCapture {
    world: WorldCapture,
    tick: u64,
    nonce: u64,
    revision: u64,
}

/// Reusable sparse preimages for reversing one committed tick on this runtime.
pub struct RuntimeUndo {
    world: Arc<WorldUndo>,
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

    /// Visit logical slots edited by this tick, including structural edits.
    /// A slot may appear more than once; consumers deduplicate as needed.
    pub fn visit_changed_slots(&self, visit: impl FnMut(u32)) {
        self.world.visit_changed_slots(visit);
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
    /// Caller-only dispatch, for game work below its measured crossover.
    pub fn serial() -> Self {
        Self {
            mode: ExecutionMode::Serial,
        }
    }

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

    /// Execute borrowed, independent query rows with reusable native worker
    /// mailboxes. Small, contended and browser calls run on the caller.
    #[cfg_attr(
        target_arch = "wasm32",
        expect(clippy::unused_self, reason = "browser query rows run on the caller")
    )]
    pub fn for_each_query_row<T: Send>(self, rows: &mut [T], run: impl Fn(&mut T) + Send + Sync) {
        #[cfg(not(target_arch = "wasm32"))]
        if self.mode() != ExecutionMode::Serial {
            scoped_workers::run(rows, run);
            return;
        }
        rows.iter_mut().for_each(run);
    }
}

impl RuntimeCapture {
    /// Hash cold authority metadata for replay diagnostics. The caller also
    /// hashes its game component and resource values to cover the full state.
    pub fn hash_authority_metadata(&self, hash: &mut impl Hasher) {
        self.tick.hash(hash);
        self.world.hash_metadata(hash);
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

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::World(error) => write!(
                f,
                "A component or identity operation failed without changing committed state: {error}"
            ),
            Self::Schedule(error) => write!(
                f,
                "System declarations could not form a valid schedule: {error}"
            ),
            Self::UndeclaredAccess => {
                f.write_str("A query or edit was requested without declared component access")
            }
            Self::AlreadyStarted => {
                f.write_str("Initial seeding was attempted after publication or the first tick")
            }
            Self::RuntimeExhausted => {
                f.write_str("This process exhausted unique runtime access-token generations")
            }
            Self::NoCommittedTick => f.write_str("No committed tick exists for a one-tick delta"),
            Self::WrongCapture => {
                f.write_str("The replay capture belongs to another runtime instance")
            }
        }
    }
}

impl Error for RuntimeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::World(error) => Some(error),
            Self::Schedule(error) => Some(error),
            _ => None,
        }
    }
}

impl From<RegisteredWorldError> for RuntimeError {
    fn from(error: RegisteredWorldError) -> Self {
        Self::World(error)
    }
}

/// A due tick is retained on refusal; the caller may retry after correcting it.
#[derive(Debug, Eq, PartialEq)]
pub enum TickError<E: Error + 'static> {
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

impl<E: Error + 'static> std::fmt::Display for TickError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TickExhausted => f.write_str("The tick counter cannot advance without wrapping"),
            Self::RevisionExhausted => {
                f.write_str("The committed endpoint revision cannot advance without wrapping")
            }
            Self::BaselineRequired => {
                f.write_str("Tick-zero seed membership must be published before ticking")
            }
            Self::Game(error) => write!(
                f,
                "Game-owned validation refused the candidate before any commit: {error}"
            ),
            Self::Edit(error) => write!(
                f,
                "Engine preflight rejected a typed candidate edit batch: {error}"
            ),
        }
    }
}

impl<E: Error + 'static> Error for TickError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Game(error) => Some(error),
            Self::Edit(error) => Some(error),
            _ => None,
        }
    }
}

/// Game-owned rules for one registered authority. Preparation receives only
/// immutable game state, mutable shared candidate scratch and an append-only
/// engine edit writer. Values stay late-bound until final preflight; later
/// callbacks cannot replace or remove earlier queued operations. `apply`
/// runs only after complete validation and must be infallible.
pub trait TickGame {
    /// Expected game-rule or input error, retained as the tick error's source.
    /// Use `std::convert::Infallible` when game rules cannot refuse a tick.
    type Error: Error + Send + 'static;
    /// Reusable game-owned candidate and query scratch, never committed state.
    type Candidate;
    /// Private reusable output for one conflict-free parallel system callback.
    type ParallelOutput: Default + Send;

    /// Reset reusable candidate work before the first registered callback.
    fn reset_candidate(&self, candidate: &mut Self::Candidate);

    /// Staged authority resources for portable-profile validation. Profile games
    /// must return the complete resource aggregate that `apply` will commit,
    /// including unchanged resources; exclude derived scratch. The engine checks
    /// its `StateEncode::is_finite` after all callbacks and parallel merges and
    /// before committing components, resources, history or publication.
    ///
    /// The default opts out for games outside the portable profile. As with
    /// component encodings, completeness is the game author's responsibility.
    fn profile_resources<'a>(
        &self,
        _candidate: &'a Self::Candidate,
    ) -> Option<&'a dyn crate::profile::StateDecode> {
        None
    }

    /// Canonical encoding of all staged resources, when the policy supplies
    /// decoding separately from tick admission.
    fn profile_resource_encoding<'a>(
        &self,
        candidate: &'a Self::Candidate,
    ) -> Option<&'a dyn crate::profile::StateEncode> {
        self.profile_resources(candidate)
            .map(|resources| -> &dyn crate::profile::StateEncode { resources })
    }

    /// Validate a changed tentative world before the next ordered rule starts.
    /// Runs only for schedules constructed with rule boundaries and only when
    /// the callback staged an edit.
    ///
    /// # Errors
    /// A game invariant refused the resulting tentative world.
    fn after_rule_edits(
        &self,
        _world: &WorldRead<'_>,
        _candidate: &mut Self::Candidate,
        _tick: u64,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    /// Clear one private parallel buffer before every callback, retaining capacity.
    fn reset_parallel_output(&self, output: &mut Self::ParallelOutput);

    /// Merge one completed private output in compiled system order. Operations
    /// appended through `edits` must use this system's permits.
    fn merge_parallel(
        &self,
        system: SystemId,
        output: &mut Self::ParallelOutput,
        candidate: &mut Self::Candidate,
        tick: u64,
        edits: &mut EditWriter<'_, Self::Candidate>,
    );

    /// Apply the fully validated candidate. All target handles and registered
    /// component presence must have been checked during `prepare`.
    fn apply(&mut self, world: &WorldRead<'_>, tick: u64, candidate: &Self::Candidate);
}

/// One game callback bound to its access declaration before schedule compile.
pub struct RegisteredSystem<G: TickGame> {
    spec: SystemSpec,
    callback: Callback<G>,
}

type SystemCallback<G> = fn(
    &G,
    &WorldRead<'_>,
    Executor,
    u64,
    &mut <G as TickGame>::Candidate,
    &mut EditWriter<'_, <G as TickGame>::Candidate>,
) -> Result<(), <G as TickGame>::Error>;

type ParallelCallback<G> = fn(
    &G,
    &WorldRead<'_>,
    Executor,
    u64,
    &mut <G as TickGame>::ParallelOutput,
) -> Result<(), <G as TickGame>::Error>;

type DirectCallback<G> = fn(
    &G,
    &mut WorldTxn<'_>,
    Executor,
    u64,
    &mut <G as TickGame>::Candidate,
    &mut EditWriter<'_, <G as TickGame>::Candidate>,
) -> Result<(), <G as TickGame>::Error>;

enum Callback<G: TickGame> {
    Serial(SystemCallback<G>),
    Parallel(ParallelCallback<G>),
    Direct(DirectCallback<G>),
}

struct BoundSystem<G: TickGame> {
    id: SystemId,
    callback: Callback<G>,
    output: G::ParallelOutput,
    #[cfg(not(target_arch = "wasm32"))]
    result: Option<Result<(), G::Error>>,
}

impl<G: TickGame> RegisteredSystem<G> {
    /// Bind one declared system to the game callback the engine will invoke.
    pub fn new(spec: SystemSpec, callback: SystemCallback<G>) -> Self {
        Self {
            spec,
            callback: Callback::Serial(callback),
        }
    }

    /// Bind a conflict-free callback that writes only to its private output.
    pub fn parallel(spec: SystemSpec, callback: ParallelCallback<G>) -> Self {
        Self {
            spec,
            callback: Callback::Parallel(callback),
        }
    }

    /// Bind a typed in-place system. Tentative component writes roll back if
    /// this or any later precommit callback refuses the tick.
    pub fn direct(spec: SystemSpec, callback: DirectCallback<G>) -> Self {
        Self {
            spec,
            callback: Callback::Direct(callback),
        }
    }
}

fn check_edit_system<C, E: Error + 'static>(
    edits: &[Edit<C>],
    before: usize,
    system: SystemId,
) -> Result<(), TickError<E>> {
    if edits[before..].iter().any(|edit| edit.system() != system) {
        return Err(TickError::Edit(EditError::WrongSystem));
    }
    Ok(())
}

struct DirectDirty {
    words: Vec<Vec<u64>>,
}

impl DirectDirty {
    fn new(components: usize, capacity: usize) -> Self {
        Self {
            words: vec![vec![0; capacity.div_ceil(64)]; components],
        }
    }

    fn clear(&mut self) {
        for words in &mut self.words {
            words.fill(0);
        }
    }

    fn is_empty(&self) -> bool {
        self.words.iter().flatten().all(|&word| word == 0)
    }

    fn mark_and_clear(&mut self, world: &mut RegisteredWorld, tick: u64) {
        for (component, words) in self.words.iter_mut().enumerate() {
            for (index, word) in words.iter_mut().enumerate() {
                let mut bits = std::mem::take(word);
                while bits != 0 {
                    let slot = index * 64 + bits.trailing_zeros() as usize;
                    let slot = u32::try_from(slot).expect("validated slot index");
                    world.mark_changed(component, world.handle_at(slot), tick);
                    bits &= bits - 1;
                }
            }
        }
    }
}

struct QueryExclusions {
    words: Vec<u64>,
    slots: Vec<u32>,
}

impl QueryExclusions {
    fn new(capacity: usize) -> Self {
        Self {
            words: vec![0; capacity.div_ceil(64)],
            slots: Vec::with_capacity(capacity),
        }
    }

    fn mark(&mut self, slot: usize) {
        let word = &mut self.words[slot / 64];
        let bit = 1_u64 << (slot % 64);
        if *word & bit == 0 {
            *word |= bit;
            self.slots
                .push(u32::try_from(slot).expect("validated slot address"));
        }
    }

    fn clear(&mut self) {
        for slot in self.slots.drain(..) {
            self.words[slot as usize / 64] &= !(1_u64 << (slot % 64));
        }
    }
}

impl TentativeDirty for DirectDirty {
    fn is_marked(&self, component: usize, slot: u32) -> bool {
        self.words[component][slot as usize / 64] & (1_u64 << (slot % 64)) != 0
    }

    fn mark_first(&mut self, component: usize, slot: u32) -> bool {
        let word = &mut self.words[component][slot as usize / 64];
        let bit = 1_u64 << (slot % 64);
        if *word & bit != 0 {
            return false;
        }
        *word |= bit;
        true
    }
}

/// World capacity at or above which requested `Rayon` splits warm page
/// capture across the pool. `None` until a quiet measured win promotes it, so
/// requested Rayon captures serially; `RayonForced` always splits.
const PARALLEL_CAPTURE_CROSSOVER: Option<usize> = None;

/// Which final-tick preimages an engine page capture receives.
#[derive(Clone, Copy)]
enum FinalUndo<'a> {
    None,
    /// The unrecorded tick's transaction scratch.
    Abort,
    /// Preimages moved from an unrecorded tick.
    Deferred,
    /// A recorded tick's undo.
    Record(&'a WorldUndo),
}

enum PublicationState {
    Current,
    Skipped,
    SkippedWithUndo,
    /// Share the committed preimages with the caller's undo record.
    SkippedWithRecord(Arc<WorldUndo>),
}

/// One engine-owned registered world, compiled schedule and leased page ring.
/// The concrete game supplies no engine-side component or action type.
pub struct GameRuntime<G: TickGame> {
    world: RegisteredWorld,
    schedule: Schedule,
    waves: Vec<Vec<BoundSystem<G>>>,
    pages: SnapshotRing<PublicationPage>,
    initial_published: bool,
    publication_state: PublicationState,
    tick: u64,
    timeline: u64,
    revision: u64,
    next_revision: u64,
    executor: Executor,
    nonce: u64,
    edit_scratch: EditScratch,
    edits: Vec<Edit<G::Candidate>>,
    direct_dirty: DirectDirty,
    query_exclusions: QueryExclusions,
    /// Transaction scratch for unrecorded ticks; every step clears it.
    abort_undo: WorldUndo,
    /// Final-tick preimages a deferred engine publication still needs. Kept
    /// apart from the transaction scratch so a refused step cannot empty it.
    deferred_undo: WorldUndo,
    structural_journal: Vec<StructuralChange>,
    staged_journal: Vec<StructuralChange>,
    rule_boundaries: bool,
    /// Set while a tick commits. A game `apply` or capture that unwinds leaves
    /// it set: the world then holds the next tick under the old tick number,
    /// so only a full `restore` makes the runtime usable again.
    poisoned: bool,
}

impl<G: TickGame> GameRuntime<G> {
    /// Digest committed authority, including game resources supplied by the caller.
    /// Component partials are reported in schema order for divergence diagnosis.
    ///
    /// # Errors
    /// Returns the schema of a component registered without portable encoding.
    pub fn world_digest(
        &self,
        resources: &impl crate::profile::StateEncode,
        mut partial: impl FnMut(super::ComponentSchemaId, u64),
    ) -> Result<u64, super::ComponentSchemaId> {
        use crate::profile::{Digest, StateEncode};
        let mut digest = Digest::default();
        self.tick.encode(&mut digest);
        self.world.digest(&mut digest, &mut partial)?;
        resources.encode(&mut digest);
        Ok(digest.finish())
    }
    /// Diagnostic digest of the allocator's free slots in allocation order.
    /// The full world digest still includes the same allocator bytes.
    pub fn free_list_digest(&self) -> u64 {
        let mut digest = crate::profile::Digest::default();
        self.world.encode_free_list(&mut digest);
        digest.finish()
    }

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
            world: Arc::new(self.world.empty_undo()),
            events: Vec::new(),
            from: 0,
            to: 0,
            nonce: self.nonce,
            from_revision: 0,
            to_revision: 0,
            spent: false,
        }
    }

    /// Capture registered components, identities, slot generations and free
    /// order after a complete committed tick. This is a cold replay path.
    pub fn capture(&self) -> RuntimeCapture {
        self.assert_usable();
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
    fn restore(
        &mut self,
        capture: &RuntimeCapture,
        make_page: impl FnMut() -> PublicationPage,
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
        self.world.begin_tick();
        self.structural_journal.clear();
        self.initial_published = false;
        self.pages = SnapshotRing::new(
            self.world.capacity(),
            self.world.component_count(),
            make_page,
        );
        self.publication_state = PublicationState::Current;
        self.poisoned = false;
        Ok(())
    }

    /// Reject reuse after a commit unwound partway.
    fn assert_usable(&self) {
        assert!(
            !self.poisoned,
            "a tick commit unwound; restore a capture before reusing this runtime"
        );
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
        self.assert_usable();
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
        // The recorded endpoint is about to be reversed. Release its page's
        // reference before moving the preimages out of the records.
        self.publication_state = PublicationState::Skipped;
        let coalesced = steps.len() > 1
            && steps
                .iter()
                .all(|step| step.events.is_empty() && !step.world.has_layout_swaps())
            && (0..self.world.component_count()).all(|component| {
                steps.iter().all(|step| {
                    !step.world.has_component_rows(component)
                        || step.world.can_coalesce_without_drop_order(component)
                })
            });
        for step in steps.iter() {
            step.world.visit_changed_components(|component, slot| {
                self.pages.accumulate_component(component, slot);
            });
            for event in &step.events {
                let slot = match event {
                    StructuralChange::Despawn(handle) | StructuralChange::Spawn(handle) => {
                        handle.slot
                    }
                };
                self.pages.accumulate_structural(slot);
            }
        }
        self.pages.merge_changes();
        if coalesced {
            for component in 0..self.world.component_count() {
                if steps
                    .iter()
                    .any(|step| step.world.has_component_rows(component))
                {
                    self.world.begin_tick();
                    for step in steps.iter_mut() {
                        let undo = Arc::get_mut(&mut step.world).expect("rewind owns preimages");
                        self.world.restore_first_seen(undo, component);
                    }
                }
            }
            for step in steps.iter_mut() {
                step.spent = true;
            }
            self.world.begin_tick();
        } else {
            for step in steps.iter_mut().rev() {
                let undo = Arc::get_mut(&mut step.world).expect("rewind owns preimages");
                undo.restore(&mut self.world, &step.events);
                step.spent = true;
            }
        }
        self.tick = tick;
        self.revision = revision;
        self.timeline = timeline;
        self.structural_journal.clear();
        self.pages.reset_timeline();
        Ok(())
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
    fn new(
        epoch: WorldEpoch,
        capacity: usize,
        registry: ComponentRegistry,
        systems: Vec<RegisteredSystem<G>>,
        make_page: impl FnMut() -> PublicationPage,
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
        let deferred_undo = world.empty_undo();
        Ok(Self {
            world,
            schedule,
            waves,
            pages: SnapshotRing::new(capacity, component_count, make_page),
            initial_published: false,
            publication_state: PublicationState::Current,
            tick: 0,
            timeline: 0,
            revision: 0,
            next_revision: 1,
            executor: Executor {
                mode: ExecutionMode::Serial,
            },
            nonce,
            edit_scratch: EditScratch::new(capacity, component_count),
            edits: Vec::new(),
            direct_dirty: DirectDirty::new(component_count, capacity),
            query_exclusions: QueryExclusions::new(capacity),
            abort_undo,
            deferred_undo,
            structural_journal: Vec::with_capacity(128),
            staged_journal: Vec::new(),
            rule_boundaries: false,
            poisoned: false,
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
        self.assert_usable();
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

    /// Prepare a mutable query whose shared component has a registered active
    /// index. Supplemental rows can still be supplied at visit time for
    /// candidate-local input or one-shot edits.
    ///
    /// # Errors
    ///
    /// Access is undeclared, a type is absent, or `U` has no active index.
    pub fn write_read2_indexed<T: Clone + Send + Sync + 'static, U: Send + Sync + 'static>(
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
            || !self.world.has_active_index(second)
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
    pub fn latest(&self) -> Option<Arc<PublicationPage>> {
        self.pages.latest()
    }

    /// Lease the immediately preceding complete page, if one exists.
    pub fn previous(&self) -> Option<Arc<PublicationPage>> {
        self.pages.previous()
    }

    /// Whether a tick can publish without blocking on a held reader.
    pub fn can_publish_next(&self) -> bool {
        self.pages.can_capture()
    }

    /// Whether the latest immutable page represents the current authority tick.
    pub fn has_current_publication(&self) -> bool {
        matches!(self.publication_state, PublicationState::Current) && self.pages.has_latest()
    }

    /// Seal a full current baseline after optional publication was skipped.
    /// Returns false while all reusable pages remain leased. This never runs
    /// game systems or advances the committed tick.
    pub fn publish_current_if_available(
        &mut self,
        game: &mut G,
        candidate: &mut G::Candidate,
    ) -> bool {
        self.assert_usable();
        if matches!(self.publication_state, PublicationState::Current) {
            return self.pages.has_latest();
        }
        if !self.pages.can_capture() {
            return false;
        }
        if self
            .capture_latest(game, candidate, FinalUndo::Deferred, true)
            .is_err()
        {
            return false;
        }
        self.publication_state = PublicationState::Current;
        true
    }

    /// Reserved page-ring bytes including selected component columns.
    pub fn publication_reserved_bytes(
        &self,
        page_bytes: impl Fn(&PublicationPage) -> usize,
    ) -> usize {
        self.pages.reserved_bytes(page_bytes)
    }

    /// Seal the initial seeded or restored endpoint before stepping.
    ///
    /// # Panics
    ///
    /// Panics only if an unpublished ring has lost all four private pages or
    /// a registered component snapshot operation panics.
    pub fn publish_initial(&mut self, game: &mut G, candidate: &mut G::Candidate) {
        self.assert_usable();
        if !self.initial_published {
            self.capture_latest(game, candidate, FinalUndo::None, false)
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
    /// exclusive tick, or game apply or component snapshot code panics. Such
    /// a panic leaves the runtime poisoned: every later step, rewind,
    /// publish, capture, delta or seed panics until `restore` replaces the
    /// whole state.
    pub fn step(
        &mut self,
        game: &mut G,
        candidate: &mut G::Candidate,
    ) -> Result<u64, TickError<G::Error>>
    where
        G: Sync,
    {
        self.step_inner(game, candidate, None, PublicationMode::Capture, true, None)
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
        self.step_inner(game, candidate, Some(undo), publication, true, None)
    }

    /// Resolve a registered setup or test edit at the current tick. The game
    /// selects which callbacks apply. A successful edit starts a new revision.
    ///
    /// # Errors
    /// The same preparation checks as a tick can refuse the edit.
    ///
    /// # Panics
    /// The same commit and snapshot invariants as a tick apply.
    pub fn edit_current(
        &mut self,
        game: &mut G,
        candidate: &mut G::Candidate,
    ) -> Result<(), TickError<G::Error>>
    where
        G: Sync,
    {
        self.step_inner(game, candidate, None, PublicationMode::Defer, false, None)
            .map(|_| ())
    }

    /// Run a between-tick callback, keeping the same history endpoint if it
    /// staged no component or structural changes and its authority is unchanged.
    /// The predicate compares the candidate authority to the committed game.
    ///
    /// # Errors
    ///
    /// The same preparation checks as a tick can refuse the edit.
    ///
    /// # Panics
    ///
    /// The same commit and snapshot invariants as a tick apply.
    pub fn edit_current_if_changed(
        &mut self,
        game: &mut G,
        candidate: &mut G::Candidate,
        authority_changed: fn(&G, &G::Candidate) -> bool,
    ) -> Result<bool, TickError<G::Error>>
    where
        G: Sync,
    {
        let revision = self.revision;
        self.step_inner(
            game,
            candidate,
            None,
            PublicationMode::Defer,
            false,
            Some(authority_changed),
        )?;
        Ok(self.revision != revision)
    }

    fn mark_publication_changes(&mut self) {
        self.world.visit_changed_components(|component, slot| {
            self.pages.accumulate_component(component, slot);
        });
        for event in &self.structural_journal {
            let slot = match event {
                StructuralChange::Despawn(handle) | StructuralChange::Spawn(handle) => handle.slot,
            };
            self.pages.accumulate_structural(slot);
        }
        self.pages.merge_changes();
    }

    fn capture_committed(
        &mut self,
        game: &mut G,
        candidate: &mut G::Candidate,
        undo: Option<&RuntimeUndo>,
        publication: PublicationMode,
    ) {
        if publication == PublicationMode::Defer || !self.pages.can_capture() {
            self.publication_state = if let Some(record) = undo {
                PublicationState::SkippedWithRecord(Arc::clone(&record.world))
            } else {
                // The unrecorded tick's preimages move out of the
                // transaction scratch, which the next step clears.
                std::mem::swap(&mut self.abort_undo, &mut self.deferred_undo);
                PublicationState::SkippedWithUndo
            };
            return;
        }
        let final_undo = undo.map_or(FinalUndo::Abort, |record| FinalUndo::Record(&record.world));
        self.capture_latest(game, candidate, final_undo, true)
            .expect("unleased page remained available during exclusive tick");
        self.publication_state = PublicationState::Current;
    }

    /// Capture the current endpoint into the next free page, through the
    /// engine-owned publication capture.
    /// `journal` passes this tick's structural changes; the tick-zero seal
    /// passes none.
    fn capture_latest(
        &mut self,
        _game: &mut G,
        _candidate: &mut G::Candidate,
        final_undo: FinalUndo<'_>,
        journal: bool,
    ) -> Result<(), PublicationBusy> {
        let Self {
            world,
            pages,
            tick,
            timeline,
            abort_undo,
            deferred_undo,
            publication_state,
            structural_journal,
            executor,
            ..
        } = self;
        let parallel = match executor.mode {
            ExecutionMode::Serial => false,
            ExecutionMode::Rayon => {
                PARALLEL_CAPTURE_CROSSOVER.is_some_and(|rows| world.capacity() >= rows)
            }
            ExecutionMode::RayonForced => true,
        };
        let final_undo = match final_undo {
            FinalUndo::None => None,
            FinalUndo::Abort => Some(&*abort_undo),
            FinalUndo::Deferred => match &*publication_state {
                PublicationState::SkippedWithRecord(record) => Some(record.as_ref()),
                PublicationState::SkippedWithUndo => Some(&*deferred_undo),
                _ => None,
            },
            FinalUndo::Record(record) => Some(record),
        };
        let events: &[StructuralChange] = if journal { structural_journal } else { &[] };
        let (tick, timeline) = (*tick, *timeline);
        pages.capture(|page, cold, page_dirty, outbound_dirty, source| {
            capture_publication_page(
                page,
                world,
                cold,
                page_dirty,
                outbound_dirty,
                source,
                tick,
                timeline,
                final_undo,
                events,
                parallel,
            );
        })
    }

    fn prepare_step(&self, advance: bool) -> Result<(u64, u64), TickError<G::Error>> {
        self.assert_usable();
        let next = self
            .tick
            .checked_add(u64::from(advance))
            .ok_or(TickError::TickExhausted)?;
        let following_revision = self
            .next_revision
            .checked_add(1)
            .ok_or(TickError::RevisionExhausted)?;
        if !self.initial_published {
            return Err(TickError::BaselineRequired);
        }
        Ok((next, following_revision))
    }

    fn step_inner(
        &mut self,
        game: &mut G,
        candidate: &mut G::Candidate,
        mut undo: Option<&mut RuntimeUndo>,
        publication: PublicationMode,
        advance: bool,
        authority_changed: Option<fn(&G, &G::Candidate) -> bool>,
    ) -> Result<u64, TickError<G::Error>>
    where
        G: Sync,
    {
        let (next, following_revision) = self.prepare_step(advance)?;
        self.direct_dirty.clear();
        self.query_exclusions.clear();
        // When a caller reuses its deferred endpoint's record, stage the new
        // preimages separately. A refused tick keeps the old record usable.
        let mut replacement = undo
            .as_ref()
            .filter(|record| Arc::strong_count(&record.world) > 1)
            .map(|_| Arc::new(self.world.empty_undo()));
        let (txn_undo, mut undo_events) = match undo.as_deref_mut() {
            Some(record) => (
                Arc::get_mut(replacement.as_mut().unwrap_or(&mut record.world))
                    .expect("tick owns transaction preimages"),
                Some(&mut record.events),
            ),
            None => (&mut self.abort_undo, None),
        };
        txn_undo.clear();
        txn_undo.rule_boundaries = self.rule_boundaries;
        self.edits.clear();
        if self.rule_boundaries {
            self.staged_journal.clear();
            self.world.begin_tick();
        }
        game.reset_candidate(candidate);
        let mut ran_barrier = false;
        let preparation = catch_unwind(AssertUnwindSafe(|| {
            for wave in &mut self.waves {
                #[cfg(not(target_arch = "wasm32"))]
                // Two tiny input callbacks stay on the tick caller to avoid pool scheduling.
                if !self.rule_boundaries
                    && self.executor.mode != ExecutionMode::Serial
                    && (wave.len() > 3 || self.executor.mode == ExecutionMode::RayonForced)
                    && wave
                        .iter()
                        .all(|bound| matches!(&bound.callback, Callback::Parallel(_)))
                {
                    run_parallel_wave(
                        wave,
                        game,
                        &self.world,
                        self.nonce,
                        self.executor,
                        next,
                        candidate,
                        &mut self.edits,
                    )?;
                    continue;
                }
                for bound in wave {
                    let before = self.edits.len();
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
                            &mut EditWriter::new(&mut self.edits),
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
                                query_exclusions: &mut self.query_exclusions,
                                nonce: self.nonce,
                                system: bound.id,
                            },
                            self.executor,
                            next,
                            candidate,
                            &mut EditWriter::new(&mut self.edits),
                        ),
                    }
                    .map_err(TickError::Game)?;
                    if matches!(&bound.callback, Callback::Parallel(_)) {
                        game.merge_parallel(
                            bound.id,
                            &mut bound.output,
                            candidate,
                            next,
                            &mut EditWriter::new(&mut self.edits),
                        );
                    }
                    check_edit_system(&self.edits, before, bound.id)?;
                    if self.rule_boundaries && !self.edits.is_empty() {
                        ran_barrier = true;
                        apply_rule_edits(
                            &mut self.world,
                            &mut self.edit_scratch,
                            &mut self.edits,
                            &mut self.direct_dirty,
                            &mut self.staged_journal,
                            txn_undo,
                            self.nonce,
                            next,
                            game,
                            candidate,
                        )?;
                    }
                }
            }
            if !self.rule_boundaries {
                self.edit_scratch
                    .validate(&self.world, self.nonce, &self.edits, candidate)
                    .map_err(TickError::Edit)?;
            }
            validate_tick_profile(&self.world, game, candidate)
        }));
        if !matches!(&preparation, Ok(Ok(()))) {
            if self.rule_boundaries {
                txn_undo.restore(&mut self.world, &self.staged_journal);
            } else {
                txn_undo.abort_tentative(&mut self.world);
            }
            self.direct_dirty.clear();
            self.query_exclusions.clear();
            match preparation {
                Ok(result) => result?,
                Err(payload) => resume_unwind(payload),
            }
        }
        if let Some(changed) = authority_changed
            && self.edits.is_empty()
            && !ran_barrier
            && self.direct_dirty.is_empty()
            && !changed(game, candidate)
        {
            return Ok(self.tick);
        }
        // Commit is infallible; an unwind from here on poisons the runtime.
        self.poisoned = true;
        let edits = &self.edits;
        if let Some(events) = undo_events.as_deref_mut() {
            events.reserve(edits.len());
        }
        if self.rule_boundaries {
            self.direct_dirty.mark_and_clear(&mut self.world, next);
            std::mem::swap(&mut self.staged_journal, &mut self.structural_journal);
        } else {
            self.edit_scratch.apply(
                &mut self.world,
                edits,
                candidate,
                next,
                &mut self.structural_journal,
                Some(txn_undo),
            );
        }
        self.world
            .finalize_activity(&self.structural_journal, txn_undo);
        self.direct_dirty.clear();
        self.mark_publication_changes();
        game.apply(
            &WorldRead {
                world: &self.world,
                nonce: self.nonce,
                system: None,
            },
            next,
            candidate,
        );
        if let Some(events) = undo_events {
            events.clear();
            events.extend_from_slice(&self.structural_journal);
        }
        if let Some(record) = undo.as_deref_mut() {
            if let Some(replacement) = replacement {
                record.world = replacement;
            }
            record.from = self.tick;
            record.to = next;
            record.from_revision = self.revision;
            record.to_revision = self.next_revision;
            record.spent = false;
        }
        self.tick = next;
        self.revision = self.next_revision;
        self.next_revision = following_revision;
        self.capture_committed(game, candidate, undo.as_deref(), publication);
        self.poisoned = false;
        Ok(next)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn run_parallel_wave<G: TickGame + Sync>(
    wave: &mut [BoundSystem<G>],
    game: &G,
    world: &RegisteredWorld,
    nonce: u64,
    executor: Executor,
    tick: u64,
    candidate: &mut G::Candidate,
    edits: &mut Vec<Edit<G::Candidate>>,
) -> Result<(), TickError<G::Error>> {
    wave.par_iter_mut().for_each(|bound| {
        let Callback::Parallel(callback) = &bound.callback else {
            unreachable!("wave checked parallel")
        };
        game.reset_parallel_output(&mut bound.output);
        bound.result = Some(callback(
            game,
            &WorldRead {
                world,
                nonce,
                system: Some(bound.id),
            },
            executor,
            tick,
            &mut bound.output,
        ));
    });
    for bound in wave {
        bound
            .result
            .take()
            .expect("callback ran")
            .map_err(TickError::Game)?;
        let before = edits.len();
        game.merge_parallel(
            bound.id,
            &mut bound.output,
            candidate,
            tick,
            &mut EditWriter::new(edits),
        );
        check_edit_system(edits, before, bound.id)?;
    }
    Ok(())
}

fn apply_rule_edits<G: TickGame>(
    world: &mut RegisteredWorld,
    scratch: &mut EditScratch,
    edits: &mut Vec<Edit<G::Candidate>>,
    dirty: &mut DirectDirty,
    journal: &mut Vec<StructuralChange>,
    undo: &mut WorldUndo,
    nonce: u64,
    tick: u64,
    game: &G,
    candidate: &mut G::Candidate,
) -> Result<(), TickError<G::Error>> {
    scratch
        .validate(world, nonce, edits, candidate)
        .map_err(TickError::Edit)?;
    dirty.mark_and_clear(world, tick);
    scratch.apply(world, edits, candidate, tick, journal, Some(undo));
    edits.clear();
    game.after_rule_edits(
        &WorldRead {
            world,
            nonce,
            system: None,
        },
        candidate,
        tick,
    )
    .map_err(TickError::Game)
}

fn validate_tick_profile<G: TickGame>(
    world: &RegisteredWorld,
    game: &G,
    candidate: &G::Candidate,
) -> Result<(), TickError<G::Error>> {
    world
        .validate_profile()
        .map_err(|error| TickError::Edit(EditError::World(error)))?;
    if game
        .profile_resource_encoding(candidate)
        .is_some_and(|resources| !resources.is_finite())
    {
        return Err(TickError::Edit(EditError::World(
            RegisteredWorldError::NonFiniteResource,
        )));
    }
    Ok(())
}

fn capture_publication_page(
    page: &mut PublicationPage,
    world: &RegisteredWorld,
    cold: bool,
    page_dirty: DirtyPage<'_>,
    outbound_dirty: DirtyPage<'_>,
    source: Option<&PublicationPage>,
    tick: u64,
    timeline: u64,
    final_undo: Option<&WorldUndo>,
    final_events: &[StructuralChange],
    parallel: bool,
) {
    page.capture(
        world,
        PublicationCapture {
            cold,
            page_dirty,
            outbound_dirty,
            source,
            tick,
            timeline,
            final_undo,
            final_events,
            parallel,
        },
    );
}

impl<G: TickGame> GameRuntime<G> {
    /// Construct a registry-selected engine publication ring before the
    /// registry moves into authority storage. Games supply only their schema
    /// and rule callbacks; page capture stays inside the engine.
    ///
    /// # Errors
    ///
    /// Capacity, registration or schedule validation failed.
    ///
    /// # Panics
    ///
    /// Only if the fixed ring requests more than its prepared page count.
    pub fn new_published(
        epoch: WorldEpoch,
        capacity: usize,
        registry: ComponentRegistry,
        systems: Vec<RegisteredSystem<G>>,
    ) -> Result<Self, RuntimeError> {
        let mut prepared = (0..PAGE_COUNT)
            .map(|_| PublicationPage::new(&registry, epoch, capacity))
            .collect::<Vec<_>>();
        Self::new(epoch, capacity, registry, systems, || {
            prepared
                .pop()
                .expect("one prepared publication per ring page")
        })
    }

    /// Construct a serial schedule whose staged edits land after each system.
    /// All systems still share one undo record and one committed endpoint.
    ///
    /// # Errors
    /// Capacity, registration or schedule validation failed.
    pub fn new_published_with_rule_boundaries(
        epoch: WorldEpoch,
        capacity: usize,
        registry: ComponentRegistry,
        systems: Vec<RegisteredSystem<G>>,
    ) -> Result<Self, RuntimeError> {
        let mut runtime = Self::new_published(epoch, capacity, registry, systems)?;
        runtime.rule_boundaries = true;
        Ok(runtime)
    }

    /// Restore a cold authority capture and rebuild engine publication pages.
    ///
    /// # Errors
    ///
    /// The capture belongs to another runtime or is incompatible.
    ///
    /// # Panics
    ///
    /// Only if the fixed ring requests more than its prepared page count.
    pub fn restore_published(&mut self, capture: &RuntimeCapture) -> Result<(), RuntimeError> {
        let epoch = self.world.epoch();
        let capacity = self.world.capacity();
        let mut prepared = (0..PAGE_COUNT)
            .map(|_| PublicationPage::new(self.world.registry(), epoch, capacity))
            .collect::<Vec<_>>();
        self.restore(capture, || {
            prepared
                .pop()
                .expect("replacement publication ring was prepared")
        })
    }
}

mod rule_storage;
pub use rule_storage::{RuleColumn, RuleStorage};

/// Checked immutable access to committed game components.
pub struct WorldRead<'a> {
    pub(super) world: &'a RegisteredWorld,
    nonce: u64,
    system: Option<SystemId>,
}

/// Tentative typed writes for one declared system. The runtime retains
/// preimages and publishes change ticks only after the whole tick succeeds.
pub struct WorldTxn<'a> {
    world: &'a mut RegisteredWorld,
    undo: &'a mut WorldUndo,
    dirty: &'a mut DirectDirty,
    query_exclusions: &'a mut QueryExclusions,
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
    /// The registered callback bound to this transaction view.
    pub fn system_id(&self) -> SystemId {
        self.system
    }

    /// Read the tentative world as seen by this system.
    pub fn read(&self) -> WorldRead<'_> {
        WorldRead {
            world: self.world,
            nonce: self.nonce,
            system: Some(self.system),
        }
    }

    /// Read a checked component under this system's declared read permit.
    ///
    /// # Errors
    ///
    /// The permit is foreign, the handle is stale, or the component is absent.
    pub fn with<T: Send + Sync + 'static, R>(
        &self,
        permit: &ReadPermit<T>,
        handle: Handle,
        read: impl FnOnce(&T) -> R,
    ) -> Result<R, RegisteredWorldError> {
        if permit.nonce != self.nonce {
            return Err(RegisteredWorldError::WrongRuntime);
        }
        if permit.system != self.system {
            return Err(RegisteredWorldError::WrongSystem);
        }
        self.world.with(handle, read)
    }

    /// Tentatively insert or replace one checked component under this system's
    /// declared write permit. A refused tick restores its first preimage.
    ///
    /// # Errors
    ///
    /// The permit is foreign or the entity handle is stale.
    pub fn put<T: Clone + PartialEq + Send + Sync + 'static>(
        &mut self,
        permit: &WritePermit<T>,
        handle: Handle,
        value: T,
    ) -> Result<bool, RegisteredWorldError> {
        if permit.nonce != self.nonce {
            return Err(RegisteredWorldError::WrongRuntime);
        }
        if permit.system != self.system {
            return Err(RegisteredWorldError::WrongSystem);
        }
        self.world
            .put_tentative(permit.component, handle, value, self.undo, self.dirty)
    }

    /// Live entities in the committed world this transaction edits.
    pub fn len(&self) -> usize {
        self.world.len()
    }

    /// Whether the committed world has no live entity.
    pub fn is_empty(&self) -> bool {
        self.world.len() == 0
    }

    /// Visit indexed rows except for selected logical slots in word-aligned
    /// storage chunks. Chunks are independent: under a non-serial executor
    /// they run on the Rayon pool, and serial runs execute the same chunk code
    /// on the caller in storage order. Callbacks share the context immutably
    /// and must not depend on row order. Other chunks may keep writing after
    /// one refuses; returning the earliest chunk's error refuses the tick, and
    /// the abort restores every tentative write. A system must not swallow it.
    /// Returns the changed row count and how many chunks ran on the pool.
    ///
    /// # Errors
    ///
    /// The game callback refuses one selected row. Return the error to refuse
    /// the tick: writes made before the refusal are restored only by the abort.
    ///
    /// # Panics
    ///
    /// The query is foreign or an excluded slot is out of range.
    pub fn visit_chunked_indexed<
        T: Clone + Send + Sync + 'static,
        U: Send + Sync + 'static,
        C: Sync,
        E: Send,
    >(
        &mut self,
        query: &WriteRead2<T, U>,
        executor: Executor,
        context: &C,
        exclude: impl FnOnce(&C, &mut dyn FnMut(usize)),
        select: impl Fn(&C, Handle, StableId, &T, &U) -> bool + Sync,
        visit: impl Fn(&C, Handle, StableId, &mut T, &U) -> Result<bool, E> + Sync,
    ) -> Result<ChunkedVisit, E> {
        assert_eq!(query.nonce, self.nonce, "query belongs to another runtime");
        assert_eq!(query.system, self.system, "query belongs to another system");
        self.query_exclusions.clear();
        exclude(context, &mut |slot| {
            let row = self
                .world
                .row_of_slot(u32::try_from(slot).expect("validated slot"));
            self.query_exclusions.mark(row);
        });
        let result = self.world.visit_mut2_chunked(
            query.first,
            query.second,
            &self.query_exclusions.words,
            executor.mode != ExecutionMode::Serial,
            self.undo,
            self.dirty,
            context,
            select,
            visit,
        );
        self.query_exclusions.clear();
        result
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
    world: &'a RegisteredWorld,
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
        Ok(self.column.get(self.world.row_of_slot(slot.handle.slot)))
    }

    /// Read a physical slot in a page capture after the column's permit has
    /// been checked at binding. Absence also covers a despawned slot.
    pub fn at_slot(&self, slot: u32) -> Option<&T> {
        if slot as usize >= self.world.capacity() {
            return None;
        }
        self.column.get(self.world.row_of_slot(slot))
    }
}

impl WorldRead<'_> {
    /// Number of logical slots, including vacant and retired slots.
    pub fn capacity(&self) -> usize {
        self.world.capacity()
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

    /// Resolve a live handle's physical row for this callback's scoped loans.
    ///
    /// # Errors
    ///
    /// The handle is stale or belongs to another epoch.
    pub fn physical_row(&self, handle: Handle) -> Result<usize, RegisteredWorldError> {
        self.world.stable_id(handle)?;
        Ok(self.world.row_of_slot(handle.slot))
    }

    /// Check whether this live entity has a registered component of the given key.
    /// Used to validate overlapping query projections before lending values.
    pub fn has_component(&self, handle: Handle, key: ComponentKey) -> bool {
        self.world.has_component(handle, key)
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
            world: self.world,
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

impl<T: Send + Sync + 'static> ReadPermit<T> {
    /// Component and structural slots accumulated for this bound publication
    /// read. The dirty view must come from the same runtime callback.
    pub fn changed_slots<'a>(&self, dirty: DirtyPage<'a>) -> impl Iterator<Item = u32> + 'a {
        dirty.component_slots(self.component)
    }
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
