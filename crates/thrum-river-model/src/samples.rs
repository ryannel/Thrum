//! A river that answers its source: solved once for each flow sample, at levels of its source
//! body across that body's stage range or at the discharges its springs declare, each sample
//! with its gauge level. Its reaches are cut from the cells any sample wets, so every sample has
//! the same charts with the same footprints and ends; and between each two samples the solve
//! finds each cell's wetting gauge, spreading water from the lower sample's wet cells and from
//! where each inflow enters, reach by reach from the sources down.
use crate::solve::{Held, Solution, reaches};
use crate::{Declaration, Grid, Profile, RiverError, solve};
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};
use thrum_water_model::{Below, Fall, ReachSample, RiverFile, RiverReach, UpperSample};

/// A river solved once for each flow sample.
#[derive(Clone, Debug)]
pub struct Samples {
    /// One solve for each sample, in rising gauge order.
    pub solutions: Vec<Solution>,
    /// Each sample's gauge level: its source body's level, or the level at its first spring.
    pub gauges: Vec<f64>,
}

/// The levels a source body is solved at across its range: evenly spaced, no more than
/// `spacing` apart, from the lowest to the highest.
pub fn sample_levels(range: [f64; 2], spacing: f64) -> Vec<f64> {
    let span = range[1] - range[0];
    if span <= 0.0 {
        return vec![range[0]];
    }
    let steps = (span / spacing - 1e-9).ceil().max(1.0);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a positive count of steps"
    )]
    let n = steps as usize;
    (0..=n)
        .map(|k| {
            if k == n {
                range[1]
            } else {
                range[0] + span * k as f64 / steps
            }
        })
        .collect()
}

/// Solve a river at each of its flow samples. A river with one source body is solved at levels
/// across that body's range no more than the profile's sample spacing apart, and `springs` is
/// empty. A river fed by springs alone is solved once for each entry of `springs`, which gives
/// every spring's discharge for that sample.
/// # Errors
/// As [`solve`] for any sample, and refuses a river with several source bodies, both a source
/// body and spring samples, spring samples of the wrong length or with a discharge that is not
/// positive, and gauge levels that do not rise from sample to sample.
pub fn solve_samples(
    grid: &Grid,
    declaration: &Declaration,
    profile: &Profile,
    springs: &[Vec<f64>],
) -> Result<Samples, RiverError> {
    let bad = |reason| Err(RiverError::invalid(&declaration.name, reason));
    let mut solutions = Vec::new();
    let mut gauges = Vec::new();
    match (declaration.sources.as_slice(), springs.is_empty()) {
        ([source], true) => {
            if !(profile.sample_spacing.is_finite() && profile.sample_spacing > 0.0) {
                return bad("invalid sample spacing");
            }
            for level in sample_levels(source.range, profile.sample_spacing) {
                let mut d = declaration.clone();
                d.sources[0].level = level;
                solutions.push(solve(grid, &d, profile)?);
                gauges.push(level);
            }
        }
        ([], false) => {
            let Some(first) = declaration.springs.first() else {
                return bad("spring samples for a river with no spring");
            };
            let Some(cell) = grid.cell(first.point) else {
                return bad("spring is off the grid");
            };
            for flows in springs {
                if flows.len() != declaration.springs.len()
                    || flows.iter().any(|q| !q.is_finite() || *q <= 0.0)
                {
                    return bad("invalid spring samples");
                }
                let mut d = declaration.clone();
                for (s, &q) in d.springs.iter_mut().zip(flows) {
                    s.discharge = q;
                }
                let s = solve(grid, &d, profile)?;
                gauges.push(s.level(cell));
                solutions.push(s);
            }
        }
        _ => return bad("a river is sampled by one source body's levels or its springs' flows"),
    }
    if gauges.windows(2).any(|w| w[1] <= w[0]) {
        return bad("gauge levels do not rise from sample to sample");
    }
    Ok(Samples { solutions, gauges })
}

/// What every sample shares: the cells any of them wets, the brinks any runs, the reaches cut
/// from those and the bodies' cells at the highest sample.
struct Union {
    wet: Vec<Vec<bool>>,
    any: Vec<bool>,
    x_brink: Vec<bool>,
    z_brink: Vec<bool>,
    reach_of: Vec<Option<usize>>,
    names: Vec<String>,
    held: Vec<Option<Held>>,
}

fn key(v: f64, i: usize) -> Reverse<(u64, usize)> {
    Reverse((v.to_bits() ^ if v < 0.0 { u64::MAX } else { 1 << 63 }, i))
}

fn neighbours(grid: &Grid, i: usize) -> impl Iterator<Item = usize> + use<> {
    let [nx, nz] = grid.size();
    let (x, y) = (i % nx, i / nx);
    [
        (x > 0).then(|| i - 1),
        (x + 1 < nx).then_some(i + 1),
        (y > 0).then(|| i - nx),
        (y + 1 < nz).then_some(i + nx),
    ]
    .into_iter()
    .flatten()
}

/// The lip cell of a fall segment, from its stable ID.
fn lip_cell(id: u64) -> usize {
    (id & 0xffff_ffff) as usize / 4
}

impl Samples {
    /// The lowest and highest gauge levels: the range a spring-fed river's stage may take.
    pub fn gauge_range(&self) -> [f64; 2] {
        [self.gauges[0], self.gauges[self.gauges.len() - 1]]
    }

    fn grid(&self) -> &Grid {
        &self.solutions[0].grid
    }

    fn union(&self) -> Union {
        let top = &self.solutions[self.solutions.len() - 1];
        let held = top.held.clone();
        let n = held.len();
        let wet: Vec<Vec<bool>> = self
            .solutions
            .iter()
            .map(|s| (0..n).map(|i| s.wet[i] && held[i].is_none()).collect())
            .collect();
        let any: Vec<bool> = (0..n).map(|i| wet.iter().any(|w| w[i])).collect();
        let or = |f: fn(&Solution) -> &Vec<bool>| -> Vec<bool> {
            let len = f(top).len();
            (0..len)
                .map(|i| self.solutions.iter().any(|s| f(s)[i]))
                .collect()
        };
        let x_brink = or(|s| &s.x_brink);
        let z_brink = or(|s| &s.z_brink);
        let (reach_of, names) =
            reaches(self.grid(), &any, &x_brink, &z_brink, &top.declaration.name);
        Union {
            wet,
            any,
            x_brink,
            z_brink,
            reach_of,
            names,
            held,
        }
    }

    /// Sample `k` as the shared cut sees it.
    fn view(&self, k: usize, u: &Union) -> Solution {
        let mut v = self.solutions[k].clone();
        v.held.clone_from(&u.held);
        v.wet.clone_from(&u.wet[k]);
        v.x_brink.clone_from(&u.x_brink);
        v.z_brink.clone_from(&u.z_brink);
        v.reach_of.clone_from(&u.reach_of);
        v.reach_names.clone_from(&u.names);
        v
    }

    fn across_brink(&self, u: &Union, i: usize, j: usize) -> bool {
        let [nx, _] = self.grid().size();
        let (a, b) = (i.min(j), i.max(j));
        let (x, z) = (b % nx, b / nx);
        if b == a + 1 {
            u.x_brink[z * (nx + 1) + x]
        } else {
            u.z_brink[z * nx + x]
        }
    }

    /// Every reach as the river file stores it: the lowest sample's rows, and each sample
    /// above's rows and wetting gauges. Reaches are cut from the cells any sample wets, so the
    /// footprints and ends are the same in every sample.
    /// # Errors
    /// As [`Solution::reaches`].
    pub fn reaches(&self) -> Result<Vec<RiverReach>, RiverError> {
        Ok(self.assemble()?.0)
    }

    #[expect(
        clippy::type_complexity,
        reason = "the reaches and, for each pair of samples, the lip wetting gauges"
    )]
    fn assemble(&self) -> Result<(Vec<RiverReach>, Vec<Vec<(u64, f64)>>, Union), RiverError> {
        let u = self.union();
        let n = u.any.len();
        let views: Vec<Solution> = (0..self.solutions.len())
            .map(|k| self.view(k, &u))
            .collect();
        let mut claimed = u.any.clone();
        let mut out = Vec::with_capacity(u.names.len());
        for r in 0..u.names.len() {
            let banks = views[0].claim_faces_for(r, &claimed, &u.any);
            let wet_r = |k: usize| -> Vec<bool> {
                (0..n)
                    .map(|i| u.wet[k][i] && u.reach_of[i] == Some(r))
                    .collect()
            };
            let mut solved = vec![false; n];
            for (k, v) in views.iter().enumerate() {
                let own = v.one_reach(r, &wet_r(k), &banks, None)?;
                for ((s, &f), h) in solved.iter_mut().zip(&own.footprint).zip(&u.held) {
                    *s |= f && h.is_none();
                }
            }
            let any: Vec<bool> = (0..n)
                .map(|i| u.reach_of[i] == Some(r) && u.wet.iter().any(|w| w[i]))
                .collect();
            let mut rows = Vec::with_capacity(views.len());
            for (k, v) in views.iter().enumerate() {
                rows.push(v.one_reach(r, &wet_r(k), &banks, Some((&solved, &any)))?);
            }
            let mut reach = rows.remove(0);
            for row in rows {
                debug_assert_eq!(row.footprint, reach.footprint);
                reach.upper.push(ReachSample {
                    levels: row.levels,
                    currents: row.currents,
                    loss: row.loss,
                    gains: row.gains,
                    wetting: vec![None; n],
                });
            }
            for (c, &f) in claimed.iter_mut().zip(&reach.footprint) {
                *c |= f;
            }
            out.push(reach);
        }
        let mut lips = Vec::with_capacity(self.gauges.len().saturating_sub(1));
        for k in 0..self.gauges.len().saturating_sub(1) {
            let (cells, lip) = self.wetting(&u, &out, k);
            for reach in &mut out {
                reach.upper[k].wetting = (0..n)
                    .map(|i| reach.footprint[i].then_some(cells[i]).flatten())
                    .collect();
            }
            lips.push(lip);
        }
        Ok((out, lips, u))
    }

    /// The wetting gauges between samples `k` and `k + 1`, for every cell of the rectangle, and
    /// for each fall segment only the upper runs, its lip's.
    fn wetting(
        &self,
        u: &Union,
        reaches: &[RiverReach],
        k: usize,
    ) -> (Vec<Option<f64>>, Vec<(u64, f64)>) {
        let grid = self.grid();
        let z = grid.heights();
        let n = z.len();
        let [nx, nz] = grid.size();
        let (ga, gb) = (self.gauges[k], self.gauges[k + 1]);
        let level = |s: usize, r: &RiverReach, i: usize| {
            if s == 0 {
                r.levels[i]
            } else {
                r.upper[s - 1].levels[i]
            }
        };
        let mut lo = vec![f64::NEG_INFINITY; n];
        let mut hi = vec![f64::NEG_INFINITY; n];
        for r in reaches {
            for i in (0..n).filter(|&i| r.footprint[i]) {
                lo[i] = level(k, r, i);
                hi[i] = level(k + 1, r, i);
            }
        }
        let lo_wet = |i: usize| lo[i] > z[i];
        let hi_wet = |i: usize| hi[i] > z[i];
        let waits = |i: usize| hi_wet(i) && !lo_wet(i);
        // Where the upper sample's falls land, by the cell of each lip.
        let upper = &self.solutions[k + 1];
        let lower_ids: Vec<u64> = self.solutions[k]
            .falls
            .iter()
            .flat_map(|f| f.segments.iter().map(|s| s.id))
            .collect();
        let mut landings: Vec<(usize, usize)> = Vec::new();
        for s in upper.falls.iter().flat_map(|f| &f.segments) {
            let Some(end) = s.path.last() else { continue };
            let Some(cell) = grid.cell(end.place) else {
                continue;
            };
            let cell = if hi_wet(cell) {
                Some(cell)
            } else {
                neighbours(grid, cell).find(|&j| hi_wet(j))
            };
            if let Some(c) = cell {
                landings.push((lip_cell(s.id), c));
            }
        }
        let mut gauge: Vec<Option<f64>> = vec![None; n];
        let mut heap = BinaryHeap::new();
        for i in 0..n {
            if lo_wet(i) {
                heap.push(key(ga, i));
            }
        }
        // A spring's cell wets from the lower sample's gauge.
        for spring in &upper.declaration.springs {
            if let Some(c) = grid.cell(spring.point).filter(|&c| waits(c)) {
                heap.push(key(ga, c));
            }
        }
        let passes = |c: usize, g: f64, j: usize| -> Option<f64> {
            let (k0, a, b) = if lo_wet(c) {
                (ga, lo[c], hi[c])
            } else {
                (g, z[c], hi[c])
            };
            if a >= z[j] {
                Some(k0)
            } else if b >= z[j] && b > a {
                Some(k0 + (z[j] - a) / (b - a) * (gb - k0))
            } else {
                None
            }
        };
        // The neighbour the upper sample's flow enters a cell from, for a cell no level reaches.
        let feeder = |i: usize| -> Option<usize> {
            let (x, y) = (i % nx, i / nx);
            let faces = [
                (x > 0).then(|| (i - 1, upper.x_flux[y * (nx + 1) + x])),
                (x + 1 < nx).then(|| (i + 1, -upper.x_flux[y * (nx + 1) + x + 1])),
                (y > 0).then(|| (i - nx, upper.z_flux[y * nx + x])),
                (y + 1 < nz).then(|| (i + nx, -upper.z_flux[(y + 1) * nx + x])),
            ];
            faces
                .into_iter()
                .flatten()
                .filter(|&(j, q)| q > 0.0 && !self.across_brink(u, i, j))
                .max_by(|a, b| a.1.total_cmp(&b.1).then(b.0.cmp(&a.0)))
                .map(|(j, _)| j)
        };
        loop {
            while let Some(Reverse((bits, c))) = heap.pop() {
                if gauge[c].is_some() {
                    continue;
                }
                let g = f64::from_bits(if bits & (1 << 63) != 0 {
                    bits ^ (1 << 63)
                } else {
                    bits ^ u64::MAX
                });
                gauge[c] = Some(g);
                for j in neighbours(grid, c) {
                    if gauge[j].is_some() || !waits(j) || self.across_brink(u, c, j) {
                        continue;
                    }
                    if let Some(p) = passes(c, g, j) {
                        heap.push(key(p.max(g), j));
                    }
                }
                for &(_, landing) in landings.iter().filter(|l| l.0 == c) {
                    if gauge[landing].is_none() && waits(landing) {
                        heap.push(key(g, landing));
                    }
                }
            }
            let late: Vec<(usize, f64)> = (0..n)
                .filter(|&i| waits(i) && gauge[i].is_none())
                .filter_map(|i| feeder(i).and_then(|j| gauge[j]).map(|g| (i, g)))
                .collect();
            if late.is_empty() {
                break;
            }
            for (i, g) in late {
                heap.push(key(g, i));
            }
        }
        let cells: Vec<Option<f64>> = (0..n)
            .map(|i| waits(i).then(|| gauge[i].unwrap_or(gb).clamp(ga, gb)))
            .collect();
        let mut lip = Vec::new();
        for s in upper.falls.iter().flat_map(|f| &f.segments) {
            if !lower_ids.contains(&s.id) {
                let c = lip_cell(s.id);
                lip.push((s.id, cells[c].unwrap_or(ga)));
            }
        }
        lip.sort_by_key(|l| l.0);
        (cells, lip)
    }

    /// Each sample's falls, cut the same way in every sample: a fall is a run of lips, beside
    /// one another, that any sample's water leaves over, named in cell order. Each sample's
    /// record holds the segments it runs, naming the shared reaches above and below.
    fn falls(&self, u: &Union) -> Vec<Vec<Fall>> {
        let grid = self.grid();
        let mut cells: Vec<usize> = self
            .solutions
            .iter()
            .flat_map(|s| {
                s.falls
                    .iter()
                    .flat_map(|f| f.segments.iter().map(|g| lip_cell(g.id)))
            })
            .collect();
        cells.sort_unstable();
        cells.dedup();
        let mut group = vec![usize::MAX; cells.len()];
        let mut count = 0;
        for a in 0..cells.len() {
            if group[a] != usize::MAX {
                continue;
            }
            group[a] = count;
            let mut q = VecDeque::from([a]);
            while let Some(k) = q.pop_front() {
                for b in 0..cells.len() {
                    let (i, j) = (cells[k], cells[b]);
                    if group[b] == usize::MAX && (i == j || neighbours(grid, i).any(|n| n == j)) {
                        group[b] = count;
                        q.push_back(b);
                    }
                }
            }
            count += 1;
        }
        let name = &self.solutions[0].declaration.name;
        let reach_name = |cell: usize| -> Option<String> {
            u.reach_of[cell]
                .or_else(|| neighbours(grid, cell).find_map(|j| u.reach_of[j]))
                .map(|r| u.names[r].clone())
        };
        self.solutions
            .iter()
            .map(|s| {
                let mut falls: Vec<Fall> = (0..count)
                    .map(|g| Fall {
                        name: format!("{name} fall {}", g + 1),
                        above: String::new(),
                        segments: Vec::new(),
                    })
                    .collect();
                for seg in s.falls.iter().flat_map(|f| &f.segments) {
                    let c = lip_cell(seg.id);
                    let g = group[cells.binary_search(&c).expect("a lip of a sample")];
                    let mut seg = seg.clone();
                    if let Below::Reach(_) = seg.below
                        && let Some(end) = seg.path.last()
                        && let Some(r) = grid.cell(end.place).and_then(reach_name)
                    {
                        seg.below = Below::Reach(r);
                    }
                    let fall = &mut falls[g];
                    if let Some(r) = reach_name(c) {
                        fall.above = r;
                    }
                    fall.segments.push(seg);
                }
                for f in &mut falls {
                    f.segments.sort_by_key(|g| g.id);
                }
                falls.retain(|f| !f.segments.is_empty());
                falls
            })
            .collect()
    }

    /// The river as a river file under `key`: every reach with each sample's rows and wetting
    /// gauges, and each sample's falls.
    /// # Errors
    /// As [`Solution::reaches`].
    pub fn file(&self, key: [u8; 32]) -> Result<RiverFile, RiverError> {
        let (reaches, lips, u) = self.assemble()?;
        let mut falls = self.falls(&u);
        let lowest = falls.remove(0);
        Ok(RiverFile {
            key,
            gauge_level: self.gauges[0],
            reaches,
            falls: lowest,
            upper: falls
                .into_iter()
                .zip(lips)
                .zip(&self.gauges[1..])
                .map(|((falls, lip_wetting), &gauge_level)| UpperSample {
                    gauge_level,
                    falls,
                    lip_wetting,
                })
                .collect(),
        })
    }
}
