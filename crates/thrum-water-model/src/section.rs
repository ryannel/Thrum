use crate::WaterError;
use std::f64::consts::TAU;

/// One table entry, in units of the limited long-wave envelope.
#[derive(Clone, Copy, Debug)]
pub struct SectionPoint {
    /// Total section height, including the plain cosine.
    pub height: f64,
    /// Forward departure along the crest's local travel direction.
    pub throw: f64,
}

/// Validated periodic phase/index table. Each row has evenly spaced phases
/// on [0, 2π), with the seam implicit. Height is total; throw is a departure.
pub struct BreakerSection {
    indices: Vec<f64>,
    rows: Vec<Vec<SectionPoint>>,
    bore: f64,
}

impl BreakerSection {
    /// Admit a table only if no row can cross itself under its height shear.
    /// # Errors
    /// Rejects malformed, nonfinite or nonmonotone tables, any departure at
    /// index zero, and throw or changed extrema at and beyond the bore index.
    pub fn new(
        indices: Vec<f64>,
        rows: Vec<Vec<SectionPoint>>,
        bore: f64,
    ) -> Result<Self, WaterError> {
        let error = || WaterError::new("breaker section", "invalid section or height shear");
        if indices.len() < 2
            || rows.len() != indices.len()
            || indices[0] != 0.0
            || indices.iter().any(|v| !v.is_finite() || *v < 0.0)
            || indices.windows(2).any(|w| w[1] <= w[0])
            || !bore.is_finite()
            || bore <= 0.0
            || !indices.contains(&bore)
            || indices.last().is_none_or(|&last| last < bore)
            || rows[0].len() < 16
            || rows.iter().any(|r| r.len() != rows[0].len())
        {
            return Err(error());
        }
        for (index, row) in indices.iter().zip(&rows) {
            if row
                .iter()
                .any(|v| !v.height.is_finite() || !v.throw.is_finite() || v.throw < 0.0)
            {
                return Err(error());
            }
            for (i, v) in row.iter().enumerate() {
                if *index == 0.0
                    && ((v.height - (TAU * i as f64 / row.len() as f64).cos()).abs() > 1e-10
                        || v.throw != 0.0)
                {
                    return Err(error());
                }
                if *index >= bore && v.throw != 0.0 {
                    return Err(error());
                }
            }
            if *index >= bore {
                let lo = row.iter().map(|v| v.height).fold(f64::INFINITY, f64::min);
                let hi = row
                    .iter()
                    .map(|v| v.height)
                    .fold(f64::NEG_INFINITY, f64::max);
                if (lo + 1.0).abs() > 1e-8 || (hi - 1.0).abs() > 1e-8 {
                    return Err(error());
                }
            }
            let mut sorted = row.clone();
            sorted.sort_by(|a, b| a.height.total_cmp(&b.height));
            for pair in sorted.windows(2) {
                if pair[1].throw + 1e-9 < pair[0].throw
                    || ((pair[1].height - pair[0].height).abs() < 1e-10
                        && (pair[1].throw - pair[0].throw).abs() > 1e-8)
                {
                    return Err(error());
                }
            }
        }
        // The two branches at the same height must give the same throw, including
        // between phase knots. Inspect the index interpolation as well as its rows.
        for pair in rows.windows(2) {
            for i in 0..=16 {
                let t = f64::from(i) / 16.0;
                let row: Vec<_> = pair[0]
                    .iter()
                    .zip(&pair[1])
                    .map(|(&a, &b)| mix(a, b, t))
                    .collect();
                if !height_shear(&row) {
                    return Err(error());
                }
            }
        }
        Ok(Self {
            indices,
            rows,
            bore,
        })
    }

    /// First profile: a skewed face and a monotone shear, fading to a bore.
    /// # Panics
    /// Only if the built-in table violates its validation invariants.
    pub fn first() -> Self {
        let indices: Vec<f64> = vec![0.0, 0.7, 1.0, 1.2, 1.4, 2.0, 8.0];
        let rows = indices
            .iter()
            .map(|&b| {
                let m: f64 = b.min(1.0);
                let skew = 0.95 * m * m * (3.0 - 2.0 * m);
                let shear = if b <= 0.7 || b >= 1.4 {
                    0.0
                } else if b < 1.0 {
                    6.0 * (b - 0.7) / 0.3
                } else if b <= 1.2 {
                    6.0
                } else {
                    6.0 * (1.4 - b) / 0.2
                };
                (0..256)
                    .map(|i| {
                        let phase = TAU * f64::from(i) / 256.0;
                        let height = (phase + skew * phase.sin()).cos();
                        SectionPoint {
                            height,
                            throw: shear * height.max(0.0),
                        }
                    })
                    .collect()
            })
            .collect();
        Self::new(indices, rows, 1.4).expect("generated monotone height shear")
    }

    /// Bilinear table read, periodic in phase and clamped in breaking index.
    pub fn sample(&self, phase: f64, index: f64) -> SectionPoint {
        let i = self
            .indices
            .partition_point(|&b| b <= index)
            .saturating_sub(1)
            .min(self.indices.len() - 2);
        let u =
            ((index - self.indices[i]) / (self.indices[i + 1] - self.indices[i])).clamp(0.0, 1.0);
        let read = |row: &[SectionPoint]| {
            let s = phase.rem_euclid(TAU) / TAU * row.len() as f64;
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "phase reduced to row extent"
            )]
            let j = s.floor() as usize % row.len();
            mix(row[j], row[(j + 1) % row.len()], s - s.floor())
        };
        mix(read(&self.rows[i]), read(&self.rows[i + 1]), u)
    }

    /// Breaking index past which the profile carries a bore with no throw.
    pub fn bore_index(&self) -> f64 {
        self.bore
    }

    /// Breaker table rows in increasing index order, with uniform periodic phase samples.
    pub fn rows(&self) -> impl ExactSizeIterator<Item = (f64, &[SectionPoint])> {
        self.indices
            .iter()
            .copied()
            .zip(self.rows.iter().map(Vec::as_slice))
    }

    /// Global maximum forward throw in units of the envelope.
    pub fn maximum_throw(&self) -> f64 {
        self.rows
            .iter()
            .flatten()
            .map(|p| p.throw)
            .fold(0.0, f64::max)
    }

    pub(crate) fn throws_below(&self, index: f64) -> bool {
        self.indices.windows(2).enumerate().any(|(i, w)| {
            index > w[0] && self.rows[i..=i + 1].iter().flatten().any(|p| p.throw > 0.0)
        })
    }

    /// Conservative largest throw for every envelope up to `amplitude` at a depth.
    /// `ratio_depth` is the breaking ratio times the floored still depth.
    pub fn throw_ceiling(&self, amplitude: f64, ratio_depth: f64) -> f64 {
        self.indices
            .windows(2)
            .enumerate()
            .map(|(i, w)| {
                let lo = w[0] * ratio_depth * 0.5;
                if amplitude <= lo {
                    return 0.0;
                }
                let max_throw = self.rows[i..=i + 1]
                    .iter()
                    .flatten()
                    .map(|p| p.throw)
                    .fold(0.0, f64::max);
                amplitude.min(w[1] * ratio_depth * 0.5) * max_throw
            })
            .fold(0.0, f64::max)
    }
}

fn height_shear(row: &[SectionPoint]) -> bool {
    for &point in row {
        for i in 0..row.len() {
            let a = row[i];
            let b = row[(i + 1) % row.len()];
            let delta = b.height - a.height;
            if delta.abs() < 1e-12 {
                if (a.throw - b.throw).abs() > 1e-8 {
                    return false;
                }
                continue;
            }
            let t = (point.height - a.height) / delta;
            if (0.0..=1.0).contains(&t) && (mix(a, b, t).throw - point.throw).abs() > 1e-8 {
                return false;
            }
        }
    }
    let mut sorted = row.to_vec();
    sorted.sort_by(|a, b| a.height.total_cmp(&b.height));
    sorted.windows(2).all(|p| p[0].throw <= p[1].throw + 1e-9)
}

fn mix(a: SectionPoint, b: SectionPoint, u: f64) -> SectionPoint {
    SectionPoint {
        height: a.height * (1.0 - u) + b.height * u,
        throw: a.throw * (1.0 - u) + b.throw * u,
    }
}
