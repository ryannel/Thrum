use crowd_2d::{Simulation, frames};
use serde_json::json;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

/// Measure one release crowd through the engine runtime. Times never decide success.
///
/// # Panics
/// If a recorded reference input cannot commit or rewind.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn costs(count: u32) -> String {
    let mut sim = Simulation::new(count, 1);
    let mut tick_times = Vec::new();
    let mut index_times = Vec::new();
    let mut digest_times = Vec::new();
    let mut neighbours = 0;
    let ticks = if count == 10_000 { 36_000 } else { 3_600 };
    for tick in 1..=ticks {
        let input = frames(tick, true);
        let start = Instant::now();
        sim.history
            .step(&mut sim.runtime, &mut sim.game, &mut sim.candidate, &input)
            .expect("cost tick");
        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
        if tick <= 120 {
            continue;
        }
        neighbours += sim.candidate.neighbours;
        tick_times.push(elapsed);
        let start = Instant::now();
        sim.rebuild_index();
        index_times.push(start.elapsed().as_secs_f64() * 1000.0);
        let start = Instant::now();
        std::hint::black_box(sim.digest());
        digest_times.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    let expected = sim.digest();
    let start = Instant::now();
    sim.history
        .rewind(
            &mut sim.runtime,
            &mut sim.game,
            8,
            |_, (), _, _, _| None,
            |_, _, (), _, _, _, _| {},
        )
        .expect("rewind eight");
    let rewind = start.elapsed().as_secs_f64() * 1000.0;
    let start = Instant::now();
    while sim
        .history
        .replay_next(&mut sim.runtime, &mut sim.game, &mut sim.candidate)
        .expect("replay")
    {}
    let replay = start.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(sim.digest(), expected);
    let tick = median(&mut tick_times);
    let visited = neighbours as f64 / tick_times.len() as f64;
    json!({ "bodies": count, "ticks": ticks, "warmup_ticks": 120, "samples": tick_times.len(), "tick_median_ms": tick,
        "index_median_ms": median(&mut index_times), "digest_median_ms": median(&mut digest_times),
        "rewind_eight_ms": rewind, "replay_eight_ms": replay, "neighbours_per_tick": visited,
        "tick_ms_per_neighbour": tick / visited, "clock_tripwire": tick > 1000.0 / 60.0 }).to_string()
}
