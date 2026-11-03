use std::{
    collections::{BTreeMap, HashMap as StdHashMap, hash_map::RandomState},
    ops::Deref,
    sync::{Arc, Weak},
};

use hashbrown::HashMap;

use crate::replica_index::ChargedReplicaIndex;
use crate::{PublicationCapacity, PublicationCharge, PublicationPool, PublicationReservation};

const PAGE_SIZE: usize = 32;
pub(crate) const MAX_SNAPSHOT_LEASES: usize = 8;

pub(crate) struct SnapshotLeaseFloor {
    _charge: PublicationCharge,
}

impl SnapshotLeaseFloor {
    pub(crate) fn required_charge() -> Result<usize, PublicationCapacity> {
        let arc_header = 2 * std::mem::size_of::<usize>();
        let token = std::mem::size_of::<SnapshotLease>()
            .checked_add(arc_header)
            .ok_or(PublicationCapacity)?;
        MAX_SNAPSHOT_LEASES
            .checked_mul(token)
            .and_then(|bytes| bytes.checked_add(std::mem::size_of::<Self>()))
            .and_then(|bytes| bytes.checked_add(arc_header))
            .ok_or(PublicationCapacity)
    }

    pub(crate) fn try_new(
        reservation: &mut PublicationReservation,
    ) -> Result<Arc<Self>, PublicationCapacity> {
        let charge = reservation.charge(Self::required_charge()?)?;
        Ok(Arc::new(Self { _charge: charge }))
    }

    fn belongs_to(&self, reservation: &PublicationReservation) -> bool {
        self._charge.belongs_to(reservation)
    }
}

pub(crate) struct SnapshotLease {
    _floor: Option<Arc<SnapshotLeaseFloor>>,
}

/// A stable game/content object identity, distinct from a runtime handle.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StableId(pub u128);
/// The identity of a controlled actor within the admitted game.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ActorId(pub u64);
/// A fresh local play session identity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SessionId(pub u128);
/// A shared authoritative network match identity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct MatchId(pub u128);
/// The local identity of one constructed world, supplied fresh by the host.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct WorldEpoch(pub u128);
/// Distinguishes standalone and match authority without conflating their IDs.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum AuthorityId {
    /// Local standalone authority.
    Standalone(SessionId),
    /// Shared match authority.
    Match(MatchId),
}
/// Checked transient runtime reference. A stale generation never resolves.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Handle {
    /// Constructed world identity.
    pub epoch: WorldEpoch,
    /// Slot within this world.
    pub slot: u32,
    /// Generation of that slot.
    pub generation: u64,
}
/// A full object record in an immutable read model or replica publication.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityRecord<T> {
    /// Stable object identity.
    pub id: StableId,
    /// Current game-owned object state.
    pub state: T,
}
/// One complete object lifecycle/update at a commit boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EntityChange<T> {
    /// New object and all required first state.
    Spawn(EntityRecord<T>),
    /// Complete replacement state for an existing object.
    Update(EntityRecord<T>),
    /// Committed destruction.
    Despawn(StableId),
}
/// One complete, game-prepared world edit. A batch is checked in order before
/// any edit is applied, so ordinary invalid references cannot partly mutate
/// the authoritative world.
pub enum WorldEdit<T> {
    /// Insert a new stable object identity.
    Spawn(StableId, T),
    /// Replace the full state through a checked existing handle.
    Replace(Handle, T),
    /// Retire an existing object through a checked handle.
    Despawn(Handle),
    /// Retire an object spawned earlier in this same batch.
    DespawnNew(StableId),
}

/// Complete ordered edits for one game decision. Small actions keep their
/// edits inline; larger admitted actions own a bounded vector.
pub enum PreparedEdits<T> {
    /// A committed decision with no object mutation.
    None,
    /// One edit with no batch-validation allocation.
    One(WorldEdit<T>),
    /// Two edits with inline batch-validation bookkeeping.
    Two(WorldEdit<T>, WorldEdit<T>),
    /// Larger ordered batch; the game profile must bound its length.
    Many(Vec<WorldEdit<T>>),
}

impl<T> PreparedEdits<T> {
    pub(crate) fn is_inline(&self) -> bool {
        !matches!(self, Self::Many(_))
    }

    pub(crate) fn validate_states<E>(
        &self,
        validate: impl Fn(&T) -> Result<(), E>,
    ) -> Result<(), E> {
        let check = |edit: &WorldEdit<T>| match edit {
            WorldEdit::Spawn(_, state) | WorldEdit::Replace(_, state) => validate(state),
            WorldEdit::Despawn(_) | WorldEdit::DespawnNew(_) => Ok(()),
        };
        match self {
            Self::None => Ok(()),
            Self::One(edit) => check(edit),
            Self::Two(first, second) => {
                check(first)?;
                check(second)
            }
            Self::Many(edits) => edits.iter().try_for_each(check),
        }
    }
    /// Number of edits the authority must admit and apply.
    pub fn len(&self) -> usize {
        match self {
            Self::None => 0,
            Self::One(_) => 1,
            Self::Two(..) => 2,
            Self::Many(edits) => edits.len(),
        }
    }

    /// Whether the prepared decision leaves the world unchanged.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Maximum new-slot high-water contribution of this batch.
    pub fn spawns(&self) -> usize {
        match self {
            Self::None => 0,
            Self::One(edit) => usize::from(matches!(edit, WorldEdit::Spawn(..))),
            Self::Two(first, second) => {
                usize::from(matches!(first, WorldEdit::Spawn(..)))
                    + usize::from(matches!(second, WorldEdit::Spawn(..)))
            }
            Self::Many(edits) => edits
                .iter()
                .filter(|edit| matches!(edit, WorldEdit::Spawn(..)))
                .count(),
        }
    }
}
/// World insertion or lookup failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorldError {
    /// A stable object identity is already active.
    DuplicateId,
    /// No free slot can be represented by a `u32`.
    SlotExhausted,
    /// A handle is stale or foreign to this world.
    InvalidHandle,
}

struct Slot<T> {
    generation: u64,
    entry: Option<EntityRecord<T>>,
}

#[derive(Clone)]
struct SnapshotEntry<T> {
    generation: u64,
    record: EntityRecord<T>,
}

struct PageData<T> {
    entries: Box<[Option<SnapshotEntry<T>>]>,
    _charge: Option<PublicationCharge>,
}

impl<T> PageData<T> {
    fn storage_bytes(len: usize) -> Result<usize, PublicationCapacity> {
        len.checked_mul(size_of::<Option<SnapshotEntry<T>>>())
            .ok_or(PublicationCapacity)
    }

    fn charge_bytes(len: usize) -> Result<usize, PublicationCapacity> {
        size_of::<Self>()
            .checked_add(2 * size_of::<usize>())
            .and_then(|base| Self::storage_bytes(len).ok()?.checked_add(base))
            .ok_or(PublicationCapacity)
    }
}

impl<T> Deref for PageData<T> {
    type Target = [Option<SnapshotEntry<T>>];

    fn deref(&self) -> &Self::Target {
        &self.entries
    }
}

type Page<T> = Arc<PageData<T>>;

struct PageRootData<T> {
    pages: Box<[Page<T>]>,
    _charge: Option<PublicationCharge>,
}

impl<T> PageRootData<T> {
    fn storage_bytes(len: usize) -> Result<usize, PublicationCapacity> {
        len.checked_mul(size_of::<Page<T>>())
            .ok_or(PublicationCapacity)
    }

    fn charge_bytes(len: usize) -> Result<usize, PublicationCapacity> {
        size_of::<Self>()
            .checked_add(2 * size_of::<usize>())
            .and_then(|base| Self::storage_bytes(len).ok()?.checked_add(base))
            .ok_or(PublicationCapacity)
    }
}

impl<T> Deref for PageRootData<T> {
    type Target = [Page<T>];

    fn deref(&self) -> &Self::Target {
        &self.pages
    }
}

type PageRoot<T> = Arc<PageRootData<T>>;

enum PublicationMode {
    Unselected,
    Uncharged,
    Charged(PublicationPool),
}

#[derive(Clone, Copy)]
enum ChangeKind {
    Spawn,
    Update,
    Despawn,
}

#[derive(Clone, Copy)]
struct VirtualObject {
    alive: bool,
    change: Option<ChangeKind>,
    spawned_in_batch: bool,
    original_handle_alive: bool,
}

/// The fixed-slot sealed path marks changes by slot without allocating a tree
/// node in an action callback. Publication sorts only the touched identities.
struct FixedSlotJournal {
    marks: Vec<Option<ChangeKind>>,
    touched: Vec<(StableId, usize)>,
    limit: usize,
    _charge: PublicationCharge,
}

struct FixedWorldStorage {
    _charge: PublicationCharge,
    slot_capacity: usize,
    dirty_capacity: usize,
    index_bytes: usize,
}

/// The structural path records identities rather than slots: a retired A and
/// a new B can share one physical slot before the publication barrier.
struct StructuralJournal {
    changes: HashMap<StableId, Option<ChangeKind>, RandomState>,
    touched: Vec<StableId>,
    limit: usize,
    allocation_bytes: usize,
    _charge: PublicationCharge,
}

impl StructuralJournal {
    fn mark(&mut self, id: StableId, kind: ChangeKind) {
        if let Some(previous) = self.changes.get_mut(&id) {
            *previous = match (*previous, kind) {
                (Some(ChangeKind::Spawn), ChangeKind::Update) => Some(ChangeKind::Spawn),
                (Some(ChangeKind::Spawn), ChangeKind::Despawn) => None,
                (_, next) => Some(next),
            };
            return;
        }
        assert!(self.touched.len() < self.limit, "structural change bound");
        self.touched.push(id);
        self.changes.insert(id, Some(kind));
        assert_eq!(self.changes.allocation_size(), self.allocation_bytes);
    }

    fn get(&self, id: StableId) -> Option<ChangeKind> {
        self.changes.get(&id).copied().flatten()
    }
}

struct StructuralWorldStorage {
    _charge: PublicationCharge,
    max_slots: usize,
    max_live: usize,
    slot_capacity: usize,
    dirty_capacity: usize,
    free_capacity: usize,
    index_bytes: usize,
}

impl FixedSlotJournal {
    fn mark(&mut self, slot: usize, id: StableId, kind: ChangeKind) {
        match &mut self.marks[slot] {
            Some(previous) => {
                *previous = match (*previous, kind) {
                    (ChangeKind::Spawn, ChangeKind::Update) => ChangeKind::Spawn,
                    (_, next) => next,
                };
            }
            mark @ None => {
                assert!(self.touched.len() < self.limit, "fixed-slot change bound");
                self.touched.push((id, slot));
                *mark = Some(kind);
            }
        }
    }

    fn get(&self, slot: usize) -> Option<ChangeKind> {
        self.marks[slot]
    }
}

enum VirtualObjects {
    Inline([Option<(StableId, VirtualObject)>; 2]),
    Many(StdHashMap<StableId, VirtualObject>),
}

impl VirtualObjects {
    fn for_edits(count: usize) -> Self {
        if count <= 2 {
            Self::Inline([None; 2])
        } else {
            Self::Many(StdHashMap::with_capacity(count))
        }
    }

    fn entry(&mut self, id: StableId, make: impl FnOnce() -> VirtualObject) -> &mut VirtualObject {
        match self {
            Self::Inline(entries) => {
                let index = entries
                    .iter()
                    .position(|entry| entry.as_ref().is_some_and(|(key, _)| *key == id))
                    .unwrap_or_else(|| {
                        entries
                            .iter()
                            .position(Option::is_none)
                            .expect("at most two distinct IDs in two edits")
                    });
                &mut entries[index].get_or_insert_with(|| (id, make())).1
            }
            Self::Many(entries) => entries.entry(id).or_insert_with(make),
        }
    }
}

/// Exclusive authoritative object store for the finite I1 slice.
///
/// Each mutation is visible to readers only after `publish`. `get_mut` marks a
/// page dirty even if the caller ultimately makes no change.
pub struct World<T: Clone> {
    epoch: WorldEpoch,
    slots: Vec<Slot<T>>,
    free: Vec<u32>,
    by_id: HashMap<StableId, u32, RandomState>,
    dirty: Vec<bool>,
    dirty_count: usize,
    changed: BTreeMap<StableId, ChangeKind>,
    fixed_journal: Option<FixedSlotJournal>,
    structural_journal: Option<StructuralJournal>,
    _free_charge: Option<PublicationCharge>,
    fixed_storage: Option<FixedWorldStorage>,
    structural_storage: Option<StructuralWorldStorage>,
    pages: PageRoot<T>,
    publication_mode: PublicationMode,
    published_tick: u64,
}

/// A charged snapshot publication could not use its proposed reservation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorldPublicationError {
    /// The reservation is too small for the selected page/root allocations.
    Capacity,
    /// This world already published through the uncharged prototype path.
    UnchargedWorld,
    /// The reservation belongs to a different process-shared pool.
    WrongPool,
    /// Initial closed state exceeds the selected per-value profile.
    ValueProfile,
}

/// Counted work at a world publication boundary.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WorldWork {
    /// Immutable page references copied into the new read model.
    pub page_references: usize,
    /// Pages rebuilt because at least one slot was touched.
    pub copied_pages: usize,
    /// Live object records cloned into those pages.
    pub copied_objects: usize,
    /// Full object records copied for replica changes.
    pub group_objects: usize,
}

/// Immutable committed read model; clones retain page leases.
#[derive(Clone)]
pub struct Snapshot<T> {
    epoch: WorldEpoch,
    tick: u64,
    pages: PageRoot<T>,
    lease: Arc<SnapshotLease>,
}

impl<T> Snapshot<T> {
    pub(crate) fn lease_token(&self) -> Weak<SnapshotLease> {
        Arc::downgrade(&self.lease)
    }
    /// Local world epoch represented by this snapshot.
    pub fn epoch(&self) -> WorldEpoch {
        self.epoch
    }
    /// Last committed authority tick represented by this snapshot.
    pub fn tick(&self) -> u64 {
        self.tick
    }
    /// Resolve a checked handle against an immutable publication.
    pub fn get(&self, handle: Handle) -> Option<&EntityRecord<T>> {
        if handle.epoch != self.epoch {
            return None;
        }
        let index = handle.slot as usize;
        self.pages
            .get(index / PAGE_SIZE)?
            .get(index % PAGE_SIZE)?
            .as_ref()
            .filter(|entry| entry.generation == handle.generation)
            .map(|entry| &entry.record)
    }
    /// Enumerate complete records in slot order for a baseline or inspection.
    pub fn records(&self) -> impl Iterator<Item = &EntityRecord<T>> {
        self.pages.iter().flat_map(|page| {
            page.iter()
                .filter_map(|entry| entry.as_ref().map(|e| &e.record))
        })
    }
    /// Approximate retained payload bytes, excluding allocator and shared ownership overhead.
    pub fn logical_bytes(&self) -> usize {
        self.pages
            .iter()
            .map(|p| p.len() * std::mem::size_of::<Option<SnapshotEntry<T>>>())
            .sum()
    }
}

/// A complete immutable read model and its in-memory lifecycle projection.
pub struct WorldPublication<T> {
    /// Committed read model.
    pub snapshot: Snapshot<T>,
    /// Full state/lifecycle changes at this barrier.
    pub changes: Vec<EntityChange<T>>,
    /// Counted publication work.
    pub work: WorldWork,
}

impl<T: Clone> World<T> {
    pub(crate) fn validate_states<E>(
        &self,
        validate: impl Fn(&T) -> Result<(), E>,
    ) -> Result<(), E> {
        self.slots
            .iter()
            .filter_map(|slot| slot.entry.as_ref())
            .try_for_each(|record| validate(&record.state))
    }

    /// Select allocation-free change recording for an already published
    /// fixed-slot world. The caller bounds all first touches before edits.
    pub(crate) fn activate_fixed_slot_journal(
        &mut self,
        pool: &PublicationPool,
        max_touches: usize,
    ) -> Result<(), PublicationCapacity> {
        let PublicationMode::Charged(owner_pool) = &self.publication_mode else {
            return Err(PublicationCapacity);
        };
        if self.fixed_journal.is_some()
            || self.structural_journal.is_some()
            || !self.changed.is_empty()
            || self.dirty_count != 0
        {
            return Err(PublicationCapacity);
        }
        let touches = max_touches.min(self.slots.len());
        let marks_bytes = self
            .slots
            .len()
            .checked_mul(size_of::<Option<ChangeKind>>())
            .ok_or(PublicationCapacity)?;
        let touched_bytes = touches
            .checked_mul(size_of::<(StableId, usize)>())
            .ok_or(PublicationCapacity)?;
        let replace_free = self.free.capacity() < self.slots.len();
        let free_capacity = if replace_free {
            self.slots.len()
        } else {
            self.free.capacity()
        };
        let free_bytes = free_capacity
            .checked_mul(size_of::<u32>())
            .ok_or(PublicationCapacity)?;
        let old_overlap = if replace_free {
            self.free
                .capacity()
                .checked_mul(size_of::<u32>())
                .ok_or(PublicationCapacity)?
        } else {
            0
        };
        let journal_bytes = marks_bytes
            .checked_add(touched_bytes)
            .ok_or(PublicationCapacity)?;
        let slot_bytes = self
            .slots
            .capacity()
            .checked_mul(size_of::<Slot<T>>())
            .ok_or(PublicationCapacity)?;
        let dirty_bytes = self
            .dirty
            .capacity()
            .checked_mul(size_of::<bool>())
            .ok_or(PublicationCapacity)?;
        let index_bytes = self.by_id.allocation_size();
        let storage_bytes = slot_bytes
            .checked_add(dirty_bytes)
            .and_then(|bytes| bytes.checked_add(index_bytes))
            .ok_or(PublicationCapacity)?;
        let peak = journal_bytes
            .checked_add(free_bytes)
            .and_then(|bytes| bytes.checked_add(old_overlap))
            .and_then(|bytes| bytes.checked_add(storage_bytes))
            .ok_or(PublicationCapacity)?;
        let mut reservation = pool.try_reserve(peak)?;
        if !reservation.belongs_to(owner_pool) {
            return Err(PublicationCapacity);
        }
        let marks = vec![None; self.slots.len()];
        let touched = Vec::with_capacity(touches);
        let replacement = replace_free.then(|| {
            let mut free = Vec::with_capacity(free_capacity);
            free.extend(self.free.iter().copied());
            free
        });
        let journal_charge = reservation.charge(journal_bytes)?;
        let free_charge = reservation.charge(free_bytes)?;
        let storage_charge = reservation.charge(storage_bytes)?;
        if let Some(replacement) = replacement {
            self.free = replacement;
        }
        self.fixed_journal = Some(FixedSlotJournal {
            marks,
            touched,
            limit: touches,
            _charge: journal_charge,
        });
        self._free_charge = Some(free_charge);
        self.fixed_storage = Some(FixedWorldStorage {
            _charge: storage_charge,
            slot_capacity: self.slots.capacity(),
            dirty_capacity: self.dirty.capacity(),
            index_bytes,
        });
        Ok(())
    }

    /// Precharge a bounded mutable world and identity-keyed journal. The
    /// caller supplies the maximum same-barrier distinct identity count.
    pub(crate) fn activate_structural_journal(
        &mut self,
        pool: &PublicationPool,
        max_slots: usize,
        max_live: usize,
        max_touches: usize,
    ) -> Result<(), PublicationCapacity> {
        let PublicationMode::Charged(owner_pool) = &self.publication_mode else {
            return Err(PublicationCapacity);
        };
        if self.fixed_journal.is_some()
            || self.structural_journal.is_some()
            || !self.changed.is_empty()
            || self.dirty_count != 0
            || max_slots > (u32::MAX as usize).saturating_add(1)
            || max_live > max_slots
            || self.slots.len() > max_slots
            || self.len() > max_live
        {
            return Err(PublicationCapacity);
        }
        let index_peak = ChargedReplicaIndex::<u32>::required_peak(max_live)?;
        let journal_peak = ChargedReplicaIndex::<Option<ChangeKind>>::required_peak(max_touches)?;
        let slot_bytes = max_slots
            .checked_mul(size_of::<Slot<T>>())
            .ok_or(PublicationCapacity)?;
        let dirty_bytes = max_slots
            .div_ceil(PAGE_SIZE)
            .checked_mul(size_of::<bool>())
            .ok_or(PublicationCapacity)?;
        let free_bytes = max_slots
            .checked_mul(size_of::<u32>())
            .ok_or(PublicationCapacity)?;
        let touched_bytes = max_touches
            .checked_mul(size_of::<StableId>())
            .ok_or(PublicationCapacity)?;
        let old_bytes = self
            .slots
            .capacity()
            .checked_mul(size_of::<Slot<T>>())
            .and_then(|v| v.checked_add(self.dirty.capacity().checked_mul(size_of::<bool>())?))
            .and_then(|v| v.checked_add(self.free.capacity().checked_mul(size_of::<u32>())?))
            .and_then(|v| v.checked_add(self.by_id.allocation_size()))
            .ok_or(PublicationCapacity)?;
        let new_bytes = slot_bytes
            .checked_add(dirty_bytes)
            .and_then(|v| v.checked_add(free_bytes))
            .and_then(|v| v.checked_add(index_peak))
            .and_then(|v| v.checked_add(journal_peak))
            .and_then(|v| v.checked_add(touched_bytes))
            .ok_or(PublicationCapacity)?;
        let mut reservation = pool.try_reserve(
            new_bytes
                .checked_add(old_bytes)
                .ok_or(PublicationCapacity)?,
        )?;
        if !reservation.belongs_to(owner_pool) {
            return Err(PublicationCapacity);
        }
        let mut slots = Vec::with_capacity(max_slots);
        let mut dirty = Vec::with_capacity(max_slots.div_ceil(PAGE_SIZE));
        dirty.extend(self.dirty.iter().copied());
        let mut free = Vec::with_capacity(max_slots);
        free.extend(self.free.iter().copied());
        let mut by_id = HashMap::with_hasher(RandomState::new());
        by_id
            .try_reserve(max_live.checked_mul(2).ok_or(PublicationCapacity)?)
            .map_err(|_| PublicationCapacity)?;
        by_id.extend(self.by_id.iter().map(|(&id, &slot)| (id, slot)));
        let mut changes = HashMap::with_hasher(RandomState::new());
        changes
            .try_reserve(max_touches.checked_mul(2).ok_or(PublicationCapacity)?)
            .map_err(|_| PublicationCapacity)?;
        let touched = Vec::with_capacity(max_touches);
        let actual_slot = slots
            .capacity()
            .checked_mul(size_of::<Slot<T>>())
            .ok_or(PublicationCapacity)?;
        let actual_dirty = dirty
            .capacity()
            .checked_mul(size_of::<bool>())
            .ok_or(PublicationCapacity)?;
        let actual_free = free
            .capacity()
            .checked_mul(size_of::<u32>())
            .ok_or(PublicationCapacity)?;
        let actual_touched = touched
            .capacity()
            .checked_mul(size_of::<StableId>())
            .ok_or(PublicationCapacity)?;
        let actual_index = by_id.allocation_size();
        let actual_journal = changes.allocation_size();
        assert!(
            actual_slot <= slot_bytes
                && actual_dirty <= dirty_bytes
                && actual_free <= free_bytes
                && actual_touched <= touched_bytes
                && actual_index <= index_peak
                && actual_journal <= journal_peak,
            "pinned structural owner exceeded preallocation bound"
        );
        let journal_charge = reservation.charge(
            actual_journal
                .checked_add(actual_touched)
                .ok_or(PublicationCapacity)?,
        )?;
        let free_charge = reservation.charge(actual_free)?;
        let storage_charge = reservation.charge(
            actual_slot
                .checked_add(actual_dirty)
                .and_then(|v| v.checked_add(actual_index))
                .ok_or(PublicationCapacity)?,
        )?;
        slots.append(&mut self.slots);
        self.slots = slots;
        self.dirty = dirty;
        self.free = free;
        self.by_id = by_id;
        self.structural_journal = Some(StructuralJournal {
            changes,
            touched,
            limit: max_touches,
            allocation_bytes: actual_journal,
            _charge: journal_charge,
        });
        self._free_charge = Some(free_charge);
        self.structural_storage = Some(StructuralWorldStorage {
            _charge: storage_charge,
            max_slots,
            max_live,
            slot_capacity: self.slots.capacity(),
            dirty_capacity: self.dirty.capacity(),
            free_capacity: self.free.capacity(),
            index_bytes: actual_index,
        });
        Ok(())
    }
    /// Construct an empty world with a fresh host-supplied epoch.
    pub fn new(epoch: WorldEpoch) -> Self {
        Self {
            epoch,
            slots: Vec::new(),
            free: Vec::new(),
            by_id: HashMap::with_hasher(RandomState::new()),
            dirty: Vec::new(),
            dirty_count: 0,
            changed: BTreeMap::new(),
            fixed_journal: None,
            structural_journal: None,
            _free_charge: None,
            fixed_storage: None,
            structural_storage: None,
            pages: Arc::new(PageRootData {
                pages: Box::new([]),
                _charge: None,
            }),
            publication_mode: PublicationMode::Unselected,
            published_tick: 0,
        }
    }
    /// Local identity of this constructed world.
    pub fn epoch(&self) -> WorldEpoch {
        self.epoch
    }
    /// Number of currently live objects.
    pub fn len(&self) -> usize {
        self.by_id.len()
    }
    pub(crate) fn slot_high_water(&self) -> usize {
        self.slots.len()
    }
    pub(crate) fn reusable_slots(&self) -> usize {
        self.free.len()
    }
    /// Whether the world contains no objects.
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
    /// Resolve stable identity into a checked transient handle.
    pub fn handle_for(&self, id: StableId) -> Option<Handle> {
        let slot = *self.by_id.get(&id)?;
        Some(Handle {
            epoch: self.epoch,
            slot,
            generation: self.slots[slot as usize].generation,
        })
    }
    /// Read a live object through a checked handle.
    pub fn get(&self, handle: Handle) -> Option<&EntityRecord<T>> {
        if handle.epoch != self.epoch {
            return None;
        }
        let slot = self.slots.get(handle.slot as usize)?;
        (slot.generation == handle.generation)
            .then_some(slot.entry.as_ref())
            .flatten()
    }
    /// Mutate a live object during the exclusive authority step.
    pub fn get_mut(&mut self, handle: Handle) -> Option<&mut T> {
        if handle.epoch != self.epoch {
            return None;
        }
        let index = handle.slot as usize;
        let slot = self.slots.get(index)?;
        if slot.generation != handle.generation {
            return None;
        }
        let id = slot.entry.as_ref()?.id;
        self.mark_dirty(index / PAGE_SIZE);
        if let Some(journal) = &mut self.fixed_journal {
            journal.mark(index, id, ChangeKind::Update);
        } else if let Some(journal) = &mut self.structural_journal {
            journal.mark(id, ChangeKind::Update);
        } else {
            self.changed.entry(id).or_insert(ChangeKind::Update);
        }
        self.slots[index]
            .entry
            .as_mut()
            .map(|record| &mut record.state)
    }
    /// Apply one complete game-prepared decision through the matching small
    /// or large atomic edit path.
    pub fn apply_edits(&mut self, edits: PreparedEdits<T>) -> Result<(), WorldError> {
        match edits {
            PreparedEdits::None => Ok(()),
            PreparedEdits::One(edit) => self.apply_one(edit),
            PreparedEdits::Two(first, second) => self.apply_pair(first, second),
            PreparedEdits::Many(edits) => self.apply_prepared(edits),
        }
    }
    /// Apply one prepared edit without allocating a validation collection.
    pub fn apply_one(&mut self, edit: WorldEdit<T>) -> Result<(), WorldError> {
        match edit {
            WorldEdit::Spawn(id, state) => self.spawn(id, state).map(|_| ()),
            WorldEdit::Replace(handle, state) => {
                *self.get_mut(handle).ok_or(WorldError::InvalidHandle)? = state;
                Ok(())
            }
            WorldEdit::Despawn(handle) => self.despawn(handle).map(|_| ()),
            WorldEdit::DespawnNew(_) => Err(WorldError::InvalidHandle),
        }
    }
    /// Validate a complete prepared batch before applying its ordered edits.
    /// A structural peak may exceed final live objects, so admission callers
    /// must reserve for every possible new slot before invoking this method.
    pub fn apply_prepared(&mut self, mut edits: Vec<WorldEdit<T>>) -> Result<(), WorldError> {
        if edits.len() == 1 {
            return self.apply_one(edits.pop().expect("one prepared edit"));
        }
        self.validate_prepared_edits(&edits)?;
        for edit in edits {
            self.apply_validated_edit(edit);
        }
        Ok(())
    }

    /// Validate two ordered edits without allocating a batch vector or
    /// validation map. A later invalid edit leaves both unapplied.
    pub fn apply_pair(
        &mut self,
        first: WorldEdit<T>,
        second: WorldEdit<T>,
    ) -> Result<(), WorldError> {
        let edits = [first, second];
        self.validate_prepared_edits(&edits)?;
        for edit in edits {
            self.apply_validated_edit(edit);
        }
        Ok(())
    }

    fn validate_prepared_edits(&self, edits: &[WorldEdit<T>]) -> Result<(), WorldError> {
        let mut virtual_objects = VirtualObjects::for_edits(edits.len());
        for edit in edits {
            let id = match edit {
                WorldEdit::Spawn(id, _) | WorldEdit::DespawnNew(id) => *id,
                WorldEdit::Replace(handle, _) | WorldEdit::Despawn(handle) => {
                    self.get(*handle).ok_or(WorldError::InvalidHandle)?.id
                }
            };
            let object = virtual_objects.entry(id, || {
                let alive = self.by_id.contains_key(&id);
                VirtualObject {
                    alive,
                    change: self.change_for(id),
                    spawned_in_batch: false,
                    original_handle_alive: alive,
                }
            });
            match edit {
                WorldEdit::Spawn(..) => {
                    if object.alive || matches!(object.change, Some(ChangeKind::Despawn)) {
                        return Err(WorldError::DuplicateId);
                    }
                    object.alive = true;
                    object.change = Some(ChangeKind::Spawn);
                    object.spawned_in_batch = true;
                }
                WorldEdit::Replace(..) => {
                    if !object.alive || !object.original_handle_alive {
                        return Err(WorldError::InvalidHandle);
                    }
                    if !matches!(object.change, Some(ChangeKind::Spawn)) {
                        object.change = Some(ChangeKind::Update);
                    }
                }
                WorldEdit::Despawn(..) => {
                    if !object.alive || !object.original_handle_alive {
                        return Err(WorldError::InvalidHandle);
                    }
                    object.alive = false;
                    object.original_handle_alive = false;
                    object.change = if matches!(object.change, Some(ChangeKind::Spawn)) {
                        None
                    } else {
                        Some(ChangeKind::Despawn)
                    };
                }
                WorldEdit::DespawnNew(..) => {
                    if !object.alive || !object.spawned_in_batch {
                        return Err(WorldError::InvalidHandle);
                    }
                    object.alive = false;
                    object.change = if matches!(object.change, Some(ChangeKind::Spawn)) {
                        None
                    } else {
                        Some(ChangeKind::Despawn)
                    };
                }
            }
        }
        self.validate_prepared_slot_capacity(edits)
    }
    fn change_for(&self, id: StableId) -> Option<ChangeKind> {
        if let Some(journal) = &self.structural_journal {
            return journal.get(id);
        }
        let Some(journal) = &self.fixed_journal else {
            return self.changed.get(&id).copied();
        };
        let slot = self.by_id.get(&id).map(|slot| *slot as usize).or_else(|| {
            journal
                .touched
                .iter()
                .find(|(key, _)| *key == id)
                .map(|(_, slot)| *slot)
        })?;
        journal.get(slot)
    }

    fn apply_validated_edit(&mut self, edit: WorldEdit<T>) {
        match edit {
            WorldEdit::Spawn(id, state) => {
                self.spawn(id, state).expect("validated prepared spawn");
            }
            WorldEdit::Replace(handle, state) => {
                *self.get_mut(handle).expect("validated prepared handle") = state;
            }
            WorldEdit::Despawn(handle) => {
                self.despawn(handle).expect("validated prepared handle");
            }
            WorldEdit::DespawnNew(id) => {
                let handle = self.handle_for(id).expect("validated new object");
                self.despawn(handle).expect("validated new object handle");
            }
        }
    }
    fn validate_prepared_slot_capacity(&self, edits: &[WorldEdit<T>]) -> Result<(), WorldError> {
        let spawns = edits
            .iter()
            .filter(|edit| matches!(edit, WorldEdit::Spawn(..)))
            .count();
        if let Some(storage) = &self.structural_storage {
            let mut available = self.free.len();
            let mut high_water = self.slots.len();
            let mut live = self.len();
            for edit in edits {
                match edit {
                    WorldEdit::Spawn(..) => {
                        if live >= storage.max_live {
                            return Err(WorldError::SlotExhausted);
                        }
                        if available != 0 {
                            available -= 1;
                        } else if high_water < storage.max_slots {
                            high_water += 1;
                        } else {
                            return Err(WorldError::SlotExhausted);
                        }
                        live += 1;
                    }
                    WorldEdit::Despawn(handle) => {
                        live -= 1;
                        if handle.generation.checked_add(1).is_some() {
                            available += 1;
                        }
                    }
                    WorldEdit::DespawnNew(..) => {
                        live -= 1;
                        // Conservatively do not count this released slot for
                        // a later spawn in a larger batch.
                    }
                    WorldEdit::Replace(..) => {}
                }
            }
            return Ok(());
        }
        if spawns == 0
            || self
                .slots
                .len()
                .checked_add(spawns - 1)
                .is_some_and(|last| u32::try_from(last).is_ok())
        {
            return Ok(());
        }
        // Only a world near u32 slot exhaustion needs to simulate reuse. The
        // ordinary path above is O(1) and never clones the free list.
        let mut free = self.free.clone();
        let mut generations = StdHashMap::new();
        let mut spawned = StdHashMap::new();
        let mut appended = 0_usize;
        for edit in edits {
            match edit {
                WorldEdit::Spawn(id, _) => {
                    let slot = if let Some(slot) = free.pop() {
                        slot
                    } else {
                        let index = self
                            .slots
                            .len()
                            .checked_add(appended)
                            .ok_or(WorldError::SlotExhausted)?;
                        let slot = u32::try_from(index).map_err(|_| WorldError::SlotExhausted)?;
                        appended += 1;
                        slot
                    };
                    let generation = generations.get(&slot).copied().unwrap_or_else(|| {
                        self.slots
                            .get(slot as usize)
                            .map_or(0, |old| old.generation)
                    });
                    spawned.insert(*id, (slot, generation));
                }
                WorldEdit::Despawn(handle) => {
                    if let Some(next) = handle.generation.checked_add(1) {
                        generations.insert(handle.slot, next);
                        free.push(handle.slot);
                    }
                }
                WorldEdit::DespawnNew(id) => {
                    let (slot, generation) = spawned.remove(id).expect("validated new object");
                    if let Some(next) = generation.checked_add(1) {
                        generations.insert(slot, next);
                        free.push(slot);
                    }
                }
                WorldEdit::Replace(..) => {}
            }
        }
        Ok(())
    }
    /// Insert a complete game-owned object; stable IDs cannot alias.
    pub fn spawn(&mut self, id: StableId, state: T) -> Result<Handle, WorldError> {
        assert!(
            self.fixed_journal.is_none(),
            "fixed-slot world cannot spawn"
        );
        if self.by_id.contains_key(&id) {
            return Err(WorldError::DuplicateId);
        }
        if matches!(self.change_for(id), Some(ChangeKind::Despawn)) {
            return Err(WorldError::DuplicateId);
        }
        if let Some(storage) = &self.structural_storage
            && (self.len() >= storage.max_live
                || (self.free.is_empty() && self.slots.len() >= storage.max_slots))
        {
            return Err(WorldError::SlotExhausted);
        }
        let slot = if let Some(slot) = self.free.pop() {
            slot
        } else {
            let slot = u32::try_from(self.slots.len()).map_err(|_| WorldError::SlotExhausted)?;
            self.slots.push(Slot {
                generation: 0,
                entry: None,
            });
            if self.slots.len() % PAGE_SIZE == 1 {
                self.dirty.push(false);
            }
            slot
        };
        let index = slot as usize;
        self.slots[index].entry = Some(EntityRecord { id, state });
        self.by_id.insert(id, slot);
        if let Some(storage) = &self.structural_storage {
            assert_eq!(
                self.by_id.allocation_size(),
                storage.index_bytes,
                "structural ID index reallocated"
            );
        }
        self.mark_dirty(index / PAGE_SIZE);
        if let Some(journal) = &mut self.structural_journal {
            journal.mark(id, ChangeKind::Spawn);
        } else {
            self.changed.insert(id, ChangeKind::Spawn);
        }
        Ok(Handle {
            epoch: self.epoch,
            slot,
            generation: self.slots[index].generation,
        })
    }
    /// Destroy an object and invalidate its runtime handle.
    pub fn despawn(&mut self, handle: Handle) -> Result<EntityRecord<T>, WorldError> {
        let record = self.get(handle).ok_or(WorldError::InvalidHandle)?.clone();
        let index = handle.slot as usize;
        self.mark_dirty(index / PAGE_SIZE);
        let slot = &mut self.slots[index];
        slot.entry = None;
        self.by_id.remove(&record.id);
        if let Some(storage) = &self.fixed_storage {
            assert_eq!(
                self.by_id.allocation_size(),
                storage.index_bytes,
                "fixed-slot ID index reallocated"
            );
        }
        if let Some(storage) = &self.structural_storage {
            assert_eq!(
                self.by_id.allocation_size(),
                storage.index_bytes,
                "structural ID index reallocated"
            );
        }
        if let Some(journal) = &mut self.fixed_journal {
            journal.mark(index, record.id, ChangeKind::Despawn);
        } else if let Some(journal) = &mut self.structural_journal {
            journal.mark(record.id, ChangeKind::Despawn);
        } else if matches!(self.changed.get(&record.id), Some(ChangeKind::Spawn)) {
            self.changed.remove(&record.id);
        } else {
            self.changed.insert(record.id, ChangeKind::Despawn);
        }
        if let Some(next) = slot.generation.checked_add(1) {
            slot.generation = next;
            self.free.push(handle.slot);
        }
        Ok(record)
    }
    fn mark_dirty(&mut self, page_index: usize) {
        if !self.dirty[page_index] {
            self.dirty[page_index] = true;
            self.dirty_count += 1;
        }
    }

    /// Conservative number of full-state group entries after at most this
    /// many prepared edits. Existing unpublished changes also occupy entries;
    /// each new edit can introduce at most one new stable ID.
    pub(crate) fn conservative_change_count(
        &self,
        max_edits: usize,
    ) -> Result<usize, PublicationCapacity> {
        self.structural_journal
            .as_ref()
            .map_or_else(
                || {
                    self.fixed_journal
                        .as_ref()
                        .map_or(self.changed.len(), |journal| journal.touched.len())
                },
                |journal| journal.touched.len(),
            )
            .checked_add(max_edits)
            .ok_or(PublicationCapacity)
    }

    /// Conservative logical page/root peak for the current dirty set,
    /// including possible Vec-to-Box staging overlap and initial empty root.
    /// A caller preparing a tick must reserve its possible peak earlier.
    pub fn required_charged_publication_bytes(&self) -> Result<usize, PublicationCapacity> {
        let first = matches!(self.publication_mode, PublicationMode::Unselected);
        if self.dirty_count == 0 && !first {
            return Ok(0);
        }
        let count = self.slots.len().div_ceil(PAGE_SIZE);
        // The Vec used to construct a boxed slice can briefly overlap its
        // final allocation if conversion must relocate it.
        let mut bytes = PageRootData::<T>::charge_bytes(count)?
            .checked_add(PageRootData::<T>::storage_bytes(count)?)
            .ok_or(PublicationCapacity)?;
        if first {
            // The prototype constructor's empty root still overlaps the first
            // charged root until it is replaced, but has no lifetime charge.
            bytes = bytes
                .checked_add(PageRootData::<T>::charge_bytes(0)?)
                .ok_or(PublicationCapacity)?;
        }
        for (page_index, dirty) in self.dirty.iter().enumerate() {
            if *dirty {
                let len = (self.slots.len() - page_index * PAGE_SIZE).min(PAGE_SIZE);
                bytes = bytes
                    .checked_add(PageData::<T>::charge_bytes(len)?)
                    .and_then(|bytes| bytes.checked_add(PageData::<T>::storage_bytes(len).ok()?))
                    .ok_or(PublicationCapacity)?;
            }
        }
        Ok(bytes)
    }

    /// O(1) conservative page/root reservation for a future batch. `max_edits`
    /// counts every possible touched slot, including structural edits;
    /// `max_spawns` bounds slot high-water even if objects retire before the
    /// barrier. Game payloads and all non-snapshot outputs need other bounds.
    pub fn conservative_charged_publication_peak(
        &self,
        max_edits: usize,
        max_spawns: usize,
    ) -> Result<usize, WorldPublicationError> {
        let possible_slots = self
            .slots
            .len()
            .checked_add(max_spawns)
            .ok_or(WorldPublicationError::Capacity)?;
        self.conservative_charged_publication_peak_at(max_edits.max(max_spawns), possible_slots)
    }

    /// Price a structural future credit at an already admitted absolute slot
    /// horizon, including reusable free slots without double-counting spawns.
    pub(crate) fn conservative_charged_publication_peak_at(
        &self,
        max_edits: usize,
        possible_slots: usize,
    ) -> Result<usize, WorldPublicationError> {
        if matches!(self.publication_mode, PublicationMode::Uncharged) {
            return Err(WorldPublicationError::UnchargedWorld);
        }
        if possible_slots < self.slots.len() {
            return Err(WorldPublicationError::Capacity);
        }
        let first = matches!(self.publication_mode, PublicationMode::Unselected);
        let possible_pages = possible_slots.div_ceil(PAGE_SIZE);
        let touches = self
            .dirty_count
            .checked_add(max_edits)
            .ok_or(WorldPublicationError::Capacity)?;
        let dirty_pages = possible_pages.min(touches);
        if dirty_pages == 0 && !first {
            return Ok(0);
        }
        let root = PageRootData::<T>::charge_bytes(possible_pages)
            .and_then(|bytes| {
                bytes
                    .checked_add(PageRootData::<T>::storage_bytes(possible_pages)?)
                    .ok_or(PublicationCapacity)
            })
            .map_err(|_| WorldPublicationError::Capacity)?;
        let page = PageData::<T>::charge_bytes(PAGE_SIZE)
            .and_then(|bytes| {
                bytes
                    .checked_add(PageData::<T>::storage_bytes(PAGE_SIZE)?)
                    .ok_or(PublicationCapacity)
            })
            .map_err(|_| WorldPublicationError::Capacity)?;
        let mut peak = dirty_pages
            .checked_mul(page)
            .and_then(|pages| pages.checked_add(root))
            .ok_or(WorldPublicationError::Capacity)?;
        if first {
            peak = peak
                .checked_add(
                    PageRootData::<T>::charge_bytes(0)
                        .map_err(|_| WorldPublicationError::Capacity)?,
                )
                .ok_or(WorldPublicationError::Capacity)?;
        }
        Ok(peak)
    }

    /// Publish only page/root owners from a pre-existing reservation. This is
    /// a partial accounting primitive: game payload, groups and replica still
    /// require separate bounds. Do not mix charged and prototype publication.
    pub fn try_publish_charged(
        &mut self,
        tick: u64,
        reservation: &mut PublicationReservation,
    ) -> Result<WorldPublication<T>, WorldPublicationError> {
        self.try_publish_charged_inner(tick, reservation, None)
    }

    pub(crate) fn try_publish_charged_with_lease_floor(
        &mut self,
        tick: u64,
        reservation: &mut PublicationReservation,
        floor: &Arc<SnapshotLeaseFloor>,
    ) -> Result<WorldPublication<T>, WorldPublicationError> {
        if !floor.belongs_to(reservation) {
            return Err(WorldPublicationError::WrongPool);
        }
        self.try_publish_charged_inner(tick, reservation, Some(floor))
    }

    fn try_publish_charged_inner(
        &mut self,
        tick: u64,
        reservation: &mut PublicationReservation,
        floor: Option<&Arc<SnapshotLeaseFloor>>,
    ) -> Result<WorldPublication<T>, WorldPublicationError> {
        match &self.publication_mode {
            PublicationMode::Uncharged => return Err(WorldPublicationError::UnchargedWorld),
            PublicationMode::Charged(pool) if !reservation.belongs_to(pool) => {
                return Err(WorldPublicationError::WrongPool);
            }
            PublicationMode::Unselected | PublicationMode::Charged(_) => {}
        }
        let first = matches!(self.publication_mode, PublicationMode::Unselected);
        let required = self
            .required_charged_publication_bytes()
            .map_err(|_| WorldPublicationError::Capacity)?;
        if reservation.remaining_bytes() < required {
            return Err(WorldPublicationError::Capacity);
        }
        let pool = first.then(|| reservation.pool());
        let published = self.publish_inner(tick, Some(reservation), first, floor);
        if let Some(pool) = pool {
            self.publication_mode = PublicationMode::Charged(pool);
        }
        Ok(published)
    }

    /// Build an uncharged prototype publication. Charged worlds must use
    /// `try_publish_charged` so later ticks cannot silently bypass the pool.
    pub fn publish(&mut self, tick: u64) -> WorldPublication<T> {
        assert!(
            !matches!(self.publication_mode, PublicationMode::Charged(_)),
            "charged world requires try_publish_charged"
        );
        self.publication_mode = PublicationMode::Uncharged;
        self.publish_inner(tick, None, false, None)
    }

    fn publish_inner(
        &mut self,
        tick: u64,
        mut reservation: Option<&mut PublicationReservation>,
        force_root: bool,
        floor: Option<&Arc<SnapshotLeaseFloor>>,
    ) -> WorldPublication<T> {
        assert!(tick >= self.published_tick, "tick cannot move backwards");
        if let Some(storage) = &self.fixed_storage {
            assert_eq!(self.slots.capacity(), storage.slot_capacity);
            assert_eq!(self.dirty.capacity(), storage.dirty_capacity);
            assert_eq!(self.by_id.allocation_size(), storage.index_bytes);
        }
        if let Some(storage) = &self.structural_storage {
            assert_eq!(self.slots.capacity(), storage.slot_capacity);
            assert_eq!(self.dirty.capacity(), storage.dirty_capacity);
            assert_eq!(self.free.capacity(), storage.free_capacity);
            assert_eq!(self.by_id.allocation_size(), storage.index_bytes);
        }
        let count = self.slots.len().div_ceil(PAGE_SIZE);
        let mut work = WorldWork {
            ..WorldWork::default()
        };
        if self.dirty_count != 0 || force_root {
            let mut pages = Vec::with_capacity(count);
            pages.extend(self.pages.iter().cloned());
            work.page_references = count;
            for page_index in 0..count {
                if !self.dirty[page_index] {
                    continue;
                }
                let start = page_index * PAGE_SIZE;
                let end = (start + PAGE_SIZE).min(self.slots.len());
                let mut entries = Vec::with_capacity(end - start);
                for slot in &self.slots[start..end] {
                    if slot.entry.is_some() {
                        work.copied_objects += 1;
                    }
                    entries.push(slot.entry.as_ref().map(|record| SnapshotEntry {
                        generation: slot.generation,
                        record: record.clone(),
                    }));
                }
                let page = Arc::new(PageData {
                    entries: entries.into_boxed_slice(),
                    _charge: reservation.as_deref_mut().map(|reservation| {
                        reservation
                            .charge(PageData::<T>::charge_bytes(end - start).expect("preflight"))
                            .expect("preflight")
                    }),
                });
                if page_index == pages.len() {
                    pages.push(page);
                } else {
                    pages[page_index] = page;
                }
                self.dirty[page_index] = false;
                work.copied_pages += 1;
            }
            self.pages = Arc::new(PageRootData {
                pages: pages.into_boxed_slice(),
                _charge: reservation.map(|reservation| {
                    reservation
                        .charge(PageRootData::<T>::charge_bytes(count).expect("preflight"))
                        .expect("preflight")
                }),
            });
            self.dirty_count = 0;
        }
        let change_count = self
            .conservative_change_count(0)
            .expect("bounded change count");
        let mut changes = Vec::with_capacity(change_count);
        let by_id = &self.by_id;
        let slots = &self.slots;
        let mut emit = |id, kind| match kind {
            ChangeKind::Spawn | ChangeKind::Update => {
                let slot = by_id[&id] as usize;
                let record = slots[slot]
                    .entry
                    .as_ref()
                    .expect("live indexed object")
                    .clone();
                work.group_objects += 1;
                changes.push(if matches!(kind, ChangeKind::Spawn) {
                    EntityChange::Spawn(record)
                } else {
                    EntityChange::Update(record)
                });
            }
            ChangeKind::Despawn => changes.push(EntityChange::Despawn(id)),
        };
        if let Some(journal) = &mut self.fixed_journal {
            journal.touched.sort_unstable_by_key(|(id, _)| *id);
            for &(id, slot) in &journal.touched {
                let kind = journal.marks[slot].take().expect("touched slot marked");
                emit(id, kind);
            }
            journal.touched.clear();
        } else if let Some(journal) = &mut self.structural_journal {
            journal.touched.sort_unstable();
            for &id in &journal.touched {
                if let Some(kind) = journal.changes.get(&id).copied().flatten() {
                    emit(id, kind);
                }
            }
            journal.touched.clear();
            journal.changes.clear();
            assert_eq!(journal.changes.allocation_size(), journal.allocation_bytes);
        } else {
            for (id, kind) in std::mem::take(&mut self.changed) {
                emit(id, kind);
            }
        }
        self.published_tick = tick;
        WorldPublication {
            snapshot: Snapshot {
                epoch: self.epoch,
                tick,
                pages: self.pages.clone(),
                lease: Arc::new(SnapshotLease {
                    _floor: floor.cloned(),
                }),
            },
            changes,
            work,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{hint::black_box, time::Instant};

    #[test]
    #[ignore = "local development cost fixture; run explicitly in release mode"]
    fn compare_fixed_and_structural_one_change_publication_cost() {
        fn run(structural: bool) -> u128 {
            let pool = PublicationPool::new(8_000_000);
            let mut world = World::new(WorldEpoch(299));
            for id in 0..4_096_u128 {
                world.spawn(StableId(id), id as u64).unwrap();
            }
            let peak = world.conservative_charged_publication_peak(0, 0).unwrap();
            let mut reservation = pool.try_reserve(peak).unwrap();
            let initial = world.try_publish_charged(0, &mut reservation).unwrap();
            drop(initial);
            drop(reservation);
            if structural {
                world
                    .activate_structural_journal(&pool, 4_096, 4_096, 2)
                    .unwrap();
            } else {
                world.activate_fixed_slot_journal(&pool, 2).unwrap();
            }
            let handle = world.handle_for(StableId(2_048)).unwrap();
            let mut elapsed = 0_u128;
            for tick in 1..=6_000_u64 {
                let start = Instant::now();
                *world.get_mut(handle).unwrap() = black_box(tick);
                let peak = world.conservative_charged_publication_peak(0, 0).unwrap();
                let mut reservation = pool.try_reserve(peak).unwrap();
                let publication = world.try_publish_charged(tick, &mut reservation).unwrap();
                black_box(&publication.changes);
                drop(publication);
                drop(reservation);
                if tick > 1_000 {
                    elapsed += start.elapsed().as_nanos();
                }
            }
            drop(world);
            assert_eq!(pool.used_bytes(), 0);
            elapsed / 5_000
        }
        let fixed = run(false);
        let structural = run(true);
        eprintln!(
            "one-change 4096-object world-only publication: fixed={fixed} ns structural={structural} ns"
        );
    }

    #[test]
    fn structural_journal_preserves_reused_identity_and_neutral_touches() {
        let pool = PublicationPool::new(1_000_000);
        let mut world = World::new(WorldEpoch(291));
        let old = world.spawn(StableId(1), 10_u8).unwrap();
        world.spawn(StableId(2), 20).unwrap();
        let peak = world.conservative_charged_publication_peak(0, 0).unwrap();
        let mut reservation = pool.try_reserve(peak).unwrap();
        let initial = world.try_publish_charged(0, &mut reservation).unwrap();
        drop(reservation);

        let requested_peak = 4 * size_of::<Slot<u8>>()
            + 4_usize.div_ceil(PAGE_SIZE) * size_of::<bool>()
            + 4 * size_of::<u32>()
            + ChargedReplicaIndex::<u32>::required_peak(3).unwrap()
            + ChargedReplicaIndex::<Option<ChangeKind>>::required_peak(4).unwrap()
            + 4 * size_of::<StableId>()
            + world.slots.capacity() * size_of::<Slot<u8>>()
            + world.dirty.capacity() * size_of::<bool>()
            + world.free.capacity() * size_of::<u32>()
            + world.by_id.allocation_size();
        let blocker = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes() - (requested_peak - 1))
            .unwrap();
        assert_eq!(
            world.activate_structural_journal(&pool, 4, 3, 4),
            Err(PublicationCapacity)
        );
        assert!(world.structural_journal.is_none());
        assert_eq!(world.handle_for(StableId(1)), Some(old));
        drop(blocker);
        world.activate_structural_journal(&pool, 4, 3, 4).unwrap();
        let retained = pool.used_bytes();
        let index_bytes = world.structural_storage.as_ref().unwrap().index_bytes;
        let journal_bytes = world.structural_journal.as_ref().unwrap().allocation_bytes;

        world
            .apply_pair(WorldEdit::Despawn(old), WorldEdit::Spawn(StableId(3), 30))
            .unwrap();
        let replacement = world.handle_for(StableId(3)).unwrap();
        assert_eq!(replacement.slot, old.slot);
        assert_ne!(replacement.generation, old.generation);
        assert!(world.get(old).is_none());
        let peak = world.conservative_charged_publication_peak(0, 0).unwrap();
        let mut reservation = pool.try_reserve(peak).unwrap();
        let first = world.try_publish_charged(1, &mut reservation).unwrap();
        assert!(matches!(
            first.changes.as_slice(),
            [
                EntityChange::Despawn(StableId(1)),
                EntityChange::Spawn(EntityRecord {
                    id: StableId(3),
                    ..
                })
            ]
        ));
        assert_eq!(initial.snapshot.get(old).unwrap().state, 10);
        drop(first);
        drop(reservation);

        world
            .apply_pair(
                WorldEdit::Spawn(StableId(4), 40),
                WorldEdit::DespawnNew(StableId(4)),
            )
            .unwrap();
        assert_eq!(
            world.structural_journal.as_ref().unwrap().touched,
            [StableId(4)]
        );
        assert!(world.change_for(StableId(4)).is_none());
        world.spawn(StableId(4), 44).unwrap();
        assert_eq!(
            world.structural_journal.as_ref().unwrap().touched,
            [StableId(4)]
        );
        let peak = world.conservative_charged_publication_peak(0, 0).unwrap();
        let mut reservation = pool.try_reserve(peak).unwrap();
        let second = world.try_publish_charged(2, &mut reservation).unwrap();
        assert!(matches!(
            second.changes.as_slice(),
            [EntityChange::Spawn(EntityRecord {
                id: StableId(4),
                state: 44
            })]
        ));
        drop(second);
        drop(reservation);
        assert_eq!(world.by_id.allocation_size(), index_bytes);
        assert_eq!(
            world
                .structural_journal
                .as_ref()
                .unwrap()
                .changes
                .allocation_size(),
            journal_bytes
        );
        assert!(pool.used_bytes() >= retained);

        let mut current = StableId(4);
        for n in 5..=1004 {
            let next = StableId(n);
            let handle = world.handle_for(current).unwrap();
            world
                .apply_pair(WorldEdit::Despawn(handle), WorldEdit::Spawn(next, 1))
                .unwrap();
            let peak = world.conservative_charged_publication_peak(0, 0).unwrap();
            let mut reservation = pool.try_reserve(peak).unwrap();
            let publication = world
                .try_publish_charged(n as u64, &mut reservation)
                .unwrap();
            assert_eq!(publication.changes.len(), 2);
            drop(publication);
            drop(reservation);
            assert_eq!(world.by_id.allocation_size(), index_bytes);
            assert_eq!(
                world
                    .structural_journal
                    .as_ref()
                    .unwrap()
                    .changes
                    .allocation_size(),
                journal_bytes
            );
            current = next;
        }
        drop(initial);
        drop(world);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn structural_world_retires_exhausted_generation_and_refuses_full_high_water() {
        let pool = PublicationPool::new(100_000);
        let mut world = World::new(WorldEpoch(292));
        world.spawn(StableId(1), 1_u8).unwrap();
        let peak = world.conservative_charged_publication_peak(0, 0).unwrap();
        let mut reservation = pool.try_reserve(peak).unwrap();
        let initial = world.try_publish_charged(0, &mut reservation).unwrap();
        drop(reservation);
        world.activate_structural_journal(&pool, 2, 2, 3).unwrap();
        world.slots[0].generation = u64::MAX;
        let exhausted = world.handle_for(StableId(1)).unwrap();
        world.despawn(exhausted).unwrap();
        assert!(world.free.is_empty());
        let new = world.spawn(StableId(2), 2).unwrap();
        assert_eq!(new.slot, 1);
        assert_eq!(world.spawn(StableId(3), 3), Err(WorldError::SlotExhausted));
        assert!(world.handle_for(StableId(3)).is_none());
        assert_eq!(world.structural_journal.as_ref().unwrap().touched.len(), 2);
        let peak = world.conservative_charged_publication_peak(0, 0).unwrap();
        let mut reservation = pool.try_reserve(peak).unwrap();
        let publication = world.try_publish_charged(1, &mut reservation).unwrap();
        assert!(matches!(
            publication.changes.as_slice(),
            [
                EntityChange::Despawn(StableId(1)),
                EntityChange::Spawn(EntityRecord {
                    id: StableId(2),
                    ..
                })
            ]
        ));
        drop(publication);
        drop(reservation);
        drop(initial);
        drop(world);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn structural_mixed_neutral_and_live_changes_keep_requested_output_capacity() {
        let pool = PublicationPool::new(100_000);
        let mut world = World::new(WorldEpoch(293));
        world.spawn(StableId(1), 1_u8).unwrap();
        let peak = world.conservative_charged_publication_peak(0, 0).unwrap();
        let mut reservation = pool.try_reserve(peak).unwrap();
        let initial = world.try_publish_charged(0, &mut reservation).unwrap();
        drop(reservation);
        world.activate_structural_journal(&pool, 3, 2, 3).unwrap();
        world
            .apply_pair(
                WorldEdit::Spawn(StableId(2), 2),
                WorldEdit::DespawnNew(StableId(2)),
            )
            .unwrap();
        *world
            .get_mut(world.handle_for(StableId(1)).unwrap())
            .unwrap() = 11;
        let peak = world.conservative_charged_publication_peak(0, 0).unwrap();
        let mut reservation = pool.try_reserve(peak).unwrap();
        let publication = world.try_publish_charged(1, &mut reservation).unwrap();
        assert_eq!(publication.changes.len(), 1);
        assert!(publication.changes.capacity() >= 2);
        assert!(matches!(
            publication.changes[0],
            EntityChange::Update(EntityRecord {
                id: StableId(1),
                state: 11
            })
        ));
        drop(publication);
        drop(reservation);
        drop(initial);
        drop(world);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn absolute_structural_peak_uses_reusable_slot_without_phantom_page() {
        let pool = PublicationPool::new(200_000);
        let mut world = World::new(WorldEpoch(294));
        for id in 0..32 {
            world.spawn(StableId(id), id as u8).unwrap();
        }
        let peak = world.conservative_charged_publication_peak(0, 0).unwrap();
        let mut reservation = pool.try_reserve(peak).unwrap();
        let initial = world.try_publish_charged(0, &mut reservation).unwrap();
        drop(initial);
        drop(reservation);
        let handle = world.handle_for(StableId(1)).unwrap();
        world.despawn(handle).unwrap();
        let peak = world.conservative_charged_publication_peak(0, 0).unwrap();
        let mut reservation = pool.try_reserve(peak).unwrap();
        let deletion = world.try_publish_charged(1, &mut reservation).unwrap();
        drop(deletion);
        drop(reservation);
        assert_eq!(world.slots.len(), 32);
        assert_eq!(world.free.len(), 1);
        let absolute = world
            .conservative_charged_publication_peak_at(1, 32)
            .unwrap();
        let naive = world.conservative_charged_publication_peak(1, 1).unwrap();
        assert!(absolute < naive);
        world.spawn(StableId(33), 33).unwrap();
        assert_eq!(world.slots.len(), 32);
        assert!(world.required_charged_publication_bytes().unwrap() <= absolute);
    }

    #[test]
    fn fixed_slot_journal_retries_activation_and_publishes_sorted_changes() {
        let pool = PublicationPool::new(100_000);
        let mut world = World::new(WorldEpoch(251));
        for id in [4, 2, 1, 3] {
            world.spawn(StableId(id), id as u8).unwrap();
        }
        let peak = world.conservative_charged_publication_peak(0, 0).unwrap();
        let mut reservation = pool.try_reserve(peak).unwrap();
        let initial = world.try_publish_charged(0, &mut reservation).unwrap();
        drop(reservation);

        let before_free_capacity = world.free.capacity();
        let blocker = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes())
            .unwrap();
        assert_eq!(
            world.activate_fixed_slot_journal(&pool, 3),
            Err(PublicationCapacity)
        );
        assert!(world.fixed_journal.is_none());
        assert_eq!(world.free.capacity(), before_free_capacity);
        drop(blocker);
        world.activate_fixed_slot_journal(&pool, 3).unwrap();
        assert!(world.free.capacity() >= world.slots.len());

        let first = world.handle_for(StableId(1)).unwrap();
        assert_eq!(
            world.apply_pair(
                WorldEdit::Replace(first, 11),
                WorldEdit::Replace(
                    Handle {
                        generation: 9,
                        ..first
                    },
                    12
                ),
            ),
            Err(WorldError::InvalidHandle)
        );
        assert!(world.fixed_journal.as_ref().unwrap().touched.is_empty());
        *world
            .get_mut(world.handle_for(StableId(3)).unwrap())
            .unwrap() = 33;
        *world.get_mut(first).unwrap() = 11;
        *world.get_mut(first).unwrap() = 12;
        world
            .despawn(world.handle_for(StableId(2)).unwrap())
            .unwrap();
        let peak = world.conservative_charged_publication_peak(0, 0).unwrap();
        let mut reservation = pool.try_reserve(peak).unwrap();
        let occupied = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes())
            .unwrap();
        let publication = world.try_publish_charged(1, &mut reservation).unwrap();
        let changes = publication
            .changes
            .iter()
            .map(|change| match change {
                EntityChange::Spawn(record) | EntityChange::Update(record) => record.id,
                EntityChange::Despawn(id) => *id,
            })
            .collect::<Vec<_>>();
        assert_eq!(changes, [StableId(1), StableId(2), StableId(3)]);
        assert!(matches!(
            publication.changes[1],
            EntityChange::Despawn(StableId(2))
        ));
        assert_eq!(publication.snapshot.get(first).unwrap().state, 12);
        assert_eq!(initial.snapshot.get(first).unwrap().state, 1);
        assert!(world.fixed_journal.as_ref().unwrap().touched.is_empty());
        drop(occupied);
        drop(reservation);
        drop(publication);

        *world
            .get_mut(world.handle_for(StableId(4)).unwrap())
            .unwrap() = 44;
        let peak = world.conservative_charged_publication_peak(0, 0).unwrap();
        let mut reservation = pool.try_reserve(peak).unwrap();
        let publication = world.try_publish_charged(2, &mut reservation).unwrap();
        assert_eq!(publication.changes.len(), 1);
        assert!(
            matches!(&publication.changes[0], EntityChange::Update(record) if record.id == StableId(4))
        );
        drop(publication);
        drop(reservation);

        for id in [4, 1, 3] {
            world
                .despawn(world.handle_for(StableId(id)).unwrap())
                .unwrap();
        }
        assert_eq!(world.free.len(), world.slots.len());
        let peak = world.conservative_charged_publication_peak(0, 0).unwrap();
        let mut reservation = pool.try_reserve(peak).unwrap();
        let publication = world.try_publish_charged(3, &mut reservation).unwrap();
        assert_eq!(publication.changes.len(), 3);
        assert!(
            publication
                .changes
                .iter()
                .all(|change| matches!(change, EntityChange::Despawn(_)))
        );
        drop(publication);
        drop(reservation);
        drop(initial);
        drop(world);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn fixed_slot_journal_does_not_reuse_generation_exhausted_slot() {
        let pool = PublicationPool::new(100_000);
        let mut world = World::new(WorldEpoch(252));
        let handle = world.spawn(StableId(1), 1_u8).unwrap();
        world.slots[handle.slot as usize].generation = u64::MAX;
        let handle = Handle {
            generation: u64::MAX,
            ..handle
        };
        let peak = world.conservative_charged_publication_peak(0, 0).unwrap();
        let mut reservation = pool.try_reserve(peak).unwrap();
        let initial = world.try_publish_charged(0, &mut reservation).unwrap();
        drop(reservation);
        world.activate_fixed_slot_journal(&pool, 1).unwrap();
        world.despawn(handle).unwrap();
        assert!(world.free.is_empty());
        let peak = world.conservative_charged_publication_peak(0, 0).unwrap();
        let mut reservation = pool.try_reserve(peak).unwrap();
        let publication = world.try_publish_charged(1, &mut reservation).unwrap();
        assert!(matches!(
            publication.changes.as_slice(),
            [EntityChange::Despawn(StableId(1))]
        ));
        drop(publication);
        drop(reservation);
        drop(initial);
        drop(world);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn fixed_slot_journal_preserves_existing_free_slots_and_rejects_foreign_pool() {
        let pool = PublicationPool::new(100_000);
        let foreign = PublicationPool::new(100_000);
        let mut world = World::new(WorldEpoch(253));
        for id in 1..=3 {
            world.spawn(StableId(id), id as u8).unwrap();
        }
        world
            .despawn(world.handle_for(StableId(2)).unwrap())
            .unwrap();
        let peak = world.conservative_charged_publication_peak(0, 0).unwrap();
        let mut reservation = pool.try_reserve(peak).unwrap();
        let initial = world.try_publish_charged(0, &mut reservation).unwrap();
        drop(reservation);
        assert_eq!(world.free.len(), 1);
        assert_eq!(
            world.activate_fixed_slot_journal(&foreign, 2),
            Err(PublicationCapacity)
        );
        assert!(world.fixed_journal.is_none());
        assert_eq!(foreign.used_bytes(), 0);
        world.activate_fixed_slot_journal(&pool, 2).unwrap();
        world
            .despawn(world.handle_for(StableId(1)).unwrap())
            .unwrap();
        world
            .despawn(world.handle_for(StableId(3)).unwrap())
            .unwrap();
        assert_eq!(world.free.len(), 3);
        let peak = world.conservative_charged_publication_peak(0, 0).unwrap();
        let mut reservation = pool.try_reserve(peak).unwrap();
        let publication = world.try_publish_charged(1, &mut reservation).unwrap();
        assert_eq!(publication.changes.len(), 2);
        assert!(matches!(
            publication.changes.as_slice(),
            [
                EntityChange::Despawn(StableId(1)),
                EntityChange::Despawn(StableId(3))
            ]
        ));
        drop(publication);
        drop(reservation);
        drop(initial);
        drop(world);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn fixed_slot_activation_charges_empty_and_spare_retained_world_buffers() {
        let pool = PublicationPool::new(100_000);
        let mut empty = World::<u8>::new(WorldEpoch(254));
        let peak = empty.conservative_charged_publication_peak(0, 0).unwrap();
        let mut reservation = pool.try_reserve(peak).unwrap();
        let initial = empty.try_publish_charged(0, &mut reservation).unwrap();
        drop(reservation);
        let before = pool.used_bytes();
        empty.activate_fixed_slot_journal(&pool, 0).unwrap();
        assert_eq!(pool.used_bytes(), before);
        drop(initial);
        drop(empty);
        assert_eq!(pool.used_bytes(), 0);

        let mut world = World::new(WorldEpoch(255));
        world.slots.reserve(32);
        world.dirty.reserve(10);
        let handle = world.spawn(StableId(1), 1_u8).unwrap();
        world.despawn(handle).unwrap();
        let peak = world.conservative_charged_publication_peak(0, 0).unwrap();
        let mut reservation = pool.try_reserve(peak).unwrap();
        let initial = world.try_publish_charged(0, &mut reservation).unwrap();
        drop(reservation);
        assert!(world.by_id.is_empty());
        assert!(world.by_id.allocation_size() > 0);
        let retained = world.slots.capacity() * size_of::<Slot<u8>>()
            + world.dirty.capacity() * size_of::<bool>()
            + world.by_id.allocation_size()
            + world.slots.len() * size_of::<Option<ChangeKind>>()
            + size_of::<(StableId, usize)>()
            + world.free.capacity() * size_of::<u32>();
        let before = pool.used_bytes();
        let blocker = pool
            .try_reserve(pool.limit_bytes() - before - (retained - 1))
            .unwrap();
        assert_eq!(
            world.activate_fixed_slot_journal(&pool, 1),
            Err(PublicationCapacity)
        );
        assert_eq!(pool.used_bytes(), pool.limit_bytes() - (retained - 1));
        drop(blocker);
        world.activate_fixed_slot_journal(&pool, 1).unwrap();
        assert_eq!(pool.used_bytes(), before + retained);
        drop(initial);
        drop(world);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn live_publication_changes_follow_stable_id_order() {
        let mut world = World::new(WorldEpoch(240));
        world.spawn(StableId(3), 3_u8).unwrap();
        world.spawn(StableId(1), 1_u8).unwrap();
        world.spawn(StableId(2), 2_u8).unwrap();
        let publication = world.publish(0);
        let ids = publication
            .changes
            .iter()
            .map(|change| match change {
                EntityChange::Spawn(record) | EntityChange::Update(record) => record.id,
                EntityChange::Despawn(id) => *id,
            })
            .collect::<Vec<_>>();
        assert_eq!(ids, [StableId(1), StableId(2), StableId(3)]);
    }

    #[test]
    fn charged_snapshot_rejects_a_foreign_lease_floor_before_publication() {
        let pool = PublicationPool::new(100_000);
        let foreign = PublicationPool::new(100_000);
        let mut foreign_reservation = foreign
            .try_reserve(SnapshotLeaseFloor::required_charge().unwrap())
            .unwrap();
        let floor = SnapshotLeaseFloor::try_new(&mut foreign_reservation).unwrap();
        let mut world = World::new(WorldEpoch(300));
        world.spawn(StableId(1), 7_u64).unwrap();
        let peak = world.conservative_charged_publication_peak(0, 0).unwrap();
        let mut reservation = pool.try_reserve(peak).unwrap();
        assert_eq!(
            world
                .try_publish_charged_with_lease_floor(0, &mut reservation, &floor)
                .err(),
            Some(WorldPublicationError::WrongPool)
        );
        assert_eq!(world.published_tick, 0);
        assert_eq!(world.dirty_count, 1);
        assert_eq!(reservation.remaining_bytes(), peak);
        drop(floor);
        drop(foreign_reservation);
        assert_eq!(foreign.used_bytes(), 0);
    }

    #[test]
    fn prepared_batch_does_not_reuse_a_retired_generation() {
        let mut world = World::new(WorldEpoch(1));
        let original = world.spawn(StableId(1), 10_u64).unwrap();
        world.publish(0);
        world.slots[original.slot as usize].generation = u64::MAX;
        let retiring = Handle {
            generation: u64::MAX,
            ..original
        };

        world
            .apply_prepared(vec![
                WorldEdit::Despawn(retiring),
                WorldEdit::Spawn(StableId(2), 20),
            ])
            .unwrap();

        let new_handle = world.handle_for(StableId(2)).unwrap();
        assert_ne!(new_handle.slot, retiring.slot);
        assert_eq!(world.get(retiring), None);
        assert_eq!(world.get(new_handle).unwrap().state, 20);
        assert!(world.free.is_empty());
    }

    #[test]
    fn inline_pair_validates_before_either_edit_and_keeps_old_snapshot() {
        let mut world = World::new(WorldEpoch(9));
        let first = world.spawn(StableId(1), 10_u64).unwrap();
        let second = world.spawn(StableId(2), 20_u64).unwrap();
        let before = world.publish(0).snapshot;
        let invalid = Handle {
            generation: second.generation + 1,
            ..second
        };
        assert_eq!(
            world.apply_pair(
                WorldEdit::Replace(first, 11),
                WorldEdit::Replace(invalid, 21)
            ),
            Err(WorldError::InvalidHandle)
        );
        assert_eq!(world.get(first).unwrap().state, 10);
        assert_eq!(world.get(second).unwrap().state, 20);
        world
            .apply_pair(
                WorldEdit::Replace(first, 11),
                WorldEdit::Replace(second, 21),
            )
            .unwrap();
        assert_eq!(before.get(first).unwrap().state, 10);
        assert_eq!(before.get(second).unwrap().state, 20);
        let after = world.publish(1).snapshot;
        assert_eq!(after.get(first).unwrap().state, 11);
        assert_eq!(after.get(second).unwrap().state, 21);
    }

    #[test]
    fn charged_page_and_root_owners_release_only_after_the_last_reader() {
        let pool = PublicationPool::new(usize::MAX);
        let mut world = World::new(WorldEpoch(2));
        let first = world.spawn(StableId(1), 10_u64).unwrap();
        for id in 2..=64 {
            world.spawn(StableId(id), id as u64).unwrap();
        }
        let initial_peak = world.required_charged_publication_bytes().unwrap();
        let mut reservation = pool.try_reserve(initial_peak).unwrap();
        let old = world
            .try_publish_charged(0, &mut reservation)
            .unwrap()
            .snapshot;
        drop(reservation);
        let initial_retained = PageRootData::<u64>::charge_bytes(2).unwrap()
            + 2 * PageData::<u64>::charge_bytes(PAGE_SIZE).unwrap();
        assert_eq!(pool.used_bytes(), initial_retained);

        *world.get_mut(first).unwrap() = 11;
        let next_bytes = world.required_charged_publication_bytes().unwrap();
        let mut too_small = pool.try_reserve(next_bytes - 1).unwrap();
        assert_eq!(
            world.try_publish_charged(1, &mut too_small).err(),
            Some(WorldPublicationError::Capacity)
        );
        assert_eq!(too_small.remaining_bytes(), next_bytes - 1);
        drop(too_small);
        assert_eq!(pool.used_bytes(), initial_retained);
        assert_eq!(old.get(first).unwrap().state, 10);

        let mut reservation = pool.try_reserve(next_bytes).unwrap();
        let new = world
            .try_publish_charged(1, &mut reservation)
            .unwrap()
            .snapshot;
        drop(reservation);
        let next_retained = PageRootData::<u64>::charge_bytes(2).unwrap()
            + PageData::<u64>::charge_bytes(PAGE_SIZE).unwrap();
        assert_eq!(pool.used_bytes(), initial_retained + next_retained);
        assert_eq!(old.get(first).unwrap().state, 10);
        assert_eq!(new.get(first).unwrap().state, 11);

        let old_root = PageRootData::<u64>::charge_bytes(2).unwrap();
        let old_dirty_page = PageData::<u64>::charge_bytes(PAGE_SIZE).unwrap();
        drop(old);
        assert_eq!(
            pool.used_bytes(),
            initial_retained + next_retained - old_root - old_dirty_page
        );
        drop(new);
        drop(world);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn charged_publication_rejects_untracked_world_and_foreign_pool() {
        let pool = PublicationPool::new(usize::MAX);
        let foreign = PublicationPool::new(usize::MAX);
        let mut prototype = World::<u64>::new(WorldEpoch(3));
        prototype.publish(0);
        let mut reservation = pool.try_reserve(usize::MAX / 2).unwrap();
        assert_eq!(
            prototype.try_publish_charged(1, &mut reservation).err(),
            Some(WorldPublicationError::UnchargedWorld)
        );
        drop(reservation);

        let mut charged = World::<u64>::new(WorldEpoch(4));
        let bytes = charged.required_charged_publication_bytes().unwrap();
        let mut reservation = pool.try_reserve(bytes).unwrap();
        charged.try_publish_charged(0, &mut reservation).unwrap();
        drop(reservation);
        let mut foreign_reservation = foreign.try_reserve(100).unwrap();
        assert_eq!(
            charged
                .try_publish_charged(1, &mut foreign_reservation)
                .err(),
            Some(WorldPublicationError::WrongPool)
        );
        assert_eq!(charged.conservative_charged_publication_peak(0, 0), Ok(0));
    }

    #[test]
    fn charged_roots_from_two_worlds_compete_for_one_process_pool() {
        let mut first = World::new(WorldEpoch(5));
        let mut second = World::new(WorldEpoch(6));
        first.spawn(StableId(1), 1_u64).unwrap();
        second.spawn(StableId(2), 2_u64).unwrap();
        let first_peak = first.required_charged_publication_bytes().unwrap();
        let second_peak = second.required_charged_publication_bytes().unwrap();
        let first_retained = PageRootData::<u64>::charge_bytes(1).unwrap()
            + PageData::<u64>::charge_bytes(1).unwrap();
        let pool = PublicationPool::new(first_retained + second_peak - 1);
        let mut reservation = pool.try_reserve(first_peak).unwrap();
        let held = first
            .try_publish_charged(0, &mut reservation)
            .unwrap()
            .snapshot;
        drop(reservation);
        assert!(pool.try_reserve(second_peak).is_err());
        drop(first);
        assert!(pool.try_reserve(second_peak).is_err());
        drop(held);
        let mut reservation = pool.try_reserve(second_peak).unwrap();
        let published = second.try_publish_charged(0, &mut reservation).unwrap();
        assert_eq!(published.snapshot.records().count(), 1);
        drop(published);
        drop(reservation);
        drop(second);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn precommit_page_bound_covers_net_zero_structural_high_water() {
        let mut world = World::new(WorldEpoch(7));
        let peak = world
            .conservative_charged_publication_peak(130, 65)
            .unwrap();
        let too_small = PublicationPool::new(peak - 1);
        assert!(too_small.try_reserve(peak).is_err());
        assert!(world.is_empty());
        assert_eq!(
            world.required_charged_publication_bytes().unwrap(),
            2 * PageRootData::<u64>::charge_bytes(0).unwrap()
        );

        let pool = PublicationPool::new(peak);
        let mut reservation = pool.try_reserve(peak).unwrap();
        let edits = (1..=65)
            .map(|id| WorldEdit::Spawn(StableId(id), id as u64))
            .chain((1..=65).map(|id| WorldEdit::DespawnNew(StableId(id))))
            .collect();
        world.apply_prepared(edits).unwrap();
        assert!(world.is_empty());
        assert!(world.required_charged_publication_bytes().unwrap() <= peak);
        let published = world.try_publish_charged(0, &mut reservation).unwrap();
        assert_eq!(published.work.copied_pages, 3);
        assert!(published.snapshot.records().next().is_none());
        drop(published);
        drop(reservation);
        drop(world);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn precommit_page_bound_covers_existing_dirty_and_partial_page_growth() {
        let pool = PublicationPool::new(usize::MAX);
        let mut world = World::new(WorldEpoch(8));
        for id in 1..=31 {
            world.spawn(StableId(id), id as u64).unwrap();
        }
        let mut reservation = pool
            .try_reserve(world.required_charged_publication_bytes().unwrap())
            .unwrap();
        world.try_publish_charged(0, &mut reservation).unwrap();
        drop(reservation);

        let handle = world.handle_for(StableId(1)).unwrap();
        *world.get_mut(handle).unwrap() = 99;
        let peak = world.conservative_charged_publication_peak(2, 2).unwrap();
        let mut reservation = pool.try_reserve(peak).unwrap();
        world.spawn(StableId(32), 32).unwrap();
        world.spawn(StableId(33), 33).unwrap();
        assert!(world.required_charged_publication_bytes().unwrap() <= peak);
        let published = world.try_publish_charged(1, &mut reservation).unwrap();
        assert_eq!(published.work.copied_pages, 2);
        assert_eq!(published.snapshot.records().count(), 33);
        drop(published);
        drop(reservation);
        drop(world);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn group_change_bound_covers_existing_and_cancelled_structural_edits() {
        let mut world = World::new(WorldEpoch(9));
        world.spawn(StableId(1), 1_u64).unwrap();
        assert_eq!(world.conservative_change_count(130).unwrap(), 131);
        assert!(world.conservative_change_count(usize::MAX).is_err());
        let edits = (2..=66)
            .map(|id| WorldEdit::Spawn(StableId(id), id as u64))
            .chain((2..=66).map(|id| WorldEdit::DespawnNew(StableId(id))))
            .collect();
        world.apply_prepared(edits).unwrap();
        assert!(world.changed.len() <= 131);
        assert_eq!(world.publish(0).changes.len(), 1);

        // Distinct new IDs attain the additive bound even when existing
        // unpublished work was performed before the admitted tick.
        world.spawn(StableId(67), 67).unwrap();
        assert_eq!(world.conservative_change_count(2).unwrap(), 3);
        world.spawn(StableId(68), 68).unwrap();
        world.spawn(StableId(69), 69).unwrap();
        assert_eq!(world.publish(1).changes.len(), 3);
    }
}
