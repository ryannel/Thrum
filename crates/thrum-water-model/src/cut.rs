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
    /// Number of fine cells across a coarse cell.
    pub coarse_cells: u32,
    /// Minimum expansion of a coarse cell's projected bounds, metres.
    pub margin_floor: f64,
    /// Scale of corner displacement spread added to the margin.
    pub bend_factor: f64,
    /// Maximum plan residual after Newton refinement, metres.
    pub tolerance: f64,
}

impl Default for CutConfig {
    fn default() -> Self {
        Self {
            cell: 0.25,
            coarse_cells: 8,
            margin_floor: 0.5,
            bend_factor: 1.0,
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
    let fine = Bounds::for_column(sheet, point, config)?;
    let step = i32::try_from(config.coarse_cells)
        .map_err(|_| WaterError::new("surface cut", "invalid coarse cell"))?;
    let coarse = Bounds {
        lo: fine.lo.map(|v| v.div_euclid(step)),
        hi: fine.hi.map(|v| -(-v).div_euclid(step)),
    };
    let coarse_cell = config.cell * f64::from(step);
    let grid = Lattice::new(coarse, coarse_cell, |rest| sheet.forward(rest))?;
    let mut best = None;
    let mut hits = 0;
    let mut nodes = grid.nodes.len();
    for z in coarse.lo[1]..coarse.hi[1] {
        for x in coarse.lo[0]..coarse.hi[0] {
            let corners = grid.corners([x, z]);
            if !keeps_column(corners, point, coarse_cell, config) {
                continue;
            }
            let bounds = Bounds {
                lo: [x, z].map(|v| v * step),
                hi: [x, z].map(|v| (v + 1) * step),
            }
            .intersection(fine);
            let grid = Lattice::new(bounds, config.cell, |rest| sheet.forward(rest))?;
            nodes += grid.nodes.len();
            grid.scan(bounds, point, &mut best, &mut hits);
        }
    }
    finish(sheet, point, config, best, hits, nodes)
}

impl CutConfig {
    pub(crate) fn valid(self) -> bool {
        self.cell.is_finite()
            && self.cell > 0.0
            && (1..=32).contains(&self.coarse_cells)
            && self.margin_floor.is_finite()
            && self.margin_floor >= 0.0
            && self.bend_factor.is_finite()
            && self.bend_factor >= 0.0
            && self.tolerance.is_finite()
            && self.tolerance > 0.0
    }
}

#[derive(Clone, Copy)]
struct Bounds {
    lo: [i32; 2],
    hi: [i32; 2],
}

impl Bounds {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "integer lattice coordinates are range checked before conversion"
    )]
    fn for_column(
        sheet: &impl Sheet,
        point: [f64; 2],
        config: CutConfig,
    ) -> Result<Self, WaterError> {
        let reach = sheet.reach(point);
        if !config.valid() || !reach.is_finite() || reach < 0.0 {
            return Err(WaterError::new(
                "surface cut",
                "invalid cut controls or reach",
            ));
        }
        let lo = point.map(|p| ((p - reach) / config.cell).floor());
        let hi = point.map(|p| ((p + reach) / config.cell).ceil() + 1.0);
        if lo.into_iter().chain(hi).any(|v| !(-1e9..=1e9).contains(&v)) {
            return Err(WaterError::new("surface cut", "invalid lattice bounds"));
        }
        Ok(Self {
            lo: lo.map(|v| v as i32),
            hi: hi.map(|v| v as i32),
        })
    }

    fn intersection(self, other: Self) -> Self {
        Self {
            lo: std::array::from_fn(|a| self.lo[a].max(other.lo[a])),
            hi: std::array::from_fn(|a| self.hi[a].min(other.hi[a])),
        }
    }
}

struct Lattice {
    bounds: Bounds,
    cell: f64,
    width: usize,
    nodes: Vec<SheetPoint>,
}

impl Lattice {
    fn new(
        bounds: Bounds,
        cell: f64,
        forward: impl Fn([f64; 2]) -> SheetPoint,
    ) -> Result<Self, WaterError> {
        let mut grid = Self::allocate(bounds, cell)?;
        let rows = grid.rows();
        rows.fill(0, &mut grid.nodes, forward);
        grid.validate()?;
        Ok(grid)
    }

    fn allocate(bounds: Bounds, cell: f64) -> Result<Self, WaterError> {
        let width = usize::try_from(bounds.hi[0] - bounds.lo[0] + 1).expect("ordered bounds");
        let height = usize::try_from(bounds.hi[1] - bounds.lo[1] + 1).expect("ordered bounds");
        let count = width
            .checked_mul(height)
            .filter(|n| *n <= 16_000_000)
            .ok_or_else(|| WaterError::new("surface cut", "cut exceeds model node capacity"))?;
        let nodes = vec![
            SheetPoint {
                world: [0.0; 2],
                height: 0.0
            };
            count
        ];
        Ok(Self {
            bounds,
            cell,
            width,
            nodes,
        })
    }

    fn rows(&self) -> LatticeRows {
        LatticeRows {
            bounds: self.bounds,
            cell: self.cell,
            width: self.width,
        }
    }

    fn validate(&self) -> Result<(), WaterError> {
        if self
            .nodes
            .iter()
            .any(|p| !p.height.is_finite() || p.world.iter().any(|v| !v.is_finite()))
        {
            return Err(WaterError::new("surface cut", "nonfinite forward sheet"));
        }
        Ok(())
    }

    fn corners(&self, [x, z]: [i32; 2]) -> [SheetPoint; 4] {
        let x = usize::try_from(x - self.bounds.lo[0]).expect("lattice x");
        let z = usize::try_from(z - self.bounds.lo[1]).expect("lattice z");
        let i = z * self.width + x;
        [
            self.nodes[i],
            self.nodes[i + 1],
            self.nodes[i + self.width + 1],
            self.nodes[i + self.width],
        ]
    }

    fn scan(&self, bounds: Bounds, point: [f64; 2], best: &mut Option<Hit>, hits: &mut usize) {
        for z in bounds.lo[1]..bounds.hi[1] {
            for x in bounds.lo[0]..bounds.hi[0] {
                self.scan_cell([x, z], point, best, hits);
            }
        }
    }

    fn scan_cell(
        &self,
        [x, z]: [i32; 2],
        point: [f64; 2],
        best: &mut Option<Hit>,
        hits: &mut usize,
    ) {
        let corners = self.corners([x, z]);
        if !in_landing_box(corners, point, 1e-9) {
            return;
        }
        let origin = [x, z].map(|v| f64::from(v) * self.cell);
        let offsets = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        for (face, ids) in [[0, 1, 2], [0, 2, 3]].into_iter().enumerate() {
            let triangle = ids.map(|j| corners[j]);
            if let Some(weights) = barycentric(triangle.map(|p| p.world), point) {
                *hits += 1;
                let height = (0..3).map(|j| weights[j] * triangle[j].height).sum::<f64>();
                let order = (z, x, face);
                if best.is_none_or(|h| {
                    height > h.height
                        || (height
                            .partial_cmp(&h.height)
                            .is_some_and(std::cmp::Ordering::is_eq)
                            && order < h.order)
                }) {
                    let rest = std::array::from_fn(|a| {
                        (0..3)
                            .map(|j| weights[j] * (origin[a] + self.cell * offsets[ids[j]][a]))
                            .sum()
                    });
                    *best = Some(Hit {
                        height,
                        rest,
                        origin,
                        order,
                    });
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
struct LatticeRows {
    bounds: Bounds,
    cell: f64,
    width: usize,
}

impl LatticeRows {
    fn fill(
        self,
        first_row: usize,
        rows: &mut [SheetPoint],
        forward: impl Fn([f64; 2]) -> SheetPoint,
    ) {
        for (index, p) in rows.iter_mut().enumerate() {
            *p = forward([
                (f64::from(self.bounds.lo[0]) + (index % self.width) as f64) * self.cell,
                (f64::from(self.bounds.lo[1]) + (first_row + index / self.width) as f64)
                    * self.cell,
            ]);
        }
    }
}

#[derive(Clone, Copy)]
struct Hit {
    height: f64,
    rest: [f64; 2],
    origin: [f64; 2],
    order: (i32, i32, usize),
}

fn in_landing_box(corners: [SheetPoint; 4], point: [f64; 2], margin: f64) -> bool {
    (0..2).all(|a| {
        let coords = corners.map(|p| p.world[a]);
        point[a] >= coords.into_iter().fold(f64::INFINITY, f64::min) - margin
            && point[a] <= coords.into_iter().fold(f64::NEG_INFINITY, f64::max) + margin
    })
}

fn keeps_column(corners: [SheetPoint; 4], point: [f64; 2], cell: f64, config: CutConfig) -> bool {
    let offsets = [[0.0, 0.0], [cell, 0.0], [cell, cell], [0.0, cell]];
    let displacement: [[f64; 2]; 4] =
        std::array::from_fn(|i| std::array::from_fn(|a| corners[i].world[a] - offsets[i][a]));
    let mut spread_squared = 0.0_f64;
    for (i, a) in displacement.iter().enumerate() {
        for b in &displacement[..i] {
            spread_squared = spread_squared.max((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2));
        }
    }
    in_landing_box(
        corners,
        point,
        config
            .bend_factor
            .mul_add(spread_squared.sqrt(), config.margin_floor),
    )
}

fn finish(
    sheet: &impl Sheet,
    point: [f64; 2],
    config: CutConfig,
    best: Option<Hit>,
    hits: usize,
    nodes: usize,
) -> Result<Column, WaterError> {
    let hit = best.ok_or_else(|| WaterError::new("surface cut", "no triangle covers column"))?;
    let refined = refine(sheet, point, hit.rest, hit.origin, config);
    let p = sheet.forward(refined);
    let residual = (p.world[0] - point[0]).hypot(p.world[1] - point[1]);
    let converged = residual <= config.tolerance;
    Ok(Column {
        height: if converged { p.height } else { hit.height },
        rest: refined,
        residual,
        converged,
        nodes,
        hits,
    })
}

/// A cut on one fine lattice covering every supplied column's reach.
/// Shares forward evaluations across the columns; it does no coarse filtering.
pub struct WholeCut<'a, S> {
    sheet: &'a S,
    grid: Lattice,
    config: CutConfig,
    projection: Option<ProjectedCells>,
}

struct ProjectedCells {
    lo: [f64; 2],
    hi: [f64; 2],
    cells: Vec<[i32; 2]>,
}

impl<'a, S: Sheet + Sync> WholeCut<'a, S> {
    pub(crate) fn is_for(&self, sheet: &S) -> bool {
        std::ptr::eq(self.sheet, sheet)
    }

    /// Sample the union bounds of all requested columns.
    /// # Errors
    /// Refuses empty, invalid or oversized lattices.
    pub fn new(sheet: &'a S, points: &[[f64; 2]], config: CutConfig) -> Result<Self, WaterError> {
        let (first, rest) = points
            .split_first()
            .ok_or_else(|| WaterError::new("whole surface cut", "no columns supplied"))?;
        let mut bounds = Bounds::for_column(sheet, *first, config)?;
        for &p in rest {
            let next = Bounds::for_column(sheet, p, config)?;
            bounds.lo = std::array::from_fn(|a| bounds.lo[a].min(next.lo[a]));
            bounds.hi = std::array::from_fn(|a| bounds.hi[a].max(next.hi[a]));
        }
        let mut grid = Lattice::allocate(bounds, config.cell)?;
        let rows = grid.rows();
        let forward = |rest| sheet.forward(rest);
        let workers = std::thread::available_parallelism().map_or(1, std::num::NonZero::get);
        if !cfg!(target_family = "wasm") && workers > 1 && grid.nodes.len() >= 100_000 {
            let chunk_rows = (grid.nodes.len() / rows.width).div_ceil(workers.min(12));
            std::thread::scope(|scope| {
                for (index, chunk) in grid.nodes.chunks_mut(chunk_rows * rows.width).enumerate() {
                    scope.spawn(move || rows.fill(index * chunk_rows, chunk, forward));
                }
            });
        } else {
            rows.fill(0, &mut grid.nodes, forward);
        }
        grid.validate()?;
        Ok(Self {
            sheet,
            grid,
            config,
            projection: None,
        })
    }

    /// Keep the cells whose displaced bounds overlap a rectangle in world x/z.
    /// Columns outside it continue to scan their full lattice reach.
    #[must_use]
    pub fn with_projection(mut self, lo: [f64; 2], hi: [f64; 2]) -> Self {
        let bounds = self.grid.bounds;
        let mut cells = Vec::new();
        for z in bounds.lo[1]..bounds.hi[1] {
            for x in bounds.lo[0]..bounds.hi[0] {
                let corners = self.grid.corners([x, z]);
                if (0..2).all(|axis| {
                    corners.iter().any(|p| p.world[axis] <= hi[axis] + 1e-9)
                        && corners.iter().any(|p| p.world[axis] >= lo[axis] - 1e-9)
                }) {
                    cells.push([x, z]);
                }
            }
        }
        self.projection = Some(ProjectedCells { lo, hi, cells });
        self
    }

    /// Find and refine the highest triangle across this column's whole reach.
    /// # Errors
    /// Refuses a column outside the prepared bounds or with no triangle hit.
    pub fn column(&self, point: [f64; 2]) -> Result<Column, WaterError> {
        let bounds = Bounds::for_column(self.sheet, point, self.config)?;
        let covered = bounds.intersection(self.grid.bounds);
        if covered.lo != bounds.lo || covered.hi != bounds.hi {
            return Err(WaterError::new(
                "whole surface cut",
                "column outside prepared lattice",
            ));
        }
        let mut best = None;
        let mut hits = 0;
        if let Some(region) = &self.projection
            && (0..2).all(|i| point[i] >= region.lo[i] && point[i] <= region.hi[i])
        {
            for &[x, z] in &region.cells {
                if x >= bounds.lo[0] && x < bounds.hi[0] && z >= bounds.lo[1] && z < bounds.hi[1] {
                    self.grid.scan_cell([x, z], point, &mut best, &mut hits);
                }
            }
        } else {
            self.grid.scan(bounds, point, &mut best, &mut hits);
        }
        finish(
            self.sheet,
            point,
            self.config,
            best,
            hits,
            self.grid.nodes.len(),
        )
    }
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
