use std::{
    collections::{BTreeMap, HashMap},
    ops::Deref,
    sync::{Arc, Weak},
};

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

enum VirtualObjects {
    Inline([Option<(StableId, VirtualObject)>; 2]),
    Many(HashMap<StableId, VirtualObject>),
}

impl VirtualObjects {
    fn for_edits(count: usize) -> Self {
        if count <= 2 {
            Self::Inline([None; 2])
        } else {
            Self::Many(HashMap::with_capacity(count))
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
    by_id: HashMap<StableId, u32>,
    dirty: Vec<bool>,
    dirty_count: usize,
    changed: BTreeMap<StableId, ChangeKind>,
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
    /// Construct an empty world with a fresh host-supplied epoch.
    pub fn new(epoch: WorldEpoch) -> Self {
        Self {
            epoch,
            slots: Vec::new(),
            free: Vec::new(),
            by_id: HashMap::new(),
            dirty: Vec::new(),
            dirty_count: 0,
            changed: BTreeMap::new(),
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
        self.changed.entry(id).or_insert(ChangeKind::Update);
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
                    change: self.changed.get(&id).copied(),
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
        let mut generations = HashMap::new();
        let mut spawned = HashMap::new();
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
        if self.by_id.contains_key(&id) {
            return Err(WorldError::DuplicateId);
        }
        if matches!(self.changed.get(&id), Some(ChangeKind::Despawn)) {
            return Err(WorldError::DuplicateId);
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
        self.mark_dirty(index / PAGE_SIZE);
        self.changed.insert(id, ChangeKind::Spawn);
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
        if matches!(self.changed.get(&record.id), Some(ChangeKind::Spawn)) {
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
        self.changed
            .len()
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
        if matches!(self.publication_mode, PublicationMode::Uncharged) {
            return Err(WorldPublicationError::UnchargedWorld);
        }
        let first = matches!(self.publication_mode, PublicationMode::Unselected);
        let possible_slots = self
            .slots
            .len()
            .checked_add(max_spawns)
            .ok_or(WorldPublicationError::Capacity)?;
        let possible_pages = possible_slots.div_ceil(PAGE_SIZE);
        let touches = self
            .dirty_count
            .checked_add(max_edits.max(max_spawns))
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
        let mut changes = Vec::with_capacity(self.changed.len());
        for (id, kind) in std::mem::take(&mut self.changed) {
            match kind {
                ChangeKind::Spawn | ChangeKind::Update => {
                    let slot = self.by_id[&id] as usize;
                    let record = self.slots[slot]
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
