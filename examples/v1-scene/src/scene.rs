//! The concrete 10k-entity V1 workload. Engine crates remain game agnostic.

use std::num::NonZeroU32;

use thrum_core::{
    ActorId, Origin, PeerId, StableId, WorldEpoch,
    sim::{
        ComponentEntity, InputFrame, Ownership, Position, SimAction, SimConfig, Simulation,
        Velocity, VisualSnapshotLease,
    },
};
use thrum_render::{
    extract::{ExtractReport, RenderColumns, RenderExtractor, SourceTick},
    synthetic::SyntheticScene,
};

pub(crate) const ENTITY_COUNT: usize = 10_000;
pub(crate) const MOVING_COUNT: usize = 1_500;
pub(crate) const CONTROLLED_ID: StableId = StableId((ENTITY_COUNT - 1) as u128);
const LOCAL_CONTROL: Ownership = Ownership {
    peer: PeerId(1),
    connection_epoch: 1,
    actor: ActorId(1),
};

pub(crate) fn control_frame(axes: [f32; 3], tick: u64, sequence: u64) -> InputFrame<SimAction> {
    InputFrame {
        origin: Origin {
            peer: LOCAL_CONTROL.peer,
            connection_epoch: LOCAL_CONTROL.connection_epoch,
            sequence,
        },
        actor: LOCAL_CONTROL.actor,
        tick,
        axes,
        actions: Vec::new(),
    }
}

pub(crate) fn simulation() -> Result<Simulation, String> {
    let mut sim = Simulation::new(
        WorldEpoch(1),
        SimConfig {
            tick_hz: NonZeroU32::new(60).expect("nonzero fixture tick rate"),
            entity_capacity: ENTITY_COUNT,
            peer_capacity: 4,
            actions_per_frame: 8,
            movement_speed: 6.0,
        },
    )
    .map_err(|error| format!("simulation setup: {error:?}"))?;
    for index in 0..ENTITY_COUNT {
        let x = -0.9 + (index % 100) as f64 * 0.018 + 0.009;
        let y = -0.9 + (index / 100) as f64 * 0.018 + 0.009;
        sim.seed(ComponentEntity {
            id: StableId(index as u128),
            position: Position([x, y, 0.5]),
            velocity: Velocity([if index < MOVING_COUNT { 0.05 } else { 0.0 }, 0.0, 0.0]),
            health: 100,
            ownership: (index == ENTITY_COUNT - 1).then_some(LOCAL_CONTROL),
            visual: Some((index % 2) as u64),
        })
        .map_err(|error| format!("seed entity {index}: {error:?}"))?;
    }
    let controlled = sim
        .handle(CONTROLLED_ID)
        .ok_or("seeded control identity missing")?;
    sim.register_peer(LOCAL_CONTROL, controlled)
        .map_err(|error| format!("register local control: {error:?}"))?;
    Ok(sim)
}

pub(crate) fn extract(
    extractor: &mut RenderExtractor,
    snapshot: &VisualSnapshotLease,
) -> ExtractReport {
    let mut report = extractor.extract(
        SourceTick {
            epoch: snapshot.epoch().0,
            tick: snapshot.tick(),
        },
        [0.0; 3],
        |sink| {
            for ((id, position), visual) in snapshot
                .ids()
                .iter()
                .zip(snapshot.positions())
                .zip(snapshot.visuals())
            {
                sink.push(id.0, position.0, *visual);
            }
        },
    );
    report.dropped_capacity += snapshot.dropped_capacity();
    report
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ApplyReport {
    pub(crate) moving: usize,
    pub(crate) dropped_invalid: usize,
}

pub(crate) fn apply_frame_to_scene(
    scene: &mut SyntheticScene,
    frame: &RenderColumns,
    changed_slots: &mut Vec<u32>,
) -> ApplyReport {
    changed_slots.clear();
    let mut report = ApplyReport::default();
    let material_count = scene.materials.len();
    for row in 0..frame.len() {
        let Some(index) = usize::try_from(frame.ids()[row]).ok() else {
            report.dropped_invalid += 1;
            continue;
        };
        let Some(instance) = scene.instances.get_mut(index) else {
            report.dropped_invalid += 1;
            continue;
        };
        let Some(material) = u32::try_from(frame.visuals()[row])
            .ok()
            .filter(|material| usize::try_from(*material).is_ok_and(|row| row < material_count))
        else {
            report.dropped_invalid += 1;
            continue;
        };
        let current = frame.current()[row];
        let previous = frame.previous()[row];
        if instance.current[3][..3] != current
            || instance.previous[3][..3] != previous
            || instance.ids[1] != material
        {
            instance.current[3][..3].copy_from_slice(&current);
            instance.previous[3][..3].copy_from_slice(&previous);
            instance.ids[1] = material;
            changed_slots.push(u32::try_from(index).expect("10k fixture slot fits u32"));
        }
        report.moving += usize::from((current[0] - previous[0]).abs() > 1e-6);
    }
    changed_slots.sort_unstable();
    report
}
