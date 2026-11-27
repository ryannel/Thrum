use super::{Cell, DRY, Wet};
use bytemuck::{Pod, Zeroable};
use glam::DVec2;

pub(crate) const CARRIAGE_COUNTS: usize = 4;

/// Weights for the two-, three- and four-current ladders, indexed by
/// the number of admitted currents minus two.
#[repr(C)]
#[derive(Clone, Copy, Default, Pod, Zeroable)]
pub(crate) struct CellCarriage {
    pub weights: [[f32; CARRIAGE_COUNTS]; CARRIAGE_COUNTS - 1],
}

#[derive(Default)]
pub(in crate::interaction) struct Carriage {
    pub rows: Vec<CellCarriage>,
    pub regions: Vec<RegionCarriage>,
}

#[derive(Clone, Copy)]
pub(crate) struct RegionCarriage {
    pub currents: [[DVec2; CARRIAGE_COUNTS]; CARRIAGE_COUNTS],
    pub error: [f64; CARRIAGE_COUNTS],
    pub count: usize,
    linear: bool,
}

#[derive(Clone, Copy, Default)]
struct Range {
    first: Option<DVec2>,
    axis: DVec2,
    sum: DVec2,
    count: u32,
    low: DVec2,
    high: DVec2,
}

impl Range {
    fn include(&mut self, value: DVec2) {
        let delta = value - *self.first.get_or_insert(value);
        if delta.length_squared() > self.axis.length_squared() {
            self.axis = delta;
        }
        self.sum += value;
        self.count += 1;
    }

    fn bounds(&mut self, value: DVec2) {
        let basis = DVec2::new(value.dot(self.axis), value.dot(self.axis.perp()));
        self.low = self.low.min(basis);
        self.high = self.high.max(basis);
    }

    fn ladder(self) -> RegionCarriage {
        let mean = self.sum / f64::from(self.count);
        let world = |p: DVec2| self.axis * p.x + self.axis.perp() * p.y;
        let origin = world(self.low);
        let x = self.axis * (self.high.x - self.low.x);
        let y = self.axis.perp() * (self.high.y - self.low.y);
        let centre = origin + y * 0.5;
        let line = [
            [mean; 4],
            [centre, centre + x, mean, mean],
            [centre, centre + x * 0.5, centre + x, mean],
            [
                centre,
                centre + x / 3.0,
                centre + x * (2.0 / 3.0),
                centre + x,
            ],
        ];
        let count = if x.length_squared() <= 1e-12 { 1 } else { 4 };
        let linear = y.length_squared() <= 1e-12;
        let currents = if linear {
            line
        } else {
            [
                line[0],
                line[1],
                [origin, origin + 2.0 * x, origin + 2.0 * y, mean],
                [origin, origin + x, origin + x + y, origin + y],
            ]
        };
        RegionCarriage {
            currents,
            error: [0.0; 4],
            count,
            linear,
        }
    }
}

impl Carriage {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "weights are in [0, 1] for WGSL"
    )]
    pub(super) fn label(&mut self, cells: &[Cell], wet: &[Option<Wet>], count: usize) {
        let mut ranges = vec![Range::default(); count];
        let samples = || {
            cells
                .iter()
                .zip(wet)
                .filter(|(cell, _)| cell.region != DRY)
                .map(|(cell, wet)| {
                    (
                        cell.region as usize,
                        DVec2::from_array(wet.expect("labelled cell is wet").current),
                    )
                })
        };
        for (region, value) in samples() {
            ranges[region].include(value);
        }
        for range in &mut ranges {
            range.axis = range.axis.try_normalize().unwrap_or(DVec2::X);
            range.low = DVec2::splat(f64::INFINITY);
            range.high = DVec2::splat(f64::NEG_INFINITY);
        }
        for (region, value) in samples() {
            ranges[region].bounds(value);
        }
        self.regions.clear();
        self.regions.extend(ranges.into_iter().map(Range::ladder));
        self.rows.clear();
        self.rows.extend(cells.iter().zip(wet).map(|(cell, wet)| {
            let mut row = CellCarriage::default();
            if cell.region == DRY {
                return row;
            }
            let region = &mut self.regions[cell.region as usize];
            let value = DVec2::from_array(wet.expect("labelled cell is wet").current);
            for count in 1..=region.count {
                let points = &region.currents[count - 1][..count];
                let (distance, weights) = if region.linear && count > 2 {
                    interpolate_line(value, points)
                } else {
                    interpolate(value, points)
                };
                region.error[count - 1] = region.error[count - 1].max(distance.sqrt());
                if count > 1 {
                    row.weights[count - 2] = weights.map(|weight| weight as f32);
                }
            }
            row
        }));
    }
}

// A straight reach takes the adjacent pair of carriage currents.
fn interpolate_line(value: DVec2, points: &[DVec2]) -> (f64, [f64; 4]) {
    let mut best = (f64::INFINITY, [0.0; 4]);
    for (index, pair) in points.windows(2).enumerate() {
        let (distance, weights) = interpolate(value, pair);
        if distance < best.0 {
            let mut adjacent = [0.0; 4];
            adjacent[index..index + 2].copy_from_slice(&weights[..2]);
            best = (distance, adjacent);
        }
    }
    best
}

// A convex polygon is split at the diagonal from vertex zero. Where the
// point is outside the span, the closest segment supplies its weights.
fn interpolate(value: DVec2, points: &[DVec2]) -> (f64, [f64; 4]) {
    if points.len() == 1 {
        return (value.distance_squared(points[0]), [1.0, 0.0, 0.0, 0.0]);
    }
    for i in 1..points.len() - 1 {
        let a = points[i] - points[0];
        let b = points[i + 1] - points[0];
        let area = a.perp_dot(b);
        if area.abs() <= 1e-12 {
            continue;
        }
        let delta = value - points[0];
        let u = delta.perp_dot(b) / area;
        let v = a.perp_dot(delta) / area;
        if u >= 0.0 && v >= 0.0 && u + v <= 1.0 {
            let mut weights = [0.0; 4];
            weights[0] = 1.0 - u - v;
            weights[i] = u;
            weights[i + 1] = v;
            return (0.0, weights);
        }
    }
    let mut best = (f64::INFINITY, [0.0; 4]);
    for i in 0..points.len() {
        let j = (i + 1) % points.len();
        let segment = points[j] - points[i];
        let length = segment.length_squared();
        let t = if length > 0.0 {
            ((value - points[i]).dot(segment) / length).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let distance = value.distance_squared(points[i] + segment * t);
        if distance < best.0 {
            let mut weights = [0.0; 4];
            weights[i] = 1.0 - t;
            weights[j] = t;
            best = (distance, weights);
        }
    }
    best
}
