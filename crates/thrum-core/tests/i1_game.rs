//! Game-owned I1 rule and authority/replica conformance fixture.

use thrum_core::{
    ActionRejection, ActionResult, ActorId, Admission, AdmissionError, AdmittedGroup, Authority,
    AuthorityId, ChargedBaselineError, ChargedReplicaSetupError, ClockConfig, ClockState,
    ClosedGameLimits, ClosedValueLimits, CommittedEvent, EntityChange, EntityRecord, GameRules,
    Handle, HeadlessHost, HostAdmissionError, HostBudget, HostError, HostPublication, Input,
    InputSchedule, MatchId, Origin, PeerId, PreparedCommit, PreparedEdits, PublicationBytes,
    PublicationPool, PublicationRecord, Replica, ReplicaError, ResultDisposition, SessionId,
    SnapshotWriteBounds, StableId, StateGroup, StepError, TickClock, World, WorldEdit, WorldEpoch,
    WorldError, WorldPublicationError,
};

const ACTOR: ActorId = ActorId(7);
const ACTOR_TWO: ActorId = ActorId(8);
const ACTOR_OBJECT: StableId = StableId(1);
const ACTOR_OBJECT_TWO: StableId = StableId(4);
const ITEM: StableId = StableId(2);
const DOOR: StableId = StableId(3);
const PEER: PeerId = PeerId(4);
const PEER_TWO: PeerId = PeerId(5);

fn closed_game_limits(entity: ClosedValueLimits) -> ClosedGameLimits {
    let empty_output = ClosedValueLimits {
        max_inline_bytes: 0,
        max_payload_slots: 0,
        max_payload_bytes: 0,
    };
    ClosedGameLimits {
        action: ClosedValueLimits {
            max_inline_bytes: 16,
            max_payload_slots: 0,
            max_payload_bytes: 0,
        },
        entity,
        event: empty_output,
        rejection: empty_output,
    }
}

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
        _context: &mut thrum_core::RuleContext<'_>,
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

    fn prepare(
        &self,
        world: &World<Object>,
        change: Change,
        _context: &mut thrum_core::RuleContext<'_>,
    ) -> PreparedCommit<Object, Event> {
        match change {
            Change::Collect(actor_object) => {
                let item = world.handle_for(ITEM).expect("validated item");
                let actor = world.handle_for(actor_object).expect("validated actor");
                let mut item_state = world.get(item).expect("validated item").state.clone();
                let Object::Collectible { available, .. } = &mut item_state else {
                    unreachable!()
                };
                *available = false;
                let mut actor_state = world.get(actor).expect("validated actor").state.clone();
                let Object::Actor { key, .. } = &mut actor_state else {
                    unreachable!()
                };
                *key = true;
                PreparedCommit {
                    edits: PreparedEdits::Two(
                        WorldEdit::Replace(item, item_state),
                        WorldEdit::Replace(actor, actor_state),
                    ),
                    event: Some(Event::Collected(ITEM)),
                }
            }
            Change::Open => {
                let door = world.handle_for(DOOR).expect("validated door");
                let mut state = world.get(door).expect("validated door").state.clone();
                let Object::Door { open, .. } = &mut state else {
                    unreachable!()
                };
                *open = true;
                PreparedCommit {
                    edits: PreparedEdits::One(WorldEdit::Replace(door, state)),
                    event: Some(Event::Opened(DOOR)),
                }
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

struct InvalidSecondRule {
    prepares: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    panic_on_second: bool,
}

impl GameRules for InvalidSecondRule {
    type Entity = Object;
    type Action = Action;
    type Change = Action;
    type Rejection = Rejection;
    type Event = Event;

    fn decide(
        &self,
        _world: &World<Object>,
        _actor: ActorId,
        action: &Action,
        _context: &mut thrum_core::RuleContext<'_>,
    ) -> Result<Action, Rejection> {
        Ok(action.clone())
    }

    fn prepare(
        &self,
        world: &World<Object>,
        change: Action,
        _context: &mut thrum_core::RuleContext<'_>,
    ) -> PreparedCommit<Object, Event> {
        use std::sync::atomic::Ordering;
        self.prepares.fetch_add(1, Ordering::Relaxed);
        let door = world.handle_for(DOOR).unwrap();
        let mut state = world.get(door).unwrap().state.clone();
        let Object::Door { open, .. } = &mut state else {
            unreachable!()
        };
        match change {
            Action::Open(_) => {
                *open = true;
                PreparedCommit {
                    edits: PreparedEdits::One(WorldEdit::Replace(door, state)),
                    event: None,
                }
            }
            Action::Collect(_) => {
                if self.panic_on_second {
                    panic!("game preparation failed after an earlier same-tick edit");
                }
                *open = false;
                PreparedCommit {
                    edits: PreparedEdits::Two(
                        WorldEdit::Replace(door, state),
                        WorldEdit::DespawnNew(StableId(999)),
                    ),
                    event: None,
                }
            }
            Action::ChargedProbe(_) => unreachable!(),
        }
    }
}

fn submit_fault_pair<G: GameRules<Action = Action, Rejection = Rejection>>(
    authority: &mut Authority<G>,
) {
    authority.register_peer(PEER, 1, ACTOR).unwrap();
    for (sequence, action) in [(1, Action::Open(DOOR)), (2, Action::Collect(ITEM))] {
        authority
            .submit(input(
                1,
                sequence,
                InputSchedule::NextTick { observed_tick: 0 },
                action,
            ))
            .unwrap();
    }
}

#[test]
fn invalid_later_action_poisoned_authority_cannot_replay_or_expose_partial_tick() {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    let prepares = Arc::new(AtomicUsize::new(0));
    let mut authority = Authority::new(
        AuthorityId::Match(MatchId(180)),
        InvalidSecondRule {
            prepares: prepares.clone(),
            panic_on_second: false,
        },
        game_world(181),
    );
    submit_fault_pair(&mut authority);
    assert!(authority.try_baseline().is_ok());
    assert_eq!(
        authority.step().err(),
        Some(StepError::TerminalGameContract)
    );
    assert_eq!(prepares.load(Ordering::Relaxed), 2);
    assert!(authority.is_faulted());
    assert_eq!(authority.tick_index(), 0);
    assert!(matches!(
        authority
            .world()
            .get(authority.world().handle_for(DOOR).unwrap())
            .unwrap()
            .state,
        Object::Door { open: true, .. }
    ));
    assert!(matches!(
        authority.try_baseline(),
        Err(StepError::TerminalGameContract)
    ));
    assert_eq!(
        authority.step().err(),
        Some(StepError::TerminalGameContract)
    );
    assert_eq!(prepares.load(Ordering::Relaxed), 2);
    assert_eq!(
        authority.submit(input(
            1,
            3,
            InputSchedule::NextTick { observed_tick: 1 },
            Action::Open(DOOR),
        )),
        Err(AdmissionError::Faulted)
    );
    assert_eq!(
        authority.acknowledge_results(PEER, 1),
        Err(AdmissionError::Faulted)
    );
}

#[test]
fn caught_game_unwind_keeps_a_partly_processed_tick_terminal() {
    use std::{
        panic::{AssertUnwindSafe, catch_unwind},
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
    };

    let prepares = Arc::new(AtomicUsize::new(0));
    let mut authority = Authority::new(
        AuthorityId::Match(MatchId(186)),
        InvalidSecondRule {
            prepares: prepares.clone(),
            panic_on_second: true,
        },
        game_world(187),
    );
    submit_fault_pair(&mut authority);
    assert!(catch_unwind(AssertUnwindSafe(|| authority.step())).is_err());
    assert_eq!(prepares.load(Ordering::Relaxed), 2);
    assert!(authority.is_faulted());
    assert_eq!(
        authority.step().err(),
        Some(StepError::TerminalGameContract)
    );
    assert!(matches!(
        authority.try_baseline(),
        Err(StepError::TerminalGameContract)
    ));
    assert_eq!(prepares.load(Ordering::Relaxed), 2);
}

#[test]
fn hosted_terminal_rule_fault_stops_clock_and_keeps_replica_at_prior_tick() {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::{Duration, Instant},
    };

    let prepares = Arc::new(AtomicUsize::new(0));
    let mut authority = Authority::new(
        AuthorityId::Match(MatchId(182)),
        InvalidSecondRule {
            prepares: prepares.clone(),
            panic_on_second: false,
        },
        game_world(183),
    );
    submit_fault_pair(&mut authority);
    let origin = Instant::now();
    let (mut host, _) = HeadlessHost::with_replica(
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
    assert!(matches!(
        host.on_callback(origin + Duration::from_millis(17)),
        Err(HostError::Authority(StepError::TerminalGameContract))
    ));
    assert_eq!(host.replica().unwrap().tick(), 0);
    assert!(matches!(
        host.replica().unwrap().get(DOOR),
        Some(Object::Door { open: false, .. })
    ));
    assert_eq!(prepares.load(Ordering::Relaxed), 2);
    assert!(matches!(
        host.on_callback(origin + Duration::from_millis(34)),
        Err(HostError::Faulted)
    ));
    assert_eq!(prepares.load(Ordering::Relaxed), 2);
}

fn assert_catchup_fault_delivers(panic_on_second: bool) {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::{Duration, Instant},
    };

    let prepares = Arc::new(AtomicUsize::new(0));
    let mut authority = Authority::new(
        AuthorityId::Match(MatchId(184)),
        InvalidSecondRule {
            prepares: prepares.clone(),
            panic_on_second,
        },
        game_world(185),
    );
    authority.register_peer(PEER, 1, ACTOR).unwrap();
    authority
        .submit(input(
            1,
            1,
            InputSchedule::NextTick { observed_tick: 0 },
            Action::Open(DOOR),
        ))
        .unwrap();
    authority
        .submit(input(
            1,
            2,
            InputSchedule::Timed { intended_tick: 2 },
            Action::Collect(ITEM),
        ))
        .unwrap();
    let origin = Instant::now();
    let (mut host, _) = HeadlessHost::with_replica(
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
    let report = host
        .on_callback(origin + Duration::from_millis(34))
        .unwrap();
    assert_eq!(report.clock.completed, 1);
    assert_eq!(
        report.clock.step_error,
        Some(StepError::TerminalGameContract)
    );
    assert_eq!(
        report.terminal_fault,
        Some(HostError::Authority(StepError::TerminalGameContract))
    );
    assert_eq!(report.publications.len(), 1);
    let HostPublication::Replicated { admitted, .. } = &report.publications[0] else {
        panic!("expected earlier confirmed group")
    };
    assert_eq!(admitted.results.len(), 1);
    assert_eq!(admitted.results[0].origin.sequence, 1);
    assert_eq!(host.replica().unwrap().tick(), 1);
    assert_eq!(prepares.load(Ordering::Relaxed), 2);
    assert!(matches!(
        host.on_callback(origin + Duration::from_millis(51)),
        Err(HostError::Faulted)
    ));
    assert_eq!(prepares.load(Ordering::Relaxed), 2);
}

#[test]
fn callback_delivers_earlier_committed_tick_before_a_later_terminal_fault() {
    assert_catchup_fault_delivers(false);
}

#[test]
fn callback_delivers_earlier_committed_tick_before_a_caught_game_unwind() {
    assert_catchup_fault_delivers(true);
}

#[test]
fn charged_authority_backpressures_before_edit_and_releases_old_reader_bytes() {
    use std::time::{Duration, Instant};

    let pool = PublicationPool::new(1_000_000);
    let bounds = SnapshotWriteBounds {
        max_edits_per_action: 2,
        max_spawns_per_action: 0,
    };
    let other = Authority::try_new_snapshot_charged(
        AuthorityId::Match(MatchId(188)),
        Rule,
        game_world(189),
        pool.clone(),
        bounds,
    )
    .unwrap();
    let mut authority = Authority::try_new_snapshot_charged(
        AuthorityId::Match(MatchId(190)),
        Rule,
        game_world(191),
        pool.clone(),
        bounds,
    )
    .unwrap();
    assert!(pool.used_bytes() > 0);
    authority.register_peer(PEER, 1, ACTOR).unwrap();
    let old_reader = authority.snapshot().clone();
    let origin = Instant::now();
    let (mut host, _) = HeadlessHost::with_replica(
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
    host.submit(input(
        1,
        1,
        InputSchedule::NextTick { observed_tick: 0 },
        Action::Collect(ITEM),
    ))
    .unwrap();
    let occupied = pool.limit_bytes() - pool.used_bytes();
    let held = pool.try_reserve(occupied).unwrap();
    let blocked = host
        .on_callback(origin + Duration::from_millis(17))
        .unwrap();
    assert_eq!(
        blocked.clock.step_error,
        Some(StepError::PublicationBackpressure)
    );
    assert_eq!(blocked.clock.completed, 0);
    assert_eq!(host.authority().tick_index(), 0);
    assert_eq!(host.replica().unwrap().tick(), 0);
    assert!(matches!(
        host.authority()
            .world()
            .get(host.authority().world().handle_for(ITEM).unwrap())
            .unwrap()
            .state,
        Object::Collectible {
            available: true,
            ..
        }
    ));
    drop(held);
    let committed = host
        .on_callback(origin + Duration::from_millis(17))
        .unwrap();
    assert_eq!(committed.clock.completed, 1);
    assert_eq!(committed.replica_groups, 1);
    assert_eq!(host.authority().tick_index(), 1);
    assert_eq!(host.replica().unwrap().tick(), 1);
    let with_old_reader = pool.used_bytes();
    drop(old_reader);
    assert!(pool.used_bytes() < with_old_reader);
    let with_second_world = pool.used_bytes();
    drop(other);
    assert!(pool.used_bytes() < with_second_world);
}

#[test]
fn charged_lease_floor_preserves_idle_progress_and_last_reader_charge() {
    let pool = PublicationPool::new(100_000);
    let mut authority = Authority::try_new_snapshot_charged(
        AuthorityId::Match(MatchId(192)),
        Rule,
        World::new(WorldEpoch(192)),
        pool.clone(),
        SnapshotWriteBounds {
            max_edits_per_action: 1,
            max_spawns_per_action: 0,
        },
    )
    .unwrap();
    let activated = pool.used_bytes();
    assert!(activated > 0);
    let occupied = pool.try_reserve(pool.limit_bytes() - activated).unwrap();
    for tick in 1..=16 {
        let publication = authority.step().unwrap();
        assert_eq!(publication.snapshot.tick(), tick);
        assert!(publication.group.changes.is_empty());
        assert_eq!(pool.used_bytes(), pool.limit_bytes());
    }
    drop(occupied);
    let reader = authority.snapshot().clone();
    drop(authority);
    assert!(pool.used_bytes() > 0);
    drop(reader);
    assert_eq!(pool.used_bytes(), 0);
}

#[test]
fn charged_peer_registry_keeps_order_and_failed_join_invisible() {
    let pool = PublicationPool::new(100_000);
    let mut authority = Authority::try_new_snapshot_charged(
        AuthorityId::Match(MatchId(251)),
        Rule,
        World::new(WorldEpoch(251)),
        pool.clone(),
        SnapshotWriteBounds {
            max_edits_per_action: 0,
            max_spawns_per_action: 0,
        },
    )
    .unwrap();
    authority.register_peer(PeerId(9), 1, ACTOR).unwrap();
    authority.register_peer(PeerId(3), 1, ACTOR_TWO).unwrap();
    assert_eq!(
        authority.baseline().resolved_through,
        [(PeerId(3), 0), (PeerId(9), 0)]
    );
    let occupied = pool
        .try_reserve(pool.limit_bytes() - pool.used_bytes())
        .unwrap();
    assert_eq!(
        authority.register_peer(PeerId(5), 1, ACTOR),
        Err(AdmissionError::PublicationCapacity)
    );
    assert_eq!(pool.used_bytes(), pool.limit_bytes());
    assert_eq!(
        authority.baseline().resolved_through,
        [(PeerId(3), 0), (PeerId(9), 0)]
    );
    authority.register_peer(PeerId(9), 2, ACTOR).unwrap();
    drop(occupied);
    authority.register_peer(PeerId(5), 1, ACTOR).unwrap();
    assert_eq!(
        authority.baseline().resolved_through,
        [(PeerId(3), 0), (PeerId(5), 0), (PeerId(9), 0)]
    );
    drop(authority);
    assert_eq!(pool.used_bytes(), 0);
}

#[test]
fn charged_pending_storage_reserves_before_peer_join_and_survives_reconnect() {
    let pool = PublicationPool::new(100_000);
    let mut authority = Authority::try_new_snapshot_charged(
        AuthorityId::Match(MatchId(193)),
        Rule,
        World::new(WorldEpoch(193)),
        pool.clone(),
        SnapshotWriteBounds {
            max_edits_per_action: 1,
            max_spawns_per_action: 0,
        },
    )
    .unwrap();
    let activated = pool.used_bytes();
    let occupied = pool.try_reserve(pool.limit_bytes() - activated).unwrap();
    assert_eq!(
        authority.register_peer(PEER, 1, ACTOR),
        Err(AdmissionError::PublicationCapacity)
    );
    assert_eq!(pool.used_bytes(), pool.limit_bytes());
    drop(occupied);

    authority.register_peer(PEER, 1, ACTOR).unwrap();
    let with_peer = pool.used_bytes();
    assert!(with_peer > activated);
    let tight_pool = PublicationPool::new(with_peer - 1);
    let mut tight_authority = Authority::try_new_snapshot_charged(
        AuthorityId::Match(MatchId(194)),
        Rule,
        World::new(WorldEpoch(194)),
        tight_pool.clone(),
        SnapshotWriteBounds {
            max_edits_per_action: 1,
            max_spawns_per_action: 0,
        },
    )
    .unwrap();
    assert_eq!(tight_pool.used_bytes(), activated);
    assert_eq!(
        tight_authority.register_peer(PEER, 1, ACTOR),
        Err(AdmissionError::PublicationCapacity)
    );
    assert_eq!(tight_pool.used_bytes(), activated);
    drop(tight_authority);
    assert_eq!(tight_pool.used_bytes(), 0);
    authority
        .submit(input(
            1,
            1,
            InputSchedule::NextTick { observed_tick: 0 },
            Action::Collect(ITEM),
        ))
        .unwrap();
    let occupied = pool
        .try_reserve(pool.limit_bytes() - pool.used_bytes())
        .unwrap();
    assert_eq!(
        authority.submit(input(
            1,
            2,
            InputSchedule::NextTick { observed_tick: 0 },
            Action::Collect(ITEM),
        )),
        Err(AdmissionError::PublicationCapacity)
    );
    authority.register_peer(PEER, 2, ACTOR).unwrap();
    assert_eq!(pool.used_bytes(), pool.limit_bytes());
    assert_eq!(
        authority.submit(input(
            1,
            1,
            InputSchedule::NextTick { observed_tick: 0 },
            Action::Collect(ITEM),
        )),
        Err(AdmissionError::StaleEpoch)
    );
    authority
        .submit(input(
            2,
            1,
            InputSchedule::NextTick { observed_tick: 0 },
            Action::Collect(ITEM),
        ))
        .unwrap();
    drop(occupied);
    drop(authority);
    assert_eq!(pool.used_bytes(), 0);
}

#[test]
fn charged_retry_allows_a_new_peer_before_replaying_due_inputs() {
    let pool = PublicationPool::new(1_000_000);
    let mut authority = Authority::try_new_snapshot_charged(
        AuthorityId::Match(MatchId(195)),
        Rule,
        game_world(195),
        pool.clone(),
        SnapshotWriteBounds {
            max_edits_per_action: 2,
            max_spawns_per_action: 0,
        },
    )
    .unwrap();
    authority.register_peer(PEER, 1, ACTOR).unwrap();
    authority
        .submit(input(
            1,
            1,
            InputSchedule::NextTick { observed_tick: 0 },
            Action::Collect(ITEM),
        ))
        .unwrap();
    let snapshot_peak = authority
        .world()
        .conservative_charged_publication_peak(2, 0)
        .unwrap();
    let available = snapshot_peak - 1;
    let occupied = pool
        .try_reserve(pool.limit_bytes() - pool.used_bytes() - available)
        .unwrap();
    assert_eq!(
        authority.step().err(),
        Some(StepError::PublicationBackpressure)
    );
    assert_eq!(authority.tick_index(), 0);
    authority.register_peer(PEER_TWO, 1, ACTOR_TWO).unwrap();
    drop(occupied);
    let committed = authority.step().unwrap();
    assert_eq!(committed.group.results.len(), 1);
    assert_eq!(committed.group.results[0].origin.peer, PEER);
    assert_eq!(
        committed.group.resolved_through,
        vec![(PEER, 1), (PEER_TWO, 0)]
    );
    assert_eq!(authority.tick_index(), 1);
}

#[test]
fn charged_due_scratch_growth_reserves_old_and_new_buffers_together() {
    let bounds = SnapshotWriteBounds {
        max_edits_per_action: 1,
        max_spawns_per_action: 0,
    };
    let probe_pool = PublicationPool::new(100_000);
    let mut probe = Authority::try_new_snapshot_charged(
        AuthorityId::Match(MatchId(196)),
        Rule,
        World::new(WorldEpoch(196)),
        probe_pool.clone(),
        bounds,
    )
    .unwrap();
    probe.register_peer(PEER, 1, ACTOR).unwrap();
    let first_peer_bytes = probe_pool.used_bytes();
    probe.register_peer(PEER_TWO, 1, ACTOR_TWO).unwrap();
    let final_two_peer_bytes = probe_pool.used_bytes();
    drop(probe);
    assert_eq!(probe_pool.used_bytes(), 0);

    let tight_pool = PublicationPool::new(final_two_peer_bytes);
    let mut authority = Authority::try_new_snapshot_charged(
        AuthorityId::Match(MatchId(197)),
        Rule,
        World::new(WorldEpoch(197)),
        tight_pool.clone(),
        bounds,
    )
    .unwrap();
    authority.register_peer(PEER, 1, ACTOR).unwrap();
    assert_eq!(tight_pool.used_bytes(), first_peer_bytes);
    assert_eq!(
        authority.register_peer(PEER_TWO, 1, ACTOR_TWO),
        Err(AdmissionError::PublicationCapacity)
    );
    assert_eq!(tight_pool.used_bytes(), first_peer_bytes);
    drop(authority);
    assert_eq!(tight_pool.used_bytes(), 0);
}

#[test]
fn charged_authority_faults_if_prepared_edits_exceed_declared_count() {
    let pool = PublicationPool::new(1_000_000);
    let mut authority = Authority::try_new_snapshot_charged(
        AuthorityId::Match(MatchId(192)),
        Rule,
        game_world(193),
        pool.clone(),
        SnapshotWriteBounds {
            max_edits_per_action: 1,
            max_spawns_per_action: 0,
        },
    )
    .unwrap();
    authority.register_peer(PEER, 1, ACTOR).unwrap();
    authority
        .submit(input(
            1,
            1,
            InputSchedule::NextTick { observed_tick: 0 },
            Action::Collect(ITEM),
        ))
        .unwrap();
    let before = pool.used_bytes();
    assert_eq!(
        authority.step().err(),
        Some(StepError::TerminalGameContract)
    );
    assert!(authority.is_faulted());
    assert_eq!(authority.tick_index(), 0);
    assert_eq!(pool.used_bytes(), before);
    assert!(matches!(
        authority
            .world()
            .get(authority.world().handle_for(ITEM).unwrap())
            .unwrap()
            .state,
        Object::Collectible {
            available: true,
            ..
        }
    ));
}

#[test]
fn charged_authority_reserves_transient_structural_peak() {
    struct StructuralRule;
    impl GameRules for StructuralRule {
        type Entity = u64;
        type Action = ();
        type Change = ();
        type Rejection = ();
        type Event = ();

        fn decide(
            &self,
            _world: &World<u64>,
            _actor: ActorId,
            _action: &(),
            _context: &mut thrum_core::RuleContext<'_>,
        ) -> Result<(), ()> {
            Ok(())
        }

        fn prepare(
            &self,
            _world: &World<u64>,
            _change: (),
            _context: &mut thrum_core::RuleContext<'_>,
        ) -> PreparedCommit<u64, ()> {
            let edits = (1..=65)
                .map(|id| WorldEdit::Spawn(StableId(id), id as u64))
                .chain((1..=65).map(|id| WorldEdit::DespawnNew(StableId(id))))
                .collect();
            PreparedCommit {
                edits: PreparedEdits::Many(edits),
                event: None,
            }
        }
    }

    let pool = PublicationPool::new(1_000_000);
    let mut authority = Authority::try_new_snapshot_charged(
        AuthorityId::Match(MatchId(194)),
        StructuralRule,
        World::new(WorldEpoch(195)),
        pool.clone(),
        SnapshotWriteBounds {
            max_edits_per_action: 130,
            max_spawns_per_action: 65,
        },
    )
    .unwrap();
    authority.register_peer(PEER, 1, ACTOR).unwrap();
    authority
        .submit(Input {
            origin: Origin {
                peer: PEER,
                connection_epoch: 1,
                sequence: 1,
            },
            actor: ACTOR,
            schedule: InputSchedule::NextTick { observed_tick: 0 },
            action: (),
        })
        .unwrap();
    let published = authority.step().unwrap();
    assert_eq!(published.world_work.copied_pages, 3);
    assert!(published.group.changes.is_empty());
    assert!(authority.world().is_empty());
    assert!(published.snapshot.records().next().is_none());
    assert!(pool.used_bytes() > 0);
}

#[test]
fn charged_authority_preserves_sequential_rules_and_competing_peers() {
    let pool = PublicationPool::new(1_000_000);
    let bounds = SnapshotWriteBounds {
        max_edits_per_action: 2,
        max_spawns_per_action: 0,
    };
    let mut sequential = Authority::try_new_snapshot_charged(
        AuthorityId::Match(MatchId(196)),
        Rule,
        game_world(197),
        pool.clone(),
        bounds,
    )
    .unwrap();
    sequential.register_peer(PEER, 1, ACTOR).unwrap();
    let (mut sequential_replica, _) = Replica::from_baseline(sequential.baseline()).unwrap();
    for (sequence, action) in [(1, Action::Collect(ITEM)), (2, Action::Open(DOOR))] {
        sequential
            .submit(input(
                1,
                sequence,
                InputSchedule::NextTick { observed_tick: 0 },
                action,
            ))
            .unwrap();
    }
    let group = sequential.step().unwrap().group;
    assert_eq!(group.results.len(), 2);
    assert!(
        group
            .results
            .iter()
            .all(|result| result.disposition == ResultDisposition::Committed)
    );
    sequential_replica.apply(group).unwrap();
    assert!(matches!(
        sequential_replica.get(DOOR),
        Some(Object::Door { open: true, .. })
    ));

    let mut competing = Authority::try_new_snapshot_charged(
        AuthorityId::Match(MatchId(198)),
        Rule,
        two_actor_world(199),
        pool.clone(),
        bounds,
    )
    .unwrap();
    competing.register_peer(PEER, 1, ACTOR).unwrap();
    competing.register_peer(PEER_TWO, 1, ACTOR_TWO).unwrap();
    let (mut competing_replica, _) = Replica::from_baseline(competing.baseline()).unwrap();
    for (peer, actor) in [(PEER, ACTOR), (PEER_TWO, ACTOR_TWO)] {
        competing
            .submit(peer_input(
                peer,
                actor,
                1,
                1,
                InputSchedule::NextTick { observed_tick: 0 },
                Action::Collect(ITEM),
            ))
            .unwrap();
    }
    let group = competing.step().unwrap().group;
    assert_eq!(group.results.len(), 2);
    assert_eq!(group.results[0].disposition, ResultDisposition::Committed);
    assert_eq!(
        group.results[1].disposition,
        ResultDisposition::Rejected(ActionRejection::Game(Rejection::AlreadyTaken))
    );
    competing_replica.apply(group).unwrap();
    assert!(matches!(
        competing_replica.get(ITEM),
        Some(Object::Collectible {
            available: false,
            ..
        })
    ));
}

#[test]
fn idle_replica_fast_path_keeps_peer_progress_validation() {
    let mut authority =
        Authority::new(AuthorityId::Match(MatchId(200)), Rule, two_actor_world(201));
    authority.register_peer(PEER, 1, ACTOR).unwrap();
    authority.register_peer(PEER_TWO, 1, ACTOR_TWO).unwrap();
    let (mut replica, _) = Replica::from_baseline(authority.baseline()).unwrap();

    replica.apply(authority.step().unwrap().group).unwrap();
    assert_eq!(replica.tick(), 1);
    let mut reordered = authority.step().unwrap().group;
    reordered.resolved_through.reverse();
    replica.apply(reordered).unwrap();
    assert_eq!(replica.tick(), 2);

    let mut duplicate = authority.step().unwrap().group;
    duplicate.resolved_through[1] = duplicate.resolved_through[0];
    assert!(matches!(
        replica.apply(duplicate),
        Err(ReplicaError::IncompleteOutcome)
    ));
    assert_eq!(replica.tick(), 2);
    let valid: StateGroup<Object, Event, Rejection> = StateGroup {
        authority: AuthorityId::Match(MatchId(200)),
        group_id: 3,
        commit_tick: 3,
        event_cursor: 0,
        changes: vec![],
        results: vec![],
        events: vec![],
        resolved_through: vec![(PEER, 0), (PEER_TWO, 0)],
    };
    replica.apply(valid).unwrap();
    assert_eq!(replica.tick(), 3);
}

#[test]
fn idle_replica_does_not_skip_gap_normalization() {
    let mut authority = Authority::new(AuthorityId::Match(MatchId(202)), Rule, game_world(203));
    authority.register_peer(PEER, 1, ACTOR).unwrap();
    let mut baseline = authority.baseline();
    baseline.results.push(ActionResult {
        origin: Origin {
            peer: PEER,
            connection_epoch: 1,
            sequence: 1,
        },
        resolved_tick: 0,
        disposition: ResultDisposition::Committed,
        event_id: None,
    });
    let (mut replica, _) = Replica::from_baseline(baseline).unwrap();
    let stale = authority.step().unwrap().group;
    let mut corrected = stale.clone();
    corrected.resolved_through[0].1 = 1;
    assert!(matches!(
        replica.apply(stale),
        Err(ReplicaError::IncompleteOutcome)
    ));
    assert_eq!(replica.tick(), 0);
    replica.apply(corrected).unwrap();
    assert_eq!(replica.tick(), 1);
}

#[test]
fn closed_record_payload_survives_snapshot_and_replica_until_final_owner() {
    type Record = PublicationRecord<8, 1>;

    struct RecordRule {
        pool: PublicationPool,
    }
    impl GameRules for RecordRule {
        type Entity = Record;
        type Action = StableId;
        type Change = Handle;
        type Rejection = ();
        type Event = ();

        fn decide(
            &self,
            world: &World<Record>,
            _actor: ActorId,
            action: &StableId,
            _context: &mut thrum_core::RuleContext<'_>,
        ) -> Result<Handle, ()> {
            world.handle_for(*action).ok_or(())
        }

        fn prepare(
            &self,
            world: &World<Record>,
            handle: Handle,
            _context: &mut thrum_core::RuleContext<'_>,
        ) -> PreparedCommit<Record, ()> {
            let old = &world.get(handle).unwrap().state;
            let next = u64::from_le_bytes(*old.inline()) + 1;
            let state = Record::try_new(
                &self.pool,
                next.to_le_bytes(),
                [old.payload_owner(0).cloned()],
            )
            .unwrap();
            PreparedCommit {
                edits: PreparedEdits::One(WorldEdit::Replace(handle, state)),
                event: None,
            }
        }
    }

    let pool = PublicationPool::new(100_000);
    let payload_charge = PublicationBytes::required_charge(512).unwrap();
    let payload = PublicationBytes::try_copy_from_slice(&pool, &[9; 512]).unwrap();
    let state = Record::try_new(&pool, 4_u64.to_le_bytes(), [Some(payload.clone())]).unwrap();
    let mut world = World::new(WorldEpoch(205));
    world.spawn(StableId(1), state).unwrap();
    let mut authority = Authority::try_new_closed_snapshot_charged(
        AuthorityId::Match(MatchId(204)),
        RecordRule { pool: pool.clone() },
        world,
        pool.clone(),
        SnapshotWriteBounds {
            max_edits_per_action: 1,
            max_spawns_per_action: 0,
        },
        closed_game_limits(ClosedValueLimits {
            max_inline_bytes: 8,
            max_payload_slots: 1,
            max_payload_bytes: 512,
        }),
    )
    .unwrap();
    authority.register_peer(PEER, 1, ACTOR).unwrap();
    let (mut replica, _) = Replica::from_baseline(authority.baseline()).unwrap();
    let old_reader = authority.snapshot().clone();
    drop(payload);
    authority
        .submit(Input {
            origin: Origin {
                peer: PEER,
                connection_epoch: 1,
                sequence: 1,
            },
            actor: ACTOR,
            schedule: InputSchedule::NextTick { observed_tick: 0 },
            action: StableId(1),
        })
        .unwrap();
    let published = authority.step().unwrap();
    let admitted = replica.apply(published.group).unwrap();
    assert_eq!(admitted.results.len(), 1);
    assert_eq!(
        u64::from_le_bytes(*replica.get(StableId(1)).unwrap().inline()),
        5
    );
    assert_eq!(
        replica
            .get(StableId(1))
            .unwrap()
            .payload_owner(0)
            .unwrap()
            .as_slice(),
        &[9; 512]
    );
    assert_eq!(
        u64::from_le_bytes(*old_reader.records().next().unwrap().state.inline()),
        4
    );
    drop(admitted);
    drop(published.snapshot);
    drop(old_reader);
    drop(authority);
    assert_eq!(pool.used_bytes(), payload_charge);
    drop(replica);
    assert_eq!(pool.used_bytes(), 0);
}

#[test]
fn closed_authority_rejects_foreign_initial_and_prepared_record_owners() {
    type Record = PublicationRecord<8, 1>;

    struct ForeignEditRule {
        foreign: PublicationPool,
    }
    impl GameRules for ForeignEditRule {
        type Entity = Record;
        type Action = StableId;
        type Change = Handle;
        type Rejection = ();
        type Event = ();

        fn decide(
            &self,
            world: &World<Record>,
            _actor: ActorId,
            action: &StableId,
            _context: &mut thrum_core::RuleContext<'_>,
        ) -> Result<Handle, ()> {
            world.handle_for(*action).ok_or(())
        }

        fn prepare(
            &self,
            _world: &World<Record>,
            handle: Handle,
            _context: &mut thrum_core::RuleContext<'_>,
        ) -> PreparedCommit<Record, ()> {
            let foreign_payload =
                PublicationBytes::try_copy_from_slice(&self.foreign, &[9; 32]).unwrap();
            PreparedCommit {
                edits: PreparedEdits::Two(
                    WorldEdit::Replace(
                        handle,
                        Record::try_new(&self.foreign, 2_u64.to_le_bytes(), [None]).unwrap(),
                    ),
                    WorldEdit::Spawn(
                        StableId(2),
                        Record::try_new(
                            &self.foreign,
                            3_u64.to_le_bytes(),
                            [Some(foreign_payload)],
                        )
                        .unwrap(),
                    ),
                ),
                event: None,
            }
        }
    }

    let pool = PublicationPool::new(100_000);
    let foreign = PublicationPool::new(100_000);
    let foreign_payload = PublicationBytes::try_copy_from_slice(&foreign, &[7; 32]).unwrap();
    let foreign_record =
        Record::try_new(&foreign, 1_u64.to_le_bytes(), [Some(foreign_payload)]).unwrap();
    let mut wrong_world = World::new(WorldEpoch(207));
    wrong_world.spawn(StableId(1), foreign_record).unwrap();
    let bounds = SnapshotWriteBounds {
        max_edits_per_action: 2,
        max_spawns_per_action: 1,
    };
    let value_limits = closed_game_limits(ClosedValueLimits {
        max_inline_bytes: 8,
        max_payload_slots: 1,
        max_payload_bytes: 32,
    });
    assert!(matches!(
        Authority::try_new_closed_snapshot_charged(
            AuthorityId::Match(MatchId(206)),
            ForeignEditRule {
                foreign: foreign.clone(),
            },
            wrong_world,
            pool.clone(),
            bounds,
            value_limits,
        ),
        Err(WorldPublicationError::WrongPool)
    ));
    assert_eq!(pool.used_bytes(), 0);

    let oversized_payload = PublicationBytes::try_copy_from_slice(&pool, &[8; 33]).unwrap();
    let oversized = Record::try_new(&pool, 1_u64.to_le_bytes(), [Some(oversized_payload)]).unwrap();
    let mut oversized_world = World::new(WorldEpoch(209));
    oversized_world.spawn(StableId(1), oversized).unwrap();
    assert!(matches!(
        Authority::try_new_closed_snapshot_charged(
            AuthorityId::Match(MatchId(209)),
            ForeignEditRule {
                foreign: foreign.clone(),
            },
            oversized_world,
            pool.clone(),
            bounds,
            value_limits,
        ),
        Err(WorldPublicationError::ValueProfile)
    ));
    assert_eq!(pool.used_bytes(), 0);

    let mut world = World::new(WorldEpoch(208));
    let initial = Record::try_new(&pool, 1_u64.to_le_bytes(), [None]).unwrap();
    let handle = world.spawn(StableId(1), initial).unwrap();
    let mut authority = Authority::try_new_closed_snapshot_charged(
        AuthorityId::Match(MatchId(208)),
        ForeignEditRule { foreign },
        world,
        pool,
        bounds,
        value_limits,
    )
    .unwrap();
    authority.register_peer(PEER, 1, ACTOR).unwrap();
    authority
        .submit(Input {
            origin: Origin {
                peer: PEER,
                connection_epoch: 1,
                sequence: 1,
            },
            actor: ACTOR,
            schedule: InputSchedule::NextTick { observed_tick: 0 },
            action: StableId(1),
        })
        .unwrap();
    assert!(matches!(
        authority.step(),
        Err(StepError::TerminalGameContract)
    ));
    assert_eq!(authority.tick_index(), 0);
    assert_eq!(
        u64::from_le_bytes(*authority.snapshot().get(handle).unwrap().state.inline()),
        1
    );
    assert_eq!(
        u64::from_le_bytes(*authority.world().get(handle).unwrap().state.inline()),
        1
    );
    assert!(authority.world().handle_for(StableId(2)).is_none());
    assert!(matches!(
        authority.step(),
        Err(StepError::TerminalGameContract)
    ));

    let profile_pool = PublicationPool::new(100_000);
    let mut profile_world = World::new(WorldEpoch(210));
    let initial = Record::try_new(&profile_pool, 1_u64.to_le_bytes(), [None]).unwrap();
    let handle = profile_world.spawn(StableId(1), initial).unwrap();
    let mut profile_authority = Authority::try_new_closed_snapshot_charged(
        AuthorityId::Match(MatchId(210)),
        ForeignEditRule {
            foreign: profile_pool.clone(),
        },
        profile_world,
        profile_pool,
        bounds,
        ClosedGameLimits {
            entity: ClosedValueLimits {
                max_payload_bytes: 31,
                ..value_limits.entity
            },
            ..value_limits
        },
    )
    .unwrap();
    profile_authority.register_peer(PEER, 1, ACTOR).unwrap();
    profile_authority
        .submit(Input {
            origin: Origin {
                peer: PEER,
                connection_epoch: 1,
                sequence: 1,
            },
            actor: ACTOR,
            schedule: InputSchedule::NextTick { observed_tick: 0 },
            action: StableId(1),
        })
        .unwrap();
    assert!(matches!(
        profile_authority.step(),
        Err(StepError::TerminalGameContract)
    ));
    assert_eq!(
        u64::from_le_bytes(
            *profile_authority
                .world()
                .get(handle)
                .unwrap()
                .state
                .inline()
        ),
        1
    );
    assert!(profile_authority.world().handle_for(StableId(2)).is_none());
}

#[test]
fn closed_authority_rejects_oversized_record_type_in_empty_world() {
    struct NoopRule;
    impl GameRules for NoopRule {
        type Entity = PublicationRecord<16, 2>;
        type Action = ();
        type Change = ();
        type Rejection = ();
        type Event = ();

        fn decide(
            &self,
            _world: &World<Self::Entity>,
            _actor: ActorId,
            _action: &(),
            _context: &mut thrum_core::RuleContext<'_>,
        ) -> Result<(), ()> {
            Ok(())
        }

        fn prepare(
            &self,
            _world: &World<Self::Entity>,
            _change: (),
            _context: &mut thrum_core::RuleContext<'_>,
        ) -> PreparedCommit<Self::Entity, ()> {
            PreparedCommit {
                edits: PreparedEdits::None,
                event: None,
            }
        }
    }

    let pool = PublicationPool::new(100_000);
    assert!(matches!(
        Authority::try_new_closed_snapshot_charged(
            AuthorityId::Match(MatchId(211)),
            NoopRule,
            World::new(WorldEpoch(211)),
            pool.clone(),
            SnapshotWriteBounds {
                max_edits_per_action: 0,
                max_spawns_per_action: 0,
            },
            closed_game_limits(ClosedValueLimits {
                max_inline_bytes: 8,
                max_payload_slots: 1,
                max_payload_bytes: 32,
            }),
        ),
        Err(WorldPublicationError::ValueProfile)
    ));
    assert_eq!(pool.used_bytes(), 0);
}

mod closed_output_fixture {
    use super::*;

    type Entity = PublicationRecord<8, 0>;
    type Output = PublicationRecord<0, 1>;

    fn output(pool: &PublicationPool) -> Output {
        let payload = PublicationBytes::try_copy_from_slice(pool, &[8; 32]).unwrap();
        Output::try_new(pool, [], [Some(payload)]).unwrap()
    }

    #[test]
    fn sealed_host_delivers_two_peer_charged_group_once() {
        use std::time::{Duration, Instant};

        let pool = PublicationPool::new(100_000);
        let mut world = World::new(WorldEpoch(235));
        world
            .spawn(
                StableId(1),
                Entity::try_new(&pool, 1_u64.to_le_bytes(), []).unwrap(),
            )
            .unwrap();
        let output_limits = ClosedValueLimits {
            max_inline_bytes: 0,
            max_payload_slots: 1,
            max_payload_bytes: 32,
        };
        let mut authority = Authority::try_new_closed_snapshot_charged(
            AuthorityId::Match(MatchId(235)),
            OutputRule {
                entity_pool: pool.clone(),
                output_pool: pool.clone(),
            },
            world,
            pool.clone(),
            SnapshotWriteBounds {
                max_edits_per_action: 1,
                max_spawns_per_action: 0,
            },
            ClosedGameLimits {
                action: ClosedValueLimits {
                    max_inline_bytes: 1,
                    max_payload_slots: 0,
                    max_payload_bytes: 0,
                },
                entity: ClosedValueLimits {
                    max_inline_bytes: 8,
                    max_payload_slots: 0,
                    max_payload_bytes: 0,
                },
                event: output_limits,
                rejection: output_limits,
            },
        )
        .unwrap();
        authority.register_peer(PEER, 1, ACTOR).unwrap();
        authority.register_peer(PEER_TWO, 1, ACTOR_TWO).unwrap();
        let origin = Instant::now();
        let (mut host, initial) = HeadlessHost::with_charged_replica(
            origin,
            ClockConfig::default(),
            HostBudget {
                callback_cpu: Duration::from_millis(5),
                other_work: Duration::from_millis(1),
                tick_cost_bound: Duration::from_millis(1),
            },
            authority,
            &pool,
            2,
        )
        .unwrap();
        assert_eq!(initial.resolved_through(), &[(PEER, 0), (PEER_TWO, 0)]);
        assert_eq!(
            host.on_callback(origin + Duration::from_millis(17)).err(),
            Some(HostError::WrongMode)
        );
        for (peer, actor, action) in [(PEER, ACTOR, 1), (PEER_TWO, ACTOR_TWO, 0)] {
            host.submit(Input {
                origin: Origin {
                    peer,
                    connection_epoch: 1,
                    sequence: 1,
                },
                actor,
                schedule: InputSchedule::NextTick { observed_tick: 0 },
                action,
            })
            .unwrap();
        }
        let held = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes())
            .unwrap();
        let blocked = host
            .on_callback_charged(origin + Duration::from_millis(17))
            .unwrap();
        assert_eq!(
            blocked.clock.step_error,
            Some(StepError::PublicationBackpressure)
        );
        assert_eq!(blocked.clock.completed, 0);
        assert_eq!(host.authority().tick_index(), 0);
        assert_eq!(host.replica().unwrap().tick(), 0);
        assert_eq!(
            host.authority()
                .world()
                .get(host.authority().world().handle_for(StableId(1)).unwrap())
                .unwrap()
                .state
                .inline(),
            &1_u64.to_le_bytes()
        );
        drop(blocked);
        drop(held);
        let report = host
            .on_callback_charged(origin + Duration::from_millis(17))
            .unwrap();
        assert_eq!(report.clock.completed, 1);
        assert_eq!(report.replica_groups, 1);
        let HostPublication::ReplicatedCharged { admitted, .. } = &report.publications[0] else {
            panic!("expected charged replica publication")
        };
        assert_eq!(admitted.results().len(), 2);
        assert_eq!(admitted.events().len(), 1);
        assert_eq!(admitted.resolved_through(), &[(PEER, 1), (PEER_TWO, 1)]);
        assert_eq!(
            host.replica().unwrap().get(StableId(1)).unwrap().inline(),
            &2_u64.to_le_bytes()
        );
        drop(initial);
        drop(host);
        assert!(pool.used_bytes() > 0);
        drop(report);
        assert_eq!(pool.used_bytes(), 0);
    }

    struct OutputRule {
        entity_pool: PublicationPool,
        output_pool: PublicationPool,
    }

    #[test]
    fn charged_baseline_reserves_all_columns_and_shares_last_owner() {
        let pool = PublicationPool::new(100_000);
        let mut world = World::new(WorldEpoch(231));
        world
            .spawn(
                StableId(1),
                Entity::try_new(&pool, 1_u64.to_le_bytes(), []).unwrap(),
            )
            .unwrap();
        let output_limits = ClosedValueLimits {
            max_inline_bytes: 0,
            max_payload_slots: 1,
            max_payload_bytes: 32,
        };
        let mut authority = Authority::try_new_closed_snapshot_charged(
            AuthorityId::Match(MatchId(231)),
            OutputRule {
                entity_pool: pool.clone(),
                output_pool: pool.clone(),
            },
            world,
            pool.clone(),
            SnapshotWriteBounds {
                max_edits_per_action: 1,
                max_spawns_per_action: 0,
            },
            ClosedGameLimits {
                action: ClosedValueLimits {
                    max_inline_bytes: 1,
                    max_payload_slots: 0,
                    max_payload_bytes: 0,
                },
                entity: ClosedValueLimits {
                    max_inline_bytes: 8,
                    max_payload_slots: 0,
                    max_payload_bytes: 0,
                },
                event: output_limits,
                rejection: output_limits,
            },
        )
        .unwrap();
        for (peer, actor) in [(PEER, ACTOR), (PEER_TWO, ACTOR_TWO)] {
            authority.register_peer(peer, 1, actor).unwrap();
            authority
                .submit(Input {
                    origin: Origin {
                        peer,
                        connection_epoch: 1,
                        sequence: 1,
                    },
                    actor,
                    schedule: InputSchedule::NextTick { observed_tick: 0 },
                    action: u8::from(peer == PEER),
                })
                .unwrap();
        }
        drop(authority.step().unwrap());
        let prototype = authority.baseline();
        let before = pool.used_bytes();
        let occupied = pool.try_reserve(pool.limit_bytes() - before).unwrap();
        assert!(matches!(
            authority.try_baseline_charged(),
            Err(ChargedBaselineError::Capacity)
        ));
        assert_eq!(pool.used_bytes(), pool.limit_bytes());
        drop(occupied);

        let charged = authority.try_baseline_charged().unwrap();
        assert_eq!(charged.authority(), prototype.authority);
        assert_eq!(charged.group_id(), prototype.group_id);
        assert_eq!(charged.commit_tick(), prototype.commit_tick);
        assert_eq!(charged.event_cursor(), prototype.event_cursor);
        assert_eq!(charged.changes(), prototype.changes);
        assert_eq!(charged.results(), prototype.results);
        assert_eq!(charged.events(), prototype.events);
        assert_eq!(charged.resolved_through(), prototype.resolved_through);
        assert_eq!(charged.results().len(), 2);
        assert_eq!(charged.events().len(), 1);
        assert_eq!(charged.resolved_through().len(), 2);
        let with_columns = pool.used_bytes();
        assert!(with_columns > before);
        let reader = charged.clone();
        assert_eq!(pool.used_bytes(), with_columns);
        drop(prototype);
        drop(authority);
        assert!(pool.used_bytes() > 0);
        drop(charged);
        assert!(pool.used_bytes() > 0);
        drop(reader);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn charged_tick_reserves_group_before_edit_and_keeps_two_peer_outputs() {
        let pool = PublicationPool::new(100_000);
        let mut world = World::new(WorldEpoch(232));
        world
            .spawn(
                StableId(1),
                Entity::try_new(&pool, 1_u64.to_le_bytes(), []).unwrap(),
            )
            .unwrap();
        let output_limits = ClosedValueLimits {
            max_inline_bytes: 0,
            max_payload_slots: 1,
            max_payload_bytes: 32,
        };
        let mut authority = Authority::try_new_closed_snapshot_charged(
            AuthorityId::Match(MatchId(232)),
            OutputRule {
                entity_pool: pool.clone(),
                output_pool: pool.clone(),
            },
            world,
            pool.clone(),
            SnapshotWriteBounds {
                max_edits_per_action: 1,
                max_spawns_per_action: 0,
            },
            ClosedGameLimits {
                action: ClosedValueLimits {
                    max_inline_bytes: 1,
                    max_payload_slots: 0,
                    max_payload_bytes: 0,
                },
                entity: ClosedValueLimits {
                    max_inline_bytes: 8,
                    max_payload_slots: 0,
                    max_payload_bytes: 0,
                },
                event: output_limits,
                rejection: output_limits,
            },
        )
        .unwrap();
        for (peer, actor) in [(PEER, ACTOR), (PEER_TWO, ACTOR_TWO)] {
            authority.register_peer(peer, 1, actor).unwrap();
            authority
                .submit(Input {
                    origin: Origin {
                        peer,
                        connection_epoch: 1,
                        sequence: 1,
                    },
                    actor,
                    schedule: InputSchedule::NextTick { observed_tick: 0 },
                    action: u8::from(peer == PEER),
                })
                .unwrap();
        }
        let before_tick = authority.tick_index();
        let snapshot_peak = authority
            .world()
            .conservative_charged_publication_peak(2, 0)
            .unwrap();
        let occupied = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes() - snapshot_peak)
            .unwrap();
        assert_eq!(
            authority.step_charged().err(),
            Some(StepError::PublicationBackpressure)
        );
        assert_eq!(authority.tick_index(), before_tick);
        assert!(!authority.is_faulted());
        assert_eq!(
            authority
                .world()
                .get(authority.world().handle_for(StableId(1)).unwrap())
                .unwrap()
                .state
                .inline(),
            &1_u64.to_le_bytes()
        );
        drop(occupied);

        let tick = authority.step_charged().unwrap();
        assert_eq!(tick.group.commit_tick(), 1);
        assert_eq!(tick.group.changes().len(), 1);
        assert_eq!(tick.group.results().len(), 2);
        assert_eq!(tick.group.events().len(), 1);
        assert_eq!(tick.group.resolved_through(), &[(PEER, 1), (PEER_TWO, 1)]);
        let with_group = pool.used_bytes();
        let reader = tick.group.clone();
        assert_eq!(pool.used_bytes(), with_group);
        drop(tick);
        let occupied = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes())
            .unwrap();
        let idle = authority.step_charged().unwrap();
        assert!(idle.group.changes().is_empty());
        assert!(idle.group.results().is_empty());
        assert_eq!(idle.group.resolved_through(), &[(PEER, 1), (PEER_TWO, 1)]);
        assert_eq!(pool.used_bytes(), pool.limit_bytes());
        drop(idle);
        drop(occupied);
        authority
            .submit(Input {
                origin: Origin {
                    peer: PEER,
                    connection_epoch: 1,
                    sequence: 2,
                },
                actor: ACTOR,
                schedule: InputSchedule::NextTick { observed_tick: 2 },
                action: 0,
            })
            .unwrap();
        let next = authority.step_charged().unwrap();
        assert_eq!(next.group.results().len(), 1);
        assert_eq!(next.group.resolved_through(), &[(PEER, 2), (PEER_TWO, 1)]);
        drop(next);
        drop(authority);
        assert!(pool.used_bytes() > 0);
        drop(reader);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn charged_tick_rejects_unqualified_mode_without_poisoning_prototype() {
        let pool = PublicationPool::new(100_000);
        let mut authority = Authority::new(
            AuthorityId::Match(MatchId(233)),
            OutputRule {
                entity_pool: pool.clone(),
                output_pool: pool,
            },
            World::new(WorldEpoch(233)),
        );
        assert_eq!(authority.step_charged().err(), Some(StepError::Unqualified));
        assert!(!authority.is_faulted());
        assert_eq!(authority.step().unwrap().group.commit_tick, 1);
    }

    #[test]
    fn charged_two_peer_baseline_and_tick_admit_into_one_finite_replica() {
        let pool = PublicationPool::new(1_000_000);
        let foreign = PublicationPool::new(1_000_000);
        let mut world = World::new(WorldEpoch(234));
        world
            .spawn(
                StableId(1),
                Entity::try_new(&pool, 1_u64.to_le_bytes(), []).unwrap(),
            )
            .unwrap();
        let output_limits = ClosedValueLimits {
            max_inline_bytes: 0,
            max_payload_slots: 1,
            max_payload_bytes: 32,
        };
        let mut authority = Authority::try_new_closed_snapshot_charged(
            AuthorityId::Match(MatchId(234)),
            OutputRule {
                entity_pool: pool.clone(),
                output_pool: pool.clone(),
            },
            world,
            pool.clone(),
            SnapshotWriteBounds {
                max_edits_per_action: 1,
                max_spawns_per_action: 0,
            },
            ClosedGameLimits {
                action: ClosedValueLimits {
                    max_inline_bytes: 1,
                    max_payload_slots: 0,
                    max_payload_bytes: 0,
                },
                entity: ClosedValueLimits {
                    max_inline_bytes: 8,
                    max_payload_slots: 0,
                    max_payload_bytes: 0,
                },
                event: output_limits,
                rejection: output_limits,
            },
        )
        .unwrap();
        authority.register_peer(PEER, 1, ACTOR).unwrap();
        authority.register_peer(PEER_TWO, 1, ACTOR_TWO).unwrap();
        let baseline = authority.try_baseline_charged().unwrap();
        let prior = pool.used_bytes();
        assert!(matches!(
            Replica::from_charged_baseline(baseline.clone(), &foreign, 2),
            Err(ChargedReplicaSetupError::WrongPool)
        ));
        assert_eq!(pool.used_bytes(), prior);
        let (mut replica, initial) = Replica::from_charged_baseline(baseline, &pool, 2).unwrap();
        assert!(replica.charged_object_bytes().unwrap() > 0);
        assert_eq!(replica.len(), 1);
        assert_eq!(
            replica.get(StableId(1)).unwrap().inline(),
            &1_u64.to_le_bytes()
        );
        assert!(initial.results().is_empty());
        assert_eq!(initial.resolved_through(), &[(PEER, 0), (PEER_TWO, 0)]);
        assert_eq!(replica.can_admit_charged_spawns(1), Ok(true));
        assert_eq!(replica.can_admit_charged_spawns(2), Ok(false));
        for (peer, actor) in [(PEER, ACTOR), (PEER_TWO, ACTOR_TWO)] {
            authority
                .submit(Input {
                    origin: Origin {
                        peer,
                        connection_epoch: 1,
                        sequence: 1,
                    },
                    actor,
                    schedule: InputSchedule::NextTick { observed_tick: 0 },
                    action: u8::from(peer == PEER),
                })
                .unwrap();
        }
        let tick = authority.step_charged().unwrap();
        let slow_reader = tick.group.clone();
        let admitted = replica.apply_charged(tick.group).unwrap();
        assert_eq!(replica.tick(), 1);
        assert_eq!(
            replica.get(StableId(1)).unwrap().inline(),
            &2_u64.to_le_bytes()
        );
        assert_eq!(admitted.results().len(), 2);
        assert_eq!(admitted.events().len(), 1);
        assert_eq!(admitted.resolved_through(), &[(PEER, 1), (PEER_TWO, 1)]);
        assert_eq!(
            replica.apply(authority.baseline()).err(),
            Some(ReplicaError::UnqualifiedGroup)
        );
        drop(tick.snapshot);
        drop(initial);
        drop(authority);
        drop(replica);
        assert!(pool.used_bytes() > 0);
        drop(admitted);
        assert!(pool.used_bytes() > 0);
        drop(slow_reader);
        assert_eq!(pool.used_bytes(), 0);
    }
    impl GameRules for OutputRule {
        type Entity = Entity;
        type Action = u8;
        type Change = Handle;
        type Rejection = Output;
        type Event = Output;

        fn decide(
            &self,
            world: &World<Entity>,
            _actor: ActorId,
            action: &u8,
            _context: &mut thrum_core::RuleContext<'_>,
        ) -> Result<Handle, Output> {
            if *action == 0 {
                Err(output(&self.output_pool))
            } else {
                Ok(world.handle_for(StableId(1)).unwrap())
            }
        }

        fn prepare(
            &self,
            _world: &World<Entity>,
            handle: Handle,
            _context: &mut thrum_core::RuleContext<'_>,
        ) -> PreparedCommit<Entity, Output> {
            PreparedCommit {
                edits: PreparedEdits::One(WorldEdit::Replace(
                    handle,
                    Entity::try_new(&self.entity_pool, 2_u64.to_le_bytes(), []).unwrap(),
                )),
                event: Some(output(&self.output_pool)),
            }
        }
    }

    #[test]
    fn rejects_output_shape_without_input() {
        let shape_pool = PublicationPool::new(100_000);
        assert!(matches!(
            Authority::try_new_closed_snapshot_charged(
                AuthorityId::Match(MatchId(219)),
                OutputRule {
                    entity_pool: shape_pool.clone(),
                    output_pool: shape_pool.clone(),
                },
                World::new(WorldEpoch(219)),
                shape_pool.clone(),
                SnapshotWriteBounds {
                    max_edits_per_action: 1,
                    max_spawns_per_action: 0,
                },
                closed_game_limits(ClosedValueLimits {
                    max_inline_bytes: 8,
                    max_payload_slots: 0,
                    max_payload_bytes: 0,
                }),
            ),
            Err(WorldPublicationError::ValueProfile)
        ));
        assert_eq!(shape_pool.used_bytes(), 0);
        assert!(matches!(
            Authority::try_new_closed_snapshot_charged(
                AuthorityId::Match(MatchId(218)),
                OutputRule {
                    entity_pool: shape_pool.clone(),
                    output_pool: shape_pool.clone(),
                },
                World::new(WorldEpoch(218)),
                shape_pool.clone(),
                SnapshotWriteBounds {
                    max_edits_per_action: 1,
                    max_spawns_per_action: 0,
                },
                ClosedGameLimits {
                    action: ClosedValueLimits {
                        max_inline_bytes: 1,
                        max_payload_slots: 0,
                        max_payload_bytes: 0,
                    },
                    entity: ClosedValueLimits {
                        max_inline_bytes: 8,
                        max_payload_slots: 0,
                        max_payload_bytes: 0,
                    },
                    event: ClosedValueLimits {
                        max_inline_bytes: 0,
                        max_payload_slots: 1,
                        max_payload_bytes: 32,
                    },
                    rejection: ClosedValueLimits {
                        max_inline_bytes: 0,
                        max_payload_slots: 0,
                        max_payload_bytes: 32,
                    },
                },
            ),
            Err(WorldPublicationError::ValueProfile)
        ));
        assert_eq!(shape_pool.used_bytes(), 0);
    }

    #[test]
    fn rejects_foreign_and_oversized_output_values_before_edit() {
        for (case, action, foreign_output, max_output_bytes) in [
            (0_u128, 0_u8, true, 32_usize),
            (1, 1, true, 32),
            (2, 0, false, 31),
            (3, 1, false, 31),
        ] {
            let pool = PublicationPool::new(100_000);
            let foreign = PublicationPool::new(100_000);
            let output_pool = if foreign_output {
                foreign.clone()
            } else {
                pool.clone()
            };
            let mut world = World::new(WorldEpoch(220 + case));
            let initial = Entity::try_new(&pool, 1_u64.to_le_bytes(), []).unwrap();
            let handle = world.spawn(StableId(1), initial).unwrap();
            let output_limits = ClosedValueLimits {
                max_inline_bytes: 0,
                max_payload_slots: 1,
                max_payload_bytes: max_output_bytes,
            };
            let mut authority = Authority::try_new_closed_snapshot_charged(
                AuthorityId::Match(MatchId(220 + case)),
                OutputRule {
                    entity_pool: pool.clone(),
                    output_pool,
                },
                world,
                pool,
                SnapshotWriteBounds {
                    max_edits_per_action: 1,
                    max_spawns_per_action: 0,
                },
                ClosedGameLimits {
                    action: ClosedValueLimits {
                        max_inline_bytes: 1,
                        max_payload_slots: 0,
                        max_payload_bytes: 0,
                    },
                    entity: ClosedValueLimits {
                        max_inline_bytes: 8,
                        max_payload_slots: 0,
                        max_payload_bytes: 0,
                    },
                    event: output_limits,
                    rejection: output_limits,
                },
            )
            .unwrap();
            authority.register_peer(PEER, 1, ACTOR).unwrap();
            authority
                .submit(Input {
                    origin: Origin {
                        peer: PEER,
                        connection_epoch: 1,
                        sequence: 1,
                    },
                    actor: ACTOR,
                    schedule: InputSchedule::NextTick { observed_tick: 0 },
                    action,
                })
                .unwrap();
            assert!(matches!(
                authority.step(),
                Err(StepError::TerminalGameContract)
            ));
            assert_eq!(authority.tick_index(), 0);
            assert_eq!(
                u64::from_le_bytes(*authority.world().get(handle).unwrap().state.inline()),
                1
            );
            assert_eq!(foreign.used_bytes(), 0);
        }
    }

    #[test]
    fn retains_valid_outputs_until_acknowledgement_and_final_reader() {
        let pool = PublicationPool::new(100_000);
        let mut world = World::new(WorldEpoch(230));
        let initial = Entity::try_new(&pool, 1_u64.to_le_bytes(), []).unwrap();
        world.spawn(StableId(1), initial).unwrap();
        let mut authority = Authority::try_new_closed_snapshot_charged(
            AuthorityId::Match(MatchId(230)),
            OutputRule {
                entity_pool: pool.clone(),
                output_pool: pool.clone(),
            },
            world,
            pool.clone(),
            SnapshotWriteBounds {
                max_edits_per_action: 1,
                max_spawns_per_action: 0,
            },
            ClosedGameLimits {
                action: ClosedValueLimits {
                    max_inline_bytes: 1,
                    max_payload_slots: 0,
                    max_payload_bytes: 0,
                },
                entity: ClosedValueLimits {
                    max_inline_bytes: 8,
                    max_payload_slots: 0,
                    max_payload_bytes: 0,
                },
                event: ClosedValueLimits {
                    max_inline_bytes: 0,
                    max_payload_slots: 1,
                    max_payload_bytes: 32,
                },
                rejection: ClosedValueLimits {
                    max_inline_bytes: 0,
                    max_payload_slots: 1,
                    max_payload_bytes: 32,
                },
            },
        )
        .unwrap();
        authority.register_peer(PEER, 1, ACTOR).unwrap();
        let (mut replica, _) = Replica::from_baseline(authority.baseline()).unwrap();
        let idle_charge = pool.used_bytes();
        let rejection_input = Input {
            origin: Origin {
                peer: PEER,
                connection_epoch: 1,
                sequence: 1,
            },
            actor: ACTOR,
            schedule: InputSchedule::NextTick { observed_tick: 0 },
            action: 0_u8,
        };
        authority.submit(rejection_input.clone()).unwrap();
        let after_history_admission = pool.used_bytes();
        assert!(after_history_admission > idle_charge);
        let rejected = authority.step().unwrap();
        let rejected_admitted = replica.apply(rejected.group).unwrap();
        let ResultDisposition::Rejected(ActionRejection::Game(reason)) =
            &rejected_admitted.results[0].disposition
        else {
            panic!("closed rejection missing from replica result");
        };
        assert_eq!(reason.payload_owner(0).unwrap().as_slice(), &[8; 32]);
        let duplicate = authority.submit(rejection_input.clone()).unwrap();
        assert!(matches!(duplicate, Admission::ResolvedDuplicate(_)));
        let rejection_baseline = authority.baseline();
        assert_eq!(rejection_baseline.results.len(), 1);
        authority.acknowledge_results(PEER, 1).unwrap();
        assert!(matches!(
            authority.submit(rejection_input),
            Err(AdmissionError::Expired)
        ));
        drop(duplicate);
        drop(rejection_baseline);
        drop(rejected_admitted);
        drop(rejected.snapshot);
        assert_eq!(pool.used_bytes(), after_history_admission);

        authority
            .submit(Input {
                origin: Origin {
                    peer: PEER,
                    connection_epoch: 1,
                    sequence: 2,
                },
                actor: ACTOR,
                schedule: InputSchedule::NextTick { observed_tick: 1 },
                action: 1,
            })
            .unwrap();
        let committed = authority.step().unwrap();
        let event_admitted = replica.apply(committed.group).unwrap();
        assert_eq!(event_admitted.events.len(), 1);
        assert_eq!(
            event_admitted.events[0]
                .payload
                .payload_owner(0)
                .unwrap()
                .as_slice(),
            &[8; 32]
        );
        assert_eq!(
            u64::from_le_bytes(*replica.get(StableId(1)).unwrap().inline()),
            2
        );
        let event_charge = PublicationBytes::required_charge(32).unwrap();
        let with_event = pool.used_bytes();
        let event_baseline = authority.baseline();
        assert_eq!(event_baseline.events.len(), 1);
        authority.acknowledge_results(PEER, 2).unwrap();
        drop(event_baseline);
        drop(event_admitted);
        drop(committed.snapshot);
        assert_eq!(pool.used_bytes(), with_event - event_charge);
    }
}

mod closed_action_fixture {
    use super::*;

    type Entity = PublicationRecord<8, 0>;
    type ActionValue = PublicationRecord<0, 1>;

    struct Rule {
        pool: PublicationPool,
    }
    impl GameRules for Rule {
        type Entity = Entity;
        type Action = ActionValue;
        type Change = Handle;
        type Rejection = ();
        type Event = ();

        fn decide(
            &self,
            world: &World<Entity>,
            _actor: ActorId,
            _action: &ActionValue,
            _context: &mut thrum_core::RuleContext<'_>,
        ) -> Result<Handle, ()> {
            world.handle_for(StableId(1)).ok_or(())
        }

        fn prepare(
            &self,
            _world: &World<Entity>,
            handle: Handle,
            _context: &mut thrum_core::RuleContext<'_>,
        ) -> PreparedCommit<Entity, ()> {
            PreparedCommit {
                edits: PreparedEdits::One(WorldEdit::Replace(
                    handle,
                    Entity::try_new(&self.pool, 2_u64.to_le_bytes(), []).unwrap(),
                )),
                event: None,
            }
        }
    }

    fn limits(max_payload_slots: usize, max_payload_bytes: usize) -> ClosedGameLimits {
        ClosedGameLimits {
            action: ClosedValueLimits {
                max_inline_bytes: 0,
                max_payload_slots,
                max_payload_bytes,
            },
            ..closed_game_limits(ClosedValueLimits {
                max_inline_bytes: 8,
                max_payload_slots: 0,
                max_payload_bytes: 0,
            })
        }
    }

    fn action(pool: &PublicationPool, len: usize) -> ActionValue {
        let payload = PublicationBytes::try_copy_from_slice(pool, &vec![5; len]).unwrap();
        ActionValue::try_new(pool, [], [Some(payload)]).unwrap()
    }

    #[test]
    fn rejects_action_type_shape_before_activation() {
        let pool = PublicationPool::new(100_000);
        assert!(matches!(
            Authority::try_new_closed_snapshot_charged(
                AuthorityId::Match(MatchId(240)),
                Rule { pool: pool.clone() },
                World::new(WorldEpoch(240)),
                pool.clone(),
                SnapshotWriteBounds {
                    max_edits_per_action: 1,
                    max_spawns_per_action: 0,
                },
                limits(0, 32),
            ),
            Err(WorldPublicationError::ValueProfile)
        ));
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn rejects_bad_actions_before_sequence_and_releases_valid_history_after_ack() {
        let pool = PublicationPool::new(100_000);
        let foreign = PublicationPool::new(100_000);
        let mut world = World::new(WorldEpoch(241));
        world
            .spawn(
                StableId(1),
                Entity::try_new(&pool, 1_u64.to_le_bytes(), []).unwrap(),
            )
            .unwrap();
        let mut authority = Authority::try_new_closed_snapshot_charged(
            AuthorityId::Match(MatchId(241)),
            Rule { pool: pool.clone() },
            world,
            pool.clone(),
            SnapshotWriteBounds {
                max_edits_per_action: 1,
                max_spawns_per_action: 0,
            },
            limits(1, 32),
        )
        .unwrap();
        authority.register_peer(PEER, 1, ACTOR).unwrap();
        let (mut replica, _) = Replica::from_baseline(authority.baseline()).unwrap();
        let origin = Origin {
            peer: PEER,
            connection_epoch: 1,
            sequence: 1,
        };
        let make_input = |value| Input {
            origin,
            actor: ACTOR,
            schedule: InputSchedule::NextTick { observed_tick: 0 },
            action: value,
        };
        let before = pool.used_bytes();
        assert!(matches!(
            authority.submit(make_input(action(&foreign, 32))),
            Err(AdmissionError::WrongPool)
        ));
        assert_eq!(foreign.used_bytes(), 0);
        assert!(matches!(
            authority.submit(make_input(action(&pool, 33))),
            Err(AdmissionError::ValueProfile)
        ));
        assert_eq!(pool.used_bytes(), before);

        let valid = make_input(action(&pool, 32));
        let action_charge = PublicationBytes::required_charge(32).unwrap();
        assert_eq!(authority.submit(valid.clone()).unwrap(), Admission::Queued);
        assert_eq!(
            authority.submit(valid.clone()).unwrap(),
            Admission::PendingDuplicate
        );
        assert_eq!(
            authority.submit(make_input(action(&foreign, 32))).unwrap(),
            Admission::PendingDuplicate
        );
        assert_eq!(foreign.used_bytes(), 0);
        let committed = authority.step().unwrap();
        let admitted = replica.apply(committed.group).unwrap();
        assert_eq!(admitted.results.len(), 1);
        assert_eq!(
            u64::from_le_bytes(*replica.get(StableId(1)).unwrap().inline()),
            2
        );
        let with_action = pool.used_bytes();
        authority.acknowledge_results(PEER, 1).unwrap();
        drop(valid);
        assert_eq!(pool.used_bytes(), with_action - action_charge);

        let before_pending = pool.used_bytes();
        let pending = Input {
            origin: Origin {
                connection_epoch: 1,
                sequence: 2,
                ..origin
            },
            actor: ACTOR,
            schedule: InputSchedule::NextTick { observed_tick: 1 },
            action: action(&pool, 32),
        };
        authority.submit(pending).unwrap();
        assert_eq!(pool.used_bytes(), before_pending + action_charge);
        authority.register_peer(PEER, 2, ACTOR).unwrap();
        assert_eq!(pool.used_bytes(), before_pending);
        let stale = Input {
            origin: Origin {
                connection_epoch: 1,
                sequence: 1,
                ..origin
            },
            actor: ACTOR,
            schedule: InputSchedule::NextTick { observed_tick: 1 },
            action: action(&foreign, 32),
        };
        assert_eq!(authority.submit(stale), Err(AdmissionError::StaleEpoch));
        assert_eq!(foreign.used_bytes(), 0);
    }
}

mod protected_output_fixture {
    use super::*;
    use std::time::{Duration, Instant};

    type Entity = PublicationRecord<1, 1>;
    type Output = PublicationRecord<0, 1>;

    struct Rule {
        pool: PublicationPool,
    }

    impl GameRules for Rule {
        type Entity = Entity;
        type Action = u8;
        type Change = (Handle, u8);
        type Rejection = Output;
        type Event = Output;

        fn decide(
            &self,
            world: &World<Entity>,
            _actor: ActorId,
            action: &u8,
            context: &mut thrum_core::RuleContext<'_>,
        ) -> Result<(Handle, u8), Output> {
            if *action == 0 {
                let payload = context.try_copy_payload(&[3; 16]).unwrap();
                return Err(Output::try_new(&self.pool, [], [Some(payload)]).unwrap());
            }
            if *action == 6 {
                drop(context.try_copy_payload(&[6; 16]).unwrap());
            }
            Ok((world.handle_for(StableId(1)).unwrap(), *action))
        }

        fn prepare(
            &self,
            _world: &World<Entity>,
            (handle, value): (Handle, u8),
            context: &mut thrum_core::RuleContext<'_>,
        ) -> PreparedCommit<Entity, Output> {
            let payload = context.try_copy_payload(&[value; 32]).unwrap();
            if value == 9 {
                let _ = context.try_copy_payload(&[9; 32]);
            }
            PreparedCommit {
                edits: PreparedEdits::One(WorldEdit::Replace(
                    handle,
                    Entity::try_new(&self.pool, [value], [Some(payload.clone())]).unwrap(),
                )),
                event: Some(Output::try_new(&self.pool, [], [Some(payload)]).unwrap()),
            }
        }
    }

    fn host_with_allowance(
        output_allowance: usize,
    ) -> (
        HeadlessHost<Rule>,
        thrum_core::InitialChargedOutcomes<Rule>,
        PublicationPool,
        Instant,
    ) {
        let pool = PublicationPool::new(200_000);
        let mut world = World::new(WorldEpoch(260));
        world
            .spawn(StableId(1), Entity::try_new(&pool, [1], [None]).unwrap())
            .unwrap();
        let max_payload = ClosedValueLimits {
            max_inline_bytes: 0,
            max_payload_slots: 1,
            max_payload_bytes: 32,
        };
        let mut authority = Authority::try_new_closed_snapshot_charged(
            AuthorityId::Match(MatchId(260)),
            Rule { pool: pool.clone() },
            world,
            pool.clone(),
            SnapshotWriteBounds {
                max_edits_per_action: 1,
                max_spawns_per_action: 0,
            },
            ClosedGameLimits {
                action: ClosedValueLimits {
                    max_inline_bytes: 1,
                    max_payload_slots: 0,
                    max_payload_bytes: 0,
                },
                entity: ClosedValueLimits {
                    max_inline_bytes: 1,
                    ..max_payload
                },
                event: max_payload,
                rejection: max_payload,
            },
        )
        .unwrap();
        authority.register_peer(PeerId(1), 1, ActorId(1)).unwrap();
        authority.register_peer(PeerId(2), 1, ActorId(2)).unwrap();
        let origin = Instant::now();
        let (host, initial) = HeadlessHost::with_protected_output_replica(
            origin,
            ClockConfig::default(),
            HostBudget {
                callback_cpu: Duration::from_millis(5),
                other_work: Duration::from_millis(1),
                tick_cost_bound: Duration::from_millis(1),
            },
            authority,
            &pool,
            1,
            2,
            output_allowance,
        )
        .unwrap();
        (host, initial, pool, origin)
    }

    fn host() -> (
        HeadlessHost<Rule>,
        thrum_core::InitialChargedOutcomes<Rule>,
        PublicationPool,
        Instant,
    ) {
        host_with_allowance(PublicationBytes::required_charge(32).unwrap())
    }

    fn input(peer: u64, action: u8) -> Input<u8> {
        Input {
            origin: Origin {
                peer: PeerId(peer),
                connection_epoch: 1,
                sequence: 1,
            },
            actor: ActorId(peer),
            schedule: InputSchedule::NextTick { observed_tick: 0 },
            action,
        }
    }

    #[test]
    fn two_peer_state_event_and_rejection_payloads_use_separate_credits() {
        let (mut host, initial, pool, origin) = host();
        let old_snapshot = host.authority().snapshot().clone();
        host.submit(input(1, 4)).unwrap();
        host.submit(input(2, 0)).unwrap();
        let occupied = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes())
            .unwrap();
        let report = host
            .on_callback_charged(origin + Duration::from_millis(17))
            .unwrap();
        assert_eq!(report.clock.completed, 1);
        let HostPublication::ReplicatedCharged { admitted, .. } = &report.publications[0] else {
            panic!("expected charged publication")
        };
        assert_eq!(admitted.results().len(), 2);
        assert_eq!(admitted.events().len(), 1);
        assert_eq!(
            admitted.events()[0]
                .payload
                .payload_owner(0)
                .unwrap()
                .as_slice(),
            &[4; 32]
        );
        let rejected = admitted
            .results()
            .iter()
            .find_map(|result| match &result.disposition {
                ResultDisposition::Rejected(ActionRejection::Game(reason)) => Some(reason),
                _ => None,
            })
            .unwrap();
        assert_eq!(rejected.payload_owner(0).unwrap().as_slice(), &[3; 16]);
        assert_eq!(
            host.replica()
                .unwrap()
                .get(StableId(1))
                .unwrap()
                .payload_owner(0)
                .unwrap()
                .as_slice(),
            &[4; 32]
        );
        assert_eq!(
            old_snapshot
                .records()
                .find(|record| record.id == StableId(1))
                .unwrap()
                .state
                .inline(),
            &[1]
        );
        drop(report);
        host.acknowledge_results(PeerId(1), 1).unwrap();
        host.acknowledge_results(PeerId(2), 1).unwrap();
        drop(occupied);
        drop(old_snapshot);
        drop(initial);
        drop(host);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn future_output_credit_survives_a_delayed_sequence_under_competing_use() {
        let (mut host, initial, pool, origin) = host();
        let mut future = input(1, 4);
        future.schedule = InputSchedule::Timed { intended_tick: 3 };
        host.submit(future).unwrap();
        let mut early = input(1, 5);
        early.origin.sequence = 2;
        host.submit(early).unwrap();
        let occupied = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes())
            .unwrap();
        for (tick, expected_results, expected_value, expected_watermark) in
            [(1, 1, 5, 0), (2, 0, 5, 0), (3, 1, 4, 2)]
        {
            let report = host
                .on_callback_charged(origin + Duration::from_millis(17 * tick))
                .unwrap();
            assert_eq!(report.clock.completed, 1);
            let HostPublication::ReplicatedCharged { admitted, .. } = &report.publications[0]
            else {
                panic!("expected charged publication")
            };
            assert_eq!(admitted.results().len(), expected_results);
            assert_eq!(
                admitted.resolved_through()[0],
                (PeerId(1), expected_watermark)
            );
            assert_eq!(
                host.replica().unwrap().get(StableId(1)).unwrap().inline(),
                &[expected_value]
            );
        }
        host.acknowledge_results(PeerId(1), 2).unwrap();
        drop(occupied);
        drop(initial);
        drop(host);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn decision_and_preparation_spend_one_cumulative_payload_allowance() {
        let allowance = PublicationBytes::required_charge(16).unwrap()
            + PublicationBytes::required_charge(32).unwrap();
        let (mut host, initial, pool, origin) = host_with_allowance(allowance);
        host.submit(input(1, 6)).unwrap();
        let occupied = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes())
            .unwrap();
        let report = host
            .on_callback_charged(origin + Duration::from_millis(17))
            .unwrap();
        assert_eq!(report.clock.completed, 1);
        assert_eq!(
            host.replica().unwrap().get(StableId(1)).unwrap().inline(),
            &[6]
        );
        drop(report);
        host.acknowledge_results(PeerId(1), 1).unwrap();
        drop(occupied);
        drop(initial);
        drop(host);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn old_snapshot_keeps_variable_payload_after_replacement_and_host_drop() {
        let (mut host, initial, pool, origin) = host();
        host.submit(input(1, 4)).unwrap();
        let first = host
            .on_callback_charged(origin + Duration::from_millis(17))
            .unwrap();
        let HostPublication::ReplicatedCharged { snapshot, .. } = &first.publications[0] else {
            panic!("expected charged publication")
        };
        let old = snapshot.clone();
        drop(first);
        host.acknowledge_results(PeerId(1), 1).unwrap();
        let mut next = input(1, 5);
        next.origin.sequence = 2;
        next.schedule = InputSchedule::NextTick { observed_tick: 1 };
        host.submit(next).unwrap();
        let occupied = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes())
            .unwrap();
        let second = host
            .on_callback_charged(origin + Duration::from_millis(34))
            .unwrap();
        assert_eq!(second.clock.completed, 1);
        assert_eq!(
            old.records()
                .next()
                .unwrap()
                .state
                .payload_owner(0)
                .unwrap()
                .as_slice(),
            &[4; 32]
        );
        assert_eq!(
            host.replica()
                .unwrap()
                .get(StableId(1))
                .unwrap()
                .payload_owner(0)
                .unwrap()
                .as_slice(),
            &[5; 32]
        );
        drop(second);
        host.acknowledge_results(PeerId(1), 2).unwrap();
        drop(occupied);
        drop(initial);
        drop(host);
        assert!(pool.used_bytes() >= PublicationBytes::required_charge(32).unwrap());
        drop(old);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn exceeded_cumulative_payload_allowance_faults_before_world_edit() {
        let (mut host, initial, pool, origin) = host();
        host.submit(input(1, 9)).unwrap();
        let occupied = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes())
            .unwrap();
        assert!(matches!(
            host.on_callback_charged(origin + Duration::from_millis(17)),
            Err(HostError::Authority(StepError::TerminalGameContract))
        ));
        assert_eq!(host.authority().tick_index(), 0);
        assert_eq!(
            host.replica().unwrap().get(StableId(1)).unwrap().inline(),
            &[1]
        );
        drop(occupied);
        drop(initial);
        drop(host);
        assert_eq!(pool.used_bytes(), 0);
    }
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
fn changed_group_without_results_preserves_multipeer_progress_validation() {
    let mut authority = Authority::new(AuthorityId::Match(MatchId(29)), Rule, game_world(29));
    authority.register_peer(PEER, 1, ACTOR).unwrap();
    authority.register_peer(PEER_TWO, 1, ACTOR_TWO).unwrap();
    let (mut replica, _) = Replica::from_baseline(authority.baseline()).unwrap();
    let mut group = authority.step().unwrap().group;
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

    replica.apply(group).unwrap();
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
    held.remove(3);
    let resumed = authority.step().unwrap();
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
    let mut duplicate_reference = group.clone();
    duplicate_reference.results.push(group.results[0].clone());
    assert_eq!(
        replica.apply(duplicate_reference).err(),
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
            context: &mut thrum_core::RuleContext<'_>,
        ) -> Result<Change, Rejection> {
            thread::sleep(Duration::from_millis(3));
            Rule.decide(world, actor, action, context)
        }
        fn prepare(
            &self,
            world: &World<Object>,
            change: Change,
            context: &mut thrum_core::RuleContext<'_>,
        ) -> PreparedCommit<Object, Event> {
            Rule.prepare(world, change, context)
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
