use crate::WaterError;

/// Regular height samples at cell centres, bilinearly interpolated between centres.
#[derive(Clone, Debug)]
pub struct Bed {
    /// World x/z of the first centre.
    pub(crate) origin: [f64; 2],
    /// Centre spacing in metres.
    pub(crate) spacing: f64,
    /// Sample counts in x/z; each at least two.
    pub(crate) size: [usize; 2],
    pub(crate) heights: Vec<f64>,
}

impl Bed {
    /// First centre in world x/z, uniform spacing in metres and centre counts.
    pub fn lattice(&self) -> ([f64; 2], f64, [usize; 2]) {
        (self.origin, self.spacing, self.size)
    }

    /// Heights in row-major x-fastest order.
    pub fn heights(&self) -> &[f64] {
        &self.heights
    }

    /// Validate a row-major x-fastest height grid.
    /// # Errors
    /// Refuses nonfinite heights, invalid spacing, dimensions or sample count.
    pub fn new(
        origin: [f64; 2],
        spacing: f64,
        size: [usize; 2],
        heights: Vec<f64>,
    ) -> Result<Self, WaterError> {
        if origin.iter().any(|x| !x.is_finite())
            || !spacing.is_finite()
            || spacing <= 0.0
            || size.iter().any(|&n| n < 2)
            || size[0].checked_mul(size[1]) != Some(heights.len())
            || heights.iter().any(|x| !x.is_finite())
        {
            return Err(WaterError::new("bed", "invalid grid"));
        }
        Ok(Self {
            origin,
            spacing,
            size,
            heights,
        })
    }

    /// Rasterize vertical rays through centres, keeping the highest intersected triangle.
    /// `floor` supplies uncovered centres. Triangle coordinates are x/y/z in metres.
    /// # Errors
    /// Refuses invalid grids, a nonfinite floor or nonfinite triangle coordinates.
    pub fn from_triangles(
        origin: [f64; 2],
        spacing: f64,
        size: [usize; 2],
        floor: f64,
        triangles: &[[[f64; 3]; 3]],
    ) -> Result<Self, WaterError> {
        let count = size[0]
            .checked_mul(size[1])
            .ok_or_else(|| WaterError::new("bed", "grid overflow"))?;
        if triangles.iter().flatten().flatten().any(|v| !v.is_finite()) {
            return Err(WaterError::new("bed", "nonfinite triangle"));
        }
        let mut bed = Self::new(origin, spacing, size, vec![floor; count])?;
        let mut covered = vec![false; count];
        for &[a, b, c] in triangles {
            let det = (b[2] - c[2]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[2] - c[2]);
            if det.abs() < 1e-12 {
                continue;
            }
            let range = |axis: usize, coordinate: usize| {
                let min = a[coordinate].min(b[coordinate]).min(c[coordinate]);
                let max = a[coordinate].max(b[coordinate]).max(c[coordinate]);
                // Cover the same barycentric tolerance as the intersection.
                let pad = (max - min) * 3e-10;
                let lo = ((min - pad - origin[axis]) / spacing)
                    .ceil()
                    .clamp(0.0, size[axis] as f64);
                let hi = (((max + pad - origin[axis]) / spacing).floor() + 1.0)
                    .clamp(0.0, size[axis] as f64);
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "clipped integral lattice indices"
                )]
                let indices = lo as usize..hi as usize;
                indices
            };
            let columns = range(0, 0);
            for row in range(1, 2) {
                for col in columns.clone() {
                    let i = row * size[0] + col;
                    let [x, z] = bed.point(i);
                    let u = ((b[2] - c[2]) * (x - c[0]) + (c[0] - b[0]) * (z - c[2])) / det;
                    let v = ((c[2] - a[2]) * (x - c[0]) + (a[0] - c[0]) * (z - c[2])) / det;
                    if u >= -1e-10 && v >= -1e-10 && u + v <= 1.0 + 1e-10 {
                        let height = u * a[1] + v * b[1] + (1.0 - u - v) * c[1];
                        if !covered[i] || height > bed.heights[i] {
                            covered[i] = true;
                            bed.heights[i] = height;
                        }
                    }
                }
            }
        }
        Ok(bed)
    }

    /// Height at a point, or none beyond the sample extent.
    pub fn height(&self, point: [f64; 2]) -> Option<f64> {
        self.weights(point)
            .map(|w| w.iter().map(|&(i, a)| self.heights[i] * a).sum())
    }

    pub(crate) fn weights(&self, point: [f64; 2]) -> Option<[(usize, f64); 4]> {
        let x = (point[0] - self.origin[0]) / self.spacing;
        let z = (point[1] - self.origin[1]) / self.spacing;
        if !x.is_finite()
            || !z.is_finite()
            || x < 0.0
            || z < 0.0
            || x > (self.size[0] - 1) as f64
            || z > (self.size[1] - 1) as f64
        {
            return None;
        }
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "coordinates checked within allocated grid"
        )]
        let (ix, iz) = (
            (x.floor() as usize).min(self.size[0] - 2),
            (z.floor() as usize).min(self.size[1] - 2),
        );
        let (u, v) = (x - ix as f64, z - iz as f64);
        let i = iz * self.size[0] + ix;
        Some([
            (i, (1.0 - u) * (1.0 - v)),
            (i + 1, u * (1.0 - v)),
            (i + self.size[0], (1.0 - u) * v),
            (i + self.size[0] + 1, u * v),
        ])
    }

    /// World x/z of a centre given its index in the height table.
    pub fn point(&self, i: usize) -> [f64; 2] {
        [
            self.origin[0] + (i % self.size[0]) as f64 * self.spacing,
            self.origin[1] + (i / self.size[0]) as f64 * self.spacing,
        ]
    }

    /// Can this cell's corner continue water onto the query without descending
    /// above its level? In the corner/query rectangle the bilinear bed is
    /// a + b*s + c*t + d*s*t. Its only possible interior critical point is a
    /// saddle. The endpoint slopes and that saddle decide monotone reach;
    /// comparing endpoint heights alone would bridge the far side of a crest.
    pub(crate) fn reaches(&self, corner: usize, weights: [(usize, f64); 4], level: f64) -> bool {
        let [a, b, c, d] = self.path_coefficients(corner, weights);
        let end = a + b + c + d;
        if end.max(level) < a.max(level) || (a > level && b < 0.0 && c < 0.0) {
            return false;
        }
        if b + d < 0.0 && c + d < 0.0 {
            if end > level {
                return false;
            }
            if b > 0.0 && c > 0.0 && a - b * c / d > level {
                return false;
            }
        }
        true
    }

    /// Connectivity in the strict sublevel set of the bilinear cell. Opposite
    /// wet lobes connect only above the interior saddle, not at its dry point.
    /// This also handles seeds on an edge/corner and corners of zero weight.
    pub(crate) fn wet_reaches(
        &self,
        corner: usize,
        weights: [(usize, f64); 4],
        level: f64,
    ) -> bool {
        let end: f64 = weights.iter().map(|&(i, w)| self.heights[i] * w).sum();
        if self.heights[corner] >= level || end >= level {
            return false;
        }
        let [a, b, c, d] = self.path_coefficients(corner, weights);
        !(b > 0.0 && c > 0.0 && b + d < 0.0 && c + d < 0.0 && a - b * c / d >= level)
    }

    fn path_coefficients(&self, corner: usize, weights: [(usize, f64); 4]) -> [f64; 4] {
        let h = weights.map(|(i, _)| self.heights[i]);
        let u = weights[1].1 + weights[3].1;
        let v = weights[2].1 + weights[3].1;
        let k = weights
            .iter()
            .position(|&(i, _)| i == corner)
            .expect("cell corner");
        let x = if k % 2 == 0 { 0.0 } else { 1.0 };
        let z = if k < 2 { 0.0 } else { 1.0 };
        let cross = h[3] - h[2] - h[1] + h[0];
        let a = h[k];
        let b = (u - x) * (h[1] - h[0] + cross * z);
        let c = (v - z) * (h[2] - h[0] + cross * x);
        let d = cross * (u - x) * (v - z);
        [a, b, c, d]
    }

    /// All neighbouring centres sharing a bilinear cell, with weights for the
    /// neighbour in that cell. Diagonal edges still need the wet/reach predicate.
    pub(crate) fn connections(&self, i: usize) -> impl Iterator<Item = (usize, [(usize, f64); 4])> {
        let [w, h] = self.size;
        let (x, z) = (i % w, i / w);
        let diagonals = [
            (x > 0 && z > 0).then(|| i - w - 1),
            (x + 1 < w && z > 0).then(|| i - w + 1),
            (x > 0 && z + 1 < h).then(|| i + w - 1),
            (x + 1 < w && z + 1 < h).then(|| i + w + 1),
        ];
        self.neighbours(i)
            .chain(diagonals.into_iter().flatten())
            .map(move |j| {
                let cx = x.min(j % w).min(w - 2);
                let cz = z.min(j / w).min(h - 2);
                let first = cz * w + cx;
                let weights = [first, first + 1, first + w, first + w + 1]
                    .map(|k| (k, if k == j { 1.0 } else { 0.0 }));
                (j, weights)
            })
    }

    pub(crate) fn neighbours(&self, i: usize) -> impl Iterator<Item = usize> {
        let [w, h] = self.size;
        [
            (!i.is_multiple_of(w)).then(|| i - 1),
            (i % w + 1 < w).then_some(i + 1),
            (i / w > 0).then(|| i - w),
            (i / w + 1 < h).then_some(i + w),
        ]
        .into_iter()
        .flatten()
    }
    pub(crate) fn edge(&self, i: usize) -> bool {
        i.is_multiple_of(self.size[0])
            || i % self.size[0] + 1 == self.size[0]
            || i / self.size[0] == 0
            || i / self.size[0] + 1 == self.size[1]
    }
}
