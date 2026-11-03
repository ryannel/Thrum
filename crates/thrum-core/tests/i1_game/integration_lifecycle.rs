//! Whole-engine authority, replica, and host lifecycle conformance.

use super::*;

#[test]
fn game_rule_is_shared_by_local_and_match_authority() {
    for id in [
        AuthorityId::Standalone(SessionId(91)),
        AuthorityId::Match(MatchId(92)),
    ] {
        let mut authority = Authority::new(id, Rule, game_world(93));
        authority
            .register_peer(PEER, 1, ACTOR)
            .expect("test setup or assertion requires a value");
        let (mut replica, _) = Replica::from_baseline(authority.baseline())
            .expect("test setup or assertion requires a value");
        assert_eq!(replica.len(), 3);
        let collect = input(
            1,
            1,
            InputSchedule::NextTick { observed_tick: 0 },
            Action::Collect(ITEM),
        );
        assert_eq!(
            authority
                .submit(collect.clone())
                .expect("test setup or assertion requires a value"),
            Admission::Queued
        );
        assert_eq!(
            authority
                .submit(collect.clone())
                .expect("test setup or assertion requires a value"),
            Admission::PendingDuplicate
        );
        let open = input(
            1,
            2,
            InputSchedule::NextTick { observed_tick: 0 },
            Action::Open(DOOR),
        );
        assert_eq!(
            authority
                .submit(open)
                .expect("test setup or assertion requires a value"),
            Admission::Queued
        );
        let publication = authority
            .step()
            .expect("test setup or assertion requires a value");
        assert_eq!(publication.group.results.len(), 2);
        assert_eq!(publication.group.events.len(), 2);
        assert_eq!(publication.group.events[0].id, 1);
        assert_eq!(publication.group.events[1].id, 2);
        assert_eq!(publication.world_work.copied_pages, 1);
        let admitted: AdmittedGroup<Event, Rejection> = replica
            .apply(publication.group)
            .expect("test setup or assertion requires a value");
        assert_eq!(admitted.resolved_through, vec![(PEER, 2)]);
        assert!(matches!(
            replica.get(ITEM),
            Some(Object::Collectible {
                available: false,
                ..
            })
        ));
        assert!(matches!(
            replica.get(DOOR),
            Some(Object::Door { open: true, .. })
        ));
        assert!(matches!(
            authority
                .submit(collect)
                .expect("test setup or assertion requires a value"),
            Admission::ResolvedDuplicate(_)
        ));
        assert_eq!(
            authority
                .step()
                .expect("test setup or assertion requires a value")
                .group
                .events
                .len(),
            0
        );
    }
}

#[test]
fn admission_timing_gap_and_reconnect_do_not_duplicate_effects() {
    let mut authority = Authority::new(AuthorityId::Match(MatchId(1)), Rule, game_world(2));
    authority
        .register_peer(PEER, 10, ACTOR)
        .expect("test setup or assertion requires a value");
    let mut forbidden = input(
        10,
        1,
        InputSchedule::NextTick { observed_tick: 0 },
        Action::Collect(ITEM),
    );
    forbidden.actor = ActorId(99);
    assert_eq!(
        authority.submit(forbidden),
        Err(AdmissionError::UnauthorizedActor)
    );
    assert_eq!(
        authority.submit(input(
            9,
            1,
            InputSchedule::NextTick { observed_tick: 0 },
            Action::Collect(ITEM)
        )),
        Err(AdmissionError::StaleEpoch)
    );
    authority
        .submit(input(
            10,
            1,
            InputSchedule::NextTick { observed_tick: 0 },
            Action::Collect(ITEM),
        ))
        .expect("test setup or assertion requires a value");
    let first = authority
        .step()
        .expect("test setup or assertion requires a value");
    assert_eq!(
        first.group.results[0].disposition,
        ResultDisposition::Committed
    );
    assert_eq!(first.group.events.len(), 1);
    assert_eq!(
        authority.register_peer(PEER, 10, ACTOR),
        Err(AdmissionError::StaleEpoch)
    );
    authority
        .submit(input(
            10,
            2,
            InputSchedule::Timed { intended_tick: 4 },
            Action::Open(DOOR),
        ))
        .expect("test setup or assertion requires a value");
    authority
        .submit(input(
            10,
            3,
            InputSchedule::NextTick { observed_tick: 1 },
            Action::Open(DOOR),
        ))
        .expect("test setup or assertion requires a value");
    let second = authority
        .step()
        .expect("test setup or assertion requires a value");
    assert_eq!(second.group.results.len(), 1);
    assert_eq!(second.group.results[0].origin.sequence, 3);
    assert_eq!(second.group.resolved_through, vec![(PEER, 1)]);
    assert_eq!(second.group.events[0].id, 2);
    let baseline = authority.baseline();
    let mut duplicate_result = baseline.clone();
    duplicate_result
        .results
        .push(duplicate_result.results[0].clone());
    assert!(matches!(
        Replica::from_baseline(duplicate_result),
        Err(ReplicaError::IncompleteOutcome)
    ));
    let mut balanced_duplicate = baseline.clone();
    balanced_duplicate.results[1] = balanced_duplicate.results[0].clone();
    assert_eq!(
        balanced_duplicate.results.len(),
        balanced_duplicate.events.len()
    );
    assert!(matches!(
        Replica::from_baseline(balanced_duplicate),
        Err(ReplicaError::IncompleteOutcome)
    ));
    let (mut resynced, baseline_outcomes) =
        Replica::from_baseline(baseline).expect("test setup or assertion requires a value");
    assert_eq!(resynced.tick(), 2);
    assert_eq!(baseline_outcomes.results.len(), 2);
    assert_eq!(baseline_outcomes.events.len(), 2);
    assert_eq!(baseline_outcomes.resolved_through, vec![(PEER, 1)]);
    assert!(
        baseline_outcomes
            .results
            .iter()
            .any(|r| r.origin.sequence == 3)
    );
    let third = authority
        .step()
        .expect("test setup or assertion requires a value");
    resynced
        .apply(third.group)
        .expect("test setup or assertion requires a value");
    let fourth = authority
        .step()
        .expect("test setup or assertion requires a value");
    assert_eq!(fourth.group.results[0].origin.sequence, 2);
    assert_eq!(fourth.group.resolved_through, vec![(PEER, 3)]);
    assert_eq!(
        fourth.group.results[0].disposition,
        ResultDisposition::Rejected(ActionRejection::Game(Rejection::AlreadyOpen))
    );
    assert_eq!(
        resynced
            .apply(fourth.group)
            .expect("test setup or assertion requires a value")
            .resolved_through,
        vec![(PEER, 3)]
    );
    authority
        .submit(input(
            10,
            4,
            InputSchedule::Timed { intended_tick: 1 },
            Action::Collect(ITEM),
        ))
        .expect("test setup or assertion requires a value");
    let late = authority
        .step()
        .expect("test setup or assertion requires a value");
    assert_eq!(
        late.group.results[0].disposition,
        ResultDisposition::Rejected(ActionRejection::Late)
    );
    assert_eq!(late.group.events, []);
    assert_eq!(authority.register_peer(PEER, 11, ACTOR), Ok(()));
    assert_eq!(
        authority.submit(input(
            10,
            5,
            InputSchedule::NextTick { observed_tick: 5 },
            Action::Open(DOOR)
        )),
        Err(AdmissionError::StaleEpoch)
    );
    authority
        .submit(input(
            11,
            1,
            InputSchedule::NextTick { observed_tick: 5 },
            Action::Collect(ITEM),
        ))
        .expect("test setup or assertion requires a value");
    let rejected = authority
        .step()
        .expect("test setup or assertion requires a value");
    assert_eq!(
        rejected.group.results[0].disposition,
        ResultDisposition::Rejected(ActionRejection::Game(Rejection::AlreadyTaken))
    );
    assert_eq!(rejected.group.events, []);
    for _ in 0..14 {
        authority
            .step()
            .expect("test setup or assertion requires a value");
    }
    authority
        .submit(input(
            11,
            2,
            InputSchedule::NextTick { observed_tick: 0 },
            Action::Open(DOOR),
        ))
        .expect("test setup or assertion requires a value");
    let stale = authority
        .step()
        .expect("test setup or assertion requires a value");
    assert_eq!(
        stale.group.results[0].disposition,
        ResultDisposition::Rejected(ActionRejection::StaleObservation)
    );
}

#[test]
fn immutable_snapshot_handle_and_atomic_replica_lifecycle() {
    let mut world = game_world(100);
    let old = world
        .handle_for(ITEM)
        .expect("test setup or assertion requires a value");
    let before = world.publish(0).snapshot;
    world
        .despawn(old)
        .expect("test setup or assertion requires a value");
    let replacement = world
        .spawn(
            StableId(5),
            Object::Collectible {
                at: [3, 0, 0],
                available: true,
            },
        )
        .expect("test setup or assertion requires a value");
    assert_eq!(old.slot, replacement.slot);
    assert_ne!(old.generation, replacement.generation);
    assert_eq!(world.get(old), None);
    let after = world.publish(1).snapshot;
    assert_eq!(after.get(old), None);
    assert!(before.get(old).is_some());
    assert_eq!(
        world.spawn(
            StableId(5),
            Object::Door {
                at: [0, 0, 0],
                open: false
            }
        ),
        Err(WorldError::DuplicateId)
    );
    assert_eq!(World::<Object>::new(WorldEpoch(101)).get(replacement), None);

    let authority = Authority::new(AuthorityId::Standalone(SessionId(8)), Rule, game_world(102));
    let (mut replica, _) = Replica::from_baseline(authority.baseline())
        .expect("test setup or assertion requires a value");
    let valid_spawn = EntityChange::Spawn(EntityRecord {
        id: StableId(8),
        state: Object::Door {
            at: [5, 0, 0],
            open: false,
        },
    });
    let bad: StateGroup<_, Event, Rejection> = StateGroup {
        authority: AuthorityId::Standalone(SessionId(8)),
        group_id: 1,
        commit_tick: 1,
        event_cursor: 0,
        changes: vec![
            valid_spawn.clone(),
            EntityChange::Update(EntityRecord {
                id: StableId(9),
                state: Object::Door {
                    at: [0, 0, 0],
                    open: true,
                },
            }),
        ],
        results: vec![],
        events: vec![],
        resolved_through: vec![],
    };
    assert!(matches!(
        replica.apply(bad),
        Err(ReplicaError::InvalidLifecycle)
    ));
    assert_eq!(replica.get(StableId(8)), None);
    assert_eq!(replica.tick(), 0);
    let good: StateGroup<_, Event, Rejection> = StateGroup {
        authority: AuthorityId::Standalone(SessionId(8)),
        group_id: 1,
        commit_tick: 1,
        event_cursor: 0,
        changes: vec![valid_spawn],
        results: vec![],
        events: vec![],
        resolved_through: vec![],
    };
    replica
        .apply(good)
        .expect("test setup or assertion requires a value");
    assert!(matches!(
        replica.get(StableId(8)),
        Some(Object::Door {
            at: [5, 0, 0],
            open: false
        })
    ));
    let false_watermark: StateGroup<Object, Event, Rejection> = StateGroup {
        authority: AuthorityId::Standalone(SessionId(8)),
        group_id: 2,
        commit_tick: 2,
        event_cursor: 0,
        changes: vec![],
        results: vec![],
        events: vec![],
        resolved_through: vec![(PEER, 1)],
    };
    assert_eq!(
        replica.apply(false_watermark).err(),
        Some(ReplicaError::IncompleteOutcome)
    );
    assert_eq!(replica.tick(), 1);
}

#[test]
fn changed_group_without_results_preserves_multipeer_progress_validation() {
    let mut authority = Authority::new(AuthorityId::Match(MatchId(29)), Rule, game_world(29));
    authority
        .register_peer(PEER, 1, ACTOR)
        .expect("test setup or assertion requires a value");
    authority
        .register_peer(PEER_TWO, 1, ACTOR_TWO)
        .expect("test setup or assertion requires a value");
    let (mut replica, _) = Replica::from_baseline(authority.baseline())
        .expect("test setup or assertion requires a value");
    let mut group = authority
        .step()
        .expect("test setup or assertion requires a value")
        .group;
    group.changes.push(EntityChange::Update(EntityRecord {
        id: DOOR,
        state: Object::Door {
            at: [0, 0, 0],
            open: true,
        },
    }));
    assert_eq!(group.resolved_through, vec![(PEER, 0), (PEER_TWO, 0)]);

    let mut bad = group.clone();
    bad.resolved_through[1].1 = 1;
    assert_eq!(
        replica.apply(bad).err(),
        Some(ReplicaError::IncompleteOutcome)
    );
    assert_eq!(replica.tick(), 0);
    assert!(matches!(
        replica.get(DOOR),
        Some(Object::Door { open: false, .. })
    ));

    replica
        .apply(group)
        .expect("test setup or assertion requires a value");
    assert_eq!(replica.tick(), 1);
    assert!(matches!(
        replica.get(DOOR),
        Some(Object::Door { open: true, .. })
    ));
}

#[test]
fn retained_snapshots_backpressure_before_action_commit() {
    let mut authority =
        Authority::new(AuthorityId::Standalone(SessionId(30)), Rule, game_world(31));
    authority
        .register_peer(PEER, 1, ACTOR)
        .expect("test setup or assertion requires a value");
    let mut held = Vec::new();
    for _ in 0..8 {
        held.push(
            authority
                .step()
                .expect("test setup or assertion requires a value")
                .snapshot,
        );
    }
    assert_eq!(authority.tick_index(), 8);
    authority
        .submit(input(
            1,
            1,
            InputSchedule::NextTick { observed_tick: 8 },
            Action::Collect(ITEM),
        ))
        .expect("test setup or assertion requires a value");
    assert_eq!(
        authority.step().err(),
        Some(StepError::SnapshotBackpressure)
    );
    assert_eq!(authority.tick_index(), 8);
    assert!(matches!(
        authority.world().get(
            authority
                .world()
                .handle_for(ITEM)
                .expect("test setup or assertion requires a value")
        ),
        Some(EntityRecord {
            state: Object::Collectible {
                available: true,
                ..
            },
            ..
        })
    ));
    held.remove(3);
    let resumed = authority
        .step()
        .expect("test setup or assertion requires a value");
    assert_eq!(
        resumed.group.results[0].disposition,
        ResultDisposition::Committed
    );
    assert_eq!(resumed.group.events.len(), 1);
    assert_eq!(
        authority.step().err(),
        Some(StepError::SnapshotBackpressure)
    );
}

#[test]
fn clock_retries_blocked_authority_without_double_action() {
    use std::time::Duration;

    let mut authority = Authority::new(AuthorityId::Match(MatchId(40)), Rule, game_world(41));
    authority
        .register_peer(PEER, 1, ACTOR)
        .expect("test setup or assertion requires a value");
    let mut held = Vec::new();
    for _ in 0..8 {
        held.push(
            authority
                .step()
                .expect("test setup or assertion requires a value")
                .snapshot,
        );
    }
    authority
        .submit(input(
            1,
            1,
            InputSchedule::NextTick { observed_tick: 8 },
            Action::Collect(ITEM),
        ))
        .expect("test setup or assertion requires a value");
    let mut clock = TickClock::new(ClockConfig::default(), Duration::ZERO)
        .expect("test setup or assertion requires a value");
    let blocked = clock
        .advance(Duration::from_millis(17), 4, || {
            authority.step().map(|_| ())
        })
        .expect("test setup or assertion requires a value");
    assert_eq!(blocked.step_error, Some(StepError::SnapshotBackpressure));
    assert_eq!(blocked.queued, 1);
    assert_eq!(authority.tick_index(), 8);
    held.remove(0);
    let mut groups = Vec::new();
    let resumed = clock
        .advance(Duration::from_millis(17), 4, || {
            groups.push(authority.step()?.group);
            Ok::<_, StepError>(())
        })
        .expect("test setup or assertion requires a value");
    assert_eq!(resumed.completed, 1);
    assert_eq!(resumed.queued, 0);
    assert_eq!(authority.tick_index(), 9);
    assert_eq!(groups[0].results.len(), 1);
    assert_eq!(groups[0].events.len(), 1);
}

#[test]
fn event_identity_and_commit_tick_must_match_its_result() {
    let mut authority = Authority::new(AuthorityId::Match(MatchId(50)), Rule, game_world(51));
    authority
        .register_peer(PEER, 1, ACTOR)
        .expect("test setup or assertion requires a value");
    let (mut replica, _) = Replica::from_baseline(authority.baseline())
        .expect("test setup or assertion requires a value");
    authority
        .submit(input(
            1,
            1,
            InputSchedule::NextTick { observed_tick: 0 },
            Action::Collect(ITEM),
        ))
        .expect("test setup or assertion requires a value");
    let group = authority
        .step()
        .expect("test setup or assertion requires a value")
        .group;
    let _: &CommittedEvent<Event> = &group.events[0];
    let mut wrong_origin = group.clone();
    wrong_origin.events[0].origin.sequence = 99;
    assert_eq!(
        replica.apply(wrong_origin).err(),
        Some(ReplicaError::IncompleteOutcome)
    );
    let mut wrong_tick = group.clone();
    wrong_tick.events[0].tick = 0;
    assert_eq!(
        replica.apply(wrong_tick).err(),
        Some(ReplicaError::IncompleteOutcome)
    );
    let mut orphan_event = group.clone();
    orphan_event.results[0].event_id = None;
    assert_eq!(
        replica.apply(orphan_event).err(),
        Some(ReplicaError::IncompleteOutcome)
    );
    let mut duplicate_reference = group.clone();
    duplicate_reference.results.push(group.results[0].clone());
    assert_eq!(
        replica.apply(duplicate_reference).err(),
        Some(ReplicaError::IncompleteOutcome)
    );
    assert_eq!(replica.tick(), 0);
    replica
        .apply(group)
        .expect("test setup or assertion requires a value");
    let mut baseline = authority.baseline();
    baseline.events[0].origin.sequence = 99;
    assert!(matches!(
        Replica::from_baseline(baseline),
        Err(ReplicaError::IncompleteOutcome)
    ));
    let mut baseline = authority.baseline();
    baseline.events[0].tick = 0;
    assert!(matches!(
        Replica::from_baseline(baseline),
        Err(ReplicaError::IncompleteOutcome)
    ));
    let mut baseline = authority.baseline();
    baseline.results[0].event_id = None;
    assert!(matches!(
        Replica::from_baseline(baseline),
        Err(ReplicaError::IncompleteOutcome)
    ));
}

#[test]
fn acknowledging_a_gap_removes_late_completion_behind_higher_sequence() {
    let mut authority = Authority::new(AuthorityId::Match(MatchId(60)), Rule, game_world(61));
    authority
        .register_peer(PEER, 1, ACTOR)
        .expect("test setup or assertion requires a value");
    authority
        .submit(input(
            1,
            1,
            InputSchedule::Timed { intended_tick: 4 },
            Action::Open(DOOR),
        ))
        .expect("test setup or assertion requires a value");
    authority
        .submit(input(
            1,
            2,
            InputSchedule::NextTick { observed_tick: 0 },
            Action::Open(DOOR),
        ))
        .expect("test setup or assertion requires a value");
    for _ in 0..4 {
        authority
            .step()
            .expect("test setup or assertion requires a value");
    }
    assert_eq!(
        authority
            .baseline()
            .results
            .iter()
            .map(|result| result.origin.sequence)
            .collect::<Vec<_>>(),
        vec![2, 1]
    );
    authority
        .acknowledge_results(PEER, 1)
        .expect("test setup or assertion requires a value");
    assert_eq!(
        authority
            .baseline()
            .results
            .iter()
            .map(|result| result.origin.sequence)
            .collect::<Vec<_>>(),
        vec![2]
    );
    assert_eq!(
        authority.submit(input(
            1,
            1,
            InputSchedule::Timed { intended_tick: 4 },
            Action::Open(DOOR),
        )),
        Err(AdmissionError::Expired)
    );
    authority
        .acknowledge_results(PEER, 2)
        .expect("test setup or assertion requires a value");
    assert_eq!(authority.baseline().results, []);
}

#[test]
fn dirty_page_accounting_resets_after_publication() {
    let mut world = game_world(70);
    let handle = world
        .handle_for(ITEM)
        .expect("test setup or assertion requires a value");
    world.publish(0);
    world
        .get_mut(handle)
        .expect("test setup or assertion requires a value");
    world
        .get_mut(handle)
        .expect("test setup or assertion requires a value");
    assert_eq!(world.publish(1).work.copied_pages, 1);
    assert_eq!(world.publish(2).work.copied_pages, 0);
    world
        .get_mut(handle)
        .expect("test setup or assertion requires a value");
    assert_eq!(world.publish(3).work.copied_pages, 1);
}

#[test]
fn growing_world_publishes_complete_new_pages_without_changing_old_snapshot() {
    let mut world = World::new(WorldEpoch(71));
    for id in 1..=70 {
        world
            .spawn(StableId(id), id)
            .expect("test setup or assertion requires a value");
    }
    let first = world.publish(0);
    assert_eq!(first.work.copied_pages, 3);
    assert_eq!(first.snapshot.records().count(), 70);
    for id in 71..=103 {
        world
            .spawn(StableId(id), id)
            .expect("test setup or assertion requires a value");
    }
    let second = world.publish(1);
    assert_eq!(second.work.copied_pages, 2);
    assert_eq!(second.snapshot.records().count(), 103);
    assert_eq!(first.snapshot.records().count(), 70);
    assert_eq!(
        first.snapshot.get(
            world
                .handle_for(StableId(103))
                .expect("test setup or assertion requires a value")
        ),
        None
    );
}

#[test]
fn invalid_prepared_batch_leaves_every_object_and_publication_unchanged() {
    let mut world = World::new(WorldEpoch(72));
    let handle = world
        .spawn(StableId(1), 10_u64)
        .expect("test setup or assertion requires a value");
    let before = world.publish(0).snapshot;
    assert_eq!(
        world.apply_prepared(vec![
            WorldEdit::Replace(handle, 11),
            WorldEdit::DespawnNew(StableId(2)),
        ]),
        Err(WorldError::InvalidHandle)
    );
    assert_eq!(
        world
            .get(handle)
            .expect("test setup or assertion requires a value")
            .state,
        10
    );
    assert_eq!(
        world.apply_prepared(vec![
            WorldEdit::Spawn(StableId(2), 20),
            WorldEdit::Spawn(StableId(2), 21),
        ]),
        Err(WorldError::DuplicateId)
    );
    assert_eq!(world.len(), 1);
    let after = world.publish(1);
    assert_eq!(after.work.copied_pages, 0);
    assert_eq!(before.get(handle), after.snapshot.get(handle));
}

#[test]
fn prepared_existing_edit_rejects_a_reused_slot_generation() {
    let mut world = World::new(WorldEpoch(74));
    let stale = world
        .spawn(StableId(1), 10_u64)
        .expect("test setup or assertion requires a value");
    world.publish(0);
    world
        .despawn(stale)
        .expect("test setup or assertion requires a value");
    let current = world
        .spawn(StableId(2), 20)
        .expect("test setup or assertion requires a value");
    assert_eq!(stale.slot, current.slot);
    assert_ne!(stale.generation, current.generation);
    assert_eq!(
        world.apply_prepared(vec![WorldEdit::Replace(stale, 99)]),
        Err(WorldError::InvalidHandle)
    );
    assert_eq!(
        world
            .get(current)
            .expect("test setup or assertion requires a value")
            .state,
        20
    );
}

#[test]
fn prepared_batch_cannot_reuse_an_original_handle_after_respawn() {
    let mut world = World::new(WorldEpoch(75));
    let original = world
        .spawn(StableId(1), 10_u64)
        .expect("test setup or assertion requires a value");
    assert_eq!(
        world.apply_prepared(vec![
            WorldEdit::Despawn(original),
            WorldEdit::Spawn(StableId(1), 20),
            WorldEdit::Replace(original, 30),
        ]),
        Err(WorldError::InvalidHandle)
    );
    assert_eq!(
        world
            .get(original)
            .expect("test setup or assertion requires a value")
            .state,
        10
    );
    assert_eq!(
        world
            .publish(0)
            .snapshot
            .get(original)
            .expect("test setup or assertion requires a value")
            .state,
        10
    );
}

#[test]
fn single_prepared_edits_preserve_snapshot_and_handle_semantics() {
    let mut world = World::new(WorldEpoch(76));
    let first = world
        .spawn(StableId(1), 10_u64)
        .expect("test setup or assertion requires a value");
    let prior = world.publish(0).snapshot;
    world
        .apply_one(WorldEdit::Replace(first, 11))
        .expect("test setup or assertion requires a value");
    assert_eq!(
        world
            .get(first)
            .expect("test setup or assertion requires a value")
            .state,
        11
    );
    assert_eq!(
        prior
            .get(first)
            .expect("test setup or assertion requires a value")
            .state,
        10
    );
    world
        .apply_one(WorldEdit::Despawn(first))
        .expect("test setup or assertion requires a value");
    world
        .apply_one(WorldEdit::Spawn(StableId(2), 20))
        .expect("test setup or assertion requires a value");
    let second = world
        .handle_for(StableId(2))
        .expect("test setup or assertion requires a value");
    assert_eq!(first.slot, second.slot);
    assert_ne!(first.generation, second.generation);
    assert_eq!(world.get(first), None);
    assert_eq!(
        world
            .get(second)
            .expect("test setup or assertion requires a value")
            .state,
        20
    );
}

#[test]
fn prepared_structural_peak_survives_net_zero_lifecycle() {
    let mut world = World::new(WorldEpoch(73));
    let edits = (1..=65)
        .map(|id| WorldEdit::Spawn(StableId(id), id))
        .chain((1..=65).map(|id| WorldEdit::DespawnNew(StableId(id))))
        .collect();
    world
        .apply_prepared(edits)
        .expect("test setup or assertion requires a value");
    assert!(world.is_empty());
    let published = world.publish(0);
    assert_eq!(published.work.copied_pages, 3);
    assert_eq!(published.changes, []);
    assert_eq!(published.snapshot.records().count(), 0);
}

#[test]
fn acknowledgements_release_bounded_result_capacity() {
    let mut authority = Authority::new(AuthorityId::Match(MatchId(80)), Rule, game_world(81));
    authority
        .register_peer(PEER, 1, ACTOR)
        .expect("test setup or assertion requires a value");
    for sequence in 1..=256 {
        authority
            .submit(input(
                1,
                sequence,
                InputSchedule::NextTick {
                    observed_tick: authority.tick_index(),
                },
                Action::Open(DOOR),
            ))
            .expect("test setup or assertion requires a value");
        authority
            .step()
            .expect("test setup or assertion requires a value");
    }
    let next = input(
        1,
        257,
        InputSchedule::NextTick {
            observed_tick: authority.tick_index(),
        },
        Action::Open(DOOR),
    );
    assert_eq!(authority.submit(next.clone()), Err(AdmissionError::Busy));
    authority
        .acknowledge_results(PEER, 256)
        .expect("test setup or assertion requires a value");
    assert_eq!(authority.submit(next), Ok(Admission::Queued));
}

#[test]
fn charged_action_bytes_follow_pending_and_completed_duplicate_history() {
    let charge =
        PublicationBytes::required_charge(1024).expect("test setup or assertion requires a value");
    let pool = PublicationPool::new(charge);
    let payload = PublicationBytes::try_copy_from_slice(&pool, &[7; 1024])
        .expect("test setup or assertion requires a value");
    let mut authority = Authority::new(AuthorityId::Match(MatchId(82)), Rule, game_world(83));
    authority
        .register_peer(PEER, 1, ACTOR)
        .expect("test setup or assertion requires a value");
    let action = input(
        1,
        1,
        InputSchedule::NextTick { observed_tick: 0 },
        Action::ChargedProbe(payload.clone()),
    );
    drop(payload);
    let pending_duplicate = action.clone();
    let resolved_duplicate = action.clone();
    assert_eq!(authority.submit(action), Ok(Admission::Queued));
    assert_eq!(
        authority.submit(pending_duplicate),
        Ok(Admission::PendingDuplicate)
    );
    assert_eq!(pool.used_bytes(), charge);

    let published = authority
        .step()
        .expect("test setup or assertion requires a value");
    assert_eq!(published.group.results.len(), 1);
    assert_eq!(
        published.group.results[0].disposition,
        ResultDisposition::Rejected(ActionRejection::Game(Rejection::MissingItem))
    );
    drop(published);
    assert!(matches!(
        authority.submit(resolved_duplicate),
        Ok(Admission::ResolvedDuplicate(_))
    ));
    assert_eq!(pool.used_bytes(), charge);
    authority
        .acknowledge_results(PEER, 1)
        .expect("test setup or assertion requires a value");
    assert_eq!(pool.used_bytes(), 0);

    let second = PublicationBytes::try_copy_from_slice(&pool, &[8; 1024])
        .expect("test setup or assertion requires a value");
    authority
        .submit(input(
            1,
            2,
            InputSchedule::NextTick { observed_tick: 1 },
            Action::ChargedProbe(second.clone()),
        ))
        .expect("test setup or assertion requires a value");
    drop(second);
    assert_eq!(pool.used_bytes(), charge);
    authority
        .register_peer(PEER, 2, ACTOR)
        .expect("test setup or assertion requires a value");
    assert_eq!(pool.used_bytes(), 0);
}

#[test]
fn two_peers_share_rules_and_slow_replica_reaches_same_committed_state() {
    let mut authority = Authority::new(AuthorityId::Match(MatchId(90)), Rule, two_actor_world(91));
    authority
        .register_peer(PEER, 1, ACTOR)
        .expect("test setup or assertion requires a value");
    authority
        .register_peer(PEER_TWO, 7, ACTOR_TWO)
        .expect("test setup or assertion requires a value");
    let baseline = authority.baseline();
    let (mut fast, _) =
        Replica::from_baseline(baseline.clone()).expect("test setup or assertion requires a value");
    let (mut slow, _) =
        Replica::from_baseline(baseline).expect("test setup or assertion requires a value");
    assert_eq!(fast.len(), 4);
    assert_eq!(slow.len(), 4);

    let forbidden = peer_input(
        PEER_TWO,
        ACTOR,
        7,
        1,
        InputSchedule::NextTick { observed_tick: 0 },
        Action::Collect(ITEM),
    );
    assert_eq!(
        authority.submit(forbidden),
        Err(AdmissionError::UnauthorizedActor)
    );
    let first_input = input(
        1,
        1,
        InputSchedule::NextTick { observed_tick: 0 },
        Action::Collect(ITEM),
    );
    let second_input = peer_input(
        PEER_TWO,
        ACTOR_TWO,
        7,
        1,
        InputSchedule::NextTick { observed_tick: 0 },
        Action::Collect(ITEM),
    );
    authority
        .submit(first_input.clone())
        .expect("test setup or assertion requires a value");
    authority
        .submit(second_input.clone())
        .expect("test setup or assertion requires a value");
    let first = authority
        .step()
        .expect("test setup or assertion requires a value")
        .group;
    assert_eq!(first.results.len(), 2);
    assert_eq!(first.events.len(), 1);
    assert_eq!(first.results[0].disposition, ResultDisposition::Committed);
    assert_eq!(
        first.results[1].disposition,
        ResultDisposition::Rejected(ActionRejection::Game(Rejection::AlreadyTaken))
    );
    assert_eq!(first.resolved_through, vec![(PEER, 1), (PEER_TWO, 1)]);
    let mut delayed = vec![first.clone()];
    fast.apply(first)
        .expect("test setup or assertion requires a value");
    assert_eq!(slow.tick(), 0);
    assert!(matches!(
        authority
            .submit(first_input)
            .expect("test setup or assertion requires a value"),
        Admission::ResolvedDuplicate(_)
    ));
    assert!(matches!(
        authority
            .submit(second_input)
            .expect("test setup or assertion requires a value"),
        Admission::ResolvedDuplicate(_)
    ));

    authority
        .submit(peer_input(
            PEER_TWO,
            ACTOR_TWO,
            7,
            2,
            InputSchedule::Timed { intended_tick: 4 },
            Action::Open(DOOR),
        ))
        .expect("test setup or assertion requires a value");
    authority
        .submit(peer_input(
            PEER_TWO,
            ACTOR_TWO,
            7,
            3,
            InputSchedule::NextTick { observed_tick: 1 },
            Action::Open(DOOR),
        ))
        .expect("test setup or assertion requires a value");
    for tick in 2..=4 {
        let group = authority
            .step()
            .expect("test setup or assertion requires a value")
            .group;
        if tick == 2 {
            assert_eq!(group.results[0].origin.sequence, 3);
            assert_eq!(group.resolved_through, vec![(PEER, 1), (PEER_TWO, 1)]);
        }
        if tick == 4 {
            assert_eq!(group.results[0].origin.sequence, 2);
            assert_eq!(group.resolved_through, vec![(PEER, 1), (PEER_TWO, 3)]);
        }
        fast.apply(group.clone())
            .expect("test setup or assertion requires a value");
        delayed.push(group);
    }

    authority
        .submit(input(
            1,
            2,
            InputSchedule::NextTick { observed_tick: 4 },
            Action::Open(DOOR),
        ))
        .expect("test setup or assertion requires a value");
    authority
        .submit(peer_input(
            PEER_TWO,
            ACTOR_TWO,
            7,
            4,
            InputSchedule::Timed { intended_tick: 1 },
            Action::Open(DOOR),
        ))
        .expect("test setup or assertion requires a value");
    let fifth = authority
        .step()
        .expect("test setup or assertion requires a value")
        .group;
    assert_eq!(fifth.events.len(), 1);
    assert_eq!(fifth.events[0].id, 2);
    assert_eq!(fifth.results[0].disposition, ResultDisposition::Committed);
    assert_eq!(
        fifth.results[1].disposition,
        ResultDisposition::Rejected(ActionRejection::Late)
    );
    assert_eq!(fifth.resolved_through, vec![(PEER, 2), (PEER_TWO, 4)]);
    fast.apply(fifth.clone())
        .expect("test setup or assertion requires a value");
    delayed.push(fifth);

    for group in delayed {
        slow.apply(group)
            .expect("test setup or assertion requires a value");
    }
    assert_eq!(slow.tick(), 5);
    for id in [ACTOR_OBJECT, ACTOR_OBJECT_TWO, ITEM, DOOR] {
        assert_eq!(fast.get(id), slow.get(id));
    }
    assert!(matches!(
        slow.get(ACTOR_OBJECT_TWO),
        Some(Object::Actor { key: false, .. })
    ));
    assert!(matches!(
        slow.get(DOOR),
        Some(Object::Door { open: true, .. })
    ));
    let (late_join, outcomes) = Replica::from_baseline(authority.baseline())
        .expect("test setup or assertion requires a value");
    assert_eq!(late_join.tick(), 5);
    assert_eq!(late_join.get(DOOR), slow.get(DOOR));
    assert_eq!(outcomes.resolved_through, vec![(PEER, 2), (PEER_TWO, 4)]);
    assert_eq!(outcomes.events.len(), 2);

    authority
        .register_peer(PEER_TWO, 8, ACTOR_TWO)
        .expect("test setup or assertion requires a value");
    assert_eq!(
        authority.submit(peer_input(
            PEER_TWO,
            ACTOR_TWO,
            7,
            5,
            InputSchedule::NextTick { observed_tick: 5 },
            Action::Open(DOOR),
        )),
        Err(AdmissionError::StaleEpoch)
    );
}

#[test]
fn hosted_callback_runs_authority_then_replica_once() {
    use std::time::{Duration, Instant};

    let origin = Instant::now();
    let mut authority = Authority::new(AuthorityId::Match(MatchId(100)), Rule, game_world(101));
    authority
        .register_peer(PEER, 1, ACTOR)
        .expect("test setup or assertion requires a value");
    let (mut host, initial) = HeadlessHost::with_replica(
        origin,
        ClockConfig::default(),
        HostBudget {
            callback_cpu: Duration::from_millis(5),
            other_work: Duration::from_millis(1),
            tick_cost_bound: Duration::from_millis(1),
        },
        authority,
    )
    .expect("test setup or assertion requires a value");
    assert_eq!(initial.results, []);
    host.submit(input(
        1,
        1,
        InputSchedule::NextTick { observed_tick: 0 },
        Action::Collect(ITEM),
    ))
    .expect("test setup or assertion requires a value");
    let report = host
        .on_callback(origin + Duration::from_millis(17))
        .expect("test setup or assertion requires a value");
    assert_eq!(report.clock.completed, 1);
    assert_eq!(report.replica_groups, 1);
    assert_eq!(report.world_work.copied_pages, 1);
    assert!(!report.step_budget_overrun);
    let HostPublication::Replicated { admitted, .. } = &report.publications[0] else {
        panic!("expected admitted replica group")
    };
    assert_eq!(admitted.results.len(), 1);
    assert_eq!(admitted.events.len(), 1);
    assert_eq!(host.authority().baseline().results.len(), 1);
    host.acknowledge_results(PEER, admitted.resolved_through[0].1)
        .expect("test setup or assertion requires a value");
    assert_eq!(host.authority().tick_index(), 1);
    assert_eq!(
        host.replica()
            .expect("test setup or assertion requires a value")
            .tick(),
        1
    );
    assert!(matches!(
        host.replica()
            .expect("test setup or assertion requires a value")
            .get(ITEM),
        Some(Object::Collectible {
            available: false,
            ..
        })
    ));
    assert_eq!(host.authority().baseline().results, []);
    let repeat = host
        .on_callback(origin + Duration::from_millis(17))
        .expect("test setup or assertion requires a value");
    assert_eq!(repeat.clock.completed, 0);
    assert_eq!(host.authority().tick_index(), 1);
    host.pause();
    assert_eq!(
        host.submit(input(
            1,
            2,
            InputSchedule::NextTick { observed_tick: 1 },
            Action::Open(DOOR),
        )),
        Err(HostAdmissionError::Unavailable)
    );
    let paused = host
        .on_callback(origin + Duration::from_secs(1))
        .expect("test setup or assertion requires a value");
    assert_eq!(paused.clock.completed, 0);
    host.resume(origin + Duration::from_secs(1) + Duration::from_millis(1))
        .expect("test setup or assertion requires a value");
    host.submit(input(
        1,
        2,
        InputSchedule::NextTick { observed_tick: 1 },
        Action::Open(DOOR),
    ))
    .expect("test setup or assertion requires a value");
    let after_resume = host
        .on_callback(origin + Duration::from_secs(1) + Duration::from_millis(18))
        .expect("test setup or assertion requires a value");
    assert_eq!(after_resume.clock.completed, 1);
    assert!(matches!(
        host.replica()
            .expect("test setup or assertion requires a value")
            .get(DOOR),
        Some(Object::Door { open: true, .. })
    ));
}
