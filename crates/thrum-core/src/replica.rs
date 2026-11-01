use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::{
    ActionResult, ClosedPublicationValue, EntityChange, EntityRecord, PublicationPool,
    ResultDisposition, StableId,
    authority::{CommittedEvent, PeerId},
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

    /// Full changed state together with lifecycle.
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
    progress: BTreeMap<PeerId, PeerProgress>,
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

struct ValidatedNext {
    idle: bool,
    progress: Option<BTreeMap<PeerId, PeerProgress>>,
}

fn validate_baseline_outcomes<E, R>(
    commit_tick: u64,
    event_cursor: u64,
    results: &[ActionResult<R>],
    events: &[CommittedEvent<E>],
    resolved_through: &[(PeerId, u64)],
) -> Result<BTreeMap<PeerId, PeerProgress>, ReplicaError> {
    let mut unmatched_events = BTreeMap::new();
    let mut last_event_id = 0;
    for event in events {
        if event.id <= last_event_id
            || event.id > event_cursor
            || event.tick > commit_tick
            || unmatched_events
                .insert(event.id, (event.origin, event.tick))
                .is_some()
        {
            return Err(ReplicaError::InvalidEvent);
        }
        last_event_id = event.id;
    }
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
        if result.resolved_tick > commit_tick
            || matches!(result.disposition, ResultDisposition::Rejected(_))
                && result.event_id.is_some()
            || result.event_id.is_some_and(|id| {
                unmatched_events.remove(&id) != Some((result.origin, result.resolved_tick))
            })
            || !baseline_result_ids.insert((result.origin.peer, result.origin.sequence))
        {
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
    if !unmatched_events.is_empty() {
        return Err(ReplicaError::IncompleteOutcome);
    }
    Ok(progress)
}

impl<T> Replica<T> {
    fn validate_next<E, R>(
        &self,
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
            && resolved_through.len() == self.progress.len()
            && resolved_through.iter().zip(&self.progress).all(
                |((peer_id, watermark), (known_id, progress))| {
                    peer_id == known_id
                        && *watermark == progress.resolved_through
                        && progress.resolved_above_gap.is_empty()
                },
            );
        if changes.is_empty() && events.is_empty() && unchanged_progress {
            return Ok(ValidatedNext {
                idle: true,
                progress: None,
            });
        }
        let mut changed = BTreeSet::new();
        for change in changes {
            let id = match change {
                EntityChange::Spawn(record) | EntityChange::Update(record) => record.id,
                EntityChange::Despawn(id) => *id,
            };
            if !changed.insert(id) {
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
        let mut unmatched_events = BTreeMap::new();
        let mut last = self.last_event_id;
        for event in events {
            if event.id <= last
                || event.id > event_cursor
                || event.tick > commit_tick
                || unmatched_events
                    .insert(event.id, (event.origin, event.tick))
                    .is_some()
            {
                return Err(ReplicaError::InvalidEvent);
            }
            last = event.id;
        }
        for result in results {
            if result.resolved_tick > commit_tick
                || matches!(result.disposition, ResultDisposition::Rejected(_))
                    && result.event_id.is_some()
                || result.event_id.is_some_and(|id| {
                    unmatched_events.remove(&id) != Some((result.origin, result.resolved_tick))
                })
            {
                return Err(ReplicaError::IncompleteOutcome);
            }
        }
        if !unmatched_events.is_empty() {
            return Err(ReplicaError::IncompleteOutcome);
        }
        let new_progress = if unchanged_progress {
            None
        } else {
            let mut progress = self.progress.clone();
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
            Some(progress)
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
                progress,
            },
            AdmittedGroup {
                results,
                events,
                resolved_through,
            },
        ))
    }
    /// Admit a sealed baseline into a precharged finite object table. Peer
    /// progress and validation scratch are still outside the pool gate.
    pub fn from_charged_baseline<E, R>(
        baseline: ChargedStateGroup<T, E, R>,
        pool: &PublicationPool,
        max_objects: usize,
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
        let progress = validate_baseline_outcomes(
            commit_tick,
            event_cursor,
            results.as_slice(),
            events.as_slice(),
            resolved_through.as_slice(),
        )
        .map_err(ChargedReplicaSetupError::Invalid)?;
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
                progress,
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
    /// finite object index. Progress scratch is not yet pool-charged.
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
        if let Some(progress) = validated.progress {
            self.progress = progress;
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
        if let Some(progress) = validated.progress {
            self.progress = progress;
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
    use crate::MatchId;
    use crate::publication_pool::PublicationColumn;

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
                EntityChange::Spawn(EntityRecord {
                    id: StableId(2),
                    state: 2,
                }),
                EntityChange::Despawn(StableId(1)),
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
