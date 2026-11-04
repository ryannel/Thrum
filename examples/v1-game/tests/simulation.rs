//! Running-game authority checks after the G2 boundary migration.
#![allow(unsafe_code, reason = "test counting allocator delegates to System")]

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    num::NonZeroU32,
};

use thrum_core::{
    ActorId, Origin, PeerId, StableId, WorldEpoch,
    sim::{ExecutionMode, FrameError, InputFrame},
};
use v1_game::simulation::{
    Command, ComponentEntity, Ownership, Position, SimAction, SimConfig, SimError, Simulation,
    Velocity,
};
use v1_game::simulation_with_count;

thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
}

struct CountingAllocator;

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

// SAFETY: All operations forward their original arguments to System.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        COUNTING.with(|active| {
            if active.get() {
                ALLOCATIONS.with(|count| count.set(count.get() + 1));
            }
        });
        // SAFETY: The same layout is forwarded to System.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: The original pointer and layout are forwarded to System.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        COUNTING.with(|active| {
            if active.get() {
                ALLOCATIONS.with(|count| count.set(count.get() + 1));
            }
        });
        // SAFETY: The original pointer and layout are forwarded to System.
        unsafe { System.realloc(ptr, layout, size) }
    }
}

fn config(capacity: usize) -> SimConfig {
    SimConfig {
        tick_hz: NonZeroU32::new(60).expect("nonzero rate"),
        entity_capacity: capacity,
        peer_capacity: 4,
        actions_per_frame: 8,
        movement_speed: 6.0,
    }
}

fn entity(id: u128, speed: f64) -> ComponentEntity {
    ComponentEntity {
        id: StableId(id),
        position: Position::default(),
        velocity: Velocity([speed, 0.0, 0.0]),
        health: 100,
        ownership: None,
        visual: Some(2),
    }
}

#[test]
fn failed_game_tick_preserves_input_and_page_then_churn_reuses_checked_slot() {
    let mut sim = Simulation::new(WorldEpoch(9), config(2)).expect("game");
    let owned = Ownership {
        peer: PeerId(1),
        connection_epoch: 1,
        actor: ActorId(1),
    };
    let mut player = entity(1, 0.0);
    player.ownership = Some(owned);
    let player_handle = sim.seed(player).expect("player");
    let old = sim.seed(entity(2, 1.0)).expect("mover");
    sim.register_peer(owned, player_handle).expect("binding");
    let retained = sim.published_page().expect("baseline");
    let mut frame = InputFrame::new(
        Origin {
            peer: owned.peer,
            connection_epoch: owned.connection_epoch,
            sequence: 1,
        },
        owned.actor,
        1,
        [1.0, 0.0, 0.0],
    );
    frame.actions.push(SimAction::Damage(7));
    sim.admit_frame(frame).expect("admitted frame");
    assert_eq!(
        sim.step(&[Command::SetPosition(old, Position([f64::NAN, 0.0, 0.0]))]),
        Err(SimError::NonFinite)
    );
    assert_eq!(sim.tick(), 0);
    assert_eq!(retained.id().tick, 0);
    assert_eq!(sim.get(player_handle).expect("player").health, 100);

    let committed = sim
        .step(&[Command::Despawn(old), Command::Spawn(entity(3, 2.0))])
        .expect("whole tick");
    assert_eq!(committed.tick, 1);
    assert_eq!(committed.committed_inputs.len(), 1);
    let replacement = sim.handle(StableId(3)).expect("new entity");
    assert_eq!(replacement.slot, old.slot);
    assert_ne!(replacement.generation, old.generation);
    assert_eq!(sim.get(old), Err(SimError::InvalidHandle));
    assert_eq!(sim.get(player_handle).expect("player").health, 93);
    assert_eq!(retained.stable_id(old.slot), Some(StableId(2)));
    let current = sim.published_page().expect("committed page");
    assert_eq!(current.stable_id(old.slot), Some(StableId(3)));
    assert_eq!(current.id().tick, 1);
}

#[test]
fn ten_thousand_running_game_ticks_reuse_allocations() {
    let mut sim = Simulation::new(WorldEpoch(4), config(10_000)).expect("game");
    for index in 0usize..10_000 {
        let speed = if index < 1_500 { 1.0 } else { 0.0 };
        sim.seed(entity(index as u128, speed)).expect("seed");
    }
    for _ in 0..60 {
        sim.step(&[]).expect("warmup");
    }
    COUNTING.with(|active| active.set(true));
    for _ in 0..100 {
        sim.step(&[]).expect("steady tick");
    }
    COUNTING.with(|active| active.set(false));
    assert_eq!(ALLOCATIONS.with(Cell::get), 0);
    assert_eq!(sim.tick(), 160);
    assert_eq!(sim.published_page().expect("page").len(), 10_000);
}

#[test]
fn warmed_spawn_churn_reuses_tick_storage() {
    let mut sim = Simulation::new(WorldEpoch(18), config(2)).expect("game");
    let mut old = sim.seed(entity(1, 1.0)).expect("mover");
    for next_id in 2..22 {
        sim.step(&[Command::Despawn(old), Command::Spawn(entity(next_id, 1.0))])
            .expect("warm churn");
        old = sim.handle(StableId(next_id)).expect("new handle");
    }
    ALLOCATIONS.with(|count| count.set(0));
    COUNTING.with(|active| active.set(true));
    for next_id in 22..122 {
        sim.step(&[Command::Despawn(old), Command::Spawn(entity(next_id, 1.0))])
            .expect("steady churn");
        old = sim.handle(StableId(next_id)).expect("new handle");
    }
    COUNTING.with(|active| active.set(false));
    assert_eq!(ALLOCATIONS.with(Cell::get), 0);
}

#[test]
fn capture_after_churn_replays_input_and_structure_to_the_same_endpoint() {
    let mut sim = Simulation::new(WorldEpoch(12), config(3)).expect("game");
    let owned = Ownership {
        peer: PeerId(5),
        connection_epoch: 2,
        actor: ActorId(9),
    };
    let mut player = entity(1, 0.0);
    player.ownership = Some(owned);
    let player_handle = sim.seed(player).expect("player");
    let old = sim.seed(entity(2, 1.0)).expect("mover");
    sim.register_peer(owned, player_handle).expect("binding");
    sim.step(&[Command::Despawn(old), Command::Spawn(entity(3, 2.0))])
        .expect("churn before capture");
    let replacement = sim.handle(StableId(3)).expect("replacement");
    let capture = sim.capture();
    let mut input = InputFrame::new(
        Origin {
            peer: owned.peer,
            connection_epoch: owned.connection_epoch,
            sequence: 8,
        },
        owned.actor,
        2,
        [1.0, 0.0, 0.0],
    );
    input.actions.push(SimAction::Damage(11));
    let replay = |sim: &mut Simulation| {
        sim.admit_frame(input.clone()).expect("same admitted input");
        sim.step(&[Command::SetVelocity(replacement, Velocity([3.0, 0.0, 0.0]))])
            .expect("resolved velocity");
        sim.step(&[
            Command::Despawn(replacement),
            Command::Spawn(entity(4, 4.0)),
        ])
        .expect("second churn");
        let page = sim.published_page().expect("endpoint");
        let mut rows = Vec::new();
        page.visit(|handle, id, position, visual| {
            rows.push((id, handle, position, page.health(handle), visual));
        });
        (
            page.id(),
            rows,
            page.dirty().position().to_vec(),
            page.dirty().structural().to_vec(),
            page.structural_events().to_vec(),
        )
    };
    let expected = replay(&mut sim);
    sim.restore(&capture).expect("compatible committed capture");
    sim.set_execution_mode(ExecutionMode::Rayon);
    assert_eq!(sim.tick(), 1);
    assert_eq!(sim.handle(StableId(3)), Some(replacement));
    assert_eq!(sim.get(old), Err(SimError::InvalidHandle));
    assert_eq!(replay(&mut sim), expected);
}

#[test]
fn leased_pages_refuse_tick_and_new_input_until_a_reader_releases_one() {
    let mut sim = Simulation::new(WorldEpoch(15), config(1)).expect("game");
    let owned = Ownership {
        peer: PeerId(3),
        connection_epoch: 1,
        actor: ActorId(3),
    };
    let mut body = entity(1, 0.0);
    body.ownership = Some(owned);
    let handle = sim.seed(body).expect("body");
    sim.register_peer(owned, handle).expect("peer");
    let zero = sim.published_page().expect("baseline");
    sim.step(&[]).expect("tick one");
    let one = sim.published_page().expect("tick one page");
    sim.step(&[]).expect("tick two");
    let two = sim.published_page().expect("tick two page");
    sim.step(&[]).expect("tick three");
    let three = sim.published_page().expect("tick three page");
    assert_eq!(sim.step(&[]), Err(SimError::PublicationBusy));
    let input = InputFrame::<SimAction>::new(
        Origin {
            peer: owned.peer,
            connection_epoch: owned.connection_epoch,
            sequence: 1,
        },
        owned.actor,
        4,
        [1.0, 0.0, 0.0],
    );
    assert_eq!(
        sim.admit_frame(input.clone()),
        Err(FrameError::PublicationBusy)
    );
    assert_eq!(sim.tick(), 3);
    drop(zero);
    sim.admit_frame(input).expect("released page permits input");
    sim.step(&[]).expect("exact retry");
    assert_eq!(sim.tick(), 4);
    assert_eq!(one.id().tick, 1);
    assert_eq!(two.id().tick, 2);
    assert_eq!(three.id().tick, 3);
}

#[test]
fn four_peer_ten_thousand_entity_serial_and_rayon_replay_match() {
    let mut sim = simulation_with_count(10_000).expect("running game fixture");
    let old = sim.handle(StableId(9_995)).expect("static body");
    let capture = sim.capture();
    let run = |sim: &mut Simulation| {
        for peer_number in 1..=4u64 {
            let mut frame = InputFrame::new(
                Origin {
                    peer: PeerId(peer_number),
                    connection_epoch: 1,
                    sequence: 1,
                },
                ActorId(peer_number),
                1,
                [0.25, 0.0, 0.0],
            );
            frame.actions.push(SimAction::Damage(1));
            sim.admit_frame(frame).expect("one frame per peer");
        }
        assert_eq!(
            sim.step(&[Command::SetVelocity(old, Velocity([f64::NAN, 0.0, 0.0]))]),
            Err(SimError::NonFinite)
        );
        let first = sim
            .step(&[Command::Despawn(old), Command::Spawn(entity(20_000, 1.0))])
            .expect("whole churn tick");
        let origins = first.committed_inputs.to_vec();
        sim.step(&[]).expect("held controls tick");
        let page = sim.published_page().expect("complete endpoint");
        let mut rows = Vec::with_capacity(10_000);
        page.visit(|handle, id, position, _| {
            rows.push((id, handle, position, page.health(handle)));
        });
        let mut positions = page.dirty().position().to_vec();
        positions.sort_unstable();
        (
            page.id(),
            origins,
            rows,
            positions,
            page.structural_events().to_vec(),
        )
    };
    let serial = run(&mut sim);
    sim.restore(&capture).expect("tick-zero capture");
    sim.set_execution_mode(ExecutionMode::Rayon);
    assert_eq!(run(&mut sim), serial);
}
