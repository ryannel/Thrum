//! Exposure: the share of the rain that reaches a point, from shelters a game gives as
//! content. Rain falls straight down, so a shelter covers the columns its triangles cross.
use crate::{Profile, RainError};

/// Triangles that shelter what is under them, and the share of the rain they let through:
/// none for a roof, a part for a tree's crown. Triangles are x/y/z in metres.
#[derive(Clone, Debug)]
pub struct Shelter {
    /// Name used when the shelter is refused.
    pub name: String,
    /// Triangles; a vertical triangle covers no column.
    pub triangles: Vec<[[f64; 3]; 3]>,
    /// Share of the rain let through, from 0 to 1.
    pub share: f64,
}

/// One shelter's lowest underside over a column, and its share.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Layer {
    underside: f64,
    share: f64,
}

const NO_LAYER: Layer = Layer {
    underside: f64::NEG_INFINITY,
    share: 1.0,
};

/// The canopy grid: for each column, up to two layers.
#[derive(Clone, Debug)]
pub struct Canopy {
    origin: [f64; 2],
    cell: f64,
    size: [usize; 2],
    layers: Vec<[Layer; 2]>,
    edge_width: f64,
}

impl Canopy {
    /// Open sky everywhere: exposure is 1 at every point.
    pub fn open() -> Self {
        Self {
            origin: [0.0; 2],
            cell: 1.0,
            size: [0, 0],
            layers: Vec::new(),
            edge_width: 0.0,
        }
    }

    /// Rasterise shelters from above into a grid of `size` columns of the profile's cell,
    /// whose first column's corner is `origin`. A shelter counts once in a column however
    /// many of its triangles cross it, at its lowest underside there.
    /// # Errors
    /// Refuses a bad grid or profile, a malformed shelter, and, by name and place, a column
    /// that a third shelter crosses.
    pub fn prepare(
        origin: [f64; 2],
        size: [usize; 2],
        shelters: &[Shelter],
        profile: &Profile,
    ) -> Result<Self, RainError> {
        let cell = profile.canopy_cell;
        if !cell.is_finite() || cell <= 0.0 || !(profile.edge_width.is_finite()) {
            return Err(RainError::Profile("invalid canopy cell or edge width"));
        }
        if profile.edge_width < 0.0 {
            return Err(RainError::Profile("negative edge width"));
        }
        let count = size[0]
            .checked_mul(size[1])
            .ok_or(RainError::Ground("canopy grid overflow"))?;
        if origin.iter().any(|v| !v.is_finite()) {
            return Err(RainError::Ground("nonfinite canopy origin"));
        }
        let mut canopy = Self {
            origin,
            cell,
            size,
            layers: vec![[NO_LAYER; 2]; count],
            edge_width: profile.edge_width,
        };
        for shelter in shelters {
            canopy.add(shelter)?;
        }
        Ok(canopy)
    }

    fn add(&mut self, shelter: &Shelter) -> Result<(), RainError> {
        let first = shelter
            .triangles
            .first()
            .map_or([0.0; 2], |t| [t[0][0], t[0][2]]);
        let refuse = |place, reason| RainError::Shelter {
            name: shelter.name.clone(),
            place,
            reason,
        };
        if !(0.0..=1.0).contains(&shelter.share) {
            return Err(refuse(first, "share outside 0 to 1"));
        }
        if shelter
            .triangles
            .iter()
            .flatten()
            .flatten()
            .any(|v| !v.is_finite())
        {
            return Err(refuse(first, "nonfinite triangle"));
        }
        if self.layers.is_empty() {
            return Ok(());
        }
        let mut lo = [f64::INFINITY; 2];
        let mut hi = [f64::NEG_INFINITY; 2];
        for v in shelter.triangles.iter().flatten() {
            for (axis, k) in [(0, 0), (1, 2)] {
                lo[axis] = lo[axis].min(v[k]);
                hi[axis] = hi[axis].max(v[k]);
            }
        }
        let Some(([x0, z0], [x1, z1])) = self.cell_range(lo, hi) else {
            return Ok(());
        };
        for iz in z0..=z1 {
            for ix in x0..=x1 {
                let centre = [
                    self.origin[0] + (ix as f64 + 0.5) * self.cell,
                    self.origin[1] + (iz as f64 + 0.5) * self.cell,
                ];
                let Some(underside) = lowest(&shelter.triangles, centre) else {
                    continue;
                };
                let column = &mut self.layers[iz * self.size[0] + ix];
                let layer = Layer {
                    underside,
                    share: shelter.share,
                };
                if column[0] == NO_LAYER {
                    column[0] = layer;
                } else if column[1] == NO_LAYER {
                    column[1] = layer;
                } else {
                    return Err(refuse(centre, "a third shelter crosses this column"));
                }
            }
        }
        Ok(())
    }

    /// Columns whose centres may lie in a world x/z box, clamped to the grid.
    fn cell_range(&self, lo: [f64; 2], hi: [f64; 2]) -> Option<([usize; 2], [usize; 2])> {
        let mut a = [0; 2];
        let mut b = [0; 2];
        for axis in 0..2 {
            let first = ((lo[axis] - self.origin[axis]) / self.cell - 0.5).ceil();
            let last = ((hi[axis] - self.origin[axis]) / self.cell - 0.5).floor();
            let top = (self.size[axis] - 1) as f64;
            if last < 0.0 || first > top || first > last {
                return None;
            }
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "clamped within the grid"
            )]
            {
                a[axis] = first.max(0.0) as usize;
                b[axis] = last.min(top) as usize;
            }
        }
        Some((a, b))
    }

    /// Exposure of one column at a height: the product of the shares of the layers whose
    /// underside is above it, and 1 outside the grid.
    fn column(&self, ix: i64, iz: i64, height: f64) -> f64 {
        let inside = |i: i64, n: usize| usize::try_from(i).ok().filter(|&i| i < n);
        let (Some(x), Some(z)) = (inside(ix, self.size[0]), inside(iz, self.size[1])) else {
            return 1.0;
        };
        self.layers[z * self.size[0] + x]
            .iter()
            .filter(|l| l.underside > height)
            .map(|l| l.share)
            .product()
    }

    /// The heights above a point's at which its exposure changes, each with the exposure
    /// from that height up to the next: the undersides above it of the layers in the columns
    /// its edge-width square touches, lowest first. Exposure never falls with height.
    pub(crate) fn steps(&self, point: [f64; 3]) -> Vec<(f64, f64)> {
        let [x, y, z] = point;
        let mut heights: Vec<f64> = Vec::new();
        if !self.layers.is_empty() {
            let ((x0, x1), (z0, z1)) = self.span(x, z);
            for iz in z0..=z1 {
                for ix in x0..=x1 {
                    let inside = |i: i64, n: usize| usize::try_from(i).ok().filter(|&i| i < n);
                    let (Some(cx), Some(cz)) = (inside(ix, self.size[0]), inside(iz, self.size[1]))
                    else {
                        continue;
                    };
                    for l in &self.layers[cz * self.size[0] + cx] {
                        if l.underside > y {
                            heights.push(l.underside);
                        }
                    }
                }
            }
        }
        heights.sort_by(f64::total_cmp);
        heights.dedup();
        heights
            .into_iter()
            .map(|h| (h, self.exposure([x, h, z])))
            .collect()
    }

    /// The columns a point's edge-width square may touch, as inclusive index ranges.
    fn span(&self, x: f64, z: f64) -> ((i64, i64), (i64, i64)) {
        let local = [
            (x - self.origin[0]) / self.cell,
            (z - self.origin[1]) / self.cell,
        ];
        let half = 0.5 * self.edge_width / self.cell;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "cell indices of finite points; columns outside the grid are open"
        )]
        let span = |c: f64| ((c - half).floor() as i64, (c + half).floor() as i64);
        (span(local[0]), span(local[1]))
    }

    /// Exposure at a world point (x, y, z): the columns' exposures at height y averaged
    /// over a square of the profile's edge width centred on the point, so it passes from
    /// one shelter's share to the next over that width. The same function on any device.
    pub fn exposure(&self, point: [f64; 3]) -> f64 {
        if self.layers.is_empty() {
            return 1.0;
        }
        let [x, y, z] = point;
        let local = [
            (x - self.origin[0]) / self.cell,
            (z - self.origin[1]) / self.cell,
        ];
        #[expect(
            clippy::cast_possible_truncation,
            reason = "cell indices of finite points; columns outside the grid are open"
        )]
        let index = |v: f64| v.floor() as i64;
        if self.edge_width == 0.0 {
            return self.column(index(local[0]), index(local[1]), y);
        }
        let half = 0.5 * self.edge_width / self.cell;
        let span = |c: f64| (index(c - half), index(c + half));
        let overlap = |i: i64, c: f64| ((i + 1) as f64).min(c + half) - (i as f64).max(c - half);
        let ((x0, x1), (z0, z1)) = (span(local[0]), span(local[1]));
        let mut sum = 0.0;
        for iz in z0..=z1 {
            let wz = overlap(iz, local[1]);
            if wz <= 0.0 {
                continue;
            }
            for ix in x0..=x1 {
                let wx = overlap(ix, local[0]);
                if wx > 0.0 {
                    sum += wx * wz * self.column(ix, iz, y);
                }
            }
        }
        // Rounding in the weights may carry the mean of open columns a little past 1.
        (sum / (4.0 * half * half)).min(1.0)
    }
}

impl thrum_water_model::Exposure for Canopy {
    fn exposure(&self, point: [f64; 3]) -> f64 {
        Self::exposure(self, point)
    }

    fn canopy_grid(&self) -> Option<thrum_water_model::CanopyGrid> {
        Some(thrum_water_model::CanopyGrid {
            origin: self.origin,
            cell: self.cell,
            size: self.size,
            edge_width: self.edge_width,
            columns: self
                .layers
                .iter()
                .map(|column| column.map(|layer| [layer.underside, layer.share]))
                .collect(),
        })
    }
}

/// The lowest height at which a vertical line through a point crosses a triangle.
fn lowest(triangles: &[[[f64; 3]; 3]], [x, z]: [f64; 2]) -> Option<f64> {
    triangles
        .iter()
        .filter_map(|&[a, b, c]| {
            let det = (b[2] - c[2]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[2] - c[2]);
            if det.abs() < 1e-12 {
                return None;
            }
            let u = ((b[2] - c[2]) * (x - c[0]) + (c[0] - b[0]) * (z - c[2])) / det;
            let v = ((c[2] - a[2]) * (x - c[0]) + (a[0] - c[0]) * (z - c[2])) / det;
            (u >= -1e-10 && v >= -1e-10 && u + v <= 1.0 + 1e-10)
                .then(|| u * a[1] + v * b[1] + (1.0 - u - v) * c[1])
        })
        .reduce(f64::min)
}
