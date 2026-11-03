//! Event-free result progress and R4 state commit share one admission boundary.

use thrum_network::{
    progress::{PendingInput, Progress, ProgressError},
    r4::{self, ActionResult, Actor, Event, GroupHeader, GroupInput},
    replica::{
        LifecycleCodes, PartitionSeed, ProgressAdmissionError, Replica, ReplicaError,
        SchemaValidator,
    },
};

struct Schema;
impl SchemaValidator for Schema {
    fn baseline(&self, seeds: &[PartitionSeed<'_>]) -> bool {
        seeds
            .iter()
            .all(|seed| seed.actors.iter().all(|actor| actor.mode == 7))
    }
    fn group(&self, _replica: &Replica, group: &r4::GroupView<'_>) -> bool {
        group.actors().all(|actor| actor.mode == 7)
    }
}

fn actor(health: u16) -> Actor {
    Actor {
        wire_id: 1,
        sector: [0; 3],
        offset: [0; 3],
        quaternion: [0; 4],
        velocity: [0; 3],
        mode: 7,
        flags: 0,
        health,
        inventory_revision: 0,
        owner: 0,
        visibility_generation: 1,
    }
}

fn result(sequence: u64, resolved_tick: u64) -> ActionResult {
    ActionResult {
        input_sequence: sequence,
        resolved_tick,
        first_event_id: 0,
        action_type: 5,
        reason: 0,
        event_count: 0,
        disposition: 0,
    }
}

fn setup() -> (Replica, Progress) {
    let initial = [actor(10)];
    let replica = Replica::from_baseline(
        &[PartitionSeed {
            group_id: 10,
            interest_generation: 1,
            sample_tick: 0,
            actors: &initial,
        }],
        LifecycleCodes {
            spawn: 1,
            despawn: 2,
            leave: 3,
            reenter: 4,
        },
        &Schema,
    )
    .unwrap();
    let progress = Progress::from_baseline(
        10,
        1,
        6,
        &[
            PendingInput {
                sequence: 7,
                action_type: 5,
                resolved: false,
            },
            PendingInput {
                sequence: 8,
                action_type: 5,
                resolved: false,
            },
        ],
    )
    .unwrap();
    (replica, progress)
}

fn admit(
    replica: &mut Replica,
    progress: &mut Progress,
    tick: u64,
    health: u16,
    watermark: u64,
    results: &[ActionResult],
    events: &[Event],
) -> Result<(), ProgressAdmissionError> {
    let mut bytes = [0u8; r4::MAX_PAYLOAD_BYTES];
    let size = r4::encode(
        GroupInput {
            header: GroupHeader {
                group_id: 10,
                interest_generation: 1,
                commit_tick: tick,
                resolved_through: watermark,
            },
            actors: &[actor(health)],
            lifecycle: &[],
            results,
            events,
        },
        &mut bytes,
    )
    .unwrap();
    let view = r4::decode(&bytes[..size]).unwrap();
    replica.admit_with_progress(&view, progress, &Schema)
}

#[test]
fn higher_result_waits_for_gap_then_watermark_advances_exactly() {
    let (mut replica, mut progress) = setup();
    let reserved = progress.reserved_capacity_bytes();
    admit(&mut replica, &mut progress, 1, 11, 6, &[result(8, 1)], &[]).unwrap();
    assert_eq!(replica.actor(1), Some((actor(11), 1)));
    assert_eq!(progress.resolved_through(), 6);
    assert_eq!(progress.pending_count(), 2);
    assert_eq!(
        admit(&mut replica, &mut progress, 2, 99, 8, &[result(7, 1)], &[]),
        Err(ProgressAdmissionError::Progress(
            ProgressError::InvalidResult
        ))
    );
    assert_eq!(replica.actor(1), Some((actor(11), 1)));
    assert_eq!(
        admit(&mut replica, &mut progress, 2, 99, 7, &[result(7, 2)], &[]),
        Err(ProgressAdmissionError::Progress(
            ProgressError::InvalidWatermark
        ))
    );
    assert_eq!(replica.actor(1), Some((actor(11), 1)));
    admit(&mut replica, &mut progress, 2, 12, 8, &[result(7, 2)], &[]).unwrap();
    assert_eq!(progress.resolved_through(), 8);
    assert_eq!(progress.pending_count(), 0);
    assert_eq!(
        admit(&mut replica, &mut progress, 3, 99, 8, &[result(8, 1)], &[]),
        Err(ProgressAdmissionError::Progress(
            ProgressError::InvalidResult
        ))
    );
    assert_eq!(replica.actor(1), Some((actor(12), 2)));
    assert_eq!(progress.reserved_capacity_bytes(), reserved);
}

#[test]
fn state_refusal_does_not_commit_preflighted_result() {
    let (mut replica, mut progress) = setup();
    let mut bytes = [0u8; r4::MAX_PAYLOAD_BYTES];
    let mut bad = actor(99);
    bad.mode = 8;
    let size = r4::encode(
        GroupInput {
            header: GroupHeader {
                group_id: 10,
                interest_generation: 1,
                commit_tick: 1,
                resolved_through: 6,
            },
            actors: &[bad],
            lifecycle: &[],
            results: &[result(8, 1)],
            events: &[],
        },
        &mut bytes,
    )
    .unwrap();
    let view = r4::decode(&bytes[..size]).unwrap();
    assert_eq!(
        replica.admit_with_progress(&view, &mut progress, &Schema),
        Err(ProgressAdmissionError::State(ReplicaError::Schema))
    );
    assert_eq!(replica.actor(1), Some((actor(10), 0)));
    assert_eq!(progress.resolved_through(), 6);
    admit(&mut replica, &mut progress, 1, 11, 6, &[result(8, 1)], &[]).unwrap();
}

#[test]
fn event_bearing_result_refuses_before_state_or_progress() {
    let (mut replica, mut progress) = setup();
    let event = Event {
        event_id: 9,
        commit_tick: 1,
        origin_epoch: 1,
        origin_input_sequence: 8,
        kind: 1,
        ordinal: 0,
        flags: 0,
    };
    assert_eq!(
        admit(
            &mut replica,
            &mut progress,
            1,
            99,
            6,
            &[result(8, 1)],
            &[event]
        ),
        Err(ProgressAdmissionError::Progress(
            ProgressError::UnsupportedEvent
        ))
    );
    assert_eq!(replica.actor(1), Some((actor(10), 0)));
    assert_eq!(progress.pending_count(), 2);
}

#[test]
fn finite_registration_refuses_before_next_sequence() {
    let mut progress = Progress::from_baseline(10, 1, 0, &[]).unwrap();
    let reserved = progress.reserved_capacity_bytes();
    for expected in 1..=256 {
        assert_eq!(progress.reserve_input(5), Ok(expected));
    }
    assert_eq!(progress.reserve_input(5), Err(ProgressError::Capacity));
    assert_eq!(progress.pending_count(), 256);
    assert_eq!(progress.reserved_capacity_bytes(), reserved);
    assert!(matches!(
        Progress::from_baseline(
            10,
            1,
            6,
            &[PendingInput {
                sequence: 8,
                action_type: 5,
                resolved: false
            }]
        ),
        Err(ProgressError::InvalidBaseline)
    ));
}

#[test]
fn baseline_sample_time_refuses_old_newly_reported_result() {
    let initial = [actor(10)];
    let mut replica = Replica::from_baseline(
        &[PartitionSeed {
            group_id: 10,
            interest_generation: 1,
            sample_tick: 10,
            actors: &initial,
        }],
        LifecycleCodes {
            spawn: 1,
            despawn: 2,
            leave: 3,
            reenter: 4,
        },
        &Schema,
    )
    .unwrap();
    let mut progress = Progress::from_baseline(
        10,
        1,
        6,
        &[PendingInput {
            sequence: 7,
            action_type: 5,
            resolved: false,
        }],
    )
    .unwrap();
    assert_eq!(
        admit(&mut replica, &mut progress, 11, 99, 7, &[result(7, 5)], &[]),
        Err(ProgressAdmissionError::Progress(
            ProgressError::InvalidResult
        ))
    );
    assert_eq!(replica.actor(1), Some((actor(10), 10)));
    assert_eq!(progress.resolved_through(), 6);
}

#[test]
fn baseline_individually_resolved_higher_input_closes_after_gap() {
    let (mut replica, _) = setup();
    let mut progress = Progress::from_baseline(
        10,
        1,
        6,
        &[
            PendingInput {
                sequence: 7,
                action_type: 5,
                resolved: false,
            },
            PendingInput {
                sequence: 8,
                action_type: 5,
                resolved: true,
            },
        ],
    )
    .unwrap();
    admit(&mut replica, &mut progress, 1, 11, 8, &[result(7, 1)], &[]).unwrap();
    assert_eq!(progress.resolved_through(), 8);
    assert_eq!(progress.pending_count(), 0);
}
