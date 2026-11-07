//! One-command G2R authority benchmark: `cargo bench -p v1-game --bench g2r`.
#![cfg(not(target_arch = "wasm32"))]
#![allow(
    unsafe_code,
    reason = "process-wide counting allocator delegates to System"
)]

use std::{
    alloc::{GlobalAlloc, Layout, System},
    hint::black_box,
    num::{NonZeroU32, NonZeroUsize},
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
    time::Instant,
};

use thrum_core::{
    ActorId, Origin, PeerId, StableId, WorldEpoch,
    sim::{ExecutionMode, InputFrame},
};
use v1_game::simulation::{
    Command, ComponentEntity, Ownership, Position, SimAction, SimConfig, Simulation,
    SimulationHistory, Velocity,
};

static COUNTING: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicU64 = AtomicU64::new(0);

struct CountingAllocator;

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

// SAFETY: Every allocation operation delegates its original arguments to System.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        // SAFETY: The original layout is forwarded to System.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: The original pointer and layout are forwarded to System.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        if COUNTING.load(Ordering::Relaxed) {
            ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
        }
        // SAFETY: The original pointer, layout and size are forwarded to System.
        unsafe { System.realloc(ptr, layout, size) }
    }
}

const SAMPLES: usize = 1_000;

fn fixture(count: usize, shuffled: bool, mode: ExecutionMode) -> Simulation {
    fixture_with_capacity(count, count, shuffled, mode)
}

fn fixture_with_capacity(
    count: usize,
    capacity: usize,
    shuffled: bool,
    mode: ExecutionMode,
) -> Simulation {
    let mut sim = Simulation::new(
        WorldEpoch(1),
        SimConfig {
            tick_hz: NonZeroU32::new(60).expect("tick rate"),
            entity_capacity: capacity,
            peer_capacity: 4,
            actions_per_frame: 8,
            movement_speed: 6.0,
        },
    )
    .expect("fixture configuration");
    for index in 0..count {
        let moving = if shuffled {
            index % 20 < 3
        } else {
            index < count * 3 / 20
        };
        let peer = (index >= count - 4).then(|| {
            let peer = u64::try_from(index - (count - 4) + 1).expect("four peers");
            Ownership {
                peer: PeerId(peer),
                connection_epoch: 1,
                actor: ActorId(peer),
            }
        });
        sim.seed(ComponentEntity {
            id: StableId(index as u128),
            position: Position([index as f64 * 0.001, 0.0, 0.0]),
            velocity: Velocity([if moving { 0.05 } else { 0.0 }, 0.0, 0.0]),
            health: if peer.is_some() { u16::MAX } else { 100 },
            ownership: peer,
            visual: Some(1),
        })
        .expect("seed row");
    }
    for number in 1..=4_u64 {
        let owner = Ownership {
            peer: PeerId(number),
            connection_epoch: 1,
            actor: ActorId(number),
        };
        let id = StableId((count - 4 + usize::try_from(number - 1).expect("peer index")) as u128);
        let handle = sim.handle(id).expect("peer row");
        sim.register_peer(owner, handle).expect("bind peer");
    }
    sim.set_execution_mode(mode);
    sim
}

fn measured(mut run: impl FnMut()) -> (u64, u64) {
    ALLOCATIONS.store(0, Ordering::Relaxed);
    COUNTING.store(true, Ordering::Release);
    let start = Instant::now();
    run();
    let elapsed = start.elapsed();
    COUNTING.store(false, Ordering::Release);
    (
        u64::try_from(elapsed.as_nanos()).expect("sample fits u64"),
        ALLOCATIONS.load(Ordering::Relaxed),
    )
}

#[expect(
    clippy::print_stdout,
    reason = "the explicit benchmark emits its reproducible evidence table"
)]
fn report(label: &str, samples: &mut [u64], allocations: u64) {
    samples.sort_unstable();
    let quantile = |numerator: usize| samples[(samples.len() - 1) * numerator / 100];
    println!(
        "\"{label}\",{},{},{},{}",
        quantile(50),
        quantile(95),
        quantile(99),
        allocations
    );
    if !label.starts_with("crossover,") && !label.starts_with("G0,") {
        assert_eq!(
            allocations, 0,
            "warmed {label} allocated on a process thread"
        );
    }
}

fn crossover_case(count: usize, mode: ExecutionMode) {
    let mut sim = fixture(count, false, mode);
    let handle = sim.handle(StableId(0)).expect("first mover");
    let command = [Command::SetHealth(handle, 100)];
    for _ in 0..80 {
        sim.step(&command).expect("warm candidate tick");
    }
    let mut samples = Vec::with_capacity(SAMPLES);
    let mut allocations = 0;
    for _ in 0..SAMPLES {
        let (ns, allocs) = measured(|| {
            black_box(sim.step(&command).expect("candidate tick"));
        });
        samples.push(ns);
        allocations += allocs;
    }
    report(
        &format!("crossover,{count},{mode:?}"),
        &mut samples,
        allocations,
    );
}

fn paired_tick_case(count: usize, recording: bool) {
    let mut sim = fixture(count, false, ExecutionMode::Serial);
    let mut history = sim
        .begin_history(NonZeroUsize::new(64).expect("window"))
        .expect("history endpoint");
    for _ in 0..80 {
        if recording {
            history.step(&mut sim, &[]).expect("warm recorded tick");
        } else {
            sim.step(&[]).expect("warm tick");
        }
    }
    let mut serial = Vec::with_capacity(SAMPLES);
    let mut rayon = Vec::with_capacity(SAMPLES);
    let mut serial_allocations = 0;
    let mut rayon_allocations = 0;
    for index in 0..SAMPLES {
        let modes = if index % 2 == 0 {
            [ExecutionMode::Serial, ExecutionMode::Rayon]
        } else {
            [ExecutionMode::Rayon, ExecutionMode::Serial]
        };
        for mode in modes {
            sim.set_execution_mode(mode);
            let (ns, count_allocs) = measured(|| {
                if recording {
                    black_box(history.step(&mut sim, &[]).expect("recorded tick"));
                } else {
                    black_box(sim.step(&[]).expect("tick"));
                }
            });
            if mode == ExecutionMode::Serial {
                serial.push(ns);
                serial_allocations += count_allocs;
            } else {
                rayon.push(ns);
                rayon_allocations += count_allocs;
            }
        }
    }
    report(
        &format!("tick,{count},Serial,false,{recording}"),
        &mut serial,
        serial_allocations,
    );
    report(
        &format!("tick,{count},Rayon,false,{recording}"),
        &mut rayon,
        rayon_allocations,
    );
}

fn rewind_case(count: usize, shuffled: bool, mode: ExecutionMode, window: usize, held: bool) {
    let mut sim = fixture(count, shuffled, mode);
    let mut history = sim
        .begin_history(NonZeroUsize::new(window).expect("window"))
        .expect("history endpoint");
    let mut leases = Vec::with_capacity(4);
    if held {
        leases.push(sim.published_page().expect("baseline"));
    }
    for tick in 0..(window + 4) {
        history.step(&mut sim, &[]).expect("warm record");
        if held && tick < 3 {
            leases.push(sim.published_page().expect("held page"));
        }
    }
    let mut rewind_samples = Vec::with_capacity(SAMPLES);
    let mut replay_samples = Vec::with_capacity(SAMPLES);
    let mut rewind_allocs = 0;
    let mut replay_allocs = 0;
    for _ in 0..SAMPLES {
        let (ns, allocs) = measured(|| history.rewind(&mut sim, 8).expect("rewind eight"));
        rewind_samples.push(ns);
        rewind_allocs += allocs;
        let (ns, allocs) = measured(|| {
            for _ in 0..8 {
                black_box(history.replay_next(&mut sim).expect("replay tick"));
            }
        });
        replay_samples.push(ns);
        replay_allocs += allocs;
    }
    black_box(&leases);
    report(
        &format!("rewind8,{count},{mode:?},{shuffled},w{window},held{held}"),
        &mut rewind_samples,
        rewind_allocs,
    );
    report(
        &format!("resim8,{count},{mode:?},{shuffled},w{window},held{held}"),
        &mut replay_samples,
        replay_allocs,
    );
}

fn churn_case(count: usize, mode: ExecutionMode) {
    let mut sim = fixture(count, true, mode);
    let mut history = sim
        .begin_history(NonZeroUsize::new(16).expect("window"))
        .expect("history endpoint");
    let mut id = StableId((count - 20) as u128);
    let mut next = StableId(count as u128);
    let mut samples = Vec::with_capacity(SAMPLES);
    let mut allocations = 0;
    for index in 0..(SAMPLES + 80) {
        let handle = sim.handle(id).expect("churn target");
        let replacement = ComponentEntity {
            id: next,
            position: Position::default(),
            velocity: Velocity([0.05, 0.0, 0.0]),
            health: 100,
            ownership: None,
            visual: Some(1),
        };
        let command = [Command::Despawn(handle), Command::Spawn(replacement)];
        if index < 80 {
            history.step(&mut sim, &command).expect("warm churn");
        } else {
            let (ns, allocs) = measured(|| {
                black_box(history.step(&mut sim, &command).expect("churn tick"));
            });
            samples.push(ns);
            allocations += allocs;
        }
        id = next;
        next = StableId(next.0 + 1);
    }
    report(
        &format!("churn,{count},{mode:?},shuffled"),
        &mut samples,
        allocations,
    );
}

fn admit_four_peers(sim: &mut Simulation, damage: bool, extra_damage: bool, axes: [f32; 3]) {
    let tick = sim.tick() + 1;
    for peer in 1..=4_u64 {
        let mut frame = InputFrame::new(
            Origin {
                peer: PeerId(peer),
                connection_epoch: 1,
                sequence: tick,
            },
            ActorId(peer),
            tick,
            if peer == 1 { axes } else { [0.25, 0.0, 0.0] },
        );
        if damage && peer == 1 {
            frame.actions.push(SimAction::Damage(1));
        }
        if extra_damage && peer == 2 {
            frame.actions.push(SimAction::Damage(1));
        }
        sim.admit_frame(frame).expect("four peer inputs");
    }
}

fn four_peer_case(count: usize, mode: ExecutionMode, damage: bool) {
    let mut sim = fixture(count, true, mode);
    let mut history = sim
        .begin_history(NonZeroUsize::new(16).expect("window"))
        .expect("history endpoint");
    let mut samples = Vec::with_capacity(SAMPLES);
    let mut allocations = 0;
    for index in 0..(SAMPLES + 80) {
        admit_four_peers(&mut sim, damage, false, [0.25, 0.0, 0.0]);
        if index < 80 {
            history.step(&mut sim, &[]).expect("warm four peers");
        } else {
            let (ns, allocs) = measured(|| {
                black_box(history.step(&mut sim, &[]).expect("four-peer tick"));
            });
            samples.push(ns);
            allocations += allocs;
        }
    }
    report(
        &format!("four_peers,{count},{mode:?},shuffled,damage{damage}"),
        &mut samples,
        allocations,
    );
}

fn paired_four_peer_case(count: usize, damage: bool) {
    let mut sim = fixture(count, true, ExecutionMode::Serial);
    let mut history = sim
        .begin_history(NonZeroUsize::new(16).expect("window"))
        .expect("history endpoint");
    for _ in 0..80 {
        admit_four_peers(&mut sim, damage, false, [0.25, 0.0, 0.0]);
        history.step(&mut sim, &[]).expect("warm four peers");
    }
    let mut serial = Vec::with_capacity(SAMPLES);
    let mut rayon = Vec::with_capacity(SAMPLES);
    let mut serial_allocations = 0;
    let mut rayon_allocations = 0;
    for index in 0..SAMPLES {
        let modes = if index % 2 == 0 {
            [ExecutionMode::Serial, ExecutionMode::Rayon]
        } else {
            [ExecutionMode::Rayon, ExecutionMode::Serial]
        };
        for mode in modes {
            sim.set_execution_mode(mode);
            admit_four_peers(&mut sim, damage, false, [0.25, 0.0, 0.0]);
            let (ns, allocs) = measured(|| {
                black_box(history.step(&mut sim, &[]).expect("four-peer tick"));
            });
            if mode == ExecutionMode::Serial {
                serial.push(ns);
                serial_allocations += allocs;
            } else {
                rayon.push(ns);
                rayon_allocations += allocs;
            }
        }
    }
    report(
        &format!("four_peers,{count},Serial,shuffled,damage{damage}"),
        &mut serial,
        serial_allocations,
    );
    report(
        &format!("four_peers,{count},Rayon,shuffled,damage{damage}"),
        &mut rayon,
        rayon_allocations,
    );
}

#[derive(Clone, Copy, Debug)]
enum S1Extra {
    None,
    Command,
    Damage,
}

fn s1_case(count: usize, extra: S1Extra) {
    let mut sim = fixture(count, true, ExecutionMode::Serial);
    let mut history = sim
        .begin_history(NonZeroUsize::new(16).expect("window"))
        .expect("history endpoint");
    let primary = sim.handle(StableId(0)).expect("first static row");
    let secondary = sim.handle(StableId(1)).expect("second static row");
    let mut samples = Vec::with_capacity(SAMPLES);
    let mut allocations = 0;
    for index in 0..(SAMPLES + 80) {
        admit_four_peers(
            &mut sim,
            true,
            matches!(extra, S1Extra::Damage),
            [0.25, 0.0, 0.0],
        );
        let value = if index % 2 == 0 { 99 } else { 100 };
        let commands = [
            Command::SetHealth(primary, value),
            Command::SetHealth(secondary, value),
        ];
        let selected = if matches!(extra, S1Extra::Command) {
            &commands[..2]
        } else {
            &commands[..1]
        };
        if index < 80 {
            history.step(&mut sim, selected).expect("warm S1 row");
        } else {
            let (ns, allocs) = measured(|| {
                black_box(
                    history
                        .step(&mut sim, selected)
                        .expect("S1 tick and publish"),
                );
            });
            samples.push(ns);
            allocations += allocs;
        }
    }
    report(
        &format!("G0,S1,{count},Serial,shuffled,damage1,sethealth1,{extra:?}"),
        &mut samples,
        allocations,
    );
    assert_eq!(allocations, 0, "S1 must not allocate after warm-up");
}

fn measured_s1_step(
    sim: &mut Simulation,
    history: &mut SimulationHistory,
    extra: S1Extra,
    value: u16,
    primary: thrum_core::Handle,
    secondary: thrum_core::Handle,
) -> (u64, u64) {
    admit_four_peers(
        sim,
        true,
        matches!(extra, S1Extra::Damage),
        [0.25, 0.0, 0.0],
    );
    let commands = [
        Command::SetHealth(primary, value),
        Command::SetHealth(secondary, value),
    ];
    let selected = if matches!(extra, S1Extra::Command) {
        &commands[..2]
    } else {
        &commands[..1]
    };
    measured(|| {
        black_box(history.step(sim, selected).expect("paired S1 tick"));
    })
}

fn paired_s1_sensitivity_case(count: usize, extra: S1Extra) {
    let mut base = fixture(count, true, ExecutionMode::Serial);
    let mut changed = fixture(count, true, ExecutionMode::Serial);
    let mut base_history = base
        .begin_history(NonZeroUsize::new(16).expect("window"))
        .expect("base history");
    let mut changed_history = changed
        .begin_history(NonZeroUsize::new(16).expect("window"))
        .expect("changed history");
    let base_handles = [
        base.handle(StableId(0)).expect("base primary"),
        base.handle(StableId(1)).expect("base secondary"),
    ];
    let changed_handles = [
        changed.handle(StableId(0)).expect("changed primary"),
        changed.handle(StableId(1)).expect("changed secondary"),
    ];
    let mut base_samples = Vec::with_capacity(SAMPLES);
    let mut changed_samples = Vec::with_capacity(SAMPLES);
    let mut base_allocations = 0;
    let mut changed_allocations = 0;
    for index in 0..(SAMPLES + 80) {
        let value = if index % 2 == 0 { 99 } else { 100 };
        let mut base_step = || {
            measured_s1_step(
                &mut base,
                &mut base_history,
                S1Extra::None,
                value,
                base_handles[0],
                base_handles[1],
            )
        };
        let mut changed_step = || {
            measured_s1_step(
                &mut changed,
                &mut changed_history,
                extra,
                value,
                changed_handles[0],
                changed_handles[1],
            )
        };
        let (base_result, changed_result) = if index % 2 == 0 {
            (base_step(), changed_step())
        } else {
            let changed_result = changed_step();
            (base_step(), changed_result)
        };
        if index >= 80 {
            base_samples.push(base_result.0);
            changed_samples.push(changed_result.0);
            base_allocations += base_result.1;
            changed_allocations += changed_result.1;
        }
    }
    report(
        &format!("G0,S1_paired,{count},base_vs_{extra:?},base"),
        &mut base_samples,
        base_allocations,
    );
    report(
        &format!("G0,S1_paired,{count},base_vs_{extra:?},extra"),
        &mut changed_samples,
        changed_allocations,
    );
    assert_eq!(
        base_allocations + changed_allocations,
        0,
        "paired S1 must not allocate"
    );
}

fn run_s2_correction(
    sim: &mut Simulation,
    history: &mut SimulationHistory,
    axes: [f32; 3],
    health: thrum_core::Handle,
) {
    history.rewind(sim, 8).expect("rewind eight");
    for replay in 0..8 {
        let first_axes = if replay == 0 { axes } else { [0.25, 0.0, 0.0] };
        admit_four_peers(sim, true, false, first_axes);
        let value = if sim.tick().is_multiple_of(2) {
            99
        } else {
            100
        };
        black_box(
            history
                .step(sim, &[Command::SetHealth(health, value)])
                .expect("recreate corrected tick"),
        );
    }
    black_box(sim.published_page().expect("corrected endpoint published"));
}

fn s2_current_correction_case() {
    let mut sim = fixture(10_000, true, ExecutionMode::Serial);
    let mut history = sim
        .begin_history(NonZeroUsize::new(16).expect("window"))
        .expect("history endpoint");
    let health = sim.handle(StableId(0)).expect("static health row");
    for _ in 0..24 {
        admit_four_peers(&mut sim, true, false, [0.25, 0.0, 0.0]);
        let value = if sim.tick().is_multiple_of(2) {
            99
        } else {
            100
        };
        history
            .step(&mut sim, &[Command::SetHealth(health, value)])
            .expect("warm retained tick");
    }
    let mut samples = Vec::with_capacity(SAMPLES);
    let mut allocations = 0;
    for index in 0..(SAMPLES + 80) {
        let corrected_axes = if index % 2 == 0 {
            [0.5, 0.0, 0.0]
        } else {
            [-0.5, 0.0, 0.0]
        };
        if index < 80 {
            run_s2_correction(&mut sim, &mut history, corrected_axes, health);
            continue;
        }
        let (ns, allocs) = measured(|| {
            run_s2_correction(&mut sim, &mut history, corrected_axes, health);
        });
        samples.push(ns);
        allocations += allocs;
    }
    report(
        "G0,S2,10000,Serial,shuffled,damage1,sethealth1,branch_recreate_without_record_amend",
        &mut samples,
        allocations,
    );
}

fn run_s2_retained_correction(
    sim: &mut Simulation,
    history: &mut SimulationHistory,
    axes: [f32; 3],
) -> [u64; 4] {
    let started = Instant::now();
    history.rewind(sim, 8).expect("rewind eight");
    let rewound = Instant::now();
    let tick = sim.tick() + 1;
    let mut frame = InputFrame::new(
        Origin {
            peer: PeerId(1),
            connection_epoch: 1,
            sequence: tick,
        },
        ActorId(1),
        tick,
        axes,
    );
    frame.actions.push(SimAction::Damage(1));
    history
        .amend_input(sim, frame)
        .expect("amend retained input");
    let amended = Instant::now();
    for _ in 0..8 {
        black_box(history.replay_next(sim).expect("retained replay"));
    }
    let replayed = Instant::now();
    black_box(sim.published_page().expect("corrected endpoint published"));
    let published = Instant::now();
    [
        nanos(rewound.duration_since(started)),
        nanos(amended.duration_since(rewound)),
        nanos(replayed.duration_since(amended)),
        nanos(published.duration_since(replayed)),
    ]
}

fn nanos(duration: std::time::Duration) -> u64 {
    u64::try_from(duration.as_nanos()).expect("sample fits u64")
}

fn s2_retained_correction_case() {
    let mut sim = fixture(10_000, true, ExecutionMode::Serial);
    let mut history = sim
        .begin_history(NonZeroUsize::new(16).expect("window"))
        .expect("history endpoint");
    let health = sim.handle(StableId(0)).expect("static health row");
    for _ in 0..24 {
        admit_four_peers(&mut sim, true, false, [0.25, 0.0, 0.0]);
        let value = if sim.tick().is_multiple_of(2) {
            99
        } else {
            100
        };
        history
            .step(&mut sim, &[Command::SetHealth(health, value)])
            .expect("warm retained tick");
    }
    let mut samples = Vec::with_capacity(SAMPLES);
    let mut stage_samples: [Vec<u64>; 4] = std::array::from_fn(|_| Vec::with_capacity(SAMPLES));
    let mut allocations = 0;
    for index in 0..(SAMPLES + 80) {
        let axes = if index % 2 == 0 {
            [0.5, 0.0, 0.0]
        } else {
            [-0.5, 0.0, 0.0]
        };
        if index < 80 {
            black_box(run_s2_retained_correction(&mut sim, &mut history, axes));
        } else {
            let mut stages = [0; 4];
            let (ns, allocs) = measured(|| {
                stages = run_s2_retained_correction(&mut sim, &mut history, axes);
            });
            samples.push(ns);
            for (column, stage) in stage_samples.iter_mut().zip(stages) {
                column.push(stage);
            }
            allocations += allocs;
        }
    }
    report(
        "G2S,S2,10000,Serial,shuffled,damage1,sethealth1,retained_amend_replay_publish",
        &mut samples,
        allocations,
    );
    for (stage, values) in [
        "rewind",
        "amend",
        "replay8_including_final_publish",
        "page_lease",
    ]
    .into_iter()
    .zip(&mut stage_samples)
    {
        report(&format!("G2S_profile,{stage}"), values, 0);
    }
}

fn s1_spawn_burst_case() {
    const LIVE: usize = 10_000;
    const BURST: usize = 200;
    let mut sim = fixture_with_capacity(LIVE, LIVE + BURST, true, ExecutionMode::Serial);
    let mut history = sim
        .begin_history(NonZeroUsize::new(16).expect("window"))
        .expect("history endpoint");
    let health = sim.handle(StableId(0)).expect("static health row");
    let mut spawn = Vec::with_capacity(BURST + 1);
    spawn.push(Command::SetHealth(health, 99));
    for index in 0..BURST {
        spawn.push(Command::Spawn(ComponentEntity {
            id: StableId((LIVE + index) as u128),
            position: Position([index as f64 * 0.001, 0.0, 0.0]),
            velocity: Velocity::default(),
            health: 100,
            ownership: None,
            visual: Some(1),
        }));
    }
    let mut despawn = Vec::with_capacity(BURST);
    let mut samples = Vec::with_capacity(SAMPLES);
    let mut allocations = 0;
    for index in 0..(SAMPLES + 80) {
        spawn[0] = Command::SetHealth(health, if index % 2 == 0 { 99 } else { 100 });
        admit_four_peers(&mut sim, true, false, [0.25, 0.0, 0.0]);
        if index < 80 {
            history.step(&mut sim, &spawn).expect("warm burst spawn");
        } else {
            let (ns, allocs) = measured(|| {
                black_box(history.step(&mut sim, &spawn).expect("S1 burst tick"));
            });
            samples.push(ns);
            allocations += allocs;
        }
        despawn.clear();
        for id in LIVE..(LIVE + BURST) {
            despawn.push(Command::Despawn(
                sim.handle(StableId(id as u128)).expect("spawned burst row"),
            ));
        }
        history.step(&mut sim, &despawn).expect("retire burst rows");
    }
    report(
        "G0,S1,10000,Serial,shuffled,damage1,sethealth1,spawn200",
        &mut samples,
        allocations,
    );
    assert_eq!(
        allocations, 0,
        "spawn burst must not allocate after warm-up"
    );
}

#[expect(
    clippy::print_stdout,
    reason = "the explicit benchmark emits its reproducible evidence table"
)]
fn main() {
    println!("case,p50_ns,p95_ns,p99_ns,allocations_all_threads");
    if std::env::args().any(|argument| argument == "--g2s-qualify") {
        for count in [10_000, 100_000] {
            for extra in [S1Extra::None, S1Extra::Command, S1Extra::Damage] {
                s1_case(count, extra);
            }
            for extra in [S1Extra::Command, S1Extra::Damage] {
                paired_s1_sensitivity_case(count, extra);
            }
            for mode in [
                ExecutionMode::Serial,
                ExecutionMode::Rayon,
                ExecutionMode::RayonForced,
            ] {
                crossover_case(count, mode);
            }
        }
        s2_retained_correction_case();
        s1_spawn_burst_case();
        return;
    }
    if std::env::args().any(|argument| argument == "--profile-recorded-10k") {
        let mut sim = fixture(10_000, false, ExecutionMode::Serial);
        let mut history = sim
            .begin_history(NonZeroUsize::new(64).expect("window"))
            .expect("history endpoint");
        for _ in 0..100_000 {
            black_box(history.step(&mut sim, &[]).expect("recorded tick"));
        }
        return;
    }
    if std::env::args().any(|argument| argument == "--profile-g2s-s1") {
        let mut sim = fixture(10_000, true, ExecutionMode::Serial);
        let mut history = sim
            .begin_history(NonZeroUsize::new(16).expect("window"))
            .expect("history endpoint");
        let health = sim.handle(StableId(0)).expect("static health row");
        for index in 0..100_000 {
            admit_four_peers(&mut sim, true, false, [0.25, 0.0, 0.0]);
            black_box(
                history
                    .step(
                        &mut sim,
                        &[Command::SetHealth(
                            health,
                            if index % 2 == 0 { 99 } else { 100 },
                        )],
                    )
                    .expect("profile S1 tick"),
            );
        }
        return;
    }
    if std::env::args().any(|argument| argument == "--profile-g2s-s2") {
        let mut sim = fixture(10_000, true, ExecutionMode::Serial);
        let mut history = sim
            .begin_history(NonZeroUsize::new(16).expect("window"))
            .expect("history endpoint");
        let health = sim.handle(StableId(0)).expect("static health row");
        for _ in 0..24 {
            admit_four_peers(&mut sim, true, false, [0.25, 0.0, 0.0]);
            history
                .step(&mut sim, &[Command::SetHealth(health, 99)])
                .expect("warm retained tick");
        }
        for index in 0..20_000 {
            let axes = if index % 2 == 0 {
                [0.5, 0.0, 0.0]
            } else {
                [-0.5, 0.0, 0.0]
            };
            black_box(run_s2_retained_correction(&mut sim, &mut history, axes));
        }
        return;
    }
    if std::env::args().any(|argument| argument == "--tick") {
        for count in [10_000, 100_000] {
            paired_tick_case(count, false);
            paired_tick_case(count, true);
        }
        return;
    }
    if std::env::args().any(|argument| argument == "--four-peer") {
        for count in [10_000, 100_000] {
            for mode in [ExecutionMode::Serial, ExecutionMode::Rayon] {
                four_peer_case(count, mode, false);
                four_peer_case(count, mode, true);
            }
        }
        return;
    }
    if std::env::args().any(|argument| argument == "--g0-s1") {
        for count in [10_000, 100_000] {
            for extra in [S1Extra::None, S1Extra::Command, S1Extra::Damage] {
                s1_case(count, extra);
            }
        }
        return;
    }
    if std::env::args().any(|argument| argument == "--g0-s1-paired") {
        for count in [10_000, 100_000] {
            for extra in [S1Extra::Command, S1Extra::Damage] {
                paired_s1_sensitivity_case(count, extra);
            }
        }
        return;
    }
    if std::env::args().any(|argument| argument == "--g0-s2") {
        s2_current_correction_case();
        return;
    }
    if std::env::args().any(|argument| argument == "--g2s-s2") {
        s2_retained_correction_case();
        return;
    }
    if std::env::args().any(|argument| argument == "--g0-burst") {
        s1_spawn_burst_case();
        return;
    }
    if std::env::args().any(|argument| argument == "--crossover") {
        for count in [10_000, 100_000, 200_000, 500_000] {
            for mode in [
                ExecutionMode::Serial,
                ExecutionMode::Rayon,
                ExecutionMode::RayonForced,
            ] {
                crossover_case(count, mode);
            }
        }
        return;
    }
    if std::env::args().any(|argument| argument == "--focus") {
        churn_case(10_000, ExecutionMode::Serial);
        rewind_case(100_000, true, ExecutionMode::Serial, 16, false);
        rewind_case(100_000, true, ExecutionMode::Serial, 64, false);
        return;
    }
    for count in [10_000, 100_000] {
        paired_tick_case(count, false);
        paired_tick_case(count, true);
        paired_four_peer_case(count, false);
        paired_four_peer_case(count, true);
        churn_case(count, ExecutionMode::Serial);
        for window in [16, 64] {
            rewind_case(count, true, ExecutionMode::Serial, window, false);
        }
        rewind_case(count, true, ExecutionMode::Serial, 16, true);
    }
    for count in [10_000, 100_000, 200_000, 500_000] {
        for mode in [ExecutionMode::Serial, ExecutionMode::RayonForced] {
            crossover_case(count, mode);
        }
    }
}
