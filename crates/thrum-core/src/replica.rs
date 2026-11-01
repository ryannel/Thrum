use std::{
    alloc::Layout,
    collections::{BTreeMap, BTreeSet, HashMap},
};

use crate::{
    ActionResult, ClosedPublicationValue, EntityChange, EntityRecord, PublicationCharge,
    PublicationPool, ResultDisposition, StableId,
    authority::{CommittedEvent, MAX_RESULTS, PeerId},
    publication_pool::PublicationColumn,
    replica_index::ChargedReplicaIndex,
    world::AuthorityId,
};

/// One atomic in-memory publication. Wire framing, schema and byte limits belong to I2n.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateGroup<T, E, R> {
    /// Standalone session or match authority that produced the group.
    pub authority: AuthorityId,
    /// Monotonic publication ID within that authority.
    pub group_id: u64,
    /// Exact committed state tick represented here.
    pub commit_tick: u64,
    /// Highest authority event ID represented by this complete state.
    pub event_cursor: u64,
    /// Full changed state together with lifecycle, including spawn's first state.
    pub changes: Vec<EntityChange<T>>,
    /// Every newly resolved action represented by this group's state.
    pub results: Vec<ActionResult<R>>,
    /// Immediate committed events with stable identity.
    pub events: Vec<CommittedEvent<E>>,
    /// Per-peer contiguous sequence watermark; individual higher results remain named.
    pub resolved_through: Vec<(PeerId, u64)>,
}

/// A complete group whose columns share one charged owner per nonempty buffer.
/// The pool identity also covers an all-empty publication.
pub struct ChargedStateGroup<T, E, R> {
    pub(crate) pool: PublicationPool,
    pub(crate) authority: AuthorityId,
    pub(crate) group_id: u64,
    pub(crate) commit_tick: u64,
    pub(crate) event_cursor: u64,
    pub(crate) changes: PublicationColumn<EntityChange<T>>,
    pub(crate) results: PublicationColumn<ActionResult<R>>,
    pub(crate) events: PublicationColumn<CommittedEvent<E>>,
    pub(crate) resolved_through: PublicationColumn<(PeerId, u64)>,
}

impl<T, E, R> Clone for ChargedStateGroup<T, E, R> {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            authority: self.authority,
            group_id: self.group_id,
            commit_tick: self.commit_tick,
            event_cursor: self.event_cursor,
            changes: self.changes.clone(),
            results: self.results.clone(),
            events: self.events.clone(),
            resolved_through: self.resolved_through.clone(),
        }
    }
}

impl<T, E, R> ChargedStateGroup<T, E, R> {
    pub(crate) fn belongs_to(&self, pool: &PublicationPool) -> bool {
        self.pool.same_pool(pool)
            && self.changes.belongs_to(pool)
            && self.results.belongs_to(pool)
            && self.events.belongs_to(pool)
            && self.resolved_through.belongs_to(pool)
    }

    /// Authority that produced this baseline or group.
    pub fn authority(&self) -> AuthorityId {
        self.authority
    }

    /// Monotonic publication ID within that authority.
    pub fn group_id(&self) -> u64 {
        self.group_id
    }

    /// Exact committed state tick represented here.
    pub fn commit_tick(&self) -> u64 {
        self.commit_tick
    }

    /// Highest committed event ID represented here.
    pub fn event_cursor(&self) -> u64 {
        self.event_cursor
    }

    /// Full changed state together with lifecycle. Live charged groups carry
    /// changes in strictly increasing stable-ID order.
    pub fn changes(&self) -> &[EntityChange<T>] {
        self.changes.as_slice()
    }

    /// Resolved action outcomes in this publication.
    pub fn results(&self) -> &[ActionResult<R>] {
        self.results.as_slice()
    }

    /// Immediate committed events in ID order.
    pub fn events(&self) -> &[CommittedEvent<E>] {
        self.events.as_slice()
    }

    /// Contiguous per-peer resolved sequence watermarks.
    pub fn resolved_through(&self) -> &[(PeerId, u64)] {
        self.resolved_through.as_slice()
    }
}

/// Replica admission failure; no state from the invalid group is published.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplicaError {
    /// Prototype and charged group storage cannot be mixed across one replica.
    UnqualifiedGroup,
    /// A charged group belongs to another publication pool.
    WrongPool,
    /// The finite replica object profile cannot hold the group's state.
    Capacity,
    /// Authority identity differs from the admitted baseline.
    WrongAuthority,
    /// Group ID is not the next publication or tick did not advance.
    OutOfOrder,
    /// A stable object ID appeared twice in one group or baseline.
    DuplicateObject,
    /// Spawn/update/despawn was inconsistent with the previously admitted state.
    InvalidLifecycle,
    /// An event ID repeated or moved backwards.
    InvalidEvent,
    /// A committed result refers to an immediate event absent from this group.
    IncompleteOutcome,
}

/// Failure while preparing a charged replica baseline before it is visible.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChargedReplicaSetupError {
    /// A group column belongs to another publication pool.
    WrongPool,
    /// The declared object capacity or shared pool cannot admit the baseline.
    Capacity,
    /// The baseline failed the same semantic checks as the prototype path.
    Invalid(ReplicaError),
}

/// Validated results and events released only after atomic state admission.
pub struct AdmittedGroup<E, R> {
    /// Results represented by the newly admitted state.
    pub results: Vec<ActionResult<R>>,
    /// Events represented by the newly admitted state.
    pub events: Vec<CommittedEvent<E>>,
    /// Contiguous resolved input prefixes for controlled peers.
    pub resolved_through: Vec<(PeerId, u64)>,
}

/// Admitted outcomes whose columns retain one shared charge each.
pub struct ChargedAdmittedGroup<E, R> {
    results: PublicationColumn<ActionResult<R>>,
    events: PublicationColumn<CommittedEvent<E>>,
    resolved_through: PublicationColumn<(PeerId, u64)>,
}

impl<E, R> Clone for ChargedAdmittedGroup<E, R> {
    fn clone(&self) -> Self {
        Self {
            results: self.results.clone(),
            events: self.events.clone(),
            resolved_through: self.resolved_through.clone(),
        }
    }
}

impl<E, R> ChargedAdmittedGroup<E, R> {
    /// Outcomes represented by the admitted state.
    pub fn results(&self) -> &[ActionResult<R>] {
        self.results.as_slice()
    }

    /// Immediate committed events represented by the admitted state.
    pub fn events(&self) -> &[CommittedEvent<E>] {
        self.events.as_slice()
    }

    /// Contiguous resolved input prefixes for controlled peers.
    pub fn resolved_through(&self) -> &[(PeerId, u64)] {
        self.resolved_through.as_slice()
    }
}

/// Confirmed, non-authoritative object state.
pub struct Replica<T> {
    authority: AuthorityId,
    group_id: u64,
    tick: u64,
    objects: ObjectStore<T>,
    publication_pool: Option<PublicationPool>,
    last_event_id: u64,
    progress: ProgressStore,
}

enum ObjectStore<T> {
    Prototype(HashMap<StableId, T>),
    Charged(ChargedReplicaIndex<T>),
}

impl<T> ObjectStore<T> {
    fn contains_key(&self, id: &StableId) -> bool {
        match self {
            Self::Prototype(map) => map.contains_key(id),
            Self::Charged(index) => index.contains(*id),
        }
    }

    fn get(&self, id: &StableId) -> Option<&T> {
        match self {
            Self::Prototype(map) => map.get(id),
            Self::Charged(index) => index.get(*id),
        }
    }

    fn len(&self) -> usize {
        match self {
            Self::Prototype(map) => map.len(),
            Self::Charged(index) => index.len(),
        }
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Clone, Default)]
struct PeerProgress {
    connection_epoch: Option<u64>,
    resolved_through: u64,
    resolved_above_gap: BTreeSet<u64>,
}

enum ProgressStore {
    Prototype(BTreeMap<PeerId, PeerProgress>),
    Charged(ChargedProgress),
}

#[derive(Clone, Copy)]
struct ChargedPeerProgress {
    peer: PeerId,
    connection_epoch: Option<u64>,
    resolved_through: u64,
    above_gap: [u64; MAX_RESULTS / 64],
}

struct ChargedProgress {
    confirmed: Vec<ChargedPeerProgress>,
    candidate: Vec<ChargedPeerProgress>,
    max_peers: usize,
    _charge: Option<PublicationCharge>,
}

impl ChargedProgress {
    fn try_from_baseline<E, R>(
        commit_tick: u64,
        event_cursor: u64,
        results: &[ActionResult<R>],
        events: &[CommittedEvent<E>],
        watermarks: &[(PeerId, u64)],
        pool: &PublicationPool,
        max_peers: usize,
    ) -> Result<Self, ChargedReplicaSetupError> {
        validate_event_links(0, commit_tick, event_cursor, results, events)
            .map_err(ChargedReplicaSetupError::Invalid)?;
        if watermarks.len() > max_peers {
            return Err(ChargedReplicaSetupError::Capacity);
        }
        let progress_bytes = Layout::array::<ChargedPeerProgress>(max_peers)
            .map_err(|_| ChargedReplicaSetupError::Capacity)?
            .size()
            .checked_mul(2)
            .ok_or(ChargedReplicaSetupError::Capacity)?;
        let key_bytes = Layout::array::<(PeerId, u64)>(results.len())
            .map_err(|_| ChargedReplicaSetupError::Capacity)?
            .size();
        let total_bytes = progress_bytes
            .checked_add(key_bytes)
            .ok_or(ChargedReplicaSetupError::Capacity)?;
        let mut reservation = pool
            .try_reserve(total_bytes)
            .map_err(|_| ChargedReplicaSetupError::Capacity)?;
        let charge = if progress_bytes == 0 {
            None
        } else {
            Some(
                reservation
                    .charge(progress_bytes)
                    .map_err(|_| ChargedReplicaSetupError::Capacity)?,
            )
        };
        let _key_charge = if key_bytes == 0 {
            None
        } else {
            Some(
                reservation
                    .charge(key_bytes)
                    .map_err(|_| ChargedReplicaSetupError::Capacity)?,
            )
        };
        let mut keys = Vec::with_capacity(results.len());
        keys.extend(
            results
                .iter()
                .map(|result| (result.origin.peer, result.origin.sequence)),
        );
        keys.sort_unstable();
        if keys.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(ChargedReplicaSetupError::Invalid(
                ReplicaError::IncompleteOutcome,
            ));
        }
        let mut confirmed = Vec::with_capacity(max_peers);
        let mut previous = None;
        for (peer, prefix) in watermarks {
            if previous.is_some_and(|old| *peer <= old) {
                return Err(ChargedReplicaSetupError::Invalid(
                    ReplicaError::IncompleteOutcome,
                ));
            }
            previous = Some(*peer);
            confirmed.push(ChargedPeerProgress {
                peer: *peer,
                connection_epoch: None,
                resolved_through: *prefix,
                above_gap: [0; MAX_RESULTS / 64],
            });
        }
        for result in results {
            let index = confirmed
                .binary_search_by_key(&result.origin.peer, |row| row.peer)
                .map_err(|_| ChargedReplicaSetupError::Invalid(ReplicaError::IncompleteOutcome))?;
            let row = &mut confirmed[index];
            if row
                .connection_epoch
                .is_some_and(|epoch| epoch != result.origin.connection_epoch)
            {
                return Err(ChargedReplicaSetupError::Invalid(
                    ReplicaError::IncompleteOutcome,
                ));
            }
            row.connection_epoch = Some(result.origin.connection_epoch);
            if result.origin.sequence > row.resolved_through {
                let offset = result
                    .origin
                    .sequence
                    .checked_sub(row.resolved_through.checked_add(1).ok_or(
                        ChargedReplicaSetupError::Invalid(ReplicaError::IncompleteOutcome),
                    )?)
                    .and_then(|offset| usize::try_from(offset).ok())
                    .filter(|offset| *offset < MAX_RESULTS)
                    .ok_or(ChargedReplicaSetupError::Invalid(
                        ReplicaError::IncompleteOutcome,
                    ))?;
                row.above_gap[offset / 64] |= 1_u64 << (offset % 64);
            }
        }
        Ok(Self {
            confirmed,
            candidate: Vec::with_capacity(max_peers),
            max_peers,
            _charge: charge,
        })
    }

    fn unchanged(&self, watermarks: &[(PeerId, u64)]) -> bool {
        watermarks.len() == self.confirmed.len()
            && watermarks
                .iter()
                .zip(&self.confirmed)
                .all(|((id, mark), row)| {
                    *id == row.peer
                        && *mark == row.resolved_through
                        && row.above_gap == [0; MAX_RESULTS / 64]
                })
    }

    fn prepare<R>(
        &mut self,
        results: &[ActionResult<R>],
        watermarks: &[(PeerId, u64)],
    ) -> Result<(), ReplicaError> {
        if watermarks.len() > self.max_peers {
            return Err(ReplicaError::Capacity);
        }
        self.candidate.clear();
        self.candidate.extend_from_slice(&self.confirmed);
        let mut previous = None;
        for (peer, _) in watermarks {
            if previous.is_some_and(|old| *peer <= old) {
                return Err(ReplicaError::IncompleteOutcome);
            }
            previous = Some(*peer);
            if let Err(index) = self.candidate.binary_search_by_key(peer, |row| row.peer) {
                if self.candidate.len() == self.max_peers {
                    return Err(ReplicaError::Capacity);
                }
                self.candidate.insert(
                    index,
                    ChargedPeerProgress {
                        peer: *peer,
                        connection_epoch: None,
                        resolved_through: 0,
                        above_gap: [0; MAX_RESULTS / 64],
                    },
                );
            }
        }
        for result in results {
            let index = self
                .candidate
                .binary_search_by_key(&result.origin.peer, |row| row.peer)
                .map_err(|_| ReplicaError::IncompleteOutcome)?;
            let row = &mut self.candidate[index];
            if row
                .connection_epoch
                .is_some_and(|epoch| epoch != result.origin.connection_epoch)
            {
                return Err(ReplicaError::IncompleteOutcome);
            }
            let offset = result
                .origin
                .sequence
                .checked_sub(
                    row.resolved_through
                        .checked_add(1)
                        .ok_or(ReplicaError::IncompleteOutcome)?,
                )
                .and_then(|offset| usize::try_from(offset).ok())
                .filter(|offset| *offset < MAX_RESULTS)
                .ok_or(ReplicaError::IncompleteOutcome)?;
            let bit = 1_u64 << (offset % 64);
            if row.above_gap[offset / 64] & bit != 0 {
                return Err(ReplicaError::IncompleteOutcome);
            }
            row.above_gap[offset / 64] |= bit;
            row.connection_epoch = Some(result.origin.connection_epoch);
        }
        for (peer, watermark) in watermarks {
            let index = self
                .candidate
                .binary_search_by_key(peer, |row| row.peer)
                .expect("advertised peer inserted");
            let row = &mut self.candidate[index];
            while row.above_gap[0] & 1 != 0 {
                for word in 0..row.above_gap.len() - 1 {
                    row.above_gap[word] =
                        (row.above_gap[word] >> 1) | (row.above_gap[word + 1] << 63);
                }
                let last = row.above_gap.len() - 1;
                row.above_gap[last] >>= 1;
                row.resolved_through += 1;
            }
            if row.resolved_through != *watermark {
                return Err(ReplicaError::IncompleteOutcome);
            }
        }
        Ok(())
    }

    fn commit(&mut self) {
        std::mem::swap(&mut self.confirmed, &mut self.candidate);
    }
}

enum ProgressUpdate {
    None,
    Prototype(BTreeMap<PeerId, PeerProgress>),
    ChargedCandidate,
}

struct ValidatedNext {
    idle: bool,
    progress: ProgressUpdate,
}

fn validate_event_links<E, R>(
    previous_event_id: u64,
    commit_tick: u64,
    event_cursor: u64,
    results: &[ActionResult<R>],
    events: &[CommittedEvent<E>],
) -> Result<(), ReplicaError> {
    let mut last = previous_event_id;
    for event in events {
        if event.id <= last || event.id > event_cursor || event.tick > commit_tick {
            return Err(ReplicaError::InvalidEvent);
        }
        last = event.id;
    }
    let mut matched = 0;
    for result in results {
        if result.resolved_tick > commit_tick
            || matches!(result.disposition, ResultDisposition::Rejected(_))
                && result.event_id.is_some()
            || result.event_id.is_some_and(|id| {
                !events
                    .binary_search_by_key(&id, |event| event.id)
                    .ok()
                    .is_some_and(|index| {
                        events[index].origin == result.origin
                            && events[index].tick == result.resolved_tick
                    })
            })
        {
            return Err(ReplicaError::IncompleteOutcome);
        }
        matched += usize::from(result.event_id.is_some());
    }
    if matched != events.len() {
        return Err(ReplicaError::IncompleteOutcome);
    }
    Ok(())
}

fn validate_baseline_outcomes<E, R>(
    commit_tick: u64,
    event_cursor: u64,
    results: &[ActionResult<R>],
    events: &[CommittedEvent<E>],
    resolved_through: &[(PeerId, u64)],
) -> Result<BTreeMap<PeerId, PeerProgress>, ReplicaError> {
    validate_event_links(0, commit_tick, event_cursor, results, events)?;
    let mut progress = BTreeMap::new();
    for (peer, prefix) in resolved_through {
        if progress
            .insert(
                *peer,
                PeerProgress {
                    connection_epoch: None,
                    resolved_through: *prefix,
                    resolved_above_gap: BTreeSet::new(),
                },
            )
            .is_some()
        {
            return Err(ReplicaError::IncompleteOutcome);
        }
    }
    let mut baseline_result_ids = BTreeSet::new();
    for result in results {
        if !baseline_result_ids.insert((result.origin.peer, result.origin.sequence)) {
            return Err(ReplicaError::IncompleteOutcome);
        }
        let Some(peer) = progress.get_mut(&result.origin.peer) else {
            return Err(ReplicaError::IncompleteOutcome);
        };
        if peer
            .connection_epoch
            .is_some_and(|epoch| epoch != result.origin.connection_epoch)
        {
            return Err(ReplicaError::IncompleteOutcome);
        }
        peer.connection_epoch = Some(result.origin.connection_epoch);
        if result.origin.sequence > peer.resolved_through
            && !peer.resolved_above_gap.insert(result.origin.sequence)
        {
            return Err(ReplicaError::IncompleteOutcome);
        }
    }
    Ok(progress)
}

impl<T> Replica<T> {
    pub(crate) fn ready_for(&self, authority: AuthorityId, tick: u64, group_id: u64) -> bool {
        self.authority == authority && self.tick == tick && self.group_id == group_id
    }

    fn validate_next<E, R>(
        &mut self,
        authority: AuthorityId,
        group_id: u64,
        commit_tick: u64,
        event_cursor: u64,
        changes: &[EntityChange<T>],
        results: &[ActionResult<R>],
        events: &[CommittedEvent<E>],
        resolved_through: &[(PeerId, u64)],
    ) -> Result<ValidatedNext, ReplicaError> {
        if authority != self.authority {
            return Err(ReplicaError::WrongAuthority);
        }
        if group_id != self.group_id + 1 || commit_tick <= self.tick {
            return Err(ReplicaError::OutOfOrder);
        }
        if event_cursor < self.last_event_id {
            return Err(ReplicaError::InvalidEvent);
        }
        let unchanged_progress = results.is_empty()
            && match &self.progress {
                ProgressStore::Prototype(progress) => {
                    resolved_through.len() == progress.len()
                        && resolved_through.iter().zip(progress).all(
                            |((peer_id, watermark), (known_id, progress))| {
                                peer_id == known_id
                                    && *watermark == progress.resolved_through
                                    && progress.resolved_above_gap.is_empty()
                            },
                        )
                }
                ProgressStore::Charged(progress) => progress.unchanged(resolved_through),
            };
        if changes.is_empty() && events.is_empty() && unchanged_progress {
            return Ok(ValidatedNext {
                idle: true,
                progress: ProgressUpdate::None,
            });
        }
        let charged_changes = matches!(self.progress, ProgressStore::Charged(_));
        let mut changed = BTreeSet::new();
        let mut previous_change = None;
        for change in changes {
            let id = match change {
                EntityChange::Spawn(record) | EntityChange::Update(record) => record.id,
                EntityChange::Despawn(id) => *id,
            };
            if charged_changes {
                if let Some(previous) = previous_change {
                    if id == previous {
                        return Err(ReplicaError::DuplicateObject);
                    }
                    if id < previous {
                        return Err(ReplicaError::IncompleteOutcome);
                    }
                }
                previous_change = Some(id);
            } else if !changed.insert(id) {
                return Err(ReplicaError::DuplicateObject);
            }
            let exists = self.objects.contains_key(&id);
            if match change {
                EntityChange::Spawn(_) => exists,
                EntityChange::Update(_) | EntityChange::Despawn(_) => !exists,
            } {
                return Err(ReplicaError::InvalidLifecycle);
            }
        }
        validate_event_links(
            self.last_event_id,
            commit_tick,
            event_cursor,
            results,
            events,
        )?;
        let new_progress = if unchanged_progress {
            ProgressUpdate::None
        } else if let ProgressStore::Charged(progress) = &mut self.progress {
            progress.prepare(results, resolved_through)?;
            ProgressUpdate::ChargedCandidate
        } else {
            let ProgressStore::Prototype(current) = &self.progress else {
                unreachable!("charged progress handled above")
            };
            let mut progress = current.clone();
            let mut advertised = BTreeMap::new();
            for (peer_id, watermark) in resolved_through {
                if advertised.insert(*peer_id, *watermark).is_some() {
                    return Err(ReplicaError::IncompleteOutcome);
                }
                progress.entry(*peer_id).or_default();
            }
            for result in results {
                let Some(peer) = progress.get_mut(&result.origin.peer) else {
                    return Err(ReplicaError::IncompleteOutcome);
                };
                if peer
                    .connection_epoch
                    .is_some_and(|epoch| epoch != result.origin.connection_epoch)
                    || result.origin.sequence <= peer.resolved_through
                    || !peer.resolved_above_gap.insert(result.origin.sequence)
                {
                    return Err(ReplicaError::IncompleteOutcome);
                }
                peer.connection_epoch = Some(result.origin.connection_epoch);
            }
            for (peer_id, watermark) in advertised {
                let peer = progress
                    .get_mut(&peer_id)
                    .expect("advertised peer inserted");
                while let Some(next) = peer.resolved_through.checked_add(1) {
                    if !peer.resolved_above_gap.remove(&next) {
                        break;
                    }
                    peer.resolved_through = next;
                }
                if peer.resolved_through != watermark {
                    return Err(ReplicaError::IncompleteOutcome);
                }
            }
            ProgressUpdate::Prototype(progress)
        };
        Ok(ValidatedNext {
            idle: false,
            progress: new_progress,
        })
    }

    /// Admit a complete baseline at once, including outstanding results above
    /// a pending sequence gap. The baseline consists only of full spawns.
    pub fn from_baseline<E, R>(
        baseline: StateGroup<T, E, R>,
    ) -> Result<(Self, AdmittedGroup<E, R>), ReplicaError> {
        let StateGroup {
            authority,
            group_id,
            commit_tick,
            event_cursor,
            changes,
            results,
            events,
            resolved_through,
        } = baseline;
        let mut objects = HashMap::new();
        for change in changes {
            let EntityChange::Spawn(EntityRecord { id, state }) = change else {
                return Err(ReplicaError::InvalidLifecycle);
            };
            if objects.insert(id, state).is_some() {
                return Err(ReplicaError::DuplicateObject);
            }
        }
        let progress = validate_baseline_outcomes(
            commit_tick,
            event_cursor,
            &results,
            &events,
            &resolved_through,
        )?;
        Ok((
            Self {
                authority,
                group_id,
                tick: commit_tick,
                objects: ObjectStore::Prototype(objects),
                publication_pool: None,
                last_event_id: event_cursor,
                progress: ProgressStore::Prototype(progress),
            },
            AdmittedGroup {
                results,
                events,
                resolved_through,
            },
        ))
    }
    /// Admit a sealed baseline into finite object and peer-progress storage.
    /// The default peer limit is the baseline's current peer count; use the
    /// explicit-limit constructor when later charged groups may add peers.
    pub fn from_charged_baseline<E, R>(
        baseline: ChargedStateGroup<T, E, R>,
        pool: &PublicationPool,
        max_objects: usize,
    ) -> Result<(Self, ChargedAdmittedGroup<E, R>), ChargedReplicaSetupError>
    where
        T: ClosedPublicationValue,
    {
        let max_peers = baseline.resolved_through().len();
        Self::from_charged_baseline_with_peer_limit(baseline, pool, max_objects, max_peers)
    }

    /// Admit a sealed baseline with an explicit maximum peer count. Confirmed
    /// and candidate progress buffers are both charged before visibility.
    pub fn from_charged_baseline_with_peer_limit<E, R>(
        baseline: ChargedStateGroup<T, E, R>,
        pool: &PublicationPool,
        max_objects: usize,
        max_peers: usize,
    ) -> Result<(Self, ChargedAdmittedGroup<E, R>), ChargedReplicaSetupError>
    where
        T: ClosedPublicationValue,
    {
        if !baseline.belongs_to(pool) {
            return Err(ChargedReplicaSetupError::WrongPool);
        }
        if baseline.changes().len() > max_objects {
            return Err(ChargedReplicaSetupError::Capacity);
        }
        let ChargedStateGroup {
            pool: _,
            authority,
            group_id,
            commit_tick,
            event_cursor,
            changes,
            results,
            events,
            resolved_through,
        } = baseline;
        let progress = ChargedProgress::try_from_baseline(
            commit_tick,
            event_cursor,
            results.as_slice(),
            events.as_slice(),
            resolved_through.as_slice(),
            pool,
            max_peers,
        )?;
        let mut index = ChargedReplicaIndex::try_new(pool, max_objects)
            .map_err(|_| ChargedReplicaSetupError::Capacity)?;
        for change in changes.as_slice() {
            let EntityChange::Spawn(EntityRecord { id, state }) = change else {
                return Err(ChargedReplicaSetupError::Invalid(
                    ReplicaError::InvalidLifecycle,
                ));
            };
            index.insert_new(*id, state.clone()).map_err(|error| {
                ChargedReplicaSetupError::Invalid(match error {
                    crate::replica_index::IndexError::Duplicate => ReplicaError::DuplicateObject,
                    crate::replica_index::IndexError::Full => ReplicaError::InvalidLifecycle,
                    crate::replica_index::IndexError::Missing => unreachable!(),
                })
            })?;
        }
        Ok((
            Self {
                authority,
                group_id,
                tick: commit_tick,
                objects: ObjectStore::Charged(index),
                publication_pool: Some(pool.clone()),
                last_event_id: event_cursor,
                progress: ProgressStore::Charged(progress),
            },
            ChargedAdmittedGroup {
                results,
                events,
                resolved_through,
            },
        ))
    }
    /// Last completely admitted authority tick.
    pub fn tick(&self) -> u64 {
        self.tick
    }
    /// Read confirmed object state by stable identity.
    pub fn get(&self, id: StableId) -> Option<&T> {
        self.objects.get(&id)
    }
    /// Number of fully admitted objects.
    pub fn len(&self) -> usize {
        self.objects.len()
    }
    /// Whether the replica has no admitted objects.
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }
    /// Requested table bytes retained by a charged object index, if selected.
    pub fn charged_object_bytes(&self) -> Option<usize> {
        match &self.objects {
            ObjectStore::Prototype(_) => None,
            ObjectStore::Charged(index) => Some(index.allocation_bytes()),
        }
    }
    /// Conservative object-capacity check for a required charged replica.
    /// The host calls this with the authority's maximum possible spawns
    /// before allowing a tick to commit.
    pub fn can_admit_charged_spawns(&self, max_spawns: usize) -> Result<bool, ReplicaError> {
        let ObjectStore::Charged(index) = &self.objects else {
            return Err(ReplicaError::UnqualifiedGroup);
        };
        Ok(index.can_add(max_spawns))
    }

    /// Atomically validate a shared charged group, then admit it into the
    /// finite object index. Peer-progress candidates are pool-charged.
    pub fn apply_charged<E, R>(
        &mut self,
        group: ChargedStateGroup<T, E, R>,
    ) -> Result<ChargedAdmittedGroup<E, R>, ReplicaError>
    where
        T: ClosedPublicationValue,
    {
        let pool = self
            .publication_pool
            .as_ref()
            .ok_or(ReplicaError::UnqualifiedGroup)?;
        if !group.belongs_to(pool) {
            return Err(ReplicaError::WrongPool);
        }
        let validated = self.validate_next(
            group.authority,
            group.group_id,
            group.commit_tick,
            group.event_cursor,
            group.changes(),
            group.results(),
            group.events(),
            group.resolved_through(),
        )?;
        let changes = group.changes();
        let spawns = changes
            .iter()
            .filter(|change| matches!(change, EntityChange::Spawn(_)))
            .count();
        let despawns = changes
            .iter()
            .filter(|change| matches!(change, EntityChange::Despawn(_)))
            .count();
        let ObjectStore::Charged(index) = &self.objects else {
            unreachable!("charged pool checked above")
        };
        if !index
            .len()
            .checked_sub(despawns)
            .and_then(|after_removal| after_removal.checked_add(spawns))
            .is_some_and(|final_count| final_count <= index.max_live())
        {
            return Err(ReplicaError::Capacity);
        }
        let ChargedStateGroup {
            pool: _,
            authority: _,
            group_id,
            commit_tick,
            event_cursor,
            changes,
            results,
            events,
            resolved_through,
        } = group;
        if !validated.idle {
            let ObjectStore::Charged(index) = &mut self.objects else {
                unreachable!("charged pool checked above")
            };
            // Distinct IDs were validated already. Free despawns first so
            // final-count admission also bounds transient occupancy.
            for change in changes.as_slice() {
                if let EntityChange::Despawn(id) = change {
                    index.remove(*id).expect("validated despawn");
                }
            }
            let mut admit = |change: EntityChange<T>| match change {
                EntityChange::Spawn(record) => index
                    .insert_new(record.id, record.state)
                    .expect("validated precharged spawn"),
                EntityChange::Update(record) => {
                    index
                        .replace(record.id, record.state)
                        .expect("validated update");
                }
                EntityChange::Despawn(_) => {}
            };
            match changes.try_into_unique_iter() {
                Ok(values) => {
                    for change in values {
                        admit(change);
                    }
                }
                Err(shared) => {
                    for change in shared.as_slice().iter().cloned() {
                        admit(change);
                    }
                }
            }
        }
        self.group_id = group_id;
        self.tick = commit_tick;
        self.last_event_id = event_cursor;
        match validated.progress {
            ProgressUpdate::None => {}
            ProgressUpdate::ChargedCandidate => {
                let ProgressStore::Charged(progress) = &mut self.progress else {
                    unreachable!("charged group checked above")
                };
                progress.commit();
            }
            ProgressUpdate::Prototype(_) => unreachable!("charged group checked above"),
        }
        Ok(ChargedAdmittedGroup {
            results,
            events,
            resolved_through,
        })
    }

    /// Atomically validate and admit the next full-state group.
    pub fn apply<E, R>(
        &mut self,
        group: StateGroup<T, E, R>,
    ) -> Result<AdmittedGroup<E, R>, ReplicaError> {
        if matches!(self.objects, ObjectStore::Charged(_)) {
            return Err(ReplicaError::UnqualifiedGroup);
        }
        let validated = self.validate_next(
            group.authority,
            group.group_id,
            group.commit_tick,
            group.event_cursor,
            &group.changes,
            &group.results,
            &group.events,
            &group.resolved_through,
        )?;
        if validated.idle {
            self.group_id = group.group_id;
            self.tick = group.commit_tick;
            self.last_event_id = group.event_cursor;
            return Ok(AdmittedGroup {
                results: group.results,
                events: group.events,
                resolved_through: group.resolved_through,
            });
        }
        let ObjectStore::Prototype(objects) = &mut self.objects else {
            unreachable!("prototype group checked above")
        };
        for change in group.changes {
            match change {
                EntityChange::Spawn(record) | EntityChange::Update(record) => {
                    objects.insert(record.id, record.state);
                }
                EntityChange::Despawn(id) => {
                    objects.remove(&id);
                }
            }
        }
        self.group_id = group.group_id;
        self.tick = group.commit_tick;
        self.last_event_id = group.event_cursor;
        match validated.progress {
            ProgressUpdate::None => {}
            ProgressUpdate::Prototype(progress) => {
                self.progress = ProgressStore::Prototype(progress);
            }
            ProgressUpdate::ChargedCandidate => unreachable!("prototype group checked above"),
        }
        Ok(AdmittedGroup {
            results: group.results,
            events: group.events,
            resolved_through: group.resolved_through,
        })
    }
}

#[cfg(test)]
mod charged_tests {
    use super::*;
    use crate::publication_pool::PublicationColumn;
    use crate::{MatchId, Origin};

    fn result(sequence: u64) -> ActionResult<()> {
        ActionResult {
            origin: Origin {
                peer: PeerId(1),
                connection_epoch: 1,
                sequence,
            },
            resolved_tick: sequence,
            disposition: ResultDisposition::Committed,
            event_id: None,
        }
    }

    #[test]
    fn charged_progress_candidate_rolls_back_malformed_final_peer_and_closes_gap() {
        let pool = PublicationPool::new(100_000);
        let mut progress = ChargedProgress::try_from_baseline::<(), ()>(
            2,
            0,
            &[result(2)],
            &[],
            &[(PeerId(1), 0)],
            &pool,
            2,
        )
        .unwrap();
        let charged = pool.used_bytes();
        assert!(charged > 0);
        assert_eq!(
            progress.prepare(&[result(1)], &[(PeerId(1), 2), (PeerId(2), 1)]),
            Err(ReplicaError::IncompleteOutcome)
        );
        assert_eq!(progress.confirmed[0].resolved_through, 0);
        assert_eq!(progress.confirmed.len(), 1);
        assert_eq!(pool.used_bytes(), charged);
        progress
            .prepare(&[result(1)], &[(PeerId(1), 2), (PeerId(2), 0)])
            .unwrap();
        progress.commit();
        assert_eq!(progress.confirmed.len(), 2);
        assert_eq!(progress.confirmed[0].resolved_through, 2);
        assert_eq!(progress.confirmed[0].above_gap, [0; MAX_RESULTS / 64]);
        assert_eq!(pool.used_bytes(), charged);
        drop(progress);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn charged_progress_tracks_full_window_and_rejects_excess_peer_capacity() {
        let pool = PublicationPool::new(100_000);
        assert!(matches!(
            ChargedProgress::try_from_baseline(
                0,
                0,
                &[] as &[ActionResult<()>],
                &[] as &[CommittedEvent<()>],
                &[(PeerId(1), 0)],
                &pool,
                0,
            ),
            Err(ChargedReplicaSetupError::Capacity)
        ));
        let mut progress = ChargedProgress::try_from_baseline(
            0,
            0,
            &[] as &[ActionResult<()>],
            &[] as &[CommittedEvent<()>],
            &[(PeerId(1), 0)],
            &pool,
            1,
        )
        .unwrap();
        let above_gap = (2..=MAX_RESULTS as u64).map(result).collect::<Vec<_>>();
        progress.prepare(&above_gap, &[(PeerId(1), 0)]).unwrap();
        progress.commit();
        assert_eq!(progress.confirmed[0].resolved_through, 0);
        assert_eq!(
            progress.prepare(&[result(1)], &[(PeerId(1), 256), (PeerId(2), 0)]),
            Err(ReplicaError::Capacity)
        );
        assert_eq!(progress.confirmed[0].resolved_through, 0);
        progress.prepare(&[result(1)], &[(PeerId(1), 256)]).unwrap();
        progress.commit();
        assert_eq!(progress.confirmed[0].resolved_through, 256);
        assert_eq!(progress.confirmed[0].above_gap, [0; MAX_RESULTS / 64]);
        drop(progress);
        assert_eq!(pool.used_bytes(), 0);
    }

    fn group(
        pool: &PublicationPool,
        group_id: u64,
        tick: u64,
        changes: Vec<EntityChange<u8>>,
    ) -> ChargedStateGroup<u8, (), ()> {
        let mut reservation = pool
            .try_reserve(
                PublicationColumn::<EntityChange<u8>>::required_charge(changes.len()).unwrap(),
            )
            .unwrap();
        let mut column =
            PublicationColumn::try_with_capacity(&mut reservation, changes.len()).unwrap();
        for change in changes {
            column.push(change);
        }
        ChargedStateGroup {
            pool: pool.clone(),
            authority: AuthorityId::Match(MatchId(240)),
            group_id,
            commit_tick: tick,
            event_cursor: 0,
            changes: column,
            results: PublicationColumn::try_with_capacity(&mut reservation, 0).unwrap(),
            events: PublicationColumn::try_with_capacity(&mut reservation, 0).unwrap(),
            resolved_through: PublicationColumn::try_with_capacity(&mut reservation, 0).unwrap(),
        }
    }

    fn progress_group(
        pool: &PublicationPool,
        group_id: u64,
        changes: Vec<EntityChange<u8>>,
        results: Vec<ActionResult<()>>,
        watermarks: &[(PeerId, u64)],
    ) -> ChargedStateGroup<u8, (), ()> {
        let bytes = PublicationColumn::<EntityChange<u8>>::required_charge(changes.len()).unwrap()
            + PublicationColumn::<ActionResult<()>>::required_charge(results.len()).unwrap()
            + PublicationColumn::<(PeerId, u64)>::required_charge(watermarks.len()).unwrap();
        let mut reservation = pool.try_reserve(bytes).unwrap();
        let mut change_column =
            PublicationColumn::try_with_capacity(&mut reservation, changes.len()).unwrap();
        for change in changes {
            change_column.push(change);
        }
        let mut result_column =
            PublicationColumn::try_with_capacity(&mut reservation, results.len()).unwrap();
        for result in results {
            result_column.push(result);
        }
        let mut watermark_column =
            PublicationColumn::try_with_capacity(&mut reservation, watermarks.len()).unwrap();
        for watermark in watermarks {
            watermark_column.push(*watermark);
        }
        ChargedStateGroup {
            pool: pool.clone(),
            authority: AuthorityId::Match(MatchId(240)),
            group_id,
            commit_tick: group_id,
            event_cursor: 0,
            changes: change_column,
            results: result_column,
            events: PublicationColumn::empty(),
            resolved_through: watermark_column,
        }
    }

    #[test]
    fn charged_group_rejects_bad_final_peer_before_confirming_progress_or_tick() {
        let pool = PublicationPool::new(100_000);
        let spawn = || {
            EntityChange::Spawn(EntityRecord {
                id: StableId(1),
                state: 1,
            })
        };
        let update = || {
            EntityChange::Update(EntityRecord {
                id: StableId(1),
                state: 2,
            })
        };
        let baseline = progress_group(&pool, 0, vec![spawn()], vec![], &[(PeerId(1), 0)]);
        let (mut tight, tight_initial) =
            Replica::from_charged_baseline(baseline, &pool, 1).unwrap();
        let excess = progress_group(&pool, 1, vec![], vec![], &[(PeerId(1), 0), (PeerId(2), 0)]);
        assert!(matches!(
            tight.apply_charged(excess),
            Err(ReplicaError::Capacity)
        ));
        assert_eq!(tight.tick(), 0);
        drop(tight_initial);
        drop(tight);

        let baseline = progress_group(&pool, 0, vec![spawn()], vec![], &[(PeerId(1), 0)]);
        let (mut replica, initial) =
            Replica::from_charged_baseline_with_peer_limit(baseline, &pool, 1, 2).unwrap();
        let bad = progress_group(
            &pool,
            1,
            vec![update()],
            vec![result(1)],
            &[(PeerId(1), 1), (PeerId(2), 1)],
        );
        assert!(matches!(
            replica.apply_charged(bad),
            Err(ReplicaError::IncompleteOutcome)
        ));
        assert_eq!(replica.tick(), 0);
        assert_eq!(replica.get(StableId(1)), Some(&1));
        let valid = progress_group(
            &pool,
            1,
            vec![update()],
            vec![result(1)],
            &[(PeerId(1), 1), (PeerId(2), 0)],
        );
        let admitted = replica.apply_charged(valid).unwrap();
        assert_eq!(replica.tick(), 1);
        assert_eq!(replica.get(StableId(1)), Some(&2));
        assert_eq!(
            admitted.resolved_through(),
            &[(PeerId(1), 1), (PeerId(2), 0)]
        );
        drop(admitted);
        drop(initial);
        drop(replica);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn charged_progress_baseline_reserves_both_buffers_and_rejects_duplicate_below_prefix() {
        let baseline_bytes = PublicationColumn::<(PeerId, u64)>::required_charge(1).unwrap();
        let progress_bytes = Layout::array::<ChargedPeerProgress>(1).unwrap().size() * 2;
        let tight = PublicationPool::new(baseline_bytes + progress_bytes - 1);
        let baseline = progress_group(&tight, 0, vec![], vec![], &[(PeerId(1), 0)]);
        assert!(matches!(
            Replica::from_charged_baseline(baseline, &tight, 0),
            Err(ChargedReplicaSetupError::Capacity)
        ));
        assert_eq!(tight.used_bytes(), 0);

        let pool = PublicationPool::new(100_000);
        let duplicate = progress_group(
            &pool,
            1,
            vec![],
            vec![result(1), result(1)],
            &[(PeerId(1), 1)],
        );
        assert!(matches!(
            Replica::from_charged_baseline(duplicate, &pool, 0),
            Err(ChargedReplicaSetupError::Invalid(
                ReplicaError::IncompleteOutcome
            ))
        ));
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn charged_baseline_result_keys_are_reserved_with_progress_before_copying() {
        let baseline_bytes = PublicationColumn::<ActionResult<()>>::required_charge(1).unwrap()
            + PublicationColumn::<(PeerId, u64)>::required_charge(1).unwrap();
        let progress_bytes = Layout::array::<ChargedPeerProgress>(1).unwrap().size() * 2;
        let key_bytes = Layout::array::<(PeerId, u64)>(1).unwrap().size();
        let pool = PublicationPool::new(baseline_bytes + progress_bytes + key_bytes - 1);
        let baseline = progress_group(&pool, 1, vec![], vec![result(1)], &[(PeerId(1), 1)]);
        assert!(matches!(
            Replica::from_charged_baseline(baseline, &pool, 0),
            Err(ChargedReplicaSetupError::Capacity)
        ));
        assert_eq!(pool.used_bytes(), 0);

        let pool = PublicationPool::new(baseline_bytes + progress_bytes + key_bytes);
        let baseline = progress_group(&pool, 1, vec![], vec![result(1)], &[(PeerId(1), 1)]);
        let (replica, admitted) = Replica::from_charged_baseline(baseline, &pool, 0).unwrap();
        assert_eq!(replica.tick(), 1);
        assert_eq!(pool.used_bytes(), baseline_bytes + progress_bytes);
        drop(admitted);
        drop(replica);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn charged_baseline_requires_ordered_peer_watermarks_but_prototype_keeps_its_order() {
        let pool = PublicationPool::new(100_000);
        let watermarks = [(PeerId(2), 0), (PeerId(1), 0)];
        let charged = progress_group(&pool, 0, vec![], vec![], &watermarks);
        assert!(matches!(
            Replica::from_charged_baseline(charged, &pool, 0),
            Err(ChargedReplicaSetupError::Invalid(
                ReplicaError::IncompleteOutcome
            ))
        ));
        assert_eq!(pool.used_bytes(), 0);
        let prototype = StateGroup::<u8, (), ()> {
            authority: AuthorityId::Match(MatchId(240)),
            group_id: 0,
            commit_tick: 0,
            event_cursor: 0,
            changes: vec![],
            results: vec![],
            events: vec![],
            resolved_through: watermarks.into(),
        };
        assert!(Replica::from_baseline(prototype).is_ok());
    }

    #[test]
    fn charged_baseline_rejects_out_of_window_gap_and_mixed_epochs() {
        let pool = PublicationPool::new(100_000);
        let distant = result(MAX_RESULTS as u64 + 1);
        let charged = progress_group(
            &pool,
            distant.resolved_tick,
            vec![],
            vec![distant.clone()],
            &[(PeerId(1), 0)],
        );
        assert!(matches!(
            Replica::from_charged_baseline(charged, &pool, 0),
            Err(ChargedReplicaSetupError::Invalid(
                ReplicaError::IncompleteOutcome
            ))
        ));
        let prototype = StateGroup::<u8, (), ()> {
            authority: AuthorityId::Match(MatchId(240)),
            group_id: distant.resolved_tick,
            commit_tick: distant.resolved_tick,
            event_cursor: 0,
            changes: vec![],
            results: vec![distant],
            events: vec![],
            resolved_through: vec![(PeerId(1), 0)],
        };
        assert!(Replica::from_baseline(prototype).is_ok());

        let mut next_epoch = result(2);
        next_epoch.origin.connection_epoch = 2;
        let charged = progress_group(
            &pool,
            2,
            vec![],
            vec![result(1), next_epoch],
            &[(PeerId(1), 2)],
        );
        assert!(matches!(
            Replica::from_charged_baseline(charged, &pool, 0),
            Err(ChargedReplicaSetupError::Invalid(
                ReplicaError::IncompleteOutcome
            ))
        ));
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn charged_baseline_retains_out_of_order_results_until_gap_closes() {
        let pool = PublicationPool::new(100_000);
        let baseline = progress_group(&pool, 2, vec![], vec![result(2)], &[(PeerId(1), 0)]);
        let (mut replica, initial) = Replica::from_charged_baseline(baseline, &pool, 0).unwrap();
        assert_eq!(replica.tick(), 2);
        let closing = progress_group(&pool, 3, vec![], vec![result(1)], &[(PeerId(1), 2)]);
        let admitted = replica.apply_charged(closing).unwrap();
        assert_eq!(replica.tick(), 3);
        assert_eq!(admitted.resolved_through(), &[(PeerId(1), 2)]);
        drop(admitted);
        drop(initial);
        drop(replica);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn charged_live_group_requires_sorted_change_ids_before_mutation() {
        let pool = PublicationPool::new(100_000);
        let baseline = group(
            &pool,
            0,
            0,
            vec![
                EntityChange::Spawn(EntityRecord {
                    id: StableId(2),
                    state: 2,
                }),
                EntityChange::Spawn(EntityRecord {
                    id: StableId(1),
                    state: 1,
                }),
            ],
        );
        let (mut replica, initial) = Replica::from_charged_baseline(baseline, &pool, 2).unwrap();
        let reversed = group(
            &pool,
            1,
            1,
            vec![
                EntityChange::Update(EntityRecord {
                    id: StableId(2),
                    state: 3,
                }),
                EntityChange::Update(EntityRecord {
                    id: StableId(1),
                    state: 4,
                }),
            ],
        );
        assert!(matches!(
            replica.apply_charged(reversed),
            Err(ReplicaError::IncompleteOutcome)
        ));
        assert_eq!(replica.tick(), 0);
        assert_eq!(replica.get(StableId(1)), Some(&1));
        assert_eq!(replica.get(StableId(2)), Some(&2));
        let sorted = group(
            &pool,
            1,
            1,
            vec![
                EntityChange::Update(EntityRecord {
                    id: StableId(1),
                    state: 4,
                }),
                EntityChange::Update(EntityRecord {
                    id: StableId(2),
                    state: 3,
                }),
            ],
        );
        replica.apply_charged(sorted).unwrap();
        assert_eq!(replica.get(StableId(1)), Some(&4));
        assert_eq!(replica.get(StableId(2)), Some(&3));
        drop(initial);
        drop(replica);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn charged_replica_validates_before_change_and_admits_net_zero_capacity() {
        let pool = PublicationPool::new(100_000);
        let foreign = PublicationPool::new(100_000);
        let baseline = group(
            &pool,
            0,
            0,
            vec![EntityChange::Spawn(EntityRecord {
                id: StableId(1),
                state: 1,
            })],
        );
        let (mut replica, initial) = Replica::from_charged_baseline(baseline, &pool, 1).unwrap();
        assert_eq!(replica.get(StableId(1)), Some(&1));
        assert_eq!(replica.can_admit_charged_spawns(1), Ok(false));

        let invalid = group(
            &pool,
            1,
            1,
            vec![
                EntityChange::Update(EntityRecord {
                    id: StableId(1),
                    state: 2,
                }),
                EntityChange::Update(EntityRecord {
                    id: StableId(1),
                    state: 3,
                }),
            ],
        );
        assert_eq!(
            replica.apply_charged(invalid).err(),
            Some(ReplicaError::DuplicateObject)
        );
        assert_eq!(replica.tick(), 0);
        assert_eq!(replica.get(StableId(1)), Some(&1));

        let wrong_pool = group(&foreign, 1, 1, vec![]);
        assert_eq!(
            replica.apply_charged(wrong_pool).err(),
            Some(ReplicaError::WrongPool)
        );
        assert_eq!(replica.tick(), 0);

        let valid = group(
            &pool,
            1,
            1,
            vec![
                EntityChange::Despawn(StableId(1)),
                EntityChange::Spawn(EntityRecord {
                    id: StableId(2),
                    state: 2,
                }),
            ],
        );
        let admitted = replica.apply_charged(valid).unwrap();
        assert_eq!(replica.tick(), 1);
        assert_eq!(replica.get(StableId(1)), None);
        assert_eq!(replica.get(StableId(2)), Some(&2));
        assert!(admitted.results().is_empty());

        let overfull = group(
            &pool,
            2,
            2,
            vec![EntityChange::Spawn(EntityRecord {
                id: StableId(3),
                state: 3,
            })],
        );
        assert_eq!(
            replica.apply_charged(overfull).err(),
            Some(ReplicaError::Capacity)
        );
        assert_eq!(replica.tick(), 1);
        assert_eq!(replica.get(StableId(3)), None);
        drop(initial);
        drop(admitted);
        drop(replica);
        assert_eq!(pool.used_bytes(), 0);
        assert_eq!(foreign.used_bytes(), 0);
    }
}
