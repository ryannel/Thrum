//! Game-owned I1 rule and authority/replica conformance fixture.

use thrum_core::{
    ActionRejection, ActorId, Admission, AdmissionError, AdmittedGroup, Authority, AuthorityId,
    ClockConfig, ClockState, CommittedEvent, EntityChange, EntityRecord, GameRules, HeadlessHost,
    HostAdmissionError, HostBudget, HostPublication, Input, InputSchedule, MatchId, Origin, PeerId,
    PublicationBytes, PublicationPool, Replica, ReplicaError, ResultDisposition, SessionId,
    StableId, StateGroup, StepError, TickClock, World, WorldEdit, WorldEpoch, WorldError,
};

const ACTOR: ActorId = ActorId(7);
const ACTOR_TWO: ActorId = ActorId(8);
const ACTOR_OBJECT: StableId = StableId(1);
const ACTOR_OBJECT_TWO: StableId = StableId(4);
const ITEM: StableId = StableId(2);
const DOOR: StableId = StableId(3);
const PEER: PeerId = PeerId(4);
const PEER_TWO: PeerId = PeerId(5);

#[derive(Clone, Debug, Eq, PartialEq)]
enum Object {
    Actor {
        owner: ActorId,
        at: [i32; 3],
        key: bool,
    },
    Collectible {
        at: [i32; 3],
        available: bool,
    },
    Door {
        at: [i32; 3],
        open: bool,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Action {
    Collect(StableId),
    Open(StableId),
    ChargedProbe(PublicationBytes),
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Rejection {
    WrongActor,
    TooFar,
    MissingItem,
    AlreadyTaken,
    NeedKey,
    AlreadyOpen,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Event {
    Collected(StableId),
    Opened(StableId),
}

enum Change {
    Collect(StableId),
    Open,
}

struct Rule;

fn nearby(a: [i32; 3], b: [i32; 3]) -> bool {
    a.into_iter()
        .zip(b)
        .map(|(x, y)| (x - y).pow(2))
        .sum::<i32>()
        <= 4
}

impl GameRules for Rule {
    type Entity = Object;
    type Action = Action;
    type Change = Change;
    type Rejection = Rejection;
    type Event = Event;

    fn decide(
        &self,
        world: &World<Object>,
        actor: ActorId,
        action: &Action,
    ) -> Result<Change, Rejection> {
        let actor_object = match actor {
            ACTOR => ACTOR_OBJECT,
            ACTOR_TWO => ACTOR_OBJECT_TWO,
            _ => return Err(Rejection::WrongActor),
        };
        let owned = world.handle_for(actor_object).and_then(|h| world.get(h));
        let Some(EntityRecord {
            state: Object::Actor { owner, at, key },
            ..
        }) = owned
        else {
            return Err(Rejection::WrongActor);
        };
        if *owner != actor {
            return Err(Rejection::WrongActor);
        }
        match action {
            Action::Collect(id) if *id == ITEM => {
                let item = world.handle_for(*id).and_then(|h| world.get(h));
                let Some(EntityRecord {
                    state:
                        Object::Collectible {
                            at: item_at,
                            available,
                        },
                    ..
                }) = item
                else {
                    return Err(Rejection::MissingItem);
                };
                if !available {
                    return Err(Rejection::AlreadyTaken);
                }
                if !nearby(*at, *item_at) {
                    return Err(Rejection::TooFar);
                }
                Ok(Change::Collect(actor_object))
            }
            Action::Open(id) if *id == DOOR => {
                let door = world.handle_for(*id).and_then(|h| world.get(h));
                let Some(EntityRecord {
                    state: Object::Door { at: door_at, open },
                    ..
                }) = door
                else {
                    return Err(Rejection::TooFar);
                };
                if !key {
                    return Err(Rejection::NeedKey);
                }
                if *open {
                    return Err(Rejection::AlreadyOpen);
                }
                if !nearby(*at, *door_at) {
                    return Err(Rejection::TooFar);
                }
                Ok(Change::Open)
            }
            _ => Err(Rejection::MissingItem),
        }
    }

    fn commit(&self, world: &mut World<Object>, change: Change) -> Option<Event> {
        match change {
            Change::Collect(actor_object) => {
                let item = world.handle_for(ITEM).expect("validated item");
                let actor = world.handle_for(actor_object).expect("validated actor");
                let Object::Collectible { available, .. } =
                    world.get_mut(item).expect("validated item")
                else {
                    unreachable!()
                };
                *available = false;
                let Object::Actor { key, .. } = world.get_mut(actor).expect("validated actor")
                else {
                    unreachable!()
                };
                *key = true;
                Some(Event::Collected(ITEM))
            }
            Change::Open => {
                let door = world.handle_for(DOOR).expect("validated door");
                let Object::Door { open, .. } = world.get_mut(door).expect("validated door") else {
                    unreachable!()
                };
                *open = true;
                Some(Event::Opened(DOOR))
            }
        }
    }
}

fn game_world(epoch: u128) -> World<Object> {
    let mut world = World::new(WorldEpoch(epoch));
    world
        .spawn(
            ACTOR_OBJECT,
            Object::Actor {
                owner: ACTOR,
                at: [0, 0, 0],
                key: false,
            },
        )
        .unwrap();
    world
        .spawn(
            ITEM,
            Object::Collectible {
                at: [1, 0, 0],
                available: true,
            },
        )
        .unwrap();
    world
        .spawn(
            DOOR,
            Object::Door {
                at: [2, 0, 0],
                open: false,
            },
        )
        .unwrap();
    world
}

fn two_actor_world(epoch: u128) -> World<Object> {
    let mut world = game_world(epoch);
    world
        .spawn(
            ACTOR_OBJECT_TWO,
            Object::Actor {
                owner: ACTOR_TWO,
                at: [0, 0, 0],
                key: false,
            },
        )
        .unwrap();
    world
}

fn input(epoch: u64, sequence: u64, schedule: InputSchedule, action: Action) -> Input<Action> {
    peer_input(PEER, ACTOR, epoch, sequence, schedule, action)
}

fn peer_input(
    peer: PeerId,
    actor: ActorId,
    epoch: u64,
    sequence: u64,
    schedule: InputSchedule,
    action: Action,
) -> Input<Action> {
    Input {
        origin: Origin {
            peer,
            connection_epoch: epoch,
            sequence,
        },
        actor,
        schedule,
        action,
    }
}

#[test]
fn game_rule_is_shared_by_local_and_match_authority() {
    for id in [
        AuthorityId::Standalone(SessionId(91)),
        AuthorityId::Match(MatchId(92)),
    ] {
        let mut authority = Authority::new(id, Rule, game_world(93));
        authority.register_peer(PEER, 1, ACTOR).unwrap();
        let (mut replica, _) = Replica::from_baseline(authority.baseline()).unwrap();
        assert_eq!(replica.len(), 3);
        let collect = input(
            1,
            1,
            InputSchedule::NextTick { observed_tick: 0 },
            Action::Collect(ITEM),
        );
        assert_eq!(
            authority.submit(collect.clone()).unwrap(),
            Admission::Queued
        );
        assert_eq!(
            authority.submit(collect.clone()).unwrap(),
            Admission::PendingDuplicate
        );
        let open = input(
            1,
            2,
            InputSchedule::NextTick { observed_tick: 0 },
            Action::Open(DOOR),
        );
        assert_eq!(authority.submit(open).unwrap(), Admission::Queued);
        let publication = authority.step().unwrap();
        assert_eq!(publication.group.results.len(), 2);
        assert_eq!(publication.group.events.len(), 2);
        assert_eq!(publication.group.events[0].id, 1);
        assert_eq!(publication.group.events[1].id, 2);
        assert_eq!(publication.world_work.copied_pages, 1);
        let admitted: AdmittedGroup<Event, Rejection> = replica.apply(publication.group).unwrap();
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
            authority.submit(collect).unwrap(),
            Admission::ResolvedDuplicate(_)
        ));
        assert_eq!(authority.step().unwrap().group.events.len(), 0);
    }
}

#[test]
fn admission_timing_gap_and_reconnect_do_not_duplicate_effects() {
    let mut authority = Authority::new(AuthorityId::Match(MatchId(1)), Rule, game_world(2));
    authority.register_peer(PEER, 10, ACTOR).unwrap();
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
        .unwrap();
    let first = authority.step().unwrap();
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
        .unwrap();
    authority
        .submit(input(
            10,
            3,
            InputSchedule::NextTick { observed_tick: 1 },
            Action::Open(DOOR),
        ))
        .unwrap();
    let second = authority.step().unwrap();
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
    let (mut resynced, baseline_outcomes) = Replica::from_baseline(baseline).unwrap();
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
    let third = authority.step().unwrap();
    resynced.apply(third.group).unwrap();
    let fourth = authority.step().unwrap();
    assert_eq!(fourth.group.results[0].origin.sequence, 2);
    assert_eq!(fourth.group.resolved_through, vec![(PEER, 3)]);
    assert_eq!(
        fourth.group.results[0].disposition,
        ResultDisposition::Rejected(ActionRejection::Game(Rejection::AlreadyOpen))
    );
    assert_eq!(
        resynced.apply(fourth.group).unwrap().resolved_through,
        vec![(PEER, 3)]
    );
    authority
        .submit(input(
            10,
            4,
            InputSchedule::Timed { intended_tick: 1 },
            Action::Collect(ITEM),
        ))
        .unwrap();
    let late = authority.step().unwrap();
    assert_eq!(
        late.group.results[0].disposition,
        ResultDisposition::Rejected(ActionRejection::Late)
    );
    assert!(late.group.events.is_empty());
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
        .unwrap();
    let rejected = authority.step().unwrap();
    assert_eq!(
        rejected.group.results[0].disposition,
        ResultDisposition::Rejected(ActionRejection::Game(Rejection::AlreadyTaken))
    );
    assert!(rejected.group.events.is_empty());
    for _ in 0..14 {
        authority.step().unwrap();
    }
    authority
        .submit(input(
            11,
            2,
            InputSchedule::NextTick { observed_tick: 0 },
            Action::Open(DOOR),
        ))
        .unwrap();
    let stale = authority.step().unwrap();
    assert_eq!(
        stale.group.results[0].disposition,
        ResultDisposition::Rejected(ActionRejection::StaleObservation)
    );
}

#[test]
fn immutable_snapshot_handle_and_atomic_replica_lifecycle() {
    let mut world = game_world(100);
    let old = world.handle_for(ITEM).unwrap();
    let before = world.publish(0).snapshot;
    world.despawn(old).unwrap();
    let replacement = world
        .spawn(
            StableId(5),
            Object::Collectible {
                at: [3, 0, 0],
                available: true,
            },
        )
        .unwrap();
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
    let (mut replica, _) = Replica::from_baseline(authority.baseline()).unwrap();
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
    replica.apply(good).unwrap();
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
fn retained_snapshots_backpressure_before_action_commit() {
    let mut authority =
        Authority::new(AuthorityId::Standalone(SessionId(30)), Rule, game_world(31));
    authority.register_peer(PEER, 1, ACTOR).unwrap();
    let mut held = Vec::new();
    for _ in 0..8 {
        held.push(authority.step().unwrap().snapshot);
    }
    assert_eq!(authority.tick_index(), 8);
    authority
        .submit(input(
            1,
            1,
            InputSchedule::NextTick { observed_tick: 8 },
            Action::Collect(ITEM),
        ))
        .unwrap();
    assert_eq!(
        authority.step().err(),
        Some(StepError::SnapshotBackpressure)
    );
    assert_eq!(authority.tick_index(), 8);
    assert!(matches!(
        authority
            .world()
            .get(authority.world().handle_for(ITEM).unwrap()),
        Some(EntityRecord {
            state: Object::Collectible {
                available: true,
                ..
            },
            ..
        })
    ));
    held.remove(0);
    let resumed = authority.step().unwrap();
    assert_eq!(
        resumed.group.results[0].disposition,
        ResultDisposition::Committed
    );
    assert_eq!(resumed.group.events.len(), 1);
}

#[test]
fn clock_retries_blocked_authority_without_double_action() {
    use std::time::Duration;

    let mut authority = Authority::new(AuthorityId::Match(MatchId(40)), Rule, game_world(41));
    authority.register_peer(PEER, 1, ACTOR).unwrap();
    let mut held = Vec::new();
    for _ in 0..8 {
        held.push(authority.step().unwrap().snapshot);
    }
    authority
        .submit(input(
            1,
            1,
            InputSchedule::NextTick { observed_tick: 8 },
            Action::Collect(ITEM),
        ))
        .unwrap();
    let mut clock = TickClock::new(ClockConfig::default(), Duration::ZERO).unwrap();
    let blocked = clock
        .advance(Duration::from_millis(17), 4, || {
            authority.step().map(|_| ())
        })
        .unwrap();
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
        .unwrap();
    assert_eq!(resumed.completed, 1);
    assert_eq!(resumed.queued, 0);
    assert_eq!(authority.tick_index(), 9);
    assert_eq!(groups[0].results.len(), 1);
    assert_eq!(groups[0].events.len(), 1);
}

#[test]
fn event_identity_and_commit_tick_must_match_its_result() {
    let mut authority = Authority::new(AuthorityId::Match(MatchId(50)), Rule, game_world(51));
    authority.register_peer(PEER, 1, ACTOR).unwrap();
    let (mut replica, _) = Replica::from_baseline(authority.baseline()).unwrap();
    authority
        .submit(input(
            1,
            1,
            InputSchedule::NextTick { observed_tick: 0 },
            Action::Collect(ITEM),
        ))
        .unwrap();
    let group = authority.step().unwrap().group;
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
    assert_eq!(replica.tick(), 0);
    replica.apply(group).unwrap();
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
}

#[test]
fn acknowledging_a_gap_removes_late_completion_behind_higher_sequence() {
    let mut authority = Authority::new(AuthorityId::Match(MatchId(60)), Rule, game_world(61));
    authority.register_peer(PEER, 1, ACTOR).unwrap();
    authority
        .submit(input(
            1,
            1,
            InputSchedule::Timed { intended_tick: 4 },
            Action::Open(DOOR),
        ))
        .unwrap();
    authority
        .submit(input(
            1,
            2,
            InputSchedule::NextTick { observed_tick: 0 },
            Action::Open(DOOR),
        ))
        .unwrap();
    for _ in 0..4 {
        authority.step().unwrap();
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
    authority.acknowledge_results(PEER, 1).unwrap();
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
    authority.acknowledge_results(PEER, 2).unwrap();
    assert!(authority.baseline().results.is_empty());
}

#[test]
fn dirty_page_accounting_resets_after_publication() {
    let mut world = game_world(70);
    let handle = world.handle_for(ITEM).unwrap();
    world.publish(0);
    world.get_mut(handle).unwrap();
    world.get_mut(handle).unwrap();
    assert_eq!(world.publish(1).work.copied_pages, 1);
    assert_eq!(world.publish(2).work.copied_pages, 0);
    world.get_mut(handle).unwrap();
    assert_eq!(world.publish(3).work.copied_pages, 1);
}

#[test]
fn growing_world_publishes_complete_new_pages_without_changing_old_snapshot() {
    let mut world = World::new(WorldEpoch(71));
    for id in 1..=70 {
        world.spawn(StableId(id), id).unwrap();
    }
    let first = world.publish(0);
    assert_eq!(first.work.copied_pages, 3);
    assert_eq!(first.snapshot.records().count(), 70);
    for id in 71..=103 {
        world.spawn(StableId(id), id).unwrap();
    }
    let second = world.publish(1);
    assert_eq!(second.work.copied_pages, 2);
    assert_eq!(second.snapshot.records().count(), 103);
    assert_eq!(first.snapshot.records().count(), 70);
    assert_eq!(
        first.snapshot.get(world.handle_for(StableId(103)).unwrap()),
        None
    );
}

#[test]
fn invalid_prepared_batch_leaves_every_object_and_publication_unchanged() {
    let mut world = World::new(WorldEpoch(72));
    let handle = world.spawn(StableId(1), 10_u64).unwrap();
    let before = world.publish(0).snapshot;
    assert_eq!(
        world.apply_prepared(vec![
            WorldEdit::Replace(handle, 11),
            WorldEdit::DespawnNew(StableId(2)),
        ]),
        Err(WorldError::InvalidHandle)
    );
    assert_eq!(world.get(handle).unwrap().state, 10);
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
    let stale = world.spawn(StableId(1), 10_u64).unwrap();
    world.publish(0);
    world.despawn(stale).unwrap();
    let current = world.spawn(StableId(2), 20).unwrap();
    assert_eq!(stale.slot, current.slot);
    assert_ne!(stale.generation, current.generation);
    assert_eq!(
        world.apply_prepared(vec![WorldEdit::Replace(stale, 99)]),
        Err(WorldError::InvalidHandle)
    );
    assert_eq!(world.get(current).unwrap().state, 20);
}

#[test]
fn prepared_batch_cannot_reuse_an_original_handle_after_respawn() {
    let mut world = World::new(WorldEpoch(75));
    let original = world.spawn(StableId(1), 10_u64).unwrap();
    assert_eq!(
        world.apply_prepared(vec![
            WorldEdit::Despawn(original),
            WorldEdit::Spawn(StableId(1), 20),
            WorldEdit::Replace(original, 30),
        ]),
        Err(WorldError::InvalidHandle)
    );
    assert_eq!(world.get(original).unwrap().state, 10);
    assert_eq!(world.publish(0).snapshot.get(original).unwrap().state, 10);
}

#[test]
fn single_prepared_edits_preserve_snapshot_and_handle_semantics() {
    let mut world = World::new(WorldEpoch(76));
    let first = world.spawn(StableId(1), 10_u64).unwrap();
    let prior = world.publish(0).snapshot;
    world.apply_one(WorldEdit::Replace(first, 11)).unwrap();
    assert_eq!(world.get(first).unwrap().state, 11);
    assert_eq!(prior.get(first).unwrap().state, 10);
    world.apply_one(WorldEdit::Despawn(first)).unwrap();
    world.apply_one(WorldEdit::Spawn(StableId(2), 20)).unwrap();
    let second = world.handle_for(StableId(2)).unwrap();
    assert_eq!(first.slot, second.slot);
    assert_ne!(first.generation, second.generation);
    assert_eq!(world.get(first), None);
    assert_eq!(world.get(second).unwrap().state, 20);
}

#[test]
fn prepared_structural_peak_survives_net_zero_lifecycle() {
    let mut world = World::new(WorldEpoch(73));
    let edits = (1..=65)
        .map(|id| WorldEdit::Spawn(StableId(id), id))
        .chain((1..=65).map(|id| WorldEdit::DespawnNew(StableId(id))))
        .collect();
    world.apply_prepared(edits).unwrap();
    assert!(world.is_empty());
    let published = world.publish(0);
    assert_eq!(published.work.copied_pages, 3);
    assert!(published.changes.is_empty());
    assert_eq!(published.snapshot.records().count(), 0);
}

#[test]
fn acknowledgements_release_bounded_result_capacity() {
    let mut authority = Authority::new(AuthorityId::Match(MatchId(80)), Rule, game_world(81));
    authority.register_peer(PEER, 1, ACTOR).unwrap();
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
            .unwrap();
        authority.step().unwrap();
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
    authority.acknowledge_results(PEER, 256).unwrap();
    assert_eq!(authority.submit(next), Ok(Admission::Queued));
}

#[test]
fn charged_action_bytes_follow_pending_and_completed_duplicate_history() {
    let charge = PublicationBytes::required_charge(1024).unwrap();
    let pool = PublicationPool::new(charge);
    let payload = PublicationBytes::try_copy_from_slice(&pool, &[7; 1024]).unwrap();
    let mut authority = Authority::new(AuthorityId::Match(MatchId(82)), Rule, game_world(83));
    authority.register_peer(PEER, 1, ACTOR).unwrap();
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

    let published = authority.step().unwrap();
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
    authority.acknowledge_results(PEER, 1).unwrap();
    assert_eq!(pool.used_bytes(), 0);

    let second = PublicationBytes::try_copy_from_slice(&pool, &[8; 1024]).unwrap();
    authority
        .submit(input(
            1,
            2,
            InputSchedule::NextTick { observed_tick: 1 },
            Action::ChargedProbe(second.clone()),
        ))
        .unwrap();
    drop(second);
    assert_eq!(pool.used_bytes(), charge);
    authority.register_peer(PEER, 2, ACTOR).unwrap();
    assert_eq!(pool.used_bytes(), 0);
}

#[test]
fn two_peers_share_rules_and_slow_replica_reaches_same_committed_state() {
    let mut authority = Authority::new(AuthorityId::Match(MatchId(90)), Rule, two_actor_world(91));
    authority.register_peer(PEER, 1, ACTOR).unwrap();
    authority.register_peer(PEER_TWO, 7, ACTOR_TWO).unwrap();
    let baseline = authority.baseline();
    let (mut fast, _) = Replica::from_baseline(baseline.clone()).unwrap();
    let (mut slow, _) = Replica::from_baseline(baseline).unwrap();
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
    authority.submit(first_input.clone()).unwrap();
    authority.submit(second_input.clone()).unwrap();
    let first = authority.step().unwrap().group;
    assert_eq!(first.results.len(), 2);
    assert_eq!(first.events.len(), 1);
    assert_eq!(first.results[0].disposition, ResultDisposition::Committed);
    assert_eq!(
        first.results[1].disposition,
        ResultDisposition::Rejected(ActionRejection::Game(Rejection::AlreadyTaken))
    );
    assert_eq!(first.resolved_through, vec![(PEER, 1), (PEER_TWO, 1)]);
    let mut delayed = vec![first.clone()];
    fast.apply(first).unwrap();
    assert_eq!(slow.tick(), 0);
    assert!(matches!(
        authority.submit(first_input).unwrap(),
        Admission::ResolvedDuplicate(_)
    ));
    assert!(matches!(
        authority.submit(second_input).unwrap(),
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
        .unwrap();
    authority
        .submit(peer_input(
            PEER_TWO,
            ACTOR_TWO,
            7,
            3,
            InputSchedule::NextTick { observed_tick: 1 },
            Action::Open(DOOR),
        ))
        .unwrap();
    for tick in 2..=4 {
        let group = authority.step().unwrap().group;
        if tick == 2 {
            assert_eq!(group.results[0].origin.sequence, 3);
            assert_eq!(group.resolved_through, vec![(PEER, 1), (PEER_TWO, 1)]);
        }
        if tick == 4 {
            assert_eq!(group.results[0].origin.sequence, 2);
            assert_eq!(group.resolved_through, vec![(PEER, 1), (PEER_TWO, 3)]);
        }
        fast.apply(group.clone()).unwrap();
        delayed.push(group);
    }

    authority
        .submit(input(
            1,
            2,
            InputSchedule::NextTick { observed_tick: 4 },
            Action::Open(DOOR),
        ))
        .unwrap();
    authority
        .submit(peer_input(
            PEER_TWO,
            ACTOR_TWO,
            7,
            4,
            InputSchedule::Timed { intended_tick: 1 },
            Action::Open(DOOR),
        ))
        .unwrap();
    let fifth = authority.step().unwrap().group;
    assert_eq!(fifth.events.len(), 1);
    assert_eq!(fifth.events[0].id, 2);
    assert_eq!(fifth.results[0].disposition, ResultDisposition::Committed);
    assert_eq!(
        fifth.results[1].disposition,
        ResultDisposition::Rejected(ActionRejection::Late)
    );
    assert_eq!(fifth.resolved_through, vec![(PEER, 2), (PEER_TWO, 4)]);
    fast.apply(fifth.clone()).unwrap();
    delayed.push(fifth);

    for group in delayed {
        slow.apply(group).unwrap();
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
    let (late_join, outcomes) = Replica::from_baseline(authority.baseline()).unwrap();
    assert_eq!(late_join.tick(), 5);
    assert_eq!(late_join.get(DOOR), slow.get(DOOR));
    assert_eq!(outcomes.resolved_through, vec![(PEER, 2), (PEER_TWO, 4)]);
    assert_eq!(outcomes.events.len(), 2);

    authority.register_peer(PEER_TWO, 8, ACTOR_TWO).unwrap();
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
    authority.register_peer(PEER, 1, ACTOR).unwrap();
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
    .unwrap();
    assert!(initial.results.is_empty());
    host.submit(input(
        1,
        1,
        InputSchedule::NextTick { observed_tick: 0 },
        Action::Collect(ITEM),
    ))
    .unwrap();
    let report = host
        .on_callback(origin + Duration::from_millis(17))
        .unwrap();
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
        .unwrap();
    assert_eq!(host.authority().tick_index(), 1);
    assert_eq!(host.replica().unwrap().tick(), 1);
    assert!(matches!(
        host.replica().unwrap().get(ITEM),
        Some(Object::Collectible {
            available: false,
            ..
        })
    ));
    assert!(host.authority().baseline().results.is_empty());
    let repeat = host
        .on_callback(origin + Duration::from_millis(17))
        .unwrap();
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
    let paused = host.on_callback(origin + Duration::from_secs(1)).unwrap();
    assert_eq!(paused.clock.completed, 0);
    host.resume(origin + Duration::from_secs(1) + Duration::from_millis(1))
        .unwrap();
    host.submit(input(
        1,
        2,
        InputSchedule::NextTick { observed_tick: 1 },
        Action::Open(DOOR),
    ))
    .unwrap();
    let after_resume = host
        .on_callback(origin + Duration::from_secs(1) + Duration::from_millis(18))
        .unwrap();
    assert_eq!(after_resume.clock.completed, 1);
    assert!(matches!(
        host.replica().unwrap().get(DOOR),
        Some(Object::Door { open: true, .. })
    ));
}

#[test]
fn slow_hosted_callback_reports_overload_and_stops_new_input() {
    use std::{
        thread,
        time::{Duration, Instant},
    };

    struct SlowRule;
    impl GameRules for SlowRule {
        type Entity = Object;
        type Action = Action;
        type Change = Change;
        type Rejection = Rejection;
        type Event = Event;

        fn decide(
            &self,
            world: &World<Object>,
            actor: ActorId,
            action: &Action,
        ) -> Result<Change, Rejection> {
            thread::sleep(Duration::from_millis(3));
            Rule.decide(world, actor, action)
        }
        fn commit(&self, world: &mut World<Object>, change: Change) -> Option<Event> {
            Rule.commit(world, change)
        }
    }

    let origin = Instant::now();
    let mut authority = Authority::new(AuthorityId::Match(MatchId(110)), SlowRule, game_world(111));
    authority.register_peer(PEER, 1, ACTOR).unwrap();
    let (mut host, initial) = HeadlessHost::with_replica(
        origin,
        ClockConfig::default(),
        HostBudget {
            callback_cpu: Duration::from_millis(10),
            other_work: Duration::from_millis(8),
            tick_cost_bound: Duration::from_millis(1),
        },
        authority,
    )
    .unwrap();
    assert!(initial.results.is_empty());
    host.submit(input(
        1,
        1,
        InputSchedule::NextTick { observed_tick: 0 },
        Action::Collect(ITEM),
    ))
    .unwrap();
    let report = host
        .on_callback(origin + Duration::from_millis(34))
        .unwrap();
    assert_eq!(report.clock.completed, 1);
    assert!(report.step_budget_overrun);
    assert_eq!(report.clock.state, ClockState::Overloaded);
    assert_eq!(report.clock.queued, 1);
    assert_eq!(host.authority().tick_index(), 1);
    assert_eq!(host.replica().unwrap().tick(), 1);
    let HostPublication::Replicated { admitted, .. } = &report.publications[0] else {
        panic!("expected admitted replica group")
    };
    assert_eq!(admitted.results.len(), 1);
    host.acknowledge_results(PEER, admitted.resolved_through[0].1)
        .unwrap();
    assert_eq!(
        host.submit(input(
            1,
            2,
            InputSchedule::NextTick { observed_tick: 1 },
            Action::Open(DOOR),
        )),
        Err(HostAdmissionError::Unavailable)
    );
    let drained = host
        .on_callback(origin + Duration::from_millis(35))
        .unwrap();
    assert_eq!(drained.clock.completed, 1);
    assert_eq!(drained.clock.queued, 0);
    host.resume(origin + Duration::from_millis(36)).unwrap();
    assert_eq!(host.clock_state(), ClockState::Active);
    assert_eq!(host.authority().baseline().event_cursor, 1);
}

#[test]
fn local_host_delivers_results_and_acknowledgement_restores_capacity() {
    use std::time::{Duration, Instant};

    let origin = Instant::now();
    let mut authority = Authority::new(
        AuthorityId::Standalone(SessionId(120)),
        Rule,
        game_world(121),
    );
    authority.register_peer(PEER, 1, ACTOR).unwrap();
    let mut host = HeadlessHost::local(
        origin,
        ClockConfig::default(),
        HostBudget {
            callback_cpu: Duration::from_millis(5),
            other_work: Duration::from_millis(1),
            tick_cost_bound: Duration::from_millis(1),
        },
        authority,
    )
    .unwrap();
    for sequence in 1..=256_u64 {
        let observed_tick = host.authority().tick_index();
        host.submit(input(
            1,
            sequence,
            InputSchedule::NextTick { observed_tick },
            Action::Open(DOOR),
        ))
        .unwrap();
        let report = host
            .on_callback(origin + Duration::from_millis(sequence * 20))
            .unwrap();
        let observed = report.publications.iter().any(|publication| {
            let HostPublication::Local(committed) = publication else {
                panic!("local host returned replica output")
            };
            committed
                .group
                .results
                .iter()
                .any(|result| result.origin.sequence == sequence)
        });
        assert!(observed);
    }
    let next = input(
        1,
        257,
        InputSchedule::NextTick {
            observed_tick: host.authority().tick_index(),
        },
        Action::Open(DOOR),
    );
    assert_eq!(
        host.submit(next.clone()),
        Err(HostAdmissionError::Authority(AdmissionError::Busy))
    );
    host.acknowledge_results(PEER, 256).unwrap();
    assert_eq!(host.submit(next), Ok(Admission::Queued));
}
