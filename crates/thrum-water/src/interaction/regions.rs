use super::{CELLS, SIDE};
use bytemuck::{Pod, Zeroable};
use thrum_water_model::Map;

mod currents;
mod hollows;
pub(super) use currents::{CARRIAGE_COUNTS, CellCarriage};
pub(super) use hollows::WetHollows;

pub(super) const DRY: u32 = u32::MAX;
pub(super) const REST: u32 = u32::MAX;
pub(super) const DEPTHS: [&[f32]; 3] = [
    &[0.25, 0.5, 1.0, 2.0, 4.0, 8.0],
    &[1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0],
    &[0.005, 0.01, 0.02, 0.04, 0.08, 0.16, 0.32],
];

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(super) struct Cell {
    pub region: u32,
    pub low: u32,
    pub high: u32,
    pub blend: f32,
    pub shore_loss: f32,
}

#[derive(Clone, Copy)]
struct Wet {
    body: usize,
    component: Option<u32>,
    depth: f32,
    current: [f64; 2],
}

#[derive(Default)]
pub(super) struct Window {
    key: Option<([i32; 2], [u64; 2])>,
    wet: Vec<Option<Wet>>,
    next_wet: Vec<Option<Wet>>,
    stack: Vec<usize>,
    hollow_regions: Vec<(usize, u32)>,
    pub cells: Vec<Cell>,
    pub depths: Vec<u32>,
    pub current: [f32; 2],
    pub carriage: Option<currents::Carriage>,
}

/// Track source recency by wet support, independent of temporary region numbers.
#[derive(Default)]
pub(super) struct RegionActivity {
    key: Option<([i32; 2], [u64; 2])>,
    labels: Vec<u32>,
    pub latest: Vec<Option<u64>>,
}

impl RegionActivity {
    pub(super) fn reset(&mut self) {
        self.key = None;
        self.latest.clear();
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        reason = "coordinates clipped to the fixed grid before indexing"
    )]
    pub(super) fn inherit(&mut self, old: &Self, window: &Window) {
        let key = window.key.expect("labelled interaction window");
        self.latest.clear();
        self.latest.resize(window.depths.len(), None);
        if old.key == Some(key) {
            self.latest.copy_from_slice(&old.latest);
        } else if let Some((old_origin, _)) = old.key
            && old.latest.iter().any(Option::is_some)
        {
            let [dx, dz] = std::array::from_fn(|axis| key.0[axis] - old_origin[axis]);
            let overlap = |shift: i32| (-shift).max(0)..(SIDE as i32 - shift).min(SIDE as i32);
            for z in overlap(dz) {
                for x in overlap(dx) {
                    let region = window.cells[z as usize * SIDE + x as usize].region;
                    if region == DRY {
                        continue;
                    }
                    let old_region = old.labels[(z + dz) as usize * SIDE + (x + dx) as usize];
                    if old_region != DRY {
                        let latest = &mut self.latest[region as usize];
                        *latest = (*latest).max(old.latest[old_region as usize]);
                    }
                }
            }
        }
        if self.key != Some(key) {
            self.labels.clear();
            self.labels
                .extend(window.cells.iter().map(|cell| cell.region));
            self.key = Some(key);
        }
    }
}

impl Window {
    pub(super) fn new(varying_current: bool) -> Self {
        Self {
            carriage: varying_current.then(currents::Carriage::default),
            ..Self::default()
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        reason = "window indices are checked against the fixed 256-cell dimensions; samples use GPU floats"
    )]
    pub(super) fn label(
        &mut self,
        map: &Map,
        hollows: &WetHollows,
        origin: [i32; 2],
        metres: f32,
        level: usize,
    ) -> bool {
        let revision = map.query_revision();
        if self.key == Some((origin, revision)) {
            return false;
        }
        let previous = self
            .key
            .filter(|(_, previous_revision)| *previous_revision == revision)
            .map(|(old, _)| old);
        self.next_wet.clear();
        self.next_wet.extend((0..CELLS).map(|i| {
            let x = origin[0] + (i % SIDE) as i32;
            let z = origin[1] + (i / SIDE) as i32;
            if let Some([ox, oz]) = previous {
                let (dx, dz) = (x - ox, z - oz);
                if (0..SIDE as i32).contains(&dx) && (0..SIDE as i32).contains(&dz) {
                    return self.wet[dz as usize * SIDE + dx as usize];
                }
            }
            let metres = f64::from(metres);
            let point = [f64::from(x) * metres, f64::from(z) * metres];
            let sample = map.sample(point).filter(|sample| sample.depth > 0.0)?;
            let component = if let Some(grid) = hollows.grid(sample.body) {
                Some(grid.component(point)?)
            } else {
                None
            };
            if !hollows.supports(point, sample.body, component, metres) {
                return None;
            }
            Some(Wet {
                body: sample.body,
                component,
                depth: sample.depth as f32,
                current: sample.row.current,
            })
        }));
        std::mem::swap(&mut self.wet, &mut self.next_wet);
        let ladder = DEPTHS[level];
        self.cells.clear();
        self.depths.clear();
        self.hollow_regions.clear();
        let mut current = [0.0; 2];
        let mut wet_count = 0;
        self.cells.extend(self.wet.iter().map(|wet| {
            let mut cell = Cell {
                region: DRY,
                low: REST,
                high: REST,
                blend: 0.0,
                shore_loss: 0.0,
            };
            if let Some(wet) = wet {
                for (sum, value) in current.iter_mut().zip(wet.current) {
                    *sum += value;
                }
                wet_count += 1;
                let index = ladder.partition_point(|depth| *depth < wet.depth);
                let hi = index.min(ladder.len() - 1);
                if index == ladder.len() || wet.depth.to_bits() == ladder[hi].to_bits() {
                    cell.low = hi as u32;
                    cell.high = hi as u32;
                } else if hi == 0 {
                    cell.high = 0;
                    cell.blend = wet.depth / ladder[0];
                } else {
                    let low = ladder[hi - 1].sqrt();
                    cell.low = (hi - 1) as u32;
                    cell.high = hi as u32;
                    cell.blend = (wet.depth.sqrt() - low) / (ladder[hi].sqrt() - low);
                }
            }
            cell
        }));
        self.current = current.map(|sum| (sum / f64::from(wet_count.max(1))) as f32);

        for i in 0..CELLS {
            let Some(wet) = self.wet[i] else { continue };
            if self.cells[i].region != DRY {
                continue;
            }
            let region = self.depths.len() as u32;
            if let Some(component) = wet.component {
                self.hollow_regions.push((wet.body, component));
            }
            let mut depths = 0;
            self.stack.push(i);
            self.cells[i].region = region;
            while let Some(at) = self.stack.pop() {
                let cell = self.cells[at];
                depths |= 1 << cell.high;
                if cell.low != REST {
                    depths |= 1 << cell.low;
                }
                let x = at % SIDE;
                let mut visit = |next: usize| {
                    if self.cells[next].region == DRY
                        && self.wet[next].is_some_and(|other| {
                            other.body == wet.body && other.component == wet.component
                        })
                    {
                        self.cells[next].region = region;
                        self.stack.push(next);
                    }
                };
                if x > 0 {
                    visit(at - 1);
                }
                if x < SIDE - 1 {
                    visit(at + 1);
                }
                if at >= SIDE {
                    visit(at - SIDE);
                }
                if at < CELLS - SIDE {
                    visit(at + SIDE);
                }
            }
            self.depths.push(depths);
        }
        if let Some(carriage) = &mut self.carriage {
            carriage.label(&self.cells, &self.wet, self.depths.len());
        }
        if level == 2 && self.wet.iter().flatten().any(|wet| wet.component.is_some()) {
            self.absorb_at_shore();
        }
        self.key = Some((origin, revision));
        true
    }

    fn absorb_at_shore(&mut self) {
        // Fade the fine wave into the bank over eight samples. A gradual
        // sponge absorbs energy before the support mask cuts the field.
        const WIDTH: f32 = 8.0;
        let distance = |cells: &[Cell], i: usize, adjacent: usize| {
            if cells[i].region == cells[adjacent].region {
                cells[adjacent].shore_loss + 1.0
            } else {
                1.0
            }
        };
        for i in 0..CELLS {
            let mut d = WIDTH;
            if i % SIDE > 0 {
                d = d.min(distance(&self.cells, i, i - 1));
            }
            if i >= SIDE {
                d = d.min(distance(&self.cells, i, i - SIDE));
            }
            self.cells[i].shore_loss = d;
        }
        for i in (0..CELLS).rev() {
            let mut d = self.cells[i].shore_loss;
            if i % SIDE + 1 < SIDE {
                d = d.min(distance(&self.cells, i, i + 1));
            }
            if i + SIDE < CELLS {
                d = d.min(distance(&self.cells, i, i + SIDE));
            }
            self.cells[i].shore_loss = d;
        }
        for (cell, wet) in self.cells.iter_mut().zip(&self.wet) {
            cell.shore_loss = if wet.is_some_and(|wet| wet.component.is_some()) {
                28.0 * (1.0 - cell.shore_loss / WIDTH).powi(2)
            } else {
                0.0
            };
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        reason = "rounded source cells are checked against the fixed 256-cell dimensions"
    )]
    pub(super) fn region(&self, place: [f32; 2]) -> u32 {
        let [x, z] = place.map(|v| v.round() as i32);
        if (0..SIDE as i32).contains(&x) && (0..SIDE as i32).contains(&z) {
            self.cells[z as usize * SIDE + x as usize].region
        } else {
            DRY
        }
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        reason = "source coordinates are checked against the fixed 256-cell dimensions"
    )]
    pub(super) fn source_region(
        &self,
        place: [f32; 2],
        world: [f64; 2],
        hollows: &WetHollows,
    ) -> u32 {
        let [x, z] = place.map(|v| v.round() as i32);
        if !(0..SIDE as i32).contains(&x) || !(0..SIDE as i32).contains(&z) {
            return DRY;
        }
        let nearest = z as usize * SIDE + x as usize;
        let (body, component) = match hollows.at(world) {
            Some((body, Some(component))) => (body, component),
            Some((_, None)) => return DRY,
            None if self.wet[nearest].is_some_and(|wet| wet.component.is_some()) => return DRY,
            None => return self.cells[nearest].region,
        };
        // A source near a sill may round to a missing sample or to the
        // opposite puddle. Use the closest represented sample of its own.
        if self.wet[nearest].is_some_and(|wet| wet.body == body && wet.component == Some(component))
        {
            return self.cells[nearest].region;
        }
        if !self.hollow_regions.contains(&(body, component)) {
            return DRY;
        }
        let last = SIDE as i32 - 1;
        let mut best = (f32::INFINITY, DRY);
        let visit = |sx: i32, sz: i32, best: &mut (f32, u32)| {
            let i = sz as usize * SIDE + sx as usize;
            if self.wet[i].is_some_and(|wet| wet.body == body && wet.component == Some(component)) {
                let candidate = (
                    (place[0] - sx as f32).powi(2) + (place[1] - sz as f32).powi(2),
                    self.cells[i].region,
                );
                if candidate < *best {
                    *best = candidate;
                }
            }
        };
        for radius in 1..=last {
            let (left, right, bottom, top) = (x - radius, x + radius, z - radius, z + radius);
            for sx in left.max(0)..=right.min(last) {
                if bottom >= 0 {
                    visit(sx, bottom, &mut best);
                }
                if top <= last {
                    visit(sx, top, &mut best);
                }
            }
            for sz in (bottom + 1).max(0)..=(top - 1).min(last) {
                if left >= 0 {
                    visit(left, sz, &mut best);
                }
                if right <= last {
                    visit(right, sz, &mut best);
                }
            }
            // A rounded coordinate is at most half a cell off the source.
            // Points on the next perimeter cannot be closer than this.
            if best.0 < (radius as f32 + 0.5).powi(2) {
                break;
            }
        }
        best.1
    }
}
