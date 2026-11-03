//! Bounded, allocation-stable R4 state/lifecycle replica after an admitted baseline.
//! Transport sequence, baseline staging and game-specific schema closure remain
//! separate owners. Outcome/event/watermark admission is deliberately refused
//! until one bounded progress owner can admit it atomically with state.

use crate::progress::{Progress, ProgressError};
use crate::r4::{Actor, GroupView, Lifecycle};

/// First-profile maximum stable partitions.
pub const MAX_PARTITIONS: usize = 4;
/// First-profile maximum simultaneously visible actors.
pub const MAX_VISIBLE: usize = 128;
/// First-profile maximum distinct wire IDs assigned since baseline admission.
pub const MAX_ASSIGNMENTS: usize = 256;
const MAX_PARTITION_ACTORS: usize = 32;

/// A manifest's four distinct lifecycle ordinals.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LifecycleCodes {
    /// First appearance of a never-assigned wire ID.
    pub spawn: u8,
    /// Terminal removal of a wire ID.
    pub despawn: u8,
    /// Temporary interest loss.
    pub leave: u8,
    /// Return after temporary interest loss.
    pub reenter: u8,
}

impl LifecycleCodes {
    fn valid(self) -> bool {
        let codes = [self.spawn, self.despawn, self.leave, self.reenter];
        codes
            .iter()
            .enumerate()
            .all(|(i, code)| !codes[..i].contains(code))
    }

    fn enters(self, code: u8) -> Option<bool> {
        if code == self.spawn || code == self.reenter {
            Some(true)
        } else if code == self.despawn || code == self.leave {
            Some(false)
        } else {
            None
        }
    }
}

/// A baseline-bound partition with its complete initially visible actor state.
pub struct PartitionSeed<'a> {
    /// Stable baseline-bound partition ID.
    pub group_id: u32,
    /// Initial nonzero partition interest generation.
    pub interest_generation: u32,
    /// Common baseline sample tick.
    pub sample_tick: u64,
    /// Complete actor set for this partition.
    pub actors: &'a [Actor],
}

/// Game-provided manifest and dependency-closure validator. This runs before
/// generic admission and may inspect the current replica. It must be pure;
/// a refusal leaves the replica unchanged. Implementations must check schema
/// ordinals, actor ranges, flags, event origins and dependency closure.
pub trait SchemaValidator {
    /// Validate all initial content and dependency closure before visibility.
    fn baseline(&self, seeds: &[PartitionSeed<'_>]) -> bool;
    /// Validate manifest ordinals, fields and closure for one live group.
    fn group(&self, replica: &Replica, group: &GroupView<'_>) -> bool;
}

/// An invalid baseline or live group; no state is published on refusal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplicaError {
    /// Game-owned schema or dependency validation refused.
    Schema,
    /// Manifest lifecycle ordinals are not distinct.
    InvalidCodes,
    /// More than four partitions were supplied.
    PartitionCapacity,
    /// Partition or total visible actor capacity was exceeded.
    ActorCapacity,
    /// A 257th distinct wire ID appeared before replacement baseline.
    AssignmentCapacity,
    /// A baseline repeated a partition identity.
    DuplicatePartition,
    /// An actor ID repeated or zero appeared.
    DuplicateActor,
    /// A live group named no admitted partition.
    UnknownPartition,
    /// A partition sample tick failed to advance.
    StaleTick,
    /// Interest generation skipped, regressed or wrapped.
    InterestGeneration,
    /// Lifecycle record, visibility generation or actor relation was invalid.
    Lifecycle,
    /// A wire ID attempted to move across baseline-bound partitions.
    CrossPartition,
    /// A lifecycle or generation transition lacked its exact actor closure.
    MissingFullState,
    /// Outcome, event or watermark admission requires the pending progress owner.
    UnsupportedOutcome,
}

/// Refusal from paired state and event-free result-progress admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgressAdmissionError {
    /// Result inventory, watermark or controller closure is invalid.
    Progress(ProgressError),
    /// State, lifecycle or game schema is invalid.
    State(ReplicaError),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Assignment {
    wire_id: u64,
    group_id: u32,
    visibility_generation: u32,
    retired: bool,
}

#[derive(Debug, PartialEq, Eq)]
struct Partition {
    group_id: u32,
    interest_generation: u32,
    sample_tick: u64,
    actors: Vec<Actor>,
}

/// Confirmed R4 state. The four actor arrays, assignment table and scratch
/// are reserved at construction; live admission does not grow their capacity.
#[derive(Debug, PartialEq, Eq)]
pub struct Replica {
    partitions: Vec<Partition>,
    assignments: Vec<Assignment>,
    scratch: Vec<Actor>,
    codes: LifecycleCodes,
}

impl Replica {
    /// Construct from an already staged baseline, after the supplied game
    /// validator has checked its complete schema and dependency closure.
    pub fn from_baseline(
        seeds: &[PartitionSeed<'_>],
        codes: LifecycleCodes,
        validator: &impl SchemaValidator,
    ) -> Result<Self, ReplicaError> {
        if !codes.valid() {
            return Err(ReplicaError::InvalidCodes);
        }
        if seeds.len() > MAX_PARTITIONS {
            return Err(ReplicaError::PartitionCapacity);
        }
        if !validator.baseline(seeds) {
            return Err(ReplicaError::Schema);
        }
        let mut partitions = Vec::with_capacity(MAX_PARTITIONS);
        let mut assignments = Vec::with_capacity(MAX_ASSIGNMENTS);
        for seed in seeds {
            if seeds
                .first()
                .is_some_and(|first| seed.sample_tick != first.sample_tick)
            {
                return Err(ReplicaError::StaleTick);
            }
            if seed.interest_generation == 0 {
                return Err(ReplicaError::InterestGeneration);
            }
            if seed.actors.len() > MAX_PARTITION_ACTORS {
                return Err(ReplicaError::ActorCapacity);
            }
            if partitions
                .iter()
                .any(|p: &Partition| p.group_id == seed.group_id)
            {
                return Err(ReplicaError::DuplicatePartition);
            }
            let mut actors = Vec::with_capacity(MAX_PARTITION_ACTORS);
            actors.extend_from_slice(seed.actors);
            actors.sort_unstable_by_key(|actor| actor.wire_id);
            for pair in actors.windows(2) {
                if pair[0].wire_id == pair[1].wire_id {
                    return Err(ReplicaError::DuplicateActor);
                }
            }
            for actor in &actors {
                if actor.wire_id == 0 || actor.visibility_generation == 0 {
                    return Err(ReplicaError::Lifecycle);
                }
                assignments.push(Assignment {
                    wire_id: actor.wire_id,
                    group_id: seed.group_id,
                    visibility_generation: u32::from(actor.visibility_generation),
                    retired: false,
                });
            }
            partitions.push(Partition {
                group_id: seed.group_id,
                interest_generation: seed.interest_generation,
                sample_tick: seed.sample_tick,
                actors,
            });
        }
        if assignments.len() > MAX_VISIBLE {
            return Err(ReplicaError::ActorCapacity);
        }
        assignments.sort_unstable_by_key(|row| row.wire_id);
        for pair in assignments.windows(2) {
            if pair[0].wire_id == pair[1].wire_id {
                return Err(ReplicaError::DuplicateActor);
            }
        }
        Ok(Self {
            partitions,
            assignments,
            scratch: Vec::with_capacity(MAX_PARTITION_ACTORS),
            codes,
        })
    }

    /// Exact sample age of one stable partition.
    pub fn partition_sample(&self, group_id: u32) -> Option<(u32, u64)> {
        self.partitions
            .iter()
            .find(|p| p.group_id == group_id)
            .map(|p| (p.interest_generation, p.sample_tick))
    }

    /// Confirmed actor and its partition's sample tick. A departed ID keeps
    /// its assignment but has no visible state.
    pub fn actor(&self, wire_id: u64) -> Option<(Actor, u64)> {
        let assignment = self.assignment(wire_id)?;
        let partition = self
            .partitions
            .iter()
            .find(|p| p.group_id == assignment.group_id)?;
        partition
            .actors
            .binary_search_by_key(&wire_id, |actor| actor.wire_id)
            .ok()
            .map(|index| (partition.actors[index], partition.sample_tick))
    }

    /// Distinct wire IDs charged to this baseline incarnation.
    pub fn assignment_count(&self) -> usize {
        self.assignments.len()
    }
    /// Current visible actor count across all partitions.
    pub fn visible_count(&self) -> usize {
        self.partitions.iter().map(|p| p.actors.len()).sum()
    }

    /// Reserved owner bytes in Rust values and Vec element capacities.
    /// Allocator bookkeeping and unrelated game-validator storage are excluded.
    pub fn reserved_capacity_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.partitions.capacity() * std::mem::size_of::<Partition>()
            + self.assignments.capacity() * std::mem::size_of::<Assignment>()
            + self.scratch.capacity() * std::mem::size_of::<Actor>()
            + self
                .partitions
                .iter()
                .map(|partition| partition.actors.capacity() * std::mem::size_of::<Actor>())
                .sum::<usize>()
    }

    fn assignment(&self, wire_id: u64) -> Option<Assignment> {
        self.assignments
            .binary_search_by_key(&wire_id, |row| row.wire_id)
            .ok()
            .map(|index| self.assignments[index])
    }

    /// Admit one decoded state/lifecycle-only R4 payload. Outcomes, events and
    /// watermarks are refused until their own bounded atomic progress path is
    /// installed. A schema failure or generic refusal publishes no state.
    pub fn admit(
        &mut self,
        group: &GroupView<'_>,
        validator: &impl SchemaValidator,
    ) -> Result<(), ReplicaError> {
        self.admit_inner(group, validator, false)
    }

    /// Admit event-free results and state together after exact progress
    /// preflight. The result inventory commits only after state admission is
    /// known to succeed. Event-bearing groups still refuse before mutation.
    pub fn admit_with_progress(
        &mut self,
        group: &GroupView<'_>,
        progress: &mut Progress,
        validator: &impl SchemaValidator,
    ) -> Result<(), ProgressAdmissionError> {
        let prior_sample_tick = self
            .partition_sample(group.header.group_id)
            .map_or(0, |(_, tick)| tick);
        let candidate = progress
            .preflight(group, prior_sample_tick)
            .map_err(ProgressAdmissionError::Progress)?;
        self.admit_inner(group, validator, true)
            .map_err(ProgressAdmissionError::State)?;
        progress.commit(candidate);
        Ok(())
    }

    fn admit_inner(
        &mut self,
        group: &GroupView<'_>,
        validator: &impl SchemaValidator,
        permit_event_free_results: bool,
    ) -> Result<(), ReplicaError> {
        if !permit_event_free_results
            && (group.header.resolved_through != 0
                || group.results().len() != 0
                || group.events().len() != 0)
        {
            return Err(ReplicaError::UnsupportedOutcome);
        }
        if !validator.group(self, group) {
            return Err(ReplicaError::Schema);
        }
        let index = self
            .partitions
            .iter()
            .position(|p| p.group_id == group.header.group_id)
            .ok_or(ReplicaError::UnknownPartition)?;
        let partition = &self.partitions[index];
        if group.header.commit_tick <= partition.sample_tick {
            return Err(ReplicaError::StaleTick);
        }
        let generation_change = group.header.interest_generation != partition.interest_generation;
        if generation_change
            && partition.interest_generation.checked_add(1)
                != Some(group.header.interest_generation)
        {
            return Err(ReplicaError::InterestGeneration);
        }
        self.scratch.clear();
        if generation_change {
            self.build_replacement(index, group)?;
        } else {
            self.build_incremental(index, group)?;
        }
        let old_len = self.partitions[index].actors.len();
        let new_visible = self.visible_count() - old_len + self.scratch.len();
        if new_visible > MAX_VISIBLE {
            return Err(ReplicaError::ActorCapacity);
        }
        let new_assignments = self
            .scratch
            .iter()
            .filter(|actor| self.assignment(actor.wire_id).is_none())
            .count();
        if self.assignments.len() + new_assignments > MAX_ASSIGNMENTS {
            return Err(ReplicaError::AssignmentCapacity);
        }
        // All fallible checks precede this point. Both Vecs have fixed spare
        // capacity; the swap publishes an entire partition in one operation.
        for actor in &self.scratch {
            if self.assignment(actor.wire_id).is_none() {
                let at = self
                    .assignments
                    .binary_search_by_key(&actor.wire_id, |row| row.wire_id)
                    .unwrap_err();
                self.assignments.insert(
                    at,
                    Assignment {
                        wire_id: actor.wire_id,
                        group_id: group.header.group_id,
                        visibility_generation: u32::from(actor.visibility_generation),
                        retired: false,
                    },
                );
            }
        }
        for lifecycle in group.lifecycle() {
            let at = self
                .assignments
                .binary_search_by_key(&lifecycle.wire_id, |row| row.wire_id)
                .expect("validated lifecycle has an assignment");
            if self.codes.enters(lifecycle.kind) == Some(true) {
                self.assignments[at].visibility_generation = lifecycle.visibility_generation;
            } else if lifecycle.kind == self.codes.despawn {
                self.assignments[at].retired = true;
            }
        }
        let partition = &mut self.partitions[index];
        std::mem::swap(&mut partition.actors, &mut self.scratch);
        partition.interest_generation = group.header.interest_generation;
        partition.sample_tick = group.header.commit_tick;
        Ok(())
    }

    fn build_replacement(
        &mut self,
        index: usize,
        group: &GroupView<'_>,
    ) -> Result<(), ReplicaError> {
        self.scratch.extend(group.actors());
        self.scratch.sort_unstable_by_key(|actor| actor.wire_id);
        self.check_actor_set(group.header.group_id)?;
        let old = &self.partitions[index].actors;
        // A generation transition is complete: every appearance/disappearance
        // has one matching lifecycle, and retained actors carry full state.
        for actor in &self.scratch {
            if old
                .binary_search_by_key(&actor.wire_id, |row| row.wire_id)
                .is_err()
                && !self.has_lifecycle(group, actor.wire_id, true)
            {
                return Err(ReplicaError::MissingFullState);
            }
        }
        for actor in old {
            if self
                .scratch
                .binary_search_by_key(&actor.wire_id, |row| row.wire_id)
                .is_err()
                && !self.has_lifecycle(group, actor.wire_id, false)
            {
                return Err(ReplicaError::MissingFullState);
            }
        }
        for (i, lifecycle) in group.lifecycle().enumerate() {
            if group
                .lifecycle()
                .take(i)
                .any(|prior| prior.wire_id == lifecycle.wire_id)
            {
                return Err(ReplicaError::Lifecycle);
            }
            self.validate_lifecycle_shape(group, lifecycle)?;
            let old_has = old
                .binary_search_by_key(&lifecycle.wire_id, |row| row.wire_id)
                .is_ok();
            let new_has = self
                .scratch
                .binary_search_by_key(&lifecycle.wire_id, |row| row.wire_id)
                .is_ok();
            let invisible_terminal = !old_has && !new_has && lifecycle.kind == self.codes.despawn;
            if !invisible_terminal
                && (self.codes.enters(lifecycle.kind) != Some(!old_has && new_has)
                    || old_has == new_has)
            {
                return Err(ReplicaError::Lifecycle);
            }
        }
        for actor in &self.scratch {
            if old
                .binary_search_by_key(&actor.wire_id, |row| row.wire_id)
                .is_ok()
                && self.assignment(actor.wire_id).is_some_and(|row| {
                    row.visibility_generation != u32::from(actor.visibility_generation)
                })
            {
                return Err(ReplicaError::Lifecycle);
            }
        }
        Ok(())
    }

    fn build_incremental(
        &mut self,
        index: usize,
        group: &GroupView<'_>,
    ) -> Result<(), ReplicaError> {
        self.scratch
            .extend_from_slice(&self.partitions[index].actors);
        // Remove exits first so a full partition may replace one actor with
        // another regardless of lifecycle record order.
        for (i, lifecycle) in group.lifecycle().enumerate() {
            if group
                .lifecycle()
                .take(i)
                .any(|prior| prior.wire_id == lifecycle.wire_id)
            {
                return Err(ReplicaError::Lifecycle);
            }
            self.validate_lifecycle_shape(group, lifecycle)?;
            if lifecycle.kind == self.codes.leave || lifecycle.kind == self.codes.reenter {
                return Err(ReplicaError::InterestGeneration);
            }
            if self.codes.enters(lifecycle.kind) == Some(false) {
                let pos = self
                    .scratch
                    .binary_search_by_key(&lifecycle.wire_id, |row| row.wire_id);
                if group
                    .actors()
                    .any(|actor| actor.wire_id == lifecycle.wire_id)
                {
                    return Err(ReplicaError::Lifecycle);
                }
                match pos {
                    Ok(pos) => {
                        self.scratch.remove(pos);
                    }
                    Err(_) if lifecycle.kind == self.codes.despawn => {}
                    Err(_) => return Err(ReplicaError::Lifecycle),
                }
            }
        }
        for lifecycle in group.lifecycle() {
            if self.codes.enters(lifecycle.kind) == Some(true) {
                let pos = self
                    .scratch
                    .binary_search_by_key(&lifecycle.wire_id, |row| row.wire_id);
                if pos.is_ok() {
                    return Err(ReplicaError::Lifecycle);
                }
                let actor = group
                    .actors()
                    .find(|actor| actor.wire_id == lifecycle.wire_id)
                    .ok_or(ReplicaError::MissingFullState)?;
                if self.scratch.len() == MAX_PARTITION_ACTORS {
                    return Err(ReplicaError::ActorCapacity);
                }
                self.scratch.insert(pos.unwrap_err(), actor);
            }
        }
        for (i, actor) in group.actors().enumerate() {
            if group
                .actors()
                .take(i)
                .any(|prior| prior.wire_id == actor.wire_id)
            {
                return Err(ReplicaError::DuplicateActor);
            }
            if !self.has_lifecycle(group, actor.wire_id, true)
                && self.assignment(actor.wire_id).is_some_and(|row| {
                    row.visibility_generation != u32::from(actor.visibility_generation)
                })
            {
                return Err(ReplicaError::Lifecycle);
            }
            match self
                .scratch
                .binary_search_by_key(&actor.wire_id, |row| row.wire_id)
            {
                Ok(pos) => self.scratch[pos] = actor,
                Err(_) => return Err(ReplicaError::Lifecycle),
            }
        }
        self.check_actor_set(group.header.group_id)
    }

    fn check_actor_set(&self, group_id: u32) -> Result<(), ReplicaError> {
        for (i, actor) in self.scratch.iter().enumerate() {
            if actor.wire_id == 0 || (i != 0 && self.scratch[i - 1].wire_id == actor.wire_id) {
                return Err(ReplicaError::DuplicateActor);
            }
            if self
                .assignment(actor.wire_id)
                .is_some_and(|row| row.group_id != group_id)
            {
                return Err(ReplicaError::CrossPartition);
            }
        }
        Ok(())
    }

    fn validate_lifecycle_shape(
        &self,
        group: &GroupView<'_>,
        lifecycle: Lifecycle,
    ) -> Result<(), ReplicaError> {
        let enters = self
            .codes
            .enters(lifecycle.kind)
            .ok_or(ReplicaError::Lifecycle)?;
        if lifecycle.wire_id == 0 || lifecycle.visibility_generation == 0 {
            return Err(ReplicaError::Lifecycle);
        }
        if self
            .assignment(lifecycle.wire_id)
            .is_some_and(|row| row.group_id != group.header.group_id)
        {
            return Err(ReplicaError::CrossPartition);
        }
        if enters {
            let actor = group
                .actors()
                .find(|actor| actor.wire_id == lifecycle.wire_id)
                .ok_or(ReplicaError::MissingFullState)?;
            if u32::from(actor.visibility_generation) != lifecycle.visibility_generation {
                return Err(ReplicaError::Lifecycle);
            }
            match self.assignment(lifecycle.wire_id) {
                Some(row)
                    if lifecycle.kind == self.codes.reenter
                        && !row.retired
                        && row.visibility_generation.checked_add(1)
                            == Some(lifecycle.visibility_generation) => {}
                None if lifecycle.kind == self.codes.spawn => {}
                _ => return Err(ReplicaError::Lifecycle),
            }
        } else if group
            .actors()
            .any(|actor| actor.wire_id == lifecycle.wire_id)
            || self.assignment(lifecycle.wire_id).is_none_or(|row| {
                row.retired || row.visibility_generation != lifecycle.visibility_generation
            })
        {
            return Err(ReplicaError::Lifecycle);
        }
        Ok(())
    }

    fn has_lifecycle(&self, group: &GroupView<'_>, id: u64, enters: bool) -> bool {
        group
            .lifecycle()
            .any(|record| record.wire_id == id && self.codes.enters(record.kind) == Some(enters))
    }
}
