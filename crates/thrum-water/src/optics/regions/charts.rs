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

    fn include_mix(&mut self, other: Self, weight: f64) {
        let mix = |a: f64, b: f64| (b - a).mul_add(weight, a);
        self.include(Self {
            level: [
                mix(self.level[0], other.level[0]),
                mix(self.level[1], other.level[1]),
            ],
            depth: mix(self.depth, other.depth),
            gain: mix(self.gain, other.gain),
        });
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
        let [left, front] = bed.point(z * width + x);
        let [right, back] = bed.point(last_z * width + last_x);
        let center = [left.midpoint(right), front.midpoint(back)];
        let radius = 0.5 * (right - left).hypot(back - front);

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
            let mut mixed = own;
            let mut full_joins = RowBounds::EMPTY;
            for end in chart.ends.iter().filter(|end| !end.is_brink()) {
                let other = rows[end.body];
                let [least, most] =
                    join_weights(end, center, radius, own, other, map.profile().margin_height);
                if least >= 1.0 {
                    // A full join returns immediately. Earlier ends could
                    // already have returned at other points in this tile.
                    mixed = other;
                    break;
                }
                if most >= 1.0 {
                    full_joins.include(other);
                }
                // Normalising an end against earlier ends can only reduce
                // its weight in a partial mix.
                mixed.include_mix(other, most);
            }
            mixed.include(full_joins);
            rows[body_id] = mixed;
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

fn join_weights(
    end: &End,
    center: [f64; 2],
    radius: f64,
    own: RowBounds,
    other: RowBounds,
    margin: f64,
) -> [f64; 2] {
    let [[x0, z0], [x1, z1]] = end.segment;
    let [dx, dz] = [x1 - x0, z1 - z0];
    let [px, pz] = [center[0] - x0, center[1] - z0];
    let nearest = ((px * dx + pz * dz) / (dx * dx + dz * dz)).clamp(0.0, 1.0);
    let distance = (px - nearest * dx).hypot(pz - nearest * dz);
    let falloff = |t: f64| {
        let t = t.clamp(0.0, 1.0);
        1.0 - t * t * (3.0 - 2.0 * t)
    };
    let mut least = falloff((distance + radius) / end.length);
    let mut most = falloff((distance - radius) / end.length);
    if end.mouth {
        // Match the map's backwater reach. A mouth can contribute beyond
        // its geometric join length where the receiving level rises.
        let reach = 0.175 * margin;
        least = least.max(falloff((own.level[1] - other.level[0] - reach) / 0.25));
        most = most.max(falloff((own.level[0] - other.level[1] - reach) / 0.25));
    }
    [least, most]
}
