//! Initial immutable-page publication cost at several finite world sizes.
//! `RUSTC_WRAPPER= cargo run --release --locked --offline -p thrum-core --example i1_growth_cost`

use std::{hint::black_box, time::Instant};

use thrum_core::{StableId, World, WorldEpoch};

const WARMUP: usize = 10;
const MEASURED: usize = 100;
const PAGE_SIZE: usize = 32;

fn run(objects: usize) {
    let mut samples = Vec::with_capacity(MEASURED);
    let pages = objects.div_ceil(PAGE_SIZE);
    for trial in 0..WARMUP + MEASURED {
        let mut world = World::new(WorldEpoch((trial + 1) as u128));
        for id in 1..=objects {
            world.spawn(StableId(id as u128), id as u64).unwrap();
        }
        let start = Instant::now();
        let publication = world.publish(0);
        let elapsed = start.elapsed().as_nanos();
        assert_eq!(publication.work.copied_pages, pages);
        assert_eq!(publication.work.copied_objects, objects);
        assert_eq!(publication.snapshot.records().count(), objects);
        black_box(publication);
        if trial >= WARMUP {
            samples.push(elapsed);
        }
    }
    samples.sort_unstable();
    println!(
        "initial publish: objects={objects} pages={pages} samples={MEASURED} ns[p50/p95/p99/max]={}/{}/{}/{}",
        samples[49], samples[94], samples[98], samples[99]
    );
}

fn main() {
    for objects in [64, 4_096, 16_384] {
        run(objects);
    }
}
