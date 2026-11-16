//! A channel that falls, made by these tests and named nowhere else. It runs in +x between
//! walls: a spring at its head, an upper reach on a steady slope to a lip, a cliff, and below it
//! either a pool held at a level or gentle ground that runs on to a lake. A rim of walls `RIM`
//! cells thick lies round it, so a chart's banks stay inside the bed.
use std::ops::Range;
use thrum_river_model::{Declaration, Grid, Manning, Receiver, Spring};

/// The lip's bed, metres.
pub(crate) const LIP: f64 = 10.0;
/// Cells across the channel.
pub(crate) const WIDTH: usize = 6;
/// Cells of wall round the channel.
pub(crate) const RIM: usize = 3;

/// The cliff below the lip.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Cliff {
    /// From the lip's bed to the foot's, metres.
    pub(crate) height: f64,
    /// Cells on its face between the lip and the foot; 0 is a vertical cliff.
    pub(crate) face_cells: usize,
}

/// What lies below the cliff.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Foot {
    /// A pool, `cells` long, whose floor is the foot and whose water stands `depth` over it.
    Pool { cells: usize, depth: f64 },
    /// Ground falling at `slope`, `cells` long, and then a lake.
    Ground { slope: f64, cells: usize },
}

/// A fall's channel.
#[derive(Clone, Debug)]
pub(crate) struct Channel {
    pub(crate) grid: Grid,
    pub(crate) declaration: Declaration,
    /// Column of the lip.
    pub(crate) lip: usize,
    /// First column below the face: the foot.
    pub(crate) foot: usize,
    /// The level of the water below the fall, metres: the pool's or the lake's.
    pub(crate) level: f64,
    /// The receiving body's box.
    pub(crate) basin: [[f64; 2]; 2],
}

impl Channel {
    /// The grid's cell at a column and a row.
    pub(crate) fn cell(&self, x: usize, z: usize) -> usize {
        z * self.grid.size()[0] + x
    }
    /// The channel's rows.
    pub(crate) fn rows(&self) -> Range<usize> {
        RIM..self.grid.size()[1] - RIM
    }
    /// The channel's columns, from its head to the end of what lies below the cliff.
    pub(crate) fn columns(&self) -> Range<usize> {
        RIM..self.grid.size()[0] - RIM
    }
}

/// A channel `upper` cells long at `slope` above the lip, carrying `unit` m²/s for each metre
/// of its width, with Manning's `n`.
pub(crate) fn channel(
    dx: f64,
    upper: usize,
    slope: f64,
    unit: f64,
    n: f64,
    cliff: Cliff,
    foot: Foot,
) -> Channel {
    let lip = RIM - 1 + upper;
    let first = lip + 1 + cliff.face_cells;
    let low = LIP - cliff.height;
    let face_slope = cliff.height / (cliff.face_cells + 1) as f64;
    let (below, level, lake) = match foot {
        Foot::Pool { cells, depth } => (cells, low + depth, 0),
        Foot::Ground { cells, .. } => (cells, 0.0, 8),
    };
    let nx = first + below + lake + RIM;
    let nz = WIDTH + 2 * RIM;
    let top = LIP + slope * upper as f64 * dx + 5.0;
    let mut heights = vec![top; nx * nz];
    let mut last_ground = low;
    for x in RIM..nx - RIM {
        let h = if x <= lip {
            LIP + slope * (lip - x) as f64 * dx
        } else if x < first {
            LIP - face_slope * (x - lip) as f64
        } else if x < first + below {
            match foot {
                Foot::Pool { .. } => low,
                Foot::Ground { slope, .. } => {
                    last_ground = low - slope * (x - first) as f64 * dx;
                    last_ground
                }
            }
        } else {
            // The lake's floor, a step under the least fall below the ground's end.
            last_ground - 0.6
        };
        for z in RIM..RIM + WIDTH {
            heights[z * nx + x] = h;
        }
    }
    let (level, start) = match foot {
        Foot::Pool { .. } => (level, first),
        Foot::Ground { .. } => (last_ground - 0.2, first + below),
    };
    let grid = Grid::new([0.0, 0.0], dx, [nx, nz], heights).expect("channel grid");
    let middle = (RIM as f64 + 0.5 * (WIDTH as f64 - 1.0)) * dx;
    let basin = [
        [(start as f64 - 0.5) * dx, (RIM as f64 - 0.5) * dx],
        [
            ((nx - RIM) as f64 - 0.5) * dx,
            ((RIM + WIDTH) as f64 - 0.5) * dx,
        ],
    ];
    let declaration = Declaration {
        name: "channel".into(),
        springs: vec![Spring {
            name: "spring".into(),
            point: [(RIM + 2) as f64 * dx, middle],
            discharge: unit * WIDTH as f64 * dx,
            radius: 0.5 * (WIDTH as f64 - 0.8) * dx,
        }],
        sources: vec![],
        receivers: vec![Receiver {
            name: "pool".into(),
            seed: [f64::midpoint(basin[0][0], basin[1][0]), middle],
            extent: basin,
            level,
        }],
        manning: Manning::Uniform(n),
    };
    Channel {
        grid,
        declaration,
        lip,
        foot: first,
        level,
        basin,
    }
}

/// Critical depth for `q` m²/s for each metre, metres.
pub(crate) fn critical(q: f64) -> f64 {
    (q * q / 9.81).cbrt()
}

/// Manning's normal depth for `q` m²/s for each metre on a slope, metres.
pub(crate) fn normal(q: f64, n: f64, slope: f64) -> f64 {
    (q * n / slope.sqrt()).powf(0.6)
}

/// Discharge through the x-faces left of column `x` across the channel, m³/s.
pub(crate) fn through(s: &thrum_river_model::Solution, x: usize) -> f64 {
    let nx = s.grid().size()[0];
    (RIM..RIM + WIDTH).map(|z| s.x_flux[z * (nx + 1) + x]).sum()
}
