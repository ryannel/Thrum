//! Bounded pre-submission ownership, chronology and pressure tests.

use thrum_network::{
    outbound::{
        OutboundError, OutboundValidator, PartitionSeed, StateCoverage, TransferError, UnsentQueue,
    },
    r4::{self, Actor, Event, GroupHeader, GroupInput},
    retirement::PeerKey,
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

fn key(peer_id: u64) -> PeerKey {
    PeerKey {
        match_id: *b"0123456789abcdef",
        peer_id,
        connection_epoch: peer_id + 100,
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
    let actors = [actor(
        u64::from(group_id),
        u16::try_from(tick).expect("bounded test value"),
    )];
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
    .expect("test operation should succeed");
    (bytes, size)
}

#[test]
fn state_only_coalesces_to_chronological_tail_across_other_partition() {
    let invalid = PeerKey {
        match_id: [0; 16],
        ..key(1)
    };
    assert_eq!(
        UnsentQueue::from_baseline(invalid, &[seed(10)]).err(),
        Some(OutboundError::WrongIdentity)
    );
    let mut queue = UnsentQueue::from_baseline(key(1), &[seed(10), seed(20)])
        .expect("test operation should succeed");
    let reserved = queue.reserved_capacity_bytes();
    let (mut first, size) = payload(10, 1, false);
    queue
        .capture(&first[..size], &Schema)
        .expect("test operation should succeed");
    first[..size].fill(0);
    assert_eq!(
        queue
            .peek_next()
            .expect("test operation should succeed")
            .expect("test operation should succeed")
            .header
            .commit_tick,
        1
    );
    let (middle, size) = payload(20, 2, true);
    queue
        .capture(&middle[..size], &Schema)
        .expect("test operation should succeed");
    let (last, size) = payload(10, 3, false);
    queue
        .capture(&last[..size], &Schema)
        .expect("test operation should succeed");
    assert_eq!(queue.len(), 2);
    assert_eq!(
        queue
            .peek_next()
            .expect("test operation should succeed")
            .expect("test operation should succeed")
            .header,
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
                assert_eq!(
                    r4::decode(unit.payload)
                        .expect("test operation should succeed")
                        .events()
                        .len(),
                    1
                );
                Ok::<_, ()>(())
            })
            .expect("test operation should succeed")
    );
    assert_eq!(
        queue
            .peek_next()
            .expect("test operation should succeed")
            .expect("test operation should succeed")
            .header
            .commit_tick,
        3
    );
    assert_eq!(
        queue
            .peek_next()
            .expect("test operation should succeed")
            .expect("test operation should succeed")
            .payload,
        &last[..size]
    );
    assert_eq!(queue.reserved_capacity_bytes(), reserved);
}

#[test]
fn mandatory_barrier_prevents_crossing_and_failed_transfer_retains_bytes() {
    let mut queue =
        UnsentQueue::from_baseline(key(1), &[seed(10)]).expect("test operation should succeed");
    for (tick, mandatory) in [(1, false), (2, true), (3, false)] {
        let (bytes, size) = payload(10, tick, mandatory);
        queue
            .capture(&bytes[..size], &Schema)
            .expect("test operation should succeed");
    }
    assert_eq!(queue.len(), 3);
    let before = queue
        .peek_next()
        .expect("test operation should succeed")
        .expect("test operation should succeed")
        .payload
        .to_vec();
    assert_eq!(
        queue.transfer_next(|_| Err::<(), _>("no ledger credit")),
        Err(TransferError::Sink("no ledger credit"))
    );
    assert_eq!(queue.len(), 3);
    assert_eq!(
        queue
            .peek_next()
            .expect("test operation should succeed")
            .expect("test operation should succeed")
            .payload,
        before
    );
    let mut ticks = Vec::new();
    while queue
        .transfer_next(|unit| {
            ticks.push(unit.header.commit_tick);
            Ok::<_, ()>(())
        })
        .expect("test operation should succeed")
    {}
    assert_eq!(ticks, [1, 2, 3]);
    assert!(queue.is_empty());
}

#[test]
fn seventeenth_mandatory_unit_refuses_without_consuming_tick_or_slot() {
    let mut queue =
        UnsentQueue::from_baseline(key(1), &[seed(10)]).expect("test operation should succeed");
    let reserved = queue.reserved_capacity_bytes();
    for tick in 1..=16 {
        let (bytes, size) = payload(10, tick, true);
        queue
            .capture(&bytes[..size], &Schema)
            .expect("test operation should succeed");
    }
    let (seventeenth, size) = payload(10, 17, true);
    assert_eq!(
        queue.capture(&seventeenth[..size], &Schema),
        Err(OutboundError::Capacity)
    );
    assert_eq!(queue.len(), 16);
    queue
        .transfer_next(|_| Ok::<_, ()>(()))
        .expect("test operation should succeed");
    queue
        .capture(&seventeenth[..size], &Schema)
        .expect("test operation should succeed");
    assert_eq!(queue.len(), 16);
    assert_eq!(queue.reserved_capacity_bytes(), reserved);
}

#[test]
fn batch_capacity_refusal_preserves_coalesced_bytes_and_all_cursors() {
    let mut queue = UnsentQueue::from_baseline(key(1), &[seed(10), seed(20)])
        .expect("test operation should succeed");
    let (old, old_len) = payload(10, 1, false);
    queue
        .capture(&old[..old_len], &Schema)
        .expect("test operation should succeed");
    for tick in 2..=16 {
        let (unit, len) = payload(20, tick, true);
        queue
            .capture(&unit[..len], &Schema)
            .expect("test operation should succeed");
    }
    let (new_state, state_len) = payload(10, 17, false);
    let (mandatory, mandatory_len) = payload(20, 17, true);
    let batch = [&new_state[..state_len], &mandatory[..mandatory_len]];
    assert_eq!(
        queue.capture_batch(&batch, &Schema),
        Err(OutboundError::Capacity)
    );
    assert_eq!(queue.len(), 16);
    let first = queue
        .peek_next()
        .expect("test operation should succeed")
        .expect("test operation should succeed");
    assert_eq!(first.header.commit_tick, 1);
    assert_eq!(first.payload, &old[..old_len]);
    for _ in 0..2 {
        queue
            .transfer_next(|_| Ok::<_, ()>(()))
            .expect("test operation should succeed");
    }
    queue
        .capture_batch(&batch, &Schema)
        .expect("test operation should succeed");
    assert_eq!(queue.len(), 16);
    let mut last_two = Vec::new();
    while queue
        .transfer_next(|unit| {
            last_two.push(unit.header);
            Ok::<_, ()>(())
        })
        .expect("test operation should succeed")
    {}
    assert_eq!(last_two[last_two.len() - 2].group_id, 10);
    assert_eq!(last_two[last_two.len() - 1].group_id, 20);
    assert_eq!(last_two[last_two.len() - 2].commit_tick, 17);
    assert_eq!(last_two[last_two.len() - 1].commit_tick, 17);
}

#[test]
fn batch_schema_failure_and_hook_unwind_leave_queue_unmodified() {
    use std::cell::Cell;
    use std::panic::{AssertUnwindSafe, catch_unwind};

    struct PanicOnSecond(Cell<usize>);
    impl OutboundValidator for PanicOnSecond {
        fn group(&self, _: &r4::GroupView<'_>) -> Option<StateCoverage> {
            let prior = self.0.get();
            self.0.set(prior + 1);
            assert_eq!(prior, 0, "second group hook unwinds");
            Some(StateCoverage::CompletePartition)
        }
    }
    struct RejectGroup20;
    impl OutboundValidator for RejectGroup20 {
        fn group(&self, group: &r4::GroupView<'_>) -> Option<StateCoverage> {
            (group.header.group_id != 20).then_some(StateCoverage::CompletePartition)
        }
    }

    let mut queue = UnsentQueue::from_baseline(key(1), &[seed(10), seed(20)])
        .expect("test operation should succeed");
    let (first, first_len) = payload(10, 1, false);
    let (second, second_len) = payload(20, 1, false);
    let malformed = &second[..second_len - 1];
    assert_eq!(
        queue.capture_batch(&[&first[..first_len], malformed], &Schema),
        Err(OutboundError::Framing(r4::GroupError::LengthMismatch))
    );
    assert!(queue.is_empty());
    let batch = [&first[..first_len], &second[..second_len]];
    assert_eq!(
        queue.capture_batch(&batch, &RejectGroup20),
        Err(OutboundError::Schema)
    );
    assert!(queue.is_empty());
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            queue.capture_batch(&batch, &PanicOnSecond(Cell::new(0)))
        }))
        .is_err()
    );
    assert!(queue.is_empty());
    queue
        .capture_batch(&batch, &Schema)
        .expect("test operation should succeed");
    assert_eq!(queue.len(), 2);
    assert_eq!(
        queue.capture_batch(&[batch[0]; 5], &Schema),
        Err(OutboundError::BatchCapacity)
    );
    assert_eq!(queue.len(), 2);
}

#[test]
fn empty_batch_is_noop_and_repeated_destination_uses_final_exact_length() {
    let mut queue =
        UnsentQueue::from_baseline(key(1), &[seed(10)]).expect("test operation should succeed");
    assert_eq!(queue.capture_batch(&[], &Schema), Ok(()));
    assert!(queue.is_empty());
    let (long, long_len) = payload(10, 1, false);
    let mut short = [0u8; r4::MAX_PAYLOAD_BYTES];
    let short_len = r4::encode(
        GroupInput {
            header: GroupHeader {
                group_id: 10,
                interest_generation: 1,
                commit_tick: 2,
                resolved_through: 0,
            },
            actors: &[],
            lifecycle: &[],
            results: &[],
            events: &[],
        },
        &mut short,
    )
    .expect("test operation should succeed");
    queue
        .capture_batch(&[&long[..long_len], &short[..short_len]], &Schema)
        .expect("test operation should succeed");
    assert_eq!(queue.len(), 1);
    let final_unit = queue
        .peek_next()
        .expect("test operation should succeed")
        .expect("test operation should succeed");
    assert_eq!(final_unit.header.commit_tick, 2);
    assert_eq!(final_unit.payload, &short[..short_len]);
    assert_eq!(
        r4::decode(final_unit.payload)
            .expect("test operation should succeed")
            .actors()
            .len(),
        0
    );
}

#[test]
fn malformed_and_stale_group_refuse_without_queue_mutation() {
    let mut queue =
        UnsentQueue::from_baseline(key(1), &[seed(10)]).expect("test operation should succeed");
    let (bytes, size) = payload(10, 1, false);
    assert_eq!(
        queue.capture(&bytes[..size - 1], &Schema),
        Err(OutboundError::Framing(r4::GroupError::LengthMismatch))
    );
    assert!(queue.is_empty());
    queue
        .capture(&bytes[..size], &Schema)
        .expect("test operation should succeed");
    assert_eq!(
        queue.capture(&bytes[..size], &Schema),
        Err(OutboundError::StaleTick)
    );
    assert_eq!(queue.len(), 1);
    assert_eq!(
        queue
            .peek_next()
            .expect("test operation should succeed")
            .expect("test operation should succeed")
            .header
            .commit_tick,
        1
    );
}

#[test]
fn partial_actor_updates_cannot_erase_each_other() {
    let mut queue =
        UnsentQueue::from_baseline(key(1), &[seed(10)]).expect("test operation should succeed");
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
                actors: &[actor(id, u16::try_from(tick).expect("bounded test value"))],
                lifecycle: &[],
                results: &[],
                events: &[],
            },
            &mut bytes,
        )
        .expect("test operation should succeed");
        queue
            .capture(&bytes[..size], &PartialSchema)
            .expect("test operation should succeed");
    }
    assert_eq!(queue.len(), 2);
    let mut ids = Vec::new();
    while queue
        .transfer_next(|unit| {
            assert!(unit.mandatory);
            ids.push(
                r4::decode(unit.payload)
                    .expect("test operation should succeed")
                    .actors()
                    .next()
                    .expect("test operation should succeed")
                    .wire_id,
            );
            Ok::<_, ()>(())
        })
        .expect("test operation should succeed")
    {}
    assert_eq!(ids, [101, 102]);
}

#[test]
fn capture_refuses_cross_partition_time_regression() {
    let mut queue = UnsentQueue::from_baseline(key(1), &[seed(10), seed(20)])
        .expect("test operation should succeed");
    let (later, later_size) = payload(10, 3, false);
    queue
        .capture(&later[..later_size], &Schema)
        .expect("test operation should succeed");
    let (earlier, earlier_size) = payload(20, 2, false);
    assert_eq!(
        queue.capture(&earlier[..earlier_size], &Schema),
        Err(OutboundError::OutOfOrderBarrier)
    );
    assert_eq!(queue.len(), 1);
    let (equal, equal_size) = payload(20, 3, false);
    queue
        .capture(&equal[..equal_size], &Schema)
        .expect("test operation should succeed");
    assert_eq!(queue.len(), 2);
    let mut groups = Vec::new();
    while queue
        .transfer_next(|unit| {
            groups.push((unit.header.commit_tick, unit.header.group_id));
            Ok::<_, ()>(())
        })
        .expect("test operation should succeed")
    {}
    assert_eq!(groups, [(3, 10), (3, 20)]);
}

#[test]
fn unwound_external_transfer_poisons_queue_until_peer_teardown() {
    use std::panic::{AssertUnwindSafe, catch_unwind};

    let mut queue =
        UnsentQueue::from_baseline(key(1), &[seed(10)]).expect("test operation should succeed");
    let (bytes, size) = payload(10, 1, true);
    queue
        .capture(&bytes[..size], &Schema)
        .expect("test operation should succeed");
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
