//! The fill step: once a tick, in f64 and a fixed order, rain goes to its first receiver,
//! puddles lose what dries and goes down, and full hollows spill, merge and part.
//!
//! Water stands in a set of nodes, none an ancestor of another, that covers every hollow:
//! each such node is one puddle, at one level. A full node's excess goes to the hollow
//! across its sill; if the node there is full to the same sill, the two become their parent
//! (a merge). A joined puddle whose volume falls under what its two halves hold at their
//! sill becomes the two again (a parting), each keeping its side's water.
use crate::RainError;
use crate::tree::Tree;

/// One tick's rain record: both uniform over the body and held over the tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rain {
    /// Rain, kilograms of water a square metre a second (1 mm/h is 1/3600).
    pub flux: f64,
    /// Drying from water surfaces, in the same units.
    pub drying: f64,
}

/// The ground a hollows body is made of, as M1's water sees it: bare ground passes water
/// down at its percolation rate and the rest runs off; a puddle loses water at the same
/// rate through its bed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundClass {
    /// Kilograms of water a square metre a second the ground passes down.
    pub percolation: f64,
}

/// Running totals of where the body's rain went, cubic metres since the state began.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Account {
    /// Rain on the body's whole rectangle.
    pub rain: f64,
    /// Withheld by the canopy: its named sink.
    pub canopy: f64,
    /// Gone down into bare ground.
    pub ground: f64,
    /// Gone down through puddles' beds.
    pub percolated: f64,
    /// Dried from puddles.
    pub dried: f64,
    /// Left over the rectangle's edge: runoff and spill.
    pub edge: f64,
    /// Held in puddles when the state began.
    pub initial: f64,
}

/// What one hollow's catchment gained in a tick, cubic metres.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LeafGain {
    /// Rain that fell on this hollow's water, by the exposure there.
    pub on_water: f64,
    /// Rain on this hollow's bare ground that ran off to its puddle.
    pub runoff: f64,
    /// Withheld by the canopy over this hollow's cells.
    pub canopy: f64,
    /// Gone down into this hollow's bare ground.
    pub ground: f64,
}

/// One tick's receivers, each by itself, cubic metres. Reused from tick to tick.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TickReport {
    /// Each hollow's gains.
    pub hollows: Vec<LeafGain>,
    /// Gains of the cells whose water leaves over the edge: their runoff left.
    pub edge_cells: LeafGain,
    /// Water that spilled out of the tree over the edge.
    pub edge_spill: f64,
    /// Dried from puddles.
    pub dried: f64,
    /// Gone down through puddles' beds.
    pub percolated: f64,
    /// Merges in the tick.
    pub merges: u32,
    /// Partings in the tick.
    pub partings: u32,
}

/// The rain state a game keeps in its world: a volume for each puddle and the account.
#[derive(Clone, Debug, PartialEq)]
pub struct RainState {
    pub(crate) tick: u64,
    pub(crate) volume: Vec<f64>,
    pub(crate) active: Vec<bool>,
    pub(crate) account: Account,
}

impl RainState {
    /// Dry hollows, each its own puddle.
    pub fn new(tree: &Tree) -> Self {
        let n = tree.node_count();
        let h = tree.hollow_count();
        Self {
            tick: 0,
            volume: vec![0.0; n],
            active: (0..n).map(|k| k < h).collect(),
            account: Account::default(),
        }
    }

    /// Ticks stepped.
    pub fn tick(&self) -> u64 {
        self.tick
    }

    /// The account so far.
    pub fn account(&self) -> &Account {
        &self.account
    }

    /// The puddles: each node that holds water as one puddle, with its volume.
    pub fn puddles(&self) -> impl Iterator<Item = (u32, f64)> + '_ {
        self.active
            .iter()
            .zip(&self.volume)
            .enumerate()
            .filter(|(_, (a, _))| **a)
            .map(|(k, (_, &v))| (id(k), v))
    }

    /// Cubic metres held in every puddle.
    pub fn held(&self) -> f64 {
        self.puddles().map(|(_, v)| v).sum()
    }

    /// The node whose puddle a hollow is part of.
    /// # Panics
    /// If the state belongs to another tree.
    pub fn holder(&self, tree: &Tree, hollow: u32) -> u32 {
        let mut k = hollow;
        while !self.active[k as usize] {
            k = tree.nodes[k as usize]
                .parent
                .expect("the puddles cover every hollow");
        }
        k
    }

    /// A puddle's level: its node's level for its volume, or a dry hollow's pit.
    pub fn level(&self, tree: &Tree, node: u32) -> f64 {
        let v = self.volume[node as usize];
        if v > 0.0 {
            tree.level(node, v)
        } else {
            tree.node(node).lower
        }
    }

    /// The level of each hollow: its puddle's. The map's table for this tick.
    pub fn levels(&self, tree: &Tree) -> Vec<f64> {
        (0..tree.hollow_count())
            .map(|h| {
                let h = id(h);
                self.level(tree, self.holder(tree, h))
            })
            .collect()
    }

    /// Put water in one hollow, as if spilled there, and let it settle. For tests and for a
    /// game that starts with puddles. Counted as water the state began with.
    /// # Errors
    /// Refuses a negative or nonfinite volume or an unknown hollow.
    pub fn pour(&mut self, tree: &Tree, hollow: u32, volume: f64) -> Result<(), RainError> {
        if !volume.is_finite() || volume < 0.0 || hollow as usize >= tree.hollow_count() {
            return Err(RainError::Input("invalid pour"));
        }
        let k = self.holder(tree, hollow);
        self.volume[k as usize] += volume;
        self.account.initial += volume;
        let mut report = TickReport::default();
        self.settle(tree, &mut report);
        self.account.initial -= report.edge_spill;
        Ok(())
    }

    /// Step one tick of `dt` seconds. The per-hollow work is spread over `threads`; the
    /// result is the same bytes for any count.
    /// # Errors
    /// Refuses a negative or nonfinite rain, drying, percolation or step, a state that
    /// belongs to another tree, and no threads, without changing state.
    pub fn step(
        &mut self,
        tree: &Tree,
        rain: Rain,
        class: GroundClass,
        dt: f64,
        threads: usize,
        report: &mut TickReport,
    ) -> Result<(), RainError> {
        for v in [rain.flux, rain.drying, class.percolation, dt] {
            if !v.is_finite() || v < 0.0 {
                return Err(RainError::Input(
                    "negative or nonfinite rain, drying or step",
                ));
            }
        }
        if threads == 0 {
            return Err(RainError::Input("no threads"));
        }
        if self.volume.len() != tree.node_count() {
            return Err(RainError::State("state belongs to another tree"));
        }
        let fall = rain.flux / 1000.0 * dt;
        let soak = class.percolation / 1000.0 * dt;
        let area = tree.ground().spacing().powi(2);
        let h = tree.hollow_count();
        *report = TickReport {
            hollows: std::mem::take(&mut report.hollows),
            ..TickReport::default()
        };
        report.hollows.resize(h, LeafGain::default());
        // Each hollow's level at the start of the tick sets which of its cells are covered.
        let levels: Vec<f64> = (0..h)
            .map(|l| {
                let k = self.holder(tree, id(l));
                if self.volume[k as usize] > 0.0 {
                    tree.level(k, self.volume[k as usize])
                } else {
                    f64::NEG_INFINITY
                }
            })
            .collect();
        let gather = |l: usize| {
            let g = tree.catchment.gather(l, levels[l], fall, soak, area);
            LeafGain {
                on_water: g.water,
                runoff: g.runoff,
                canopy: g.canopy,
                ground: g.ground,
            }
        };
        let chunk = h.div_ceil(threads).max(1);
        std::thread::scope(|scope| {
            let mut rest = report.hollows.as_mut_slice();
            let mut first = 0;
            while !rest.is_empty() {
                let (part, tail) = rest.split_at_mut(chunk.min(rest.len()));
                let start = first;
                first += part.len();
                rest = tail;
                let mut work = move || {
                    for (i, out) in part.iter_mut().enumerate() {
                        *out = gather(start + i);
                    }
                };
                if threads == 1 || rest.is_empty() {
                    work();
                } else {
                    scope.spawn(work);
                }
            }
        });
        let edge = tree
            .catchment
            .gather(h, f64::NEG_INFINITY, fall, soak, area);
        report.edge_cells = LeafGain {
            on_water: 0.0,
            runoff: edge.runoff,
            canopy: edge.canopy,
            ground: edge.ground,
        };
        let a = &mut self.account;
        a.rain += fall * area * tree.ground().heights().len() as f64;
        a.canopy += edge.canopy;
        a.ground += edge.ground;
        a.edge += edge.runoff;
        for l in 0..h {
            let g = report.hollows[l];
            self.account.canopy += g.canopy;
            self.account.ground += g.ground;
            let k = self.holder(tree, id(l));
            self.volume[k as usize] += g.on_water + g.runoff;
        }
        self.lose(tree, rain.drying / 1000.0 * dt, soak, report);
        self.settle(tree, report);
        self.account.edge += report.edge_spill;
        self.tick += 1;
        Ok(())
    }

    /// Drying and percolation from each puddle's water surface after the tick's rain.
    fn lose(&mut self, tree: &Tree, dry: f64, soak: f64, report: &mut TickReport) {
        if dry + soak <= 0.0 {
            return;
        }
        for k in 0..self.volume.len() {
            let v = self.volume[k];
            if !self.active[k] || v <= 0.0 {
                continue;
            }
            let node = id(k);
            let surface = tree.surface(node, tree.level(node, v));
            let take = v.min((dry + soak) * surface);
            let dried = take * dry / (dry + soak);
            self.volume[k] = v - take;
            report.dried += dried;
            report.percolated += take - dried;
        }
        self.account.dried += report.dried;
        self.account.percolated += report.percolated;
    }

    /// Spill, merge and part until every puddle stands within its node.
    pub(crate) fn settle(&mut self, tree: &Tree, report: &mut TickReport) {
        let mut changed = true;
        while changed {
            changed = false;
            for k in 0..self.volume.len() {
                if self.active[k] && self.settle_one(tree, k, report) {
                    changed = true;
                }
            }
        }
    }

    fn settle_one(&mut self, tree: &Tree, k: usize, report: &mut TickReport) -> bool {
        let node = tree.nodes[k];
        let id = id(k);
        if let Some([a, b]) = node.children {
            let (ca, cb) = (tree.capacity(a), tree.capacity(b));
            if self.volume[k] < ca + cb {
                // Part at the sill: each side gives up its share of the shortfall.
                let short = ca + cb - self.volume[k];
                let (sa, sb) = (tree.surface(a, node.lower), tree.surface(b, node.lower));
                let mut va = ca - short * sa / (sa + sb);
                let mut vb = ca + cb - short - va;
                if va < 0.0 {
                    vb += va;
                    va = 0.0;
                }
                if vb < 0.0 {
                    va += vb;
                    vb = 0.0;
                }
                self.active[k] = false;
                self.volume[k] = 0.0;
                for (child, v) in [(a, va), (b, vb)] {
                    self.active[child as usize] = true;
                    self.volume[child as usize] = v;
                }
                report.partings += 1;
                return true;
            }
        }
        let cap = tree.capacity(id);
        let over = self.volume[k] - cap;
        if over <= 0.0 {
            return false;
        }
        if let Some(p) = node.parent {
            let [a, b] = tree.nodes[p as usize]
                .children
                .expect("a parent has two children");
            let sibling = if a == id { b } else { a };
            if self.active[sibling as usize]
                && self.volume[sibling as usize] >= tree.capacity(sibling)
            {
                // Both full to one sill: one puddle at one level.
                self.volume[p as usize] = self.volume[k] + self.volume[sibling as usize];
                self.active[p as usize] = true;
                for c in [a, b] {
                    self.active[c as usize] = false;
                    self.volume[c as usize] = 0.0;
                }
                report.merges += 1;
                return true;
            }
        }
        self.volume[k] = cap;
        match node.over {
            Some(hollow) => {
                let target = self.holder(tree, hollow);
                self.volume[target as usize] += over;
            }
            None => report.edge_spill += over,
        }
        true
    }
}

/// A node's or hollow's number; the profile's counts keep every count within `u32`.
pub(crate) fn id(k: usize) -> u32 {
    u32::try_from(k).expect("counts fit the profile's limits")
}
