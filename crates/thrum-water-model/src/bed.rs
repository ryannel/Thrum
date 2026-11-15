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
        for i in 0..count {
            let [x, z] = bed.point(i);
            for &[a, b, c] in triangles {
                let det = (b[2] - c[2]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[2] - c[2]);
                if det.abs() < 1e-12 {
                    continue;
                }
                let u = ((b[2] - c[2]) * (x - c[0]) + (c[0] - b[0]) * (z - c[2])) / det;
                let v = ((c[2] - a[2]) * (x - c[0]) + (a[0] - c[0]) * (z - c[2])) / det;
                if u >= -1e-10 && v >= -1e-10 && u + v <= 1.0 + 1e-10 {
                    bed.heights[i] = bed.heights[i].max(u * a[1] + v * b[1] + (1.0 - u - v) * c[1]);
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

    pub(crate) fn point(&self, i: usize) -> [f64; 2] {
        [
            self.origin[0] + (i % self.size[0]) as f64 * self.spacing,
            self.origin[1] + (i / self.size[0]) as f64 * self.spacing,
        ]
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
