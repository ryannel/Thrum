use crate::Error;

/// A validated density grid in X/Y/Z order, Y vertical, values in 0..=1.
#[derive(Clone, Debug)]
pub struct Shape {
    size: [usize; 3],
    spacing: f64,
    values: Vec<f64>,
    footprint: Footprint,
}

/// Headless column sums and occupied height bounds derived from the shape's voxels.
#[derive(Clone, Debug, PartialEq)]
pub struct Footprint {
    size: [usize; 2],
    spacing: f64,
    columns: Vec<[f64; 3]>,
}

impl Shape {
    /// Build a grid. Index is `(z * ny + y) * nx + x`; its base is y=0,
    /// its horizontal centre is the origin. Horizontal interpolation is bilinear
    /// between voxel centres, with a zero border; vertical voxels are slabs.
    /// # Errors
    /// Rejects invalid dimensions, spacing, count or density values.
    pub fn new(size: [usize; 3], spacing: f64, values: Vec<f64>) -> Result<Self, Error> {
        let count = size.iter().try_fold(1_usize, |a, &b| a.checked_mul(b));
        if size.contains(&0)
            || count != Some(values.len())
            || !spacing.is_finite()
            || spacing <= 0.0
            || !values.iter().all(|&x| crate::unit(x))
        {
            return Err(Error::Invalid);
        }
        let mut columns = Vec::with_capacity(size[0] * size[2]);
        for z in 0..size[2] {
            for x in 0..size[0] {
                let mut column = [0.0, f64::INFINITY, 0.0];
                for y in 0..size[1] {
                    let value = values[(z * size[1] + y) * size[0] + x];
                    column[0] += value * spacing;
                    if value > 0.0 {
                        column[1] = column[1].min(y as f64 * spacing);
                        column[2] = (y + 1) as f64 * spacing;
                    }
                }
                if column[0] == 0.0 {
                    column[1] = 0.0;
                }
                columns.push(column);
            }
        }
        Ok(Self {
            size,
            spacing,
            values,
            footprint: Footprint {
                size: [size[0], size[2]],
                spacing,
                columns,
            },
        })
    }

    /// Derived, camera-independent footprint.
    pub fn footprint(&self) -> &Footprint {
        &self.footprint
    }

    /// Uneroded local density, before multiplying by physical extinction.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "vertical index bounded by grid height"
    )]
    pub fn density(&self, point: [f64; 3]) -> f64 {
        if point[1] < 0.0 || point[1] >= self.size[1] as f64 * self.spacing {
            return 0.0;
        }
        let y = (point[1] / self.spacing).floor() as usize;
        self.footprint
            .weights([point[0], point[2]])
            .into_iter()
            .map(|(index, weight)| {
                let x = index % self.size[0];
                let z = index / self.size[0];
                weight * self.values[(z * self.size[1] + y) * self.size[0] + x]
            })
            .sum()
    }
}

impl Footprint {
    /// Local column integral in metres, and occupied bottom/top in metres.
    pub fn column(&self, point: [f64; 2]) -> [f64; 3] {
        let mut column = [0.0, f64::INFINITY, 0.0];
        for (index, weight) in self.weights(point) {
            let c = self.columns[index];
            column[0] += weight * c[0];
            if weight > 0.0 && c[0] > 0.0 {
                column[1] = column[1].min(c[1]);
                column[2] = column[2].max(c[2]);
            }
        }
        if column[0] == 0.0 {
            column[1] = 0.0;
        }
        column
    }

    pub(crate) fn radius(&self) -> f64 {
        (self.size[0] as f64 + 1.0).hypot(self.size[1] as f64 + 1.0) * self.spacing * 0.5
    }

    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "indices are bounded before conversion"
    )]
    fn weights(&self, point: [f64; 2]) -> [(usize, f64); 4] {
        let p: [f64; 2] =
            std::array::from_fn(|i| point[i] / self.spacing + (self.size[i] as f64 - 1.0) * 0.5);
        let mut result = [(0, 0.0); 4];
        let mut slot = 0;
        for dz in 0..=1 {
            for dx in 0..=1 {
                let x = p[0].floor() + f64::from(dx);
                let z = p[1].floor() + f64::from(dz);
                if x >= 0.0 && z >= 0.0 && x < self.size[0] as f64 && z < self.size[1] as f64 {
                    let fx = p[0] - p[0].floor();
                    let fz = p[1] - p[1].floor();
                    result[slot] = (
                        z as usize * self.size[0] + x as usize,
                        if dx == 0 { 1.0 - fx } else { fx } * if dz == 0 { 1.0 - fz } else { fz },
                    );
                    slot += 1;
                }
            }
        }
        result
    }
}
