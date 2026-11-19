//! Slot-indexed tick endpoints and the retained back-to-front world order.

use super::{SpriteRow, SpriteScene};
use crate::extract::SourceTick;
use std::cmp::Ordering;

#[derive(Clone, Copy)]
pub(super) struct OrderedRow {
    pub(super) key: OrderKey,
    centre: [f64; 2],
    radius: f64,
}

#[derive(Clone, Copy)]
pub(super) struct OrderKey {
    layer: i32,
    sort: f64,
    frame_row: bool,
    identity: u128,
}

impl OrderKey {
    pub(super) fn compare(self, other: Self) -> Ordering {
        self.layer
            .cmp(&other.layer)
            .then_with(|| {
                self.sort
                    .partial_cmp(&other.sort)
                    .expect("finite sprite sort values")
            })
            .then_with(|| self.frame_row.cmp(&other.frame_row))
            .then_with(|| self.identity.cmp(&other.identity))
    }
}

#[derive(Clone, Copy)]
pub(super) struct Pose {
    pub(super) position: [f64; 2],
    pub(super) rotation: f32,
    pub(super) scale: [f32; 2],
}

impl From<SpriteRow> for Pose {
    fn from(row: SpriteRow) -> Self {
        Self {
            position: row.position,
            rotation: row.rotation,
            scale: row.scale,
        }
    }
}

pub(super) fn describe(
    scene: &SpriteScene,
    row: SpriteRow,
    frame_row: bool,
    identity: u128,
) -> Option<OrderedRow> {
    let source = &scene.sheets.get(row.sheet as usize)?.sheet;
    let rect = source.frames.get(row.frame as usize)?;
    let sort = row.sort.unwrap_or(-row.position[1]);
    let [x, y] = [0, 1].map(|axis| {
        f64::from(row.scale[axis] / source.texels_per_unit) * f64::from(rect.size[axis]) * 0.5
    });
    let (sin, cos) = f64::from(row.rotation).sin_cos();
    let centre = [
        row.position[0] + x * cos + y * sin,
        row.position[1] + x * sin - y * cos,
    ];
    let radius = x.hypot(y);
    // Headroom for GPU subtraction, interpolation and both quad axes.
    let limit = f64::from(f32::MAX) / 16.0;
    if !sort.is_finite()
        || row
            .position
            .into_iter()
            .any(|value| !(-limit..=limit).contains(&value))
        || !(radius > 0.0 && radius <= limit)
        || x == 0.0
        || y == 0.0
        || !row.rotation.is_finite()
        || !row
            .tint
            .into_iter()
            .chain(row.flash)
            .all(|value| (0.0..=65_504.0).contains(&value))
        || row.tint[3] > 1.0
        || row.flash[3] > 1.0
        || !(0.0..=1.0).contains(&row.additive)
    {
        return None;
    }
    Some(OrderedRow {
        key: OrderKey {
            layer: row.layer,
            sort,
            frame_row,
            identity,
        },
        centre,
        radius,
    })
}

#[derive(Clone, Copy)]
pub(super) struct WorldRow {
    pub(super) source: SourceTick,
    generation: u64,
    pub(super) current: SpriteRow,
    pub(super) previous: Pose,
    pub(super) ordered: OrderedRow,
    pub(super) moving: bool,
    has_history: bool,
}

/// Bounded world sprite writer for one immutable tick publication.
pub struct SpriteSink<'a> {
    world: &'a mut SpriteWorld,
    scene: &'a SpriteScene,
    source: SourceTick,
}

impl SpriteSink<'_> {
    /// Map a visible entity's physical slot, slot generation and stable identity.
    /// Invalid, duplicate and out-of-capacity rows are counted and skipped.
    pub fn push(&mut self, slot: u32, generation: u64, identity: u128, row: SpriteRow) {
        if !self.try_push(slot, generation, identity, row) {
            self.world.dropped = self.world.dropped.saturating_add(1);
        }
    }

    fn try_push(&mut self, slot: u32, generation: u64, identity: u128, row: SpriteRow) -> bool {
        let Some(saved) = self.world.slots.get_mut(slot as usize) else {
            return false;
        };
        if saved.is_some_and(|saved| saved.source == self.source) {
            return false;
        }
        let Some(ordered) = describe(self.scene, row, false, identity) else {
            return false;
        };
        let old = saved.filter(|old| {
            old.source.epoch == self.source.epoch
                && old.source.timeline == self.source.timeline
                && old.source.tick.checked_add(1) == Some(self.source.tick)
                && old.generation == generation
                && old.ordered.key.identity == identity
        });
        let previous = old.map_or(row, |old| old.current);
        *saved = Some(WorldRow {
            source: self.source,
            generation,
            current: row,
            previous: previous.into(),
            ordered,
            moving: previous.position.map(f64::to_bits) != row.position.map(f64::to_bits)
                || previous.rotation.to_bits() != row.rotation.to_bits()
                || previous.scale.map(f32::to_bits) != row.scale.map(f32::to_bits),
            has_history: old.is_some(),
        });
        self.world.visited.push(slot);
        true
    }
}

pub(super) struct SpriteWorld {
    source: Option<SourceTick>,
    pub(super) slots: Vec<Option<WorldRow>>,
    pub(super) order: Vec<u32>,
    visited: Vec<u32>,
    fresh: Vec<u32>,
    merged: Vec<u32>,
    pub(super) dropped: u32,
}

impl SpriteWorld {
    pub(super) fn new(capacity: u32) -> Self {
        let capacity = capacity as usize;
        Self {
            source: None,
            slots: vec![None; capacity],
            order: Vec::with_capacity(capacity),
            visited: Vec::with_capacity(capacity),
            fresh: Vec::with_capacity(capacity),
            merged: Vec::with_capacity(capacity),
            dropped: 0,
        }
    }

    pub(super) fn extract(
        &mut self,
        scene: &SpriteScene,
        source: SourceTick,
        visit: impl FnOnce(&mut SpriteSink<'_>),
    ) {
        self.dropped = 0;
        self.visited.clear();
        visit(&mut SpriteSink {
            world: self,
            scene,
            source,
        });
        self.source = Some(source);
    }

    pub(super) fn clear_history(&mut self) {
        self.source = None;
        self.slots.fill(None);
        self.order.clear();
    }

    pub(super) fn source(&self) -> Option<SourceTick> {
        self.source
    }

    pub(super) fn row(&self, slot: u32) -> &WorldRow {
        self.slots[slot as usize]
            .as_ref()
            .expect("only extracted slots enter the order")
    }

    pub(super) fn order_tick(&mut self, band: f64) {
        let row = |slot: u32| self.slots[slot as usize].expect("slot was extracted");
        let compare =
            |left: &u32, right: &u32| row(*left).ordered.key.compare(row(*right).ordered.key);
        if band == 0.0 {
            self.order.clear();
            self.order.extend_from_slice(&self.visited);
            self.order.sort_unstable_by(compare);
            return;
        }

        self.order.retain(|slot| {
            let row = row(*slot);
            Some(row.source) == self.source && row.has_history
        });

        // Each swap can expose a new inversion on either side. Back up one
        // place after the swap and walk forward again once that pair settles.
        let mut index = 1;
        while index < self.order.len() {
            let left = row(self.order[index - 1]).ordered;
            let right = row(self.order[index]).ordered;
            if exchanges(left, right, band) {
                self.order.swap(index - 1, index);
                index = (index - 1).max(1);
            } else {
                index += 1;
            }
        }

        self.fresh.clear();
        self.fresh.extend(
            self.visited
                .iter()
                .copied()
                .filter(|slot| !row(*slot).has_history),
        );
        if !self.fresh.is_empty() {
            self.fresh.sort_unstable_by(compare);
            self.merged.clear();
            merge_order(&self.order, &self.fresh, &mut self.merged, compare);
            std::mem::swap(&mut self.order, &mut self.merged);
        }
    }
}

fn exchanges(left: OrderedRow, right: OrderedRow, band: f64) -> bool {
    if left.key.compare(right.key) != Ordering::Greater {
        return false;
    }
    if left.key.layer != right.key.layer || left.key.sort - right.key.sort > band {
        return true;
    }
    let reach = left.radius + right.radius + 2.0 * band;
    let dx = left.centre[0] - right.centre[0];
    let dy = left.centre[1] - right.centre[1];
    dx * dx + dy * dy > reach * reach
}

// The retained order can have local inversions within the band. A sorted
// addition belongs at the first crossing of its exact key in that list.
pub(super) fn merge_order(
    kept: &[u32],
    mut additions: &[u32],
    out: &mut Vec<u32>,
    compare: impl Fn(&u32, &u32) -> Ordering,
) {
    for slot in kept {
        let mut take = 0;
        while take < additions.len() && compare(&additions[take], slot) == Ordering::Less {
            take += 1;
        }
        out.extend_from_slice(&additions[..take]);
        additions = &additions[take..];
        out.push(*slot);
    }
    out.extend_from_slice(additions);
}
