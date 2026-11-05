//! The concrete 10k-entity V1 workload. Engine crates remain game agnostic.

use std::num::NonZeroU32;

use crate::simulation::{
    ComponentEntity, Ownership, Position, PublishedPage, SimAction, SimConfig, Simulation, Velocity,
};
use thrum_core::{ActorId, Origin, PeerId, StableId, WorldEpoch, sim::InputFrame};
use thrum_render::extract::{ExtractReport, RenderExtractor, SourceTick};

pub(crate) const ENTITY_COUNT: usize = 10_000;
pub(crate) const MOVING_COUNT: usize = 1_500;
pub(crate) const CONTROLLED_ID: StableId = StableId((ENTITY_COUNT - 1) as u128);
pub(crate) const LOCAL_PEER: PeerId = PeerId(1);
pub(crate) const PEER_COUNT: usize = 4;

pub(crate) fn binding(peer: PeerId) -> Ownership {
    Ownership {
        peer,
        connection_epoch: 1,
        actor: ActorId(peer.0),
    }
}

pub(crate) fn control_frame(
    peer: PeerId,
    axes: [f32; 3],
    tick: u64,
    sequence: u64,
) -> InputFrame<SimAction> {
    let owner = binding(peer);
    InputFrame::new(
        Origin {
            peer: owner.peer,
            connection_epoch: owner.connection_epoch,
            sequence,
        },
        owner.actor,
        tick,
        axes,
    )
}

pub(crate) fn simulation() -> Result<Simulation, String> {
    simulation_with_count(ENTITY_COUNT)
}

/// Build a parameterized V1 workload for the game-tick benchmark.
///
/// # Errors
///
/// Returns fixture setup or seeding failure.
pub fn simulation_with_count(entity_count: usize) -> Result<Simulation, String> {
    if entity_count < PEER_COUNT || entity_count > u32::MAX as usize {
        return Err("fixture entity count is outside the physical slot range".into());
    }
    let mut sim = Simulation::new(
        WorldEpoch(1),
        SimConfig {
            tick_hz: NonZeroU32::new(60).ok_or("invalid fixture tick rate")?,
            entity_capacity: entity_count,
            peer_capacity: PEER_COUNT,
            actions_per_frame: 8,
            movement_speed: 6.0,
        },
    )
    .map_err(|error| format!("simulation setup: {error:?}"))?;
    let moving_count = entity_count / ENTITY_COUNT * MOVING_COUNT
        + entity_count % ENTITY_COUNT * MOVING_COUNT / ENTITY_COUNT;
    for index in 0..entity_count {
        let x = -0.9 + (index % 100) as f64 * 0.018 + 0.009;
        let y = -0.9 + (index / 100) as f64 * 0.018 + 0.009;
        let peer_number = entity_count - index;
        let ownership = if peer_number <= PEER_COUNT {
            Some(binding(PeerId(
                u64::try_from(peer_number).map_err(|error| error.to_string())?,
            )))
        } else {
            None
        };
        sim.seed(ComponentEntity {
            id: StableId(index as u128),
            position: Position([x, y, 0.5]),
            velocity: Velocity([if index < moving_count { 0.05 } else { 0.0 }, 0.0, 0.0]),
            health: 100,
            ownership,
            visual: Some((index % 2) as u64),
        })
        .map_err(|error| format!("seed entity {index}: {error:?}"))?;
    }
    for peer_number in 1..=PEER_COUNT {
        let peer = PeerId(u64::try_from(peer_number).map_err(|error| error.to_string())?);
        let id = StableId(
            u128::try_from(entity_count - peer_number).map_err(|error| error.to_string())?,
        );
        let controlled = sim.handle(id).ok_or("seeded control identity missing")?;
        sim.register_peer(binding(peer), controlled)
            .map_err(|error| format!("register control peer {peer_number}: {error:?}"))?;
    }
    Ok(sim)
}

pub(crate) fn extract(extractor: &mut RenderExtractor, snapshot: &PublishedPage) -> ExtractReport {
    extractor.extract(
        SourceTick {
            epoch: snapshot.id().epoch.0,
            tick: snapshot.id().tick,
            timeline: snapshot.timeline(),
        },
        [0.0; 3],
        |sink| {
            snapshot.visit(|handle, id, position, visual| {
                if let Some(visual) = visual {
                    sink.push(handle.slot, handle.generation, id.0, position.0, visual);
                }
            });
        },
    )
}

#[cfg(test)]
mod tests {
    use thrum_core::{PeerId, StableId, sim::FrameError};
    use thrum_render::{
        extract::{RenderExtractor, SourceTick},
        synthetic::SyntheticScene,
    };

    use super::{
        ENTITY_COUNT, PEER_COUNT, control_frame, extract, simulation, simulation_with_count,
    };
    use crate::simulation::{Command, Position};

    #[test]
    fn skipped_replay_baseline_invalidates_render_pose_history() {
        let mut sim = simulation_with_count(4).expect("small game");
        let mut extractor = RenderExtractor::new(4, 4);
        sim.step(&[]).expect("tick one");
        let saved = sim.capture();
        sim.step(&[]).expect("old tick two");
        extract(&mut extractor, &sim.published_page().expect("old page"));
        sim.restore(&saved).expect("restore tick one");
        let moved = sim.handle(StableId(0)).expect("entity");
        sim.step(&[Command::SetPosition(moved, Position([1.0, 0.0, 0.5]))])
            .expect("alternate tick two");
        sim.step(&[]).expect("alternate tick three");
        let report = extract(
            &mut extractor,
            &sim.published_page().expect("replayed page"),
        );
        assert_eq!(report.history_reused, 0);
        assert!(!extractor.frame().history_valid().iter().any(|valid| *valid));
    }

    #[test]
    fn physical_slot_is_used_instead_of_stable_identity_as_gpu_slot() {
        let ids = [StableId(u128::from(u32::MAX) + 8), StableId(7)];
        let mut scene = SyntheticScene::grid(2).expect("two synthetic instances");
        let mut extractor = RenderExtractor::new(2, 2);
        extractor.extract(
            SourceTick {
                epoch: 1,
                tick: 1,
                timeline: 0,
            },
            [0.0; 3],
            |sink| {
                assert!(sink.push(1, 1, 7, [0.25, 0.0, 0.0], 0));
                assert!(sink.push(0, 1, ids[0].0, [0.75, 0.0, 0.0], 1));
            },
        );
        let mut changed = Vec::new();
        let report = scene.apply_render_columns(extractor.frame(), &mut changed);
        assert_eq!(report.dropped_invalid, 0);
        assert_eq!(changed, [0, 1]);
        assert_eq!(scene.instances[0].current[3][0], 0.75);
        assert_eq!(scene.instances[1].current[3][0], 0.25);
        extractor.extract(
            SourceTick {
                epoch: 1,
                tick: 2,
                timeline: 0,
            },
            [0.0; 3],
            |sink| {
                assert!(sink.push(0, 1, ids[0].0, [0.8, 0.0, 0.0], 1));
            },
        );
        // Worker result for tick two is discarded before reaching the scene.
        extractor.extract(
            SourceTick {
                epoch: 1,
                tick: 3,
                timeline: 0,
            },
            [0.0; 3],
            |sink| {
                assert!(sink.push(0, 1, ids[0].0, [0.9, 0.0, 0.0], 1));
            },
        );
        scene.apply_render_columns(extractor.frame(), &mut changed);
        assert_eq!(scene.instances[1].ids[3], 0, "retired slot is hidden");
        extractor.extract(
            SourceTick {
                epoch: 1,
                tick: 4,
                timeline: 0,
            },
            [0.0; 3],
            |sink| {
                assert!(sink.push(1, 2, 11, [0.3, 0.0, 0.0], 0));
            },
        );
        scene.apply_render_columns(extractor.frame(), &mut changed);
        assert_eq!(
            scene.instances[1].ids[3], 2,
            "reused slot has a new GPU history token"
        );
    }

    #[test]
    fn reused_slot_retags_gpu_history_even_when_pose_is_unchanged() {
        let mut scene = SyntheticScene::grid(1).expect("one GPU slot");
        let mut extractor = RenderExtractor::new(1, 1);
        let mut changed = Vec::new();
        extractor.extract(
            SourceTick {
                epoch: 1,
                tick: 1,
                timeline: 0,
            },
            [0.0; 3],
            |sink| assert!(sink.push(0, u64::from(u32::MAX), 7, [0.0; 3], 0)),
        );
        scene.apply_render_columns(extractor.frame(), &mut changed);
        assert_eq!(scene.instances[0].ids[3], u32::MAX);
        extractor.extract(
            SourceTick {
                epoch: 1,
                tick: 2,
                timeline: 0,
            },
            [0.0; 3],
            |sink| assert!(sink.push(0, u64::from(u32::MAX) + 1, 8, [0.0; 3], 0)),
        );
        assert!(!extractor.frame().history_valid()[0]);
        scene.apply_render_columns(extractor.frame(), &mut changed);
        assert_eq!(changed, [0]);
        assert_eq!(scene.instances[0].ids[3], 1);
        extractor.extract(
            SourceTick {
                epoch: 1,
                tick: 3,
                timeline: 0,
            },
            [0.0; 3],
            |sink| assert!(sink.push(0, u64::from(u32::MAX) + 2, 9, [0.0; 3], 0)),
        );
        scene.apply_render_columns(extractor.frame(), &mut changed);
        assert_eq!(changed, [0], "next wrapped generation must retag the slot");
        assert_eq!(scene.instances[0].ids[3], 2);
    }

    #[test]
    fn repeated_display_samples_keep_simulation_endpoints_without_extra_upload() {
        let mut scene = SyntheticScene::grid(2).expect("scene");
        let mut extractor = RenderExtractor::new(2, 2);
        let mut changed = Vec::new();
        extractor.extract(
            SourceTick {
                epoch: 1,
                tick: 4,
                timeline: 0,
            },
            [0.0; 3],
            |sink| assert!(sink.push(0, 1, 1, [0.0, 0.0, 0.0], 0)),
        );
        extractor.extract(
            SourceTick {
                epoch: 1,
                tick: 5,
                timeline: 0,
            },
            [0.0; 3],
            |sink| assert!(sink.push(0, 1, 1, [1.0, 0.0, 0.0], 0)),
        );
        scene.apply_render_columns(extractor.frame(), &mut changed);
        assert_eq!(scene.instances[0].current[3][0], 1.0);
        assert_eq!(scene.instances[0].previous[3][0], 0.0);
        assert_eq!(changed, [0, 1]);
        extractor.extract(
            SourceTick {
                epoch: 1,
                tick: 5,
                timeline: 0,
            },
            [0.0; 3],
            |sink| assert!(sink.push(0, 1, 1, [1.0, 0.0, 0.0], 0)),
        );
        scene.apply_render_columns(extractor.frame(), &mut changed);
        assert_eq!(scene.instances[0].current[3][0], 1.0);
        assert_eq!(changed, []);
    }

    #[test]
    fn four_peers_share_one_tick_authority_and_keep_independent_held_controls() {
        let mut sim = simulation().expect("10k fixture");
        let axes = [
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, -1.0, 0.0],
        ];
        let initial = std::array::from_fn::<_, PEER_COUNT, _>(|index| {
            let number = index + 1;
            let id = StableId(u128::try_from(ENTITY_COUNT - number).expect("fixture ID"));
            let handle = sim.handle(id).expect("registered actor");
            sim.get(handle).expect("owned entity").position.0
        });
        assert_eq!(
            sim.admit_frame(control_frame(PeerId(4), [2.0, 0.0, 0.0], 1, 1)),
            Err(FrameError::InvalidAxis)
        );
        for (index, axis) in axes.into_iter().enumerate() {
            sim.admit_frame(control_frame(
                PeerId(u64::try_from(index + 1).expect("four peers")),
                axis,
                1,
                1,
            ))
            .expect("independent accepted frame");
        }
        assert_eq!(
            sim.admit_frame(control_frame(PeerId(1), [0.0; 3], 1, 2)),
            Err(FrameError::PendingFrame)
        );
        assert_eq!(sim.step(&[]).expect("one committed tick").tick, 1);
        for (index, axis) in axes.into_iter().enumerate() {
            let id = StableId(u128::try_from(ENTITY_COUNT - index - 1).expect("fixture ID"));
            let handle = sim.handle(id).expect("same actor");
            let current = sim.get(handle).expect("committed entity").position.0;
            for dimension in 0..3 {
                assert!(
                    (current[dimension]
                        - initial[index][dimension]
                        - f64::from(axis[dimension]) * 0.1)
                        .abs()
                        < 1e-9
                );
            }
        }
        assert_eq!(
            sim.admit_frame(control_frame(PeerId(1), [0.0; 3], 2, 1)),
            Err(FrameError::StaleSequence)
        );
        sim.admit_frame(control_frame(PeerId(1), [0.0; 3], 2, 2))
            .expect("release first actor");
        sim.step(&[]).expect("held second tick");
        let first = sim
            .get(
                sim.handle(StableId((ENTITY_COUNT - 1) as u128))
                    .expect("first actor"),
            )
            .expect("first state")
            .position
            .0;
        assert!((first[0] - initial[0][0] - 0.1).abs() < 1e-9);
        let second = sim
            .get(
                sim.handle(StableId((ENTITY_COUNT - 2) as u128))
                    .expect("second actor"),
            )
            .expect("second state")
            .position
            .0;
        assert!((second[1] - initial[1][1] - 0.2).abs() < 1e-9);
    }
}
