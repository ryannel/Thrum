use std::{hint::black_box, time::Instant};
use thrum_world_storage_spike::{Adapter, Bevy, COUNT, Custom, Hecs, Row};

fn stats(samples: &mut [u64]) -> (u64, u64, u64) {
    samples.sort_unstable();
    let p50 = samples[samples.len() / 2];
    let p99 = samples[samples.len() * 99 / 100];
    (p50, p99, *samples.last().unwrap())
}

fn measure<A: Adapter>(name: &str) {
    const SAMPLES: usize = 1_200;
    let mut world = A::new();
    let mut four = std::array::from_fn::<_, 4, _>(|_| vec![Row::default(); COUNT]);
    for _ in 0..600 {
        world.tick();
    }
    let mut tick = vec![0; SAMPLES];
    let mut publish = vec![0; SAMPLES];
    for sample in 0..SAMPLES {
        let start = Instant::now();
        world.tick();
        tick[sample] = start.elapsed().as_nanos() as u64;
        let start = Instant::now();
        for out in &mut four {
            world.copy_snapshot(out);
        }
        publish[sample] = start.elapsed().as_nanos() as u64;
        black_box(&four);
    }
    let tick = stats(&mut tick);
    let publish = stats(&mut publish);
    let mut ring = vec![vec![Row::default(); COUNT]; 16];
    for slot in &mut ring {
        world.tick();
        world.copy_snapshot(slot);
    }
    println!(
        "{name}: tick ns p50={} p99={} max={}; four snapshots ns p50={} p99={} max={}; 16 full history rows={} bytes={}",
        tick.0,
        tick.1,
        tick.2,
        publish.0,
        publish.1,
        publish.2,
        COUNT * 16,
        COUNT * 16 * size_of::<Row>()
    );
}

fn main() {
    println!(
        "macOS native release, {} entities, {} moving, 100 spawn/despawn pairs/s; 600 warm + 1200 sampled ticks",
        COUNT,
        thrum_world_storage_spike::MOVING
    );
    measure::<Custom>("custom");
    measure::<Hecs>("hecs");
    measure::<Bevy>("bevy");
}
