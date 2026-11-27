//! The tree of hollows: one flood of the bed from its pits, lowest ground first, labels each
//! cell with the hollow it was reached from; the passes between labels, taken lowest first,
//! join hollows into a tree (a depression hierarchy). Each node keeps its curve of volume for
//! level as the sorted heights of only the cells whose water first stands in it, so every
//! cell is kept once however deep the tree.
use crate::{Canopy, Profile, RainError};
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use thrum_water_model::NO_HOLLOW;

pub(crate) mod catchment;
mod equality;

pub(crate) use catchment::Catchment;

/// A hollows body's ground: square cells, row-major x-fastest, `origin` the first cell's
/// corner. Heights are the ground's at each cell, in metres.
#[derive(Clone, Debug)]
pub struct Ground {
    origin: [f64; 2],
    spacing: f64,
    size: [usize; 2],
    heights: Vec<f64>,
    outlets: Vec<bool>,
}

impl Ground {
    /// Validate a grid of heights.
    /// # Errors
    /// Refuses a nonfinite origin or height, a bad spacing or a wrong count.
    pub fn new(
        origin: [f64; 2],
        spacing: f64,
        size: [usize; 2],
        heights: Vec<f64>,
    ) -> Result<Self, RainError> {
        if origin.iter().any(|v| !v.is_finite())
            || !spacing.is_finite()
            || spacing <= 0.0
            || size.contains(&0)
            || size[0].checked_mul(size[1]) != Some(heights.len())
            || heights.iter().any(|v| !v.is_finite())
        {
            return Err(RainError::Ground("invalid grid"));
        }
        // One zero, so the flood's order of heights is the order of their values.
        let heights = heights.into_iter().map(|h| h + 0.0).collect();
        Ok(Self {
            origin,
            spacing,
            size,
            heights,
            outlets: Vec::new(),
        })
    }

    /// Cells that drain into the surrounding ground outside the authored
    /// catchment. Their measured heights still determine the overflow sills.
    /// # Errors
    /// Refuses a mask that does not have one entry per cell.
    pub fn with_outlets(mut self, outlets: Vec<bool>) -> Result<Self, RainError> {
        if outlets.len() != self.heights.len() {
            return Err(RainError::Ground(
                "outlet mask size differs from the ground",
            ));
        }
        self.outlets = outlets;
        Ok(self)
    }

    /// Cell counts in x and z.
    pub fn size(&self) -> [usize; 2] {
        self.size
    }

    /// Cell width in metres.
    pub fn spacing(&self) -> f64 {
        self.spacing
    }

    /// The first cell's corner.
    pub fn origin(&self) -> [f64; 2] {
        self.origin
    }

    /// Height of each cell.
    pub fn heights(&self) -> &[f64] {
        &self.heights
    }

    /// A cell's centre in world x/z.
    pub fn centre(&self, i: usize) -> [f64; 2] {
        [
            self.origin[0] + ((i % self.size[0]) as f64 + 0.5) * self.spacing,
            self.origin[1] + ((i / self.size[0]) as f64 + 0.5) * self.spacing,
        ]
    }

    /// The cells that share an edge with a cell.
    pub fn neighbours(&self, i: usize) -> impl Iterator<Item = usize> + use<> {
        let [w, h] = self.size;
        [
            (!i.is_multiple_of(w)).then(|| i - 1),
            (i % w + 1 < w).then_some(i + 1),
            (i >= w).then(|| i - w),
            (i / w + 1 < h).then_some(i + w),
        ]
        .into_iter()
        .flatten()
    }

    /// Whether a cell lies on the rectangle's edge.
    pub fn edge(&self, i: usize) -> bool {
        let [w, h] = self.size;
        i.is_multiple_of(w) || i % w + 1 == w || i < w || i / w + 1 == h
    }
}

/// A node of the tree: a hollow (a leaf) or two that have joined above their sill.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Node {
    /// The node the two become above this node's sill; none for a root.
    pub parent: Option<u32>,
    /// The two nodes this one joins; none for a hollow.
    pub children: Option<[u32; 2]>,
    /// The level at which its water leaves it: into its sibling, or for a root over the
    /// pass its overflow takes out of the tree.
    pub sill: f64,
    /// The hollow its overflow enters, or none where it leaves the rectangle.
    pub over: Option<u32>,
    /// The level its curve starts at: a hollow's pit, or its children's sill.
    pub lower: f64,
}

/// A hollow fewer cells across at its sill than the profile's least.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SmallHollow {
    /// The hollow.
    pub hollow: u32,
    /// Cells across: the smaller side of the box round its cells below its sill.
    pub across: usize,
}

/// One node's curve: the cells whose water first stands in this node, with heights
/// relative to `lower`, sorted, and their running sums.
#[derive(Clone, Copy, Debug, Default)]
struct Curve {
    start: u32,
    len: u32,
    /// Cells below `lower` (all in descendants).
    base_cells: u32,
    /// Volume held at `lower`, cubic metres.
    base_volume: f64,
    /// Volume held at the sill.
    capacity: f64,
}

/// A prepared hollows body: the bed with shallow hollows filled, a label for each cell, the
/// tree and each node's curve, and the catchment of each hollow under its exposure.
#[derive(Clone, Debug)]
pub struct Tree {
    pub(crate) ground: Ground,
    pub(crate) labels: Vec<u32>,
    pub(crate) pits: Vec<u32>,
    pub(crate) nodes: Vec<Node>,
    curves: Vec<Curve>,
    band: Vec<f64>,
    band_sum: Vec<f64>,
    small: Vec<SmallHollow>,
    pub(crate) catchment: Catchment,
}

impl Tree {
    /// Cut a bed into its tree of hollows, and take each cell's exposure from the canopy at
    /// the cell's centre and ground. A hollow shallower than the profile's least depth is
    /// filled: its cells' bed is raised to its sill and they join the hollow across it.
    /// # Errors
    /// Refuses a bad profile and a body over the profile's count of cells or hollows.
    pub fn prepare(ground: &Ground, canopy: &Canopy, profile: &Profile) -> Result<Self, RainError> {
        if !profile.least_depth.is_finite() || profile.least_depth < 0.0 {
            return Err(RainError::Profile("invalid least depth"));
        }
        let cells = ground.heights.len();
        if cells > profile.max_cells {
            return Err(RainError::OverLimit {
                what: "cells",
                count: cells,
                limit: profile.max_cells,
            });
        }
        let flood = flood(ground);
        let outlets = outlets(ground, &flood.labels);
        let joined = join(ground, &flood, &outlets, profile.least_depth);
        if joined.pits.len() > profile.max_hollows {
            return Err(RainError::OverLimit {
                what: "hollows",
                count: joined.pits.len(),
                limit: profile.max_hollows,
            });
        }
        let mut tree = Self {
            ground: Ground {
                heights: joined.heights,
                ..ground.clone()
            },
            labels: joined.labels,
            pits: joined.pits,
            nodes: joined.nodes,
            curves: Vec::new(),
            band: Vec::new(),
            band_sum: Vec::new(),
            small: Vec::new(),
            catchment: Catchment::default(),
        };
        tree.build_curves();
        tree.small = tree.find_small(profile.least_across);
        tree.catchment = Catchment::new(&tree, canopy);
        Ok(tree)
    }

    /// The prepared ground, with shallow hollows filled.
    pub fn ground(&self) -> &Ground {
        &self.ground
    }

    /// Each cell's hollow, or [`NO_HOLLOW`] where its water leaves over the edge.
    pub fn labels(&self) -> &[u32] {
        &self.labels
    }

    /// Number of hollows. Hollow `h` is node `h`.
    pub fn hollow_count(&self) -> usize {
        self.pits.len()
    }

    /// Number of nodes: the hollows, then the joined nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// A hollow's pit cell.
    pub fn pit(&self, hollow: u32) -> usize {
        self.pits[hollow as usize] as usize
    }

    /// A node.
    pub fn node(&self, node: u32) -> Node {
        self.nodes[node as usize]
    }

    /// Hollows fewer cells across than the profile's least.
    pub fn small_hollows(&self) -> &[SmallHollow] {
        &self.small
    }

    /// Cubic metres a node holds at its sill.
    pub fn capacity(&self, node: u32) -> f64 {
        self.curves[node as usize].capacity
    }

    /// Payload bytes of the nodes' curves of level for volume: the arrays' lengths, not
    /// their allocated capacity.
    pub fn curve_bytes(&self) -> usize {
        size_of_val(self.band.as_slice())
            + size_of_val(self.band_sum.as_slice())
            + size_of_val(self.curves.as_slice())
    }

    /// Payload bytes of the prepared body's arrays: their lengths, not allocated capacity,
    /// and without the small-hollow report or the structs themselves.
    pub fn bytes(&self) -> usize {
        self.curve_bytes()
            + size_of_val(self.ground.heights.as_slice())
            + size_of_val(self.labels.as_slice())
            + size_of_val(self.pits.as_slice())
            + size_of_val(self.nodes.as_slice())
            + self.catchment.bytes()
    }

    fn area(&self) -> f64 {
        self.ground.spacing * self.ground.spacing
    }

    /// Sum of the first `m` relative heights of a node's curve.
    fn prefix(&self, c: &Curve, m: usize) -> f64 {
        if m == 0 {
            0.0
        } else {
            self.band_sum[c.start as usize + m - 1]
        }
    }

    fn rel(&self, c: &Curve) -> &[f64] {
        &self.band[c.start as usize..(c.start + c.len) as usize]
    }

    /// Cubic metres a node holds with its water at `level`, up to its sill.
    pub fn volume(&self, node: u32, level: f64) -> f64 {
        let n = self.nodes[node as usize];
        let c = &self.curves[node as usize];
        let t = (level.min(n.sill) - n.lower).max(0.0);
        let m = self.rel(c).partition_point(|&z| z < t);
        c.base_volume + self.area() * ((f64::from(c.base_cells) + m as f64) * t - self.prefix(c, m))
    }

    /// The level of a node holding `volume` cubic metres, at most its sill.
    pub fn level(&self, node: u32, volume: f64) -> f64 {
        let n = self.nodes[node as usize];
        let c = &self.curves[node as usize];
        if volume >= c.capacity {
            return n.sill;
        }
        let a = self.area();
        let extra = (volume - c.base_volume).max(0.0) / a;
        let rel = self.rel(c);
        let base = f64::from(c.base_cells);
        // Cells of the curve under the water: those at which the water held is no more than
        // `extra`. The volume at each cell's height rises with the cell, so search.
        let (mut lo, mut hi) = (0, rel.len());
        while lo < hi {
            let m = usize::midpoint(lo, hi);
            if (base + m as f64) * rel[m] - self.prefix(c, m) <= extra {
                lo = m + 1;
            } else {
                hi = m;
            }
        }
        let k = lo;
        let t = if k == 0 {
            if base > 0.0 { extra / base } else { 0.0 }
        } else {
            (extra + self.prefix(c, k)) / (base + k as f64)
        };
        n.lower + t.min(n.sill - n.lower)
    }

    /// Square metres of water surface a node has with its water at `level`.
    pub fn surface(&self, node: u32, level: f64) -> f64 {
        let n = self.nodes[node as usize];
        let c = &self.curves[node as usize];
        // A joined node's water never stands below its children's sill, where its base
        // cells are all under water.
        let t = level - n.lower;
        let m = self.rel(c).partition_point(|&z| z < t);
        (f64::from(c.base_cells) + m as f64) * self.area()
    }

    /// The map's declaration of this body, with a level for each hollow.
    /// # Errors
    /// Refuses a level table of the wrong length or with a nonfinite level.
    pub fn water_hollows(
        &self,
        levels: &[f64],
    ) -> Result<thrum_water_model::Hollows, thrum_water_model::WaterError> {
        thrum_water_model::Hollows::new(
            self.ground.origin,
            self.ground.spacing,
            self.ground.size,
            self.ground.heights.clone(),
            self.labels.clone(),
            levels.to_vec(),
        )
    }

    fn build_curves(&mut self) {
        let count = self.nodes.len();
        let mut members: Vec<Vec<f64>> = vec![Vec::new(); count];
        for (i, &label) in self.labels.iter().enumerate() {
            if label == NO_HOLLOW {
                continue;
            }
            let z = self.ground.heights[i];
            let mut node = label as usize;
            loop {
                let n = self.nodes[node];
                if z < n.sill {
                    members[node].push(z - n.lower);
                    break;
                }
                match n.parent {
                    Some(p) => node = p as usize,
                    None => break,
                }
            }
        }
        self.curves = vec![Curve::default(); count];
        // Children always precede their parent.
        for (node, mut rel) in members.into_iter().enumerate() {
            rel.sort_by(f64::total_cmp);
            let start = u32::try_from(self.band.len()).expect("cells fit the profile's count");
            let mut sum = 0.0;
            for &z in &rel {
                sum += z;
                self.band.push(z);
                self.band_sum.push(sum);
            }
            let (base_cells, base_volume) = match self.nodes[node].children {
                None => (0, 0.0),
                Some([a, b]) => {
                    let (ca, cb) = (self.curves[a as usize], self.curves[b as usize]);
                    (
                        ca.base_cells + ca.len + cb.base_cells + cb.len,
                        ca.capacity + cb.capacity,
                    )
                }
            };
            self.curves[node] = Curve {
                start,
                len: u32::try_from(rel.len()).expect("cells fit the profile's count"),
                base_cells,
                base_volume,
                capacity: 0.0,
            };
            let sill = self.nodes[node].sill;
            self.curves[node].capacity = self.volume(
                u32::try_from(node).expect("nodes fit the profile's count"),
                sill,
            );
        }
    }

    fn find_small(&self, least: usize) -> Vec<SmallHollow> {
        let w = self.ground.size[0];
        let mut boxes = vec![[usize::MAX, usize::MAX, 0, 0]; self.pits.len()];
        for (i, &label) in self.labels.iter().enumerate() {
            if label == NO_HOLLOW || self.ground.heights[i] >= self.nodes[label as usize].sill {
                continue;
            }
            let b = &mut boxes[label as usize];
            let (x, z) = (i % w, i / w);
            *b = [b[0].min(x), b[1].min(z), b[2].max(x), b[3].max(z)];
        }
        boxes
            .iter()
            .enumerate()
            .filter_map(|(h, b)| {
                let across = if b[0] == usize::MAX {
                    0
                } else {
                    (b[2] - b[0] + 1).min(b[3] - b[1] + 1)
                };
                (across < least).then(|| SmallHollow {
                    hollow: u32::try_from(h).expect("hollows fit the profile's count"),
                    across,
                })
            })
            .collect()
    }
}

/// Labels from the flood: 0 for the edge's, `k + 1` for the pit at `pits[k]`.
struct Flood {
    labels: Vec<u32>,
    pits: Vec<u32>,
}

/// Heights in the flood's order: by value, ties broken by cell.
fn key(z: f64, i: usize) -> (u64, usize) {
    let bits = z.to_bits();
    (
        if bits >> 63 == 1 {
            !bits
        } else {
            bits | 1 << 63
        },
        i,
    )
}

/// One flood from the edge and from every pit, lowest ground first. A cell is labelled by
/// the first labelled neighbour taken from the queue, which is never higher than it, so
/// every cell is reached from its seed by a path that never descends.
fn flood(ground: &Ground) -> Flood {
    let z = &ground.heights;
    let n = z.len();
    let mut labels = vec![u32::MAX; n];
    let mut heap = BinaryHeap::new();
    for i in (0..n).filter(|&i| ground.edge(i) || ground.outlets.get(i).copied().unwrap_or(false)) {
        labels[i] = 0;
        heap.push(Reverse(key(z[i], i)));
    }
    let mut pits = Vec::new();
    for i in 0..n {
        if labels[i] == u32::MAX && ground.neighbours(i).all(|j| key(z[i], i) < key(z[j], j)) {
            pits.push(u32::try_from(i).expect("cells fit the profile's count"));
            labels[i] = u32::try_from(pits.len()).expect("cells fit the profile's count");
            heap.push(Reverse(key(z[i], i)));
        }
    }
    while let Some(Reverse((_, i))) = heap.pop() {
        for j in ground.neighbours(i) {
            if labels[j] == u32::MAX {
                labels[j] = labels[i];
                heap.push(Reverse(key(z[j], j)));
            }
        }
    }
    Flood { labels, pits }
}

/// The lowest pass between each pair of labels that touch: `(pass, a, b)` with `a < b`.
fn outlets(ground: &Ground, labels: &[u32]) -> Vec<(f64, u32, u32)> {
    let z = &ground.heights;
    let w = ground.size[0];
    let mut pairs = Vec::new();
    for i in 0..z.len() {
        for j in [
            (i % w + 1 < w).then(|| i + 1),
            (i + w < z.len()).then(|| i + w),
        ]
        .into_iter()
        .flatten()
        {
            let (a, b) = (labels[i], labels[j]);
            if a != b {
                pairs.push((a.min(b), a.max(b), z[i].max(z[j])));
            }
        }
    }
    pairs.sort_by(|p, q| (p.0, p.1).cmp(&(q.0, q.1)).then(p.2.total_cmp(&q.2)));
    pairs.dedup_by_key(|p| (p.0, p.1));
    let mut out: Vec<_> = pairs.into_iter().map(|(a, b, e)| (e, a, b)).collect();
    out.sort_by(|p, q| p.0.total_cmp(&q.0).then((p.1, p.2).cmp(&(q.1, q.2))));
    out
}

/// What joining produces: the raised bed, final labels, live pits and the nodes.
struct Joined {
    heights: Vec<f64>,
    labels: Vec<u32>,
    pits: Vec<u32>,
    nodes: Vec<Node>,
}

/// A component while passes are taken lowest first.
#[derive(Clone, Copy)]
struct Part {
    /// Its node in the building tree, or none once it drains over the edge.
    node: Option<u32>,
    /// Its one hollow, while it is a single hollow that may still be filled.
    single: Option<u32>,
    pit: f64,
}

/// Take passes lowest first, as Kruskal's algorithm does. Two parts that meet become a
/// node; a part that meets the edge's becomes a root. A single hollow shallower than the
/// least depth when it meets another part is filled and joins the hollow across its sill.
fn join(ground: &Ground, flood: &Flood, outlets: &[(f64, u32, u32)], least: f64) -> Joined {
    let leaves = flood.pits.len();
    let z = &ground.heights;
    // Union-find over labels; label 0 is the edge's.
    let mut root: Vec<u32> = (0..=u32::try_from(leaves).expect("cells fit")).collect();
    let find = |root: &mut Vec<u32>, mut a: u32| {
        while root[a as usize] != a {
            root[a as usize] = root[root[a as usize] as usize];
            a = root[a as usize];
        }
        a
    };
    let mut parts: Vec<Part> = (0..=leaves)
        .map(|l| Part {
            node: (l > 0).then(|| u32::try_from(l - 1).expect("cells fit")),
            single: (l > 0).then(|| u32::try_from(l).expect("cells fit")),
            pit: if l > 0 {
                z[flood.pits[l - 1] as usize]
            } else {
                f64::NEG_INFINITY
            },
        })
        .collect();
    // Each label's hollow after filling (0 for the edge's) and its raise.
    let mut into: Vec<u32> = (0..=u32::try_from(leaves).expect("cells fit")).collect();
    let mut raise = vec![f64::NEG_INFINITY; leaves + 1];
    let resolve = |into: &Vec<u32>, mut l: u32| {
        while into[l as usize] != l {
            l = into[l as usize];
        }
        l
    };
    let mut nodes: Vec<Node> = (0..leaves)
        .map(|k| Node {
            parent: None,
            children: None,
            sill: f64::INFINITY,
            over: None,
            lower: z[flood.pits[k] as usize],
        })
        .collect();
    let shallow = |p: &Part, e: f64| p.single.is_some() && e - p.pit < least;
    for &(e, a, b) in outlets {
        let (ra, rb) = (find(&mut root, a), find(&mut root, b));
        if ra == rb {
            continue;
        }
        let (pa, pb) = (parts[ra as usize], parts[rb as usize]);
        let edge_a = pa.node.is_none();
        let edge_b = pb.node.is_none();
        let merged = if edge_a || edge_b {
            // One side drains over the edge: the other becomes a root, or is filled.
            let (p, across) = if edge_a { (pb, a) } else { (pa, b) };
            let target = resolve(&into, across);
            if shallow(&p, e) {
                let leaf = p.single.expect("shallow part is one hollow");
                into[leaf as usize] = target;
                raise[leaf as usize] = e;
            } else {
                let n = &mut nodes[p.node.expect("hollow part has a node") as usize];
                n.sill = e;
                n.over = (target > 0).then(|| target - 1);
            }
            Part {
                node: None,
                single: None,
                pit: f64::NEG_INFINITY,
            }
        } else {
            let (sa, sb) = (shallow(&pa, e), shallow(&pb, e));
            if sa || sb {
                // Fill the shallow one; if both are, the one with the higher pit.
                let fill_a = sa && (!sb || (pa.pit, ra) > (pb.pit, rb));
                let (gone, stays, across) = if fill_a { (pa, pb, b) } else { (pb, pa, a) };
                let leaf = gone.single.expect("shallow part is one hollow");
                into[leaf as usize] = resolve(&into, across);
                raise[leaf as usize] = e;
                stays
            } else {
                let k = u32::try_from(nodes.len()).expect("nodes fit");
                let (na, nb) = (pa.node.expect("node"), pb.node.expect("node"));
                for (child, across) in [(na, b), (nb, a)] {
                    let n = &mut nodes[child as usize];
                    n.parent = Some(k);
                    n.sill = e;
                    n.over = Some(resolve(&into, across) - 1);
                }
                nodes.push(Node {
                    parent: None,
                    children: Some([na, nb]),
                    sill: f64::INFINITY,
                    over: None,
                    lower: e,
                });
                Part {
                    node: Some(k),
                    single: None,
                    pit: pa.pit.min(pb.pit),
                }
            }
        };
        let r = ra.min(rb);
        root[ra as usize] = r;
        root[rb as usize] = r;
        parts[r as usize] = merged;
    }
    renumber(z, flood, &into, &raise, &nodes)
}

/// Raise filled hollows' cells, drop filled hollows from the tree and number what is left:
/// the live hollows in order of their pits, then the joined nodes in the order they formed.
fn renumber(z: &[f64], flood: &Flood, into: &[u32], raise: &[f64], nodes: &[Node]) -> Joined {
    let leaves = flood.pits.len();
    let live: Vec<bool> = (1..=leaves).map(|l| into[l] as usize == l).collect();
    let mut number = vec![u32::MAX; nodes.len()];
    let mut next = 0u32;
    for (k, _) in live.iter().enumerate().filter(|(_, l)| **l) {
        number[k] = next;
        next += 1;
    }
    for slot in number.iter_mut().skip(leaves) {
        *slot = next;
        next += 1;
    }
    let map = |k: u32| number[k as usize];
    let leaf_of = |l: u32| (l > 0).then(|| map(l - 1));
    let out: Vec<Node> = nodes
        .iter()
        .enumerate()
        .filter(|(k, _)| *k >= leaves || live[*k])
        .map(|(_, n)| Node {
            parent: n.parent.map(map),
            children: n.children.map(|c| c.map(map)),
            sill: n.sill,
            over: n.over.map(map),
            lower: n.lower,
        })
        .collect();
    let mut heights = z.to_vec();
    let labels = flood
        .labels
        .iter()
        .enumerate()
        .map(|(i, &l)| {
            let mut l = l;
            while into[l as usize] != l {
                heights[i] = heights[i].max(raise[l as usize]);
                l = into[l as usize];
            }
            leaf_of(l).unwrap_or(NO_HOLLOW)
        })
        .collect();
    let pits = flood
        .pits
        .iter()
        .zip(&live)
        .filter_map(|(&p, &l)| l.then_some(p))
        .collect();
    Joined {
        heights,
        labels,
        pits,
        nodes: out,
    }
}
