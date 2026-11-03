//! Private N1 multi-partition admission oracle. No production replica or codec.

use std::collections::{BTreeMap, BTreeSet};

const ASSIGNMENT_CAP: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Partition {
    generation: u32,
    sample_tick: u64,
    entities: BTreeMap<u64, u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CompositeReplica {
    partitions: BTreeMap<u32, Partition>,
    entity_owner: BTreeMap<u64, u32>,
    // Baseline-bound assignment survives a live leave/despawn.
    entity_home: BTreeMap<u64, u32>,
}

#[derive(Clone, Copy)]
enum Change {
    Update(u64, u64),
    Spawn(u64, u64),
    Despawn(u64),
}

struct Group {
    partition: u32,
    generation: u32,
    tick: u64,
    changes: Vec<Change>,
    // An incremented generation carries the full current partition state.
    complete_replacement: Option<BTreeMap<u64, u64>>,
}

type PartitionSeed<'a> = (u32, u32, &'a [(u64, u64)]);

#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    UnknownPartition,
    StaleTick,
    StaleOrSkippedGeneration,
    MissingFullState,
    DuplicateEntity,
    UnknownEntity,
    CrossPartitionEntity,
    RepeatedEntity,
    AssignmentCapacity,
}

impl CompositeReplica {
    fn baseline(capture_tick: u64, partitions: [PartitionSeed<'_>; 2]) -> Self {
        let mut owner = BTreeMap::new();
        let mut rows = BTreeMap::new();
        for (id, generation, entities) in partitions {
            assert!(generation > 0);
            let mut state = BTreeMap::new();
            for &(entity, value) in entities {
                assert!(owner.insert(entity, id).is_none());
                assert!(state.insert(entity, value).is_none());
            }
            assert!(
                rows.insert(
                    id,
                    Partition {
                        generation,
                        sample_tick: capture_tick,
                        entities: state,
                    },
                )
                .is_none()
            );
        }
        assert!(owner.len() <= ASSIGNMENT_CAP);
        Self {
            partitions: rows,
            entity_home: owner.clone(),
            entity_owner: owner,
        }
    }

    fn apply(&mut self, group: Group) -> Result<(), Refusal> {
        let prior = self
            .partitions
            .get(&group.partition)
            .ok_or(Refusal::UnknownPartition)?;
        if group.tick <= prior.sample_tick {
            return Err(Refusal::StaleTick);
        }
        if group.generation != prior.generation
            && prior.generation.checked_add(1) != Some(group.generation)
        {
            return Err(Refusal::StaleOrSkippedGeneration);
        }

        // Validate against scratch state; publish all three owners together.
        // This model uses tree clones for clarity and makes no allocation claim.
        let mut next = prior.clone();
        let mut owner = self.entity_owner.clone();
        let mut home = self.entity_home.clone();
        if group.generation == prior.generation {
            if group.complete_replacement.is_some() {
                return Err(Refusal::MissingFullState);
            }
            let mut seen = BTreeSet::new();
            for change in group.changes {
                let id = match change {
                    Change::Update(id, _) | Change::Spawn(id, _) | Change::Despawn(id) => id,
                };
                if !seen.insert(id) {
                    return Err(Refusal::RepeatedEntity);
                }
                match change {
                    Change::Update(id, value) => {
                        let slot = next.entities.get_mut(&id).ok_or(Refusal::UnknownEntity)?;
                        *slot = value;
                    }
                    Change::Spawn(id, value) => {
                        if home
                            .get(&id)
                            .is_some_and(|assigned| *assigned != group.partition)
                        {
                            return Err(Refusal::CrossPartitionEntity);
                        }
                        if !home.contains_key(&id) && home.len() == ASSIGNMENT_CAP {
                            return Err(Refusal::AssignmentCapacity);
                        }
                        if owner.contains_key(&id) {
                            return Err(Refusal::DuplicateEntity);
                        }
                        home.insert(id, group.partition);
                        owner.insert(id, group.partition);
                        next.entities.insert(id, value);
                    }
                    Change::Despawn(id) => {
                        if next.entities.remove(&id).is_none() {
                            return Err(Refusal::UnknownEntity);
                        }
                        owner.remove(&id);
                    }
                }
            }
        } else {
            let replacement = group
                .complete_replacement
                .ok_or(Refusal::MissingFullState)?;
            for entity in next.entities.keys() {
                owner.remove(entity);
            }
            for entity in replacement.keys() {
                if home
                    .get(entity)
                    .is_some_and(|assigned| *assigned != group.partition)
                {
                    return Err(Refusal::CrossPartitionEntity);
                }
                if !home.contains_key(entity) && home.len() == ASSIGNMENT_CAP {
                    return Err(Refusal::AssignmentCapacity);
                }
                if owner.insert(*entity, group.partition).is_some() {
                    return Err(Refusal::CrossPartitionEntity);
                }
                home.insert(*entity, group.partition);
            }
            next.entities = replacement;
            next.generation = group.generation;
            if !group.changes.is_empty() {
                return Err(Refusal::RepeatedEntity);
            }
        }
        next.sample_tick = group.tick;
        self.partitions.insert(group.partition, next);
        self.entity_owner = owner;
        self.entity_home = home;
        Ok(())
    }

    fn state(&self, entity: u64) -> Option<(u64, u64)> {
        let partition = self.partitions.get(self.entity_owner.get(&entity)?)?;
        Some((*partition.entities.get(&entity)?, partition.sample_tick))
    }
}

fn group(partition: u32, generation: u32, tick: u64, changes: Vec<Change>) -> Group {
    Group {
        partition,
        generation,
        tick,
        changes,
        complete_replacement: None,
    }
}

#[test]
fn one_replica_admits_same_tick_partitions_and_tracks_distinct_sample_ages() {
    let mut replica = CompositeReplica::baseline(0, [(10, 1, &[(1, 0)]), (20, 1, &[(2, 0)])]);
    replica
        .apply(group(10, 1, 1, vec![Change::Update(1, 1)]))
        .expect("test setup or assertion requires a value");
    replica
        .apply(group(20, 1, 1, vec![Change::Update(2, 5)]))
        .expect("test setup or assertion requires a value");
    assert_eq!(replica.state(1), Some((1, 1)));
    assert_eq!(replica.state(2), Some((5, 1)));

    replica
        .apply(group(10, 1, 3, vec![Change::Update(1, 3)]))
        .expect("test setup or assertion requires a value");
    assert_eq!(replica.state(1), Some((3, 3)));
    assert_eq!(replica.state(2), Some((5, 1)));

    let before = replica.clone();
    assert_eq!(
        replica.apply(group(20, 1, 1, vec![Change::Update(2, 6)])),
        Err(Refusal::StaleTick)
    );
    assert_eq!(replica, before);
    assert_eq!(
        replica.apply(group(10, 1, 4, vec![Change::Spawn(2, 99)])),
        Err(Refusal::CrossPartitionEntity)
    );
    assert_eq!(replica, before);
    assert_eq!(
        replica.apply(group(10, 1, 4, vec![Change::Despawn(99)])),
        Err(Refusal::UnknownEntity)
    );
    assert_eq!(replica, before);

    replica
        .apply(Group {
            partition: 20,
            generation: 2,
            tick: 4,
            changes: Vec::new(),
            complete_replacement: Some(BTreeMap::from([(2, 7), (3, 8)])),
        })
        .expect("test setup or assertion requires a value");
    assert_eq!(replica.state(2), Some((7, 4)));
    assert_eq!(replica.state(3), Some((8, 4)));
    assert_eq!(replica.state(1), Some((3, 3)));
}

#[test]
fn generation_transition_requires_full_collision_free_partition_state() {
    let mut replica = CompositeReplica::baseline(0, [(10, 1, &[(1, 0)]), (20, 1, &[(2, 0)])]);
    let before = replica.clone();
    assert_eq!(
        replica.apply(group(20, 2, 1, vec![Change::Update(2, 2)])),
        Err(Refusal::MissingFullState)
    );
    assert_eq!(replica, before);
    assert_eq!(
        replica.apply(Group {
            partition: 20,
            generation: 2,
            tick: 1,
            changes: Vec::new(),
            complete_replacement: Some(BTreeMap::from([(1, 9)])),
        }),
        Err(Refusal::CrossPartitionEntity)
    );
    assert_eq!(replica, before);
    assert_eq!(
        replica.apply(group(20, 3, 1, Vec::new())),
        Err(Refusal::StaleOrSkippedGeneration)
    );
    assert_eq!(replica, before);
}

#[test]
fn leaving_visibility_does_not_transfer_baseline_partition_assignment() {
    let mut replica = CompositeReplica::baseline(0, [(10, 1, &[(1, 0)]), (20, 1, &[(2, 0)])]);
    replica
        .apply(Group {
            partition: 20,
            generation: 2,
            tick: 1,
            changes: Vec::new(),
            complete_replacement: Some(BTreeMap::new()),
        })
        .expect("test setup or assertion requires a value");
    assert_eq!(replica.state(2), None);
    let before = replica.clone();
    assert_eq!(
        replica.apply(Group {
            partition: 10,
            generation: 2,
            tick: 1,
            changes: Vec::new(),
            complete_replacement: Some(BTreeMap::from([(1, 0), (2, 4)])),
        }),
        Err(Refusal::CrossPartitionEntity)
    );
    assert_eq!(replica, before);
    replica
        .apply(group(20, 2, 2, vec![Change::Spawn(2, 5)]))
        .expect("test setup or assertion requires a value");
    assert_eq!(replica.state(2), Some((5, 2)));
}

#[test]
fn invisible_id_churn_refuses_before_assignment_table_growth() {
    let mut replica = CompositeReplica::baseline(0, [(10, 1, &[(1, 0)]), (20, 1, &[(2, 0)])]);
    let mut tick = 0;
    for entity in 3..=ASSIGNMENT_CAP as u64 {
        tick += 1;
        replica
            .apply(group(20, 1, tick, vec![Change::Spawn(entity, entity)]))
            .expect("test setup or assertion requires a value");
        tick += 1;
        replica
            .apply(group(20, 1, tick, vec![Change::Despawn(entity)]))
            .expect("test setup or assertion requires a value");
    }
    assert_eq!(replica.entity_home.len(), ASSIGNMENT_CAP);
    assert_eq!(replica.entity_owner.len(), 2);
    let before = replica.clone();
    assert_eq!(
        replica.apply(group(
            20,
            1,
            tick + 1,
            vec![Change::Spawn(ASSIGNMENT_CAP as u64 + 1, 1)]
        )),
        Err(Refusal::AssignmentCapacity)
    );
    assert_eq!(replica, before);
    replica
        .apply(group(10, 1, 1, vec![Change::Update(1, 1)]))
        .expect("test setup or assertion requires a value");
    assert_eq!(replica.state(1), Some((1, 1)));
}
