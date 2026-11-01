//! Local qualified baseline construction and clone cost. Run with:
//! `RUSTC_WRAPPER= cargo run --release --locked --offline -p thrum-core --example i1_baseline_cost`

use std::{hint::black_box, time::Instant};
use thrum_core::{
    ActorId, Authority, AuthorityId, ClosedGameLimits, ClosedValueLimits, GameRules, MatchId,
    PeerId, PreparedCommit, PublicationBytes, PublicationPool, PublicationRecord, Replica,
    SnapshotWriteBounds, StableId, World, WorldEpoch,
};

type State = PublicationRecord<8, 1>;

struct Reject;

impl GameRules for Reject {
    type Entity = State;
    type Action = ();
    type Change = ();
    type Rejection = ();
    type Event = ();

    fn decide(&self, _world: &World<State>, _actor: ActorId, _action: &()) -> Result<(), ()> {
        Err(())
    }

    fn prepare(&self, _world: &World<State>, _change: ()) -> PreparedCommit<State, ()> {
        unreachable!("rejected action has no prepared edit")
    }
}

fn percentile(samples: &mut [u128], percent: usize) -> u128 {
    samples.sort_unstable();
    samples[(samples.len() - 1) * percent / 100]
}

fn measure(mut operation: impl FnMut(), batch: usize) -> (u128, u128, u128) {
    const WARMUP: usize = 20;
    const SAMPLES: usize = 100;
    let mut times = Vec::with_capacity(SAMPLES);
    for sample in 0..WARMUP + SAMPLES {
        let start = Instant::now();
        for _ in 0..batch {
            operation();
        }
        if sample >= WARMUP {
            times.push(start.elapsed().as_nanos());
        }
    }
    (
        percentile(&mut times, 50),
        percentile(&mut times, 95),
        percentile(&mut times, 99),
    )
}

fn run(count: usize) {
    let pool = PublicationPool::new(usize::MAX);
    let payload = PublicationBytes::try_copy_from_slice(&pool, &[3; 512]).unwrap();
    let mut world = World::new(WorldEpoch(1));
    for index in 0..count {
        world
            .spawn(
                StableId(index as u128 + 1),
                State::try_new(&pool, (index as u64).to_le_bytes(), [Some(payload.clone())])
                    .unwrap(),
            )
            .unwrap();
    }
    let mut authority = Authority::try_new_closed_snapshot_charged(
        AuthorityId::Match(MatchId(1)),
        Reject,
        world,
        pool.clone(),
        SnapshotWriteBounds {
            max_edits_per_action: 0,
            max_spawns_per_action: 0,
        },
        ClosedGameLimits {
            action: ClosedValueLimits {
                max_inline_bytes: 0,
                max_payload_slots: 0,
                max_payload_bytes: 0,
            },
            entity: ClosedValueLimits {
                max_inline_bytes: 8,
                max_payload_slots: 1,
                max_payload_bytes: 512,
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
    let baseline_charge = pool.used_bytes();
    let prototype_build = measure(
        || {
            black_box(authority.baseline());
        },
        1,
    );
    let charged_build = measure(
        || {
            black_box(authority.try_baseline_charged().unwrap());
        },
        1,
    );
    let prototype_replica_build = measure(
        || {
            black_box(Replica::from_baseline(authority.baseline()).unwrap());
        },
        1,
    );
    let charged_replica_build = measure(
        || {
            black_box(
                Replica::from_charged_baseline(
                    authority.try_baseline_charged().unwrap(),
                    &pool,
                    count,
                )
                .unwrap(),
            );
        },
        1,
    );
    assert_eq!(pool.used_bytes(), baseline_charge);
    let prototype = authority.baseline();
    let charged = authority.try_baseline_charged().unwrap();
    assert_eq!(charged.changes(), prototype.changes);
    let prototype_clone = measure(
        || {
            black_box(black_box(&prototype).clone());
        },
        64,
    );
    let charged_clone = measure(
        || {
            black_box(black_box(&charged).clone());
        },
        64,
    );
    println!(
        "baseline/{count}: build_ns[p50/p95/p99] prototype={}/{}/{} charged={}/{}/{} clone_batch_64_ns[p50/p95/p99] prototype={}/{}/{} charged={}/{}/{}",
        prototype_build.0,
        prototype_build.1,
        prototype_build.2,
        charged_build.0,
        charged_build.1,
        charged_build.2,
        prototype_clone.0,
        prototype_clone.1,
        prototype_clone.2,
        charged_clone.0,
        charged_clone.1,
        charged_clone.2,
    );
    println!(
        "baseline_replica/{count}: build_admit_ns[p50/p95/p99] prototype={}/{}/{} charged={}/{}/{}",
        prototype_replica_build.0,
        prototype_replica_build.1,
        prototype_replica_build.2,
        charged_replica_build.0,
        charged_replica_build.1,
        charged_replica_build.2,
    );
    let prototype_idle = measure(
        || {
            black_box(authority.step().unwrap());
        },
        1_000,
    );
    let charged_idle = measure(
        || {
            black_box(authority.step_charged().unwrap());
        },
        1_000,
    );
    println!(
        "idle_tick/{count}: batch_1000_ns[p50/p95/p99] prototype={}/{}/{} charged={}/{}/{}",
        prototype_idle.0,
        prototype_idle.1,
        prototype_idle.2,
        charged_idle.0,
        charged_idle.1,
        charged_idle.2,
    );
    let (mut prototype_replica, _) = Replica::from_baseline(authority.baseline()).unwrap();
    let prototype_path = measure(
        || {
            let tick = authority.step().unwrap();
            black_box(prototype_replica.apply(tick.group).unwrap());
        },
        1_000,
    );
    let (mut charged_replica, _) =
        Replica::from_charged_baseline(authority.try_baseline_charged().unwrap(), &pool, count)
            .unwrap();
    let charged_index_bytes = charged_replica.charged_object_bytes().unwrap();
    let charged_path = measure(
        || {
            let tick = authority.step_charged().unwrap();
            black_box(charged_replica.apply_charged(tick.group).unwrap());
        },
        1_000,
    );
    println!(
        "idle_authority_replica/{count}: batch_1000_ns[p50/p95/p99] prototype={}/{}/{} charged={}/{}/{} charged_index_bytes={}",
        prototype_path.0,
        prototype_path.1,
        prototype_path.2,
        charged_path.0,
        charged_path.1,
        charged_path.2,
        charged_index_bytes,
    );
}

fn main() {
    println!(
        "I1 baseline fixture host={}/{} profile=release",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    for count in [64, 4_096] {
        run(count);
    }
}
