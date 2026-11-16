//! Brinks: the faces of the bed where the ground drops away. A face is a brink where the ground
//! beyond it, followed downhill away from the face, falls at least the profile's least fall at a
//! slope steeper than the profile's steepest chart slope, and the cell above it is the top of that
//! steep run. It is a property of the bed alone, so a river has the same brinks at every flow.
use crate::{Grid, Profile};

/// Which way the ground drops away across a brink, from the cell above it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Side {
    /// Toward +x.
    PlusX,
    /// Toward −x.
    MinusX,
    /// Toward +z.
    PlusZ,
    /// Toward −z.
    MinusZ,
}

impl Side {
    /// Every side, in the order brinks are listed.
    pub const ALL: [Self; 4] = [Self::PlusX, Self::MinusX, Self::PlusZ, Self::MinusZ];

    /// Unit vector in world x/z.
    pub fn normal(self) -> [f64; 2] {
        match self {
            Self::PlusX => [1.0, 0.0],
            Self::MinusX => [-1.0, 0.0],
            Self::PlusZ => [0.0, 1.0],
            Self::MinusZ => [0.0, -1.0],
        }
    }

    pub(crate) fn index(self) -> u64 {
        match self {
            Self::PlusX => 0,
            Self::MinusX => 1,
            Self::PlusZ => 2,
            Self::MinusZ => 3,
        }
    }

    /// Whether the face's normal is along x.
    pub(crate) fn along_x(self) -> bool {
        matches!(self, Self::PlusX | Self::MinusX)
    }

    /// Whether the cell above lies on the face's low-index side.
    pub(crate) fn upper_first(self) -> bool {
        matches!(self, Self::PlusX | Self::PlusZ)
    }
}

/// A face of the bed where the ground drops away.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Brink {
    /// The cell above the face: the lip.
    pub cell: usize,
    /// Which way the ground drops.
    pub side: Side,
    /// The fall from the lip's bed to the foot's, metres.
    pub drop: f64,
    /// The cell where the steep run ends, on the ground below.
    pub foot: usize,
}

impl Brink {
    /// The cell across the face from the lip.
    /// # Panics
    /// If the brink is not of `grid`.
    pub fn below(&self, grid: &Grid) -> usize {
        step(grid, self.cell, self.side).expect("a brink has a cell below it")
    }

    /// The face's two ends, world x/z.
    pub fn face(&self, grid: &Grid) -> [[f64; 2]; 2] {
        let c = grid.point(self.cell);
        let h = 0.5 * grid.spacing();
        let [nx, nz] = self.side.normal();
        let m = [c[0] + nx * h, c[1] + nz * h];
        // Ends ordered along the lip's direction, +z for an x-face and +x for a z-face.
        if self.side.along_x() {
            [[m[0], m[1] - h], [m[0], m[1] + h]]
        } else {
            [[m[0] - h, m[1]], [m[0] + h, m[1]]]
        }
    }

    /// The cells strictly on the face: those between the lip and the foot.
    /// # Panics
    /// If the brink is not of `grid`.
    pub fn face_cells(&self, grid: &Grid) -> Vec<usize> {
        let mut out = Vec::new();
        let mut c = self.below(grid);
        while c != self.foot {
            out.push(c);
            c = step(grid, c, self.side).expect("the run reaches its foot");
        }
        out
    }
}

/// The neighbour of `cell` on `side`, if the grid has one.
pub(crate) fn step(grid: &Grid, cell: usize, side: Side) -> Option<usize> {
    let [nx, nz] = grid.size();
    let (x, z) = (cell % nx, cell / nx);
    match side {
        Side::PlusX => (x + 1 < nx).then_some(cell + 1),
        Side::MinusX => (x > 0).then(|| cell - 1),
        Side::PlusZ => (z + 1 < nz).then_some(cell + nx),
        Side::MinusZ => (z > 0).then(|| cell - nx),
    }
}

fn back(side: Side) -> Side {
    match side {
        Side::PlusX => Side::MinusX,
        Side::MinusX => Side::PlusX,
        Side::PlusZ => Side::MinusZ,
        Side::MinusZ => Side::PlusZ,
    }
}

/// Every brink of the bed, in cell order and then side order.
pub fn brinks(grid: &Grid, profile: &Profile) -> Vec<Brink> {
    let z = grid.heights();
    let steep = profile.steepest_slope * grid.spacing();
    let drops = |a: usize, b: usize| z[a] - z[b] > steep;
    let mut out = Vec::new();
    for cell in 0..z.len() {
        for side in Side::ALL {
            let Some(below) = step(grid, cell, side) else {
                continue;
            };
            // The lip is the top of the steep run.
            if !drops(cell, below) || step(grid, cell, back(side)).is_some_and(|b| drops(b, cell)) {
                continue;
            }
            let mut foot = below;
            while let Some(next) = step(grid, foot, side).filter(|&n| drops(foot, n)) {
                foot = next;
            }
            let drop = z[cell] - z[foot];
            if drop >= profile.least_fall {
                out.push(Brink {
                    cell,
                    side,
                    drop,
                    foot,
                });
            }
        }
    }
    out
}
