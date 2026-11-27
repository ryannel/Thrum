//! Test cliffs and probes shared by the CPU and Metal map checks.

/// Height of the receiving body’s shore margin.
pub const MARGIN: f64 = 0.65;
/// The x coordinate of the brink row.
pub const LIP: u32 = 2;
/// The centreline z coordinate.
pub const CENTRE: f64 = 3.0;
/// The upper reach’s own water level.
pub const TOP: f64 = 4.1;
/// The receiver’s level at zero stage.
pub const LEVEL: f64 = 0.2;
/// The receiver’s maximum stage.
pub const RISE: f64 = 0.25;
/// Dimensions of the test bed.
pub const SIZE: [usize; 2] = [14, 7];
const BOTTOM: f64 = -0.7;

/// Quarter-cell stations from lip to foot.
pub fn points(width: u32) -> impl Iterator<Item = [f64; 2]> {
    (0..=4 * width).map(|quarter| [f64::from(LIP) + f64::from(quarter) * 0.25, CENTRE])
}

/// Test points just off either end of the narrow brink.
pub fn flanks() -> [[f64; 2]; 2] {
    [[2.953_125, 1.953_125], [2.953_125, 4.046_875]]
}

/// Analytic centreline bed height at a test station.
pub fn rock(width: u32, x: f64) -> f64 {
    let heights = ramp(width, 4.0);
    let offset = x - f64::from(LIP);
    for (d, endpoints) in (0..width).zip(heights.windows(2)) {
        let t = offset - f64::from(d);
        if t <= 1.0 {
            return (endpoints[1] - endpoints[0]).mul_add(t, endpoints[0]);
        }
    }
    BOTTOM
}

fn ramp(width: u32, lip: f64) -> Vec<f64> {
    match width {
        1 => vec![lip, BOTTOM],
        4 => vec![lip, 2.6, 1.6, 0.9, BOTTOM],
        _ => unreachable!("fixture cliff widths"),
    }
}

/// Lattice heights for a given cliff width and lip height.
pub fn bed_heights(width: u32, lip: f64) -> Vec<f64> {
    let heights = ramp(width, lip);
    cells()
        .map(|(x, z)| {
            if x == 0 || x > 10 || !(2..=4).contains(&z) {
                10.0
            } else if x <= LIP {
                lip
            } else {
                heights.get((x - LIP) as usize).copied().unwrap_or(BOTTOM)
            }
        })
        .collect()
}

fn cells() -> impl Iterator<Item = (u32, u32)> {
    (0..u32::try_from(SIZE[1]).expect("rows"))
        .flat_map(|z| (0..u32::try_from(SIZE[0]).expect("columns")).map(move |x| (x, z)))
}
