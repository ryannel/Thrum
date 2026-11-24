use crate::Bed;

// Follow the rising bed (or its reverse on the dry side) to the sampled
// stage's still-water contour. Offsets are relative to each bed node.
pub(super) fn prepare(
    bed: &Bed,
    open: &[bool],
    joins: &[bool],
    level: f64,
    margin: f64,
) -> Vec<[f64; 3]> {
    let eligible = |i: usize| open[i] && !joins.get(i).copied().unwrap_or(false);
    bed.heights
        .iter()
        .enumerate()
        .map(|(i, height)| {
            if !eligible(i) || (level - height).abs() > 4.0 * margin {
                return [0.0; 3];
            }
            let start = bed.point(i);
            let mut point = start;
            for _ in 0..32 {
                let Some(neighbourhood) = bed.weights(point) else {
                    break;
                };
                if !neighbourhood.iter().any(|&(i, w)| eligible(i) && w > 0.0) {
                    break;
                }
                let height: f64 = neighbourhood.iter().map(|&(i, w)| bed.heights[i] * w).sum();
                let error = level - height;
                let slope = slope(bed, point, height);
                let magnitude = slope[0].hypot(slope[1]);
                if magnitude < 1e-6 {
                    break;
                }
                if error.abs() < 1e-6 {
                    return [point[0] - start[0], point[1] - start[1], 1.0];
                }
                let step = (error / magnitude).clamp(-bed.spacing, bed.spacing);
                for axis in 0..2 {
                    point[axis] += step * slope[axis] / magnitude;
                }
            }
            [0.0; 3]
        })
        .collect()
}

// Sample on both sides of the point to avoid steering along a cell boundary.
fn slope(bed: &Bed, point: [f64; 2], height: f64) -> [f64; 2] {
    let step = 0.5 * bed.spacing;
    std::array::from_fn(|axis| {
        let mut lo = point;
        let mut hi = point;
        lo[axis] -= step;
        hi[axis] += step;
        let a = bed.height(lo);
        let b = bed.height(hi);
        let width = step * (f64::from(a.is_some()) + f64::from(b.is_some()));
        (b.unwrap_or(height) - a.unwrap_or(height)) / width.max(step)
    })
}
