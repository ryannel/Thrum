//! Each hollow's catchment, the cells labelled with it, sorted by height, so the fill step
//! finds the cells under a level by one search and needs a cell by cell sum only for cells
//! a shelter covers, whose exposure it reads at the ground or at the water's surface. The cells whose water leaves over the edge
//! are kept as one more catchment after the hollows'.
use super::Tree;
use crate::Canopy;
use crate::ground::reach_weights;
use thrum_water_model::NO_HOLLOW;

#[derive(Clone, Debug, Default)]
pub(crate) struct Catchment {
    /// Start of each catchment in the arrays below; one more entry than catchments.
    start: Vec<u32>,
    z: Vec<f64>,
    /// Running count of cells under open sky (exposure 1), inclusive.
    open: Vec<u32>,
    /// Cells with exposure below 1 at the ground: position in the catchment and the start
    /// of their steps.
    shaded: Vec<(u32, u32)>,
    shaded_start: Vec<u32>,
    /// Each shaded cell's exposure by height: (height, exposure from there up), the first
    /// at minus infinity with the ground's exposure, then each shelter underside above the
    /// ground. Water standing at a level is hit at that level, so its exposure is the last
    /// step at or below the level; one more entry ends the last cell's steps.
    steps: Vec<(f64, f64)>,
}

/// Where one catchment's rain went in a tick, cubic metres.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Gathered {
    pub(crate) canopy: f64,
    pub(crate) water: f64,
    pub(crate) runoff: f64,
    pub(crate) ground: f64,
    /// Open cells' summed weights on the stored reaches, times the cell's area.
    pub(crate) weights: [f64; 3],
}

impl Catchment {
    pub(crate) fn new(tree: &Tree, canopy: &Canopy) -> Self {
        let hollows = tree.pits.len();
        let mut cells: Vec<Vec<(f64, usize)>> = vec![Vec::new(); hollows + 1];
        for (i, &label) in tree.labels.iter().enumerate() {
            let slot = if label == NO_HOLLOW {
                hollows
            } else {
                label as usize
            };
            cells[slot].push((tree.ground.heights[i], i));
        }
        let mut out = Self::default();
        for list in &mut cells {
            list.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            out.start.push(as_u32(out.z.len()));
            out.shaded_start.push(as_u32(out.shaded.len()));
            let mut open = 0;
            for (pos, &(z, i)) in list.iter().enumerate() {
                let [x, y] = tree.ground.centre(i);
                let e = canopy.exposure([x, z, y]);
                if e >= 1.0 {
                    open += 1;
                } else {
                    out.shaded.push((as_u32(pos), as_u32(out.steps.len())));
                    out.steps.push((f64::NEG_INFINITY, e));
                    out.steps.extend(canopy.steps([x, z, y]));
                }
                out.z.push(z);
                out.open.push(open);
            }
        }
        out.start.push(as_u32(out.z.len()));
        out.shaded_start.push(as_u32(out.shaded.len()));
        out
    }

    /// Exposure of the `n`th shaded cell at a height at or above its ground.
    fn exposure_at(&self, n: usize, height: f64) -> f64 {
        let from = self.shaded[n].1 as usize;
        let to = self
            .shaded
            .get(n + 1)
            .map_or(self.steps.len(), |s| s.1 as usize);
        let steps = &self.steps[from..to];
        let k = steps.partition_point(|&(h, _)| h <= height);
        steps[k.max(1) - 1].1
    }

    pub(crate) fn bytes(&self) -> usize {
        size_of_val(self.start.as_slice())
            + size_of_val(self.z.as_slice())
            + size_of_val(self.open.as_slice())
            + size_of_val(self.shaded.as_slice())
            + size_of_val(self.shaded_start.as_slice())
            + size_of_val(self.steps.as_slice())
    }

    /// One catchment's rain over a tick: `rain` is metres of water falling over the tick;
    /// cells below `level` are under water, and rain on them is first stopped by what is
    /// above the water's surface, not the ground. Where `bare` gives the metres a stateless
    /// ground passes down over the tick, each uncovered cell's take and runoff are its own;
    /// either way the uncovered cells' weights on the stored reaches are summed.
    pub(crate) fn gather(
        &self,
        slot: usize,
        level: f64,
        rain: f64,
        bare: Option<f64>,
        area: f64,
    ) -> Gathered {
        let soak = bare.unwrap_or(0.0);
        let (s, e) = (self.start[slot] as usize, self.start[slot + 1] as usize);
        let open_total = if e == s { 0 } else { self.open[e - 1] };
        let k = self.z[s..e].partition_point(|&z| z < level);
        let open_covered = if k == 0 { 0 } else { self.open[s + k - 1] };
        let open_bare = f64::from(open_total - open_covered);
        let mut withheld = 0.0;
        let mut water = f64::from(open_covered);
        let mut ground = open_bare * rain.min(soak);
        let mut runoff = open_bare * (rain - soak).max(0.0);
        let mut weights = [open_bare, 0.0, 0.0];
        let (a, b) = (
            self.shaded_start[slot] as usize,
            self.shaded_start[slot + 1] as usize,
        );
        let first = a + self.shaded[a..b].partition_point(|&(pos, _)| (pos as usize) < k);
        for c in a..first {
            let x = self.exposure_at(c, level);
            water += x;
            withheld += 1.0 - x;
        }
        for c in first..b {
            let x = self.steps[self.shaded[c].1 as usize].1;
            withheld += 1.0 - x;
            ground += (x * rain).min(soak);
            runoff += (x * rain - soak).max(0.0);
            let w = reach_weights(x);
            for r in 0..3 {
                weights[r] += w[r];
            }
        }
        Gathered {
            canopy: rain * area * withheld,
            water: rain * area * water,
            runoff: if bare.is_some() { runoff * area } else { 0.0 },
            ground: if bare.is_some() { ground * area } else { 0.0 },
            weights: weights.map(|w| w * area),
        }
    }

    /// The cells of a catchment with ground at or above `lo` and below `hi`: their count
    /// and their summed weights on the stored reaches, by their exposure at the ground.
    pub(crate) fn band(&self, slot: usize, lo: f64, hi: f64) -> (usize, [f64; 3]) {
        let (s, e) = (self.start[slot] as usize, self.start[slot + 1] as usize);
        let z = &self.z[s..e];
        let (i, j) = (
            z.partition_point(|&v| v < lo),
            z.partition_point(|&v| v < hi),
        );
        if j <= i {
            return (0, [0.0; 3]);
        }
        let open = |m: usize| if m == 0 { 0 } else { self.open[s + m - 1] };
        let mut weights = [f64::from(open(j) - open(i)), 0.0, 0.0];
        let (a, b) = (
            self.shaded_start[slot] as usize,
            self.shaded_start[slot + 1] as usize,
        );
        let from = a + self.shaded[a..b].partition_point(|&(pos, _)| (pos as usize) < i);
        let to = a + self.shaded[a..b].partition_point(|&(pos, _)| (pos as usize) < j);
        for c in from..to {
            let w = reach_weights(self.steps[self.shaded[c].1 as usize].1);
            for r in 0..3 {
                weights[r] += w[r];
            }
        }
        (j - i, weights)
    }

    /// The cells of a catchment with ground at or above `lo` and below `hi`, for a sum
    /// that is not linear in exposure: how many stand under open sky, and each other
    /// cell's exposure at the ground.
    pub(crate) fn exposures(
        &self,
        slot: usize,
        lo: f64,
        hi: f64,
    ) -> (u32, impl Iterator<Item = f64> + '_) {
        let (s, e) = (self.start[slot] as usize, self.start[slot + 1] as usize);
        let z = &self.z[s..e];
        let (i, j) = (
            z.partition_point(|&v| v < lo),
            z.partition_point(|&v| v < hi)
                .max(z.partition_point(|&v| v < lo)),
        );
        let open = |m: usize| if m == 0 { 0 } else { self.open[s + m - 1] };
        let (a, b) = (
            self.shaded_start[slot] as usize,
            self.shaded_start[slot + 1] as usize,
        );
        let from = a + self.shaded[a..b].partition_point(|&(pos, _)| (pos as usize) < i);
        let to = a + self.shaded[a..b].partition_point(|&(pos, _)| (pos as usize) < j);
        (
            open(j) - open(i),
            (from..to).map(|c| self.steps[self.shaded[c].1 as usize].1),
        )
    }
}

fn as_u32(v: usize) -> u32 {
    u32::try_from(v).expect("cells fit the profile's count")
}

/// Catchments are equal when every value is identical, floats bit for bit.
impl PartialEq for Catchment {
    fn eq(&self, other: &Self) -> bool {
        let float_pairs = |a: &[(f64, f64)], b: &[(f64, f64)]| {
            a.len() == b.len()
                && a.iter()
                    .zip(b)
                    .all(|(x, y)| super::equality::same_floats(&[x.0, x.1], &[y.0, y.1]))
        };
        self.start == other.start
            && super::equality::same_floats(&self.z, &other.z)
            && self.open == other.open
            && self.shaded == other.shaded
            && self.shaded_start == other.shaded_start
            && float_pairs(&self.steps, &other.steps)
    }
}
