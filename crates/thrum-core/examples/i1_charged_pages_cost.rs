//! Local charged-page cost fixture. Run with:
//! `RUSTC_WRAPPER= cargo run --release --locked --offline -p thrum-core --example i1_charged_pages_cost`

use std::{hint::black_box, time::Instant};
use thrum_core::{PublicationPool, StableId, World, WorldEpoch};

fn percentile(samples: &mut [u128], percent: usize) -> u128 {
    samples.sort_unstable();
    samples[(samples.len() - 1) * percent / 100]
}

fn run(count: usize, percent: usize) {
    const WARMUP: usize = 100;
    const SAMPLES: usize = 1_000;
    let pool = PublicationPool::new(usize::MAX);
    let mut world = World::new(WorldEpoch(count as u128));
    for id in 1..=count {
        world.spawn(StableId(id as u128), 0_u64).unwrap();
    }
    let mut reservation = pool
        .try_reserve(world.required_charged_publication_bytes().unwrap())
        .unwrap();
    let mut prior = world
        .try_publish_charged(0, &mut reservation)
        .unwrap()
        .snapshot;
    drop(reservation);
    let changes = if percent == 0 {
        0
    } else {
        (count * percent).div_ceil(100)
    };
    let mut admission = Vec::with_capacity(SAMPLES);
    let mut publication = Vec::with_capacity(SAMPLES);
    let mut peak_bytes = 0;
    for tick in 1..=WARMUP + SAMPLES {
        for j in 0..changes {
            let id = 1 + ((j * count / changes.max(1) + tick) % count);
            let handle = world.handle_for(StableId(id as u128)).unwrap();
            *world.get_mut(handle).unwrap() += 1;
        }
        let start = Instant::now();
        let bytes = world.required_charged_publication_bytes().unwrap();
        let mut reservation = pool.try_reserve(bytes).unwrap();
        let admitted = start.elapsed().as_nanos();
        let start = Instant::now();
        let published = world
            .try_publish_charged(tick as u64, &mut reservation)
            .unwrap();
        let published_ns = start.elapsed().as_nanos();
        peak_bytes = peak_bytes.max(pool.used_bytes());
        drop(reservation);
        black_box(prior.get(world.handle_for(StableId(1)).unwrap()));
        prior = published.snapshot;
        if tick > WARMUP {
            admission.push(admitted);
            publication.push(published_ns);
        }
    }
    assert_eq!(prior.records().count(), count);
    let final_bytes = pool.used_bytes();
    println!(
        "charged/{count}/{percent}%: ticks={SAMPLES} admission_ns[p50/p95/p99]={}/{}/{} publication_ns[p50/p95/p99]={}/{}/{} final_bytes={final_bytes} peak_bytes={peak_bytes}",
        percentile(&mut admission, 50),
        percentile(&mut admission, 95),
        percentile(&mut admission, 99),
        percentile(&mut publication, 50),
        percentile(&mut publication, 95),
        percentile(&mut publication, 99),
    );
    drop(prior);
    drop(world);
    assert_eq!(pool.used_bytes(), 0);
}

fn main() {
    println!(
        "I1 charged pages fixture host={}/{} profile=release",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    for count in [64, 4_096, 16_384] {
        for percent in [0, 1, 25] {
            run(count, percent);
        }
    }
}
