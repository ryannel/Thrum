//! Local changed-only replica cost fixture. Run with:
//! `RUSTC_WRAPPER= cargo run --release --locked --offline -p thrum-core --example i1_changed_progress_cost`

use std::{hint::black_box, time::Instant};
use thrum_core::{
    AuthorityId, EntityChange, EntityRecord, MatchId, PeerId, Replica, StableId, StateGroup,
};

const WARMUP: usize = 1_000;
const SAMPLES: usize = 10_000;

fn percentile(samples: &mut [u128], percent: usize) -> u128 {
    samples.sort_unstable();
    samples[(samples.len() - 1) * percent / 100]
}

fn run(peers: u64) {
    let resolved_through: Vec<_> = (1..=peers).map(|id| (PeerId(id), 0)).collect();
    let initial: StateGroup<u64, (), ()> = StateGroup {
        authority: AuthorityId::Match(MatchId(1)),
        group_id: 0,
        commit_tick: 0,
        event_cursor: 0,
        changes: vec![EntityChange::Spawn(EntityRecord {
            id: StableId(1),
            state: 0,
        })],
        results: Vec::new(),
        events: Vec::new(),
        resolved_through: resolved_through.clone(),
    };
    let (mut replica, _) = Replica::from_baseline(initial).unwrap();
    let template = StateGroup {
        authority: AuthorityId::Match(MatchId(1)),
        group_id: 0,
        commit_tick: 0,
        event_cursor: 0,
        changes: vec![EntityChange::Update(EntityRecord {
            id: StableId(1),
            state: 1,
        })],
        results: Vec::<thrum_core::ActionResult<()>>::new(),
        events: Vec::<thrum_core::CommittedEvent<()>>::new(),
        resolved_through,
    };
    let mut times = Vec::with_capacity(SAMPLES);
    for index in 0..WARMUP + SAMPLES {
        let mut group = template.clone();
        group.group_id = index as u64 + 1;
        group.commit_tick = group.group_id;
        let start = Instant::now();
        black_box(replica.apply(group).unwrap());
        if index >= WARMUP {
            times.push(start.elapsed().as_nanos());
        }
    }
    assert_eq!(replica.tick(), (WARMUP + SAMPLES) as u64);
    assert_eq!(replica.get(StableId(1)), Some(&1));
    println!(
        "changed-only/{peers}-peers: samples={SAMPLES} replica_apply_ns[p50/p95/p99]={}/{}/{}",
        percentile(&mut times, 50),
        percentile(&mut times, 95),
        percentile(&mut times, 99)
    );
}

fn main() {
    println!(
        "I1 changed-only fixture host={}/{} profile=release",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    for peers in [2, 64] {
        run(peers);
    }
}
