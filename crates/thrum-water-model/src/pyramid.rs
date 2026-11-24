use crate::Bed;

/// Prepared min/max bed hierarchy, with exact leaf extrema. Rectangle queries
/// include the neighbouring centres needed by bilinear interpolation.
pub struct BedPyramid {
    origin: [f64; 2],
    spacing: f64,
    levels: Vec<Level>,
}

struct Level {
    size: [usize; 2],
    bounds: Vec<[f64; 2]>,
}

impl BedPyramid {
    /// Prepare the bed's min/max pyramid once with the immutable lag content.
    pub fn new(bed: &Bed) -> Self {
        let mut levels = vec![Level {
            size: bed.size,
            bounds: bed.heights.iter().map(|&h| [h, h]).collect(),
        }];
        while levels[levels.len() - 1].size != [1, 1] {
            let prev = &levels[levels.len() - 1];
            let size = prev.size.map(|n| n.div_ceil(2));
            let mut bounds = Vec::with_capacity(size[0] * size[1]);
            for z in 0..size[1] {
                for x in 0..size[0] {
                    let mut bound = [f64::INFINITY, f64::NEG_INFINITY];
                    for dz in 0..2 {
                        for dx in 0..2 {
                            let (px, pz) = (2 * x + dx, 2 * z + dz);
                            if px < prev.size[0] && pz < prev.size[1] {
                                let b = prev.bounds[pz * prev.size[0] + px];
                                bound[0] = bound[0].min(b[0]);
                                bound[1] = bound[1].max(b[1]);
                            }
                        }
                    }
                    bounds.push(bound);
                }
            }
            levels.push(Level { size, bounds });
        }
        Self {
            origin: bed.origin,
            spacing: bed.spacing,
            levels,
        }
    }

    /// Retained payload bytes, including this hierarchy's duplicated leaf extrema.
    pub fn bytes(&self) -> usize {
        self.levels
            .iter()
            .map(|l| l.bounds.len() * size_of::<[f64; 2]>())
            .sum()
    }

    /// Row-major extrema by level, from the bed lattice to its root.
    pub fn levels(&self) -> impl ExactSizeIterator<Item = ([usize; 2], &[[f64; 2]])> {
        self.levels
            .iter()
            .map(|level| (level.size, level.bounds.as_slice()))
    }

    /// Conservative height interval in a world-space square. None off the bed.
    pub fn range(&self, point: [f64; 2], radius: f64) -> Option<[f64; 2]> {
        if !radius.is_finite() || radius < 0.0 || point.iter().any(|v| !v.is_finite()) {
            return None;
        }
        let size = self.levels[0].size;
        let lo: [f64; 2] =
            std::array::from_fn(|a| ((point[a] - radius - self.origin[a]) / self.spacing).floor());
        let hi: [f64; 2] =
            std::array::from_fn(|a| ((point[a] + radius - self.origin[a]) / self.spacing).ceil());
        if (0..2).any(|a| hi[a] < 0.0 || lo[a] > (size[a] - 1) as f64) {
            return None;
        }
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "rectangle clamped to allocated grid"
        )]
        let rect = [
            std::array::from_fn(|a| lo[a].max(0.0) as usize),
            std::array::from_fn(|a| hi[a].min((size[a] - 1) as f64) as usize),
        ];
        let mut bound = [f64::INFINITY, f64::NEG_INFINITY];
        self.visit(self.levels.len() - 1, [0, 0], rect, &mut bound);
        Some(bound)
    }

    fn visit(&self, level: usize, cell: [usize; 2], rect: [[usize; 2]; 2], out: &mut [f64; 2]) {
        let scale = 1 << level;
        let lo = cell.map(|p| p * scale);
        let hi = lo.map(|p| p + scale - 1);
        if (0..2).any(|a| lo[a] > rect[1][a] || hi[a] < rect[0][a]) {
            return;
        }
        let l = &self.levels[level];
        if level == 0 || (0..2).all(|a| lo[a] >= rect[0][a] && hi[a] <= rect[1][a]) {
            let b = l.bounds[cell[1] * l.size[0] + cell[0]];
            out[0] = out[0].min(b[0]);
            out[1] = out[1].max(b[1]);
        } else {
            let next = &self.levels[level - 1];
            for dz in 0..2 {
                for dx in 0..2 {
                    let child = [2 * cell[0] + dx, 2 * cell[1] + dz];
                    if (0..2).all(|a| child[a] < next.size[a]) {
                        self.visit(level - 1, child, rect, out);
                    }
                }
            }
        }
    }
}
