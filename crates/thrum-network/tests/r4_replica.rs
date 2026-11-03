//! Fixed R4 replica admission across independent partitions and refusal paths.

use thrum_network::{
    r4::{self, ActionResult, Actor, GroupHeader, GroupInput, Lifecycle},
    replica::{LifecycleCodes, PartitionSeed, Replica, ReplicaError, SchemaValidator},
};

const CODES: LifecycleCodes = LifecycleCodes {
    spawn: 1,
    despawn: 2,
    leave: 3,
    reenter: 4,
};

struct GameSchema;
impl SchemaValidator for GameSchema {
    fn baseline(&self, seeds: &[PartitionSeed<'_>]) -> bool {
        seeds
            .iter()
            .all(|seed| seed.actors.iter().all(|actor| actor.mode == 7))
    }

    fn group(&self, _replica: &Replica, group: &r4::GroupView<'_>) -> bool {
        group.actors().all(|actor| actor.mode == 7)
            && group
                .lifecycle()
                .all(|record| record.schema_id == 1 && record.flags == 0)
    }
}

fn actor(wire_id: u64, health: u16, generation: u16) -> Actor {
    Actor {
        wire_id,
        sector: [0; 3],
        offset: [0; 3],
        quaternion: [0; 4],
        velocity: [0; 3],
        mode: 7,
        flags: 0,
        health,
        inventory_revision: 0,
        owner: 0,
        visibility_generation: generation,
    }
}

fn life(wire_id: u64, generation: u32, kind: u8) -> Lifecycle {
    Lifecycle {
        wire_id,
        visibility_generation: generation,
        schema_id: 1,
        kind,
        flags: 0,
    }
}

fn baseline() -> Replica {
    let one = [actor(1, 10, 1)];
    let two = [actor(2, 20, 1)];
    Replica::from_baseline(
        &[
            PartitionSeed {
                group_id: 10,
                interest_generation: 1,
                sample_tick: 0,
                actors: &one,
            },
            PartitionSeed {
                group_id: 20,
                interest_generation: 1,
                sample_tick: 0,
                actors: &two,
            },
        ],
        CODES,
        &GameSchema,
    )
    .unwrap()
}

fn admit(
    replica: &mut Replica,
    partition: u32,
    generation: u32,
    tick: u64,
    actors: &[Actor],
    lifecycle: &[Lifecycle],
) -> Result<(), ReplicaError> {
    let mut bytes = [0u8; r4::MAX_PAYLOAD_BYTES];
    let size = r4::encode(
        GroupInput {
            header: GroupHeader {
                group_id: partition,
                interest_generation: generation,
                commit_tick: tick,
                resolved_through: 0,
            },
            actors,
            lifecycle,
            results: &[],
            events: &[],
        },
        &mut bytes,
    )
    .unwrap();
    let view = r4::decode(&bytes[..size]).unwrap();
    replica.admit(&view, &GameSchema)
}

#[test]
fn same_tick_partitions_and_distinct_sample_ages() {
    let mut replica = baseline();
    admit(&mut replica, 10, 1, 1, &[actor(1, 11, 1)], &[]).unwrap();
    admit(&mut replica, 20, 1, 1, &[actor(2, 21, 1)], &[]).unwrap();
    admit(&mut replica, 10, 1, 3, &[actor(1, 13, 1)], &[]).unwrap();
    assert_eq!(replica.actor(1), Some((actor(1, 13, 1), 3)));
    assert_eq!(replica.actor(2), Some((actor(2, 21, 1), 1)));
    assert_eq!(replica.partition_sample(20), Some((1, 1)));
    assert_eq!(
        admit(&mut replica, 20, 1, 1, &[], &[]),
        Err(ReplicaError::StaleTick)
    );
}

#[test]
fn refused_groups_preserve_all_visible_and_assignment_state() {
    let mut replica = baseline();
    let before = (
        replica.actor(1),
        replica.actor(2),
        replica.assignment_count(),
        replica.partition_sample(10),
    );
    assert_eq!(
        admit(
            &mut replica,
            10,
            1,
            1,
            &[actor(2, 99, 2)],
            &[life(2, 2, CODES.reenter)]
        ),
        Err(ReplicaError::CrossPartition)
    );
    assert_eq!(
        admit(
            &mut replica,
            10,
            2,
            1,
            &[actor(1, 11, 1), actor(2, 22, 2)],
            &[life(2, 2, CODES.reenter)]
        ),
        Err(ReplicaError::CrossPartition)
    );
    assert_eq!(
        admit(&mut replica, 10, 3, 1, &[], &[]),
        Err(ReplicaError::InterestGeneration)
    );
    assert_eq!(
        admit(&mut replica, 10, 2, 1, &[], &[]),
        Err(ReplicaError::MissingFullState)
    );
    assert_eq!(
        before,
        (
            replica.actor(1),
            replica.actor(2),
            replica.assignment_count(),
            replica.partition_sample(10)
        )
    );
}

#[test]
fn leave_and_reentry_retain_home_and_generation() {
    let mut replica = baseline();
    assert_eq!(
        admit(&mut replica, 20, 1, 1, &[], &[life(2, 1, CODES.leave)]),
        Err(ReplicaError::InterestGeneration)
    );
    admit(&mut replica, 20, 2, 1, &[], &[life(2, 1, CODES.leave)]).unwrap();
    assert_eq!(replica.actor(2), None);
    assert_eq!(replica.assignment_count(), 2);
    assert_eq!(
        admit(
            &mut replica,
            10,
            2,
            1,
            &[actor(1, 10, 1), actor(2, 20, 2)],
            &[life(2, 2, CODES.reenter)]
        ),
        Err(ReplicaError::CrossPartition)
    );
    assert_eq!(
        admit(
            &mut replica,
            20,
            3,
            2,
            &[actor(2, 22, 1)],
            &[life(2, 1, CODES.reenter)]
        ),
        Err(ReplicaError::Lifecycle)
    );
    admit(
        &mut replica,
        20,
        3,
        2,
        &[actor(2, 22, 2)],
        &[life(2, 2, CODES.reenter)],
    )
    .unwrap();
    assert_eq!(replica.actor(2), Some((actor(2, 22, 2), 2)));
}

#[test]
fn full_partition_can_exchange_actors_without_temp_overflow() {
    let actors: Vec<_> = (1..=32).map(|id| actor(id, id as u16, 1)).collect();
    let mut replica = Replica::from_baseline(
        &[PartitionSeed {
            group_id: 10,
            interest_generation: 1,
            sample_tick: 0,
            actors: &actors,
        }],
        CODES,
        &GameSchema,
    )
    .unwrap();
    admit(
        &mut replica,
        10,
        1,
        1,
        &[actor(33, 33, 1)],
        &[life(33, 1, CODES.spawn), life(1, 1, CODES.despawn)],
    )
    .unwrap();
    assert_eq!(replica.actor(1), None);
    assert_eq!(replica.actor(33), Some((actor(33, 33, 1), 1)));
    assert_eq!(replica.visible_count(), 32);
}

#[test]
fn terminal_destruction_after_leave_retires_home_without_visibility() {
    let mut replica = baseline();
    admit(&mut replica, 20, 2, 1, &[], &[life(2, 1, CODES.leave)]).unwrap();
    assert_eq!(
        admit(&mut replica, 10, 1, 1, &[], &[life(2, 1, CODES.despawn)]),
        Err(ReplicaError::CrossPartition)
    );
    assert_eq!(replica.partition_sample(10), Some((1, 0)));
    admit(&mut replica, 20, 2, 2, &[], &[life(2, 1, CODES.despawn)]).unwrap();
    assert_eq!(replica.assignment_count(), 2);
    assert_eq!(replica.actor(2), None);
    assert_eq!(
        admit(
            &mut replica,
            20,
            3,
            3,
            &[actor(2, 23, 2)],
            &[life(2, 2, CODES.reenter)]
        ),
        Err(ReplicaError::Lifecycle)
    );
    assert_eq!(replica.partition_sample(20), Some((2, 2)));
}

#[test]
fn invisible_churn_refuses_257th_assignment_without_poisoning_other_partition() {
    let mut replica = baseline();
    let reserved = replica.reserved_capacity_bytes();
    let mut tick = 0;
    for id in 3..=256 {
        tick += 1;
        admit(
            &mut replica,
            20,
            1,
            tick,
            &[actor(id, 1, 1)],
            &[life(id, 1, CODES.spawn)],
        )
        .unwrap();
        tick += 1;
        admit(
            &mut replica,
            20,
            1,
            tick,
            &[],
            &[life(id, 1, CODES.despawn)],
        )
        .unwrap();
    }
    assert_eq!(replica.assignment_count(), 256);
    assert_eq!(replica.visible_count(), 2);
    assert_eq!(replica.reserved_capacity_bytes(), reserved);
    assert_eq!(
        admit(
            &mut replica,
            20,
            1,
            tick + 1,
            &[actor(257, 1, 1)],
            &[life(257, 1, CODES.spawn)]
        ),
        Err(ReplicaError::AssignmentCapacity)
    );
    assert_eq!(replica.partition_sample(20), Some((1, tick)));
    assert_eq!(replica.actor(257), None);
    admit(&mut replica, 10, 1, 1, &[actor(1, 11, 1)], &[]).unwrap();
    assert_eq!(replica.actor(1), Some((actor(1, 11, 1), 1)));
    assert_eq!(replica.reserved_capacity_bytes(), reserved);
}

#[test]
fn game_schema_refusal_is_atomic() {
    let mut replica = baseline();
    let mut invalid = actor(1, 99, 1);
    invalid.mode = 8;
    assert_eq!(
        admit(&mut replica, 10, 1, 1, &[invalid], &[]),
        Err(ReplicaError::Schema)
    );
    assert_eq!(replica.actor(1), Some((actor(1, 10, 1), 0)));
    assert_eq!(replica.partition_sample(10), Some((1, 0)));
}

#[test]
fn outcomes_and_watermark_refuse_before_state_changes() {
    let mut replica = baseline();
    let mut bytes = [0u8; r4::MAX_PAYLOAD_BYTES];
    let result = ActionResult {
        input_sequence: 1,
        resolved_tick: 1,
        first_event_id: 0,
        action_type: 1,
        reason: 0,
        event_count: 0,
        disposition: 0,
    };
    for (results, watermark) in [(&[result][..], 0), (&[][..], 1)] {
        let size = r4::encode(
            GroupInput {
                header: GroupHeader {
                    group_id: 10,
                    interest_generation: 1,
                    commit_tick: 1,
                    resolved_through: watermark,
                },
                actors: &[actor(1, 99, 1)],
                lifecycle: &[],
                results,
                events: &[],
            },
            &mut bytes,
        )
        .unwrap();
        let view = r4::decode(&bytes[..size]).unwrap();
        assert_eq!(
            replica.admit(&view, &GameSchema),
            Err(ReplicaError::UnsupportedOutcome)
        );
    }
    assert_eq!(replica.actor(1), Some((actor(1, 10, 1), 0)));
}

#[test]
#[ignore = "isolated release-mode state-only four-peer admission cost probe"]
fn measure_four_peer_state_only_admission() {
    use std::{hint::black_box, time::Instant};

    let actors: [[Actor; 32]; 4] = std::array::from_fn(|group| {
        std::array::from_fn(|slot| actor((group * 32 + slot + 1) as u64, 10, 1))
    });
    let seeds: [PartitionSeed<'_>; 4] = std::array::from_fn(|group| PartitionSeed {
        group_id: (group + 1) as u32,
        interest_generation: 1,
        sample_tick: 0,
        actors: &actors[group],
    });
    let mut replicas: [Replica; 4] =
        std::array::from_fn(|_| Replica::from_baseline(&seeds, CODES, &GameSchema).unwrap());
    let reserved: usize = replicas.iter().map(Replica::reserved_capacity_bytes).sum();
    let mut payloads = [[0u8; r4::MAX_PAYLOAD_BYTES]; 4];
    let mut lengths = [0; 4];
    for group in 0..4 {
        lengths[group] = r4::encode(
            GroupInput {
                header: GroupHeader {
                    group_id: (group + 1) as u32,
                    interest_generation: 1,
                    commit_tick: 1,
                    resolved_through: 0,
                },
                actors: &actors[group],
                lifecycle: &[],
                results: &[],
                events: &[],
            },
            &mut payloads[group],
        )
        .unwrap();
    }
    const SWEEPS: u64 = 10_000;
    let start = Instant::now();
    for tick in 1..=SWEEPS {
        for group in 0..4 {
            payloads[group][8..16].copy_from_slice(&tick.to_le_bytes());
            let view = r4::decode(&payloads[group][..lengths[group]]).unwrap();
            for replica in &mut replicas {
                replica.admit(&view, &GameSchema).unwrap();
            }
        }
    }
    let nanos_per_sweep = start.elapsed().as_nanos() as f64 / SWEEPS as f64;
    assert!(
        replicas
            .iter()
            .all(|replica| replica.partition_sample(4) == Some((1, SWEEPS)))
    );
    black_box(replicas[0].actor(128));
    assert_eq!(
        reserved,
        replicas.iter().map(Replica::reserved_capacity_bytes).sum()
    );
    println!(
        "r4_replica_four_peer_state_only_sweep_ns={nanos_per_sweep:.1} reserved_owner_bytes={reserved}"
    );
}
