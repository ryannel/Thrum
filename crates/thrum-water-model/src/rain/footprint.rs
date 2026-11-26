//! Convolve the table's piecewise-linear radial rings with a station's disc.
//! Age interpolation commutes with the convolution, so it stays a lookup.
use super::{RainBands, RingTable, accumulate};
use crate::{Map, MapSample};
use std::f64::consts::{PI, TAU};
use thrum_sea_model::Sample;

// Fractions of a ring's reach. Include the yard's float and craft footprints.
const RADII: [f64; 14] = [
    0.0625, 0.125, 0.1875, 0.25, 0.3125, 0.4, 0.5, 0.625, 0.75, 0.86875, 1.0, 1.25, 1.5625, 2.0,
];

type Ring = [f64; 3];

pub(super) struct Footprints {
    means: Vec<Vec<Ring>>,
    rungs: Vec<Rung>,
}

impl Footprints {
    pub(super) fn new(table: &RingTable) -> Self {
        let means = convolve(table, &kernel(table, 0.0, table.reach(), 1.0));
        let rungs = RADII
            .map(|fraction| Rung::new(table, fraction * table.reach()))
            .into();
        Self { means, rungs }
    }
}

struct Rung {
    radius: f64,
    spacing: f64,
    classes: Vec<Vec<Ring>>,
    distances: usize,
}

impl Rung {
    fn new(table: &RingTable, radius: f64) -> Self {
        let support = table.reach() + radius;
        let intervals =
            grid_index((support / (radius / 36.0).max(table.radius_step * 0.5)).ceil()).max(1);
        let spacing = support / intervals as f64;
        let distances = intervals + 1;
        let mut classes = vec![Vec::with_capacity(distances * table.ages); table.classes.len()];
        for i in 0..intervals {
            let weights = kernel(
                table,
                i as f64 * spacing,
                radius,
                1.0 / (PI * radius * radius),
            );
            for (class, samples) in classes.iter_mut().zip(convolve(table, &weights)) {
                class.extend(samples);
            }
        }
        for class in &mut classes {
            class.resize(distances * table.ages, [0.0; 3]);
        }
        Self {
            radius,
            spacing,
            classes,
            distances,
        }
    }

    fn sample(&self, drop: &LiveDrop, distance: f64, ages: usize) -> Ring {
        if distance >= (self.distances - 1) as f64 * self.spacing {
            return [0.0; 3];
        }
        let f = distance / self.spacing;
        let index = grid_index(f).min(self.distances - 2);
        let a = index * ages + drop.age;
        let samples = &self.classes[drop.class];
        lerp(
            lerp(samples[a], samples[a + 1], drop.fraction),
            lerp(samples[a + ages], samples[a + ages + 1], drop.fraction),
            f - index as f64,
        )
    }
}

fn lerp(a: Ring, b: Ring, t: f64) -> Ring {
    std::array::from_fn(|i| (b[i] - a[i]).mul_add(t, a[i]))
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "nonnegative table or lattice coordinate"
)]
fn grid_index(f: f64) -> usize {
    f as usize
}

/// Angular measure of the disc on a ring of radius r about the drop.
/// For slope, the first angular moment projects the ring's slope onto the
/// axis from the drop to the station. The perpendicular moment is zero.
fn kernel(table: &RingTable, distance: f64, radius: f64, normalization: f64) -> Vec<[f64; 2]> {
    let mut weights = vec![[0.0; 2]; table.radii];
    let spacing = table.radius_step;
    let inner = (distance - radius).abs();
    let outer = distance + radius;
    for i in 0..table.radii - 1 {
        let lo = i as f64 * spacing;
        let hi = lo + spacing;
        if distance < radius {
            let end = hi.min(inner);
            if end > lo {
                let width = end - lo;
                let mass = width * (lo + 0.5 * width) * TAU * normalization;
                let right =
                    width * width * (0.5 * lo + width / 3.0) * TAU * normalization / spacing;
                weights[i][0] += mass - right;
                weights[i + 1][0] += right;
            }
        }
        let start = lo.max(inner);
        let end = hi.min(outer);
        if start >= end || distance == 0.0 {
            continue;
        }
        // Four-node Gauss rule on each linear table segment, splitting at
        // both angular endpoints. Only these radial weights need trig.
        let half = (end - start) * 0.5;
        let centre = end.midpoint(start);
        for (node, weight) in [
            (-0.861_136_311_594_052_6, 0.347_854_845_137_453_8),
            (-0.339_981_043_584_856_3, 0.652_145_154_862_546_1),
            (0.339_981_043_584_856_3, 0.652_145_154_862_546_1),
            (0.861_136_311_594_052_6, 0.347_854_845_137_453_8),
        ] {
            let r = node * half + centre;
            let cos = ((r * r + distance * distance - radius * radius) / (2.0 * r * distance))
                .clamp(-1.0, 1.0);
            let mass = 2.0 * r * half * weight * normalization;
            let value = [mass * cos.acos(), mass * (1.0 - cos * cos).sqrt()];
            let fraction = (r - lo) / spacing;
            for (component, contribution) in value.into_iter().enumerate() {
                weights[i][component] += contribution * (1.0 - fraction);
                weights[i + 1][component] += contribution * fraction;
            }
        }
    }
    weights
}

fn convolve(table: &RingTable, weights: &[[f64; 2]]) -> Vec<Vec<Ring>> {
    let first = weights.iter().position(|w| w[0] > 0.0).unwrap_or(0);
    let last = weights
        .iter()
        .rposition(|w| w[0] > 0.0)
        .map_or(first, |i| i + 1);
    table
        .classes
        .iter()
        .map(|class| {
            (0..table.ages)
                .map(|age| {
                    let start = age * table.radii;
                    let mut ring = [0.0; 3];
                    for (i, &[mean, slope]) in
                        (start + first..start + last).zip(&weights[first..last])
                    {
                        ring[0] += mean * class.height[i];
                        ring[1] += slope * class.slope[i];
                        ring[2] += mean * class.rate[i];
                    }
                    ring
                })
                .collect()
        })
        .collect()
}

struct LiveDrop {
    place: [f64; 2],
    class: usize,
    age: usize,
    fraction: f64,
    weight: f64,
}

struct Band {
    period: f64,
    cells: usize,
    bins: Vec<Vec<LiveDrop>>,
    mean: Sample,
}

impl Band {
    fn new(rain: &RainBands, footprints: &Footprints, band: usize, time: f64) -> Self {
        let period = rain.profile.patches[band];
        let cells = grid_index(period / rain.table.reach()).max(1);
        let mut bins: Vec<Vec<_>> = (0..cells * cells).map(|_| Vec::new()).collect();
        let mut mean = Sample::default();
        let area = period * period;
        for drop in rain
            .drops(band, time)
            .into_iter()
            .filter(|drop| drop.weight > 0.0)
        {
            let f = (time - drop.time) / rain.table.age_step;
            let age = grid_index(f).min(rain.table.ages - 2);
            let fraction = f - age as f64;
            let row = &footprints.means[drop.class];
            let v = lerp(row[age], row[age + 1], fraction);
            mean.height += drop.weight * v[0] / area;
            mean.height_dt += drop.weight * v[2] / area;
            let [x, z] = drop
                .place
                .map(|p| grid_index(p * cells as f64 / period).min(cells - 1));
            bins[z * cells + x].push(LiveDrop {
                place: drop.place,
                class: drop.class,
                age,
                fraction,
                weight: drop.weight,
            });
        }
        Self {
            period,
            cells,
            bins,
            mean,
        }
    }

    fn nearby(&self, point: [f64; 2], reach: f64, mut visit: impl FnMut(&LiveDrop, [f64; 2])) {
        let [x, z] = point.map(|v| v.rem_euclid(self.period));
        let scale = self.cells as f64 / self.period;
        // Offset by two periods so indexing both sides of the seam stays unsigned.
        let bounds = |v: f64| {
            grid_index((v - reach + 2.0 * self.period) * scale)
                ..=grid_index((v + reach + 2.0 * self.period) * scale)
        };
        for j in bounds(z) {
            for i in bounds(x) {
                let offset = [i, j].map(|v| ((v / self.cells) as f64 - 2.0) * self.period);
                for drop in &self.bins[(j % self.cells) * self.cells + i % self.cells] {
                    visit(
                        drop,
                        [
                            x - (drop.place[0] + offset[0]),
                            z - (drop.place[1] + offset[1]),
                        ],
                    );
                }
            }
        }
    }
}

/// The rain bands at one tick, shared by every station of a query batch.
pub struct RainQuery<'a> {
    rain: &'a RainBands,
    fields: Option<(&'a Footprints, [Band; 2])>,
    height: f64,
}

impl<'a> RainQuery<'a> {
    pub(super) fn new(rain: &'a RainBands, time: f64) -> Self {
        let fields = rain.active(time).then(|| {
            let footprints = rain.footprints();
            let bands = std::array::from_fn(|band| Band::new(rain, footprints, band, time));
            (footprints, bands)
        });
        let height = fields
            .as_ref()
            .map_or(0.0, |_| 4.0 * rain.variance(time).sqrt());
        Self {
            rain,
            fields,
            height,
        }
    }

    /// Shelter and depth gain at the station, with the tick's precomputed rain height.
    pub fn gain(&self, map: &Map, variances: [f64; 4], point: [f64; 2], column: MapSample) -> f64 {
        self.rain
            .gain(map, variances, point, column, || self.height)
    }

    /// Height, world-axis slopes and height rate at unit gain, before current carriage.
    pub fn sample(&self, point: [f64; 2], radius: f64) -> Sample {
        let Some((footprints, bands)) = &self.fields else {
            return Sample::default();
        };
        let table = &self.rain.table;
        let reach = table.reach();
        let radius = radius.max(0.0);
        let broad = (radius / reach - 1.0).clamp(0.0, 1.0);
        let rungs = &footprints.rungs;
        let upper = rungs
            .partition_point(|rung| rung.radius < radius)
            .min(rungs.len() - 1);
        let lower = upper.checked_sub(1).map(|index| &rungs[index]);
        let upper = &rungs[upper];
        let low_radius = lower.map_or(0.0, |rung| rung.radius);
        let fraction = (radius - low_radius) / (upper.radius - low_radius);
        let mut out = Sample::default();
        for band in bands {
            accumulate(&mut out, band.mean, broad);
            if broad >= 1.0 {
                continue;
            }
            let support = reach
                + if fraction == 0.0 {
                    low_radius
                } else {
                    upper.radius
                };
            band.nearby(point, support, |drop, offset| {
                let square = offset[0] * offset[0] + offset[1] * offset[1];
                if square >= support * support {
                    return;
                }
                let distance = square.sqrt();
                let read_lower = || {
                    lower.map_or_else(
                        || {
                            table.ring(
                                drop.class,
                                distance,
                                (drop.age as f64 + drop.fraction) * table.age_step,
                            )
                        },
                        |rung| rung.sample(drop, distance, table.ages),
                    )
                };
                let [h, dr, dt] = if fraction == 0.0 {
                    read_lower()
                } else if fraction >= 1.0 {
                    upper.sample(drop, distance, table.ages)
                } else {
                    lerp(
                        read_lower(),
                        upper.sample(drop, distance, table.ages),
                        fraction,
                    )
                };
                let gain = drop.weight * (1.0 - broad);
                out.height += gain * h;
                out.height_dt += gain * dt;
                if distance > 0.0 {
                    let gradient = gain * dr / distance;
                    out.slope[0] += gradient * offset[0];
                    out.slope[1] += gradient * offset[1];
                }
            });
        }
        out
    }
}
