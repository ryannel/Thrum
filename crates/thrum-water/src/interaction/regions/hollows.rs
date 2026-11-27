//! Wet connectivity at the authored spacing, shared by the interaction windows.

use super::DRY;
use std::ops::Range;
use thrum_water_model::{BodyKind, Hollows, Map, NO_HOLLOW};

#[derive(Default)]
pub(in crate::interaction) struct WetHollows {
    revision: Option<[u64; 2]>,
    bodies: Vec<Option<WetGrid>>,
    stack: Vec<usize>,
}

impl WetHollows {
    pub(in crate::interaction) fn update(&mut self, map: &Map) {
        let revision = map.query_revision();
        if self.revision == Some(revision) {
            return;
        }
        let mut count = 0;
        while let Some(body) = map.body(count) {
            if self.bodies.len() == count {
                self.bodies.push(None);
            }
            let entry = &mut self.bodies[count];
            if let BodyKind::Hollows(hollows) = &body.kind {
                entry
                    .get_or_insert_with(WetGrid::default)
                    .update(hollows, &mut self.stack);
            } else {
                *entry = None;
            }
            count += 1;
        }
        self.bodies.truncate(count);
        self.revision = Some(revision);
    }

    pub(in crate::interaction) fn is_present(&self) -> bool {
        self.bodies.iter().any(Option::is_some)
    }

    pub(super) fn grid(&self, body: usize) -> Option<&WetGrid> {
        self.bodies.get(body)?.as_ref()
    }

    pub(super) fn at(&self, point: [f64; 2]) -> Option<(usize, Option<u32>)> {
        self.bodies
            .iter()
            .enumerate()
            .find_map(|(body, grid)| {
                let grid = grid.as_ref()?;
                Some((body, grid.components[grid.cell(point)?]))
            })
            .map(|(body, id)| (body, (id != DRY).then_some(id)))
    }

    // A sample's centred derivatives can reach two cell spacings from it.
    // Leave that sample at rest if its support also reaches a different puddle.
    pub(super) fn supports(
        &self,
        point: [f64; 2],
        body: usize,
        component: Option<u32>,
        metres: f64,
    ) -> bool {
        self.bodies.iter().enumerate().all(|(i, grid)| {
            let Some(grid) = grid else { return true };
            let own_component = if i == body { component } else { None };
            grid.supports(point, own_component, metres)
        })
    }
}

#[derive(Default)]
pub(super) struct WetGrid {
    origin: [f64; 2],
    spacing: f64,
    size: [usize; 2],
    end: [f64; 2],
    components: Vec<u32>,
    count: u32,
}

impl WetGrid {
    fn update(&mut self, hollows: &Hollows, stack: &mut Vec<usize>) {
        let (origin, spacing, size) = hollows.lattice();
        let is_wet = |i: usize| {
            let label = hollows.labels()[i];
            label != NO_HOLLOW && hollows.levels()[label as usize] > hollows.heights()[i]
        };
        if self.origin.map(f64::to_bits) == origin.map(f64::to_bits)
            && self.spacing.to_bits() == spacing.to_bits()
            && self.size == size
            && self
                .components
                .iter()
                .enumerate()
                .all(|(i, &id)| (id != DRY) == is_wet(i))
        {
            return;
        }
        (self.origin, self.spacing, self.size) = (origin, spacing, size);
        self.end = std::array::from_fn(|i| origin[i] + size[i] as f64 * spacing);
        self.components.clear();
        self.components
            .extend((0..hollows.cell_count()).map(|i| if is_wet(i) { 0 } else { DRY }));
        self.count = 0;
        let [width, height] = size;
        for i in 0..self.components.len() {
            if self.components[i] != 0 {
                continue;
            }
            self.count += 1;
            self.components[i] = self.count;
            stack.push(i);
            while let Some(at) = stack.pop() {
                let x = at % width;
                let z = at / width;
                let mut visit = |next: usize| {
                    if self.components[next] == 0 {
                        self.components[next] = self.count;
                        stack.push(next);
                    }
                };
                if x > 0 {
                    visit(at - 1);
                }
                if x + 1 < width {
                    visit(at + 1);
                }
                if z > 0 {
                    visit(at - width);
                }
                if z + 1 < height {
                    visit(at + width);
                }
            }
        }
    }

    fn cell(&self, point: [f64; 2]) -> Option<usize> {
        let [x, z] = std::array::from_fn(|axis| self.cells(axis, point[axis], point[axis]));
        Some(z.min()? * self.size[0] + x.min()?)
    }

    pub(super) fn component(&self, point: [f64; 2]) -> Option<u32> {
        let id = self.components[self.cell(point)?];
        (id != DRY).then_some(id)
    }

    fn supports(&self, point: [f64; 2], component: Option<u32>, metres: f64) -> bool {
        if self.count == 0 || (self.count == 1 && component.is_some()) {
            return true;
        }
        let radius = point.map(|p| 2.0 * metres + 4.0 * f64::from(f32::EPSILON) * p.abs().max(1.0));
        if (0..2)
            .any(|i| point[i] + radius[i] < self.origin[i] || point[i] - radius[i] >= self.end[i])
        {
            return true;
        }
        let [x, z] =
            std::array::from_fn(|i| self.cells(i, point[i] - radius[i], point[i] + radius[i]));
        let keep = component.unwrap_or(DRY);
        z.into_iter().all(|z| {
            let row = z * self.size[0];
            self.components[row + x.start..row + x.end]
                .iter()
                .all(|&id| id == DRY || id == keep)
        })
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "coordinates clamped to the authored grid before indexing"
    )]
    fn cells(&self, axis: usize, low: f64, high: f64) -> Range<usize> {
        let cell = |world: f64| ((world - self.origin[axis]) / self.spacing).floor();
        let clamp = |i: f64| i.clamp(0.0, self.size[axis] as f64) as usize;
        clamp(cell(low))..clamp(cell(high) + 1.0)
    }
}
