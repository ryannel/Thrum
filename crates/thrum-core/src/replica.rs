use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::{
    ActionResult, EntityChange, EntityRecord, ResultDisposition, StableId,
    authority::{CommittedEvent, PeerId},
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

/// Replica admission failure; no state from the invalid group is published.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplicaError {
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

/// Validated results and events released only after atomic state admission.
pub struct AdmittedGroup<E, R> {
    /// Results represented by the newly admitted state.
    pub results: Vec<ActionResult<R>>,
    /// Events represented by the newly admitted state.
    pub events: Vec<CommittedEvent<E>>,
    /// Contiguous resolved input prefixes for controlled peers.
    pub resolved_through: Vec<(PeerId, u64)>,
}

/// Confirmed, non-authoritative object state.
pub struct Replica<T> {
    authority: AuthorityId,
    group_id: u64,
    tick: u64,
    objects: HashMap<StableId, T>,
    last_event_id: u64,
    progress: BTreeMap<PeerId, PeerProgress>,
}

#[derive(Clone, Default)]
struct PeerProgress {
    connection_epoch: Option<u64>,
    resolved_through: u64,
    resolved_above_gap: BTreeSet<u64>,
}

impl<T> Replica<T> {
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
        let mut unmatched_events = BTreeMap::new();
        let mut last_event_id = 0;
        for event in &events {
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
        for (peer, prefix) in &resolved_through {
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
        for result in &results {
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
        Ok((
            Self {
                authority,
                group_id,
                tick: commit_tick,
                objects,
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
    /// Atomically validate and admit the next full-state group.
    pub fn apply<E, R>(
        &mut self,
        group: StateGroup<T, E, R>,
    ) -> Result<AdmittedGroup<E, R>, ReplicaError> {
        if group.authority != self.authority {
            return Err(ReplicaError::WrongAuthority);
        }
        if group.group_id != self.group_id + 1 || group.commit_tick <= self.tick {
            return Err(ReplicaError::OutOfOrder);
        }
        if group.event_cursor < self.last_event_id {
            return Err(ReplicaError::InvalidEvent);
        }
        // The authority emits sorted peer watermarks. An empty group with the
        // exact existing progress needs no cloned map or validation sets.
        // Reordered or newly registered peers take the full validation path.
        if group.changes.is_empty()
            && group.results.is_empty()
            && group.events.is_empty()
            && group.resolved_through.len() == self.progress.len()
            && group.resolved_through.iter().zip(&self.progress).all(
                |((peer_id, watermark), (known_id, progress))| {
                    peer_id == known_id
                        && *watermark == progress.resolved_through
                        && progress.resolved_above_gap.is_empty()
                },
            )
        {
            self.group_id = group.group_id;
            self.tick = group.commit_tick;
            self.last_event_id = group.event_cursor;
            return Ok(AdmittedGroup {
                results: group.results,
                events: group.events,
                resolved_through: group.resolved_through,
            });
        }
        let mut changed = BTreeSet::new();
        for change in &group.changes {
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
        for event in &group.events {
            if event.id <= last
                || event.id > group.event_cursor
                || event.tick > group.commit_tick
                || unmatched_events
                    .insert(event.id, (event.origin, event.tick))
                    .is_some()
            {
                return Err(ReplicaError::InvalidEvent);
            }
            last = event.id;
        }
        for result in &group.results {
            if result.resolved_tick > group.commit_tick {
                return Err(ReplicaError::IncompleteOutcome);
            }
            if matches!(result.disposition, ResultDisposition::Rejected(_))
                && result.event_id.is_some()
            {
                return Err(ReplicaError::IncompleteOutcome);
            }
            if result.event_id.is_some_and(|id| {
                unmatched_events.remove(&id) != Some((result.origin, result.resolved_tick))
            }) {
                return Err(ReplicaError::IncompleteOutcome);
            }
        }
        if !unmatched_events.is_empty() {
            return Err(ReplicaError::IncompleteOutcome);
        }
        let mut progress = self.progress.clone();
        let mut advertised = BTreeMap::new();
        for (peer_id, watermark) in &group.resolved_through {
            if advertised.insert(*peer_id, *watermark).is_some() {
                return Err(ReplicaError::IncompleteOutcome);
            }
            progress.entry(*peer_id).or_default();
        }
        for result in &group.results {
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
        for change in group.changes {
            match change {
                EntityChange::Spawn(record) | EntityChange::Update(record) => {
                    self.objects.insert(record.id, record.state);
                }
                EntityChange::Despawn(id) => {
                    self.objects.remove(&id);
                }
            }
        }
        self.group_id = group.group_id;
        self.tick = group.commit_tick;
        self.last_event_id = group.event_cursor;
        self.progress = progress;
        Ok(AdmittedGroup {
            results: group.results,
            events: group.events,
            resolved_through: group.resolved_through,
        })
    }
}
