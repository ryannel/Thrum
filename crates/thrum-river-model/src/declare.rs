use crate::RiverError;

/// Bed heights at cell centres over the solve's rectangle, on the water map bed's lattice.
#[derive(Clone, Debug, PartialEq)]
pub struct Grid {
    origin: [f64; 2],
    spacing: f64,
    size: [usize; 2],
    heights: Vec<f64>,
}

impl Grid {
    /// A row-major, x-fastest grid whose first centre is at `origin`.
    /// # Errors
    /// Refuses a nonfinite origin or height, a spacing that is not positive, fewer than three
    /// cells on a side and a height count that does not match the size.
    pub fn new(
        origin: [f64; 2],
        spacing: f64,
        size: [usize; 2],
        heights: Vec<f64>,
    ) -> Result<Self, RiverError> {
        if origin.iter().any(|v| !v.is_finite())
            || !spacing.is_finite()
            || spacing <= 0.0
            || size.iter().any(|&n| n < 3)
            || size[0].checked_mul(size[1]) != Some(heights.len())
            || heights.iter().any(|v| !v.is_finite())
        {
            return Err(RiverError::invalid(
                "grid",
                "invalid origin, spacing, size or heights",
            ));
        }
        Ok(Self {
            origin,
            spacing,
            size,
            heights,
        })
    }

    /// World x/z of the first cell centre.
    pub fn origin(&self) -> [f64; 2] {
        self.origin
    }
    /// Cell spacing in metres.
    pub fn spacing(&self) -> f64 {
        self.spacing
    }
    /// Cell counts in x and z.
    pub fn size(&self) -> [usize; 2] {
        self.size
    }
    /// Bed heights, row-major and x-fastest.
    pub fn heights(&self) -> &[f64] {
        &self.heights
    }
    /// World x/z of a cell's centre.
    pub fn point(&self, cell: usize) -> [f64; 2] {
        [
            self.origin[0] + (cell % self.size[0]) as f64 * self.spacing,
            self.origin[1] + (cell / self.size[0]) as f64 * self.spacing,
        ]
    }
    /// The cell whose centre is nearest a world point, if the point lies on the grid.
    pub fn cell(&self, point: [f64; 2]) -> Option<usize> {
        let x = ((point[0] - self.origin[0]) / self.spacing).round();
        let z = ((point[1] - self.origin[1]) / self.spacing).round();
        if !(x.is_finite() && z.is_finite())
            || x < 0.0
            || z < 0.0
            || x >= self.size[0] as f64
            || z >= self.size[1] as f64
        {
            return None;
        }
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "checked whole, non-negative and inside the grid"
        )]
        Some(z as usize * self.size[0] + x as usize)
    }
    pub(crate) fn inside(&self, cell: usize, extent: [[f64; 2]; 2]) -> bool {
        let p = self.point(cell);
        (extent[0][0]..=extent[1][0]).contains(&p[0])
            && (extent[0][1]..=extent[1][1]).contains(&p[1])
    }
}

/// A spring: water that rises at a place at a declared discharge.
#[derive(Clone, Debug, PartialEq)]
pub struct Spring {
    /// Diagnostic name.
    pub name: String,
    /// World x/z of its centre.
    pub point: [f64; 2],
    /// Its discharge, cubic metres a second.
    pub discharge: f64,
    /// Water rises evenly in every cell whose centre lies within this distance, and at least in
    /// the nearest cell, metres.
    pub radius: f64,
}

/// A source body, such as a lake: the solve holds its level, and the flow that leaves it over
/// its outlet's sill is the solve's own.
#[derive(Clone, Debug, PartialEq)]
pub struct Source {
    /// The body's declared name, which the chart's source end names.
    pub name: String,
    /// A world point in its water.
    pub seed: [f64; 2],
    /// World x/z corners of the box its water lies in.
    pub extent: [[f64; 2]; 2],
    /// Its level for this solve, metres.
    pub level: f64,
    /// The lowest and highest levels its stage may reach, metres.
    pub range: [f64; 2],
}

/// A body the water may reach, such as a lake or the sea. Water leaves the solve there,
/// against the body's level at the low end of its stage's range.
#[derive(Clone, Debug, PartialEq)]
pub struct Receiver {
    /// The body's declared name, which the chart's mouth names.
    pub name: String,
    /// A world point in its water.
    pub seed: [f64; 2],
    /// World x/z corners of the box its water lies in.
    pub extent: [[f64; 2]; 2],
    /// Its level at the low end of its stage's range, metres.
    pub level: f64,
}

/// Manning's roughness, s/m^(1/3).
#[derive(Clone, Debug, PartialEq)]
pub enum Manning {
    /// One value everywhere.
    Uniform(f64),
    /// One value for each cell of the grid.
    Grid(Vec<f64>),
}

impl Manning {
    pub(crate) fn at(&self, cell: usize) -> f64 {
        match self {
            Self::Uniform(n) => *n,
            Self::Grid(n) => n[cell],
        }
    }
}

/// What a game declares for one river.
#[derive(Clone, Debug, PartialEq)]
pub struct Declaration {
    /// The river's name.
    pub name: String,
    /// Springs.
    pub springs: Vec<Spring>,
    /// Source bodies.
    pub sources: Vec<Source>,
    /// The bodies the water may reach.
    pub receivers: Vec<Receiver>,
    /// Bed roughness.
    pub manning: Manning,
}

/// The solve's physical profile and stopping rule. Never a quality setting.
#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    /// Courant number of each cell's own pseudo-time step.
    pub courant: f64,
    /// The largest level rate, metres a second, at which the solve may stop.
    pub level_rate: f64,
    /// The largest imbalance of discharge between sections, as a share of the inflow, at which the
    /// solve may stop.
    pub imbalance: f64,
    /// Pseudo-time steps a solve may take before it is refused.
    pub step_budget: u64,
    /// Steps between two evaluations of the stopping rule.
    pub check_every: u64,
    /// Depth over which a cell is wet in the chart, metres.
    pub wet_depth: f64,
    /// How far under its bed a dry cell's level is held where no wet level beside it is lower, metres.
    pub dry_margin: f64,
    /// Rings of dry bank cells grown round the wet cells into the footprint.
    pub margin_cells: usize,
    /// Join length at each end, metres.
    pub join_length: f64,
    /// The chop's rms height over depth times friction slope (`c`).
    pub chop: f64,
    /// `WATERS`' depth limit on a layer's significant height, as a share of the depth.
    pub depth_limit: f64,
    /// A stretch of channel narrower than this many cells is reported.
    pub least_width: usize,
    /// Worker threads; zero takes the machine's parallelism. Never changes a result.
    pub threads: usize,
}

impl Default for Profile {
    /// The first profile.
    fn default() -> Self {
        Self {
            courant: 0.45,
            level_rate: 1e-5,
            imbalance: 2e-3,
            step_budget: 400_000,
            check_every: 10,
            wet_depth: 1e-3,
            dry_margin: 0.05,
            margin_cells: 2,
            join_length: 2.0,
            chop: 2.0,
            depth_limit: 0.6,
            least_width: 4,
            threads: 0,
        }
    }
}

impl Profile {
    /// The friction slope at which the chop meets the depth limit: `S_cap = limit / (4 c)`.
    pub fn slope_cap(&self) -> f64 {
        self.depth_limit / (4.0 * self.chop)
    }
}
