use crate::map_gpu::BODY_LIMIT;
use thrum_water_model::{BodyKind, End, Map};

#[derive(Clone, Copy)]
pub(super) struct RowBounds {
    pub(super) level: [f64; 2],
    pub(super) depth: f64,
    pub(super) gain: f64,
}

impl RowBounds {
    const EMPTY: Self = Self {
        level: [f64::INFINITY, f64::NEG_INFINITY],
        depth: f64::INFINITY,
        gain: f64::INFINITY,
    };

    fn include(&mut self, other: Self) {
        self.level[0] = self.level[0].min(other.level[0]);
        self.level[1] = self.level[1].max(other.level[1]);
        self.depth = self.depth.min(other.depth);
        self.gain = self.gain.min(other.gain);
    }

    fn mix(self, other: Self, weight: f64) -> Self {
        let mix = |a: f64, b: f64| (b - a).mul_add(weight, a);
        Self {
            level: [
                mix(self.level[0], other.level[0]),
                mix(self.level[1], other.level[1]),
            ],
            depth: mix(self.depth, other.depth),
            gain: mix(self.gain, other.gain),
        }
    }
}

pub(super) struct ChartTile {
    pub(super) low: [usize; 2],
    pub(super) high: [usize; 2],
}

impl ChartTile {
    pub(super) fn row_bounds(&self, map: &Map, id: usize, bed_max: f64) -> RowBounds {
        let bed = map.bed();
        let (_, _, [width, _]) = bed.lattice();
        let [x, z] = self.low;
        let [last_x, last_z] = self.high;
        let indices = || (z..=last_z).flat_map(|j| (x..=last_x).map(move |i| j * width + i));
        let low = bed.point(z * width + x);
        let high = bed.point(last_z * width + last_x);

        // The shader reads the underlying chart row for a join, even where
        // that earlier chart does not own the point.
        let mut rows = [RowBounds::EMPTY; BODY_LIMIT];
        for body_id in 0..=id {
            let body = map.body(body_id).expect("prepared body");
            let gain = body.gains[3];
            let level = body.level + body.stage.value;
            let BodyKind::Chart(chart) = &body.kind else {
                rows[body_id] = RowBounds {
                    level: [level; 2],
                    depth: level - bed_max,
                    gain,
                };
                continue;
            };
            let mut own = RowBounds::EMPTY;
            for i in indices() {
                let level = chart.levels[i] + body.stage.value;
                // Level and bed are bilinear on the same lattice.
                own.include(RowBounds {
                    level: [level; 2],
                    depth: level - bed.heights()[i],
                    gain,
                });
            }
            let ends = chart.ends.iter().filter(|end| !end.is_brink()).map(|end| {
                let other = rows[end.body];
                let weights = join_weights(end, low, high, own, other, map.profile().margin_height);
                (other, weights)
            });
            rows[body_id] = bound_joins(own, ends);
        }

        let mut row = rows[id];
        let support = map.support_origins(id);
        for i in indices() {
            if let Some(origin) = support[i].filter(|&origin| origin != i) {
                // The bank continuation interpolates rows stored at the
                // wet origins, including origins outside this tile.
                let source = map.row(id, bed.point(origin));
                row.include(RowBounds {
                    level: [source.level; 2],
                    depth: source.level - bed_max,
                    gain: source.gains[3],
                });
            }
        }
        row
    }
}

fn bound_joins(own: RowBounds, ends: impl IntoIterator<Item = (RowBounds, [f64; 2])>) -> RowBounds {
    // A prepared chart has at most one source and one mouth.
    let mut partial = [(own, [0.0; 2]); 2];
    let mut count = 0;
    let mut bounds = RowBounds::EMPTY;
    for (other, [least, most]) in ends {
        if most >= 1.0 {
            bounds.include(other);
        }
        if least >= 1.0 {
            // A full join wins in declaration order, including at points
            // where an earlier end might already have taken over.
            return bounds;
        }
        if most > 0.0 {
            partial[count] = (other, [least, most]);
            count += 1;
        }
    }

    // Relative weights w/(1-w) preserve each end's full-join limit. In
    // that space a row component is a weighted average with own weight 1.
    // Holding every other ratio fixed makes it monotone in the remaining
    // ratio, so finite extrema occur at corners of the interval box.
    for corner in 0..(1 << count) {
        let mut row = own;
        let mut total = 1.0;
        let mut finite = true;
        for (index, &(other, weights)) in partial[..count].iter().enumerate() {
            let weight = weights[(corner >> index) & 1];
            if weight >= 1.0 {
                // Full outcomes and limits towards them are already in bounds.
                finite = false;
                break;
            }
            let relative = weight / (1.0 - weight);
            total += relative;
            row = row.mix(other, relative / total);
        }
        if finite {
            bounds.include(row);
        }
    }
    bounds
}

fn join_weights(
    end: &End,
    low: [f64; 2],
    high: [f64; 2],
    own: RowBounds,
    other: RowBounds,
    margin: f64,
) -> [f64; 2] {
    let [nearest, farthest] = segment_distances(end.segment, low, high);
    let falloff = |t: f64| {
        let t = t.clamp(0.0, 1.0);
        1.0 - t * t * (3.0 - 2.0 * t)
    };
    let mut least = falloff(farthest / end.length);
    let mut most = falloff(nearest / end.length);
    if end.mouth {
        // Match the map's backwater reach. A mouth can contribute beyond
        // its geometric join length where the receiving level rises.
        let reach = 0.175 * margin;
        least = least.max(falloff((own.level[1] - other.level[0] - reach) / 0.25));
        most = most.max(falloff((own.level[0] - other.level[1] - reach) / 0.25));
    }
    [least, most]
}

fn segment_distances(segment: [[f64; 2]; 2], low: [f64; 2], high: [f64; 2]) -> [f64; 2] {
    let [start, finish] = segment;
    let [dx, dz] = [finish[0] - start[0], finish[1] - start[1]];
    let corner_distance = |[x, z]: [f64; 2]| {
        let [px, pz] = [x - start[0], z - start[1]];
        let along = ((px * dx + pz * dz) / (dx * dx + dz * dz)).clamp(0.0, 1.0);
        (px - along * dx).hypot(pz - along * dz)
    };
    // Distance to a segment is convex, so its maximum on the rectangle
    // is at a corner. For disjoint shapes, the closest pair has an end
    // or a rectangle corner as one of its points.
    let distances = [low, [low[0], high[1]], [high[0], low[1]], high].map(corner_distance);
    let farthest = distances.into_iter().fold(0.0_f64, f64::max);
    let mut entry = 0.0_f64;
    let mut exit = 1.0_f64;
    for (axis, delta) in [dx, dz].into_iter().enumerate() {
        if delta != 0.0 {
            let a = (low[axis] - start[axis]) / delta;
            let b = (high[axis] - start[axis]) / delta;
            entry = entry.max(a.min(b));
            exit = exit.min(a.max(b));
        } else if !(low[axis]..=high[axis]).contains(&start[axis]) {
            entry = f64::INFINITY;
        }
    }
    if entry <= exit {
        return [0.0, farthest];
    }
    let end_distances = segment.map(|p| {
        let dx = p[0] - p[0].clamp(low[0], high[0]);
        let dz = p[1] - p[1].clamp(low[1], high[1]);
        dx.hypot(dz)
    });
    let nearest = distances
        .into_iter()
        .chain(end_distances)
        .fold(f64::INFINITY, f64::min);
    [nearest, farthest]
}

#[cfg(test)]
mod tests;
