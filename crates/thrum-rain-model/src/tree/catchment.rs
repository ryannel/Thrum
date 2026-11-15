//! Each hollow's catchment, the cells labelled with it, sorted by height with their
//! exposure, so the fill step finds the cells under a level by one search and needs a cell
//! by cell sum only for cells part sheltered. The cells whose water leaves over the edge
//! are kept as one more catchment after the hollows'.
use super::Tree;
use crate::Canopy;
use thrum_water_model::NO_HOLLOW;

#[derive(Clone, Debug, Default)]
pub(crate) struct Catchment {
    /// Start of each catchment in the arrays below; one more entry than catchments.
    start: Vec<u32>,
    z: Vec<f64>,
    /// Running sum of exposure, inclusive.
    exposure: Vec<f64>,
    /// Running count of cells under open sky (exposure 1), inclusive.
    open: Vec<u32>,
    /// Cells with exposure between 0 and 1: position in the catchment and exposure.
    partial: Vec<(u32, f64)>,
    partial_start: Vec<u32>,
}

/// Where one catchment's rain went in a tick, cubic metres.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Gathered {
    pub(crate) canopy: f64,
    pub(crate) water: f64,
    pub(crate) runoff: f64,
    pub(crate) ground: f64,
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
            out.partial_start.push(as_u32(out.partial.len()));
            let (mut sum, mut open) = (0.0, 0);
            for (pos, &(z, i)) in list.iter().enumerate() {
                let [x, y] = tree.ground.centre(i);
                let e = canopy.exposure([x, z, y]);
                sum += e;
                if e >= 1.0 {
                    open += 1;
                } else if e > 0.0 {
                    out.partial.push((as_u32(pos), e));
                }
                out.z.push(z);
                out.exposure.push(sum);
                out.open.push(open);
            }
        }
        out.start.push(as_u32(out.z.len()));
        out.partial_start.push(as_u32(out.partial.len()));
        out
    }

    pub(crate) fn bytes(&self) -> usize {
        size_of_val(self.start.as_slice())
            + size_of_val(self.z.as_slice())
            + size_of_val(self.exposure.as_slice())
            + size_of_val(self.open.as_slice())
            + size_of_val(self.partial.as_slice())
            + size_of_val(self.partial_start.as_slice())
    }

    /// One catchment's rain over a tick: `rain` and `soak` are metres of water falling and
    /// going down into bare ground over the tick; cells below `level` are under water.
    pub(crate) fn gather(
        &self,
        slot: usize,
        level: f64,
        rain: f64,
        soak: f64,
        area: f64,
    ) -> Gathered {
        let (s, e) = (self.start[slot] as usize, self.start[slot + 1] as usize);
        let n = e - s;
        let total = if n == 0 { 0.0 } else { self.exposure[e - 1] };
        let open_total = if n == 0 { 0 } else { self.open[e - 1] };
        let k = self.z[s..e].partition_point(|&z| z < level);
        let (covered, open_covered) = if k == 0 {
            (0.0, 0)
        } else {
            (self.exposure[s + k - 1], self.open[s + k - 1])
        };
        let open_bare = f64::from(open_total - open_covered);
        let mut ground = open_bare * rain.min(soak);
        let mut runoff = open_bare * (rain - soak).max(0.0);
        let partial =
            &self.partial[self.partial_start[slot] as usize..self.partial_start[slot + 1] as usize];
        let first = partial.partition_point(|&(pos, _)| (pos as usize) < k);
        for &(_, x) in &partial[first..] {
            ground += (x * rain).min(soak);
            runoff += (x * rain - soak).max(0.0);
        }
        Gathered {
            canopy: rain * area * (n as f64 - total),
            water: rain * area * covered,
            runoff: runoff * area,
            ground: ground * area,
        }
    }
}

fn as_u32(v: usize) -> u32 {
    u32::try_from(v).expect("cells fit the profile's count")
}

impl Catchment {
    pub(crate) fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.bytes());
        for &v in self.z.iter().chain(&self.exposure) {
            out.extend_from_slice(&v.to_le_bytes());
        }
        for &v in self
            .start
            .iter()
            .chain(&self.open)
            .chain(&self.partial_start)
        {
            out.extend_from_slice(&v.to_le_bytes());
        }
        for &(p, e) in &self.partial {
            out.extend_from_slice(&p.to_le_bytes());
            out.extend_from_slice(&e.to_le_bytes());
        }
        out
    }
}
