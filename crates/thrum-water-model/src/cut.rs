use crate::WaterError;

/// Forward sheet evaluation at a material rest place, in metres.
#[derive(Clone, Copy, Debug)]
pub struct SheetPoint {
    /// Displaced world x/z.
    pub world: [f64; 2],
    /// Surface height.
    pub height: f64,
}

/// A continuous displaced sheet with a conservative, wave-independent reach.
pub trait Sheet {
    /// Evaluate the assembled forward map at a rest place.
    fn forward(&self, rest: [f64; 2]) -> SheetPoint;
    /// Ceiling on the distance of any rest place that can map onto this column.
    fn reach(&self, point: [f64; 2]) -> f64;
}

/// Highest-surface cut controls.
#[derive(Clone, Copy, Debug)]
pub struct CutConfig {
    /// World-fixed rest lattice spacing, metres.
    pub cell: f64,
    /// Maximum plan residual after Newton refinement, metres.
    pub tolerance: f64,
}

impl Default for CutConfig {
    fn default() -> Self {
        Self {
            cell: 0.25,
            tolerance: 0.001,
        }
    }
}

/// Highest triangle hit, refined on the original sheet where possible.
#[derive(Clone, Copy, Debug)]
pub struct Column {
    /// Surface height.
    pub height: f64,
    /// Rest place of the answer.
    pub rest: [f64; 2],
    /// Actual forward residual in the horizontal plane.
    pub residual: f64,
    /// Whether Newton refinement converged within its original cell.
    pub converged: bool,
    /// Number of lattice nodes evaluated (refinement evaluations excluded).
    pub nodes: usize,
    /// Number of triangles intersecting the column.
    pub hits: usize,
}

/// Forward tessellation and vertical cut, with no direction search or inversion
/// choosing a branch. Newton only refines the highest triangle's selected branch.
/// # Errors
/// Refuses invalid controls, nonfinite sheet nodes/reach or an uncovered column.
pub fn cut(sheet: &impl Sheet, point: [f64; 2], config: CutConfig) -> Result<Column, WaterError> {
    let r = sheet.reach(point);
    if point.iter().any(|v| !v.is_finite())
        || !r.is_finite()
        || r < 0.0
        || !config.cell.is_finite()
        || config.cell <= 0.0
        || !config.tolerance.is_finite()
        || config.tolerance <= 0.0
    {
        return Err(WaterError::new(
            "surface cut",
            "invalid cut controls or reach",
        ));
    }
    let cell = config.cell;
    let origin = point.map(|p| ((p - r) / cell).floor() * cell);
    let end = point.map(|p| ((p + r) / cell).ceil() * cell + cell);
    let extent = std::array::from_fn::<_, 2, _>(|a| ((end[a] - origin[a]) / cell).round() + 1.0);
    if extent
        .iter()
        .any(|v| !v.is_finite() || !(2.0..=16_000_000.0).contains(v))
    {
        return Err(WaterError::new(
            "surface cut",
            "cut exceeds model node capacity",
        ));
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "positive lattice dimensions, bounded below before allocation"
    )]
    let size = extent.map(|v| v as usize);
    let count = size[0]
        .checked_mul(size[1])
        .filter(|&n| n <= 16_000_000)
        .ok_or_else(|| WaterError::new("surface cut", "cut exceeds model node capacity"))?;
    let mut nodes = Vec::with_capacity(count);
    for z in 0..size[1] {
        for x in 0..size[0] {
            let p = sheet.forward([origin[0] + x as f64 * cell, origin[1] + z as f64 * cell]);
            if p.world
                .iter()
                .chain([p.height].iter())
                .any(|v| !v.is_finite())
            {
                return Err(WaterError::new("surface cut", "nonfinite forward sheet"));
            }
            nodes.push(p);
        }
    }
    let mut best = None;
    let mut hits = 0;
    for z in 0..size[1] - 1 {
        for x in 0..size[0] - 1 {
            let i = z * size[0] + x;
            let rest0 = [origin[0] + x as f64 * cell, origin[1] + z as f64 * cell];
            for ids in [
                [i, i + 1, i + size[0] + 1],
                [i, i + size[0] + 1, i + size[0]],
            ] {
                let triangle = ids.map(|j| nodes[j]);
                if let Some(weights) = barycentric(triangle.map(|p| p.world), point) {
                    hits += 1;
                    let height = (0..3).map(|j| weights[j] * triangle[j].height).sum::<f64>();
                    let rest = std::array::from_fn(|a| {
                        (0..3)
                            .map(|j| {
                                let node = ids[j];
                                let s = if a == 0 {
                                    node % size[0]
                                } else {
                                    node / size[0]
                                };
                                weights[j] * (origin[a] + s as f64 * cell)
                            })
                            .sum()
                    });
                    if best.as_ref().is_none_or(|(h, _, _)| height > *h) {
                        best = Some((height, rest, rest0));
                    }
                }
            }
        }
    }
    let (height, rest, rest0) =
        best.ok_or_else(|| WaterError::new("surface cut", "no triangle covers column"))?;
    let refined = refine(sheet, point, rest, rest0, config);
    let p = sheet.forward(refined);
    let residual = (p.world[0] - point[0]).hypot(p.world[1] - point[1]);
    let converged = residual <= config.tolerance;
    Ok(Column {
        height: if converged { p.height } else { height },
        rest: refined,
        residual,
        converged,
        nodes: count,
        hits,
    })
}

fn barycentric(t: [[f64; 2]; 3], p: [f64; 2]) -> Option<[f64; 3]> {
    let [a, b, c] = t;
    let det = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
    if det.abs() < 1e-14 {
        return None;
    }
    let u = ((b[1] - c[1]) * (p[0] - c[0]) + (c[0] - b[0]) * (p[1] - c[1])) / det;
    let v = ((c[1] - a[1]) * (p[0] - c[0]) + (a[0] - c[0]) * (p[1] - c[1])) / det;
    (u >= -1e-10 && v >= -1e-10 && u + v <= 1.0 + 1e-10).then_some([u, v, 1.0 - u - v])
}

fn refine(
    sheet: &impl Sheet,
    point: [f64; 2],
    rest: [f64; 2],
    origin: [f64; 2],
    config: CutConfig,
) -> [f64; 2] {
    let mut p = rest;
    let h = config.cell * 1e-4;
    for _ in 0..16 {
        let value = sheet.forward(p).world;
        let error = [value[0] - point[0], value[1] - point[1]];
        if error[0].hypot(error[1]) <= config.tolerance {
            return p;
        }
        let jac: [[f64; 2]; 2] = std::array::from_fn(|a| {
            let mut lo = p;
            lo[a] -= h;
            let mut hi = p;
            hi[a] += h;
            let lo = sheet.forward(lo).world;
            let hi = sheet.forward(hi).world;
            std::array::from_fn(|b| (hi[b] - lo[b]) / (2.0 * h))
        });
        let det = jac[0][0] * jac[1][1] - jac[1][0] * jac[0][1];
        if det.abs() < 1e-12 {
            return rest;
        }
        let next = [
            p[0] - (jac[1][1] * error[0] - jac[1][0] * error[1]) / det,
            p[1] - (jac[0][0] * error[1] - jac[0][1] * error[0]) / det,
        ];
        if (0..2).any(|a| next[a] < origin[a] || next[a] > origin[a] + config.cell) {
            return rest;
        }
        p = next;
    }
    rest
}
