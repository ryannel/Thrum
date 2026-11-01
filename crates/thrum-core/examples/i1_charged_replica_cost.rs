//! Local changed-tick authority and replica comparison. Run with:
//! `RUSTC_WRAPPER= cargo run --release --locked --offline -p thrum-core --example i1_charged_replica_cost`

use std::{
    hint::black_box,
    time::{Duration, Instant},
};
use thrum_core::{
    ActorId, Authority, AuthorityId, ClockConfig, ClosedGameLimits, ClosedValueLimits, GameRules,
    Handle, HeadlessHost, HostBudget, HostPublication, Input, InputSchedule, MatchId, Origin,
    PeerId, PreparedCommit, PreparedEdits, PublicationPool, PublicationRecord, Replica,
    SnapshotWriteBounds, StableId, World, WorldEdit, WorldEpoch,
};

type State = PublicationRecord<8, 0>;

struct Increment {
    pool: PublicationPool,
}

impl GameRules for Increment {
    type Entity = State;
    type Action = StableId;
    type Change = Handle;
    type Rejection = ();
    type Event = ();

    fn decide(&self, world: &World<State>, _actor: ActorId, id: &StableId) -> Result<Handle, ()> {
        world.handle_for(*id).ok_or(())
    }

    fn prepare(&self, world: &World<State>, handle: Handle) -> PreparedCommit<State, ()> {
        let old = world.get(handle).expect("validated handle");
        let next = u64::from_le_bytes(*old.state.inline()) + 1;
        PreparedCommit {
            edits: PreparedEdits::One(WorldEdit::Replace(
                handle,
                State::try_new(&self.pool, next.to_le_bytes(), []).unwrap(),
            )),
            event: None,
        }
    }
}

fn authority(count: usize, pool: &PublicationPool, match_id: u128) -> Authority<Increment> {
    let mut world = World::new(WorldEpoch(match_id));
    for id in 1..=count {
        world
            .spawn(
                StableId(id as u128),
                State::try_new(pool, 0_u64.to_le_bytes(), []).unwrap(),
            )
            .unwrap();
    }
    let mut authority = Authority::try_new_closed_snapshot_charged(
        AuthorityId::Match(MatchId(match_id)),
        Increment { pool: pool.clone() },
        world,
        pool.clone(),
        SnapshotWriteBounds {
            max_edits_per_action: 1,
            max_spawns_per_action: 0,
        },
        ClosedGameLimits {
            action: ClosedValueLimits {
                max_inline_bytes: 16,
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
                max_payload_slots: 0,
                max_payload_bytes: 0,
            },
            rejection: ClosedValueLimits {
                max_inline_bytes: 0,
                max_payload_slots: 0,
                max_payload_bytes: 0,
            },
        },
    )
    .unwrap();
    authority.register_peer(PeerId(1), 1, ActorId(1)).unwrap();
    authority.register_peer(PeerId(2), 1, ActorId(2)).unwrap();
    authority
}

fn input(tick: u64, count: usize) -> Input<StableId> {
    Input {
        origin: Origin {
            peer: PeerId(1),
            connection_epoch: 1,
            sequence: tick + 1,
        },
        actor: ActorId(1),
        schedule: InputSchedule::NextTick {
            observed_tick: tick,
        },
        action: StableId((tick as usize % count + 1) as u128),
    }
}

fn summary(times: &mut [u128]) -> (u128, u128, u128, u128) {
    times.sort_unstable();
    let last = times.len() - 1;
    (
        times[last / 2],
        times[last * 95 / 100],
        times[last * 99 / 100],
        times[last],
    )
}

fn run(count: usize) {
    const WARMUP: usize = 1_000;
    const SAMPLES: usize = 10_000;
    let pool = PublicationPool::new(usize::MAX);
    let mut authority = authority(count, &pool, 10);
    let (mut prototype, _) = Replica::from_baseline(authority.baseline()).unwrap();
    let mut prototype_times = Vec::with_capacity(SAMPLES);
    let mut prototype_steps = Vec::with_capacity(SAMPLES);
    let mut prototype_replica = Vec::with_capacity(SAMPLES);
    for sample in 0..WARMUP + SAMPLES {
        let tick = authority.tick_index();
        let start = Instant::now();
        black_box(authority.submit(input(tick, count)).unwrap());
        let step_start = Instant::now();
        let publication = authority.step().unwrap();
        let step_time = step_start.elapsed().as_nanos();
        let replica_start = Instant::now();
        let outcome = prototype.apply(publication.group).unwrap();
        authority
            .acknowledge_results(PeerId(1), outcome.resolved_through[0].1)
            .unwrap();
        let replica_time = replica_start.elapsed().as_nanos();
        let total_time = start.elapsed().as_nanos();
        if sample >= WARMUP {
            prototype_times.push(total_time);
            prototype_steps.push(step_time);
            prototype_replica.push(replica_time);
        }
    }
    let (mut charged, _) =
        Replica::from_charged_baseline(authority.try_baseline_charged().unwrap(), &pool, count)
            .unwrap();
    let mut charged_times = Vec::with_capacity(SAMPLES);
    let mut charged_steps = Vec::with_capacity(SAMPLES);
    let mut charged_replica = Vec::with_capacity(SAMPLES);
    for sample in 0..WARMUP + SAMPLES {
        let tick = authority.tick_index();
        let start = Instant::now();
        black_box(authority.submit(input(tick, count)).unwrap());
        let step_start = Instant::now();
        let publication = authority.step_charged().unwrap();
        let step_time = step_start.elapsed().as_nanos();
        let replica_start = Instant::now();
        let outcome = charged.apply_charged(publication.group).unwrap();
        authority
            .acknowledge_results(PeerId(1), outcome.resolved_through()[0].1)
            .unwrap();
        let replica_time = replica_start.elapsed().as_nanos();
        let total_time = start.elapsed().as_nanos();
        if sample >= WARMUP {
            charged_times.push(total_time);
            charged_steps.push(step_time);
            charged_replica.push(replica_time);
        }
    }
    for id in 1..=count {
        let id = StableId(id as u128);
        let record = &authority
            .world()
            .get(authority.world().handle_for(id).unwrap())
            .unwrap()
            .state;
        assert_eq!(charged.get(id), Some(record));
    }
    println!(
        "changed_tick/{count}: submit_step_replica_ack_ns[p50/p95/p99/max] prototype={:?} charged={:?} charged_index_bytes={}",
        summary(&mut prototype_times),
        summary(&mut charged_times),
        charged.charged_object_bytes().unwrap(),
    );
    println!(
        "changed_stages/{count}: step_ns[p50/p95/p99/max] prototype={:?} charged={:?} replica_ack_ns[p50/p95/p99/max] prototype={:?} charged={:?}",
        summary(&mut prototype_steps),
        summary(&mut charged_steps),
        summary(&mut prototype_replica),
        summary(&mut charged_replica),
    );
}

fn run_host(count: usize) {
    const WARMUP: usize = 1_000;
    const SAMPLES: usize = 10_000;
    let origin = Instant::now();
    let budget = HostBudget {
        // A deliberately loose budget keeps OS scheduling spikes from
        // changing the service path during a microbenchmark. The separate
        // host fixture tests real overload policy.
        callback_cpu: Duration::from_secs(1),
        other_work: Duration::ZERO,
        tick_cost_bound: Duration::from_millis(1),
    };
    let prototype_pool = PublicationPool::new(usize::MAX);
    let (mut prototype, _) = HeadlessHost::with_replica(
        origin,
        ClockConfig::default(),
        budget,
        authority(count, &prototype_pool, 20),
    )
    .unwrap();
    let mut prototype_times = Vec::with_capacity(SAMPLES);
    for sample in 0..WARMUP + SAMPLES {
        let tick = prototype.authority().tick_index();
        let now = origin + Duration::from_nanos(16_666_667 * (sample as u64 + 1));
        let start = Instant::now();
        black_box(prototype.submit(input(tick, count)).unwrap());
        let report = prototype.on_callback(now).unwrap();
        assert_eq!(report.clock.completed, 1);
        let HostPublication::Replicated { admitted, .. } = &report.publications[0] else {
            panic!("prototype host publication")
        };
        let through = admitted.resolved_through[0].1;
        drop(report);
        prototype.acknowledge_results(PeerId(1), through).unwrap();
        let elapsed = start.elapsed().as_nanos();
        if sample >= WARMUP {
            prototype_times.push(elapsed);
        }
    }
    let charged_pool = PublicationPool::new(usize::MAX);
    let (mut charged, _) = HeadlessHost::with_charged_replica(
        origin,
        ClockConfig::default(),
        budget,
        authority(count, &charged_pool, 21),
        &charged_pool,
        count,
    )
    .unwrap();
    let mut charged_times = Vec::with_capacity(SAMPLES);
    for sample in 0..WARMUP + SAMPLES {
        let tick = charged.authority().tick_index();
        let now = origin + Duration::from_nanos(16_666_667 * (sample as u64 + 1));
        let start = Instant::now();
        black_box(charged.submit(input(tick, count)).unwrap());
        let report = charged.on_callback_charged(now).unwrap();
        assert_eq!(report.clock.completed, 1);
        let HostPublication::ReplicatedCharged { admitted, .. } = &report.publications[0] else {
            panic!("charged host publication")
        };
        let through = admitted.resolved_through()[0].1;
        drop(report);
        charged.acknowledge_results(PeerId(1), through).unwrap();
        let elapsed = start.elapsed().as_nanos();
        if sample >= WARMUP {
            charged_times.push(elapsed);
        }
    }
    let protected_pool = PublicationPool::new(usize::MAX);
    let (mut protected, _) = HeadlessHost::with_protected_inline_replica(
        origin,
        ClockConfig::default(),
        budget,
        authority(count, &protected_pool, 22),
        &protected_pool,
        count,
        2,
    )
    .unwrap();
    let mut protected_times = Vec::with_capacity(SAMPLES);
    for sample in 0..WARMUP + SAMPLES {
        let tick = protected.authority().tick_index();
        let now = origin + Duration::from_nanos(16_666_667 * (sample as u64 + 1));
        let start = Instant::now();
        black_box(protected.submit(input(tick, count)).unwrap());
        let report = protected.on_callback_charged(now).unwrap();
        assert_eq!(report.clock.completed, 1);
        let HostPublication::ReplicatedCharged { admitted, .. } = &report.publications[0] else {
            panic!("protected host publication")
        };
        let through = admitted.resolved_through()[0].1;
        drop(report);
        protected.acknowledge_results(PeerId(1), through).unwrap();
        let elapsed = start.elapsed().as_nanos();
        if sample >= WARMUP {
            protected_times.push(elapsed);
        }
    }
    println!(
        "host_changed_tick/{count}: submit_callback_ack_ns[p50/p95/p99/max] prototype={:?} charged={:?} protected={:?}",
        summary(&mut prototype_times),
        summary(&mut charged_times),
        summary(&mut protected_times),
    );
}

fn main() {
    println!(
        "I1 changed replica fixture host={}/{} profile=release",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    for count in [64, 4_096] {
        run(count);
        run_host(count);
    }
}
