//! The concrete 10k-entity V1 workload. Engine crates remain game agnostic.

use std::num::NonZeroU32;

use crate::simulation::{
    ComponentEntity, Ownership, Position, PublishedPage, SimAction, SimConfig, Simulation, Velocity,
};
use thrum_core::{ActorId, Origin, PeerId, StableId, WorldEpoch, sim::InputFrame};
use thrum_render::extract::{ExtractReport, RenderExtractor, SourceTick};

pub(crate) const ENTITY_COUNT: usize = 10_000;
pub(crate) const MOVING_COUNT: usize = 1_500;
pub(crate) const NEUTRAL_SUBJECT_SLOT: usize = 5050;
pub(crate) const REFLECTION_SUBJECT_SLOT: usize = 5052;
pub(crate) const REFLECTION_SUBJECT_POSITION: [f32; 3] = [-0.5, 0.009, 0.78];
pub(crate) const REFLECTION_SUBJECT_MATERIAL: u64 = 2;
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

/// The content a game host is prepared with.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GameContent {
    /// The original 10k diagnostic grid with its subjects and local lights.
    #[default]
    Grid,
    /// The dense flythrough field (about 100k instances, walls and a ground).
    Field,
}

impl GameContent {
    /// Physical entity slots the content seeds and the renderer prepares.
    #[must_use]
    pub fn entity_count(self) -> usize {
        match self {
            Self::Grid => ENTITY_COUNT,
            Self::Field => crate::field::ENTITY_COUNT,
        }
    }

    /// The local peer's controlled entity: the content's last slot.
    pub(crate) fn controlled_id(self) -> StableId {
        StableId((self.entity_count() - 1) as u128)
    }
}

pub(crate) fn simulation(content: GameContent) -> Result<Simulation, String> {
    match content {
        GameContent::Grid => simulation_with_count(ENTITY_COUNT),
        GameContent::Field => field_simulation(),
    }
}

fn new_simulation(entity_count: usize) -> Result<Simulation, String> {
    Simulation::new(
        WorldEpoch(1),
        SimConfig {
            tick_hz: NonZeroU32::new(60).ok_or("invalid fixture tick rate")?,
            entity_capacity: entity_count,
            peer_capacity: PEER_COUNT,
            actions_per_frame: 8,
            movement_speed: 6.0,
        },
    )
    .map_err(|error| format!("simulation setup: {error:?}"))
}

/// The last `PEER_COUNT` slots are owned by, and registered to, the peers.
fn peer_ownership(entity_count: usize, index: usize) -> Result<Option<Ownership>, String> {
    let peer_number = entity_count - index;
    if peer_number > PEER_COUNT {
        return Ok(None);
    }
    Ok(Some(binding(PeerId(
        u64::try_from(peer_number).map_err(|error| error.to_string())?,
    ))))
}

fn register_peers(sim: &mut Simulation, entity_count: usize) -> Result<(), String> {
    for peer_number in 1..=PEER_COUNT {
        let peer = PeerId(u64::try_from(peer_number).map_err(|error| error.to_string())?);
        let id = StableId(
            u128::try_from(entity_count - peer_number).map_err(|error| error.to_string())?,
        );
        let controlled = sim.handle(id).ok_or("seeded control identity missing")?;
        sim.register_peer(binding(peer), controlled)
            .map_err(|error| format!("register control peer {peer_number}: {error:?}"))?;
    }
    Ok(())
}

fn field_simulation() -> Result<Simulation, String> {
    let entity_count = crate::field::ENTITY_COUNT;
    let mut sim = new_simulation(entity_count)?;
    for index in 0..entity_count {
        let entity = crate::field::entity(index);
        sim.seed(ComponentEntity {
            id: StableId(index as u128),
            position: Position(entity.position),
            velocity: Velocity(entity.velocity),
            health: 100,
            ownership: peer_ownership(entity_count, index)?,
            visual: Some(entity.material),
        })
        .map_err(|error| format!("seed field entity {index}: {error:?}"))?;
    }
    register_peers(&mut sim, entity_count)?;
    Ok(sim)
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
    let mut sim = new_simulation(entity_count)?;
    let moving_count = entity_count / ENTITY_COUNT * MOVING_COUNT
        + entity_count % ENTITY_COUNT * MOVING_COUNT / ENTITY_COUNT;
    for index in 0..entity_count {
        let x = -0.9 + (index % 100) as f64 * 0.018 + 0.009;
        let y = -0.9 + (index / 100) as f64 * 0.018 + 0.009;
        let reflection_subject = entity_count == ENTITY_COUNT && index == REFLECTION_SUBJECT_SLOT;
        let ownership = peer_ownership(entity_count, index)?;
        sim.seed(ComponentEntity {
            id: StableId(index as u128),
            position: Position(if reflection_subject {
                REFLECTION_SUBJECT_POSITION.map(f64::from)
            } else {
                [x, y, 0.5]
            }),
            velocity: Velocity([if index < moving_count { 0.05 } else { 0.0 }, 0.0, 0.0]),
            health: 100,
            ownership,
            visual: Some(if reflection_subject {
                REFLECTION_SUBJECT_MATERIAL
            } else {
                (index % 2) as u64
            }),
        })
        .map_err(|error| format!("seed entity {index}: {error:?}"))?;
    }
    register_peers(&mut sim, entity_count)?;
    Ok(sim)
}

pub(crate) fn extract(extractor: &mut RenderExtractor, snapshot: &PublishedPage) -> ExtractReport {
    let source = SourceTick {
        epoch: snapshot.id().epoch.0,
        tick: snapshot.id().tick,
        timeline: snapshot.timeline(),
    };
    let source_endpoint = snapshot.source_endpoint().map(|(id, timeline)| SourceTick {
        epoch: id.epoch.0,
        tick: id.tick,
        timeline,
    });
    extractor.extract_adaptive(
        source,
        source_endpoint,
        [0.0; 3],
        |sink| {
            snapshot.visit(|handle, id, position, visual| {
                if let Some(visual) = visual {
                    sink.push(handle.slot, handle.generation, id.0, position.0, visual);
                }
            });
        },
        |sink| {
            let dirty = snapshot.dirty();
            let lists = [dirty.position(), dirty.visual(), dirty.structural()];
            let mut next_index = [0; 3];
            while let Some(slot) = (0..lists.len())
                .filter_map(|index| lists[index].get(next_index[index]).copied())
                .min()
            {
                for (index, list) in lists.iter().enumerate() {
                    while list.get(next_index[index]) == Some(&slot) {
                        next_index[index] += 1;
                    }
                }
                if let Some(handle) = snapshot.handle(slot)
                    && let (Some(id), Some(position), Some(visual)) = (
                        snapshot.stable_id(slot),
                        snapshot.position(handle),
                        snapshot.visual(handle),
                    )
                {
                    sink.set(slot, handle.generation, id.0, position.0, visual);
                } else {
                    sink.remove(slot);
                }
            }
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
    use crate::simulation::{Command, ComponentEntity, Position, Velocity};

    #[test]
    fn published_deltas_match_full_visual_extraction_through_churn_and_skips() {
        let mut sim = simulation_with_count(100).expect("small visual game");
        let mut sparse = RenderExtractor::new(100, 100);
        let mut full = RenderExtractor::new(100, 100);
        let mut churn_id = StableId(1);
        for step in 0..20 {
            let commands = if step % 4 == 3 {
                let old = sim.handle(churn_id).expect("churn target");
                churn_id = StableId(100 + step);
                vec![
                    Command::Despawn(old),
                    Command::Spawn(ComponentEntity {
                        id: churn_id,
                        position: Position([0.25, 0.0, 0.5]),
                        velocity: Velocity::default(),
                        health: 100,
                        ownership: None,
                        visual: Some(1),
                    }),
                ]
            } else if step == 5 {
                vec![Command::SetVelocity(
                    sim.handle(StableId(0)).expect("mover"),
                    Velocity::default(),
                )]
            } else if step == 12 {
                vec![Command::SetPosition(
                    sim.handle(StableId(2)).expect("static"),
                    Position([0.6, 0.1, 0.5]),
                )]
            } else {
                Vec::new()
            };
            sim.step(&commands).expect("committed tick");
            if step == 10 {
                continue;
            }
            let page = sim.published_page().expect("published endpoint");
            let sparse_report = extract(&mut sparse, &page);
            let source = SourceTick {
                epoch: page.id().epoch.0,
                tick: page.id().tick,
                timeline: page.timeline(),
            };
            let full_report = full.extract(source, [0.0; 3], |sink| {
                page.visit(|handle, id, position, visual| {
                    if let Some(visual) = visual {
                        sink.push(handle.slot, handle.generation, id.0, position.0, visual);
                    }
                });
            });
            assert_eq!(sparse_report, full_report, "tick {}", page.id().tick);
            let actual = sparse.frame();
            let expected = full.frame();
            assert_eq!(actual.len(), expected.len());
            for slot in 0..100_u32 {
                let actual_row = actual.slots().iter().position(|value| *value == slot);
                let expected_row = expected.slots().iter().position(|value| *value == slot);
                assert_eq!(actual_row.is_some(), expected_row.is_some(), "slot {slot}");
                if let (Some(actual_row), Some(expected_row)) = (actual_row, expected_row) {
                    assert_eq!(actual.ids()[actual_row], expected.ids()[expected_row]);
                    assert_eq!(
                        actual.current()[actual_row],
                        expected.current()[expected_row]
                    );
                    assert_eq!(
                        actual.previous()[actual_row],
                        expected.previous()[expected_row]
                    );
                    assert_eq!(
                        actual.history_valid()[actual_row],
                        expected.history_valid()[expected_row]
                    );
                    assert_eq!(
                        actual.visuals()[actual_row],
                        expected.visuals()[expected_row]
                    );
                    assert_eq!(
                        actual.generation_at_slot(slot as usize),
                        expected.generation_at_slot(slot as usize)
                    );
                }
            }
        }
    }

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
        assert_eq!(changed, [0_u32; 0]);
    }

    #[test]
    fn four_peers_share_one_tick_authority_and_keep_independent_held_controls() {
        let mut sim = simulation(super::GameContent::Grid).expect("10k fixture");
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
