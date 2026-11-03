//! Provisional A1 correctness and allocation evidence. This allocator delegates
//! to System; unsafe is confined to the test observer, never production storage.
#![allow(unsafe_code)]

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    num::NonZeroU32,
    time::{Duration, Instant},
};

use thrum_core::{
    ActorId, ClockConfig, Handle, Origin, PeerId, StableId, TickClock, WorldEpoch,
    sim::{
        Command, ComponentEntity, FrameError, InputFrame, Ownership, Position, SimAction,
        SimConfig, SimError, Simulation, SimulationSnapshot, Stage, Velocity,
    },
};

thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
}

struct CountingAllocator;
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        COUNTING.with(|active| {
            if active.get() {
                ALLOCATIONS.with(|count| count.set(count.get() + 1));
            }
        });
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        COUNTING.with(|active| {
            if active.get() {
                ALLOCATIONS.with(|count| count.set(count.get() + 1));
            }
        });
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

fn config(capacity: usize) -> SimConfig {
    SimConfig {
        tick_hz: NonZeroU32::new(60).unwrap(),
        entity_capacity: capacity,
        peer_capacity: 4,
        actions_per_frame: 8,
        movement_speed: 6.0,
    }
}

fn entity(id: u128, velocity: f64) -> ComponentEntity {
    ComponentEntity {
        id: StableId(id),
        position: Position::default(),
        velocity: Velocity([velocity, 0.0, 0.0]),
        health: 100,
        ownership: None,
        visual: Some(2),
    }
}

fn controlled() -> (Simulation, Handle, Ownership) {
    let mut sim = Simulation::new(WorldEpoch(1), config(8)).unwrap();
    let binding = Ownership {
        peer: PeerId(7),
        connection_epoch: 3,
        actor: ActorId(9),
    };
    let mut body = entity(1, 0.0);
    body.ownership = Some(binding);
    let handle = sim.seed(body).unwrap();
    sim.register_peer(binding, handle).unwrap();
    (sim, handle, binding)
}

fn frame(binding: Ownership, tick: u64, sequence: u64) -> InputFrame<SimAction> {
    InputFrame {
        origin: Origin {
            peer: binding.peer,
            connection_epoch: binding.connection_epoch,
            sequence,
        },
        actor: binding.actor,
        tick,
        axes: [1.0, 0.0, 0.0],
        actions: vec![],
    }
}

#[test]
fn held_axes_and_idle_ticks_advance_without_repeating_discrete_actions() {
    let (mut sim, handle, binding) = controlled();
    let mut input = frame(binding, 1, 0);
    input.actions = vec![SimAction::Impulse([6.0, 0.0, 0.0]), SimAction::Damage(7)];
    sim.admit_frame(input).unwrap();
    assert_eq!(
        sim.get(handle).unwrap().health,
        100,
        "admission does not apply input"
    );
    let result = sim.step(&[]).unwrap();
    assert_eq!(result.simulated_entities, 1);
    let first = sim.get(handle).unwrap();
    assert_eq!(first.position.0, [0.2, 0.0, 0.0]);
    assert_eq!(first.health, 93);
    assert_eq!(first.changes.health, 1);
    for _ in 0..9 {
        sim.step(&[]).unwrap();
    }
    let after = sim.get(handle).unwrap();
    assert!((after.position.0[0] - 1.1).abs() < 1e-12);
    assert_eq!(after.health, 93);
    assert_eq!(after.velocity.0, [6.0, 0.0, 0.0]);
    assert_eq!(after.changes.health, 1);
    assert_eq!(after.changes.velocity, 2);
    assert_eq!(sim.tick(), 10);
    assert_eq!(
        sim.schedule().order().map(|s| s.stage).collect::<Vec<_>>(),
        [
            Stage::Input,
            Stage::Simulate,
            Stage::Resolve,
            Stage::Publish
        ]
    );
}

#[test]
fn rejected_frames_do_not_consume_sequence_and_all_origins_share_validation() {
    let (mut sim, handle, binding) = controlled();
    let mut bad = frame(binding, 1, 10);
    bad.axes[1] = f32::NAN;
    assert_eq!(sim.admit_frame(bad), Err(FrameError::InvalidAxis));
    let mut bad = frame(binding, 1, 10);
    bad.origin.connection_epoch += 1;
    assert_eq!(sim.admit_frame(bad), Err(FrameError::WrongConnection));
    let mut bad = frame(binding, 1, 10);
    bad.actor = ActorId(100);
    assert_eq!(sim.admit_frame(bad), Err(FrameError::WrongActor));
    assert_eq!(
        sim.admit_frame(frame(binding, 0, 10)),
        Err(FrameError::WrongTick)
    );
    assert_eq!(
        sim.admit_frame(frame(binding, 2, 10)),
        Err(FrameError::WrongTick)
    );
    let mut bad = frame(binding, 1, 10);
    bad.actions = vec![SimAction::Impulse([f64::INFINITY, 0.0, 0.0])];
    assert_eq!(sim.admit_frame(bad), Err(FrameError::InvalidAction));
    let mut bad = frame(binding, 1, 10);
    bad.actions = vec![SimAction::Damage(1); 9];
    assert_eq!(sim.admit_frame(bad), Err(FrameError::TooManyActions));
    sim.admit_frame(frame(binding, 1, 10)).unwrap();
    assert_eq!(
        sim.admit_frame(frame(binding, 1, 11)),
        Err(FrameError::PendingFrame)
    );
    sim.step(&[]).unwrap();
    assert_eq!(
        sim.admit_frame(frame(binding, 2, 10)),
        Err(FrameError::StaleSequence)
    );
    sim.admit_frame(frame(binding, 2, 11)).unwrap();
    sim.step(&[]).unwrap();
    assert_eq!(sim.get(handle).unwrap().position.0[0], 0.2);
}

#[test]
fn invalid_batch_and_numeric_failure_preserve_the_complete_committed_endpoint() {
    let (mut sim, handle, binding) = controlled();
    let mut input = frame(binding, 1, 1);
    input.actions = vec![SimAction::Damage(12)];
    sim.admit_frame(input).unwrap();
    let mut before = SimulationSnapshot::with_capacity(WorldEpoch(1), 8);
    sim.copy_snapshot_into(&mut before);
    let foreign = Handle {
        epoch: WorldEpoch(99),
        ..handle
    };
    assert_eq!(
        sim.step(&[Command::SetHealth(handle, 20), Command::Despawn(foreign)]),
        Err(SimError::InvalidHandle)
    );
    let mut after = SimulationSnapshot::with_capacity(WorldEpoch(1), 8);
    sim.copy_snapshot_into(&mut after);
    assert_eq!(before, after);
    assert_eq!(
        sim.step(&[Command::SetHealth(handle, 20), Command::Despawn(handle)]),
        Err(SimError::ConflictingCommands)
    );
    sim.step(&[]).unwrap();
    assert_eq!(
        sim.get(handle).unwrap().health,
        88,
        "pending action survived the failed tick exactly once"
    );

    let mut numeric = Simulation::new(WorldEpoch(2), config(1)).unwrap();
    let mut extreme = entity(1, f64::MAX);
    extreme.position = Position([f64::MAX, 0.0, 0.0]);
    let target = numeric.seed(extreme).unwrap();
    assert_eq!(numeric.step(&[]), Err(SimError::NonFinite));
    assert_eq!(numeric.tick(), 0);
    assert_eq!(*numeric.get(target).unwrap().position, extreme.position);
    numeric
        .step(&[
            Command::SetPosition(target, Position::default()),
            Command::SetVelocity(target, Velocity::default()),
        ])
        .unwrap();
    assert_eq!(numeric.tick(), 1);
    assert_eq!(numeric.get(target).unwrap().position.0, [0.0; 3]);
}

#[test]
fn committed_destruction_reuse_and_consumer_copy_preserve_identity() {
    let mut sim = Simulation::new(WorldEpoch(1), config(1)).unwrap();
    let old = sim.seed(entity(1, 60.0)).unwrap();
    let mut retained = SimulationSnapshot::with_capacity(WorldEpoch(1), 1);
    sim.copy_snapshot_into(&mut retained);
    let old_copy = retained.clone();
    let outcome = sim
        .step(&[Command::Spawn(entity(2, 120.0)), Command::Despawn(old)])
        .unwrap();
    assert_eq!(outcome.changed_positions, 0);
    let new = sim.handle(StableId(2)).unwrap();
    assert_eq!(old.slot, new.slot);
    assert_ne!(old.generation, new.generation);
    assert!(matches!(sim.get(old), Err(SimError::InvalidHandle)));
    assert_eq!(
        sim.get(new).unwrap().position.0,
        [0.0; 3],
        "resolve-stage spawn moves next tick"
    );
    assert_eq!(sim.get(new).unwrap().changes.structural, 1);
    assert_eq!(retained, old_copy);
    sim.step(&[]).unwrap();
    assert_eq!(sim.get(new).unwrap().position.0[0], 2.0);
    assert_eq!(retained.entities()[0].id, StableId(1));
    assert_eq!(
        sim.step(&[Command::Despawn(old)]),
        Err(SimError::InvalidHandle)
    );
    assert_eq!(sim.tick(), 2);
    assert_eq!(
        sim.step(&[Command::Spawn(entity(2, 0.0))]),
        Err(SimError::DuplicateId)
    );
}

#[test]
fn destruction_retires_control_before_a_new_connection_uses_the_peer() {
    let (mut sim, old, binding) = controlled();
    sim.admit_frame(frame(binding, 1, 1)).unwrap();
    let new_binding = Ownership {
        connection_epoch: binding.connection_epoch + 1,
        ..binding
    };
    let mut stale_replacement = entity(2, 0.0);
    stale_replacement.ownership = Some(binding);
    let mut replacement = entity(3, 0.0);
    replacement.ownership = Some(new_binding);
    sim.step(&[
        Command::Despawn(old),
        Command::Spawn(stale_replacement),
        Command::Spawn(replacement),
    ])
    .unwrap();
    assert_eq!(
        sim.admit_frame(frame(binding, 2, 2)),
        Err(FrameError::InvalidControl)
    );
    assert_eq!(
        sim.register_peer(binding, sim.handle(StableId(2)).unwrap()),
        Err(SimError::InvalidOwnership),
        "the same connection generation cannot reset sequence history"
    );
    let new = sim.handle(StableId(3)).unwrap();
    sim.register_peer(new_binding, new).unwrap();
    assert_eq!(
        sim.admit_frame(frame(binding, 2, 2)),
        Err(FrameError::WrongConnection)
    );
    sim.admit_frame(frame(new_binding, 2, 1)).unwrap();
    sim.step(&[]).unwrap();
    assert_eq!(sim.get(new).unwrap().position.0, [0.1, 0.0, 0.0]);
}

#[test]
fn only_actual_component_changes_advance_change_ticks() {
    let mut sim = Simulation::new(WorldEpoch(1), config(10_000)).unwrap();
    for id in 0..10_000 {
        sim.seed(entity(id, if id < 100 { 1.0 } else { 0.0 }))
            .unwrap();
    }
    assert_eq!(sim.step(&[]).unwrap().changed_positions, 100);
    assert_eq!(
        sim.query()
            .filter(|view| view.changes.position == 1)
            .count(),
        100
    );
    assert!(
        sim.query()
            .all(|view| view.changes.health == 0 && view.changes.velocity == 0)
    );
    let static_handle = sim.handle(StableId(500)).unwrap();
    sim.step(&[
        Command::SetPosition(static_handle, Position::default()),
        Command::SetHealth(static_handle, 100),
    ])
    .unwrap();
    assert_eq!(sim.get(static_handle).unwrap().changes.position, 0);
    assert_eq!(sim.get(static_handle).unwrap().changes.health, 0);
}

#[test]
fn existing_fixed_clock_drives_every_stage_without_render_or_input_callbacks() {
    let mut sim = Simulation::new(WorldEpoch(1), config(1)).unwrap();
    let handle = sim.seed(entity(1, 60.0)).unwrap();
    let clock_config = ClockConfig {
        tick_hz: sim.tick_hz(),
        max_ticks_per_call: NonZeroU32::new(8).unwrap(),
        max_queued_ticks: NonZeroU32::new(16).unwrap(),
    };
    let mut clock = TickClock::new(clock_config, Duration::ZERO).unwrap();
    let report = clock
        .advance(Duration::from_millis(100), 8, || sim.step(&[]).map(|_| ()))
        .unwrap();
    assert_eq!(report.completed, 6);
    assert_eq!(sim.tick(), 6);
    assert_eq!(sim.get(handle).unwrap().position.0[0], 6.0);
}

fn workload() -> Simulation {
    let mut sim = Simulation::new(WorldEpoch(1), config(10_000)).unwrap();
    for id in 0..10_000 {
        sim.seed(entity(id, if id < 1_500 { 1.0 } else { 0.0 }))
            .unwrap();
    }
    sim
}

#[test]
fn ten_thousand_entity_ticks_churn_and_four_copies_allocate_nothing_after_setup() {
    let mut sim = workload();
    let mut snapshots = std::array::from_fn::<_, 4, _>(|_| {
        SimulationSnapshot::with_capacity(WorldEpoch(1), 10_000)
    });
    for _ in 0..60 {
        sim.step(&[]).unwrap();
    }
    ALLOCATIONS.with(|count| count.set(0));
    COUNTING.with(|active| active.set(true));
    let mut churn_pairs = 0;
    for step in 0..600u128 {
        // 100 spawn/despawn pairs per second, on a stable ten-thousand-live set.
        let base = step * 2;
        let h0 = sim.handle(StableId(8_000 + base)).unwrap();
        let commands = [
            Command::Despawn(h0),
            Command::Spawn(entity(20_000 + base, 0.0)),
        ];
        // Two pairs on two ticks out of three make exactly 1,000 pairs in 600 ticks.
        let outcome = if step % 3 != 0 {
            let h1 = sim.handle(StableId(8_001 + base)).unwrap();
            churn_pairs += 2;
            sim.step(&[
                commands[0],
                commands[1],
                Command::Despawn(h1),
                Command::Spawn(entity(20_001 + base, 0.0)),
            ])
            .unwrap()
        } else {
            churn_pairs += 1;
            sim.step(&commands).unwrap()
        };
        std::hint::black_box(outcome);
        for snapshot in &mut snapshots {
            sim.copy_snapshot_into(snapshot);
        }
    }
    COUNTING.with(|active| active.set(false));
    assert_eq!(ALLOCATIONS.with(Cell::get), 0);
    assert_eq!(churn_pairs, 1_000);
    assert_eq!(sim.len(), 10_000);
    assert_eq!(sim.tick(), 660);
    assert!(snapshots.iter().all(|snapshot| snapshot.tick() == 660));
}

#[test]
fn thirty_two_thousand_unique_id_replacements_need_no_rehash_or_allocation() {
    let mut sim = workload();
    let stale = sim.handle(StableId(9_000)).unwrap();
    let mut commands = [Command::Spawn(entity(0, 0.0)); 200];
    for _ in 0..60 {
        sim.step(&[]).unwrap();
    }
    ALLOCATIONS.with(|count| count.set(0));
    COUNTING.with(|active| active.set(true));
    for cycle in 0..320u128 {
        for i in 0..100u128 {
            let old = if cycle == 0 {
                StableId(9_000 + i)
            } else {
                StableId(100_000 + (cycle - 1) * 100 + i)
            };
            commands[(i * 2) as usize] = Command::Despawn(sim.handle(old).unwrap());
            commands[(i * 2 + 1) as usize] = Command::Spawn(entity(100_000 + cycle * 100 + i, 0.0));
        }
        sim.step(&commands).unwrap();
    }
    COUNTING.with(|active| active.set(false));
    assert_eq!(ALLOCATIONS.with(Cell::get), 0);
    assert_eq!(sim.len(), 10_000);
    assert_eq!(sim.tick(), 380);
    assert_eq!(sim.get(stale).err(), Some(SimError::InvalidHandle));
    for i in 0..100u128 {
        assert!(sim.handle(StableId(100_000 + 319 * 100 + i)).is_some());
    }
}

#[test]
#[ignore = "opt-in native release CPU observation; no portable performance threshold"]
fn measure_serial_ten_thousand_entity_ticks() {
    let mut sim = workload();
    let mut snapshot = SimulationSnapshot::with_capacity(WorldEpoch(1), 10_000);
    for _ in 0..600 {
        sim.step(&[]).unwrap();
    }
    let mut tick_ns = Vec::with_capacity(3_600);
    let mut extract_ns = Vec::with_capacity(3_600);
    for _ in 0..3_600 {
        let start = Instant::now();
        std::hint::black_box(sim.step(&[]).unwrap());
        tick_ns.push(start.elapsed().as_nanos());
        let start = Instant::now();
        for _ in 0..4 {
            sim.copy_snapshot_into(&mut snapshot);
            std::hint::black_box(&snapshot);
        }
        extract_ns.push(start.elapsed().as_nanos());
    }
    tick_ns.sort_unstable();
    extract_ns.sort_unstable();
    eprintln!(
        "A1 serial: 10000 entities/1500 moving, 600 warmup, 3600 samples; tick p50={}ns p99={}ns max={}ns; four full copies p50={}ns p99={}ns max={}ns",
        tick_ns[1800],
        tick_ns[3564],
        tick_ns[3599],
        extract_ns[1800],
        extract_ns[3564],
        extract_ns[3599]
    );
}
