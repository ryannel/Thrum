//! Structural four-partition mandatory pressure through the actual N1 owner.
//! The permissive validator deliberately does not certify a game schema.

use thrum_network::envelope;
use thrum_network::outbound::{OutboundError, OutboundValidator, PartitionSeed, StateCoverage};
use thrum_network::peer::{OrdinaryPeer, PeerError, PeerPhase};
use thrum_network::r4::{self, ActionResult, Actor, Event, GroupHeader, GroupInput, GroupView};
use thrum_network::retirement::PeerKey;
use thrum_network::retirement::RetirementError;
use thrum_network::sent::AckError;

fn key() -> PeerKey {
    PeerKey {
        match_id: *b"0123456789abcdef",
        peer_id: 1,
        connection_epoch: 7,
    }
}

struct StructuralOnly;
impl OutboundValidator for StructuralOnly {
    fn group(&self, _: &GroupView<'_>) -> Option<StateCoverage> {
        Some(StateCoverage::CompletePartition)
    }
}

fn payload(tick: u64, group_id: u32, event_count: usize) -> Vec<u8> {
    let first = u64::from(group_id - 1) * 32 + 1;
    let actors: [_; 32] = std::array::from_fn(|index| Actor {
        wire_id: first + index as u64,
        sector: [0; 3],
        offset: [0; 3],
        quaternion: [0, 0, 0, 32767],
        velocity: [0; 3],
        mode: 1,
        flags: 0,
        health: 100,
        inventory_revision: 0,
        owner: 0,
        visibility_generation: 1,
    });
    let results: Vec<_> = if group_id == 1 {
        (0..64)
            .map(|index| ActionResult {
                input_sequence: (tick - 1) * 64 + index + 1,
                resolved_tick: tick,
                first_event_id: 0,
                action_type: 1,
                reason: 0,
                event_count: 0,
                disposition: 0,
            })
            .collect()
    } else {
        Vec::new()
    };
    let events: Vec<_> = (0..event_count as u64)
        .map(|index| Event {
            event_id: (tick - 1) * 256 + u64::from(group_id - 1) * 64 + index + 1,
            commit_tick: tick,
            origin_epoch: 0,
            origin_input_sequence: 0,
            kind: 1,
            ordinal: 0,
            flags: 0,
        })
        .collect();
    let mut bytes = vec![0; r4::MAX_PAYLOAD_BYTES];
    let len = r4::encode(
        GroupInput {
            header: GroupHeader {
                group_id,
                interest_generation: 1,
                commit_tick: tick,
                resolved_through: if group_id == 1 { tick * 64 } else { 0 },
            },
            actors: &actors,
            lifecycle: &[],
            results: &results,
            events: &events,
        },
        &mut bytes,
    )
    .expect("test operation should succeed");
    bytes.truncate(len);
    bytes
}

fn four_partition_peer() -> OrdinaryPeer {
    let seeds: [_; 4] = std::array::from_fn(|index| PartitionSeed {
        group_id: u32::try_from(index).expect("bounded test value") + 1,
        interest_generation: 1,
        sample_tick: 0,
        resolved_through: 0,
    });
    OrdinaryPeer::from_baseline(key(), &seeds, 0, 0).expect("test operation should succeed")
}

fn mixed_queued_and_sent_peer() -> OrdinaryPeer {
    let mut peer = four_partition_peer();
    let first: [_; 4] = std::array::from_fn(|index| {
        payload(1, u32::try_from(index).expect("bounded test value") + 1, 32)
    });
    let borrowed: [&[u8]; 4] = std::array::from_fn(|index| first[index].as_slice());
    peer.capture_batch(&borrowed, &StructuralOnly)
        .expect("test operation should succeed");
    assert_eq!(peer.submit_next(0), Ok(Some(1)));
    assert_eq!(peer.submit_next(0), Ok(Some(2)));
    for sequence in 1..=2 {
        let frame = peer
            .next_write()
            .expect("test operation should succeed")
            .expect("test operation should succeed");
        assert_eq!(frame.sequence, sequence);
        let length = frame.first.len() + frame.second.len();
        peer.accept_written(sequence, length)
            .expect("test operation should succeed");
    }
    assert_eq!(peer.queued_groups(), 2);
    assert_eq!(peer.outstanding_frames(), 2);
    peer
}

#[test]
fn aggregate_event_pressure_refuses_whole_tick_before_mutation() {
    let mut peer = four_partition_peer();
    let first: [_; 4] = std::array::from_fn(|index| {
        payload(1, u32::try_from(index).expect("bounded test value") + 1, 32)
    });
    let first_refs: [&[u8]; 4] = std::array::from_fn(|index| first[index].as_slice());
    peer.capture_batch(&first_refs, &StructuralOnly)
        .expect("test operation should succeed");
    let second: [_; 4] = std::array::from_fn(|index| {
        payload(2, u32::try_from(index).expect("bounded test value") + 1, 32)
    });
    let second_refs: [&[u8]; 4] = std::array::from_fn(|index| second[index].as_slice());
    assert_eq!(
        peer.capture_batch(&second_refs, &StructuralOnly),
        Err(PeerError::Operation(OutboundError::AggregateEventCapacity))
    );
    assert_eq!(peer.queued_groups(), 4);
    assert_eq!(peer.last_sequence(), 0);
    assert_eq!(peer.phase(), PeerPhase::RequiresClose);
}

#[test]
fn aggregate_result_pressure_refuses_before_fifth_tick() {
    let seed = PartitionSeed {
        group_id: 1,
        interest_generation: 1,
        sample_tick: 0,
        resolved_through: 0,
    };
    let mut peer =
        OrdinaryPeer::from_baseline(key(), &[seed], 0, 0).expect("test operation should succeed");
    for tick in 1..=4 {
        let group = payload(tick, 1, 0);
        peer.capture_batch(&[&group], &StructuralOnly)
            .expect("test operation should succeed");
    }
    let fifth = payload(5, 1, 0);
    assert_eq!(
        peer.capture_batch(&[&fifth], &StructuralOnly),
        Err(PeerError::Operation(OutboundError::AggregateResultCapacity))
    );
    assert_eq!(peer.queued_groups(), 4);
    assert_eq!(peer.last_sequence(), 0);
    assert_eq!(peer.phase(), PeerPhase::RequiresClose);
}

#[test]
fn mixed_queued_and_sent_events_share_one_credit() {
    let mut peer = mixed_queued_and_sent_peer();
    let second: [_; 4] = std::array::from_fn(|index| {
        payload(2, u32::try_from(index).expect("bounded test value") + 1, 16)
    });
    let borrowed: [&[u8]; 4] = std::array::from_fn(|index| second[index].as_slice());
    assert_eq!(
        peer.capture_batch(&borrowed, &StructuralOnly),
        Err(PeerError::Operation(OutboundError::AggregateEventCapacity))
    );
    assert_eq!(peer.queued_groups(), 2);
    assert_eq!(peer.outstanding_frames(), 2);
    assert_eq!(peer.last_sequence(), 2);
    assert_eq!(peer.phase(), PeerPhase::RequiresClose);
}

#[test]
fn failed_authority_ack_keeps_credit_until_exact_retry() {
    let second: [_; 4] = std::array::from_fn(|index| {
        payload(2, u32::try_from(index).expect("bounded test value") + 1, 16)
    });
    let borrowed: [&[u8]; 4] = std::array::from_fn(|index| second[index].as_slice());

    let mut refused = mixed_queued_and_sent_peer();
    assert_eq!(
        refused.acknowledge_through(key(), 2, |_, _| Err::<(), _>("retry")),
        Err(PeerError::Operation(AckError::Retirement(
            RetirementError::Authority("retry")
        )))
    );
    assert_eq!(
        refused.capture_batch(&borrowed, &StructuralOnly),
        Err(PeerError::Operation(OutboundError::AggregateEventCapacity))
    );
    assert_eq!(refused.outstanding_frames(), 2);

    let mut retried = mixed_queued_and_sent_peer();
    assert!(
        retried
            .acknowledge_through(key(), 2, |_, _| Err::<(), _>("retry"))
            .is_err()
    );
    retried
        .acknowledge_through(key(), 2, |_, through| {
            assert_eq!(through, 64);
            Ok::<(), &str>(())
        })
        .expect("test operation should succeed");
    assert_eq!(retried.outstanding_frames(), 0);
    retried
        .capture_batch(&borrowed, &StructuralOnly)
        .expect("test operation should succeed");
    assert_eq!(retried.queued_groups(), 6);
    assert_eq!(retried.last_sequence(), 2);
    assert_eq!(retried.phase(), PeerPhase::Ordinary);
}

#[test]
fn exact_ack_releases_aggregate_credit_for_next_mandatory_tick() {
    let mut peer = four_partition_peer();
    let reserved = peer.reserved_capacity_bytes();
    let mut authority_frontier = 0;
    for tick in 1..=3 {
        let groups: [_; 4] = std::array::from_fn(|index| {
            payload(
                tick,
                u32::try_from(index).expect("bounded test value") + 1,
                32,
            )
        });
        let borrowed: [&[u8]; 4] = std::array::from_fn(|index| groups[index].as_slice());
        peer.capture_batch(&borrowed, &StructuralOnly)
            .expect("test operation should succeed");
        assert_eq!(peer.queued_groups(), 4);
        for group_index in 0..4 {
            let sequence = (tick - 1) * 4 + group_index + 1;
            assert_eq!(peer.submit_next(0), Ok(Some(sequence)));
        }
        for group_index in 0..4 {
            let sequence = (tick - 1) * 4 + group_index + 1;
            let frame = peer
                .next_write()
                .expect("test operation should succeed")
                .expect("test operation should succeed");
            assert_eq!(frame.sequence, sequence);
            let mut bytes = Vec::from(frame.first);
            bytes.extend_from_slice(frame.second);
            let message = envelope::decode(&bytes).expect("test operation should succeed");
            let group = r4::decode(message.payload).expect("test operation should succeed");
            let group_id = u32::try_from((sequence - 1) % 4 + 1).expect("four partitions");
            assert_eq!(message.header.message_sequence, sequence);
            assert_eq!(
                (group.header.commit_tick, group.header.group_id),
                (tick, group_id)
            );
            assert_eq!(message.payload, payload(tick, group_id, 32));
            assert_eq!(group.results().len(), if group_id == 1 { 64 } else { 0 });
            assert_eq!(group.events().len(), 32);
            let first_event = (tick - 1) * 256 + u64::from(group_id - 1) * 64 + 1;
            assert_eq!(
                group
                    .events()
                    .next()
                    .expect("test operation should succeed")
                    .event_id,
                first_event
            );
            peer.accept_written(sequence, bytes.len())
                .expect("test operation should succeed");
        }
        let through = tick * 4;
        peer.acknowledge_through(key(), through, |_, result_through| {
            assert_eq!(result_through, authority_frontier + 64);
            authority_frontier = result_through;
            Ok::<(), ()>(())
        })
        .expect("test operation should succeed");
    }
    assert_eq!(peer.queued_groups(), 0);
    assert_eq!(peer.outstanding_frames(), 0);
    assert_eq!(peer.last_sequence(), 12);
    assert_eq!(peer.result_retired_through(), 192);
    assert_eq!(peer.phase(), PeerPhase::Ordinary);
    assert_eq!(peer.reserved_capacity_bytes(), reserved);
}
