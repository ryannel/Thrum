//! The static part of a body's own pressure, used only by its query.

use super::{BodyPressure, CELL_METRES, DENSITY, GRAVITY, GRID_WIDTH};
use std::f64::consts::TAU;

pub(super) const DIVIDER_METRES: f64 = 12.0;
const BAND_START: f64 = 0.8 / DIVIDER_METRES;
const BAND_END: f64 = 1.2 / DIVIDER_METRES;
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the maximum Fourier bin is the integer part of the positive coarse-band edge"
)]
const MAX_BIN: usize = (GRID_WIDTH as f64 * CELL_METRES[1] as f64 * BAND_END) as usize;
const FINE: u32 = 1;
const COARSE: u32 = 2;

#[expect(
    clippy::cast_possible_truncation,
    reason = "the query uploads height and slope in the GPU float format"
)]
pub(super) fn at(
    source: BodyPressure,
    admitted: u32,
    point: [f64; 2],
    origins: [[i32; 2]; 2],
) -> [f32; 4] {
    if admitted == 0 || source.force == 0.0 {
        return [0.0; 4];
    }
    let dent = match admitted {
        FINE => {
            let Some(long) = long_band(source, point, origins[0], 0) else {
                return [0.0; 4];
            };
            // The short band is the Gaussian less its long-band complement
            // on the fine grid, including the DC component the step omits.
            let full = gaussian(source, point);
            std::array::from_fn(|i| full[i] - long[i])
        }
        COARSE => long_band(source, point, origins[1], 1).unwrap_or([0.0; 3]),
        _ => gaussian(source, point),
    };
    [dent[0] as f32, dent[1] as f32, dent[2] as f32, 0.0]
}

fn gaussian(source: BodyPressure, point: [f64; 2]) -> [f64; 3] {
    let [x, z] = std::array::from_fn(|i| point[i] - source.place[i]);
    let (sin, cos) = source.heading.sin_cos();
    let [along, across] = source.footprint;
    let u = (cos * x + sin * z) / along;
    let v = (-sin * x + cos * z) / across;
    let h = source.force * (-0.5 * u.mul_add(u, v * v)).exp()
        / (TAU * along * across * f64::from(DENSITY * GRAVITY));
    [
        h,
        -h * (u * cos / along - v * sin / across),
        -h * (u * sin / along + v * cos / across),
    ]
}

// Invert just the long band of the static pressure spectrum. Conjugate modes
// are paired, and the phasors and derivative multipliers depend on only one
// axis. This runs only for a query by a partly admitted source.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the band edge is positive and bounds the fixed phasor table"
)]
fn long_band(
    source: BodyPressure,
    point: [f64; 2],
    origin: [i32; 2],
    level: usize,
) -> Option<[f64; 3]> {
    let metres = f64::from(CELL_METRES[level]);
    let side = f64::from(GRID_WIDTH);
    let span = side * metres;
    let grid = std::array::from_fn::<_, 2, _>(|i| point[i] / metres - f64::from(origin[i]));
    if grid.iter().any(|&p| !(0.0..side - 1.0).contains(&p)) {
        return None;
    }
    let edge = span * BAND_END;
    let edge_squared = edge * edge;
    let limit = edge as u32;
    let mut phase = [[[0.0; 3]; MAX_BIN + 1]; 2];
    for (axis, phases) in phase.iter_mut().enumerate() {
        let cell = grid[axis].floor();
        let weight = grid[axis] - cell;
        let offset = cell + f64::from(origin[axis]) - source.place[axis] / metres;
        for bin in 0..=limit {
            let k = TAU * f64::from(bin) / side;
            let (lo_sin, lo_cos) = (k * offset).sin_cos();
            let (hi_sin, hi_cos) = (k * (offset + 1.0)).sin_cos();
            phases[bin as usize] = [
                (1.0 - weight) * lo_cos + weight * hi_cos,
                (1.0 - weight) * lo_sin + weight * hi_sin,
                k.sin() / metres,
            ];
        }
    }
    let (sin, cos) = source.heading.sin_cos();
    let [along, across] = source.footprint;
    let limit = i32::try_from(limit).expect("band lies within the grid");
    let mut sum = [if level == 0 { 1.0 } else { 0.0 }, 0.0, 0.0];
    for x in 0..=limit {
        let [cx, sx, dx] = phase[0][x as usize];
        for z in if x == 0 { 1 } else { -limit }..=limit {
            let radius_squared = f64::from(x * x + z * z);
            if radius_squared >= edge_squared {
                continue;
            }
            let cycles_per_metre = radius_squared.sqrt() / span;
            let t = ((cycles_per_metre - BAND_START) / (BAND_END - BAND_START)).clamp(0.0, 1.0);
            let band = 1.0 - t * t * (3.0 - 2.0 * t);
            let kx = TAU * f64::from(x) / span;
            let kz = TAU * f64::from(z) / span;
            let u = (cos * kx + sin * kz) * along;
            let v = (-sin * kx + cos * kz) * across;
            let amplitude = 2.0 * band * (-0.5 * u.mul_add(u, v * v)).exp();
            let [cz, mut sz, mut dz] = phase[1][z.unsigned_abs() as usize];
            if z < 0 {
                sz = -sz;
                dz = -dz;
            }
            let sine = sx * cz + cx * sz;
            sum[0] += amplitude * (cx * cz - sx * sz);
            sum[1] -= amplitude * sine * dx;
            sum[2] -= amplitude * sine * dz;
        }
    }
    let scale = source.force / (f64::from(DENSITY * GRAVITY) * span * span);
    Some(sum.map(|v| v * scale))
}
