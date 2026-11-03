//! Physical ordinary frame ring and exact result admission ACK behavior.

use thrum_network::envelope;
use thrum_network::r4::{self, ActionResult, Actor, Event, GroupHeader, GroupInput};
use thrum_network::retirement::{PeerKey, RetirementError};
use thrum_network::sent::{AckError, OrdinaryLedger, SubmitError, WriteError};

fn key() -> PeerKey {
    PeerKey {
        match_id: *b"0123456789abcdef",
        peer_id: 3,
        connection_epoch: 7,
    }
}

fn payload(tick: u64, actors: &[Actor], results: &[ActionResult], events: &[Event]) -> Vec<u8> {
    let mut bytes = vec![0; r4::MAX_PAYLOAD_BYTES];
    let len = r4::encode(
        GroupInput {
            header: GroupHeader {
                group_id: 5,
                interest_generation: 1,
                commit_tick: tick,
                resolved_through: 0,
            },
            actors,
            lifecycle: &[],
            results,
            events,
        },
        &mut bytes,
    )
    .unwrap();
    bytes.truncate(len);
    bytes
}

fn result(id: u64) -> ActionResult {
    ActionResult {
        input_sequence: id,
        resolved_tick: 1,
        first_event_id: 0,
        action_type: 1,
        reason: 0,
        event_count: 0,
        disposition: 0,
    }
}

fn event(id: u64) -> Event {
    Event {
        event_id: id,
        commit_tick: 1,
        origin_epoch: 0,
        origin_input_sequence: 0,
        kind: 1,
        ordinal: 0,
        flags: 0,
    }
}

fn actor() -> Actor {
    Actor {
        wire_id: 1,
        sector: [0; 3],
        offset: [0; 3],
        quaternion: [0; 4],
        velocity: [0; 3],
        mode: 0,
        flags: 0,
        health: 0,
        inventory_revision: 0,
        owner: 0,
        visibility_generation: 0,
    }
}

fn write_all(ledger: &mut OrdinaryLedger) {
    while let Some(frame) = ledger.next_write().unwrap() {
        let sequence = frame.sequence;
        let remaining = frame.first.len() + frame.second.len() - frame.accepted_bytes;
        ledger.accept_written(sequence, remaining).unwrap();
    }
}

#[test]
fn exact_result_identity_survives_frame_and_journal_retirement_across_gap() {
    let mut ledger = OrdinaryLedger::new(key(), 0, 6).unwrap();
    let reserved = ledger.reserved_capacity_bytes();
    let group8 = payload(1, &[], &[result(8)], &[]);
    assert_eq!(ledger.submit_state_group(key(), &group8, 0), Ok(1));
    let frame = ledger.next_write().unwrap().unwrap();
    assert_eq!(frame.sequence, 1);
    assert!(frame.second.is_empty());
    let decoded = envelope::decode(frame.first).unwrap();
    assert_eq!(decoded.header.message_sequence, 1);
    assert_eq!(decoded.payload, group8);
    assert_eq!(
        ledger.acknowledge_through(key(), 1, |_, _| Ok::<_, ()>(())),
        Err(AckError::InvalidSequence)
    );
    write_all(&mut ledger);
    assert_eq!(
        ledger.acknowledge_through(key(), 1, |_, _| -> Result<(), ()> {
            panic!("higher result cannot advance authority yet")
        }),
        Ok(1)
    );
    assert_eq!(ledger.result_retired_through(), 6);
    assert_eq!(ledger.admitted_higher_result_count(), 1);
    assert_eq!(ledger.outstanding_entries(), 0);
    assert_eq!(ledger.retained_frame_bytes(), 0);

    // Result 8's journal entry is retired. Its independent admitted-higher
    // identity persists and prevents a duplicate ordinary submission.
    assert_eq!(
        ledger.submit_state_group(key(), &group8, 0),
        Err(SubmitError::DuplicateOrInvalidIdentity)
    );
    let group9 = payload(2, &[], &[result(9)], &[]);
    assert_eq!(ledger.submit_state_group(key(), &group9, 0), Ok(2));
    write_all(&mut ledger);
    ledger
        .acknowledge_through(key(), 2, |_, _| -> Result<(), ()> {
            panic!("higher result cannot advance authority yet")
        })
        .unwrap();
    let group7 = payload(3, &[], &[result(7)], &[]);
    ledger.submit_state_group(key(), &group7, 0).unwrap();
    write_all(&mut ledger);
    let mut called = 0;
    ledger
        .acknowledge_through(key(), 3, |identity, through| {
            assert_eq!(identity, key());
            assert_eq!(through, 9);
            called += 1;
            Ok::<_, ()>(())
        })
        .unwrap();
    assert_eq!(called, 1);
    assert_eq!(ledger.result_retired_through(), 9);
    assert_eq!(ledger.admitted_higher_result_count(), 0);
    assert_eq!(ledger.reserved_capacity_bytes(), reserved);
}

#[test]
fn physical_ring_splits_frame_without_wrap_padding_or_alias() {
    let mut ledger = OrdinaryLedger::new(key(), 0, 0).unwrap();
    let actors = [actor(); 32];
    let group = payload(1, &actors, &[], &[]);
    let charge = envelope::HEADER_BYTES + group.len() + 12;
    for sequence in 1..=86 {
        assert_eq!(ledger.submit_state_group(key(), &group, 0), Ok(sequence));
    }
    assert_eq!(ledger.retained_frame_bytes(), 86 * charge);
    assert_eq!(
        ledger.submit_state_group(key(), &group, 0),
        Err(SubmitError::FrameCapacity)
    );
    assert_eq!(ledger.last_sequence(), 86);
    for sequence in 1..=5 {
        let frame = ledger.next_write().unwrap().unwrap();
        assert_eq!(frame.sequence, sequence);
        ledger
            .accept_written(sequence, frame.first.len() + frame.second.len())
            .unwrap();
    }
    ledger
        .acknowledge_through(key(), 5, |_, _| -> Result<(), ()> {
            panic!("state-only ACK has no authority result advance")
        })
        .unwrap();
    assert_eq!(ledger.retained_frame_bytes(), 81 * charge);
    assert_eq!(ledger.submit_state_group(key(), &group, 0), Ok(87));
    for sequence in 6..=86 {
        let frame = ledger.next_write().unwrap().unwrap();
        assert_eq!(frame.sequence, sequence);
        ledger
            .accept_written(sequence, frame.first.len() + frame.second.len())
            .unwrap();
    }
    let frame = ledger.next_write().unwrap().unwrap();
    assert_eq!(frame.sequence, 87);
    assert!(!frame.second.is_empty());
    let mut joined = frame.first.to_vec();
    joined.extend_from_slice(frame.second);
    let decoded = envelope::decode(&joined).unwrap();
    assert_eq!(decoded.header.message_sequence, 87);
    assert_eq!(decoded.payload, group);
    assert_eq!(ledger.retained_frame_bytes(), 82 * charge);
}

#[test]
fn exact_capacity_refuses_before_sequence_or_journal_mutation() {
    let mut ledger = OrdinaryLedger::new(key(), 0, 0).unwrap();
    let empty = payload(1, &[], &[], &[]);
    for sequence in 1..=112 {
        assert_eq!(ledger.submit_state_group(key(), &empty, 0), Ok(sequence));
    }
    let bytes = ledger.retained_frame_bytes();
    assert_eq!(
        ledger.submit_state_group(key(), &empty, 0),
        Err(SubmitError::FrameCapacity)
    );
    assert_eq!(ledger.last_sequence(), 112);
    assert_eq!(ledger.retained_frame_bytes(), bytes);

    let mut result_ledger = OrdinaryLedger::new(key(), 0, 0).unwrap();
    let results: Vec<_> = (1..=64).map(result).collect();
    for block in 0..4 {
        let results: Vec<_> = results
            .iter()
            .map(|r| result(r.input_sequence + block * 64))
            .collect();
        result_ledger
            .submit_state_group(key(), &payload(1, &[], &results, &[]), 0)
            .unwrap();
    }
    assert_eq!(
        result_ledger.submit_state_group(key(), &payload(1, &[], &[result(257)], &[]), 0),
        Err(SubmitError::JournalCapacity)
    );
    assert_eq!(result_ledger.last_sequence(), 4);

    let mut event_ledger = OrdinaryLedger::new(key(), 0, 0).unwrap();
    for block in 0..2 {
        let events: Vec<_> = (1..=64).map(|id| event(id + block * 64)).collect();
        event_ledger
            .submit_state_group(key(), &payload(1, &[], &[], &events), 0)
            .unwrap();
    }
    assert_eq!(
        event_ledger.submit_state_group(key(), &payload(1, &[], &[], &[event(129)]), 0),
        Err(SubmitError::JournalCapacity)
    );
    assert_eq!(event_ledger.last_sequence(), 2);
}

#[test]
fn callback_refusal_retains_exact_receipt_and_wrong_ack_cannot_retarget_it() {
    let mut ledger = OrdinaryLedger::new(key(), 0, 0).unwrap();
    let group = payload(1, &[], &[result(1)], &[]);
    ledger.submit_state_group(key(), &group, 0).unwrap();
    write_all(&mut ledger);
    assert_eq!(
        ledger.acknowledge_through(key(), 1, |_, _| Err("busy")),
        Err(AckError::Retirement(RetirementError::Authority("busy")))
    );
    assert!(ledger.receipt_occupied());
    assert_eq!(ledger.outstanding_entries(), 1);
    assert_eq!(
        ledger.acknowledge_through(key(), 2, |_, _| Ok::<_, &str>(())),
        Err(AckError::ReceiptOccupied)
    );
    assert_eq!(
        ledger.acknowledge_through(key(), 1, |_, through| {
            assert_eq!(through, 1);
            Ok::<_, &str>(())
        }),
        Ok(1)
    );
    assert!(!ledger.receipt_occupied());
    assert_eq!(ledger.outstanding_entries(), 0);
}

#[test]
fn known_partial_write_keeps_sequence_and_uncertain_write_quarantines() {
    let mut ledger = OrdinaryLedger::new(key(), 0, 0).unwrap();
    let group = payload(1, &[], &[], &[]);
    ledger.submit_state_group(key(), &group, 0).unwrap();
    assert_eq!(ledger.accept_written(2, 1), Err(WriteError::WrongSequence));
    assert_eq!(ledger.accept_written(1, 5), Ok(()));
    let frame = ledger.next_write().unwrap().unwrap();
    assert_eq!(frame.sequence, 1);
    assert_eq!(frame.accepted_bytes, 5);
    assert_eq!(
        ledger.accept_written(1, frame.first.len() + 1),
        Err(WriteError::InvalidCount)
    );
    ledger.quarantine_uncertain_write();
    assert_eq!(ledger.next_write().err(), Some(WriteError::Poisoned));
    assert_eq!(
        ledger.submit_state_group(key(), &group, 0),
        Err(SubmitError::Poisoned)
    );
    assert_eq!(ledger.last_sequence(), 1);
}

#[test]
fn unwound_authority_callback_quarantines_both_owners() {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    let mut ledger = OrdinaryLedger::new(key(), 0, 0).unwrap();
    let group = payload(1, &[], &[result(1)], &[]);
    ledger.submit_state_group(key(), &group, 0).unwrap();
    write_all(&mut ledger);
    let caught = catch_unwind(AssertUnwindSafe(|| {
        let _: Result<u64, AckError<()>> =
            ledger.acknowledge_through(key(), 1, |_, _| panic!("uncertain"));
    }));
    assert!(caught.is_err());
    assert!(ledger.is_poisoned());
    assert!(ledger.receipt_occupied());
    assert_eq!(ledger.next_write().err(), Some(WriteError::Poisoned));
    assert_eq!(
        ledger.submit_state_group(key(), &group, 0),
        Err(SubmitError::Poisoned)
    );
    assert_eq!(
        ledger.acknowledge_through(key(), 1, |_, _| Ok::<_, ()>(())),
        Err(AckError::Poisoned)
    );
    assert_eq!(ledger.outstanding_entries(), 1);
}

#[test]
fn wrong_epoch_duplicate_identity_and_sequence_exhaustion_do_not_mutate() {
    let wrong = PeerKey {
        connection_epoch: 8,
        ..key()
    };
    let mut ledger = OrdinaryLedger::new(key(), 0, 0).unwrap();
    let group = payload(1, &[], &[result(1)], &[event(1)]);
    assert_eq!(
        ledger.submit_state_group(wrong, &group, 0),
        Err(SubmitError::WrongIdentity)
    );
    assert_eq!(ledger.last_sequence(), 0);
    ledger.submit_state_group(key(), &group, 0).unwrap();
    let bytes = ledger.retained_frame_bytes();
    assert_eq!(
        ledger.submit_state_group(key(), &group, 0),
        Err(SubmitError::DuplicateOrInvalidIdentity)
    );
    assert_eq!(ledger.last_sequence(), 1);
    assert_eq!(ledger.retained_frame_bytes(), bytes);

    let mut exhausted = OrdinaryLedger::new(key(), u64::MAX, 0).unwrap();
    assert_eq!(
        exhausted.submit_state_group(key(), &group, 0),
        Err(SubmitError::SequenceExhausted)
    );
    assert_eq!(exhausted.retained_frame_bytes(), 0);
}
