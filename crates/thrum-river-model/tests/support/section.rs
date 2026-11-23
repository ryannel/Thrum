//! A channel one cell wide between walls: a spring in its first cell, a receiving pool after its
//! last. Channel cell `k` (from 1) has its centre at x = (k - 0.5) dx.
use thrum_river_model::{Declaration, Grid, Manning, Profile, Receiver, Spring};

pub(crate) const TAIL: usize = 3;

/// A section with `bed` along the channel, `unit` m²/s from the spring, and a pool of bed
/// `tail_bed` held at `level` after the last cell.
pub(crate) fn section(
    bed: &[f64],
    dx: f64,
    unit: f64,
    tail_bed: f64,
    level: f64,
    n: f64,
) -> (Grid, Declaration) {
    let len = bed.len();
    let nx = 1 + len + TAIL;
    let wall = bed.iter().copied().fold(level, f64::max) + 5.0;
    let mut heights = vec![wall; nx * 3];
    for (k, &z) in bed.iter().enumerate() {
        heights[nx + 1 + k] = z;
    }
    for k in 0..TAIL {
        heights[nx + 1 + len + k] = tail_bed;
    }
    let grid = Grid::new([-0.5 * dx, -dx], dx, [nx, 3], heights).expect("section grid");
    let end = len as f64 * dx;
    let declaration = Declaration {
        name: "section".into(),
        springs: vec![Spring {
            name: "spring".into(),
            point: [0.5 * dx, 0.0],
            discharge: unit * dx,
            radius: 0.0,
        }],
        sources: vec![],
        receivers: vec![Receiver {
            name: "pool".into(),
            seed: [end + 1.5 * dx, 0.0],
            extent: [[end, -dx], [end + 10.0 * dx, dx]],
            level,
        }],
        manning: Manning::Uniform(n),
    };
    (grid, declaration)
}

/// Depth along the channel's cells.
pub(crate) fn depths(solution: &thrum_river_model::Solution, len: usize) -> Vec<f64> {
    let nx = solution.grid().size()[0];
    (0..len).map(|k| solution.state.depth[nx + 1 + k]).collect()
}

pub(crate) fn profile() -> Profile {
    Profile {
        level_rate: 1e-7,
        imbalance: 1e-4,
        ..Profile::default()
    }
}

/// A short frictional reach with the usual test discharge and a held pool at its mouth.
pub(crate) fn short_reach() -> (Grid, Declaration) {
    let dx = 0.5;
    let bed: Vec<_> = (0..16).map(|i| -0.01 * (f64::from(i) + 0.5) * dx).collect();
    section(&bed, dx, super::valley::DISCHARGE / dx, -1.0, 0.35, 0.04)
}
