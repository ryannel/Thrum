//! Rain in the one water ([ruling 3 for `PUDDLES`]): two rain bands, wave layers beside
//! the wind's that are a function of time and of the rain's committed rate over the last
//! second, and keep nothing of their own between evaluations. Drops fall on a schedule
//! drawn from a hash of place and time; a scheduled drop falls if the rate at the tick it
//! lands passes its threshold. A point's height and slopes are the sum of the rings of
//! the drops that landed within reach in the last second, each read from a prepared table
//! of a ring's height by radius and age. The drops below the schedule's cut are not drawn:
//! their squared slope goes into the bands' moments over the same history.
//!
//! The ring tables are data a rain model makes; this crate takes them as data. Every
//! body's row has a rain gain: the exposure at its surface times the depth response.
//!
//! Also here: a body's wave support at a point, which `WATERS`' film reads, so that wet
//! ground on a shore shares its edge.
//!
//! [ruling 3 for `PUDDLES`]: ../../../docs/plans/ARCHITECTURE.md
use crate::{Field, Map, MapSample, WaterError};
use std::sync::Arc;
use thrum_sea_model::Sample;

/// Where a body's waves reach at a point: still water and the shore its swash washes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Support {
    /// The body whose row the point takes.
    pub body: usize,
    /// Still depth: negative above the still waterline, infinite off the bed.
    pub depth: f64,
    /// The swash depth the body's waves give here.
    pub swash_depth: f64,
}

impl Support {
    /// Whether the point is inside the body's wave support: depth above minus the swash
    /// depth.
    pub fn inside(&self) -> bool {
        self.depth > -self.swash_depth
    }

    /// Height of the point above the support's edge, metres; none or less inside it.
    pub fn above(&self) -> f64 {
        -self.depth - self.swash_depth
    }
}

impl Map {
    /// A body's wave support at a point, from the same depth limit the surface reads.
    pub fn wave_support(&self, field: &Field, point: [f64; 2]) -> Option<Support> {
        let s = self.sample(point)?;
        let lim = self.limited(field, point, s).0;
        Some(Support {
            body: s.body,
            depth: s.depth,
            swash_depth: lim.swash_depth,
        })
    }
}

/// The share of the rain that reaches a point (x, y, z), from none under a roof to all
/// under open sky. Shelter is content: it reads no camera and keeps no state.
pub trait Exposure: Send + Sync {
    /// Prepared canopy columns, when this exposure can also run on the GPU.
    fn canopy_grid(&self) -> Option<CanopyGrid> {
        None
    }

    /// Exposure at a world point.
    fn exposure(&self, point: [f64; 3]) -> f64;
}

/// Prepared canopy columns. Each column holds two (underside, transmission) pairs.
#[derive(Clone, Debug, Default)]
pub struct CanopyGrid {
    /// Corner of the first column, in world metres.
    pub origin: [f64; 2],
    /// Column width in metres.
    pub cell: f64,
    /// Columns in x and z.
    pub size: [usize; 2],
    /// Width of the square over which exposure is averaged, in metres.
    pub edge_width: f64,
    /// Row-major columns, ordered x then z.
    pub columns: Vec<[[f64; 2]; 2]>,
}

/// Open sky everywhere.
#[derive(Clone, Copy, Debug, Default)]
pub struct OpenSky;

impl Exposure for OpenSky {
    fn canopy_grid(&self) -> Option<CanopyGrid> {
        Some(CanopyGrid {
            cell: 1.0,
            ..CanopyGrid::default()
        })
    }

    fn exposure(&self, _: [f64; 3]) -> f64 {
        1.0
    }
}

/// One size class of scheduled drop: its share of the schedule and its ring on deep
/// water, row-major by age then radius.
#[derive(Clone, Debug, PartialEq)]
pub struct RingClass {
    /// Share of scheduled drops of this class; the classes' shares sum to 1.
    pub share: f64,
    /// Height, metres.
    pub height: Vec<f64>,
    /// Radial slope, dh/dr.
    pub slope: Vec<f64>,
    /// Height's rate, dh/dt, metres a second.
    pub rate: Vec<f64>,
    /// The ring's squared height over the whole water at each age, m⁴.
    pub variance: Vec<f64>,
}

/// The prepared tables the rain bands read.
#[derive(Clone, Debug, PartialEq)]
pub struct RingTable {
    /// Spacing of the radii, metres; the first radius is 0.
    pub radius_step: f64,
    /// Radii in each row.
    pub radii: usize,
    /// Spacing of the ages, seconds; the first age is 0 and the last a ring's life.
    pub age_step: f64,
    /// Ages: rows.
    pub ages: usize,
    /// The scheduled drops' classes.
    pub classes: Vec<RingClass>,
    /// A ring's height on a depth over its height on deep water: (depth, ratio),
    /// ascending by depth, read linearly and held past the last.
    pub depth_response: Vec<[f64; 2]>,
    /// Share of the profile's count of scheduled drops that falls at a rain flux:
    /// (flux, share), ascending by flux, read linearly and held past the last.
    pub scheduled: Vec<[f64; 2]>,
    /// Steady squared slope of the drops below the cut at a rain flux: (flux, variance).
    pub moments: Vec<[f64; 2]>,
    /// How a drop's squared slope spreads over its age, at each age of the table; its
    /// sum times the age step is 1.
    pub moment_age: Vec<f64>,
}

impl RingTable {
    /// A ring's life, seconds.
    pub fn life(&self) -> f64 {
        self.age_step * (self.ages - 1) as f64
    }

    /// The furthest a ring reaches, metres.
    pub fn reach(&self) -> f64 {
        self.radius_step * (self.radii - 1) as f64
    }

    /// The depth response at a still depth: none on dry ground.
    pub fn response(&self, depth: f64) -> f64 {
        if depth.is_nan() || depth <= 0.0 {
            return 0.0;
        }
        curve(&self.depth_response, depth)
    }

    /// The share of the schedule that falls at a flux.
    pub fn scheduled_share(&self, flux: f64) -> f64 {
        if flux <= 0.0 {
            0.0
        } else {
            curve(&self.scheduled, flux).clamp(0.0, 1.0)
        }
    }

    /// One class's ring at a radius and age: height, radial slope and rate.
    pub fn ring(&self, class: usize, radius: f64, age: f64) -> [f64; 3] {
        if !(0.0..self.life()).contains(&age) || radius >= self.reach() || radius < 0.0 {
            return [0.0; 3];
        }
        let c = &self.classes[class];
        let (fa, fr) = (age / self.age_step, radius / self.radius_step);
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "indices of a nonnegative age and radius inside the table"
        )]
        let (ia, ir) = (
            (fa.floor() as usize).min(self.ages - 2),
            (fr.floor() as usize).min(self.radii - 2),
        );
        let (wa, wr) = (fa - ia as f64, fr - ir as f64);
        let at = |v: &[f64]| {
            let row = |a: usize| {
                let k = a * self.radii + ir;
                v[k] + (v[k + 1] - v[k]) * wr
            };
            row(ia) + (row(ia + 1) - row(ia)) * wa
        };
        [at(&c.height), at(&c.slope), at(&c.rate)]
    }

    fn validate(&self) -> Result<(), WaterError> {
        let bad = |r| Err(WaterError::new("rain bands", r));
        let n = self.ages * self.radii;
        if self.ages < 2
            || self.radii < 2
            || !(self.age_step > 0.0 && self.radius_step > 0.0)
            || self.classes.is_empty()
            || self.moment_age.len() != self.ages
        {
            return bad("malformed ring table");
        }
        for c in &self.classes {
            if [&c.height, &c.slope, &c.rate].iter().any(|v| v.len() != n)
                || c.variance.len() != self.ages
                || c.share.is_nan()
                || c.share < 0.0
            {
                return bad("malformed ring class");
            }
        }
        let share: f64 = self.classes.iter().map(|c| c.share).sum();
        if (share - 1.0).abs() > 1e-9 {
            return bad("ring classes' shares do not sum to one");
        }
        for t in [&self.depth_response, &self.scheduled, &self.moments] {
            if t.is_empty() || t.windows(2).any(|w| w[1][0] <= w[0][0]) {
                return bad("curve not ascending");
            }
        }
        Ok(())
    }
}

/// A curve read linearly between its points and held past either end.
fn curve(points: &[[f64; 2]], x: f64) -> f64 {
    let k = points.partition_point(|p| p[0] <= x);
    if k == 0 {
        return points[0][1];
    }
    if k == points.len() {
        return points[k - 1][1];
    }
    let ([x0, y0], [x1, y1]) = (points[k - 1], points[k]);
    y0 + (y1 - y0) * (x - x0) / (x1 - x0)
}

/// The rain's committed rate over the last second: the flux of each tick, kilograms a
/// square metre a second, committed with the tick.
#[derive(Clone, Debug, PartialEq)]
pub struct RateHistory {
    /// A tick's length, seconds; tick `j` covers `[j, j + 1)` times it.
    pub tick: f64,
    /// The latest committed tick.
    pub last: i64,
    /// Fluxes of the ticks ending at `last`, oldest first.
    pub rates: Vec<f64>,
}

impl RateHistory {
    /// A history of no rain.
    pub fn dry(tick: f64, last: i64, len: usize) -> Self {
        Self {
            tick,
            last,
            rates: vec![0.0; len],
        }
    }

    /// Commit the next tick's flux, dropping the oldest.
    pub fn commit(&mut self, flux: f64) {
        if !self.rates.is_empty() {
            self.rates.remove(0);
            self.rates.push(flux);
        }
        self.last += 1;
    }

    /// The flux at a tick: none before the history, and the latest after it.
    pub fn rate(&self, tick: i64) -> f64 {
        let first = self.last - i64::try_from(self.rates.len()).unwrap_or(i64::MAX) + 1;
        if tick < first || self.rates.is_empty() {
            0.0
        } else if tick >= self.last {
            self.rates[self.rates.len() - 1]
        } else {
            #[expect(
                clippy::cast_sign_loss,
                clippy::cast_possible_truncation,
                reason = "an offset inside the history"
            )]
            let k = (tick - first) as usize;
            self.rates[k]
        }
    }

    /// Whether any tick of the history holds rain.
    pub fn raining(&self) -> bool {
        self.rates.iter().any(|&r| r > 0.0)
    }

    /// The tick a time falls in.
    pub fn tick_of(&self, time: f64) -> i64 {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "a tick index of a finite time"
        )]
        let j = (time / self.tick).floor() as i64;
        j
    }
}

/// The rain bands' physical profile. Never a quality setting.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RainBandsProfile {
    /// The two periodic patches' sides, metres.
    pub patches: [f64; 2],
    /// Scheduled drops a square metre a second at full share, over both bands.
    pub count: f64,
    /// Schedule cells along a patch's side.
    pub cells: usize,
    /// Length of a schedule bin, seconds.
    pub bin: f64,
    /// Width of the soft threshold on the scheduled share.
    pub soft: f64,
    /// Seed of the schedule's hash.
    pub seed: u64,
}

impl Default for RainBandsProfile {
    fn default() -> Self {
        Self {
            patches: [6.4, 6.4 * 1.25_f64.sqrt()],
            count: 128.0,
            cells: 8,
            bin: 0.125,
            soft: 0.02,
            seed: 0x7261_696e,
        }
    }
}

/// One scheduled drop.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drop {
    /// Its band.
    pub band: usize,
    /// Where it lands in its patch, metres from the patch's corner.
    pub place: [f64; 2],
    /// When it lands, seconds.
    pub time: f64,
    /// The tick it lands in, whose rate decides its birth.
    pub tick: i64,
    /// Its size class.
    pub class: usize,
    /// Its weight: 1 for a drop that falls, 0 for one that does not, between on the
    /// soft threshold.
    pub weight: f64,
}

/// The two rain bands: a function of the display's time and the committed rate history.
#[derive(Clone)]
pub struct RainBands {
    profile: RainBandsProfile,
    table: Arc<RingTable>,
    ring_means: Arc<[Vec<[f64; 2]>]>,
    history: RateHistory,
    exposure: Arc<dyn Exposure>,
}

impl std::fmt::Debug for RainBands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RainBands")
            .field("profile", &self.profile)
            .field("history", &self.history)
            .finish_non_exhaustive()
    }
}

const PHI: u64 = 0x9e37_79b9_7f4a_7c15;

fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

fn hash(parts: &[u64]) -> u64 {
    parts.iter().fold(PHI, |h, &p| mix(h ^ p.wrapping_add(PHI)))
}

#[expect(
    clippy::cast_precision_loss,
    reason = "53 random bits as a uniform value in [0, 1)"
)]
fn uniform(h: u64) -> f64 {
    (h >> 11) as f64 / (1u64 << 53) as f64
}

#[expect(clippy::cast_sign_loss, reason = "a signed index's bits for a hash")]
fn bits(v: i64) -> u64 {
    v as u64
}

impl RainBands {
    /// Rain bands over a history, sheltered by an exposure.
    /// # Errors
    /// Refuses a malformed table or profile, a ring that reaches past half a patch or a
    /// schedule cell that would count a drop twice, and a history shorter than a ring's
    /// life.
    pub fn new(
        profile: RainBandsProfile,
        table: Arc<RingTable>,
        history: RateHistory,
        exposure: Arc<dyn Exposure>,
    ) -> Result<Self, WaterError> {
        table.validate()?;
        let bad = |r| Err(WaterError::new("rain bands", r));
        if !profile.patches.iter().all(|p| p.is_finite() && *p > 0.0)
            || !profile.count.is_finite()
            || profile.count < 0.0
            || profile.cells == 0
            || i64::try_from(profile.cells).is_err()
            || !profile.bin.is_finite()
            || profile.bin <= 0.0
            || !profile.soft.is_finite()
            || profile.soft <= 0.0
        {
            return bad("invalid profile");
        }
        for &l in &profile.patches {
            let cell = l / profile.cells as f64;
            let span = (table.reach() / cell).ceil();
            if table.reach() >= 0.5 * l || 2.0 * span + 1.0 > profile.cells as f64 {
                return bad("a ring reaches past its patch's schedule");
            }
        }
        let out = Self {
            profile,
            ring_means: table
                .classes
                .iter()
                .map(|class| {
                    class
                        .height
                        .chunks_exact(table.radii)
                        .zip(class.rate.chunks_exact(table.radii))
                        .map(|(height, rate)| {
                            let integrate = |row: &[f64]| {
                                row.windows(2)
                                    .enumerate()
                                    .map(|(i, ends)| {
                                        // Integral of the linear table segment times radius.
                                        let i = i as f64;
                                        (i / 2.0 + 1.0 / 6.0) * ends[0]
                                            + (i / 2.0 + 1.0 / 3.0) * ends[1]
                                    })
                                    .sum::<f64>()
                                    * std::f64::consts::TAU
                                    * table.radius_step.powi(2)
                            };
                            [integrate(height), integrate(rate)]
                        })
                        .collect()
                })
                .collect(),
            table,
            history: RateHistory::dry(1.0, 0, 0),
            exposure,
        };
        let mut out = out;
        out.set_history(history)?;
        Ok(out)
    }

    /// Commit a new history with the tick.
    /// # Errors
    /// Refuses a history shorter than a ring's life, or with a negative or nonfinite rate.
    pub fn set_history(&mut self, history: RateHistory) -> Result<(), WaterError> {
        if !(history.tick.is_finite() && history.tick > 0.0)
            || history.rates.iter().any(|r| !r.is_finite() || *r < 0.0)
            || (history.rates.len() as f64) * history.tick < self.table.life() + history.tick
        {
            return Err(WaterError::new(
                "rain bands",
                "invalid history or one shorter than a ring's life",
            ));
        }
        self.history = history;
        Ok(())
    }

    /// The committed history.
    pub fn history(&self) -> &RateHistory {
        &self.history
    }

    /// The prepared tables.
    pub fn table(&self) -> &RingTable {
        &self.table
    }

    /// The profile.
    pub fn profile(&self) -> RainBandsProfile {
        self.profile
    }

    /// The prepared columns for evaluating exposure in a shader.
    pub fn canopy_grid(&self) -> Option<CanopyGrid> {
        self.exposure.canopy_grid()
    }

    /// Whether a tick within a ring's lifetime held rain at this display time.
    pub fn active(&self, time: f64) -> bool {
        let history = &self.history;
        (history.tick_of(time - self.table.life())..=history.tick_of(time))
            .any(|tick| history.rate(tick) > 0.0)
    }

    /// Exposure at a world point.
    pub fn exposure(&self, point: [f64; 3]) -> f64 {
        self.exposure.exposure(point)
    }

    /// Weight of a scheduled drop with threshold `u` at a flux: none at no rain, all at
    /// full share, soft between.
    fn weight(&self, flux: f64, u: f64) -> f64 {
        if flux <= 0.0 {
            return 0.0;
        }
        let f = self.table.scheduled_share(flux);
        let s = self.profile.soft;
        ((f * (1.0 + s) - u) / s).clamp(0.0, 1.0)
    }

    /// Drops scheduled in one cell and bin of a band, landed or not.
    fn cell_drops(&self, band: usize, cell: [i64; 2], bin: i64, out: &mut Vec<Drop>) {
        let p = &self.profile;
        let n = i64::try_from(p.cells).unwrap_or(i64::MAX);
        let side = p.patches[band] / p.cells as f64;
        let (ci, cj) = (cell[0].rem_euclid(n), cell[1].rem_euclid(n));
        let key = [p.seed, band as u64, bits(ci), bits(cj), bits(bin)];
        let lambda = 0.5 * p.count * side * side * p.bin;
        // A Poisson count by inversion of one uniform value.
        let u = uniform(hash(&key));
        let (mut k, mut term) = (0_u64, (-lambda).exp());
        let mut sum = term;
        while u > sum && k < 64 {
            k += 1;
            term *= lambda / k as f64;
            sum += term;
        }
        for m in 0..k {
            let h = |s: u64| uniform(hash(&[key[0], key[1], key[2], key[3], key[4], m, s]));
            let time = (bin as f64 + h(1)) * p.bin;
            let tick = self.history.tick_of(time);
            let mut pick = h(3);
            let mut class = self.table.classes.len() - 1;
            for (c, ring) in self.table.classes.iter().enumerate() {
                if pick < ring.share {
                    class = c;
                    break;
                }
                pick -= ring.share;
            }
            out.push(Drop {
                band,
                place: [
                    (cell[0] as f64 + h(4)) * side,
                    (cell[1] as f64 + h(5)) * side,
                ],
                time,
                tick,
                class,
                weight: self.weight(self.history.rate(tick), h(2)),
            });
        }
    }

    /// Every drop of a band whose ring is alive at a time, over its whole patch, with
    /// places inside the patch.
    pub fn drops(&self, band: usize, time: f64) -> Vec<Drop> {
        let mut out = Vec::new();
        self.drops_into(band, time, &mut out);
        out
    }

    /// Write the live schedule into caller-owned storage, reusing its capacity.
    pub fn drops_into(&self, band: usize, time: f64, out: &mut Vec<Drop>) {
        out.clear();
        let n = i64::try_from(self.profile.cells).unwrap_or(i64::MAX);
        let life = self.table.life();
        let (b0, b1) = self.bins(time - life, time);
        for bin in b0..=b1 {
            for cj in 0..n {
                for ci in 0..n {
                    self.cell_drops(band, [ci, cj], bin, out);
                }
            }
        }
        out.retain(|d| d.time <= time && d.time > time - life);
    }

    fn bins(&self, from: f64, to: f64) -> (i64, i64) {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "bin indices of finite times"
        )]
        let b = |t: f64| (t / self.profile.bin).floor() as i64;
        (b(from), b(to))
    }

    /// The two bands at unit gain: height, slopes and height's rate at a world point and
    /// time.
    pub fn sample(&self, point: [f64; 2], time: f64) -> Sample {
        self.sample_footprint(point, time, 0.0)
    }

    /// Filter height, slope and height rate over a column's footprint in metres.
    /// At a complete patch's mip the mean slope is exactly zero.
    pub fn sample_footprint(&self, point: [f64; 2], time: f64, radius: f64) -> Sample {
        let mut out = Sample::default();
        if !self.active(time) {
            // No tick of the last second held rain: the bands are exactly nothing.
            return out;
        }
        let mut drops = Vec::new();
        // Once a footprint spans many ring fronts, sparse midpoint samples
        // alias their slopes. Blend the remaining resolved detail into the
        // patch mean over a physical distance, independent of the period.
        let reach = self.table.reach();
        let broad = ((radius / reach - 2.0) * 0.5).clamp(0.0, 1.0);
        for band in 0..2 {
            if broad > 0.0 {
                accumulate(&mut out, self.patch_mean(band, time), broad);
            }
            if broad >= 1.0 {
                continue;
            }
            let length = self.profile.patches[band];
            let mip = (radius * 512.0 / length).max(1.0).log2().min(9.0);
            let lower = mip.floor();
            for (level, weight) in [(lower, 1.0 - mip.fract()), (lower + 1.0, mip.fract())] {
                if weight == 0.0 {
                    continue;
                }
                let sample = if level >= 9.0 {
                    self.patch_mean(band, time)
                } else if level == 0.0 {
                    self.sample_band(band, point, time, &mut drops)
                } else {
                    // Integrate the square mip footprint by midpoint quadrature.
                    // Four subdivisions along a side track the resolved crests of
                    // small footprints. Larger mips resolve successively less detail.
                    let extent = length * level.exp2() / 512.0;
                    let mut sample = Sample::default();
                    for z in 0..4 {
                        for x in 0..4 {
                            let offset = |i| extent * (f64::from(i) - 1.5) / 4.0;
                            let p = [point[0] + offset(x), point[1] + offset(z)];
                            accumulate(
                                &mut sample,
                                self.sample_band(band, p, time, &mut drops),
                                1.0 / 16.0,
                            );
                        }
                    }
                    sample
                };
                accumulate(&mut out, sample, (1.0 - broad) * weight);
            }
        }
        out
    }

    fn patch_mean(&self, band: usize, time: f64) -> Sample {
        let mut sample = Sample::default();
        let area = self.profile.patches[band].powi(2);
        for drop in self
            .drops(band, time)
            .into_iter()
            .filter(|drop| drop.weight > 0.0)
        {
            let f = (time - drop.time) / self.table.age_step;
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "live drop age indexes the ring table"
            )]
            let lower = (f.floor() as usize).min(self.table.ages - 2);
            let [a, b] = [
                self.ring_means[drop.class][lower],
                self.ring_means[drop.class][lower + 1],
            ];
            let w = f - lower as f64;
            sample.height += drop.weight * (a[0] + w * (b[0] - a[0])) / area;
            sample.height_dt += drop.weight * (a[1] + w * (b[1] - a[1])) / area;
        }
        sample
    }

    fn sample_band(
        &self,
        band: usize,
        point: [f64; 2],
        time: f64,
        drops: &mut Vec<Drop>,
    ) -> Sample {
        let mut out = Sample::default();
        let life = self.table.life();
        let reach = self.table.reach();
        let (b0, b1) = self.bins(time - life, time);
        let l = self.profile.patches[band];
        let side = l / self.profile.cells as f64;
        let u = [point[0].rem_euclid(l), point[1].rem_euclid(l)];
        #[expect(clippy::cast_possible_truncation, reason = "cell indices in a patch")]
        let (ci, cj, span) = (
            (u[0] / side).floor() as i64,
            (u[1] / side).floor() as i64,
            (reach / side).ceil() as i64,
        );
        for dj in -span..=span {
            for di in -span..=span {
                for bin in b0..=b1 {
                    drops.clear();
                    self.cell_drops(band, [ci + di, cj + dj], bin, drops);
                    for d in &*drops {
                        let age = time - d.time;
                        if d.weight <= 0.0 || age < 0.0 || age >= life {
                            continue;
                        }
                        let v = [u[0] - d.place[0], u[1] - d.place[1]];
                        let r = v[0].hypot(v[1]);
                        if r >= reach {
                            continue;
                        }
                        let [h, dr, dt] = self.table.ring(d.class, r, age);
                        out.height += d.weight * h;
                        out.height_dt += d.weight * dt;
                        if r > 0.0 {
                            out.slope[0] += d.weight * dr * v[0] / r;
                            out.slope[1] += d.weight * dr * v[1] / r;
                        }
                    }
                }
            }
        }
        out
    }

    /// Sum over the ticks of the last second of a value of each tick's flux, weighted by
    /// a function of the tick's age.
    fn over_history(
        &self,
        time: f64,
        value: impl Fn(f64) -> f64,
        by_age: impl Fn(f64) -> f64,
    ) -> f64 {
        let h = &self.history;
        let life = self.table.life();
        let (from, to) = (h.tick_of(time - life), h.tick_of(time));
        let mut sum = 0.0;
        for j in from..=to {
            let rate = h.rate(j);
            if rate <= 0.0 {
                continue;
            }
            let age = time - (j as f64 + 0.5) * h.tick;
            if (0.0..life).contains(&age) {
                sum += value(rate) * by_age(age) * h.tick;
            }
        }
        sum
    }

    fn at_age(&self, v: &[f64], age: f64) -> f64 {
        let f = age / self.table.age_step;
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "an index of a nonnegative age inside the table"
        )]
        let i = (f.floor() as usize).min(self.table.ages - 2);
        let w = f - i as f64;
        v[i] + (v[i + 1] - v[i]) * w
    }

    /// The squared slope the drops below the cut add to the bands' moments at unit gain,
    /// summed over the history with a ring's decay. Exactly nothing a second after the
    /// last rain.
    pub fn moments(&self, time: f64) -> f64 {
        self.over_history(
            time,
            |rate| curve(&self.table.moments, rate),
            |age| self.at_age(&self.table.moment_age, age),
        )
    }

    /// The bands' height variance at unit gain: each tick's scheduled drops' rings,
    /// squared over the water, at their age.
    pub fn variance(&self, time: f64) -> f64 {
        let c = &self.table.classes;
        self.over_history(
            time,
            |rate| self.profile.count * self.table.scheduled_share(rate),
            |age| {
                c.iter()
                    .map(|k| k.share * self.at_age(&k.variance, age))
                    .sum()
            },
        )
    }
}

fn accumulate(out: &mut Sample, sample: Sample, weight: f64) {
    out.height += weight * sample.height;
    out.height_dt += weight * sample.height_dt;
    for (slope, delta) in out.slope.iter_mut().zip(sample.slope) {
        *slope += weight * delta;
    }
}

impl Map {
    /// Give the map rain bands, or take them away. With none, every answer is what it was.
    pub fn set_rain(&mut self, rain: Option<RainBands>) {
        self.rain = rain;
    }

    /// The map's rain bands.
    pub fn rain(&self) -> Option<&RainBands> {
        self.rain.as_ref()
    }

    /// Commit the rain's rate history with the tick.
    /// # Errors
    /// Refuses a map with no rain bands, and a history the bands refuse.
    pub fn set_rate_history(&mut self, history: RateHistory) -> Result<(), WaterError> {
        self.rain
            .as_mut()
            .ok_or(WaterError::new("rain bands", "the map has no rain bands"))?
            .set_history(history)
    }

    /// The rain gain at a point on a row: exposure at the water's surface times the depth
    /// response, within the depth limit a wave layer has. None with no rain bands.
    pub fn rain_gain(&self, field: &Field, point: [f64; 2], time: f64, map: MapSample) -> f64 {
        self.rain.as_ref().map_or(0.0, |_| {
            self.rain_gain_with_variances(field.variances(), point, time, map)
        })
    }

    /// Rain gain using precomputed wave variances.
    pub fn rain_gain_with_variances(
        &self,
        variances: [f64; 4],
        point: [f64; 2],
        time: f64,
        map: MapSample,
    ) -> f64 {
        let Some(rain) = &self.rain else {
            return 0.0;
        };
        let response = if map.depth.is_infinite() {
            1.0
        } else {
            rain.table.response(map.depth)
        };
        if response <= 0.0 {
            return 0.0;
        }
        let gain = rain.exposure([point[0], map.row.level, point[1]]) * response;
        if gain <= 0.0 {
            return 0.0;
        }
        let swash = self
            .limited_with_variances(variances, point, map)
            .0
            .swash_depth;
        let cap = 0.6 * (map.depth + swash).max(0.0);
        let hs = 4.0 * rain.variance(time).sqrt();
        if hs > 0.0 { gain.min(cap / hs) } else { gain }
    }

    /// The squared slope the rain's drops below the cut add to the moments at a point.
    pub fn rain_moments(&self, field: &Field, point: [f64; 2], time: f64) -> f64 {
        let (Some(rain), Some(map)) = (&self.rain, self.sample(point)) else {
            return 0.0;
        };
        self.rain_gain(field, point, time, map).powi(2) * rain.moments(time)
    }

    /// Add the rain bands at their gain to a sample through the one sum.
    pub(crate) fn add_rain(
        &self,
        field: &Field,
        point: [f64; 2],
        time: f64,
        map: MapSample,
        sample: &mut Sample,
    ) {
        let Some(rain) = &self.rain else {
            return;
        };
        let gain = self.rain_gain(field, point, time, map);
        if gain <= 0.0 {
            return;
        }
        let current = map.row.current;
        let departure = std::array::from_fn(|axis| current[axis].mul_add(-time, point[axis]));
        let s = rain.sample(departure, time);
        sample.height += gain * s.height;
        sample.height_dt +=
            gain * (s.height_dt - (s.slope[0] * current[0] + s.slope[1] * current[1]));
        sample.velocity[1] += gain * s.height_dt;
        sample.slope[0] += gain * s.slope[0];
        sample.slope[1] += gain * s.slope[1];
    }
}
