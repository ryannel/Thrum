//! Running-game authority checks after the G2 boundary migration.
#![allow(unsafe_code, reason = "test counting allocator delegates to System")]

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    hash::{Hash, Hasher},
    num::{NonZeroU32, NonZeroUsize},
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

fn whole_world_hash(sim: &Simulation, ids: &[StableId]) -> u64 {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    sim.len().hash(&mut hash);
    sim.tick().hash(&mut hash);
    for id in ids {
        let handle = sim.handle(*id).expect("tracked world identity");
        let row = sim.get(handle).expect("committed world row");
        id.0.hash(&mut hash);
        handle.slot.hash(&mut hash);
        handle.generation.hash(&mut hash);
        row.position.0.map(f64::to_bits).hash(&mut hash);
        row.velocity.0.map(f64::to_bits).hash(&mut hash);
        row.health.hash(&mut hash);
        row.ownership
            .map(|owner| (owner.peer.0, owner.connection_epoch, owner.actor.0))
            .hash(&mut hash);
        row.visual.hash(&mut hash);
    }
    hash.finish()
}

fn randomized_replay_hashes(mode: ExecutionMode) -> Vec<u64> {
    let mut sim = simulation_with_count(256).expect("four-peer game");
    sim.set_execution_mode(mode);
    let mut history = sim
        .begin_history(NonZeroUsize::new(16).expect("window"))
        .expect("committed endpoint");
    let mut ids = (0..256_u128).map(StableId).collect::<Vec<_>>();
    let mut random = 0x9e37_79b9_7f4a_7c15_u64;
    let mut hashes = Vec::with_capacity(125);
    for (next_id, tick) in (256_u128..).zip(1..=1_000_u64) {
        random = random
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1);
        let churn_index = 8 + usize::try_from(random % 244).expect("small index");
        let old = sim.handle(ids[churn_index]).expect("churned row");
        let replacement = ComponentEntity {
            id: StableId(next_id),
            position: Position([f64::from((random & 127) as u8), 0.0, 0.0]),
            velocity: Velocity([if tick % 3 == 0 { 0.05 } else { 0.0 }, 0.0, 0.0]),
            health: 100,
            ownership: None,
            visual: Some(random & 3),
        };
        let health = sim.handle(ids[4]).expect("health target");
        let commands = [
            Command::Despawn(old),
            Command::Spawn(replacement),
            Command::SetHealth(health, u16::try_from(random & 127).expect("health")),
        ];
        for peer in 1..=4_u64 {
            let mut frame = InputFrame::new(
                Origin {
                    peer: PeerId(peer),
                    connection_epoch: 1,
                    sequence: tick,
                },
                ActorId(peer),
                tick,
                [
                    f32::from(u8::try_from((random >> peer) & 0xff).expect("byte")) / 255.0,
                    0.0,
                    0.0,
                ],
            );
            if peer == 1 && tick % 7 == 0 {
                frame.actions.push(SimAction::Damage(1));
            }
            sim.admit_frame(frame).expect("fresh randomized peer input");
        }
        history.step(&mut sim, &commands).expect("randomized tick");
        ids[churn_index] = StableId(next_id);
        if tick % 8 == 0 {
            let expected = whole_world_hash(&sim, &ids);
            history.rewind(&mut sim, 8).expect("retained eight ticks");
            for _ in 0..8 {
                assert!(history.replay_next(&mut sim).expect("valid retained input"));
            }
            assert_eq!(
                whole_world_hash(&sim, &ids),
                expected,
                "mode {mode:?} tick {tick}"
            );
            hashes.push(expected);
        }
    }
    hashes
}

#[test]
fn thousand_randomized_world_ticks_replay_identically_in_serial_and_rayon() {
    assert_eq!(
        randomized_replay_hashes(ExecutionMode::Serial),
        randomized_replay_hashes(ExecutionMode::RayonForced),
    );
}

#[test]
fn repeated_mover_preimages_restore_the_first_state_and_replay() {
    for mode in [ExecutionMode::Serial, ExecutionMode::RayonForced] {
        let mut sim = simulation_with_count(256).expect("game");
        sim.set_execution_mode(mode);
        let ids = (0..256_u128).map(StableId).collect::<Vec<_>>();
        let before = whole_world_hash(&sim, &ids);
        let mut history = sim
            .begin_history(NonZeroUsize::new(16).expect("window"))
            .expect("history");
        for _ in 0..8 {
            history.step(&mut sim, &[]).expect("moving tick");
        }
        let after = whole_world_hash(&sim, &ids);
        assert_ne!(before, after, "mode {mode:?} did not exercise movement");
        history.rewind(&mut sim, 8).expect("eight-tick rewind");
        assert_eq!(whole_world_hash(&sim, &ids), before, "mode {mode:?}");
        for _ in 0..8 {
            assert!(history.replay_next(&mut sim).expect("retained tick"));
        }
        assert_eq!(whole_world_hash(&sim, &ids), after, "mode {mode:?}");
    }
}

#[test]
fn rewind_after_partial_replay_retains_the_remaining_future() {
    let mut sim = simulation_with_count(32).expect("game");
    let ids = (0..32_u128).map(StableId).collect::<Vec<_>>();
    let mut history = sim
        .begin_history(NonZeroUsize::new(16).expect("window"))
        .expect("history");
    for _ in 0..8 {
        history.step(&mut sim, &[]).expect("record");
    }
    let expected = whole_world_hash(&sim, &ids);
    history.rewind(&mut sim, 4).expect("first rewind");
    for _ in 0..2 {
        assert!(history.replay_next(&mut sim).expect("first replay"));
    }
    history.rewind(&mut sim, 2).expect("second rewind");
    for _ in 0..4 {
        assert!(history.replay_next(&mut sim).expect("retained replay"));
    }
    assert!(!history.replay_next(&mut sim).expect("future exhausted"));
    assert_eq!(whole_world_hash(&sim, &ids), expected);
}

#[test]
fn corrected_retained_peer_input_matches_the_straight_player_row() {
    let mut predicted = simulation_with_count(4).expect("predicted game");
    let mut straight = simulation_with_count(4).expect("straight game");
    let mut history = predicted
        .begin_history(NonZeroUsize::new(16).expect("window"))
        .expect("history");
    let player = StableId(3);
    for tick in 1..=8 {
        let origin = Origin {
            peer: PeerId(1),
            connection_epoch: 1,
            sequence: tick,
        };
        let predicted_frame = InputFrame::new(origin, ActorId(1), tick, [0.25, 0.0, 0.0]);
        let straight_frame = InputFrame::new(
            origin,
            ActorId(1),
            tick,
            [if tick == 1 { 1.0 } else { 0.25 }, 0.0, 0.0],
        );
        predicted
            .admit_frame(predicted_frame)
            .expect("predicted input");
        straight
            .admit_frame(straight_frame)
            .expect("corrected input");
        history.step(&mut predicted, &[]).expect("recorded tick");
        straight.step(&[]).expect("straight tick");
    }
    history
        .rewind(&mut predicted, 8)
        .expect("rewind to correction");
    let corrected = InputFrame::new(
        Origin {
            peer: PeerId(1),
            connection_epoch: 1,
            sequence: 1,
        },
        ActorId(1),
        1,
        [1.0, 0.0, 0.0],
    );
    let mut invalid = corrected.clone();
    invalid.actor = ActorId(2);
    assert_eq!(
        history.amend_input(&predicted, invalid),
        Err(SimError::InvalidInput)
    );
    history
        .amend_input(&predicted, corrected)
        .expect("valid correction");
    for _ in 0..8 {
        assert!(
            history
                .replay_next(&mut predicted)
                .expect("retained replay")
        );
    }
    let predicted_row = predicted
        .get(predicted.handle(player).expect("predicted player"))
        .expect("predicted row");
    let straight_row = straight
        .get(straight.handle(player).expect("straight player"))
        .expect("straight row");
    assert_eq!(predicted_row, straight_row);
    assert_eq!(predicted.tick(), 8);
    assert_eq!(
        predicted
            .published_page()
            .expect("corrected page")
            .id()
            .tick,
        8
    );
}

#[test]
fn rejected_retained_correction_keeps_original_replay_record() {
    let mut sim = simulation_with_count(4).expect("game");
    let mut history = sim
        .begin_history(NonZeroUsize::new(4).expect("window"))
        .expect("history");
    let original = InputFrame::new(
        Origin {
            peer: PeerId(1),
            connection_epoch: 1,
            sequence: 1,
        },
        ActorId(1),
        1,
        [0.25, 0.0, 0.0],
    );
    sim.admit_frame(original.clone()).expect("input");
    history.step(&mut sim, &[]).expect("recorded tick");
    let player = sim.handle(StableId(3)).expect("controlled player");
    let expected = sim.get(player).expect("original endpoint");
    history.rewind(&mut sim, 1).expect("rewind");
    let mut invalid = original;
    invalid.axes = [2.0, 0.0, 0.0];
    assert_eq!(
        history.amend_input(&sim, invalid),
        Err(SimError::InvalidInput)
    );
    assert!(
        history
            .replay_next(&mut sim)
            .expect("original retained input")
    );
    assert_eq!(sim.get(player).expect("replayed endpoint"), expected);
}

#[test]
fn replay_publishes_only_the_final_reconstructed_tick() {
    let mut sim = simulation_with_count(32).expect("game");
    let mut history = sim
        .begin_history(NonZeroUsize::new(8).expect("window"))
        .expect("history");
    for _ in 0..4 {
        history.step(&mut sim, &[]).expect("record");
    }
    history.rewind(&mut sim, 4).expect("branch point");
    for tick in 1..=4 {
        assert!(history.replay_next(&mut sim).expect("replay"));
        assert_eq!(sim.has_current_page(), tick == 4);
    }
    assert_eq!(sim.published_page().expect("final page").id().tick, 4);
}

#[test]
fn rewind_patch_restores_changed_peer_health_and_velocity() {
    let mut sim = simulation_with_count(4).expect("four-peer game");
    let player = sim.handle(StableId(3)).expect("controlled row");
    let mut history = sim
        .begin_history(NonZeroUsize::new(4).expect("window"))
        .expect("history");
    let mut frame = InputFrame::new(
        Origin {
            peer: PeerId(1),
            connection_epoch: 1,
            sequence: 1,
        },
        ActorId(1),
        1,
        [1.0, 0.0, 0.0],
    );
    frame.actions.push(SimAction::Damage(7));
    sim.admit_frame(frame).expect("input");
    history.step(&mut sim, &[]).expect("changed tick");
    history.step(&mut sim, &[]).expect("later tick");
    assert_eq!(
        sim.published_page().expect("future").health(player),
        Some(93)
    );
    history.rewind(&mut sim, 2).expect("rewind");
    let patched = sim.published_page().expect("inverse page");
    assert_eq!(patched.health(player), Some(100));
    assert_eq!(patched.velocity(player), Some(Velocity::default()));
    assert!(patched.dirty().health().contains(&player.slot));
    assert!(patched.dirty().velocity().contains(&player.slot));
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
fn ten_thousand_recorded_churn_ticks_reuse_undo_and_page_storage() {
    let mut sim = simulation_with_count(10_000).expect("running fixture");
    let mut history = sim
        .begin_history(NonZeroUsize::new(16).expect("window"))
        .expect("history");
    let mut id = StableId(9_995);
    let mut handle = sim.handle(id).expect("static churn target");
    for iteration in 0..200_u128 {
        let replacement = ComponentEntity {
            id: StableId(10_001 + iteration),
            position: Position::default(),
            velocity: Velocity::default(),
            health: 100,
            ownership: None,
            visual: Some(1),
        };
        if iteration == 100 {
            ALLOCATIONS.with(|count| count.set(0));
            COUNTING.with(|active| active.set(true));
        }
        history
            .step(
                &mut sim,
                &[Command::Despawn(handle), Command::Spawn(replacement)],
            )
            .expect("recorded churn tick");
        id = replacement.id;
        handle = sim.handle(id).expect("replacement handle");
    }
    COUNTING.with(|active| active.set(false));
    assert_eq!(ALLOCATIONS.with(Cell::get), 0);
    assert_eq!(sim.published_page().expect("latest page").len(), 10_000);
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
            page.dirty().velocity().to_vec(),
            page.dirty().health().to_vec(),
            page.dirty().structural().to_vec(),
            page.structural_events().to_vec(),
        )
    };
    let expected = replay(&mut sim);
    sim.restore(&capture).expect("compatible committed capture");
    sim.set_execution_mode(ExecutionMode::RayonForced);
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
    let mut history = sim
        .begin_history(NonZeroUsize::new(4).expect("window"))
        .expect("committed endpoint");
    history
        .step(
            &mut sim,
            &[Command::Despawn(old), Command::Spawn(entity(3, 2.0))],
        )
        .expect("churn");
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
    history.step(&mut sim, &[]).expect("input tick");
    for _ in 0..2 {
        history.step(&mut sim, &[]).expect("moving tick");
    }
    let target = sim.tick();
    assert_eq!(target, 4);
    let at_target = sim.published_page().expect("target endpoint");
    let command = Command::SetVelocity(replacement, Velocity([3.0, 0.0, 0.0]));
    history.step(&mut sim, &[command]).expect("future tick");
    let expected = sim.published_page().expect("future page");
    history.step(&mut sim, &[]).expect("advance window");
    assert_eq!((history.oldest_tick(), history.latest_tick()), (2, 6));
    history
        .rewind(
            &mut sim,
            usize::try_from(history.latest_tick() - target).expect("retained count"),
        )
        .expect("retained endpoint");
    let patched = sim.published_page().expect("inverse page patch");
    assert!(!patched.is_baseline());
    assert_eq!(patched.id().tick, target);
    assert!(patched.patch_source().is_some());
    let mut original_rows = Vec::new();
    at_target.visit(|handle, id, position, visual| {
        original_rows.push((handle, id, position, visual));
    });
    let mut restored_rows = Vec::new();
    patched.visit(|handle, id, position, visual| {
        restored_rows.push((handle, id, position, visual));
    });
    assert_eq!(restored_rows, original_rows);
    assert_eq!(patched.len(), at_target.len());
    assert_eq!(sim.tick(), 4);
    history.step(&mut sim, &[command]).expect("replayed future");
    let actual = sim.published_page().expect("replayed page");
    assert_eq!(actual.position(replacement), expected.position(replacement));
    assert_eq!(actual.velocity(replacement), expected.velocity(replacement));
    assert_eq!(actual.health(player_handle), expected.health(player_handle));
    assert_eq!(actual.structural_events(), expected.structural_events());
    assert_eq!(history.latest_tick(), 5);
}

#[test]
fn rewind_with_every_page_leased_keeps_authority_live_and_defers_view() {
    let mut sim = Simulation::new(WorldEpoch(31), config(1)).expect("game");
    sim.seed(entity(1, 0.0)).expect("entity");
    let mut history = sim
        .begin_history(NonZeroUsize::new(8).expect("window"))
        .expect("committed endpoint");
    let mut held = vec![sim.published_page().expect("tick zero")];
    for _ in 0..3 {
        history.step(&mut sim, &[]).expect("tick");
        held.push(sim.published_page().expect("held page"));
    }
    ALLOCATIONS.with(|count| count.set(0));
    COUNTING.with(|active| active.set(true));
    history
        .rewind(
            &mut sim,
            usize::try_from(history.latest_tick() - 1).expect("retained count"),
        )
        .expect("authority rewind");
    COUNTING.with(|active| active.set(false));
    assert_eq!(ALLOCATIONS.with(Cell::get), 0);
    assert!(!sim.has_current_page());
    assert!(matches!(
        sim.published_page(),
        Err(SimError::PublicationBusy)
    ));
    history.step(&mut sim, &[]).expect("authority advances");
    assert_eq!(sim.tick(), 2);
    drop(held);
    let first = sim.published_page().expect("released page baseline");
    assert_eq!(first.id().tick, 2);
    assert!(first.is_baseline());
}

#[test]
fn rolling_history_has_bounded_reused_storage_at_game_scales() {
    for count in [10_000, 100_000] {
        let mut sim = simulation_with_count(count).expect("game fixture");
        let full_bytes = sim.capture().reserved_bytes();
        let mut history = sim
            .begin_history(NonZeroUsize::new(64).expect("window"))
            .expect("committed endpoint");
        for _ in 0..65 {
            history.step(&mut sim, &[]).expect("warm tick");
        }
        let retained_bytes = history.reserved_bytes();
        assert!(retained_bytes < full_bytes * 6);
        ALLOCATIONS.with(|count| count.set(0));
        COUNTING.with(|active| active.set(true));
        for _ in 0..100 {
            history.step(&mut sim, &[]).expect("steady tick");
        }
        COUNTING.with(|active| active.set(false));
        assert_eq!(ALLOCATIONS.with(Cell::get), 0);
        let target = history.latest_tick() - 8;
        ALLOCATIONS.with(|count| count.set(0));
        COUNTING.with(|active| active.set(true));
        history
            .rewind(
                &mut sim,
                usize::try_from(history.latest_tick() - target).expect("retained count"),
            )
            .expect("eight retained ticks restore");
        COUNTING.with(|active| active.set(false));
        assert_eq!(ALLOCATIONS.with(Cell::get), 0);
        assert_eq!(sim.tick(), target);
    }
}

#[test]
fn rolling_history_refuses_a_divergent_same_tick_future() {
    let mut sim = Simulation::new(WorldEpoch(30), config(2)).expect("game");
    let moving = sim.seed(entity(1, 1.0)).expect("moving entity");
    let mut history = sim
        .begin_history(NonZeroUsize::new(4).expect("window"))
        .expect("committed endpoint");
    history.step(&mut sim, &[]).expect("tick one");
    let saved = sim.capture();
    history
        .step(
            &mut sim,
            &[Command::SetVelocity(moving, Velocity([2.0, 0.0, 0.0]))],
        )
        .expect("first future");
    sim.restore(&saved).expect("branch point");
    sim.step(&[Command::SetVelocity(moving, Velocity([3.0, 0.0, 0.0]))])
        .expect("divergent future");
    assert_eq!(sim.tick(), history.latest_tick());
    assert_eq!(history.step(&mut sim, &[]), Err(SimError::InvalidSnapshot));
    assert_eq!(
        history.rewind(
            &mut sim,
            usize::try_from(history.latest_tick() - 1).expect("retained count")
        ),
        Err(SimError::InvalidSnapshot)
    );
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
fn restore_with_every_old_page_leased_keeps_old_views_and_publishes_a_new_baseline() {
    let mut sim = Simulation::new(WorldEpoch(16), config(2)).expect("game");
    let handle = sim.seed(entity(1, 0.0)).expect("body");
    let old = std::array::from_fn::<_, 4, _>(|index| {
        if index != 0 {
            sim.step(&[Command::SetPosition(
                handle,
                Position([index as f64, 0.0, 0.0]),
            )])
            .expect("next tick");
        }
        sim.published_page().expect("held endpoint")
    });
    let saved = sim.capture();
    sim.step(&[Command::SetPosition(handle, Position([9.0, 0.0, 0.0]))])
        .expect("unpublished future tick");
    sim.restore(&saved).expect("restore with full old ring");
    let current = sim.published_page().expect("new baseline");
    assert!(current.is_baseline());
    assert_eq!(current.id().tick, 3);
    assert_eq!(current.position(handle).expect("restored body").0[0], 3.0);
    for (tick, page) in old.iter().enumerate() {
        assert_eq!(page.id().tick, tick as u64);
        assert_eq!(page.position(handle).expect("old body").0[0], tick as f64);
    }
}

#[test]
fn old_page_reuse_after_journal_horizon_refreshes_membership_without_a_baseline() {
    let mut sim = Simulation::new(WorldEpoch(31), config(2)).expect("game");
    let old = sim.seed(entity(1, 1.0)).expect("mover");
    sim.seed(entity(2, 0.0)).expect("static");
    let held = sim.published_page().expect("tick-zero page");
    for _ in 0..6 {
        sim.step(&[]).expect("advance while old page held");
    }
    sim.step(&[Command::Despawn(old), Command::Spawn(entity(3, 2.0))])
        .expect("change membership beyond held page horizon");
    let replacement = sim.handle(StableId(3)).expect("replacement");
    assert_eq!(replacement.slot, old.slot);
    drop(held);
    for _ in 0..8 {
        sim.step(&[]).expect("reuse old page");
        let page = sim.published_page().expect("published endpoint");
        assert!(!page.is_baseline(), "adjacent endpoints remain incremental");
        assert_eq!(page.stable_id(old.slot), Some(StableId(3)));
        assert_eq!(
            page.position(replacement),
            Some(sim.get(replacement).expect("live").position)
        );
    }
}

#[test]
fn four_tick_page_catch_up_keeps_old_lease_and_current_dirty_flags() {
    let mut sim = Simulation::new(WorldEpoch(32), config(1)).expect("game");
    let old = sim.seed(entity(1, 1.0)).expect("mover");
    drop(sim.published_page().expect("tick-zero page"));
    sim.step(&[]).expect("first tick");
    let held = sim.published_page().expect("immutable tick-one lease");
    let held_position = held.position(old).expect("old body");
    sim.step(&[Command::Despawn(old), Command::Spawn(entity(2, 2.0))])
        .expect("same-slot churn");
    let replacement = sim.handle(StableId(2)).expect("replacement");
    sim.step(&[]).expect("third tick");
    sim.step(&[]).expect("reuse tick-zero page at tick four");
    let current = sim.published_page().expect("caught-up page");
    assert!(!current.is_baseline());
    assert_eq!(current.stable_id(old.slot), Some(StableId(2)));
    assert_eq!(
        current.position(replacement),
        Some(sim.get(replacement).expect("live").position)
    );
    assert_eq!(current.dirty().position(), &[old.slot]);
    assert_eq!(current.dirty().structural(), []);
    assert_eq!(held.stable_id(old.slot), Some(StableId(1)));
    assert_eq!(held.position(old), Some(held_position));
}

#[test]
fn reused_page_recovers_an_older_position_edit_without_republishing_it() {
    let mut sim = Simulation::new(WorldEpoch(33), config(2)).expect("game");
    let stationary = sim.seed(entity(1, 0.0)).expect("stationary body");
    let moving = sim.seed(entity(2, 1.0)).expect("moving body");
    drop(sim.published_page().expect("tick-zero page"));
    sim.step(&[Command::SetPosition(stationary, Position([5.0, 0.0, 0.0]))])
        .expect("older position edit");
    for _ in 0..3 {
        sim.step(&[]).expect("advance to page reuse");
    }
    let page = sim.published_page().expect("reused page");
    assert_eq!(page.position(stationary), Some(Position([5.0, 0.0, 0.0])));
    assert_eq!(
        page.position(moving),
        Some(sim.get(moving).expect("live").position)
    );
    assert_eq!(page.dirty().position(), &[moving.slot]);
}

#[test]
fn admission_rejects_finite_motion_beyond_game_speed_limit_without_consuming_sequence() {
    let mut sim = Simulation::new(WorldEpoch(341), config(1)).expect("game");
    let owner = Ownership {
        peer: PeerId(1),
        connection_epoch: 1,
        actor: ActorId(1),
    };
    let mut player = entity(1, 0.0);
    player.ownership = Some(owner);
    let handle = sim.seed(player).expect("player");
    sim.register_peer(owner, handle).expect("binding");
    let mut frame = InputFrame::new(
        Origin {
            peer: owner.peer,
            connection_epoch: owner.connection_epoch,
            sequence: 1,
        },
        owner.actor,
        1,
        [1.0, 0.0, 0.0],
    );
    frame.actions.push(SimAction::Impulse([600.0, 0.0, 0.0]));
    frame.actions.push(SimAction::Impulse([600.0, 0.0, 0.0]));
    assert_eq!(
        sim.admit_frame(frame.clone()),
        Err(FrameError::InvalidAction)
    );
    assert_eq!(sim.tick(), 0);
    assert_eq!(
        sim.get(handle).expect("unchanged player").velocity,
        Velocity::default()
    );

    frame.actions.truncate(1);
    sim.admit_frame(frame)
        .expect("same sequence remains available");
    sim.step(&[]).expect("admitted motion commits once");
    assert_eq!(
        sim.get(handle).expect("player").velocity,
        Velocity([606.0, 0.0, 0.0])
    );
}

#[test]
fn direct_query_refusal_restores_earlier_rows_and_keeps_the_tick_retriable() {
    let mut sim = Simulation::new(WorldEpoch(342), config(2)).expect("game");
    let first = sim.seed(entity(1, 1.0)).expect("first mover");
    let mut overflowing = entity(2, f64::MAX);
    overflowing.position = Position([f64::MAX, 0.0, 0.0]);
    let second = sim.seed(overflowing).expect("second mover");
    let baseline = sim.published_page().expect("initial page");
    let before = sim.get(first).expect("first row");
    assert_eq!(sim.step(&[]), Err(SimError::NonFinite));
    assert_eq!(sim.tick(), 0);
    assert_eq!(sim.get(first).expect("restored first row"), before);
    assert_eq!(
        sim.get(second).expect("restored second row").position,
        overflowing.position
    );
    assert_eq!(baseline.id().tick, 0);

    sim.step(&[Command::Despawn(second)])
        .expect("retire invalid mover and retry tick");
    assert_eq!(sim.tick(), 1);
    assert_ne!(
        sim.get(first).expect("moving row").position,
        before.position
    );
}

#[test]
fn unchanged_mutable_query_row_does_not_publish_a_position_change() {
    let mut sim = Simulation::new(WorldEpoch(343), config(1)).expect("game");
    let owned = Ownership {
        peer: PeerId(1),
        connection_epoch: 1,
        actor: ActorId(1),
    };
    let mut player = entity(1, 0.0);
    player.ownership = Some(owned);
    let handle = sim.seed(player).expect("player with velocity column");
    sim.register_peer(owned, handle).expect("peer");
    sim.published_page().expect("initial page");
    assert_eq!(sim.step(&[]).expect("no-op tick").changed_positions, 0);
    let page = sim.published_page().expect("committed page");
    assert_eq!(page.dirty().position(), []);
    assert_eq!(page.position(handle), Some(Position::default()));
}

#[test]
fn position_command_overrides_motion_without_a_false_change_or_double_count() {
    let mut diagnostic = Simulation::new(WorldEpoch(345), config(1)).expect("game");
    let diagnostic_handle = diagnostic.seed(entity(1, 6.0)).expect("mover");
    assert_eq!(
        diagnostic
            .step(&[Command::SetPosition(diagnostic_handle, Position::default(),)])
            .expect("overridden motion")
            .changed_positions,
        0
    );
    let mut sim = Simulation::new(WorldEpoch(344), config(1)).expect("game");
    let handle = sim.seed(entity(1, 6.0)).expect("mover");
    sim.published_page().expect("initial page");
    let mut history = sim
        .begin_history(NonZeroUsize::new(4).expect("window"))
        .expect("history");
    let commands = [Command::SetPosition(handle, Position::default())];
    history
        .step(&mut sim, &commands)
        .expect("overridden motion");
    assert_eq!(
        sim.get(handle).expect("live mover").position,
        Position::default()
    );
    assert_eq!(sim.published_page().expect("page").dirty().position(), []);
    history.rewind(&mut sim, 1).expect("rewind no-op position");
    assert_eq!(
        sim.get(handle).expect("restored mover").position,
        Position::default()
    );
    assert!(
        history
            .replay_next(&mut sim)
            .expect("replay retained command")
    );
    assert_eq!(
        sim.get(handle).expect("replayed mover").position,
        Position::default()
    );
}

#[test]
fn one_tick_impulse_reverts_to_held_axes_on_the_following_tick() {
    let mut sim = Simulation::new(WorldEpoch(34), config(1)).expect("game");
    let owned = Ownership {
        peer: PeerId(1),
        connection_epoch: 1,
        actor: ActorId(1),
    };
    let mut player = entity(1, 0.0);
    player.ownership = Some(owned);
    let handle = sim.seed(player).expect("player");
    sim.register_peer(owned, handle).expect("binding");
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
    frame.actions.push(SimAction::Impulse([2.0, 0.0, 0.0]));
    sim.admit_frame(frame).expect("impulse input");
    sim.step(&[]).expect("impulse tick");
    assert_eq!(
        sim.get(handle).expect("player").velocity,
        Velocity([8.0, 0.0, 0.0])
    );
    sim.step(&[]).expect("held axes tick");
    assert_eq!(
        sim.get(handle).expect("player").velocity,
        Velocity([6.0, 0.0, 0.0])
    );
    sim.step(&[]).expect("steady held axes tick");
    assert_eq!(
        sim.get(handle).expect("player").velocity,
        Velocity([6.0, 0.0, 0.0])
    );
}

#[test]
fn held_control_reasserts_after_a_game_velocity_command() {
    let mut sim = Simulation::new(WorldEpoch(35), config(1)).expect("game");
    let owned = Ownership {
        peer: PeerId(1),
        connection_epoch: 1,
        actor: ActorId(1),
    };
    let mut player = entity(1, 0.0);
    player.ownership = Some(owned);
    let handle = sim.seed(player).expect("player");
    sim.register_peer(owned, handle).expect("binding");
    sim.step(&[Command::SetVelocity(handle, Velocity([3.0, 0.0, 0.0]))])
        .expect("game velocity command");
    assert_eq!(
        sim.get(handle).expect("player").velocity,
        Velocity([3.0, 0.0, 0.0])
    );
    sim.step(&[]).expect("held zero axes reasserted");
    assert_eq!(
        sim.get(handle).expect("player").velocity,
        Velocity::default()
    );
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
        page.visit(|handle, id, position, visual| {
            rows.push((
                id,
                handle,
                position.0.map(f64::to_bits),
                page.velocity(handle)
                    .map(|velocity| velocity.0.map(f64::to_bits)),
                page.health(handle),
                page.ownership(handle),
                visual,
            ));
        });
        let positions = page.dirty().position().to_vec();
        assert!(positions.windows(2).all(|slots| slots[0] < slots[1]));
        let velocities = page.dirty().velocity().to_vec();
        assert!(velocities.windows(2).all(|slots| slots[0] < slots[1]));
        let health = page.dirty().health().to_vec();
        assert!(health.windows(2).all(|slots| slots[0] < slots[1]));
        let structural = page.dirty().structural().to_vec();
        assert!(structural.windows(2).all(|slots| slots[0] < slots[1]));
        (
            page.id(),
            origins,
            rows,
            positions,
            velocities,
            health,
            structural,
            page.structural_events().to_vec(),
        )
    };
    let serial = run(&mut sim);
    sim.restore(&capture).expect("tick-zero capture");
    sim.set_execution_mode(ExecutionMode::RayonForced);
    assert_eq!(run(&mut sim), serial);
}

#[test]
fn sparse_history_readmits_four_peer_inputs_once_after_churn() {
    let mut sim = simulation_with_count(10_000).expect("running game");
    let old = sim.handle(StableId(9_995)).expect("churn target");
    let mut history = sim
        .begin_history(NonZeroUsize::new(8).expect("window"))
        .expect("committed endpoint");
    for tick in 1..=3 {
        if tick <= 2 {
            for peer_number in 1..=4_u64 {
                let mut frame = InputFrame::new(
                    Origin {
                        peer: PeerId(peer_number),
                        connection_epoch: 1,
                        sequence: tick,
                    },
                    ActorId(peer_number),
                    tick,
                    [0.25 * tick as f32, 0.0, 0.0],
                );
                frame.actions.push(SimAction::Damage(1));
                sim.admit_frame(frame).expect("input");
            }
        }
        let commands = if tick == 1 {
            vec![Command::Despawn(old), Command::Spawn(entity(20_000, 1.0))]
        } else {
            Vec::new()
        };
        history.step(&mut sim, &commands).expect("committed tick");
    }
    let expected = sim.published_page().expect("future page");
    let mut expected_rows = Vec::new();
    expected.visit(|handle, id, position, _| {
        expected_rows.push((
            handle,
            id,
            position.0.map(f64::to_bits),
            expected.health(handle),
        ));
    });
    history
        .rewind(
            &mut sim,
            usize::try_from(history.latest_tick()).expect("retained count"),
        )
        .expect("rewind branch");
    for _ in 0..3 {
        assert!(history.replay_next(&mut sim).expect("retained tick"));
    }
    assert!(!history.replay_next(&mut sim).expect("drained replay"));
    let actual = sim.published_page().expect("replayed page");
    let mut actual_rows = Vec::new();
    actual.visit(|handle, id, position, _| {
        actual_rows.push((
            handle,
            id,
            position.0.map(f64::to_bits),
            actual.health(handle),
        ));
    });
    assert_eq!(actual_rows, expected_rows);
    assert_eq!(history.latest_tick(), 3);
}

#[test]
fn retained_replay_refuses_fourth_peer_conflict_without_partial_admission() {
    let mut sim = simulation_with_count(10_000).expect("running game");
    let mut history = sim
        .begin_history(NonZeroUsize::new(2).expect("window"))
        .expect("history");
    for peer_number in 1..=4_u64 {
        sim.admit_frame(InputFrame::new(
            Origin {
                peer: PeerId(peer_number),
                connection_epoch: 1,
                sequence: 1,
            },
            ActorId(peer_number),
            1,
            [0.25, 0.0, 0.0],
        ))
        .expect("original input");
    }
    history.step(&mut sim, &[]).expect("original tick");
    history
        .rewind(
            &mut sim,
            usize::try_from(history.latest_tick()).expect("retained count"),
        )
        .expect("branch point");
    sim.admit_frame(InputFrame::new(
        Origin {
            peer: PeerId(4),
            connection_epoch: 1,
            sequence: 1,
        },
        ActorId(4),
        1,
        [0.5, 0.0, 0.0],
    ))
    .expect("conflicting correction");
    assert_eq!(
        history.replay_next(&mut sim),
        Err(SimError::InvalidSnapshot)
    );
    sim.admit_frame(InputFrame::new(
        Origin {
            peer: PeerId(1),
            connection_epoch: 1,
            sequence: 1,
        },
        ActorId(1),
        1,
        [0.25, 0.0, 0.0],
    ))
    .expect("earlier peer remained unadmitted");
}

#[test]
fn history_seals_setup_and_detects_external_peer_registration() {
    let mut sim = Simulation::new(WorldEpoch(33), config(2)).expect("game");
    let binding = Ownership {
        peer: PeerId(1),
        connection_epoch: 1,
        actor: ActorId(1),
    };
    let mut player = entity(1, 0.0);
    player.ownership = Some(binding);
    let handle = sim.seed(player).expect("player");
    let mut history = sim
        .begin_history(NonZeroUsize::new(2).expect("window"))
        .expect("sealed setup");
    assert_eq!(sim.seed(entity(2, 0.0)), Err(SimError::AlreadyStarted));
    sim.register_peer(binding, handle)
        .expect("external peer registration");
    assert_eq!(history.step(&mut sim, &[]), Err(SimError::InvalidSnapshot));
    assert_eq!(sim.tick(), 0);
}

#[test]
fn history_rejects_same_numeric_resource_revision_on_restored_branch() {
    let mut sim = Simulation::new(WorldEpoch(34), config(2)).expect("game");
    let first = Ownership {
        peer: PeerId(1),
        connection_epoch: 1,
        actor: ActorId(1),
    };
    let second = Ownership {
        peer: PeerId(2),
        connection_epoch: 1,
        actor: ActorId(2),
    };
    let mut a = entity(1, 0.0);
    a.ownership = Some(first);
    let a_handle = sim.seed(a).expect("first player");
    let mut b = entity(2, 0.0);
    b.ownership = Some(second);
    let b_handle = sim.seed(b).expect("second player");
    let before_registration = sim.capture();
    sim.register_peer(first, a_handle)
        .expect("first branch peer");
    let mut history = sim
        .begin_history(NonZeroUsize::new(2).expect("window"))
        .expect("history");
    sim.restore(&before_registration).expect("old branch");
    sim.register_peer(second, b_handle)
        .expect("different peer at same revision number");
    assert_eq!(sim.tick(), history.latest_tick());
    assert_eq!(history.step(&mut sim, &[]), Err(SimError::InvalidSnapshot));
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
