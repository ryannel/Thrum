mod swash;

use crate::{Bed, Stage, WaterError};
use std::collections::VecDeque;
use std::f64::consts::{FRAC_PI_4, TAU};

/// Source policy for a connected piece of still water.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// Edge-fed water, or the upwind shore of an enclosed piece.
    Water,
    /// A validated basin's own shore, even where a chart's phase continuation
    /// reaches the array edge. The basin's actual water remains enclosed.
    Enclosed,
    /// Edge sources alone; enclosed pieces deliberately remain unreachable.
    EdgeOnly,
}

/// Immutable content inputs for the shore transform of one still body.
#[derive(Clone, Copy, Debug)]
pub struct ShoreConfig {
    /// Base level in metres.
    pub level: f64,
    /// Declared offset range, sampled every half metre including both endpoints.
    pub stage: Stage,
    /// Maximum ground rise admitted to the swash, metres.
    pub margin: f64,
    /// Minimum dispersion depth, metres.
    pub floor: f64,
    /// Declared fetch in metres for enclosed water.
    pub fetch: f64,
    /// Optional seed restricting preparation to one connected hollow.
    pub seed: Option<[f64; 2]>,
    /// How a wave enters the water.
    pub source: Source,
}

/// Finite-depth wavenumber for a deep-water wavenumber and positive depth.
pub fn dispersion(k0: f64, depth: f64) -> f64 {
    if depth == f64::INFINITY {
        return k0;
    }
    let mut k = k0.max((k0 / depth).sqrt());
    for _ in 0..16 {
        let t = (k * depth).tanh();
        let step = (k * t - k0) / (t + k * depth * (1.0 - t * t));
        k -= step;
        if step.abs() < 1e-13 * k {
            break;
        }
    }
    k
}

/// Linear-theory height gain sqrt(deep group velocity / local group velocity).
pub fn shoaling(k0: f64, depth: f64) -> f64 {
    if depth == f64::INFINITY {
        return 1.0;
    }
    let k = dispersion(k0, depth);
    let kd = k * depth;
    let n = if kd < 300.0 {
        f64::midpoint(1.0, 2.0 * kd / (2.0 * kd).sinh())
    } else {
        0.5
    };
    (0.5 * k / (n * k0)).sqrt()
}

/// One solved lag in metres on a bed's lattice. Unreachable nodes are infinite.
pub struct Lag {
    values: Vec<f64>,
}

impl Lag {
    /// First-arrival solve for an individual wavelength, direction and stage.
    /// This also serves offline content tools that require an unsampled wavelength.
    /// # Errors
    /// Refuses invalid configuration, wavelength or direction.
    pub fn solve(
        bed: &Bed,
        config: ShoreConfig,
        wavelength: f64,
        direction: [f64; 2],
        stage: f64,
    ) -> Result<Self, WaterError> {
        validate(config)?;
        let norm = direction[0].hypot(direction[1]);
        if !wavelength.is_finite()
            || wavelength <= 0.0
            || !norm.is_finite()
            || norm <= 0.0
            || !stage.is_finite()
            || stage < config.stage.min
            || stage > config.stage.max
        {
            return Err(WaterError::new("shore lag", "invalid wave or stage"));
        }
        let direction = direction.map(|v| v / norm);
        let region = Region::new(bed, config, stage);
        Ok(solve(bed, config, &region, wavelength, direction, stage))
    }

    /// Prepared node values, x fastest, in metres.
    pub fn values(&self) -> &[f64] {
        &self.values
    }

    /// Bilinear lag and its exact cell gradient; zero beyond the bed.
    pub fn sample(&self, bed: &Bed, point: [f64; 2]) -> (f64, [f64; 2]) {
        let Some(w) = bed.weights(point) else {
            return (0.0, [0.0; 2]);
        };
        if w.iter()
            .any(|&(i, a)| a > 0.0 && !self.values[i].is_finite())
        {
            return (f64::INFINITY, [0.0; 2]);
        }
        let h = w.map(|(i, _)| self.values[i]);
        let u = w[1].1 + w[3].1;
        let v = w[2].1 + w[3].1;
        // Zero-weight unreachable corners must not turn a source node into NaN.
        let value = w
            .iter()
            .filter(|(_, a)| *a > 0.0)
            .map(|&(i, a)| a * self.values[i])
            .sum();
        let gradient = if h.iter().all(|h| h.is_finite()) {
            [
                ((h[1] - h[0]) * (1.0 - v) + (h[3] - h[2]) * v) / bed.spacing,
                ((h[2] - h[0]) * (1.0 - u) + (h[3] - h[1]) * u) / bed.spacing,
            ]
        } else {
            [0.0; 2]
        };
        (value, gradient)
    }
}

/// Prepared eight-direction, six-wavelength lags and sector exposures.
pub struct ShoreTransform {
    bed: Bed,
    config: ShoreConfig,
    stages: Vec<f64>,
    // stage, direction, wavelength, then row-major nodes.
    lags: Vec<Lag>,
    exposures: Vec<Vec<f64>>,
    contours: Vec<Vec<[f64; 3]>>,
}

impl ShoreTransform {
    /// Prepare all immutable lag and exposure content without a graphics device.
    /// # Errors
    /// Refuses invalid stage, fetch, margin or depth floor.
    pub fn prepare(bed: Bed, config: ShoreConfig) -> Result<Self, WaterError> {
        Self::prepare_with_joins(bed, config, &[])
    }

    pub(crate) fn prepare_with_joins(
        bed: Bed,
        config: ShoreConfig,
        joins: &[bool],
    ) -> Result<Self, WaterError> {
        validate(config)?;
        let mut stages = vec![config.stage.min];
        let mut stage = config.stage.min;
        while stage < config.stage.max {
            let next = (stage + 0.5).min(config.stage.max);
            if next <= stage {
                return Err(WaterError::new(
                    "shore",
                    "stage spacing is below numeric resolution",
                ));
            }
            stage = next;
            stages.push(stage);
        }
        let mut lags = Vec::new();
        let mut exposures = Vec::new();
        let mut contours = Vec::new();
        for &stage in &stages {
            let region = Region::new(&bed, config, stage);
            contours.push(swash::prepare(
                &bed,
                &region.open,
                joins,
                region.level,
                config.margin,
            ));
            for direction in 0..8 {
                let angle = f64::from(direction) * FRAC_PI_4;
                let dir = [angle.cos(), angle.sin()];
                let mut values: Vec<_> = (0..bed.heights.len())
                    .map(|i| {
                        if !region.open[i] || region.exposure_source[i].is_some_and(|j| j != i) {
                            return 0.0;
                        }
                        exposure(&bed, config, &region, i, angle)
                    })
                    .collect();
                for (i, source) in region.exposure_source.iter().copied().enumerate() {
                    if let Some(source) = source.filter(|&j| i != j) {
                        values[i] = values[source];
                    }
                }
                exposures.push(values);
                for rung in 0..6 {
                    lags.push(solve(
                        &bed,
                        config,
                        &region,
                        20.0 * 2.0_f64.powi(rung),
                        dir,
                        stage,
                    ));
                }
            }
        }
        Ok(Self {
            bed,
            config,
            stages,
            lags,
            exposures,
            contours,
        })
    }

    /// Declared stage samples, including both endpoints.
    pub fn stages(&self) -> &[f64] {
        &self.stages
    }

    /// Node values in metres, in stage, sector and wavelength order.
    /// An unreachable node has infinite lag.
    pub fn lag_planes(&self) -> impl ExactSizeIterator<Item = &[f64]> {
        self.lags.iter().map(Lag::values)
    }

    /// Node values in stage and sector order, each an arriving energy fraction.
    pub fn exposure_planes(&self) -> impl ExactSizeIterator<Item = &[f64]> {
        self.exposures.iter().map(Vec::as_slice)
    }

    /// Contour offsets and validity, in stage and row-major node order.
    pub fn contour_planes(&self) -> impl ExactSizeIterator<Item = &[[f64; 3]]> {
        self.contours.iter().map(Vec::as_slice)
    }

    pub(crate) fn contour(&self, stage: f64, point: [f64; 2]) -> Option<([f64; 2], f64)> {
        let corners = self.bed.weights(point)?;
        let (lo, t) = self.stage_blend(stage);
        let hi = (lo + 1).min(self.stages.len() - 1);
        let mut sum = [0.0; 3];
        for (stage, blend) in [(lo, 1.0 - t), (hi, t)] {
            for (i, weight) in corners {
                for (target, value) in sum.iter_mut().zip(self.contours[stage][i]) {
                    *target += weight * blend * value;
                }
            }
        }
        (sum[2] > 0.0).then(|| {
            let reference = std::array::from_fn(|axis| point[axis] + sum[axis] / sum[2]);
            (reference, sum[2])
        })
    }

    /// Number of lag layers (eight directions times six wavelengths times stages).
    pub fn layers(&self) -> usize {
        self.lags.len()
    }

    /// Retained sample payload, excluding allocator and struct overhead.
    pub fn bytes(&self) -> usize {
        (self.lags.len() + self.exposures.len() + 3 * self.contours.len())
            * self.bed.heights.len()
            * size_of::<f64>()
    }

    /// Lag and gradient blended in log wavelength and stage simultaneously.
    /// Direction is a lattice sector index modulo eight. Waves outside 20..640 m
    /// keep their deep-water phase. A stage is clamped to the declared range.
    pub fn sample(
        &self,
        direction: usize,
        wavelength: f64,
        stage: f64,
        point: [f64; 2],
    ) -> (f64, [f64; 2]) {
        if !(20.0..=640.0).contains(&wavelength) {
            return (0.0, [0.0; 2]);
        }
        let (s, t) = self.stage_blend(stage);
        let r = (wavelength / 20.0).log2();
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "bounded wavelength rung"
        )]
        let lo = (r.floor() as usize).min(4);
        let u = r - lo as f64;
        let mut out = (0.0, [0.0; 2]);
        for (si, sw) in [(s, 1.0 - t), ((s + 1).min(self.stages.len() - 1), t)] {
            for (ri, rw) in [(lo, 1.0 - u), (lo + 1, u)] {
                let weight = sw * rw;
                if weight == 0.0 {
                    continue;
                }
                let (lag, gradient) =
                    self.lags[(si * 8 + direction % 8) * 6 + ri].sample(&self.bed, point);
                out.0 += weight * lag;
                for (v, g) in out.1.iter_mut().zip(gradient) {
                    *v += weight * g;
                }
            }
        }
        out
    }

    /// Fraction of sector energy arriving here; stage interpolation matches the lag.
    pub fn exposure(&self, direction: usize, stage: f64, point: [f64; 2]) -> f64 {
        let Some(w) = self.bed.weights(point) else {
            return 1.0;
        };
        let (s, t) = self.stage_blend(stage);
        [(s, 1.0 - t), ((s + 1).min(self.stages.len() - 1), t)]
            .iter()
            .map(|&(si, sw)| {
                sw * w
                    .iter()
                    .map(|&(i, a)| a * self.exposures[si * 8 + direction % 8][i])
                    .sum::<f64>()
            })
            .sum()
    }

    /// Parameters used to prepare this content.
    pub fn config(&self) -> ShoreConfig {
        self.config
    }

    fn stage_blend(&self, stage: f64) -> (usize, f64) {
        let stage = stage.clamp(self.config.stage.min, self.config.stage.max);
        let s = self
            .stages
            .partition_point(|&v| v <= stage)
            .saturating_sub(1)
            .min(self.stages.len() - 1);
        let t = self
            .stages
            .get(s + 1)
            .map_or(0.0, |&hi| (stage - self.stages[s]) / (hi - self.stages[s]));
        (s, t)
    }
}

fn validate(c: ShoreConfig) -> Result<(), WaterError> {
    if [
        c.level,
        c.stage.min,
        c.stage.max,
        c.stage.value,
        c.margin,
        c.floor,
        c.fetch,
    ]
    .iter()
    .any(|x| !x.is_finite())
        || c.stage.min > c.stage.max
        || c.stage.value < c.stage.min
        || c.stage.value > c.stage.max
        || c.margin < 0.0
        || c.floor <= 0.0
        || c.fetch <= 0.0
        || c.seed.is_some_and(|s| s.iter().any(|v| !v.is_finite()))
    {
        return Err(WaterError::new("shore", "invalid shore configuration"));
    }
    Ok(())
}

struct Region {
    open: Vec<bool>,
    edge: Vec<bool>,
    exposure_source: Vec<Option<usize>>,
    level: f64,
}

impl Region {
    fn new(bed: &Bed, config: ShoreConfig, stage: f64) -> Self {
        let level = config.level + stage;
        let mut open: Vec<_> = bed
            .heights
            .iter()
            .map(|&h| h < level + config.margin)
            .collect();
        if let Some(seed) = config.seed {
            let mut seen = vec![false; open.len()];
            let mut q = VecDeque::new();
            if let Some(w) = bed.weights(seed) {
                for (i, _) in w {
                    if open[i] {
                        seen[i] = true;
                        q.push_back(i);
                    }
                }
            }
            while let Some(i) = q.pop_front() {
                for j in bed.neighbours(i) {
                    if open[j] && !seen[j] {
                        seen[j] = true;
                        q.push_back(j);
                    }
                }
            }
            open = seen;
        }
        let mut edge = vec![false; open.len()];
        let mut q = VecDeque::new();
        for i in 0..open.len() {
            if open[i] && bed.heights[i] < level && bed.edge(i) {
                edge[i] = true;
                q.push_back(i);
            }
        }
        while let Some(i) = q.pop_front() {
            for (j, weights) in bed.connections(i) {
                if open[j] && !edge[j] && bed.wet_reaches(i, weights, level) {
                    edge[j] = true;
                    q.push_back(j);
                }
            }
        }
        if config.source == Source::Enclosed {
            edge.fill(false);
        }
        // A rising sheet takes the incident power from its wet shore. The
        // dry margin cannot seed waves or make an inland basin sea-connected.
        let mut exposure_source = vec![None; open.len()];
        for (i, wet) in edge.iter().copied().enumerate() {
            if wet {
                exposure_source[i] = Some(i);
                q.push_back(i);
            }
        }
        while let Some(i) = q.pop_front() {
            for j in bed.neighbours(i) {
                if open[j]
                    && bed.heights[j] >= level
                    && bed.heights[j] >= bed.heights[i]
                    && exposure_source[j].is_none()
                {
                    exposure_source[j] = exposure_source[i];
                    q.push_back(j);
                }
            }
        }
        Self {
            open,
            edge,
            exposure_source,
            level,
        }
    }
}

fn ray(bed: &Bed, point: [f64; 2], angle: f64, level: f64, limit: f64) -> (f64, bool) {
    let step = bed.spacing * 0.5;
    let (sin, cos) = angle.sin_cos();
    let mut t = step;
    while t <= limit {
        match bed.height([point[0] - t * cos, point[1] - t * sin]) {
            None => return (t, true),
            Some(h) if h >= level => return ((t - step).max(0.0), false),
            _ => t += step,
        }
    }
    (limit, true)
}

// The finite bed joins the ambient sea at its boundary. Relax the
// shore transform over the same band for both travel time and energy.
fn bed_border_weight(bed: &Bed, i: usize) -> f64 {
    let border = bed.size[0].min(bed.size[1]) as f64 * bed.spacing / 8.0;
    let (x, z) = (i % bed.size[0], i / bed.size[0]);
    let distance = x.min(z).min(bed.size[0] - 1 - x).min(bed.size[1] - 1 - z) as f64 * bed.spacing;
    let weight = (distance / border).min(1.0);
    weight * weight * (3.0 - 2.0 * weight)
}

fn exposure(bed: &Bed, c: ShoreConfig, region: &Region, i: usize, angle: f64) -> f64 {
    let sea = region.edge[i];
    let weight = if sea { bed_border_weight(bed, i) } else { 1.0 };
    if weight == 0.0 {
        return 1.0;
    }
    let limit = if sea {
        bed.spacing * (bed.size[0] as f64).hypot(bed.size[1] as f64)
    } else {
        c.fetch
    };
    let energy = (0..17)
        .map(|s| {
            let a = angle + FRAC_PI_4 * (f64::from(s) / 16.0 - 0.5);
            let ray_level = if sea {
                c.level + c.stage.max
            } else {
                region.level
            };
            let (fetch, clear) = ray(bed, bed.point(i), a, ray_level, limit);
            if sea {
                f64::from(clear)
            } else {
                (fetch / c.fetch).min(1.0)
            }
        })
        .sum::<f64>()
        / 17.0;
    (1.0 - weight) + weight * energy
}

fn solve(
    bed: &Bed,
    c: ShoreConfig,
    region: &Region,
    wavelength: f64,
    dir: [f64; 2],
    stage: f64,
) -> Lag {
    let k0 = TAU / wavelength;
    let mut phase = vec![f64::INFINITY; bed.heights.len()];
    let mut fixed = vec![false; phase.len()];
    let mut speed = vec![0.0; phase.len()];
    for i in 0..phase.len() {
        if !region.open[i] {
            continue;
        }
        let p = bed.point(i);
        let depth = (c.level + stage - bed.heights[i]).max(c.floor);
        let seen = if region.edge[i] {
            depth + (wavelength.max(depth) - depth) * (1.0 - bed_border_weight(bed, i))
        } else {
            depth
        };
        speed[i] = dispersion(k0, seen) / k0;
        let lake_source = !region.edge[i]
            && c.source != Source::EdgeOnly
            && bed.heights[i] < c.level + stage
            && (0..9).all(|s| {
                ray(
                    bed,
                    p,
                    dir[1].atan2(dir[0]) + FRAC_PI_4 * (f64::from(s) / 8.0 - 0.5),
                    c.level + stage,
                    bed.spacing,
                )
                .0 < bed.spacing
            });
        if bed.edge(i) && c.source != Source::Enclosed || lake_source {
            phase[i] = dir[0] * p[0] + dir[1] * p[1];
            fixed[i] = true;
        }
    }
    sweep(bed.size, bed.spacing, &mut phase, &speed, &fixed);
    Lag {
        values: phase
            .into_iter()
            .enumerate()
            .map(|(i, v)| {
                let p = bed.point(i);
                v - dir[0] * p[0] - dir[1] * p[1]
            })
            .collect(),
    }
}

fn sweep(size: [usize; 2], h: f64, phase: &mut [f64], speed: &[f64], fixed: &[bool]) {
    let [nx, nz] = size;
    for _ in 0..12 {
        let mut changed = false;
        for order in 0..4 {
            for zz in 0..nz {
                let z = if order & 2 == 0 { zz } else { nz - 1 - zz };
                for xx in 0..nx {
                    let x = if order & 1 == 0 { xx } else { nx - 1 - xx };
                    let i = z * nx + x;
                    if fixed[i] || speed[i] == 0.0 {
                        continue;
                    }
                    let a = if x > 0 { phase[i - 1] } else { f64::INFINITY }.min(if x + 1 < nx {
                        phase[i + 1]
                    } else {
                        f64::INFINITY
                    });
                    let b = if z > 0 { phase[i - nx] } else { f64::INFINITY }.min(if z + 1 < nz {
                        phase[i + nx]
                    } else {
                        f64::INFINITY
                    });
                    let nh = speed[i] * h;
                    let v = if (a - b).abs() >= nh || !a.is_finite() || !b.is_finite() {
                        a.min(b) + nh
                    } else {
                        0.5 * (a + b + (2.0 * nh * nh - (a - b).powi(2)).sqrt())
                    };
                    if v < phase[i] - 1e-10 {
                        phase[i] = v;
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
}
