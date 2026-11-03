//! N1-owned unsent payload transfer to the exact ordinary sent-frame ring.

use thrum_network::envelope;
use thrum_network::outbound::{
    OutboundValidator, PartitionSeed, StateCoverage, TransferError, UnsentQueue,
};
use thrum_network::r4::{self, ActionResult, Actor, GroupHeader, GroupInput};
use thrum_network::retirement::PeerKey;
use thrum_network::sent::{OrdinaryLedger, SubmitError};

struct Schema;
impl OutboundValidator for Schema {
    fn group(&self, group: &r4::GroupView<'_>) -> Option<StateCoverage> {
        group
            .actors()
            .all(|actor| actor.mode == 7)
            .then_some(StateCoverage::CompletePartition)
    }
}

fn key() -> PeerKey {
    PeerKey {
        match_id: *b"0123456789abcdef",
        peer_id: 3,
        connection_epoch: 7,
    }
}

fn actor(health: u16) -> Actor {
    Actor {
        wire_id: 10,
        sector: [0; 3],
        offset: [0; 3],
        quaternion: [0, 0, 0, 32767],
        velocity: [0; 3],
        mode: 7,
        flags: 0,
        health,
        inventory_revision: 0,
        owner: 0,
        visibility_generation: 1,
    }
}

fn payload(tick: u64, result: bool) -> Vec<u8> {
    let mut bytes = vec![0; r4::MAX_PAYLOAD_BYTES];
    let actors = [actor(tick as u16)];
    let outcomes = [ActionResult {
        input_sequence: 1,
        resolved_tick: tick,
        first_event_id: 0,
        action_type: 1,
        reason: 0,
        event_count: 0,
        disposition: 0,
    }];
    let len = r4::encode(
        GroupInput {
            header: GroupHeader {
                group_id: 5,
                interest_generation: 1,
                commit_tick: tick,
                resolved_through: u64::from(result),
            },
            actors: &actors,
            lifecycle: &[],
            results: if result { &outcomes } else { &[] },
            events: &[],
        },
        &mut bytes,
    )
    .unwrap();
    bytes.truncate(len);
    bytes
}

fn queue() -> UnsentQueue {
    UnsentQueue::from_baseline(
        key(),
        &[PartitionSeed {
            group_id: 5,
            interest_generation: 1,
            sample_tick: 0,
            resolved_through: 0,
        }],
    )
    .unwrap()
}

#[test]
fn coalesced_state_then_mandatory_result_transfer_exactly_once() {
    let mut queue = queue();
    let mut ledger = OrdinaryLedger::new(key(), 0, 0).unwrap();
    queue.capture(&payload(1, false), &Schema).unwrap();
    queue.capture(&payload(2, false), &Schema).unwrap();
    queue.capture(&payload(3, true), &Schema).unwrap();
    assert_eq!(queue.len(), 2);
    assert_eq!(
        ledger.submit_next_from_queue(&mut queue, key(), 0),
        Ok(Some(1))
    );
    assert_eq!(queue.len(), 1);
    let first = ledger.next_write().unwrap().unwrap();
    assert_eq!(first.sequence, 1);
    let message = envelope::decode(first.first).unwrap();
    assert_eq!(r4::decode(message.payload).unwrap().header.commit_tick, 2);
    ledger
        .accept_written(1, first.first.len() + first.second.len())
        .unwrap();
    assert_eq!(
        ledger.submit_next_from_queue(&mut queue, key(), 0),
        Ok(Some(2))
    );
    assert!(queue.is_empty());
    assert_eq!(
        ledger.submit_next_from_queue(&mut queue, key(), 0),
        Ok(None)
    );
    let second = ledger.next_write().unwrap().unwrap();
    assert_eq!(second.sequence, 2);
    let message = envelope::decode(second.first).unwrap();
    let group = r4::decode(message.payload).unwrap();
    assert_eq!(group.header.commit_tick, 3);
    assert_eq!(
        group
            .results()
            .map(|r| r.input_sequence)
            .collect::<Vec<_>>(),
        [1]
    );
    ledger
        .accept_written(2, second.first.len() + second.second.len())
        .unwrap();
    let mut calls = 0;
    assert_eq!(
        ledger.acknowledge_through(key(), 2, |identity, through| {
            assert_eq!((identity, through), (key(), 1));
            calls += 1;
            Ok::<_, ()>(())
        }),
        Ok(2)
    );
    assert_eq!(calls, 1);
    assert_eq!(ledger.result_retired_through(), 1);
    assert_eq!(ledger.retained_frame_bytes(), 0);
    assert_eq!(ledger.outstanding_entries(), 0);
}

#[test]
fn sent_capacity_and_reject_refuse_without_consuming_queued_bytes_or_sequence() {
    let mut queue = queue();
    let mut ledger = OrdinaryLedger::new(key(), 0, 0).unwrap();
    queue.capture(&payload(1, false), &Schema).unwrap();
    let before = queue.peek_next().unwrap().unwrap().payload.to_vec();
    let wrong = PeerKey {
        connection_epoch: 8,
        ..key()
    };
    assert_eq!(
        ledger.submit_next_from_queue(&mut queue, wrong, 0),
        Err(TransferError::Sink(SubmitError::WrongIdentity))
    );
    assert_eq!(queue.len(), 1);
    assert_eq!(ledger.last_sequence(), 0);
    let mut other_ledger = OrdinaryLedger::new(wrong, 0, 0).unwrap();
    assert_eq!(
        other_ledger.submit_next_from_queue(&mut queue, wrong, 0),
        Err(TransferError::Sink(SubmitError::WrongIdentity))
    );
    assert_eq!(other_ledger.last_sequence(), 0);
    assert_eq!(queue.peek_next().unwrap().unwrap().payload, before);

    let mut empty_bytes = [0u8; r4::HEADER_BYTES];
    assert_eq!(
        r4::encode(
            GroupInput {
                header: GroupHeader {
                    group_id: 5,
                    interest_generation: 1,
                    commit_tick: 1,
                    resolved_through: 0,
                },
                actors: &[],
                lifecycle: &[],
                results: &[],
                events: &[],
            },
            &mut empty_bytes,
        ),
        Ok(r4::HEADER_BYTES)
    );
    for _ in 0..112 {
        ledger.submit_state_group(key(), &empty_bytes, 0).unwrap();
    }
    assert_eq!(
        ledger.submit_next_from_queue(&mut queue, key(), 0),
        Err(TransferError::Sink(SubmitError::FrameCapacity))
    );
    assert_eq!(ledger.last_sequence(), 112);
    assert_eq!(queue.len(), 1);
    assert_eq!(queue.peek_next().unwrap().unwrap().payload, before);

    let mut rejected = OrdinaryLedger::new(key(), 0, 0).unwrap();
    rejected.submit_state_group(key(), &empty_bytes, 0).unwrap();
    let frame = rejected.next_write().unwrap().unwrap();
    rejected
        .accept_written(1, frame.first.len() + frame.second.len())
        .unwrap();
    rejected.reject_state_group(key(), 1).unwrap();
    assert_eq!(
        rejected.submit_next_from_queue(&mut queue, key(), 0),
        Err(TransferError::Sink(SubmitError::ReplacementRequired))
    );
    assert_eq!(rejected.last_sequence(), 1);
    assert_eq!(queue.len(), 1);
    assert_eq!(queue.peek_next().unwrap().unwrap().payload, before);
}

#[test]
#[ignore = "local release-only four-peer queue-to-sent release-shape probe"]
fn measure_four_peer_queue_to_sent_release_shape() {
    use std::hint::black_box;
    use std::time::Instant;

    let keys: [_; 4] = std::array::from_fn(|index| PeerKey {
        peer_id: index as u64 + 1,
        connection_epoch: index as u64 + 101,
        ..key()
    });
    let seeds: [_; 4] = std::array::from_fn(|index| PartitionSeed {
        group_id: index as u32 + 1,
        interest_generation: 1,
        sample_tick: 0,
        resolved_through: 0,
    });
    let mut queues: [_; 4] =
        std::array::from_fn(|index| UnsentQueue::from_baseline(keys[index], &seeds).unwrap());
    let mut ledgers: [_; 4] =
        std::array::from_fn(|index| OrdinaryLedger::new(keys[index], 0, 0).unwrap());
    let reserved: usize = queues
        .iter()
        .map(UnsentQueue::reserved_capacity_bytes)
        .sum::<usize>()
        + ledgers
            .iter()
            .map(OrdinaryLedger::reserved_capacity_bytes)
            .sum::<usize>();
    let mut groups: [Vec<u8>; 4] = std::array::from_fn(|partition| {
        let actors: [_; 32] = std::array::from_fn(|index| Actor {
            wire_id: (partition * 32 + index + 1) as u64,
            ..actor(1)
        });
        let mut bytes = vec![0; r4::MAX_PAYLOAD_BYTES];
        let len = r4::encode(
            GroupInput {
                header: GroupHeader {
                    group_id: partition as u32 + 1,
                    interest_generation: 1,
                    commit_tick: 1,
                    resolved_through: 0,
                },
                actors: &actors,
                lifecycle: &[],
                results: &[],
                events: &[],
            },
            &mut bytes,
        )
        .unwrap();
        bytes.truncate(len);
        bytes
    });
    const WINDOWS: u64 = 2_000;
    let start = Instant::now();
    let mut checksum = 0u64;
    for window in 0..WINDOWS {
        for within in 1..=3 {
            let tick = window * 3 + within;
            for group in &mut groups {
                group[8..16].copy_from_slice(&tick.to_le_bytes());
                for queue in &mut queues {
                    queue.capture(black_box(group), &Schema).unwrap();
                }
            }
        }
        for ((queue, ledger), &key) in queues.iter_mut().zip(&mut ledgers).zip(&keys) {
            assert_eq!(queue.len(), 4);
            while let Some(sequence) = ledger.submit_next_from_queue(queue, key, 0).unwrap() {
                checksum ^= black_box(sequence);
            }
            while let Some(frame) = ledger.next_write().unwrap() {
                for &byte in frame.first.iter().chain(frame.second) {
                    checksum = checksum
                        .wrapping_mul(16_777_619)
                        .wrapping_add(u64::from(byte));
                }
                ledger
                    .accept_written(
                        frame.sequence,
                        frame.first.len() + frame.second.len() - frame.accepted_bytes,
                    )
                    .unwrap();
            }
            ledger
                .acknowledge_through(key, ledger.last_sequence(), |_, _| Ok::<_, ()>(()))
                .unwrap();
            black_box(&*queue);
            black_box(&*ledger);
        }
    }
    assert!(queues.iter().all(UnsentQueue::is_empty));
    assert!(
        ledgers
            .iter()
            .all(|ledger| ledger.retained_frame_bytes() == 0)
    );
    assert_eq!(
        reserved,
        queues
            .iter()
            .map(UnsentQueue::reserved_capacity_bytes)
            .sum::<usize>()
            + ledgers
                .iter()
                .map(OrdinaryLedger::reserved_capacity_bytes)
                .sum::<usize>()
    );
    println!(
        "n1_queue_sent_four_peer_release_shape_ns={:.1} reserved_rust_bytes={} checksum={}",
        start.elapsed().as_nanos() as f64 / WINDOWS as f64,
        reserved,
        black_box(checksum)
    );
}
