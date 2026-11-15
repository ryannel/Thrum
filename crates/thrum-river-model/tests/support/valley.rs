//! The test valley, made by this test and named nowhere else: a spring at its head, a riffle,
//! a pool behind a sill with a notch 2 m wide, a chute, a ledge of 0.4 m, a run with a short
//! narrows and a rock, and a lake held in a box at its foot. A hollow lies beside the pool,
//! lower than the pool's water and cut off from it by ground. x runs down the valley; its
//! centre line is z = 7 m; the rectangle is 48 m by 14 m.
use thrum_river_model::{Declaration, Grid, Manning, Receiver, Spring};

pub(crate) const CENTRE: f64 = 7.0;
pub(crate) const DISCHARGE: f64 = 0.3;
pub(crate) const NOTCH: f64 = 2.0;
pub(crate) const CREST: f64 = 2.80;
pub(crate) const LAKE: f64 = 2.195;
pub(crate) const HOLLOW: [f64; 2] = [14.0, CENTRE + 4.5];
pub(crate) const ROCK: [f64; 2] = [35.0, CENTRE + 0.2];
pub(crate) const LEDGE: f64 = 24.0;
pub(crate) const NARROWS: [f64; 2] = [29.5, 30.0];
pub(crate) const MANNING: f64 = 0.04;

fn smooth(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn window(x: f64, a: f64, b: f64, ramp: f64) -> f64 {
    smooth((x - a) / ramp) * (1.0 - smooth((x - b) / ramp))
}

/// Half the channel's width.
pub(crate) fn half_width(x: f64) -> f64 {
    1.5 + window(x, 10.0, 17.0, 1.5)
        - 0.5 * window(x, 17.5, 20.5, 1.5)
        - 0.875 * window(x, 28.5, 30.0, 1.0)
}

/// The channel's floor on its centre line.
pub(crate) fn floor(x: f64) -> f64 {
    let s = std::f64::consts::PI * (x - 10.0) / 9.0;
    if x < 10.0 {
        3.0 - 0.015 * (x - 1.5)
    } else if x < 19.0 {
        2.8725 + (CREST - 2.8725) * (x - 10.0) / 9.0 - 0.6 * s.sin().powi(2)
    } else if x < 20.5 {
        CREST
    } else if x < LEDGE {
        CREST - 0.08 * (x - 20.5)
    } else if x < LEDGE + 0.5 {
        2.52 - 0.8 * (x - LEDGE)
    } else {
        2.12 - 0.01 * (x - LEDGE - 0.5)
    }
}

pub(crate) fn plateau(x: f64) -> f64 {
    4.2 - 0.015 * x
}

/// The bed at a world point.
pub(crate) fn bed(x: f64, z: f64) -> f64 {
    let off = (z - CENTRE).abs();
    if x >= 42.0 {
        return if off < 6.0 && x < 47.5 { 1.0 } else { 5.0 };
    }
    let top = plateau(x);
    if x < 1.5 {
        return top;
    }
    let d = off - half_width(x);
    let mut h = floor(x) + (6.0 * d).clamp(0.0, top - floor(x));
    h += 0.5 * (-((x - ROCK[0]).powi(2) + (z - ROCK[1]).powi(2)) / 0.35).exp();
    let r = (x - HOLLOW[0]).hypot(z - HOLLOW[1]);
    if r < 1.0 {
        h = h.min(2.7 + 0.5 * r * r);
    }
    h
}

/// The valley at `dx` cells over z from `z0` to `z1` (0 and 14 for its own rectangle).
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "cell counts of a positive extent"
)]
pub(crate) fn valley_in(dx: f64, z0: f64, z1: f64) -> (Grid, Declaration) {
    let nx = (48.0 / dx).round() as usize;
    let nz = ((z1 - z0) / dx).round() as usize;
    let origin = [0.5 * dx, z0 + 0.5 * dx];
    let heights = (0..nx * nz)
        .map(|i| {
            bed(
                origin[0] + (i % nx) as f64 * dx,
                origin[1] + (i / nx) as f64 * dx,
            )
        })
        .collect();
    let grid = Grid::new(origin, dx, [nx, nz], heights).expect("valley grid");
    let declaration = Declaration {
        name: "valley".into(),
        springs: vec![Spring {
            name: "spring".into(),
            point: [2.5, CENTRE],
            discharge: DISCHARGE,
            radius: 0.75,
        }],
        sources: vec![],
        receivers: vec![Receiver {
            name: "lake".into(),
            seed: [45.0, CENTRE],
            extent: [[42.0, z0], [48.0, z1]],
            level: LAKE,
        }],
        manning: Manning::Uniform(MANNING),
    };
    (grid, declaration)
}

pub(crate) fn valley(dx: f64) -> (Grid, Declaration) {
    valley_in(dx, 0.0, 14.0)
}
