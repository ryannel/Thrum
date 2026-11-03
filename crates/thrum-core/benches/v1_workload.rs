//! Fixture baseline only. A1/A5 add engine adapters and end-to-end benchmarks.
#[expect(
    dead_code,
    reason = "shared workload fixture has consumers across test and benchmark targets"
)]
#[path = "../tests/support/v1_workload.rs"]
mod support;

use std::num::NonZeroU32;
use thrum_core::{
    StableId, WorldEpoch,
    sim::{ComponentEntity, Position, SimConfig, Simulation, SimulationSnapshot, Velocity},
};

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

fn main() {
    divan::main();
}

#[divan::bench]
fn moving_world_and_four_peers(bencher: divan::Bencher<'_, '_>) {
    let mut workload = support::Workload::new();
    bencher.bench_local(move || {
        workload.step();
        divan::black_box(&workload);
    });
}

#[divan::bench]
fn four_peer_conditioner(bencher: divan::Bencher<'_, '_>) {
    let mut link = support::LinkConditioner::new(0xA0A0_2026_1103);
    let mut tick = 0;
    bencher.bench_local(move || {
        tick += 1;
        link.step(tick);
        divan::black_box(link.delivered);
    });
}

fn component_fixture() -> Simulation {
    let mut sim = Simulation::new(
        WorldEpoch(1),
        SimConfig {
            tick_hz: NonZeroU32::new(60).expect("test setup or assertion requires a value"),
            entity_capacity: support::ENTITY_COUNT,
            peer_capacity: support::PEER_COUNT,
            actions_per_frame: 8,
            movement_speed: 6.0,
        },
    )
    .expect("test setup or assertion requires a value");
    for id in 0..support::ENTITY_COUNT {
        sim.seed(ComponentEntity {
            id: StableId(id as u128),
            position: Position::default(),
            velocity: Velocity([if id < support::MOVING_COUNT { 1.0 } else { 0.0 }, 0.0, 0.0]),
            health: 100,
            ownership: None,
            visual: Some(2),
        })
        .expect("test setup or assertion requires a value");
    }
    sim
}

#[divan::bench]
fn a1_component_tick(bencher: divan::Bencher<'_, '_>) {
    let mut sim = component_fixture();
    bencher.bench_local(move || {
        divan::black_box(
            sim.step(&[])
                .expect("test setup or assertion requires a value"),
        );
        divan::black_box(&sim);
    });
}

#[divan::bench]
fn a1_four_full_snapshots(bencher: divan::Bencher<'_, '_>) {
    let sim = component_fixture();
    let mut snapshots = std::array::from_fn::<_, 4, _>(|_| {
        SimulationSnapshot::with_capacity(WorldEpoch(1), support::ENTITY_COUNT)
    });
    bencher.bench_local(move || {
        for snapshot in &mut snapshots {
            sim.copy_snapshot_into(snapshot);
        }
        divan::black_box(&snapshots);
    });
}
