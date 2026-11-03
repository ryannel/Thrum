//! One no-socket peer owns its queue and ordinary sent ledger together.

use thrum_network::envelope;
use thrum_network::outbound::{
    OutboundError, OutboundValidator, PartitionSeed, StateCoverage, TransferError,
};
use thrum_network::peer::{
    CaptureScratch, CaptureWriter, OrdinaryPeer, PeerError, PeerPhase, PlanError, ProjectError,
    ScratchError,
};
use thrum_network::r4::{self, ActionResult, Actor, GroupHeader, GroupInput};
use thrum_network::retirement::{PeerKey, RetirementError};
use thrum_network::sent::{AckError, SubmitError};

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

fn peer_for(identity: PeerKey) -> OrdinaryPeer {
    OrdinaryPeer::from_baseline(
        identity,
        &[PartitionSeed {
            group_id: 5,
            interest_generation: 1,
            sample_tick: 0,
            resolved_through: 0,
        }],
        0,
        0,
    )
    .expect("test operation should succeed")
}

fn peer() -> OrdinaryPeer {
    peer_for(key())
}

fn two_partition_peer_for(identity: PeerKey) -> OrdinaryPeer {
    OrdinaryPeer::from_baseline(
        identity,
        &[
            PartitionSeed {
                group_id: 5,
                interest_generation: 1,
                sample_tick: 0,
                resolved_through: 0,
            },
            PartitionSeed {
                group_id: 6,
                interest_generation: 1,
                sample_tick: 0,
                resolved_through: 0,
            },
        ],
        0,
        0,
    )
    .expect("test operation should succeed")
}

fn two_partition_peer() -> OrdinaryPeer {
    two_partition_peer_for(key())
}

fn four_partition_peer() -> OrdinaryPeer {
    let seeds: [_; 4] = std::array::from_fn(|index| PartitionSeed {
        group_id: u32::try_from(index).expect("bounded test value") + 5,
        interest_generation: 1,
        sample_tick: 0,
        resolved_through: 0,
    });
    OrdinaryPeer::from_baseline(key(), &seeds, 0, 0).expect("test operation should succeed")
}

fn push_group(
    writer: &mut CaptureWriter<'_>,
    group_id: u32,
    tick: u64,
) -> Result<(), ScratchError> {
    let actor = Actor {
        wire_id: u64::from(group_id),
        sector: [0; 3],
        offset: [0; 3],
        quaternion: [0, 0, 0, 32767],
        velocity: [0; 3],
        mode: 7,
        flags: 0,
        health: u16::try_from(tick).expect("bounded test value"),
        inventory_revision: 0,
        owner: 0,
        visibility_generation: 1,
    };
    writer.push(GroupInput {
        header: GroupHeader {
            group_id,
            interest_generation: 1,
            commit_tick: tick,
            resolved_through: 0,
        },
        actors: &[actor],
        lifecycle: &[],
        results: &[],
        events: &[],
    })
}

fn payload(tick: u64, result: bool) -> Vec<u8> {
    let actor = Actor {
        wire_id: 10,
        sector: [0; 3],
        offset: [0; 3],
        quaternion: [0, 0, 0, 32767],
        velocity: [0; 3],
        mode: 7,
        flags: 0,
        health: u16::try_from(tick).expect("bounded test value"),
        inventory_revision: 0,
        owner: 0,
        visibility_generation: 1,
    };
    let outcome = ActionResult {
        input_sequence: 1,
        resolved_tick: tick,
        first_event_id: 0,
        action_type: 1,
        reason: 0,
        event_count: 0,
        disposition: 0,
    };
    let outcomes = [outcome];
    let mut bytes = vec![0; r4::MAX_PAYLOAD_BYTES];
    let len = r4::encode(
        GroupInput {
            header: GroupHeader {
                group_id: 5,
                interest_generation: 1,
                commit_tick: tick,
                resolved_through: u64::from(result),
            },
            actors: &[actor],
            lifecycle: &[],
            results: if result { &outcomes } else { &[] },
            events: &[],
        },
        &mut bytes,
    )
    .expect("test operation should succeed");
    bytes.truncate(len);
    bytes
}

fn write_next(peer: &mut OrdinaryPeer, expected_tick: u64) {
    let frame = peer
        .next_write()
        .expect("test operation should succeed")
        .expect("test operation should succeed");
    let sequence = frame.sequence;
    let count = frame.first.len() + frame.second.len();
    let mut bytes = Vec::with_capacity(count);
    bytes.extend_from_slice(frame.first);
    bytes.extend_from_slice(frame.second);
    let message = envelope::decode(&bytes).expect("test operation should succeed");
    assert_eq!(message.header.message_sequence, sequence);
    assert_eq!(
        r4::decode(message.payload)
            .expect("test operation should succeed")
            .header
            .commit_tick,
        expected_tick
    );
    peer.accept_written(sequence, count)
        .expect("test operation should succeed");
}

#[test]
fn one_owner_coalesces_before_sequence_and_retries_exact_ack_receipt() {
    let mut peer = peer();
    let first = payload(1, false);
    let second = payload(2, false);
    let result = payload(3, true);
    peer.capture_batch(&[&first], &Schema)
        .expect("test operation should succeed");
    peer.capture_batch(&[&second], &Schema)
        .expect("test operation should succeed");
    peer.capture_batch(&[&result], &Schema)
        .expect("test operation should succeed");
    assert_eq!(peer.queued_groups(), 2);
    assert_eq!(peer.last_sequence(), 0);
    assert_eq!(peer.submit_next(0), Ok(Some(1)));
    write_next(&mut peer, 2);
    assert_eq!(peer.submit_next(0), Ok(Some(2)));
    write_next(&mut peer, 3);
    assert_eq!(peer.queued_groups(), 0);
    assert_eq!(peer.outstanding_frames(), 2);
    let stale = PeerKey {
        connection_epoch: 8,
        ..key()
    };
    assert_eq!(
        peer.acknowledge_through(stale, 2, |_, _| Ok::<_, ()>(())),
        Err(PeerError::Operation(AckError::WrongIdentity))
    );
    assert_eq!(
        peer.acknowledge_through(key(), 2, |_, _| Err::<(), _>("retry")),
        Err(PeerError::Operation(AckError::Retirement(
            RetirementError::Authority("retry")
        )))
    );
    assert_eq!(peer.outstanding_frames(), 2);
    assert_eq!(peer.result_retired_through(), 0);
    assert_eq!(peer.phase(), PeerPhase::Ordinary);
    assert_eq!(
        peer.acknowledge_through(key(), 2, |identity, through| {
            assert_eq!((identity, through), (key(), 1));
            Ok::<_, &str>(())
        }),
        Ok(2)
    );
    assert_eq!(peer.outstanding_frames(), 0);
    assert_eq!(peer.result_retired_through(), 1);
    assert!(peer.reserved_capacity_bytes() > 160 * 1024 + 16 * r4::MAX_PAYLOAD_BYTES);
}

#[test]
fn failed_capture_closes_only_this_peer_without_assigning_a_sequence() {
    let mut failed = peer();
    let mut healthy = peer_for(PeerKey {
        peer_id: 4,
        connection_epoch: 8,
        ..key()
    });
    let first = payload(1, true);
    failed
        .capture_batch(&[&first], &Schema)
        .expect("test operation should succeed");
    healthy
        .capture_batch(&[&first], &Schema)
        .expect("test operation should succeed");
    assert_eq!(
        failed.capture_batch(&[&first], &Schema),
        Err(PeerError::Operation(OutboundError::StaleTick))
    );
    assert_eq!(failed.phase(), PeerPhase::RequiresClose);
    assert_eq!(failed.queued_groups(), 1);
    assert_eq!(failed.last_sequence(), 0);
    assert_eq!(failed.submit_next(0), Err(PeerError::RequiresClose));
    assert_eq!(healthy.submit_next(0), Ok(Some(1)));
    write_next(&mut healthy, 1);
    assert_eq!(healthy.phase(), PeerPhase::Ordinary);
}

#[test]
fn caught_schema_hook_unwind_cannot_resume_publication() {
    struct PanicSchema;
    impl OutboundValidator for PanicSchema {
        fn group(&self, _: &r4::GroupView<'_>) -> Option<StateCoverage> {
            panic!("schema hook interrupted")
        }
    }

    let mut peer = peer();
    let first = payload(1, false);
    let second = payload(2, false);
    peer.capture_batch(&[&first], &Schema)
        .expect("test operation should succeed");
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = peer.capture_batch(&[&second], &PanicSchema);
    }));
    assert!(result.is_err());
    assert_eq!(peer.phase(), PeerPhase::RequiresClose);
    assert_eq!(peer.queued_groups(), 1);
    assert_eq!(peer.last_sequence(), 0);
    assert_eq!(peer.submit_next(0), Err(PeerError::RequiresClose));
}

#[test]
fn frame_pressure_retains_queued_unit_and_retries_after_admission() {
    let mut peer = peer();
    for tick in 1..=112 {
        let group = payload(tick, false);
        peer.capture_batch(&[&group], &Schema)
            .expect("test operation should succeed");
        assert_eq!(peer.submit_next(0), Ok(Some(tick)));
    }
    assert_eq!(peer.outstanding_frames(), 112);
    let waiting = payload(113, false);
    peer.capture_batch(&[&waiting], &Schema)
        .expect("test operation should succeed");
    assert_eq!(
        peer.submit_next(0),
        Err(PeerError::Operation(TransferError::Sink(
            SubmitError::FrameCapacity
        )))
    );
    assert_eq!(peer.phase(), PeerPhase::Ordinary);
    assert_eq!(peer.queued_groups(), 1);
    assert_eq!(peer.last_sequence(), 112);
    for tick in 1..=112 {
        write_next(&mut peer, tick);
    }
    assert_eq!(
        peer.acknowledge_through(key(), 112, |_, _| Ok::<_, ()>(())),
        Ok(112)
    );
    assert_eq!(peer.submit_next(0), Ok(Some(113)));
    write_next(&mut peer, 113);
    assert_eq!(peer.queued_groups(), 0);
}

#[test]
fn exact_reject_and_uncertain_write_require_close() {
    let mut rejected = peer();
    let group = payload(1, false);
    rejected
        .capture_batch(&[&group], &Schema)
        .expect("test operation should succeed");
    rejected
        .submit_next(0)
        .expect("test operation should succeed");
    write_next(&mut rejected, 1);
    rejected
        .reject_state_group(key(), 1)
        .expect("test operation should succeed");
    assert_eq!(rejected.phase(), PeerPhase::RequiresClose);
    assert_eq!(rejected.outstanding_frames(), 1);
    assert_eq!(rejected.submit_next(0), Err(PeerError::RequiresClose));

    let mut uncertain = peer();
    uncertain
        .capture_batch(&[&group], &Schema)
        .expect("test operation should succeed");
    uncertain
        .submit_next(0)
        .expect("test operation should succeed");
    uncertain.quarantine_uncertain_write();
    assert_eq!(uncertain.phase(), PeerPhase::RequiresClose);
    assert!(matches!(
        uncertain.next_write(),
        Err(PeerError::RequiresClose)
    ));
    assert_eq!(uncertain.outstanding_frames(), 1);
}

#[test]
fn caught_authority_ack_unwind_closes_the_peer_with_frame_retained() {
    let mut peer = peer();
    let result = payload(1, true);
    peer.capture_batch(&[&result], &Schema)
        .expect("test operation should succeed");
    peer.submit_next(0).expect("test operation should succeed");
    write_next(&mut peer, 1);
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = peer.acknowledge_through(key(), 1, |_, _| -> Result<(), ()> {
            panic!("authority ACK interrupted")
        });
    }));
    assert!(outcome.is_err());
    assert_eq!(peer.phase(), PeerPhase::RequiresClose);
    assert_eq!(peer.outstanding_frames(), 1);
    assert_eq!(peer.result_retired_through(), 0);
    assert_eq!(
        peer.acknowledge_through(key(), 1, |_, _| Ok::<_, ()>(())),
        Err(PeerError::RequiresClose)
    );
}

#[test]
fn projected_capture_requires_each_admitted_partition_at_one_tick() {
    let mut scratch = CaptureScratch::new();
    assert_eq!(
        scratch.reserved_capacity_bytes(),
        size_of::<CaptureScratch>() + 4 * r4::MAX_PAYLOAD_BYTES
    );
    let mut peer = two_partition_peer();
    peer.capture_projected(
        &mut scratch,
        |scratch| {
            push_group(scratch, 5, 1)?;
            push_group(scratch, 6, 1)
        },
        &Schema,
    )
    .expect("test operation should succeed");
    assert_eq!(peer.queued_groups(), 2);
    assert_eq!(peer.submit_next(0), Ok(Some(1)));
    write_next(&mut peer, 1);
    assert_eq!(peer.submit_next(0), Ok(Some(2)));
    write_next(&mut peer, 1);

    let mut other = two_partition_peer_for(PeerKey {
        peer_id: 4,
        connection_epoch: 8,
        ..key()
    });
    other
        .capture_projected(
            &mut scratch,
            |scratch| {
                push_group(scratch, 5, 1)?;
                push_group(scratch, 6, 1)
            },
            &Schema,
        )
        .expect("test operation should succeed");
    assert_eq!(other.queued_groups(), 2);

    let mut missing = two_partition_peer();
    assert_eq!(
        missing.capture_projected(&mut scratch, |scratch| push_group(scratch, 5, 1), &Schema),
        Err(ProjectError::Plan(PlanError::PartitionSet))
    );
    assert_eq!(missing.phase(), PeerPhase::RequiresClose);
    assert_eq!(missing.queued_groups(), 0);
    assert_eq!(missing.last_sequence(), 0);

    let mut duplicate = two_partition_peer();
    assert_eq!(
        duplicate.capture_projected(
            &mut scratch,
            |scratch| {
                push_group(scratch, 5, 1)?;
                push_group(scratch, 5, 1)
            },
            &Schema,
        ),
        Err(ProjectError::Plan(PlanError::PartitionSet))
    );
    assert_eq!(duplicate.queued_groups(), 0);

    let mut mismatch = two_partition_peer();
    assert_eq!(
        mismatch.capture_projected(
            &mut scratch,
            |scratch| {
                push_group(scratch, 5, 1)?;
                push_group(scratch, 6, 2)
            },
            &Schema,
        ),
        Err(ProjectError::Plan(PlanError::TickMismatch))
    );
    assert_eq!(mismatch.queued_groups(), 0);
}

#[test]
fn projected_capture_fault_and_unwind_close_without_partial_queue_mutation() {
    struct RejectSix;
    impl OutboundValidator for RejectSix {
        fn group(&self, group: &r4::GroupView<'_>) -> Option<StateCoverage> {
            (group.header.group_id != 6).then_some(StateCoverage::CompletePartition)
        }
    }
    let mut scratch = CaptureScratch::new();
    let mut refused = two_partition_peer();
    assert_eq!(
        refused.capture_projected(
            &mut scratch,
            |scratch| {
                push_group(scratch, 5, 1)?;
                Err(ScratchError::Capacity)
            },
            &Schema,
        ),
        Err(ProjectError::Projection(ScratchError::Capacity))
    );
    assert_eq!(refused.phase(), PeerPhase::RequiresClose);
    assert_eq!(refused.queued_groups(), 0);

    let mut panicked = two_partition_peer();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = panicked.capture_projected(
            &mut scratch,
            |scratch| -> Result<(), ScratchError> {
                push_group(scratch, 5, 1)?;
                panic!("game projection interrupted")
            },
            &Schema,
        );
    }));
    assert!(outcome.is_err());
    assert_eq!(panicked.phase(), PeerPhase::RequiresClose);
    assert_eq!(panicked.queued_groups(), 0);
    assert_eq!(panicked.last_sequence(), 0);

    let mut invalid_schema = two_partition_peer();
    assert_eq!(
        invalid_schema.capture_projected(
            &mut scratch,
            |scratch| {
                push_group(scratch, 5, 1)?;
                push_group(scratch, 6, 1)
            },
            &RejectSix,
        ),
        Err(ProjectError::Queue(OutboundError::Schema))
    );
    assert_eq!(invalid_schema.phase(), PeerPhase::RequiresClose);
    assert_eq!(invalid_schema.queued_groups(), 0);
    assert_eq!(invalid_schema.last_sequence(), 0);
}

#[test]
fn ignored_fifth_scratch_emission_cannot_publish_four_partial_groups() {
    let mut scratch = CaptureScratch::new();
    let mut peer = four_partition_peer();
    assert_eq!(
        peer.capture_projected(
            &mut scratch,
            |scratch| {
                for group_id in 5..=8 {
                    push_group(scratch, group_id, 1)?;
                }
                assert_eq!(push_group(scratch, 9, 1), Err(ScratchError::Capacity));
                Ok::<_, ScratchError>(())
            },
            &Schema,
        ),
        Err(ProjectError::Scratch(ScratchError::Capacity))
    );
    assert_eq!(peer.phase(), PeerPhase::RequiresClose);
    assert_eq!(peer.queued_groups(), 0);
    assert_eq!(peer.last_sequence(), 0);

    let mut healthy = four_partition_peer();
    healthy
        .capture_projected(
            &mut scratch,
            |scratch| {
                for group_id in 5..=8 {
                    push_group(scratch, group_id, 1)?;
                }
                Ok::<_, ScratchError>(())
            },
            &Schema,
        )
        .expect("test operation should succeed");
    assert_eq!(healthy.queued_groups(), 4);
}
