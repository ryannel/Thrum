use crowd_2d::{Candidate, Crowd, Simulation, frames};
use serde_json::{Value, json};
use thrum_core::sim::{
    GameRuntime, HistoryGame, RuleSetIdentity, StructuralChange, WorldImageError,
};

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use super::{hex, rule_set};

const IMAGE_LIMIT: usize = 4 * 1024 * 1024;
const SOURCE_EPOCH: u128 = 101;
const DESTINATION_EPOCH: u128 = 207;

fn step(runtime: &mut GameRuntime<Crowd>, game: &mut Crowd, candidate: &mut Candidate) {
    let next = runtime.tick() + 1;
    // Start with the corrected recorded frames. This is the straight route
    // already used to produce SIM-2D's retained corrected timeline.
    game.prepare_history_tick(runtime, next, &frames(next, true))
        .expect("recorded inputs");
    runtime
        .step(game, candidate)
        .expect("registered crowd tick");
}

fn digest(runtime: &GameRuntime<Crowd>, game: &Crowd) -> String {
    format!(
        "{:016x}",
        runtime
            .world_digest(&game.resources, |_, _| {})
            .expect("portable crowd")
    )
}

fn snapshot(
    simulation: &Simulation,
    identity: RuleSetIdentity,
    buffer: &mut Vec<u8>,
    tick_ns: u128,
) -> Value {
    let start = Instant::now();
    simulation
        .runtime
        .write_world_image(identity, &simulation.game.resources, buffer)
        .expect("write portable crowd");
    let write_ns = start.elapsed().as_nanos();
    let view = simulation.runtime.read();
    let reused = (0..u32::try_from(view.capacity()).expect("world capacity fits u32"))
        .filter_map(|slot| view.live_at_slot(slot))
        .filter(|slot| slot.handle().generation > 1)
        .count();
    json!({
        "tick": simulation.runtime.tick(),
        "digest": digest(&simulation.runtime, &simulation.game),
        "epoch": u64::try_from(simulation.runtime.epoch().0).expect("proof epoch fits u64"),
        "live": simulation.runtime.len(),
        "reused": reused,
        "bytes": buffer.len(),
        "image": hex(buffer),
        "write_ns": u64::try_from(write_ns).expect("timing fits u64"),
        "tick_ns": u64::try_from(tick_ns).expect("timing fits u64"),
    })
}

/// Step a seeded authority and write a world image at each requested tick.
///
/// # Panics
/// The reference trace or its image encoder fails.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn capture(count: u32, mixed: bool, ticks: &[u32]) -> String {
    let identity = rule_set(mixed);
    let mut simulation = if mixed {
        Simulation::mixed(count, SOURCE_EPOCH)
    } else {
        Simulation::new(count, SOURCE_EPOCH)
    };
    let mut buffer = Vec::new();
    let mut rows = Vec::with_capacity(ticks.len());
    let mut despawned = 0;
    for &target in ticks {
        let mut tick_ns = 0;
        while simulation.runtime.tick() < u64::from(target) {
            let start = Instant::now();
            step(
                &mut simulation.runtime,
                &mut simulation.game,
                &mut simulation.candidate,
            );
            tick_ns = start.elapsed().as_nanos();
            despawned += simulation
                .runtime
                .latest()
                .expect("published tick")
                .structural_events()
                .iter()
                .filter(|event| matches!(event, StructuralChange::Despawn(_)))
                .count();
        }
        assert_eq!(
            simulation.runtime.tick(),
            u64::from(target),
            "ordered capture ticks"
        );
        let mut row = snapshot(&simulation, identity, &mut buffer, tick_ns);
        row["despawned"] = json!(despawned);
        rows.push(row);
    }
    json!({"identity": hex(&identity.0), "rows": rows}).to_string()
}

/// Install a Mac image in a fresh runtime and continue the corrected trace.
///
/// # Errors
/// The runtime refuses an incompatible or invalid image.
///
/// # Panics
/// A reference trace tick fails.
pub fn restore(
    count: u32,
    mixed: bool,
    image: &[u8],
    last: u32,
) -> Result<String, WorldImageError> {
    let (mut runtime, mut game, mut candidate) =
        Simulation::image_parts(count, DESTINATION_EPOCH, mixed);
    let identity = rule_set(mixed);
    let start = Instant::now();
    runtime.restore_world_image(identity, &mut game.resources, image, IMAGE_LIMIT)?;
    let restore_ns = start.elapsed().as_nanos();
    let start_tick = runtime.tick();
    let initial = digest(&runtime, &game);
    runtime.publish_initial(&mut game, &mut candidate);
    let mut checkpoints = Vec::new();
    while runtime.tick() < u64::from(last) {
        step(&mut runtime, &mut game, &mut candidate);
        let tick = runtime.tick();
        if tick.is_multiple_of(120) {
            checkpoints.push((tick, digest(&runtime, &game)));
        }
    }
    Ok(json!({
        "identity": hex(&identity.0),
        "tick": start_tick,
        "epoch": u64::try_from(runtime.epoch().0).expect("proof epoch fits u64"),
        "initial": initial,
        "last": runtime.tick(),
        "checkpoints": checkpoints,
        "restore_ns": u64::try_from(restore_ns).expect("timing fits u64"),
    })
    .to_string())
}
