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

/// The geometric endpoint of a world sprite row.
#[derive(Clone, Copy, Debug)]
pub struct SpritePose {
    /// World position of the top-left pivot.
    pub position: [f64; 2],
    /// Counterclockwise rotation about the pivot, in radians.
    pub rotation: f32,
    /// Signed local X and Y scale.
    pub scale: [f32; 2],
}

impl From<SpriteRow> for SpritePose {
    fn from(row: SpriteRow) -> Self {
        Self {
            position: row.position,
            rotation: row.rotation,
            scale: row.scale,
        }
    }
}

// Headroom for GPU subtraction, interpolation and both quad axes.
const POSE_LIMIT: f64 = f32::MAX as f64 / 16.0;

impl SpritePose {
    fn geometry(self, units: f32, size: [u32; 2]) -> Option<([f64; 2], f64)> {
        let half_size =
            [0, 1].map(|axis| f64::from(self.scale[axis] / units) * f64::from(size[axis]) * 0.5);
        let [x, y] = half_size;
        let radius = x.hypot(y);
        (self
            .position
            .into_iter()
            .all(|v| (-POSE_LIMIT..=POSE_LIMIT).contains(&v))
            && radius > 0.0
            && radius <= POSE_LIMIT
            && x != 0.0
            && y != 0.0
            && self.rotation.is_finite())
        .then_some((half_size, radius))
    }

    fn differs_from(self, row: SpriteRow) -> bool {
        self.position.map(f64::to_bits) != row.position.map(f64::to_bits)
            || self.rotation.to_bits() != row.rotation.to_bits()
            || self.scale.map(f32::to_bits) != row.scale.map(f32::to_bits)
    }
}

pub(super) fn describe(
    scene: &SpriteScene,
    row: SpriteRow,
    frame_row: bool,
    identity: u128,
) -> Option<OrderedRow> {
    let (density, size) = geometry_input(scene, row)?;
    let sort = row.sort.unwrap_or(-row.position[1]);
    let ([x, y], radius) = SpritePose::from(row).geometry(density, size)?;
    let (sin, cos) = f64::from(row.rotation).sin_cos();
    let centre = [
        row.position[0] + x * cos + y * sin,
        row.position[1] + x * sin - y * cos,
    ];
    if !sort.is_finite()
        || !row
            .tint
            .into_iter()
            .chain(row.flash)
            .all(|value| (0.0..=crate::shader_constants::MAX_FINITE_HALF).contains(&value))
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

fn geometry_input(scene: &SpriteScene, row: SpriteRow) -> Option<(f32, [u32; 2])> {
    let slot = row.sheet as usize;
    if let Some(sheet) = scene.sheets.get(slot) {
        let rect = sheet.sheet.frames.get(row.frame as usize)?;
        return Some((sheet.sheet.texels_per_unit, rect.size));
    }
    // Keep pending rows and their previous poses. Their real bounds are resolved
    // from the offered publication immediately before recording.
    scene
        .cooked
        .get(slot - scene.sheets.len())
        .map(|_| (1.0, [1, 1]))
}

#[derive(Clone, Copy)]
pub(super) struct WorldRow {
    pub(super) source: SourceTick,
    generation: u64,
    pub(super) current: SpriteRow,
    pub(super) previous: SpritePose,
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

#[derive(Clone, Copy)]
enum PreviousPose {
    Retained,
    Published(Option<SpritePose>),
}

impl SpriteSink<'_> {
    /// Map a visible entity's physical slot, slot generation and stable identity.
    /// Invalid, duplicate and out-of-capacity rows are counted and skipped.
    pub fn push(&mut self, slot: u32, generation: u64, identity: u128, row: SpriteRow) {
        self.write(slot, generation, identity, row, PreviousPose::Retained);
    }

    /// Map a row whose tick page also supplies the previous simulation endpoint.
    /// The source must supply that pose only for the same identity and slot
    /// generation on the immediately preceding tick of this timeline. Use
    /// `None` when the row did not exist there. A missing or unusable pose
    /// holds the committed pose, including on a timeline or epoch change.
    pub fn push_pair(
        &mut self,
        slot: u32,
        generation: u64,
        identity: u128,
        row: SpriteRow,
        previous: Option<SpritePose>,
    ) {
        self.write(
            slot,
            generation,
            identity,
            row,
            PreviousPose::Published(previous),
        );
    }

    fn write(
        &mut self,
        slot: u32,
        generation: u64,
        identity: u128,
        row: SpriteRow,
        pose: PreviousPose,
    ) {
        if !self.try_push(slot, generation, identity, row, pose) {
            self.world.dropped = self.world.dropped.saturating_add(1);
        }
    }

    fn try_push(
        &mut self,
        slot: u32,
        generation: u64,
        identity: u128,
        row: SpriteRow,
        pose: PreviousPose,
    ) -> bool {
        let Some(saved) = self.world.slots.get_mut(slot as usize) else {
            return false;
        };
        if saved.is_some_and(|saved| saved.source == self.source) {
            return false;
        }
        let Some(ordered) = describe(self.scene, row, false, identity) else {
            return false;
        };
        let same_timeline = self.world.source.filter(|source| {
            source.epoch == self.source.epoch && source.timeline == self.source.timeline
        });
        let old = saved.filter(|old| {
            Some(old.source) == same_timeline
                && old.generation == generation
                && old.ordered.key.identity == identity
        });
        // The retained order follows observed identities. The pose pair may
        // come from an adjacent simulation tick the host did not observe.
        let previous = match pose {
            PreviousPose::Retained => old
                .filter(|old| old.source.tick.checked_add(1) == Some(self.source.tick))
                .map(|old| old.current.into()),
            PreviousPose::Published(previous) => previous.filter(|pose| {
                let (density, size) = geometry_input(self.scene, row).expect("admitted sprite row");
                same_timeline.is_some_and(|source| source.tick < self.source.tick)
                    && pose.geometry(density, size).is_some()
            }),
        }
        .unwrap_or_else(|| row.into());
        *saved = Some(WorldRow {
            source: self.source,
            generation,
            current: row,
            previous,
            ordered,
            moving: previous.differs_from(row),
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

    pub(super) fn refresh_cooked(
        &mut self,
        scene: &SpriteScene,
        same_tick: bool,
        resolve: impl Fn(SpriteRow) -> Option<super::pass::FrameInput>,
    ) {
        for &slot in &self.visited {
            let row = self.slots[slot as usize]
                .as_mut()
                .expect("visited sprite row");
            row.has_history |= same_tick;
            if (row.current.sheet as usize) < scene.sheets.len() {
                continue;
            }
            row.ordered.centre = row.current.position;
            row.ordered.radius = 0.0;
            let Some(input) = resolve(row.current) else {
                continue;
            };
            let Some(([x, y], radius)) =
                SpritePose::from(row.current).geometry(input.density, input.size)
            else {
                continue;
            };
            let [dx, dy] = [0, 1].map(|axis| {
                f64::from(input.offset[axis]) * f64::from(row.current.scale[axis] / input.density)
            });
            let (sin, cos) = f64::from(row.current.rotation).sin_cos();
            row.ordered.centre[0] += (x + dx) * cos + (y + dy) * sin;
            row.ordered.centre[1] += (x + dx) * sin - (y + dy) * cos;
            row.ordered.radius = radius;
        }
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
