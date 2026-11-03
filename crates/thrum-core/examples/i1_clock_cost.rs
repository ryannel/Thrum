//! Headless clock and idle authority cost fixture. Run with:
//! `RUSTC_WRAPPER= cargo run --release --locked --offline -p thrum-core --example i1_clock_cost`

use std::{
    hint::black_box,
    time::{Duration, Instant},
};

use thrum_core::{
    ActorId, Authority, AuthorityId, ClockConfig, GameRules, PreparedCommit, PreparedEdits,
    SessionId, TickClock, World, WorldEpoch,
};

struct EmptyRule;
impl GameRules for EmptyRule {
    type Entity = ();
    type Action = ();
    type Change = ();
    type Rejection = ();
    type Event = ();

    fn decide(
        &self,
        _: &World<()>,
        _: ActorId,
        _: &(),
        _context: &mut thrum_core::RuleContext<'_>,
    ) -> Result<(), ()> {
        Ok(())
    }
    fn prepare(
        &self,
        _: &World<()>,
        _: (),
        _context: &mut thrum_core::RuleContext<'_>,
    ) -> PreparedCommit<(), ()> {
        PreparedCommit {
            edits: PreparedEdits::None,
            event: None,
        }
    }
}

fn run(name: &str, callback_hz: u64, with_authority: bool) {
    const WARMUP: u64 = 10_000;
    const BLOCKS: usize = 100;
    const CALLBACKS_PER_BLOCK: u64 = 1_000;
    let mut clock = TickClock::new(ClockConfig::default(), Duration::ZERO).unwrap();
    let mut authority = Authority::new(
        AuthorityId::Standalone(SessionId(1)),
        EmptyRule,
        World::new(WorldEpoch(1)),
    );
    let mut ticks = 0_u64;
    let mut callback = 0_u64;
    let mut costs = Vec::with_capacity(BLOCKS);
    for block in 0..BLOCKS + 10 {
        let start = Instant::now();
        for _ in 0..CALLBACKS_PER_BLOCK {
            callback += 1;
            let now = Duration::from_nanos(callback * 1_000_000_000 / callback_hz);
            let report = if with_authority {
                clock.advance(now, 4, || authority.step().map(|_| ()).map_err(|_| ()))
            } else {
                clock.advance(now, 4, || {
                    black_box(());
                    Ok::<(), ()>(())
                })
            }
            .unwrap();
            assert!(report.step_error.is_none());
            assert_eq!(report.refused_on_overload, 0);
            ticks += u64::from(report.completed);
            black_box(report);
        }
        if block >= 10 {
            costs.push(start.elapsed().as_nanos() / u128::from(CALLBACKS_PER_BLOCK));
        }
    }
    let final_time_ns = callback * 1_000_000_000 / callback_hz;
    assert_eq!(ticks, final_time_ns * 60 / 1_000_000_000);
    if with_authority {
        assert_eq!(authority.tick_index(), ticks);
    }
    costs.sort_unstable();
    println!(
        "{name}: callbacks={} ticks={} ns/callback p50={} p95={} p99={} max={}",
        callback - WARMUP,
        ticks,
        costs[49],
        costs[94],
        costs[98],
        costs[99]
    );
}

fn main() {
    for callback_hz in [30, 60, 120, 240] {
        run(
            &format!("clock/{callback_hz}Hz-display"),
            callback_hz,
            false,
        );
    }
    run("clock+idle-authority/120Hz-display", 120, true);
}
