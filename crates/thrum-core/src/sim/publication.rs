//! Reused, immutable visual read-model pages from committed simulation ticks.
//! Pages are consumer-specific; they never alias the next mutable world step.

use std::{num::NonZeroUsize, sync::Arc};

use crate::{StableId, WorldEpoch};

use super::{Position, Simulation};

struct VisualSnapshot {
    epoch: WorldEpoch,
    tick: u64,
    capacity: usize,
    ids: Vec<StableId>,
    positions: Vec<Position>,
    visuals: Vec<u64>,
    dropped_capacity: usize,
}

impl VisualSnapshot {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            epoch: WorldEpoch(0),
            tick: 0,
            capacity,
            ids: Vec::with_capacity(capacity),
            positions: Vec::with_capacity(capacity),
            visuals: Vec::with_capacity(capacity),
            dropped_capacity: 0,
        }
    }

    fn capture(&mut self, sim: &Simulation) {
        self.epoch = sim.epoch();
        self.tick = sim.tick();
        self.ids.clear();
        self.positions.clear();
        self.visuals.clear();
        self.dropped_capacity = 0;
        sim.visit_components(|view| {
            if let Some(visual) = view.visual {
                if self.ids.len() == self.capacity {
                    self.dropped_capacity += 1;
                } else {
                    self.ids.push(view.id);
                    self.positions.push(view.position);
                    self.visuals.push(visual);
                }
            }
        });
    }
}

/// Immutable visual page retained independently of later world ticks or slot
/// reuse. Cloning a lease shares its preallocated page without copying rows.
#[derive(Clone)]
pub struct VisualSnapshotLease(Arc<VisualSnapshot>);

impl VisualSnapshotLease {
    /// Source world generation.
    pub fn epoch(&self) -> WorldEpoch {
        self.0.epoch
    }

    /// Complete committed source tick.
    pub fn tick(&self) -> u64 {
        self.0.tick
    }

    /// Stable identities of retained visual rows.
    pub fn ids(&self) -> &[StableId] {
        &self.0.ids
    }

    /// World-space positions aligned with `ids`.
    pub fn positions(&self) -> &[Position] {
        &self.0.positions
    }

    /// Opaque visual selections aligned with `ids`.
    pub fn visuals(&self) -> &[u64] {
        &self.0.visuals
    }

    /// Optional visual rows omitted at the configured page capacity.
    pub fn dropped_capacity(&self) -> usize {
        self.0.dropped_capacity
    }
}

/// Finite preallocated visual-page pool. A held lease makes its slot
/// unavailable; running out sheds optional publication, never a world tick.
pub struct VisualSnapshotPool {
    slots: Vec<Arc<VisualSnapshot>>,
    cursor: usize,
    dropped_views: u64,
}

impl VisualSnapshotPool {
    /// Reserve `slots` pages with room for `visual_capacity` rows each.
    pub fn new(slots: NonZeroUsize, visual_capacity: usize) -> Self {
        Self {
            slots: (0..slots.get())
                .map(|_| Arc::new(VisualSnapshot::with_capacity(visual_capacity)))
                .collect(),
            cursor: 0,
            dropped_views: 0,
        }
    }

    /// Publish one committed visual page, or count a skipped optional view if
    /// every slot is still leased. The world remains untouched on either path.
    pub fn publish(&mut self, sim: &Simulation) -> Option<VisualSnapshotLease> {
        for offset in 0..self.slots.len() {
            let index = (self.cursor + offset) % self.slots.len();
            if let Some(slot) = Arc::get_mut(&mut self.slots[index]) {
                slot.capture(sim);
                self.cursor = (index + 1) % self.slots.len();
                return Some(VisualSnapshotLease(Arc::clone(&self.slots[index])));
            }
        }
        self.dropped_views = self.dropped_views.saturating_add(1);
        None
    }

    /// Optional publications skipped because all pages were leased.
    pub fn dropped_views(&self) -> u64 {
        self.dropped_views
    }
}
