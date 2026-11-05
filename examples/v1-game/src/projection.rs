//! Game-owned, detached state projection. G6 will encode and route it over a
//! real session; no transport state or second world authority lives here.

use std::{collections::VecDeque, mem::size_of};

use crate::simulation::{ComponentEntity, Position, PublishedPage, Velocity};
use thrum_core::{
    Handle,
    sim::{SnapshotId, StructuralChange},
};

/// A copy of only the game values changed in one committed world tick.
/// It owns no page lease, so a delayed peer cannot pin the simulation ring.
pub struct GameProjection {
    id: Option<SnapshotId>,
    timeline: u64,
    source: Option<(SnapshotId, u64)>,
    baseline: bool,
    positions: Vec<(Handle, Position)>,
    velocities: Vec<(Handle, Velocity)>,
    health: Vec<(Handle, u16)>,
    spawns: Vec<(Handle, ComponentEntity)>,
    despawns: Vec<Handle>,
}

impl GameProjection {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            id: None,
            timeline: 0,
            source: None,
            baseline: false,
            positions: Vec::with_capacity(capacity),
            velocities: Vec::with_capacity(capacity),
            health: Vec::with_capacity(capacity),
            spawns: Vec::with_capacity(capacity),
            despawns: Vec::with_capacity(capacity),
        }
    }

    /// Committed source endpoint.
    pub fn id(&self) -> Option<SnapshotId> {
        self.id
    }

    /// Number of projected component and structural records.
    pub fn changed_count(&self) -> usize {
        self.positions.len()
            + self.velocities.len()
            + self.health.len()
            + self.spawns.len()
            + self.despawns.len()
    }

    /// A complete membership baseline, used for initial state or gap recovery.
    pub fn is_baseline(&self) -> bool {
        self.baseline
    }
    /// Changed positions, named by checked world handles.
    pub fn positions(&self) -> &[(Handle, Position)] {
        &self.positions
    }
    /// Changed velocities, named by checked world handles.
    pub fn velocities(&self) -> &[(Handle, Velocity)] {
        &self.velocities
    }
    /// Changed health values, named by checked world handles.
    pub fn health(&self) -> &[(Handle, u16)] {
        &self.health
    }
    /// Full records for newly live entities.
    pub fn spawns(&self) -> &[(Handle, ComponentEntity)] {
        &self.spawns
    }
    /// Retired handles in this tick.
    pub fn despawns(&self) -> &[Handle] {
        &self.despawns
    }

    pub(crate) fn capture(&mut self, page: &PublishedPage) {
        self.baseline = page.is_baseline();
        self.positions.clear();
        self.velocities.clear();
        self.health.clear();
        self.spawns.clear();
        self.despawns.clear();
        for &slot in page.dirty().position() {
            if let Some(handle) = page.handle(slot) {
                self.positions
                    .push((handle, page.position(handle).expect("live position")));
            }
        }
        for &slot in page.dirty().velocity() {
            if let Some(handle) = page.handle(slot) {
                self.velocities
                    .push((handle, page.velocity(handle).expect("live velocity")));
            }
        }
        for &slot in page.dirty().health() {
            if let Some(handle) = page.handle(slot) {
                self.health
                    .push((handle, page.health(handle).expect("live health")));
            }
        }
        for &event in page.structural_events() {
            match event {
                StructuralChange::Despawn(handle) => self.despawns.push(handle),
                StructuralChange::Spawn(handle) => self.spawns.push((
                    handle,
                    ComponentEntity {
                        id: page.stable_id(handle.slot).expect("spawn identity"),
                        position: page.position(handle).expect("spawn position"),
                        velocity: page.velocity(handle).expect("spawn velocity"),
                        health: page.health(handle).expect("spawn health"),
                        ownership: page.ownership(handle),
                        visual: page.visual(handle),
                    },
                )),
            }
        }
        self.id = Some(page.id());
        self.timeline = page.timeline();
        self.source = page.source_endpoint();
    }

    fn capture_full(&mut self, page: &PublishedPage) {
        self.positions.clear();
        self.velocities.clear();
        self.health.clear();
        self.spawns.clear();
        self.despawns.clear();
        page.visit(|handle, id, position, visual| {
            self.spawns.push((
                handle,
                ComponentEntity {
                    id,
                    position,
                    velocity: page.velocity(handle).expect("live velocity"),
                    health: page.health(handle).expect("live health"),
                    ownership: page.ownership(handle),
                    visual,
                },
            ));
        });
        self.id = Some(page.id());
        self.timeline = page.timeline();
        self.source = None;
        self.baseline = true;
    }
}

/// A consumer missed one or more complete tick projections and must request a baseline.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectionGap {
    /// Number of tick projections overwritten since the last baseline.
    pub lost_ticks: u64,
}

/// Four preallocated detached projection slots. A slow consumer cannot pin a
/// world page; overflow is explicit and requires an in-order baseline reset.
pub(crate) struct ProjectionOutbox {
    slots: Vec<GameProjection>,
    free: Vec<usize>,
    pending: VecDeque<usize>,
    delivered: Option<(SnapshotId, u64)>,
    lost_ticks: u64,
    needs_resync: bool,
}

impl ProjectionOutbox {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            slots: (0..4)
                .map(|index| GameProjection::new(if index == 0 { capacity } else { 0 }))
                .collect(),
            free: (0..4).rev().collect(),
            pending: VecDeque::with_capacity(4),
            delivered: None,
            lost_ticks: 0,
            needs_resync: false,
        }
    }

    pub(crate) fn capture(&mut self, page: &PublishedPage) -> usize {
        if page.is_baseline() {
            self.reset_full(page);
            return page.len();
        }
        if self.needs_resync {
            self.lost_ticks = self.lost_ticks.saturating_add(1);
            return 0;
        }
        let expected = self
            .pending
            .back()
            .and_then(|&index| {
                self.slots[index]
                    .id
                    .map(|id| (id, self.slots[index].timeline))
            })
            .or(self.delivered);
        if page.source_endpoint() != expected {
            self.needs_resync = true;
            self.lost_ticks = self.lost_ticks.saturating_add(1);
            return 0;
        }
        let index = self.free.pop().unwrap_or_else(|| {
            self.needs_resync = true;
            self.lost_ticks = self.lost_ticks.saturating_add(1);
            self.pending
                .pop_front()
                .expect("full outbox has an oldest slot")
        });
        self.slots[index].capture(page);
        let changed = self.slots[index].changed_count();
        self.pending.push_back(index);
        changed
    }

    pub(crate) fn poll(
        &mut self,
        timeline: u64,
        consume: impl FnOnce(&GameProjection),
    ) -> Result<bool, ProjectionGap> {
        if !self.needs_resync
            && self
                .pending
                .back()
                .is_some_and(|&index| self.slots[index].timeline != timeline)
        {
            self.lost_ticks = self.lost_ticks.saturating_add(self.pending.len() as u64);
            self.needs_resync = true;
        }
        if !self.needs_resync
            && self.pending.front().is_some_and(|&index| {
                !self.slots[index].baseline && self.slots[index].source != self.delivered
            })
        {
            self.lost_ticks = self.lost_ticks.saturating_add(self.pending.len() as u64);
            self.needs_resync = true;
        }
        if self.needs_resync {
            return Err(ProjectionGap {
                lost_ticks: self.lost_ticks,
            });
        }
        let Some(index) = self.pending.pop_front() else {
            return Ok(false);
        };
        consume(&self.slots[index]);
        self.delivered = self.slots[index]
            .id
            .map(|id| (id, self.slots[index].timeline));
        self.free.push(index);
        Ok(true)
    }

    pub(crate) fn reset_full(&mut self, page: &PublishedPage) {
        while let Some(index) = self.pending.pop_front() {
            self.free.push(index);
        }
        let index = self.free.pop().expect("fixed outbox has one free slot");
        self.slots[index].capture_full(page);
        self.pending.push_back(index);
        self.delivered = None;
        self.needs_resync = false;
        self.lost_ticks = 0;
    }

    pub(crate) fn lost_ticks(&self) -> u64 {
        self.lost_ticks
    }

    pub(crate) fn reserved_bytes(&self) -> usize {
        self.slots.capacity() * size_of::<GameProjection>()
            + self.free.capacity() * size_of::<usize>()
            + self.pending.capacity() * size_of::<usize>()
            + self
                .slots
                .iter()
                .map(|slot| {
                    slot.positions.capacity() * size_of::<(Handle, Position)>()
                        + slot.velocities.capacity() * size_of::<(Handle, Velocity)>()
                        + slot.health.capacity() * size_of::<(Handle, u16)>()
                        + slot.spawns.capacity() * size_of::<(Handle, ComponentEntity)>()
                        + slot.despawns.capacity() * size_of::<Handle>()
                })
                .sum::<usize>()
    }
}

#[cfg(test)]
mod tests {
    use super::{GameProjection, ProjectionOutbox};
    use crate::simulation::{Command, ComponentEntity, Position, SimConfig, Simulation, Velocity};
    use std::num::{NonZeroU32, NonZeroUsize};
    use thrum_core::{StableId, WorldEpoch};

    #[test]
    fn projection_copies_changes_from_exact_page_and_releases_lease() {
        let mut sim = Simulation::new(
            WorldEpoch(3),
            SimConfig {
                tick_hz: NonZeroU32::new(60).expect("nonzero"),
                entity_capacity: 2,
                peer_capacity: 0,
                actions_per_frame: 0,
                movement_speed: 0.0,
            },
        )
        .expect("world");
        let old = sim
            .seed(ComponentEntity {
                id: StableId(91),
                position: Position::default(),
                velocity: Velocity::default(),
                health: 7,
                ownership: None,
                visual: Some(1),
            })
            .expect("old");
        let mut projection = GameProjection::new(2);
        projection.capture(&sim.published_page().expect("initial"));
        assert_eq!(projection.spawns.len(), 1, "baseline includes membership");
        assert_eq!(projection.spawns[0].1.id, StableId(91));
        sim.step(&[
            Command::Despawn(old),
            Command::Spawn(ComponentEntity {
                id: StableId(1),
                position: Position([2.0, 0.0, 0.0]),
                velocity: Velocity::default(),
                health: 9,
                ownership: None,
                visual: Some(2),
            }),
        ])
        .expect("replacement");
        let page = sim.published_page().expect("replacement page");
        projection.capture(&page);
        assert_eq!(projection.id(), Some(page.id()));
        assert_eq!(projection.despawns, [old]);
        assert_eq!(projection.spawns.len(), 1);
        assert_eq!(projection.spawns[0].0.slot, old.slot);
        assert_ne!(projection.spawns[0].0.generation, old.generation);
        assert_eq!(projection.spawns[0].1.id, StableId(1));
        assert_eq!(projection.spawns[0].1.position.0, [2.0, 0.0, 0.0]);
        drop(page);
        sim.step(&[]).expect("projection did not hold page");
    }

    #[test]
    fn catch_up_retains_each_projection_and_overflow_demands_baseline() {
        let mut sim = Simulation::new(
            WorldEpoch(4),
            SimConfig {
                tick_hz: NonZeroU32::new(60).expect("nonzero"),
                entity_capacity: 2,
                peer_capacity: 0,
                actions_per_frame: 0,
                movement_speed: 0.0,
            },
        )
        .expect("world");
        let old = sim
            .seed(ComponentEntity {
                id: StableId(8),
                position: Position::default(),
                velocity: Velocity::default(),
                health: 7,
                ownership: None,
                visual: Some(1),
            })
            .expect("old");
        let mut outbox = ProjectionOutbox::new(2);
        outbox.capture(&sim.published_page().expect("initial"));
        for tick in 1..=3 {
            if tick == 2 {
                sim.step(&[
                    Command::Despawn(old),
                    Command::Spawn(ComponentEntity {
                        id: StableId(9),
                        position: Position::default(),
                        velocity: Velocity::default(),
                        health: 5,
                        ownership: None,
                        visual: Some(2),
                    }),
                ])
                .expect("replacement");
            } else {
                sim.step(&[]).expect("tick");
            }
            outbox.capture(&sim.published_page().expect("page"));
        }
        let mut seen = Vec::new();
        while outbox
            .poll(sim.timeline(), |projection| {
                let id = projection.id().expect("queued page identity");
                seen.push((
                    id.tick,
                    projection.is_baseline(),
                    projection.despawns().len(),
                    projection.spawns().len(),
                ));
            })
            .expect("no gap")
        {}
        assert_eq!(
            seen,
            [
                (0, true, 0, 1),
                (1, false, 0, 0),
                (2, false, 1, 1),
                (3, false, 0, 0)
            ]
        );
        for _ in 4..=8 {
            sim.step(&[]).expect("later tick");
            outbox.capture(&sim.published_page().expect("page"));
        }
        assert_eq!(
            outbox
                .poll(sim.timeline(), |_| panic!("gap cannot deliver deltas"))
                .expect_err("gap")
                .lost_ticks,
            1
        );
        outbox.reset_full(&sim.published_page().expect("latest"));
        assert!(
            outbox
                .poll(sim.timeline(), |projection| {
                    assert!(projection.is_baseline());
                    assert_eq!(projection.id().expect("baseline identity").tick, 8);
                    assert_eq!(projection.spawns().len(), 1);
                })
                .expect("reset baseline")
        );
    }

    #[test]
    fn rewind_invalidates_queued_old_branch_before_projection_poll() {
        let mut sim = Simulation::new(
            WorldEpoch(8),
            SimConfig {
                tick_hz: NonZeroU32::new(60).expect("rate"),
                entity_capacity: 1,
                peer_capacity: 0,
                actions_per_frame: 0,
                movement_speed: 0.0,
            },
        )
        .expect("world");
        sim.seed(ComponentEntity {
            id: StableId(1),
            position: Position::default(),
            velocity: Velocity([1.0, 0.0, 0.0]),
            health: 1,
            ownership: None,
            visual: None,
        })
        .expect("entity");
        let mut history = sim
            .begin_history(NonZeroUsize::new(2).expect("window"))
            .expect("history");
        let mut outbox = ProjectionOutbox::new(1);
        outbox.reset_full(&sim.published_page().expect("baseline"));
        history.step(&mut sim, &[]).expect("future tick");
        outbox.capture(&sim.published_page().expect("future page"));
        history
            .rewind(
                &mut sim,
                usize::try_from(history.latest_tick()).expect("retained count"),
            )
            .expect("rewind");
        assert!(
            outbox
                .poll(sim.timeline(), |_| panic!("old branch must not escape"))
                .is_err()
        );
        outbox.reset_full(&sim.published_page().expect("restored page"));
        assert!(outbox.poll(sim.timeline(), |_| {}).expect("new baseline"));
    }

    #[test]
    fn matching_rewind_source_stays_a_selective_projection() {
        let mut sim = Simulation::new(
            WorldEpoch(9),
            SimConfig {
                tick_hz: NonZeroU32::new(60).expect("rate"),
                entity_capacity: 1,
                peer_capacity: 0,
                actions_per_frame: 0,
                movement_speed: 0.0,
            },
        )
        .expect("world");
        sim.seed(ComponentEntity {
            id: StableId(1),
            position: Position::default(),
            velocity: Velocity([1.0, 0.0, 0.0]),
            health: 1,
            ownership: None,
            visual: None,
        })
        .expect("entity");
        let mut history = sim
            .begin_history(NonZeroUsize::new(4).expect("window"))
            .expect("history");
        let mut outbox = ProjectionOutbox::new(1);
        let mut stale = ProjectionOutbox::new(1);
        outbox.capture(&sim.published_page().expect("baseline"));
        stale.capture(&sim.published_page().expect("baseline"));
        assert!(
            outbox
                .poll(sim.timeline(), |_| {})
                .expect("baseline delivery")
        );
        assert!(stale.poll(sim.timeline(), |_| {}).expect("stale baseline"));
        for _ in 0..2 {
            history.step(&mut sim, &[]).expect("future tick");
            outbox.capture(&sim.published_page().expect("future page"));
            assert!(
                outbox
                    .poll(sim.timeline(), |_| {})
                    .expect("future delivery")
            );
        }
        let source = sim.published_page().expect("old source");
        history.rewind(&mut sim, 1).expect("rewind");
        let patch = sim.published_page().expect("rewound patch");
        assert_eq!(
            patch.source_endpoint(),
            Some((source.id(), source.timeline()))
        );
        assert!(!patch.is_baseline());
        assert_eq!(outbox.capture(&patch), 1);
        assert!(
            outbox
                .poll(sim.timeline(), |projection| {
                    assert!(!projection.is_baseline());
                    assert_eq!(projection.source, Some((source.id(), source.timeline())));
                    assert_eq!(projection.positions().len(), 1);
                })
                .expect("selective patch delivery")
        );
        assert_eq!(stale.capture(&patch), 0, "foreign source needs resync");
        assert!(
            stale
                .poll(sim.timeline(), |_| panic!(
                    "stale consumer cannot apply patch"
                ))
                .is_err()
        );
    }
}
