//! Bounded pre-submission ownership, chronology and pressure tests.

use thrum_network::{
    outbound::{
        OutboundError, OutboundValidator, PartitionSeed, StateCoverage, TransferError, UnsentQueue,
    },
    r4::{self, Actor, Event, GroupHeader, GroupInput},
};

struct Schema;
impl OutboundValidator for Schema {
    fn group(&self, group: &r4::GroupView<'_>) -> Option<StateCoverage> {
        group
            .actors()
            .all(|actor| actor.mode == 7)
            .then_some(StateCoverage::CompletePartition)
    }
}

struct PartialSchema;
impl OutboundValidator for PartialSchema {
    fn group(&self, group: &r4::GroupView<'_>) -> Option<StateCoverage> {
        group
            .actors()
            .all(|actor| actor.mode == 7)
            .then_some(StateCoverage::Partial)
    }
}

fn actor(id: u64, health: u16) -> Actor {
    Actor {
        wire_id: id,
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

fn seed(group_id: u32) -> PartitionSeed {
    PartitionSeed {
        group_id,
        interest_generation: 1,
        sample_tick: 0,
        resolved_through: 0,
    }
}

fn payload(group_id: u32, tick: u64, mandatory: bool) -> ([u8; r4::MAX_PAYLOAD_BYTES], usize) {
    let mut bytes = [0u8; r4::MAX_PAYLOAD_BYTES];
    let actors = [actor(u64::from(group_id), tick as u16)];
    let events = [Event {
        event_id: tick,
        commit_tick: tick,
        origin_epoch: 0,
        origin_input_sequence: 0,
        kind: 1,
        ordinal: 0,
        flags: 0,
    }];
    let size = r4::encode(
        GroupInput {
            header: GroupHeader {
                group_id,
                interest_generation: 1,
                commit_tick: tick,
                resolved_through: 0,
            },
            actors: &actors,
            lifecycle: &[],
            results: &[],
            events: if mandatory { &events } else { &[] },
        },
        &mut bytes,
    )
    .unwrap();
    (bytes, size)
}

#[test]
fn state_only_coalesces_to_chronological_tail_across_other_partition() {
    let mut queue = UnsentQueue::from_baseline(&[seed(10), seed(20)]).unwrap();
    let reserved = queue.reserved_capacity_bytes();
    let (mut first, size) = payload(10, 1, false);
    queue.capture(&first[..size], &Schema).unwrap();
    first[..size].fill(0);
    assert_eq!(queue.peek_next().unwrap().unwrap().header.commit_tick, 1);
    let (middle, size) = payload(20, 2, true);
    queue.capture(&middle[..size], &Schema).unwrap();
    let (last, size) = payload(10, 3, false);
    queue.capture(&last[..size], &Schema).unwrap();
    assert_eq!(queue.len(), 2);
    assert_eq!(
        queue.peek_next().unwrap().unwrap().header,
        GroupHeader {
            group_id: 20,
            interest_generation: 1,
            commit_tick: 2,
            resolved_through: 0
        }
    );
    assert!(
        queue
            .transfer_next(|unit| {
                assert!(unit.mandatory);
                assert_eq!(r4::decode(unit.payload).unwrap().events().len(), 1);
                Ok::<_, ()>(())
            })
            .unwrap()
    );
    assert_eq!(queue.peek_next().unwrap().unwrap().header.commit_tick, 3);
    assert_eq!(queue.peek_next().unwrap().unwrap().payload, &last[..size]);
    assert_eq!(queue.reserved_capacity_bytes(), reserved);
}

#[test]
fn mandatory_barrier_prevents_crossing_and_failed_transfer_retains_bytes() {
    let mut queue = UnsentQueue::from_baseline(&[seed(10)]).unwrap();
    for (tick, mandatory) in [(1, false), (2, true), (3, false)] {
        let (bytes, size) = payload(10, tick, mandatory);
        queue.capture(&bytes[..size], &Schema).unwrap();
    }
    assert_eq!(queue.len(), 3);
    let before = queue.peek_next().unwrap().unwrap().payload.to_vec();
    assert_eq!(
        queue.transfer_next(|_| Err::<(), _>("no ledger credit")),
        Err(TransferError::Sink("no ledger credit"))
    );
    assert_eq!(queue.len(), 3);
    assert_eq!(queue.peek_next().unwrap().unwrap().payload, before);
    let mut ticks = Vec::new();
    while queue
        .transfer_next(|unit| {
            ticks.push(unit.header.commit_tick);
            Ok::<_, ()>(())
        })
        .unwrap()
    {}
    assert_eq!(ticks, [1, 2, 3]);
    assert!(queue.is_empty());
}

#[test]
fn seventeenth_mandatory_unit_refuses_without_consuming_tick_or_slot() {
    let mut queue = UnsentQueue::from_baseline(&[seed(10)]).unwrap();
    let reserved = queue.reserved_capacity_bytes();
    for tick in 1..=16 {
        let (bytes, size) = payload(10, tick, true);
        queue.capture(&bytes[..size], &Schema).unwrap();
    }
    let (seventeenth, size) = payload(10, 17, true);
    assert_eq!(
        queue.capture(&seventeenth[..size], &Schema),
        Err(OutboundError::Capacity)
    );
    assert_eq!(queue.len(), 16);
    queue.transfer_next(|_| Ok::<_, ()>(())).unwrap();
    queue.capture(&seventeenth[..size], &Schema).unwrap();
    assert_eq!(queue.len(), 16);
    assert_eq!(queue.reserved_capacity_bytes(), reserved);
}

#[test]
fn malformed_and_stale_group_refuse_without_queue_mutation() {
    let mut queue = UnsentQueue::from_baseline(&[seed(10)]).unwrap();
    let (bytes, size) = payload(10, 1, false);
    assert_eq!(
        queue.capture(&bytes[..size - 1], &Schema),
        Err(OutboundError::Framing(r4::GroupError::LengthMismatch))
    );
    assert!(queue.is_empty());
    queue.capture(&bytes[..size], &Schema).unwrap();
    assert_eq!(
        queue.capture(&bytes[..size], &Schema),
        Err(OutboundError::StaleTick)
    );
    assert_eq!(queue.len(), 1);
    assert_eq!(queue.peek_next().unwrap().unwrap().header.commit_tick, 1);
}

#[test]
fn partial_actor_updates_cannot_erase_each_other() {
    let mut queue = UnsentQueue::from_baseline(&[seed(10)]).unwrap();
    for (tick, id) in [(1, 101), (2, 102)] {
        let mut bytes = [0u8; r4::MAX_PAYLOAD_BYTES];
        let size = r4::encode(
            GroupInput {
                header: GroupHeader {
                    group_id: 10,
                    interest_generation: 1,
                    commit_tick: tick,
                    resolved_through: 0,
                },
                actors: &[actor(id, tick as u16)],
                lifecycle: &[],
                results: &[],
                events: &[],
            },
            &mut bytes,
        )
        .unwrap();
        queue.capture(&bytes[..size], &PartialSchema).unwrap();
    }
    assert_eq!(queue.len(), 2);
    let mut ids = Vec::new();
    while queue
        .transfer_next(|unit| {
            assert!(unit.mandatory);
            ids.push(
                r4::decode(unit.payload)
                    .unwrap()
                    .actors()
                    .next()
                    .unwrap()
                    .wire_id,
            );
            Ok::<_, ()>(())
        })
        .unwrap()
    {}
    assert_eq!(ids, [101, 102]);
}

#[test]
fn capture_refuses_cross_partition_time_regression() {
    let mut queue = UnsentQueue::from_baseline(&[seed(10), seed(20)]).unwrap();
    let (later, later_size) = payload(10, 3, false);
    queue.capture(&later[..later_size], &Schema).unwrap();
    let (earlier, earlier_size) = payload(20, 2, false);
    assert_eq!(
        queue.capture(&earlier[..earlier_size], &Schema),
        Err(OutboundError::OutOfOrderBarrier)
    );
    assert_eq!(queue.len(), 1);
    let (equal, equal_size) = payload(20, 3, false);
    queue.capture(&equal[..equal_size], &Schema).unwrap();
    assert_eq!(queue.len(), 2);
    let mut groups = Vec::new();
    while queue
        .transfer_next(|unit| {
            groups.push((unit.header.commit_tick, unit.header.group_id));
            Ok::<_, ()>(())
        })
        .unwrap()
    {}
    assert_eq!(groups, [(3, 10), (3, 20)]);
}

#[test]
fn unwound_external_transfer_poisons_queue_until_peer_teardown() {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    let mut queue = UnsentQueue::from_baseline(&[seed(10)]).unwrap();
    let (bytes, size) = payload(10, 1, true);
    queue.capture(&bytes[..size], &Schema).unwrap();
    let caught = catch_unwind(AssertUnwindSafe(|| {
        let _: Result<bool, TransferError<()>> =
            queue.transfer_next(|_| panic!("uncertain sink publication"));
    }));
    assert!(caught.is_err());
    assert_eq!(queue.len(), 1);
    assert!(matches!(queue.peek_next(), Err(OutboundError::Poisoned)));
    assert_eq!(
        queue.capture(&bytes[..size], &Schema),
        Err(OutboundError::Poisoned)
    );
    assert_eq!(
        queue.transfer_next(|_| Ok::<_, ()>(())),
        Err(TransferError::Poisoned)
    );
}

#[test]
#[ignore = "isolated release-mode four-peer state-only queue cost probe"]
fn measure_four_peer_three_tick_state_only_queue() {
    use std::{hint::black_box, time::Instant};

    let seeds = [seed(10), seed(20), seed(30), seed(40)];
    let mut queues: [UnsentQueue; 4] =
        std::array::from_fn(|_| UnsentQueue::from_baseline(&seeds).unwrap());
    let reserved: usize = queues
        .iter()
        .map(UnsentQueue::reserved_capacity_bytes)
        .sum();
    let mut payloads = [[0u8; r4::MAX_PAYLOAD_BYTES]; 4];
    let mut lengths = [0; 4];
    for (group, seed) in seeds.iter().enumerate() {
        let (bytes, size) = payload(seed.group_id, 1, false);
        payloads[group] = bytes;
        lengths[group] = size;
    }
    const WINDOWS: u64 = 2_000;
    let start = Instant::now();
    let mut checksum = 0u64;
    for window in 0..WINDOWS {
        for within in 1..=3 {
            let tick = window * 3 + within;
            for group in 0..4 {
                payloads[group][8..16].copy_from_slice(&tick.to_le_bytes());
                for queue in &mut queues {
                    queue
                        .capture(&payloads[group][..lengths[group]], &Schema)
                        .unwrap();
                }
            }
        }
        for queue in &mut queues {
            assert_eq!(queue.len(), 4);
            while queue
                .transfer_next(|unit| {
                    for &byte in unit.payload {
                        checksum = checksum.rotate_left(3) ^ u64::from(byte);
                    }
                    Ok::<_, ()>(())
                })
                .unwrap()
            {}
        }
    }
    black_box(checksum);
    assert_eq!(
        reserved,
        queues
            .iter()
            .map(UnsentQueue::reserved_capacity_bytes)
            .sum()
    );
    println!(
        "n1_four_peer_three_tick_state_only_queue_window_ns={:.1} reserved_owner_bytes={reserved}",
        start.elapsed().as_nanos() as f64 / WINDOWS as f64
    );
}
