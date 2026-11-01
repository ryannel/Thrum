//! Sealed baseline admission with 256 retained action results.
//! `RUSTC_WRAPPER= cargo run --release --locked --offline -p thrum-core --example i1_baseline_results_cost`

use std::{hint::black_box, time::Instant};
use thrum_core::{
    ActorId, Authority, AuthorityId, ClosedGameLimits, ClosedValueLimits, GameRules, Input,
    InputSchedule, MatchId, Origin, PeerId, PreparedCommit, PublicationPool, Replica,
    SnapshotWriteBounds, World, WorldEpoch,
};

struct Reject;

impl GameRules for Reject {
    type Entity = ();
    type Action = ();
    type Change = ();
    type Rejection = ();
    type Event = ();

    fn decide(
        &self,
        _world: &World<()>,
        _actor: ActorId,
        _action: &(),
        _context: &mut thrum_core::RuleContext<'_>,
    ) -> Result<(), ()> {
        Err(())
    }

    fn prepare(
        &self,
        _world: &World<()>,
        _change: (),
        _context: &mut thrum_core::RuleContext<'_>,
    ) -> PreparedCommit<(), ()> {
        unreachable!("rejected action has no edit")
    }
}

fn summary(times: &mut [u128]) -> (u128, u128, u128) {
    times.sort_unstable();
    let last = times.len() - 1;
    (
        times[last / 2],
        times[last * 95 / 100],
        times[last * 99 / 100],
    )
}

fn main() {
    const RESULTS: u64 = 256;
    const WARMUP: usize = 100;
    const SAMPLES: usize = 1_000;
    let pool = PublicationPool::new(usize::MAX);
    let zero = ClosedValueLimits {
        max_inline_bytes: 0,
        max_payload_slots: 0,
        max_payload_bytes: 0,
    };
    let mut authority = Authority::try_new_closed_snapshot_charged(
        AuthorityId::Match(MatchId(412)),
        Reject,
        World::new(WorldEpoch(412)),
        pool.clone(),
        SnapshotWriteBounds {
            max_edits_per_action: 0,
            max_spawns_per_action: 0,
        },
        ClosedGameLimits {
            action: zero,
            entity: zero,
            event: zero,
            rejection: zero,
        },
    )
    .unwrap();
    authority.register_peer(PeerId(1), 1, ActorId(1)).unwrap();
    for sequence in 1..=RESULTS {
        let tick = authority.tick_index();
        authority
            .submit(Input {
                origin: Origin {
                    peer: PeerId(1),
                    connection_epoch: 1,
                    sequence,
                },
                actor: ActorId(1),
                schedule: InputSchedule::NextTick {
                    observed_tick: tick,
                },
                action: (),
            })
            .unwrap();
        drop(authority.step_charged().unwrap());
    }
    let baseline = authority.try_baseline_charged().unwrap();
    assert_eq!(baseline.results().len(), RESULTS as usize);
    let mut times = Vec::with_capacity(SAMPLES);
    for sample in 0..WARMUP + SAMPLES {
        let start = Instant::now();
        black_box(Replica::from_charged_baseline(baseline.clone(), &pool, 0).unwrap());
        if sample >= WARMUP {
            times.push(start.elapsed().as_nanos());
        }
    }
    println!(
        "baseline_results/{RESULTS}: charged_replica_admit_ns[p50/p95/p99]={:?} host={}/{} profile=release",
        summary(&mut times),
        std::env::consts::OS,
        std::env::consts::ARCH,
    );
}
