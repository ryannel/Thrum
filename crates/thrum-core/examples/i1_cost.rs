//! Reproducible headless I1 cost fixture. Run with:
//! `RUSTC_WRAPPER= cargo run --release --locked --offline -p thrum-core --example i1_cost`

use std::{hint::black_box, time::Instant};
use thrum_core::{
    ActorId, Authority, AuthorityId, EntityChange, GameRules, Handle, Input, InputSchedule, Origin,
    PeerId, PreparedCommit, PreparedEdits, Replica, SessionId, StableId, StateGroup, World,
    WorldEdit, WorldEpoch, WorldWork,
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct Counter(u64);

struct Rule;
impl GameRules for Rule {
    type Entity = Counter;
    type Action = StableId;
    type Change = Handle;
    type Rejection = ();
    type Event = ();
    fn decide(&self, world: &World<Counter>, _actor: ActorId, id: &StableId) -> Result<Handle, ()> {
        world.handle_for(*id).ok_or(())
    }
    fn prepare(&self, world: &World<Counter>, handle: Handle) -> PreparedCommit<Counter, ()> {
        let mut state = world.get(handle).expect("validated object").state.clone();
        state.0 += 1;
        PreparedCommit {
            edits: PreparedEdits::One(WorldEdit::Replace(handle, state)),
            event: None,
        }
    }
}

struct PairRule;
impl GameRules for PairRule {
    type Entity = Counter;
    type Action = StableId;
    type Change = (Handle, Handle);
    type Rejection = ();
    type Event = ();
    fn decide(
        &self,
        world: &World<Counter>,
        _actor: ActorId,
        id: &StableId,
    ) -> Result<Self::Change, ()> {
        let next = StableId(id.0 % 64 + 1);
        Ok((
            world.handle_for(*id).ok_or(())?,
            world.handle_for(next).ok_or(())?,
        ))
    }
    fn prepare(
        &self,
        world: &World<Counter>,
        (first, second): Self::Change,
    ) -> PreparedCommit<Counter, ()> {
        let mut first_state = world.get(first).expect("validated object").state.clone();
        let mut second_state = world.get(second).expect("validated object").state.clone();
        first_state.0 += 1;
        second_state.0 += 1;
        PreparedCommit {
            edits: PreparedEdits::Two(
                WorldEdit::Replace(first, first_state),
                WorldEdit::Replace(second, second_state),
            ),
            event: None,
        }
    }
}

fn percentile(samples: &mut [u128], percent: usize) -> u128 {
    samples.sort_unstable();
    samples[(samples.len() - 1) * percent / 100]
}

fn report(name: &str, samples: &mut [u128], work: WorldWork, snapshot_bytes: usize, ticks: usize) {
    let p50 = percentile(samples, 50);
    let p95 = percentile(samples, 95);
    let p99 = percentile(samples, 99);
    println!(
        "{name}: ticks={ticks} ns[p50/p95/p99/max]={p50}/{p95}/{p99}/{} refs/tick={} pages/tick={} objects/tick={} group_objects/tick={} logical_snapshot_bytes={snapshot_bytes}",
        samples[samples.len() - 1],
        work.page_references / ticks,
        work.copied_pages / ticks,
        work.copied_objects / ticks,
        work.group_objects / ticks
    );
}

fn benchmark_authority() {
    const TICKS: usize = 10_000;
    const WARMUP: usize = 1_000;
    let mut world = World::new(WorldEpoch(1));
    for id in 1..=64 {
        world.spawn(StableId(id), Counter(0)).unwrap();
    }
    let mut authority = Authority::new(AuthorityId::Standalone(SessionId(1)), Rule, world);
    authority.register_peer(PeerId(1), 1, ActorId(1)).unwrap();
    let (mut replica, _) = Replica::from_baseline(authority.baseline()).unwrap();
    let mut admit_ns = Vec::with_capacity(TICKS);
    let mut tick_ns = Vec::with_capacity(TICKS);
    let mut replica_ns = Vec::with_capacity(TICKS);
    let mut complete_ns = Vec::with_capacity(TICKS);
    let mut work = WorldWork::default();
    let mut snapshot_bytes = 0;
    for i in 0..TICKS + WARMUP {
        let seq = (i + 1) as u64;
        let input = Input {
            origin: Origin {
                peer: PeerId(1),
                connection_epoch: 1,
                sequence: seq,
            },
            actor: ActorId(1),
            schedule: InputSchedule::NextTick {
                observed_tick: authority.tick_index(),
            },
            action: StableId((i % 64 + 1) as u128),
        };
        let complete_start = Instant::now();
        let start = Instant::now();
        black_box(authority.submit(input).unwrap());
        let admitted = start.elapsed().as_nanos();
        let start = Instant::now();
        let publication = authority.step().unwrap();
        snapshot_bytes = publication.snapshot.logical_bytes();
        let ticked = start.elapsed().as_nanos();
        let start = Instant::now();
        let result = replica.apply(publication.group).unwrap();
        authority
            .acknowledge_results(PeerId(1), result.resolved_through[0].1)
            .unwrap();
        let replicated = start.elapsed().as_nanos();
        let completed = complete_start.elapsed().as_nanos();
        if i >= WARMUP {
            admit_ns.push(admitted);
            tick_ns.push(ticked);
            replica_ns.push(replicated);
            complete_ns.push(completed);
            work.page_references += publication.world_work.page_references;
            work.copied_pages += publication.world_work.copied_pages;
            work.copied_objects += publication.world_work.copied_objects;
            work.group_objects += publication.world_work.group_objects;
        }
    }
    assert_eq!(replica.tick(), (TICKS + WARMUP) as u64);
    for id in 1..=64 {
        let id = StableId(id);
        let authority_value = &authority
            .world()
            .get(authority.world().handle_for(id).unwrap())
            .unwrap()
            .state;
        assert_eq!(replica.get(id), Some(authority_value));
    }
    report(
        "authority/admit",
        &mut admit_ns,
        WorldWork::default(),
        0,
        TICKS,
    );
    report("authority/tick", &mut tick_ns, work, snapshot_bytes, TICKS);
    report(
        "authority/replica+ack",
        &mut replica_ns,
        WorldWork::default(),
        0,
        TICKS,
    );
    report(
        "authority/complete",
        &mut complete_ns,
        WorldWork::default(),
        0,
        TICKS,
    );
}

fn benchmark_pair_authority() {
    const TICKS: usize = 10_000;
    const WARMUP: usize = 1_000;
    let mut world = World::new(WorldEpoch(2));
    for id in 1..=64 {
        world.spawn(StableId(id), Counter(0)).unwrap();
    }
    let mut authority = Authority::new(AuthorityId::Standalone(SessionId(2)), PairRule, world);
    authority.register_peer(PeerId(1), 1, ActorId(1)).unwrap();
    let (mut replica, _) = Replica::from_baseline(authority.baseline()).unwrap();
    let mut tick_ns = Vec::with_capacity(TICKS);
    let mut complete_ns = Vec::with_capacity(TICKS);
    let mut work = WorldWork::default();
    let mut snapshot_bytes = 0;
    for i in 0..TICKS + WARMUP {
        let start = Instant::now();
        let input = Input {
            origin: Origin {
                peer: PeerId(1),
                connection_epoch: 1,
                sequence: (i + 1) as u64,
            },
            actor: ActorId(1),
            schedule: InputSchedule::NextTick {
                observed_tick: authority.tick_index(),
            },
            action: StableId((i % 64 + 1) as u128),
        };
        black_box(authority.submit(input).unwrap());
        let tick_start = Instant::now();
        let publication = authority.step().unwrap();
        let ticked = tick_start.elapsed().as_nanos();
        snapshot_bytes = publication.snapshot.logical_bytes();
        let result = replica.apply(publication.group).unwrap();
        authority
            .acknowledge_results(PeerId(1), result.resolved_through[0].1)
            .unwrap();
        if i >= WARMUP {
            tick_ns.push(ticked);
            complete_ns.push(start.elapsed().as_nanos());
            work.page_references += publication.world_work.page_references;
            work.copied_pages += publication.world_work.copied_pages;
            work.copied_objects += publication.world_work.copied_objects;
            work.group_objects += publication.world_work.group_objects;
        }
    }
    assert_eq!(replica.tick(), (TICKS + WARMUP) as u64);
    for id in 1..=64 {
        let id = StableId(id);
        let authoritative = &authority
            .world()
            .get(authority.world().handle_for(id).unwrap())
            .unwrap()
            .state;
        assert_eq!(replica.get(id), Some(authoritative));
    }
    report(
        "authority/two-edit-tick",
        &mut tick_ns,
        work,
        snapshot_bytes,
        TICKS,
    );
    report(
        "authority/two-edit-complete",
        &mut complete_ns,
        WorldWork::default(),
        0,
        TICKS,
    );
}

fn benchmark_churn(count: usize, percent: usize) {
    const TICKS: usize = 5_000;
    const WARMUP: usize = 500;
    let mut world = World::new(WorldEpoch(count as u128));
    for id in 1..=count {
        world.spawn(StableId(id as u128), Counter(0)).unwrap();
    }
    let baseline = world.publish(0).snapshot;
    let initial: StateGroup<Counter, (), ()> = StateGroup {
        authority: AuthorityId::Standalone(SessionId(count as u128)),
        group_id: 0,
        commit_tick: 0,
        event_cursor: 0,
        changes: baseline
            .records()
            .cloned()
            .map(EntityChange::Spawn)
            .collect(),
        results: vec![],
        events: vec![],
        resolved_through: vec![],
    };
    let (mut replica, _) = Replica::from_baseline(initial).unwrap();
    let changes = if percent == 0 {
        0
    } else {
        (count * percent).div_ceil(100)
    };
    let mut access_ns = Vec::with_capacity(TICKS);
    let mut publication_ns = Vec::with_capacity(TICKS);
    let mut replica_ns = Vec::with_capacity(TICKS);
    let mut work = WorldWork::default();
    let mut prior = baseline;
    for tick in 1..=TICKS + WARMUP {
        let start = Instant::now();
        for j in 0..changes {
            // Full-period stride spreads small churn over pages, then rotates it.
            let id = 1 + ((j * count / changes.max(1) + tick) % count);
            let handle = world.handle_for(StableId(id as u128)).unwrap();
            world.get_mut(handle).unwrap().0 += 1;
        }
        let accessed = start.elapsed().as_nanos();
        let start = Instant::now();
        let published = world.publish(tick as u64);
        let publication = start.elapsed().as_nanos();
        let start = Instant::now();
        let group: StateGroup<Counter, (), ()> = StateGroup {
            authority: AuthorityId::Standalone(SessionId(count as u128)),
            group_id: tick as u64,
            commit_tick: tick as u64,
            event_cursor: 0,
            changes: published.changes,
            results: vec![],
            events: vec![],
            resolved_through: vec![],
        };
        replica.apply(group).unwrap();
        let replicated = start.elapsed().as_nanos();
        black_box(prior.get(Handle {
            epoch: WorldEpoch(count as u128),
            slot: 0,
            generation: 0,
        }));
        prior = published.snapshot;
        if tick > WARMUP {
            access_ns.push(accessed);
            publication_ns.push(publication);
            replica_ns.push(replicated);
            work.page_references += published.work.page_references;
            work.copied_pages += published.work.copied_pages;
            work.copied_objects += published.work.copied_objects;
            work.group_objects += published.work.group_objects;
        }
    }
    assert_eq!(replica.tick(), (TICKS + WARMUP) as u64);
    assert_eq!(replica.len(), count);
    for id in 1..=count {
        let id = StableId(id as u128);
        let authority_value = &world.get(world.handle_for(id).unwrap()).unwrap().state;
        assert_eq!(replica.get(id), Some(authority_value));
    }
    let prefix = format!("world/{count}/{percent}%");
    report(
        &format!("{prefix}/access"),
        &mut access_ns,
        WorldWork::default(),
        0,
        TICKS,
    );
    report(
        &format!("{prefix}/publish"),
        &mut publication_ns,
        work,
        prior.logical_bytes(),
        TICKS,
    );
    report(
        &format!("{prefix}/replica"),
        &mut replica_ns,
        WorldWork::default(),
        0,
        TICKS,
    );
}

fn main() {
    println!(
        "I1 headless fixture thrum-core={} host={}/{} profile=release",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    benchmark_authority();
    benchmark_pair_authority();
    for count in [64, 4_096, 16_384] {
        for percent in [0, 1, 25] {
            benchmark_churn(count, percent);
        }
    }
}
