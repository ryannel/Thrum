use super::{CELLS, SIDE};
use bytemuck::{Pod, Zeroable};
use thrum_water_model::Map;

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
}

#[derive(Clone, Copy)]
struct Wet {
    body: usize,
    depth: f32,
    current: [f64; 2],
}

#[derive(Default)]
pub(super) struct Window {
    key: Option<([i32; 2], [u64; 2])>,
    wet: Vec<Option<Wet>>,
    next_wet: Vec<Option<Wet>>,
    stack: Vec<usize>,
    pub cells: Vec<Cell>,
    pub depths: Vec<u32>,
    pub current: [f32; 2],
}

/// Track source recency by wet support, independent of temporary region numbers.
#[derive(Default)]
pub(super) struct FineActivity {
    key: Option<([i32; 2], [u64; 2])>,
    labels: Vec<u32>,
    pub latest: Vec<Option<u64>>,
}

impl FineActivity {
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
        let key = window.key.expect("labelled fine window");
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
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        reason = "window indices are checked against the fixed 256-cell dimensions; samples use GPU floats"
    )]
    pub(super) fn label(&mut self, map: &Map, origin: [i32; 2], metres: f32, level: usize) -> bool {
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
            map.sample([
                f64::from(x) * f64::from(metres),
                f64::from(z) * f64::from(metres),
            ])
            .filter(|sample| sample.depth > 0.0)
            .map(|sample| Wet {
                body: sample.body,
                depth: sample.depth as f32,
                current: sample.row.current,
            })
        }));
        std::mem::swap(&mut self.wet, &mut self.next_wet);
        let ladder = DEPTHS[level];
        self.cells.clear();
        self.depths.clear();
        let mut current = [0.0; 2];
        let mut wet_count = 0;
        self.cells.extend(self.wet.iter().map(|wet| {
            let mut cell = Cell {
                region: DRY,
                low: REST,
                high: REST,
                blend: 0.0,
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
                        && self.wet[next].is_some_and(|other| other.body == wet.body)
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
        self.key = Some((origin, revision));
        true
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
}
