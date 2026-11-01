//! Local pending-input removal cost fixture. Run with:
//! `RUSTC_WRAPPER= cargo run --release --locked --offline -p thrum-core --example i1_pending_cost`

use std::{hint::black_box, time::Instant};
use thrum_core::{
    ActorId, Authority, AuthorityId, GameRules, Input, InputSchedule, MatchId, Origin, PeerId,
    PreparedCommit, World, WorldEpoch,
};

struct Reject<const N: usize>;

impl<const N: usize> GameRules for Reject<N> {
    type Entity = ();
    type Action = [u8; N];
    type Change = ();
    type Rejection = ();
    type Event = ();

    fn decide(
        &self,
        _world: &World<()>,
        _actor: ActorId,
        _action: &[u8; N],
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
        unreachable!("rejected action has no prepared edit")
    }
}

fn percentile(samples: &mut [u128], percent: usize) -> u128 {
    samples.sort_unstable();
    samples[(samples.len() - 1) * percent / 100]
}

fn run<const N: usize>() {
    const BATCH: usize = 128;
    const WARMUP: usize = 10;
    const SAMPLES: usize = 100;
    let mut times = Vec::with_capacity(SAMPLES);
    for sample in 0..WARMUP + SAMPLES {
        let mut prepared = Vec::with_capacity(BATCH);
        for index in 0..BATCH {
            let id = (sample * BATCH + index) as u128 + 1;
            let mut authority = Authority::new(
                AuthorityId::Match(MatchId(id)),
                Reject::<N>,
                World::new(WorldEpoch(id)),
            );
            authority.register_peer(PeerId(1), 1, ActorId(1)).unwrap();
            for sequence in 1..=8 {
                authority
                    .submit(Input {
                        origin: Origin {
                            peer: PeerId(1),
                            connection_epoch: 1,
                            sequence,
                        },
                        actor: ActorId(1),
                        schedule: if sequence == 1 {
                            InputSchedule::NextTick { observed_tick: 0 }
                        } else {
                            InputSchedule::Timed { intended_tick: 12 }
                        },
                        action: [sequence as u8; N],
                    })
                    .unwrap();
            }
            prepared.push(authority);
        }
        let start = Instant::now();
        let mut resolved = 0;
        for authority in &mut prepared {
            resolved += authority.step().unwrap().group.results.len();
        }
        let elapsed = start.elapsed().as_nanos();
        black_box(resolved);
        assert_eq!(resolved, BATCH);
        if sample >= WARMUP {
            times.push(elapsed / BATCH as u128);
        }
    }
    println!(
        "pending/remove-first/{N}-byte-action: batch={BATCH} samples={SAMPLES} step_ns[p50/p95/p99]={}/{}/{}",
        percentile(&mut times, 50),
        percentile(&mut times, 95),
        percentile(&mut times, 99),
    );
}

fn main() {
    println!(
        "I1 pending fixture host={}/{} profile=release",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    run::<16>();
    run::<4_096>();
}
