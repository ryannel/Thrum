//! Running-game authority checks after the G2 boundary migration.
#![allow(unsafe_code, reason = "test counting allocator delegates to System")]

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    num::{NonZeroU32, NonZeroUsize},
};

use thrum_core::{
    ActorId, Origin, PeerId, StableId, WorldEpoch,
    sim::{ExecutionMode, InputFrame},
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
fn rolling_history_reuses_deltas_and_replays_after_churn() {
    let mut sim = Simulation::new(WorldEpoch(24), config(3)).expect("game");
    let owned = Ownership {
        peer: PeerId(2),
        connection_epoch: 1,
        actor: ActorId(7),
    };
    let mut player = entity(1, 0.0);
    player.ownership = Some(owned);
    let player_handle = sim.seed(player).expect("player");
    let old = sim.seed(entity(2, 1.0)).expect("mover");
    sim.register_peer(owned, player_handle).expect("peer");
    let mut history = sim.begin_history(NonZeroUsize::new(4).expect("window"));
    sim.step(&[Command::Despawn(old), Command::Spawn(entity(3, 2.0))])
        .expect("churn");
    history.record(&sim).expect("first delta");
    let replacement = sim.handle(StableId(3)).expect("replacement");
    let mut frame = InputFrame::new(
        Origin {
            peer: owned.peer,
            connection_epoch: 1,
            sequence: 1,
        },
        owned.actor,
        2,
        [1.0, 0.0, 0.0],
    );
    frame.actions.push(SimAction::Damage(9));
    sim.admit_frame(frame).expect("admitted");
    sim.step(&[]).expect("input tick");
    history.record(&sim).expect("input delta");
    for _ in 0..2 {
        sim.step(&[]).expect("moving tick");
        history.record(&sim).expect("moving delta");
    }
    let target = sim.tick();
    assert_eq!(target, 4);
    let command = Command::SetVelocity(replacement, Velocity([3.0, 0.0, 0.0]));
    sim.step(&[command]).expect("future tick");
    history.record(&sim).expect("future delta");
    let expected = sim.published_page().expect("future page");
    sim.step(&[]).expect("advance window");
    history.record(&sim).expect("bounded delta");
    assert_eq!((history.oldest_tick(), history.latest_tick()), (2, 6));
    history
        .restore_to(&mut sim, target)
        .expect("retained endpoint");
    assert!(sim.published_page().expect("replay baseline").is_baseline());
    assert_eq!(sim.tick(), 4);
    sim.step(&[command]).expect("replayed future");
    history.record(&sim).expect("replayed delta");
    let actual = sim.published_page().expect("replayed page");
    assert_eq!(actual.position(replacement), expected.position(replacement));
    assert_eq!(actual.velocity(replacement), expected.velocity(replacement));
    assert_eq!(actual.health(player_handle), expected.health(player_handle));
    assert_eq!(actual.structural_events(), expected.structural_events());
    assert_eq!(history.latest_tick(), 5);
}

#[test]
fn rolling_history_has_bounded_reused_storage_at_game_scales() {
    for count in [10_000, 100_000] {
        let mut sim = simulation_with_count(count).expect("game fixture");
        let full_bytes = sim.capture().reserved_bytes();
        let mut history = sim.begin_history(NonZeroUsize::new(64).expect("window"));
        for _ in 0..65 {
            sim.step(&[]).expect("warm tick");
            history.record(&sim).expect("warm delta");
        }
        let retained_bytes = history.reserved_bytes();
        assert!(retained_bytes < full_bytes * 6);
        ALLOCATIONS.with(|count| count.set(0));
        COUNTING.with(|active| active.set(true));
        for _ in 0..100 {
            sim.step(&[]).expect("steady tick");
            history.record(&sim).expect("reused delta");
        }
        COUNTING.with(|active| active.set(false));
        assert_eq!(ALLOCATIONS.with(Cell::get), 0);
    }
}

#[test]
fn rolling_history_refuses_a_divergent_same_tick_future() {
    let mut sim = Simulation::new(WorldEpoch(30), config(2)).expect("game");
    let moving = sim.seed(entity(1, 1.0)).expect("moving entity");
    let mut history = sim.begin_history(NonZeroUsize::new(4).expect("window"));
    sim.step(&[]).expect("tick one");
    history.record(&sim).expect("first delta");
    let saved = sim.capture();
    sim.step(&[Command::SetVelocity(moving, Velocity([2.0, 0.0, 0.0]))])
        .expect("first future");
    history.record(&sim).expect("first future delta");
    sim.restore(&saved).expect("branch point");
    sim.step(&[Command::SetVelocity(moving, Velocity([3.0, 0.0, 0.0]))])
        .expect("divergent future");
    sim.step(&[]).expect("numerically next tick");
    assert_eq!(history.record(&sim), Err(SimError::InvalidSnapshot));
    assert_eq!(history.latest_tick(), 2);
}

#[test]
fn leased_pages_drop_publication_but_never_block_authority_or_lose_recovery_baseline() {
    let mut sim = Simulation::new(WorldEpoch(15), config(2)).expect("game");
    let owned = Ownership {
        peer: PeerId(3),
        connection_epoch: 1,
        actor: ActorId(3),
    };
    let mut body = entity(1, 0.0);
    body.ownership = Some(owned);
    let handle = sim.seed(body).expect("body");
    sim.seed(entity(2, 0.0)).expect("second body");
    sim.register_peer(owned, handle).expect("peer");
    let zero = sim.published_page().expect("baseline");
    sim.step(&[]).expect("tick one");
    let one = sim.published_page().expect("tick one page");
    sim.step(&[]).expect("tick two");
    let two = sim.published_page().expect("tick two page");
    sim.step(&[]).expect("tick three");
    let three = sim.published_page().expect("tick three page");
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
    sim.admit_frame(input)
        .expect("publication pressure does not block input");
    sim.step(&[])
        .expect("tick four commits without a free page");
    assert_eq!(sim.tick(), 4);
    assert!(!sim.has_current_page());
    assert!(matches!(
        sim.published_page(),
        Err(SimError::PublicationBusy)
    ));
    let second = sim.handle(StableId(2)).expect("second handle");
    sim.step(&[Command::Despawn(second)])
        .expect("churn commits while pages are held");
    sim.step(&[Command::Spawn(entity(3, 0.0))])
        .expect("replacement commits while pages are held");
    assert_eq!(sim.tick(), 6);
    assert_eq!(zero.id().tick, 0);
    assert_eq!(one.id().tick, 1);
    assert_eq!(two.id().tick, 2);
    assert_eq!(three.id().tick, 3);
    drop(zero);
    let recovered = sim.published_page().expect("zero-tick recovery page");
    assert_eq!(sim.tick(), 6, "recovery cannot apply the tick twice");
    assert_eq!(recovered.id().tick, 6);
    assert!(recovered.is_baseline());
    assert_eq!(recovered.len(), 2);
    assert!(recovered.position(handle).expect("moving body").0[0] > 0.0);
    assert!(recovered.stable_id(second.slot).is_some());
    assert_eq!(recovered.stable_id(second.slot), Some(StableId(3)));
    assert_eq!(recovered.dirty().position().len(), 2);
    assert_eq!(recovered.structural_events().len(), 2);
    drop(one);
    sim.step(&[]).expect("authority continues after release");
    assert_eq!(sim.tick(), 7);
    assert!(!sim.published_page().expect("adjacent page").is_baseline());
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

#[test]
fn restore_after_swap_remove_preserves_canonical_dirty_order() {
    let mut sim = Simulation::new(WorldEpoch(24), config(6)).expect("game");
    for id in 1..=6 {
        sim.seed(entity(id, 1.0)).expect("moving entity");
    }
    sim.published_page().expect("baseline");
    let retired = sim.handle(StableId(2)).expect("retired handle");
    sim.step(&[Command::Despawn(retired), Command::Spawn(entity(7, 1.0))])
        .expect("swap-remove churn");
    let capture = sim.capture();
    sim.step(&[]).expect("first future tick");
    let expected = sim
        .published_page()
        .expect("first future page")
        .dirty()
        .position()
        .to_vec();
    sim.restore(&capture).expect("restore after churn");
    sim.step(&[]).expect("replayed future tick");
    let replayed = sim
        .published_page()
        .expect("replayed page")
        .dirty()
        .position()
        .to_vec();
    assert_eq!(
        replayed, expected,
        "restore cannot change the published order"
    );
    assert_eq!(replayed.as_slice(), &[0, 1, 2, 3, 4, 5]);
}
