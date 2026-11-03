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
        .expect("test setup or assertion requires a value");
    world
        .spawn(
            ITEM,
            Object::Collectible {
                at: [1, 0, 0],
                available: true,
            },
        )
        .expect("test setup or assertion requires a value");
    world
        .spawn(
            DOOR,
            Object::Door {
                at: [2, 0, 0],
                open: false,
            },
        )
        .expect("test setup or assertion requires a value");
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
        .expect("test setup or assertion requires a value");
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
        let door = world
            .handle_for(DOOR)
            .expect("test setup or assertion requires a value");
        let mut state = world
            .get(door)
            .expect("test setup or assertion requires a value")
            .state
            .clone();
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
                assert!(
                    !self.panic_on_second,
                    "game preparation failed after an earlier same-tick edit"
                );
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
    authority
        .register_peer(PEER, 1, ACTOR)
        .expect("test setup or assertion requires a value");
    for (sequence, action) in [(1, Action::Open(DOOR)), (2, Action::Collect(ITEM))] {
        authority
            .submit(input(
                1,
                sequence,
                InputSchedule::NextTick { observed_tick: 0 },
                action,
            ))
            .expect("test setup or assertion requires a value");
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
            prepares: Arc::clone(&prepares),
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
            .get(
                authority
                    .world()
                    .handle_for(DOOR)
                    .expect("test setup or assertion requires a value")
            )
            .expect("test setup or assertion requires a value")
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
            prepares: Arc::clone(&prepares),
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
            prepares: Arc::clone(&prepares),
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
    .expect("test setup or assertion requires a value");
    assert!(matches!(
        host.on_callback(origin + Duration::from_millis(17)),
        Err(HostError::Authority(StepError::TerminalGameContract))
    ));
    assert_eq!(
        host.replica()
            .expect("test setup or assertion requires a value")
            .tick(),
        0
    );
    assert!(matches!(
        host.replica()
            .expect("test setup or assertion requires a value")
            .get(DOOR),
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
            prepares: Arc::clone(&prepares),
            panic_on_second,
        },
        game_world(185),
    );
    authority
        .register_peer(PEER, 1, ACTOR)
        .expect("test setup or assertion requires a value");
    authority
        .submit(input(
            1,
            1,
            InputSchedule::NextTick { observed_tick: 0 },
            Action::Open(DOOR),
        ))
        .expect("test setup or assertion requires a value");
    authority
        .submit(input(
            1,
            2,
            InputSchedule::Timed { intended_tick: 2 },
            Action::Collect(ITEM),
        ))
        .expect("test setup or assertion requires a value");
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
    .expect("test setup or assertion requires a value");
    let report = host
        .on_callback(origin + Duration::from_millis(34))
        .expect("test setup or assertion requires a value");
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
    assert_eq!(
        host.replica()
            .expect("test setup or assertion requires a value")
            .tick(),
        1
    );
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
    .expect("test setup or assertion requires a value");
    let mut authority = Authority::try_new_snapshot_charged(
        AuthorityId::Match(MatchId(190)),
        Rule,
        game_world(191),
        pool.clone(),
        bounds,
    )
    .expect("test setup or assertion requires a value");
    assert!(pool.used_bytes() > 0);
    authority
        .register_peer(PEER, 1, ACTOR)
        .expect("test setup or assertion requires a value");
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
    .expect("test setup or assertion requires a value");
    host.submit(input(
        1,
        1,
        InputSchedule::NextTick { observed_tick: 0 },
        Action::Collect(ITEM),
    ))
    .expect("test setup or assertion requires a value");
    let occupied = pool.limit_bytes() - pool.used_bytes();
    let held = pool
        .try_reserve(occupied)
        .expect("test setup or assertion requires a value");
    let blocked = host
        .on_callback(origin + Duration::from_millis(17))
        .expect("test setup or assertion requires a value");
    assert_eq!(
        blocked.clock.step_error,
        Some(StepError::PublicationBackpressure)
    );
    assert_eq!(blocked.clock.completed, 0);
    assert_eq!(host.authority().tick_index(), 0);
    assert_eq!(
        host.replica()
            .expect("test setup or assertion requires a value")
            .tick(),
        0
    );
    assert!(matches!(
        host.authority()
            .world()
            .get(
                host.authority()
                    .world()
                    .handle_for(ITEM)
                    .expect("test setup or assertion requires a value")
            )
            .expect("test setup or assertion requires a value")
            .state,
        Object::Collectible {
            available: true,
            ..
        }
    ));
    drop(held);
    let committed = host
        .on_callback(origin + Duration::from_millis(17))
        .expect("test setup or assertion requires a value");
    assert_eq!(committed.clock.completed, 1);
    assert_eq!(committed.replica_groups, 1);
    assert_eq!(host.authority().tick_index(), 1);
    assert_eq!(
        host.replica()
            .expect("test setup or assertion requires a value")
            .tick(),
        1
    );
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
    .expect("test setup or assertion requires a value");
    let activated = pool.used_bytes();
    assert!(activated > 0);
    let occupied = pool
        .try_reserve(pool.limit_bytes() - activated)
        .expect("test setup or assertion requires a value");
    for tick in 1..=16 {
        let publication = authority
            .step()
            .expect("test setup or assertion requires a value");
        assert_eq!(publication.snapshot.tick(), tick);
        assert_eq!(publication.group.changes, []);
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
    .expect("test setup or assertion requires a value");
    authority
        .register_peer(PeerId(9), 1, ACTOR)
        .expect("test setup or assertion requires a value");
    authority
        .register_peer(PeerId(3), 1, ACTOR_TWO)
        .expect("test setup or assertion requires a value");
    assert_eq!(
        authority.baseline().resolved_through,
        [(PeerId(3), 0), (PeerId(9), 0)]
    );
    let occupied = pool
        .try_reserve(pool.limit_bytes() - pool.used_bytes())
        .expect("test setup or assertion requires a value");
    assert_eq!(
        authority.register_peer(PeerId(5), 1, ACTOR),
        Err(AdmissionError::PublicationCapacity)
    );
    assert_eq!(pool.used_bytes(), pool.limit_bytes());
    assert_eq!(
        authority.baseline().resolved_through,
        [(PeerId(3), 0), (PeerId(9), 0)]
    );
    authority
        .register_peer(PeerId(9), 2, ACTOR)
        .expect("test setup or assertion requires a value");
    drop(occupied);
    authority
        .register_peer(PeerId(5), 1, ACTOR)
        .expect("test setup or assertion requires a value");
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
    .expect("test setup or assertion requires a value");
    let activated = pool.used_bytes();
    let occupied = pool
        .try_reserve(pool.limit_bytes() - activated)
        .expect("test setup or assertion requires a value");
    assert_eq!(
        authority.register_peer(PEER, 1, ACTOR),
        Err(AdmissionError::PublicationCapacity)
    );
    assert_eq!(pool.used_bytes(), pool.limit_bytes());
    drop(occupied);

    authority
        .register_peer(PEER, 1, ACTOR)
        .expect("test setup or assertion requires a value");
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
    .expect("test setup or assertion requires a value");
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
        .expect("test setup or assertion requires a value");
    let occupied = pool
        .try_reserve(pool.limit_bytes() - pool.used_bytes())
        .expect("test setup or assertion requires a value");
    assert_eq!(
        authority.submit(input(
            1,
            2,
            InputSchedule::NextTick { observed_tick: 0 },
            Action::Collect(ITEM),
        )),
        Err(AdmissionError::PublicationCapacity)
    );
    authority
        .register_peer(PEER, 2, ACTOR)
        .expect("test setup or assertion requires a value");
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
        .expect("test setup or assertion requires a value");
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
    .expect("test setup or assertion requires a value");
    authority
        .register_peer(PEER, 1, ACTOR)
        .expect("test setup or assertion requires a value");
    authority
        .submit(input(
            1,
            1,
            InputSchedule::NextTick { observed_tick: 0 },
            Action::Collect(ITEM),
        ))
        .expect("test setup or assertion requires a value");
    let snapshot_peak = authority
        .world()
        .conservative_charged_publication_peak(2, 0)
        .expect("test setup or assertion requires a value");
    let available = snapshot_peak - 1;
    let occupied = pool
        .try_reserve(pool.limit_bytes() - pool.used_bytes() - available)
        .expect("test setup or assertion requires a value");
    assert_eq!(
        authority.step().err(),
        Some(StepError::PublicationBackpressure)
    );
    assert_eq!(authority.tick_index(), 0);
    authority
        .register_peer(PEER_TWO, 1, ACTOR_TWO)
        .expect("test setup or assertion requires a value");
    drop(occupied);
    let committed = authority
        .step()
        .expect("test setup or assertion requires a value");
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
    .expect("test setup or assertion requires a value");
    probe
        .register_peer(PEER, 1, ACTOR)
        .expect("test setup or assertion requires a value");
    let first_peer_bytes = probe_pool.used_bytes();
    probe
        .register_peer(PEER_TWO, 1, ACTOR_TWO)
        .expect("test setup or assertion requires a value");
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
    .expect("test setup or assertion requires a value");
    authority
        .register_peer(PEER, 1, ACTOR)
        .expect("test setup or assertion requires a value");
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
    .expect("test setup or assertion requires a value");
    authority
        .register_peer(PEER, 1, ACTOR)
        .expect("test setup or assertion requires a value");
    authority
        .submit(input(
            1,
            1,
            InputSchedule::NextTick { observed_tick: 0 },
            Action::Collect(ITEM),
        ))
        .expect("test setup or assertion requires a value");
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
            .get(
                authority
                    .world()
                    .handle_for(ITEM)
                    .expect("test setup or assertion requires a value")
            )
            .expect("test setup or assertion requires a value")
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
                .map(|id| {
                    WorldEdit::Spawn(
                        StableId(id),
                        u64::try_from(id).expect("fixture ID fits u64"),
                    )
                })
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
    .expect("test setup or assertion requires a value");
    authority
        .register_peer(PEER, 1, ACTOR)
        .expect("test setup or assertion requires a value");
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
        .expect("test setup or assertion requires a value");
    let published = authority
        .step()
        .expect("test setup or assertion requires a value");
    assert_eq!(published.world_work.copied_pages, 3);
    assert_eq!(published.group.changes, []);
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
    .expect("test setup or assertion requires a value");
    sequential
        .register_peer(PEER, 1, ACTOR)
        .expect("test setup or assertion requires a value");
    let (mut sequential_replica, _) = Replica::from_baseline(sequential.baseline())
        .expect("test setup or assertion requires a value");
    for (sequence, action) in [(1, Action::Collect(ITEM)), (2, Action::Open(DOOR))] {
        sequential
            .submit(input(
                1,
                sequence,
                InputSchedule::NextTick { observed_tick: 0 },
                action,
            ))
            .expect("test setup or assertion requires a value");
    }
    let group = sequential
        .step()
        .expect("test setup or assertion requires a value")
        .group;
    assert_eq!(group.results.len(), 2);
    assert!(
        group
            .results
            .iter()
            .all(|result| result.disposition == ResultDisposition::Committed)
    );
    sequential_replica
        .apply(group)
        .expect("test setup or assertion requires a value");
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
    .expect("test setup or assertion requires a value");
    competing
        .register_peer(PEER, 1, ACTOR)
        .expect("test setup or assertion requires a value");
    competing
        .register_peer(PEER_TWO, 1, ACTOR_TWO)
        .expect("test setup or assertion requires a value");
    let (mut competing_replica, _) = Replica::from_baseline(competing.baseline())
        .expect("test setup or assertion requires a value");
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
            .expect("test setup or assertion requires a value");
    }
    let group = competing
        .step()
        .expect("test setup or assertion requires a value")
        .group;
    assert_eq!(group.results.len(), 2);
    assert_eq!(group.results[0].disposition, ResultDisposition::Committed);
    assert_eq!(
        group.results[1].disposition,
        ResultDisposition::Rejected(ActionRejection::Game(Rejection::AlreadyTaken))
    );
    competing_replica
        .apply(group)
        .expect("test setup or assertion requires a value");
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
    authority
        .register_peer(PEER, 1, ACTOR)
        .expect("test setup or assertion requires a value");
    authority
        .register_peer(PEER_TWO, 1, ACTOR_TWO)
        .expect("test setup or assertion requires a value");
    let (mut replica, _) = Replica::from_baseline(authority.baseline())
        .expect("test setup or assertion requires a value");

    replica
        .apply(
            authority
                .step()
                .expect("test setup or assertion requires a value")
                .group,
        )
        .expect("test setup or assertion requires a value");
    assert_eq!(replica.tick(), 1);
    let mut reordered = authority
        .step()
        .expect("test setup or assertion requires a value")
        .group;
    reordered.resolved_through.reverse();
    replica
        .apply(reordered)
        .expect("test setup or assertion requires a value");
    assert_eq!(replica.tick(), 2);

    let mut duplicate = authority
        .step()
        .expect("test setup or assertion requires a value")
        .group;
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
    replica
        .apply(valid)
        .expect("test setup or assertion requires a value");
    assert_eq!(replica.tick(), 3);
}

#[test]
fn idle_replica_does_not_skip_gap_normalization() {
    let mut authority = Authority::new(AuthorityId::Match(MatchId(202)), Rule, game_world(203));
    authority
        .register_peer(PEER, 1, ACTOR)
        .expect("test setup or assertion requires a value");
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
    let (mut replica, _) =
        Replica::from_baseline(baseline).expect("test setup or assertion requires a value");
    let stale = authority
        .step()
        .expect("test setup or assertion requires a value")
        .group;
    let mut corrected = stale.clone();
    corrected.resolved_through[0].1 = 1;
    assert!(matches!(
        replica.apply(stale),
        Err(ReplicaError::IncompleteOutcome)
    ));
    assert_eq!(replica.tick(), 0);
    replica
        .apply(corrected)
        .expect("test setup or assertion requires a value");
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
            let old = &world
                .get(handle)
                .expect("test setup or assertion requires a value")
                .state;
            let next = u64::from_le_bytes(*old.inline()) + 1;
            let state = Record::try_new(
                &self.pool,
                next.to_le_bytes(),
                [old.payload_owner(0).cloned()],
            )
            .expect("test setup or assertion requires a value");
            PreparedCommit {
                edits: PreparedEdits::One(WorldEdit::Replace(handle, state)),
                event: None,
            }
        }
    }

    let pool = PublicationPool::new(100_000);
    let payload_charge =
        PublicationBytes::required_charge(512).expect("test setup or assertion requires a value");
    let payload = PublicationBytes::try_copy_from_slice(&pool, &[9; 512])
        .expect("test setup or assertion requires a value");
    let state = Record::try_new(&pool, 4_u64.to_le_bytes(), [Some(payload.clone())])
        .expect("test setup or assertion requires a value");
    let mut world = World::new(WorldEpoch(205));
    world
        .spawn(StableId(1), state)
        .expect("test setup or assertion requires a value");
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
    .expect("test setup or assertion requires a value");
    authority
        .register_peer(PEER, 1, ACTOR)
        .expect("test setup or assertion requires a value");
    let (mut replica, _) = Replica::from_baseline(authority.baseline())
        .expect("test setup or assertion requires a value");
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
        .expect("test setup or assertion requires a value");
    let published = authority
        .step()
        .expect("test setup or assertion requires a value");
    let admitted = replica
        .apply(published.group)
        .expect("test setup or assertion requires a value");
    assert_eq!(admitted.results.len(), 1);
    assert_eq!(
        u64::from_le_bytes(
            *replica
                .get(StableId(1))
                .expect("test setup or assertion requires a value")
                .inline()
        ),
        5
    );
    assert_eq!(
        replica
            .get(StableId(1))
            .expect("test setup or assertion requires a value")
            .payload_owner(0)
            .expect("test setup or assertion requires a value")
            .as_slice(),
        &[9; 512]
    );
    assert_eq!(
        u64::from_le_bytes(
            *old_reader
                .records()
                .next()
                .expect("test setup or assertion requires a value")
                .state
                .inline()
        ),
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
#[expect(
    clippy::too_many_lines,
    reason = "the fixture verifies both initial and prepared owner provenance in one transactional scenario"
)]
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
            let foreign_payload = PublicationBytes::try_copy_from_slice(&self.foreign, &[9; 32])
                .expect("test setup or assertion requires a value");
            PreparedCommit {
                edits: PreparedEdits::Two(
                    WorldEdit::Replace(
                        handle,
                        Record::try_new(&self.foreign, 2_u64.to_le_bytes(), [None])
                            .expect("test setup or assertion requires a value"),
                    ),
                    WorldEdit::Spawn(
                        StableId(2),
                        Record::try_new(
                            &self.foreign,
                            3_u64.to_le_bytes(),
                            [Some(foreign_payload)],
                        )
                        .expect("test setup or assertion requires a value"),
                    ),
                ),
                event: None,
            }
        }
    }

    let pool = PublicationPool::new(100_000);
    let foreign = PublicationPool::new(100_000);
    let foreign_payload = PublicationBytes::try_copy_from_slice(&foreign, &[7; 32])
        .expect("test setup or assertion requires a value");
    let foreign_record = Record::try_new(&foreign, 1_u64.to_le_bytes(), [Some(foreign_payload)])
        .expect("test setup or assertion requires a value");
    let mut wrong_world = World::new(WorldEpoch(207));
    wrong_world
        .spawn(StableId(1), foreign_record)
        .expect("test setup or assertion requires a value");
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

    let oversized_payload = PublicationBytes::try_copy_from_slice(&pool, &[8; 33])
        .expect("test setup or assertion requires a value");
    let oversized = Record::try_new(&pool, 1_u64.to_le_bytes(), [Some(oversized_payload)])
        .expect("test setup or assertion requires a value");
    let mut oversized_world = World::new(WorldEpoch(209));
    oversized_world
        .spawn(StableId(1), oversized)
        .expect("test setup or assertion requires a value");
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
    let initial = Record::try_new(&pool, 1_u64.to_le_bytes(), [None])
        .expect("test setup or assertion requires a value");
    let handle = world
        .spawn(StableId(1), initial)
        .expect("test setup or assertion requires a value");
    let mut authority = Authority::try_new_closed_snapshot_charged(
        AuthorityId::Match(MatchId(208)),
        ForeignEditRule { foreign },
        world,
        pool,
        bounds,
        value_limits,
    )
    .expect("test setup or assertion requires a value");
    authority
        .register_peer(PEER, 1, ACTOR)
        .expect("test setup or assertion requires a value");
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
        .expect("test setup or assertion requires a value");
    assert!(matches!(
        authority.step(),
        Err(StepError::TerminalGameContract)
    ));
    assert_eq!(authority.tick_index(), 0);
    assert_eq!(
        u64::from_le_bytes(
            *authority
                .snapshot()
                .get(handle)
                .expect("test setup or assertion requires a value")
                .state
                .inline()
        ),
        1
    );
    assert_eq!(
        u64::from_le_bytes(
            *authority
                .world()
                .get(handle)
                .expect("test setup or assertion requires a value")
                .state
                .inline()
        ),
        1
    );
    assert!(authority.world().handle_for(StableId(2)).is_none());
    assert!(matches!(
        authority.step(),
        Err(StepError::TerminalGameContract)
    ));

    let profile_pool = PublicationPool::new(100_000);
    let mut profile_world = World::new(WorldEpoch(210));
    let initial = Record::try_new(&profile_pool, 1_u64.to_le_bytes(), [None])
        .expect("test setup or assertion requires a value");
    let handle = profile_world
        .spawn(StableId(1), initial)
        .expect("test setup or assertion requires a value");
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
    .expect("test setup or assertion requires a value");
    profile_authority
        .register_peer(PEER, 1, ACTOR)
        .expect("test setup or assertion requires a value");
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
        .expect("test setup or assertion requires a value");
    assert!(matches!(
        profile_authority.step(),
        Err(StepError::TerminalGameContract)
    ));
    assert_eq!(
        u64::from_le_bytes(
            *profile_authority
                .world()
                .get(handle)
                .expect("test setup or assertion requires a value")
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
        let payload = PublicationBytes::try_copy_from_slice(pool, &[8; 32])
            .expect("test setup or assertion requires a value");
        Output::try_new(pool, [], [Some(payload)])
            .expect("test setup or assertion requires a value")
    }

    #[test]
    fn sealed_host_delivers_two_peer_charged_group_once() {
        use std::time::{Duration, Instant};

        let pool = PublicationPool::new(100_000);
        let mut world = World::new(WorldEpoch(235));
        world
            .spawn(
                StableId(1),
                Entity::try_new(&pool, 1_u64.to_le_bytes(), [])
                    .expect("test setup or assertion requires a value"),
            )
            .expect("test setup or assertion requires a value");
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
        .expect("test setup or assertion requires a value");
        authority
            .register_peer(PEER, 1, ACTOR)
            .expect("test setup or assertion requires a value");
        authority
            .register_peer(PEER_TWO, 1, ACTOR_TWO)
            .expect("test setup or assertion requires a value");
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
        .expect("test setup or assertion requires a value");
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
            .expect("test setup or assertion requires a value");
        }
        let held = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes())
            .expect("test setup or assertion requires a value");
        let blocked = host
            .on_callback_charged(origin + Duration::from_millis(17))
            .expect("test setup or assertion requires a value");
        assert_eq!(
            blocked.clock.step_error,
            Some(StepError::PublicationBackpressure)
        );
        assert_eq!(blocked.clock.completed, 0);
        assert_eq!(host.authority().tick_index(), 0);
        assert_eq!(
            host.replica()
                .expect("test setup or assertion requires a value")
                .tick(),
            0
        );
        assert_eq!(
            host.authority()
                .world()
                .get(
                    host.authority()
                        .world()
                        .handle_for(StableId(1))
                        .expect("test setup or assertion requires a value")
                )
                .expect("test setup or assertion requires a value")
                .state
                .inline(),
            &1_u64.to_le_bytes()
        );
        drop(blocked);
        drop(held);
        let report = host
            .on_callback_charged(origin + Duration::from_millis(17))
            .expect("test setup or assertion requires a value");
        assert_eq!(report.clock.completed, 1);
        assert_eq!(report.replica_groups, 1);
        let HostPublication::ReplicatedCharged { admitted, .. } = &report.publications[0] else {
            panic!("expected charged replica publication")
        };
        assert_eq!(admitted.results().len(), 2);
        assert_eq!(admitted.events().len(), 1);
        assert_eq!(admitted.resolved_through(), &[(PEER, 1), (PEER_TWO, 1)]);
        assert_eq!(
            host.replica()
                .expect("test setup or assertion requires a value")
                .get(StableId(1))
                .expect("test setup or assertion requires a value")
                .inline(),
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
                Entity::try_new(&pool, 1_u64.to_le_bytes(), [])
                    .expect("test setup or assertion requires a value"),
            )
            .expect("test setup or assertion requires a value");
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
        .expect("test setup or assertion requires a value");
        for (peer, actor) in [(PEER, ACTOR), (PEER_TWO, ACTOR_TWO)] {
            authority
                .register_peer(peer, 1, actor)
                .expect("test setup or assertion requires a value");
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
                .expect("test setup or assertion requires a value");
        }
        drop(
            authority
                .step()
                .expect("test setup or assertion requires a value"),
        );
        let prototype = authority.baseline();
        let before = pool.used_bytes();
        let occupied = pool
            .try_reserve(pool.limit_bytes() - before)
            .expect("test setup or assertion requires a value");
        assert!(matches!(
            authority.try_baseline_charged(),
            Err(ChargedBaselineError::Capacity)
        ));
        assert_eq!(pool.used_bytes(), pool.limit_bytes());
        drop(occupied);

        let charged = authority
            .try_baseline_charged()
            .expect("test setup or assertion requires a value");
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
                Entity::try_new(&pool, 1_u64.to_le_bytes(), [])
                    .expect("test setup or assertion requires a value"),
            )
            .expect("test setup or assertion requires a value");
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
        .expect("test setup or assertion requires a value");
        for (peer, actor) in [(PEER, ACTOR), (PEER_TWO, ACTOR_TWO)] {
            authority
                .register_peer(peer, 1, actor)
                .expect("test setup or assertion requires a value");
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
                .expect("test setup or assertion requires a value");
        }
        let before_tick = authority.tick_index();
        let snapshot_peak = authority
            .world()
            .conservative_charged_publication_peak(2, 0)
            .expect("test setup or assertion requires a value");
        let occupied = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes() - snapshot_peak)
            .expect("test setup or assertion requires a value");
        assert_eq!(
            authority.step_charged().err(),
            Some(StepError::PublicationBackpressure)
        );
        assert_eq!(authority.tick_index(), before_tick);
        assert!(!authority.is_faulted());
        assert_eq!(
            authority
                .world()
                .get(
                    authority
                        .world()
                        .handle_for(StableId(1))
                        .expect("test setup or assertion requires a value")
                )
                .expect("test setup or assertion requires a value")
                .state
                .inline(),
            &1_u64.to_le_bytes()
        );
        drop(occupied);

        let tick = authority
            .step_charged()
            .expect("test setup or assertion requires a value");
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
            .expect("test setup or assertion requires a value");
        let idle = authority
            .step_charged()
            .expect("test setup or assertion requires a value");
        assert_eq!(idle.group.changes(), []);
        assert_eq!(idle.group.results(), []);
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
            .expect("test setup or assertion requires a value");
        let next = authority
            .step_charged()
            .expect("test setup or assertion requires a value");
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
        assert_eq!(
            authority
                .step()
                .expect("test setup or assertion requires a value")
                .group
                .commit_tick,
            1
        );
    }

    #[test]
    fn charged_two_peer_baseline_and_tick_admit_into_one_finite_replica() {
        let pool = PublicationPool::new(1_000_000);
        let foreign = PublicationPool::new(1_000_000);
        let mut world = World::new(WorldEpoch(234));
        world
            .spawn(
                StableId(1),
                Entity::try_new(&pool, 1_u64.to_le_bytes(), [])
                    .expect("test setup or assertion requires a value"),
            )
            .expect("test setup or assertion requires a value");
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
        .expect("test setup or assertion requires a value");
        authority
            .register_peer(PEER, 1, ACTOR)
            .expect("test setup or assertion requires a value");
        authority
            .register_peer(PEER_TWO, 1, ACTOR_TWO)
            .expect("test setup or assertion requires a value");
        let baseline = authority
            .try_baseline_charged()
            .expect("test setup or assertion requires a value");
        let prior = pool.used_bytes();
        assert!(matches!(
            Replica::from_charged_baseline(baseline.clone(), &foreign, 2),
            Err(ChargedReplicaSetupError::WrongPool)
        ));
        assert_eq!(pool.used_bytes(), prior);
        let (mut replica, initial) = Replica::from_charged_baseline(baseline, &pool, 2)
            .expect("test setup or assertion requires a value");
        assert!(
            replica
                .charged_object_bytes()
                .expect("test setup or assertion requires a value")
                > 0
        );
        assert_eq!(replica.len(), 1);
        assert_eq!(
            replica
                .get(StableId(1))
                .expect("test setup or assertion requires a value")
                .inline(),
            &1_u64.to_le_bytes()
        );
        assert_eq!(initial.results(), []);
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
                .expect("test setup or assertion requires a value");
        }
        let tick = authority
            .step_charged()
            .expect("test setup or assertion requires a value");
        let slow_reader = tick.group.clone();
        let admitted = replica
            .apply_charged(tick.group)
            .expect("test setup or assertion requires a value");
        assert_eq!(replica.tick(), 1);
        assert_eq!(
            replica
                .get(StableId(1))
                .expect("test setup or assertion requires a value")
                .inline(),
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
                Ok(world
                    .handle_for(StableId(1))
                    .expect("test setup or assertion requires a value"))
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
                    Entity::try_new(&self.entity_pool, 2_u64.to_le_bytes(), [])
                        .expect("test setup or assertion requires a value"),
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
            let initial = Entity::try_new(&pool, 1_u64.to_le_bytes(), [])
                .expect("test setup or assertion requires a value");
            let handle = world
                .spawn(StableId(1), initial)
                .expect("test setup or assertion requires a value");
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
            .expect("test setup or assertion requires a value");
            authority
                .register_peer(PEER, 1, ACTOR)
                .expect("test setup or assertion requires a value");
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
                .expect("test setup or assertion requires a value");
            assert!(matches!(
                authority.step(),
                Err(StepError::TerminalGameContract)
            ));
            assert_eq!(authority.tick_index(), 0);
            assert_eq!(
                u64::from_le_bytes(
                    *authority
                        .world()
                        .get(handle)
                        .expect("test setup or assertion requires a value")
                        .state
                        .inline()
                ),
                1
            );
            assert_eq!(foreign.used_bytes(), 0);
        }
    }

    #[test]
    fn retains_valid_outputs_until_acknowledgement_and_final_reader() {
        let pool = PublicationPool::new(100_000);
        let mut world = World::new(WorldEpoch(230));
        let initial = Entity::try_new(&pool, 1_u64.to_le_bytes(), [])
            .expect("test setup or assertion requires a value");
        world
            .spawn(StableId(1), initial)
            .expect("test setup or assertion requires a value");
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
        .expect("test setup or assertion requires a value");
        authority
            .register_peer(PEER, 1, ACTOR)
            .expect("test setup or assertion requires a value");
        let (mut replica, _) = Replica::from_baseline(authority.baseline())
            .expect("test setup or assertion requires a value");
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
        authority
            .submit(rejection_input.clone())
            .expect("test setup or assertion requires a value");
        let after_history_admission = pool.used_bytes();
        assert!(after_history_admission > idle_charge);
        let rejected = authority
            .step()
            .expect("test setup or assertion requires a value");
        let rejected_admitted = replica
            .apply(rejected.group)
            .expect("test setup or assertion requires a value");
        let ResultDisposition::Rejected(ActionRejection::Game(reason)) =
            &rejected_admitted.results[0].disposition
        else {
            panic!("closed rejection missing from replica result");
        };
        assert_eq!(
            reason
                .payload_owner(0)
                .expect("test setup or assertion requires a value")
                .as_slice(),
            &[8; 32]
        );
        let duplicate = authority
            .submit(rejection_input.clone())
            .expect("test setup or assertion requires a value");
        assert!(matches!(duplicate, Admission::ResolvedDuplicate(_)));
        let rejection_baseline = authority.baseline();
        assert_eq!(rejection_baseline.results.len(), 1);
        authority
            .acknowledge_results(PEER, 1)
            .expect("test setup or assertion requires a value");
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
            .expect("test setup or assertion requires a value");
        let committed = authority
            .step()
            .expect("test setup or assertion requires a value");
        let event_admitted = replica
            .apply(committed.group)
            .expect("test setup or assertion requires a value");
        assert_eq!(event_admitted.events.len(), 1);
        assert_eq!(
            event_admitted.events[0]
                .payload
                .payload_owner(0)
                .expect("test setup or assertion requires a value")
                .as_slice(),
            &[8; 32]
        );
        assert_eq!(
            u64::from_le_bytes(
                *replica
                    .get(StableId(1))
                    .expect("test setup or assertion requires a value")
                    .inline()
            ),
            2
        );
        let event_charge = PublicationBytes::required_charge(32)
            .expect("test setup or assertion requires a value");
        let with_event = pool.used_bytes();
        let event_baseline = authority.baseline();
        assert_eq!(event_baseline.events.len(), 1);
        authority
            .acknowledge_results(PEER, 2)
            .expect("test setup or assertion requires a value");
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
                    Entity::try_new(&self.pool, 2_u64.to_le_bytes(), [])
                        .expect("test setup or assertion requires a value"),
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
        let payload = PublicationBytes::try_copy_from_slice(pool, &vec![5; len])
            .expect("test setup or assertion requires a value");
        ActionValue::try_new(pool, [], [Some(payload)])
            .expect("test setup or assertion requires a value")
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
                Entity::try_new(&pool, 1_u64.to_le_bytes(), [])
                    .expect("test setup or assertion requires a value"),
            )
            .expect("test setup or assertion requires a value");
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
        .expect("test setup or assertion requires a value");
        authority
            .register_peer(PEER, 1, ACTOR)
            .expect("test setup or assertion requires a value");
        let (mut replica, _) = Replica::from_baseline(authority.baseline())
            .expect("test setup or assertion requires a value");
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
        let action_charge = PublicationBytes::required_charge(32)
            .expect("test setup or assertion requires a value");
        assert_eq!(
            authority
                .submit(valid.clone())
                .expect("test setup or assertion requires a value"),
            Admission::Queued
        );
        assert_eq!(
            authority
                .submit(valid.clone())
                .expect("test setup or assertion requires a value"),
            Admission::PendingDuplicate
        );
        assert_eq!(
            authority
                .submit(make_input(action(&foreign, 32)))
                .expect("test setup or assertion requires a value"),
            Admission::PendingDuplicate
        );
        assert_eq!(foreign.used_bytes(), 0);
        let committed = authority
            .step()
            .expect("test setup or assertion requires a value");
        let admitted = replica
            .apply(committed.group)
            .expect("test setup or assertion requires a value");
        assert_eq!(admitted.results.len(), 1);
        assert_eq!(
            u64::from_le_bytes(
                *replica
                    .get(StableId(1))
                    .expect("test setup or assertion requires a value")
                    .inline()
            ),
            2
        );
        let with_action = pool.used_bytes();
        authority
            .acknowledge_results(PEER, 1)
            .expect("test setup or assertion requires a value");
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
        authority
            .submit(pending)
            .expect("test setup or assertion requires a value");
        assert_eq!(pool.used_bytes(), before_pending + action_charge);
        authority
            .register_peer(PEER, 2, ACTOR)
            .expect("test setup or assertion requires a value");
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

        fn spawn_bound(&self, action: &u8, _configured_max: usize) -> usize {
            usize::from(*action == 7)
        }

        fn decide(
            &self,
            world: &World<Entity>,
            _actor: ActorId,
            action: &u8,
            context: &mut thrum_core::RuleContext<'_>,
        ) -> Result<(Handle, u8), Output> {
            if *action == 0 {
                let payload = context
                    .try_copy_payload(&[3; 16])
                    .expect("test setup or assertion requires a value");
                return Err(Output::try_new(&self.pool, [], [Some(payload)])
                    .expect("test setup or assertion requires a value"));
            }
            if *action == 6 {
                drop(
                    context
                        .try_copy_payload(&[6; 16])
                        .expect("test setup or assertion requires a value"),
                );
            }
            Ok((
                world
                    .handle_for(StableId(1))
                    .expect("test setup or assertion requires a value"),
                *action,
            ))
        }

        fn prepare(
            &self,
            _world: &World<Entity>,
            (handle, value): (Handle, u8),
            context: &mut thrum_core::RuleContext<'_>,
        ) -> PreparedCommit<Entity, Output> {
            let payload = if value == 8 {
                let scratch = context
                    .try_scratch_allocate(32)
                    .expect("test setup or assertion requires a value");
                context
                    .scratch_view_mut(scratch)
                    .expect("test setup or assertion requires a value")
                    .fill(value);
                context
                    .try_copy_scratch_payload(scratch)
                    .expect("test setup or assertion requires a value")
            } else {
                context
                    .try_copy_payload(&[value; 32])
                    .expect("test setup or assertion requires a value")
            };
            if value == 9 {
                drop(context.try_copy_payload(&[9; 32]));
            }
            PreparedCommit {
                edits: PreparedEdits::One(if value == 7 {
                    WorldEdit::Spawn(
                        StableId(2),
                        Entity::try_new(&self.pool, [value], [Some(payload.clone())])
                            .expect("test setup or assertion requires a value"),
                    )
                } else {
                    WorldEdit::Replace(
                        handle,
                        Entity::try_new(&self.pool, [value], [Some(payload.clone())])
                            .expect("test setup or assertion requires a value"),
                    )
                }),
                event: Some(
                    Output::try_new(&self.pool, [], [Some(payload)])
                        .expect("test setup or assertion requires a value"),
                ),
            }
        }
    }

    fn host_with_allowance(
        output_allowance: usize,
        structural: bool,
        scratch_capacity: usize,
    ) -> (
        HeadlessHost<Rule>,
        thrum_core::InitialChargedOutcomes<Rule>,
        PublicationPool,
        Instant,
    ) {
        let pool = PublicationPool::new(200_000);
        let mut world = World::new(WorldEpoch(260));
        world
            .spawn(
                StableId(1),
                Entity::try_new(&pool, [1], [None])
                    .expect("test setup or assertion requires a value"),
            )
            .expect("test setup or assertion requires a value");
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
                max_spawns_per_action: usize::from(structural),
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
        .expect("test setup or assertion requires a value");
        if scratch_capacity != 0 {
            authority
                .try_enable_rule_scratch(scratch_capacity)
                .expect("test setup or assertion requires a value");
        }
        authority
            .register_peer(PeerId(1), 1, ActorId(1))
            .expect("test setup or assertion requires a value");
        authority
            .register_peer(PeerId(2), 1, ActorId(2))
            .expect("test setup or assertion requires a value");
        let origin = Instant::now();
        let clock = ClockConfig::default();
        let budget = HostBudget {
            callback_cpu: Duration::from_millis(5),
            other_work: Duration::from_millis(1),
            tick_cost_bound: Duration::from_millis(1),
        };
        let (host, initial) = if structural {
            HeadlessHost::with_protected_structural_replica(
                origin,
                clock,
                budget,
                authority,
                &pool,
                2,
                2,
                2,
                2,
                output_allowance,
            )
        } else {
            HeadlessHost::with_protected_output_replica(
                origin,
                clock,
                budget,
                authority,
                &pool,
                1,
                2,
                output_allowance,
            )
        }
        .expect("test setup or assertion requires a value");
        (host, initial, pool, origin)
    }

    fn host() -> (
        HeadlessHost<Rule>,
        thrum_core::InitialChargedOutcomes<Rule>,
        PublicationPool,
        Instant,
    ) {
        host_with_allowance(
            PublicationBytes::required_charge(32)
                .expect("test setup or assertion requires a value"),
            false,
            0,
        )
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
        host.submit(input(1, 4))
            .expect("test setup or assertion requires a value");
        host.submit(input(2, 0))
            .expect("test setup or assertion requires a value");
        let occupied = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes())
            .expect("test setup or assertion requires a value");
        let report = host
            .on_callback_charged(origin + Duration::from_millis(17))
            .expect("test setup or assertion requires a value");
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
                .expect("test setup or assertion requires a value")
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
            .expect("test setup or assertion requires a value");
        assert_eq!(
            rejected
                .payload_owner(0)
                .expect("test setup or assertion requires a value")
                .as_slice(),
            &[3; 16]
        );
        assert_eq!(
            host.replica()
                .expect("test setup or assertion requires a value")
                .get(StableId(1))
                .expect("test setup or assertion requires a value")
                .payload_owner(0)
                .expect("test setup or assertion requires a value")
                .as_slice(),
            &[4; 32]
        );
        assert_eq!(
            old_snapshot
                .records()
                .find(|record| record.id == StableId(1))
                .expect("test setup or assertion requires a value")
                .state
                .inline(),
            &[1]
        );
        drop(report);
        host.acknowledge_results(PeerId(1), 1)
            .expect("test setup or assertion requires a value");
        host.acknowledge_results(PeerId(2), 1)
            .expect("test setup or assertion requires a value");
        drop(occupied);
        drop(old_snapshot);
        drop(initial);
        drop(host);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn scratch_payload_flows_through_state_event_and_replica_under_contention() {
        let allowance = PublicationBytes::required_charge(32)
            .expect("test setup or assertion requires a value");
        let (mut host, initial, pool, origin) = host_with_allowance(allowance, false, 32);
        let old_snapshot = host.authority().snapshot().clone();
        host.submit(input(1, 8))
            .expect("test setup or assertion requires a value");
        host.submit(input(2, 0))
            .expect("test setup or assertion requires a value");
        let occupied = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes())
            .expect("test setup or assertion requires a value");
        let report = host
            .on_callback_charged(origin + Duration::from_millis(17))
            .expect("test setup or assertion requires a value");
        let HostPublication::ReplicatedCharged { admitted, .. } = &report.publications[0] else {
            panic!("expected charged publication")
        };
        assert_eq!(admitted.results().len(), 2);
        assert_eq!(admitted.events().len(), 1);
        let event = admitted.events()[0]
            .payload
            .payload_owner(0)
            .expect("test setup or assertion requires a value");
        let state = host
            .replica()
            .expect("test setup or assertion requires a value")
            .get(StableId(1))
            .expect("test setup or assertion requires a value");
        assert_eq!(event.as_slice(), &[8; 32]);
        assert_eq!(state.inline(), &[8]);
        assert_eq!(
            state
                .payload_owner(0)
                .expect("test setup or assertion requires a value")
                .as_slice(),
            &[8; 32]
        );
        let rejection = admitted
            .results()
            .iter()
            .find_map(|result| match &result.disposition {
                ResultDisposition::Rejected(ActionRejection::Game(reason)) => Some(reason),
                _ => None,
            })
            .expect("test setup or assertion requires a value");
        assert_eq!(
            rejection
                .payload_owner(0)
                .expect("test setup or assertion requires a value")
                .as_slice(),
            &[3; 16]
        );
        assert_eq!(
            old_snapshot
                .records()
                .find(|record| record.id == StableId(1))
                .expect("test setup or assertion requires a value")
                .state
                .inline(),
            &[1]
        );
        drop(report);
        host.acknowledge_results(PeerId(1), 1)
            .expect("test setup or assertion requires a value");
        host.acknowledge_results(PeerId(2), 1)
            .expect("test setup or assertion requires a value");
        drop(occupied);
        drop(old_snapshot);
        drop(initial);
        drop(host);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn structural_spawn_state_event_and_peer_rejection_share_protected_pool() {
        let allowance = PublicationBytes::required_charge(32)
            .expect("test setup or assertion requires a value");
        let (mut host, initial, pool, origin) = host_with_allowance(allowance, true, 0);
        let old_snapshot = host.authority().snapshot().clone();
        host.submit(input(1, 7))
            .expect("test setup or assertion requires a value");
        host.submit(input(2, 0))
            .expect("test setup or assertion requires a value");
        let occupied = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes())
            .expect("test setup or assertion requires a value");
        let report = host
            .on_callback_charged(origin + Duration::from_millis(17))
            .expect("test setup or assertion requires a value");
        assert_eq!(report.clock.completed, 1);
        let HostPublication::ReplicatedCharged { admitted, .. } = &report.publications[0] else {
            panic!("expected charged replica publication")
        };
        assert_eq!(admitted.results().len(), 2);
        assert_eq!(admitted.events().len(), 1);
        assert_eq!(
            admitted.events()[0]
                .payload
                .payload_owner(0)
                .expect("test setup or assertion requires a value")
                .as_slice(),
            &[7; 32]
        );
        assert!(admitted.results().iter().any(|result| matches!(
            &result.disposition,
            ResultDisposition::Rejected(ActionRejection::Game(reason))
                if reason.payload_owner(0).expect("test setup or assertion requires a value").as_slice() == [3; 16]
        )));
        assert_eq!(
            host.replica()
                .expect("test setup or assertion requires a value")
                .get(StableId(2))
                .expect("test setup or assertion requires a value")
                .payload_owner(0)
                .expect("test setup or assertion requires a value")
                .as_slice(),
            &[7; 32]
        );
        assert_eq!(old_snapshot.records().count(), 1);
        drop(report);
        host.acknowledge_results(PeerId(1), 1)
            .expect("test setup or assertion requires a value");
        host.acknowledge_results(PeerId(2), 1)
            .expect("test setup or assertion requires a value");
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
        host.submit(future)
            .expect("test setup or assertion requires a value");
        let mut early = input(1, 5);
        early.origin.sequence = 2;
        host.submit(early)
            .expect("test setup or assertion requires a value");
        let occupied = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes())
            .expect("test setup or assertion requires a value");
        for (tick, expected_results, expected_value, expected_watermark) in
            [(1, 1, 5, 0), (2, 0, 5, 0), (3, 1, 4, 2)]
        {
            let report = host
                .on_callback_charged(origin + Duration::from_millis(17 * tick))
                .expect("test setup or assertion requires a value");
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
                host.replica()
                    .expect("test setup or assertion requires a value")
                    .get(StableId(1))
                    .expect("test setup or assertion requires a value")
                    .inline(),
                &[expected_value]
            );
        }
        host.acknowledge_results(PeerId(1), 2)
            .expect("test setup or assertion requires a value");
        drop(occupied);
        drop(initial);
        drop(host);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn decision_and_preparation_spend_one_cumulative_payload_allowance() {
        let allowance = PublicationBytes::required_charge(16)
            .expect("test setup or assertion requires a value")
            + PublicationBytes::required_charge(32)
                .expect("test setup or assertion requires a value");
        let (mut host, initial, pool, origin) = host_with_allowance(allowance, false, 0);
        host.submit(input(1, 6))
            .expect("test setup or assertion requires a value");
        let occupied = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes())
            .expect("test setup or assertion requires a value");
        let report = host
            .on_callback_charged(origin + Duration::from_millis(17))
            .expect("test setup or assertion requires a value");
        assert_eq!(report.clock.completed, 1);
        assert_eq!(
            host.replica()
                .expect("test setup or assertion requires a value")
                .get(StableId(1))
                .expect("test setup or assertion requires a value")
                .inline(),
            &[6]
        );
        drop(report);
        host.acknowledge_results(PeerId(1), 1)
            .expect("test setup or assertion requires a value");
        drop(occupied);
        drop(initial);
        drop(host);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn old_snapshot_keeps_variable_payload_after_replacement_and_host_drop() {
        let (mut host, initial, pool, origin) = host();
        host.submit(input(1, 4))
            .expect("test setup or assertion requires a value");
        let first = host
            .on_callback_charged(origin + Duration::from_millis(17))
            .expect("test setup or assertion requires a value");
        let HostPublication::ReplicatedCharged { snapshot, .. } = &first.publications[0] else {
            panic!("expected charged publication")
        };
        let old = snapshot.clone();
        drop(first);
        host.acknowledge_results(PeerId(1), 1)
            .expect("test setup or assertion requires a value");
        let mut next = input(1, 5);
        next.origin.sequence = 2;
        next.schedule = InputSchedule::NextTick { observed_tick: 1 };
        host.submit(next)
            .expect("test setup or assertion requires a value");
        let occupied = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes())
            .expect("test setup or assertion requires a value");
        let second = host
            .on_callback_charged(origin + Duration::from_millis(34))
            .expect("test setup or assertion requires a value");
        assert_eq!(second.clock.completed, 1);
        assert_eq!(
            old.records()
                .next()
                .expect("test setup or assertion requires a value")
                .state
                .payload_owner(0)
                .expect("test setup or assertion requires a value")
                .as_slice(),
            &[4; 32]
        );
        assert_eq!(
            host.replica()
                .expect("test setup or assertion requires a value")
                .get(StableId(1))
                .expect("test setup or assertion requires a value")
                .payload_owner(0)
                .expect("test setup or assertion requires a value")
                .as_slice(),
            &[5; 32]
        );
        drop(second);
        host.acknowledge_results(PeerId(1), 2)
            .expect("test setup or assertion requires a value");
        drop(occupied);
        drop(initial);
        drop(host);
        assert!(
            pool.used_bytes()
                >= PublicationBytes::required_charge(32)
                    .expect("test setup or assertion requires a value")
        );
        drop(old);
        assert_eq!(pool.used_bytes(), 0);
    }

    #[test]
    fn exceeded_cumulative_payload_allowance_faults_before_world_edit() {
        let (mut host, initial, pool, origin) = host();
        host.submit(input(1, 9))
            .expect("test setup or assertion requires a value");
        let occupied = pool
            .try_reserve(pool.limit_bytes() - pool.used_bytes())
            .expect("test setup or assertion requires a value");
        assert!(matches!(
            host.on_callback_charged(origin + Duration::from_millis(17)),
            Err(HostError::Authority(StepError::TerminalGameContract))
        ));
        assert_eq!(host.authority().tick_index(), 0);
        assert_eq!(
            host.replica()
                .expect("test setup or assertion requires a value")
                .get(StableId(1))
                .expect("test setup or assertion requires a value")
                .inline(),
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

#[path = "i1_game/integration_lifecycle.rs"]
mod integration_lifecycle;

#[path = "i1_game/host_clock_tests.rs"]
mod host_clock_tests;
