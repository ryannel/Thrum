//! Selected inline N1 ownership probe. This is not a wire codec or transport.

use std::{
    mem::size_of,
    time::{Duration, Instant},
};

use thrum_core::{
    ActorId, Authority, AuthorityId, EntityChange, GameRules, Handle, HeadlessHost, HostBudget,
    HostPublication, Input, InputSchedule, MatchId, Origin, PeerId, PreparedCommit, PreparedEdits,
    Replica, ResultDisposition, StableId, StateGroup, World, WorldEdit, WorldEpoch,
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct Counter(u64);

#[derive(Clone, Debug, Eq, PartialEq)]
enum Action {
    Add { object: StableId, emit: bool },
    Spawn(StableId),
    Despawn(StableId),
}

enum Change {
    Add(Handle, bool),
    Spawn(StableId),
    Despawn(Handle),
}

struct Rule;

impl GameRules for Rule {
    type Entity = Counter;
    type Action = Action;
    type Change = Change;
    type Rejection = ();
    type Event = u64;

    fn decide(
        &self,
        world: &World<Counter>,
        _actor: ActorId,
        action: &Action,
        _context: &mut thrum_core::RuleContext<'_>,
    ) -> Result<Self::Change, Self::Rejection> {
        match action {
            Action::Add { object, emit } => {
                Ok(Change::Add(world.handle_for(*object).ok_or(())?, *emit))
            }
            Action::Spawn(id) if world.handle_for(*id).is_none() => Ok(Change::Spawn(*id)),
            Action::Despawn(id) => Ok(Change::Despawn(world.handle_for(*id).ok_or(())?)),
            Action::Spawn(_) => Err(()),
        }
    }

    fn prepare(
        &self,
        world: &World<Counter>,
        change: Self::Change,
        _context: &mut thrum_core::RuleContext<'_>,
    ) -> PreparedCommit<Counter, u64> {
        match change {
            Change::Add(handle, emit) => {
                let next = world
                    .get(handle)
                    .expect("test setup or assertion requires a value")
                    .state
                    .0
                    + 1;
                PreparedCommit {
                    edits: PreparedEdits::One(WorldEdit::Replace(handle, Counter(next))),
                    event: emit.then_some(next),
                }
            }
            Change::Spawn(id) => PreparedCommit {
                edits: PreparedEdits::One(WorldEdit::Spawn(id, Counter(7))),
                event: None,
            },
            Change::Despawn(handle) => PreparedCommit {
                edits: PreparedEdits::One(WorldEdit::Despawn(handle)),
                event: None,
            },
        }
    }
}

type Projection = StateGroup<Counter, u64, ()>;

// Every retained field is selected inline data. Unlike a generic game value,
// Counter/u64/() cannot carry a Snapshot or another W1 owner behind an Arc.
fn for_peer(group: &Projection, peer: PeerId) -> Projection {
    Projection {
        authority: group.authority,
        group_id: group.group_id,
        commit_tick: group.commit_tick,
        event_cursor: group.event_cursor,
        changes: group.changes.clone(),
        results: group
            .results
            .iter()
            .filter(|result| result.origin.peer == peer)
            .cloned()
            .collect(),
        events: group
            .events
            .iter()
            .filter(|event| event.origin.peer == peer)
            .cloned()
            .collect(),
        resolved_through: group
            .resolved_through
            .iter()
            .copied()
            .filter(|(id, _)| *id == peer)
            .collect(),
    }
}

fn owned_bytes(group: &Projection) -> usize {
    group.changes.capacity() * size_of::<EntityChange<Counter>>()
        + group.results.capacity() * size_of::<thrum_core::ActionResult<()>>()
        + group.events.capacity() * size_of::<thrum_core::CommittedEvent<u64>>()
        + group.resolved_through.capacity() * size_of::<(PeerId, u64)>()
}

struct SlowOutbox {
    pending: Vec<Projection>,
    bytes: usize,
    first_enqueue: Option<Instant>,
    needs_baseline: bool,
}

impl SlowOutbox {
    const MAX_ENTRIES: usize = 12;
    const MAX_BYTES: usize = 128 * 1024;

    fn new() -> Self {
        let pending = Vec::with_capacity(Self::MAX_ENTRIES);
        Self {
            bytes: pending.capacity() * size_of::<Projection>(),
            pending,
            first_enqueue: None,
            needs_baseline: false,
        }
    }

    fn submit(&self, host: &mut HeadlessHost<Rule>, input: Input<Action>) -> Result<(), ()> {
        if self.needs_baseline {
            return Err(());
        }
        host.submit(input).map(|_| ()).map_err(|_| ())
    }

    fn offer_group(&mut self, group: &Projection, peer: PeerId, now: Instant) {
        if self.needs_baseline {
            return;
        }
        let too_old = self.first_enqueue.is_some_and(|first| {
            now.checked_duration_since(first)
                .is_some_and(|age| age > Duration::from_millis(250))
        });
        if too_old || self.pending.len() == Self::MAX_ENTRIES {
            self.refuse_replaceable(group, peer);
            return;
        }
        let projected = for_peer(group, peer);
        let bytes = owned_bytes(&projected);
        if bytes > Self::MAX_BYTES - self.bytes {
            self.refuse_replaceable(group, peer);
            return;
        }
        self.first_enqueue.get_or_insert(now);
        self.bytes += bytes;
        self.pending.push(projected);
    }

    fn refuse_replaceable(&mut self, group: &Projection, peer: PeerId) {
        // The rejected group in this probe is replaceable state only.
        assert!(
            group
                .results
                .iter()
                .all(|result| result.origin.peer != peer)
        );
        assert!(group.events.iter().all(|event| event.origin.peer != peer));
        assert!(
            group
                .changes
                .iter()
                .all(|change| matches!(change, EntityChange::Update(_)))
        );
        self.needs_baseline = true;
    }
}

fn input(peer: PeerId, sequence: u64, schedule: InputSchedule, action: Action) -> Input<Action> {
    Input {
        origin: Origin {
            peer,
            connection_epoch: 1,
            sequence,
        },
        actor: ActorId(peer.0),
        schedule,
        action,
    }
}

#[test]
fn stalled_peer_owns_complete_inline_groups_without_pinning_world_snapshots() {
    let fast = PeerId(1);
    let slow = PeerId(2);
    let mut world = World::new(WorldEpoch(1));
    world
        .spawn(StableId(1), Counter(0))
        .expect("test setup or assertion requires a value");
    world
        .spawn(StableId(2), Counter(0))
        .expect("test setup or assertion requires a value");
    let mut authority = Authority::new(AuthorityId::Match(MatchId(1)), Rule, world);
    authority
        .register_peer(fast, 1, ActorId(1))
        .expect("test setup or assertion requires a value");
    authority
        .register_peer(slow, 1, ActorId(2))
        .expect("test setup or assertion requires a value");
    let (mut fast_replica, _) = Replica::from_baseline(authority.baseline())
        .expect("test setup or assertion requires a value");
    let (mut slow_replica, _) = Replica::from_baseline(for_peer(&authority.baseline(), slow))
        .expect("test setup or assertion requires a value");
    let origin = Instant::now();
    let mut host = HeadlessHost::local_with_peer_limit(
        origin,
        thrum_core::ClockConfig::default(),
        HostBudget {
            callback_cpu: Duration::from_millis(5),
            other_work: Duration::from_millis(1),
            tick_cost_bound: Duration::from_millis(1),
        },
        authority,
        2,
    )
    .expect("test setup or assertion requires a value");
    // Sequence 1 stays pending until tick 10. Sequence 2 resolves first, so
    // the outbound owner must keep its individual result above the gap.
    let mut outbox = SlowOutbox::new();
    outbox
        .submit(
            &mut host,
            input(
                slow,
                1,
                InputSchedule::Timed { intended_tick: 10 },
                Action::Add {
                    object: StableId(2),
                    emit: false,
                },
            ),
        )
        .expect("test setup or assertion requires a value");
    outbox
        .submit(
            &mut host,
            input(
                slow,
                2,
                InputSchedule::NextTick { observed_tick: 0 },
                Action::Add {
                    object: StableId(2),
                    emit: true,
                },
            ),
        )
        .expect("test setup or assertion requires a value");
    for callback in 1..=13_u64 {
        let action = match callback {
            3 => Action::Spawn(StableId(3)),
            4 => Action::Despawn(StableId(3)),
            _ => Action::Add {
                object: StableId(1),
                emit: false,
            },
        };
        host.submit(input(
            fast,
            callback,
            InputSchedule::NextTick {
                observed_tick: host.authority().tick_index(),
            },
            action,
        ))
        .expect("test setup or assertion requires a value");
        let report = host
            .on_callback(origin + Duration::from_millis(17 * callback))
            .expect("test setup or assertion requires a value");
        for publication in &report.publications {
            let HostPublication::Local(tick) = publication else {
                panic!("selected local authority producer");
            };
            // The fast peer consumes the same complete committed group.
            let admitted = fast_replica
                .apply(tick.group.clone())
                .expect("test setup or assertion requires a value");
            if let Some((_, through)) = admitted.resolved_through.iter().find(|(id, _)| *id == fast)
            {
                host.acknowledge_results(fast, *through)
                    .expect("test setup or assertion requires a value");
            }
            outbox.offer_group(
                &tick.group,
                slow,
                origin + Duration::from_millis(17 * callback),
            );
        }
        drop(report);
    }
    assert_eq!(host.authority().tick_index(), 13);
    assert!(outbox.needs_baseline);
    assert_eq!(outbox.pending.len(), SlowOutbox::MAX_ENTRIES);
    assert!(outbox.bytes <= SlowOutbox::MAX_BYTES);
    assert_eq!(outbox.pending[0].commit_tick, 1);
    assert_eq!(outbox.pending[0].results.len(), 1);
    assert_eq!(outbox.pending[0].results[0].origin.sequence, 2);
    assert!(matches!(
        outbox.pending[0].results[0].disposition,
        ResultDisposition::Committed
    ));
    assert_eq!(outbox.pending[0].events.len(), 1);
    assert_eq!(outbox.pending[0].resolved_through, vec![(slow, 0)]);
    assert!(outbox.pending.iter().any(|group| {
        group.commit_tick == 10
            && group
                .results
                .iter()
                .any(|result| result.origin.sequence == 1)
    }));
    assert!(matches!(
        outbox.pending[2].changes[0],
        EntityChange::Spawn(_)
    ));
    assert!(matches!(
        outbox.pending[3].changes[0],
        EntityChange::Despawn(_)
    ));
    assert_eq!(fast_replica.get(StableId(1)), Some(&Counter(11)));
    assert_eq!(fast_replica.get(StableId(2)), Some(&Counter(2)));
    assert_eq!(fast_replica.get(StableId(3)), None);
    // The N1 owner refuses a new slow command before authority sequencing.
    let next_slow = input(
        slow,
        3,
        InputSchedule::NextTick { observed_tick: 13 },
        Action::Add {
            object: StableId(2),
            emit: false,
        },
    );
    assert_eq!(outbox.submit(&mut host, next_slow), Err(()));
    assert_eq!(host.authority().baseline().results.len(), 2);
    // A delayed callback still completes its due ticks for the healthy peer.
    host.submit(input(
        fast,
        14,
        InputSchedule::NextTick { observed_tick: 13 },
        Action::Add {
            object: StableId(1),
            emit: false,
        },
    ))
    .expect("test setup or assertion requires a value");
    let report = host
        .on_callback(origin + Duration::from_millis(272))
        .expect("test setup or assertion requires a value");
    assert_eq!(report.clock.completed, 3);
    for publication in &report.publications {
        let HostPublication::Local(tick) = publication else {
            unreachable!()
        };
        fast_replica
            .apply(tick.group.clone())
            .expect("test setup or assertion requires a value");
    }
    assert_eq!(fast_replica.tick(), host.authority().tick_index());
    assert_eq!(fast_replica.get(StableId(1)), Some(&Counter(12)));
    assert_eq!(outbox.pending[0].commit_tick, 1);
    // Delayed delivery of the exact retained peer projections remains an
    // atomic replica path, including the gap and lifecycle transitions.
    for group in &outbox.pending {
        slow_replica
            .apply(group.clone())
            .expect("test setup or assertion requires a value");
    }
    assert_eq!(slow_replica.tick(), 12);
    assert_eq!(slow_replica.get(StableId(1)), Some(&Counter(10)));
    assert_eq!(slow_replica.get(StableId(2)), Some(&Counter(2)));
    assert_eq!(slow_replica.get(StableId(3)), None);
    let mut age_box = SlowOutbox::new();
    let state_only = &outbox.pending[11];
    age_box.offer_group(state_only, slow, origin);
    age_box.offer_group(state_only, slow, origin + Duration::from_millis(251));
    assert!(age_box.needs_baseline);
    assert_eq!(age_box.pending.len(), 1);
}
