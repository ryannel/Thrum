//! Isolated duplicate-change validation cost for authority-sorted IDs.
//! `RUSTC_WRAPPER= cargo run --release --locked --offline -p thrum-core --example i1_change_validation_cost`

use std::{collections::BTreeSet, hint::black_box, time::Instant};
use thrum_core::StableId;

fn summary(times: &mut [u128]) -> (u128, u128, u128) {
    times.sort_unstable();
    let last = times.len() - 1;
    (
        times[last / 2],
        times[last * 95 / 100],
        times[last * 99 / 100],
    )
}

fn measure(mut check: impl FnMut(&[StableId]) -> bool, ids: &[StableId]) -> (u128, u128, u128) {
    const WARMUP: usize = 200;
    const SAMPLES: usize = 1_000;
    let mut times = Vec::with_capacity(SAMPLES);
    for sample in 0..WARMUP + SAMPLES {
        let start = Instant::now();
        assert!(black_box(check(black_box(ids))));
        if sample >= WARMUP {
            times.push(start.elapsed().as_nanos());
        }
    }
    summary(&mut times)
}

fn tree(ids: &[StableId]) -> bool {
    let mut seen = BTreeSet::new();
    for id in ids {
        if !seen.insert(*id) {
            return false;
        }
    }
    true
}

fn sorted(ids: &[StableId]) -> bool {
    ids.windows(2).all(|pair| pair[0] < pair[1])
}

fn main() {
    println!(
        "I1 change-ID validation host={}/{} profile=release",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    for count in [1, 64, 1_024, 4_096] {
        let ids = (1..=count)
            .map(|id| StableId(id as u128))
            .collect::<Vec<_>>();
        let old = measure(tree, &ids);
        let new = measure(sorted, &ids);
        println!(
            "changes/{count}: tree_ns[p50/p95/p99]={old:?} sorted_scan_ns[p50/p95/p99]={new:?}"
        );
    }
}
