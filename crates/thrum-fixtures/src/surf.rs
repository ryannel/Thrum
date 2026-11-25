//! Shared CPU and Metal fixture for a sheet with many long waves.
use std::f64::consts::TAU;

/// Number of nonzero, distinct components.
pub const COMPONENTS: u32 = 64;
/// World origin of the bed lattice.
pub const ORIGIN: [f64; 2] = [-110.0, -30.0];
/// Metres between bed samples.
pub const SPACING: f64 = 2.0;
/// Bed dimensions in x/z.
pub const SIZE: [u32; 2] = [71, 31];
/// Orbital displacement coefficient.
pub const CHOP: f64 = 0.25;
/// Wave surface time in sixtieths of a second.
pub const USE_TICK: u32 = 2;

/// Bed rises through the still-water level at the origin of the beach slope.
pub fn bed(x: u32) -> f64 {
    ((ORIGIN[0] + f64::from(x) * SPACING) * 0.15).clamp(-14.0, 2.0)
}

/// Wavenumber vector, amplitude and phase of one component.
pub fn component(i: u32) -> ([f64; 2], f64, f64) {
    let u = f64::from(i) / f64::from(COMPONENTS - 1);
    let angle = (u - 0.5) * 0.16;
    let k = TAU / (25.0 + 3.0 * u);
    let axis = [angle.cos() * k, angle.sin() * k];
    (axis, 2.3 / f64::from(COMPONENTS), 0.3 + 0.4 * u)
}

/// Return centres of overturned triangles on the cut-cell lattice.
pub fn fold_columns(forward: impl Fn([f64; 2]) -> [f64; 2], count: usize) -> Vec<[f64; 2]> {
    let mut columns = Vec::new();
    for z in (-68..=68).step_by(4) {
        for x in -200..=-16 {
            let p = [f64::from(x) * 0.25, f64::from(z) * 0.25];
            let [a, b, c] = [p, [p[0] + 0.25, p[1]], [p[0] + 0.25, p[1] + 0.25]].map(&forward);
            let det = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
            if det < -1e-5 {
                // Both guards compare the same representable GPU points.
                #[expect(clippy::cast_possible_truncation, reason = "share GPU coordinates")]
                let column =
                    std::array::from_fn(|i| f64::from(((a[i] + b[i] + c[i]) / 3.0) as f32));
                columns.push(column);
                if columns.len() == count {
                    return columns;
                }
            }
        }
    }
    columns
}
