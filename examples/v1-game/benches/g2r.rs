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
    Command, ComponentEntity, Ownership, Position, SimAction, SimConfig, Simulation, Velocity,
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
    let mut sim = Simulation::new(
        WorldEpoch(1),
        SimConfig {
            tick_hz: NonZeroU32::new(60).expect("tick rate"),
            entity_capacity: count,
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
    if !label.starts_with("crossover,") {
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

fn admit_four_peers(sim: &mut Simulation, damage: bool) {
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
            [0.25, 0.0, 0.0],
        );
        if damage && peer == 1 {
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
        admit_four_peers(&mut sim, damage);
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
        admit_four_peers(&mut sim, damage);
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
            admit_four_peers(&mut sim, damage);
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

#[expect(
    clippy::print_stdout,
    reason = "the explicit benchmark emits its reproducible evidence table"
)]
fn main() {
    println!("case,p50_ns,p95_ns,p99_ns,allocations_all_threads");
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
    if std::env::args().any(|argument| argument == "--crossover") {
        for count in [10_000, 100_000, 200_000, 500_000] {
            for mode in [ExecutionMode::Serial, ExecutionMode::RayonForced] {
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
