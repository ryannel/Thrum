//! The fill step: once a tick, in f64 and a fixed order, rain goes to its first receiver,
//! puddles lose what dries and goes down, and full hollows spill, merge and part.
//!
//! Water stands in a set of nodes, none an ancestor of another, that covers every hollow:
//! each such node is one puddle, at one level. A full node's excess goes to the hollow
//! across its sill; if the node there is full to the same sill, the two become their parent
//! (a merge). A joined puddle whose volume falls under what its two halves hold at their
//! sill becomes the two again (a parting), each keeping its side's water.
use crate::RainError;
use crate::ground::{Flow, GroundClass, GroundStores, Store};
use crate::tree::Tree;
use crate::tree::catchment::Gathered;

/// One tick's rain record: both uniform over the body and held over the tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rain {
    /// Rain, kilograms of water a square metre a second (1 mm/h is 1/3600).
    pub flux: f64,
    /// Drying from water surfaces, in the same units.
    pub drying: f64,
}

/// Running totals of where the body's rain went, cubic metres since the state began.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Account {
    /// Rain on the body's whole rectangle.
    pub rain: f64,
    /// Withheld by the canopy: its named sink.
    pub canopy: f64,
    /// Gone down into ground no water stands on: open ground and held ground.
    pub ground: f64,
    /// Gone down through puddles' beds.
    pub percolated: f64,
    /// Dried from puddles.
    pub dried: f64,
    /// Left over the rectangle's edge: runoff and spill.
    pub edge: f64,
    /// Held in puddles when the state began.
    pub initial: f64,
    /// Dried from uncovered ground's surface water and pores.
    pub ground_dried: f64,
    /// What full pores held beyond their class's fill when water left them.
    pub soaked: f64,
}

impl Account {
    /// Every sink, cubic metres: canopy, ground, puddles' beds, drying, the edge and
    /// soaking.
    pub fn sinks(&self) -> f64 {
        self.canopy
            + self.ground
            + self.percolated
            + self.dried
            + self.edge
            + self.ground_dried
            + self.soaked
    }
}

/// The stores' reading of one catchment's open ground, cubic metres: each cell at or
/// above `mark` at its reach's store water.
fn open_reading(tree: &Tree, l: usize, mark: f64, vol: &[f64; 3], area: f64) -> f64 {
    let (_, w) = tree.catchment.band(l, mark, f64::INFINITY);
    (0..3).map(|r| w[r] * area * vol[r]).sum::<f64>()
}

/// What a square metre of held ground does with a tick's rain on it, metres: into its
/// share of the pores under the mark, down, and runoff. The step a class's store takes,
/// with no relief.
fn held_take(class: &GroundClass, water: f64, room: f64, dt: f64) -> [f64; 3] {
    let perc = class.percolation / 1000.0 * dt;
    let (take, into) = if room > 0.0 {
        let take = water.min(class.soak / 1000.0 * dt).min(room + perc);
        (take, take.min(room))
    } else {
        (water.min(perc), 0.0)
    };
    [into, take - into, water - take]
}

/// What one hollow's catchment gained in a tick, cubic metres.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LeafGain {
    /// Rain that fell on this hollow's water, by the exposure there.
    pub on_water: f64,
    /// Rain on this hollow's open ground that ran off to its puddle.
    pub runoff: f64,
    /// Withheld by the canopy over this hollow's cells.
    pub canopy: f64,
    /// Gone down into this hollow's open ground.
    pub ground: f64,
    /// Dried from this hollow's open ground.
    pub ground_dried: f64,
    /// Rain on this hollow's held ground, by the exposure at the ground: the three
    /// entries below.
    pub on_held: f64,
    /// Held ground's rain its share of the pores under the mark took.
    pub held_into: f64,
    /// Held ground's rain that went down, a named sink.
    pub held_down: f64,
    /// Held ground's rain that ran off to the puddle.
    pub held_runoff: f64,
    /// Surface water that joined this hollow's puddle as its mark rose over ground.
    pub joined: f64,
    /// Pore water that joined the pores under the mark as it rose over ground.
    pub under: f64,
    /// Surface water this hollow's puddle left on ground its mark fell from.
    pub left: f64,
    /// Water the puddle gave to bring that ground's pores up to the class's fill.
    pub topped: f64,
    /// What that ground's share of the pores under the mark held beyond the class's fill.
    pub soaked: f64,
    /// What the puddle lacked to hand over the ground between its level and this
    /// hollow's mark: above nothing, the hollow holds that ground and nothing moved.
    pub short: f64,
    /// What the bed under this hollow's water took from the puddle.
    pub bed: f64,
    /// The part of the bed's take the pores under the mark kept; the rest went down.
    pub into: f64,
    /// Dried from the puddle's water over this hollow.
    pub dried: f64,
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

/// The rain state a game keeps in its world: a volume for each puddle; for each hollow
/// its mark, the water in the pores under the mark and a damp band above its waterline;
/// and the account. The ground's stores are the game's [`GroundStores`]. Nothing here
/// stands for water not yet delivered.
#[derive(Clone, Debug, PartialEq)]
pub struct RainState {
    pub(crate) tick: u64,
    pub(crate) volume: Vec<f64>,
    pub(crate) active: Vec<bool>,
    pub(crate) account: Account,
    pub(crate) mark: Vec<f64>,
    pub(crate) band: Vec<f64>,
    pub(crate) under: Vec<f64>,
    pub(crate) ground_tick: Option<u64>,
}

/// How a step reads the ground: a stateless class, or a class of the game's stores.
#[derive(Clone, Copy)]
enum Reading<'a> {
    Bare(GroundClass),
    Stores(&'a GroundStores, usize),
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
            mark: (0..h).map(|k| tree.nodes[k].lower).collect(),
            band: vec![0.0; h],
            under: vec![0.0; h],
            ground_tick: None,
        }
    }

    /// Each hollow's mark, a height: its ground below the mark is in the hollow's
    /// account and not the class's stores'. Ground between the puddle's level and the mark
    /// is held ground. A mark never says where water stands; the level does.
    pub fn marks(&self) -> &[f64] {
        &self.mark
    }

    /// Water in the pores of each hollow's ground under its mark, cubic metres, spread
    /// evenly over that ground.
    pub fn pores_under(&self) -> &[f64] {
        &self.under
    }

    /// Height of the damp band above each hollow's waterline, metres.
    pub fn bands(&self) -> &[f64] {
        &self.band
    }

    /// Fill over the area held under a hollow's mark.
    pub fn held_fill(&self, tree: &Tree, class: &GroundClass, hollow: usize) -> f64 {
        let held_capacity = class.pores
            * tree
                .catchment
                .band(hollow, f64::NEG_INFINITY, self.mark[hollow])
                .0 as f64
            * tree.ground().spacing().powi(2);
        if held_capacity > 0.0 {
            (self.under[hollow] / held_capacity).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// The wetness at a point (x, y, z) on a hollows body's ground: full pores and no
    /// film under a puddle; on held ground the fill of the pores under its hollow's mark
    /// and no film; elsewhere the class's stores read at the point. Ground above a
    /// waterline takes the damp band. None off the body.
    #[expect(
        clippy::too_many_arguments,
        reason = "the wetness function's fields and the state"
    )]
    pub fn wetness(
        &self,
        tree: &Tree,
        ground: &GroundStores,
        class: usize,
        canopy: &crate::Canopy,
        point: [f64; 3],
        normal: [f64; 3],
        pattern: Option<f64>,
    ) -> Option<crate::Wetness> {
        let g = tree.ground();
        let local = [
            (point[0] - g.origin()[0]) / g.spacing(),
            (point[2] - g.origin()[1]) / g.spacing(),
        ];
        let [nx, nz] = g.size();
        if local.iter().any(|v| *v < 0.0) || local[0] >= nx as f64 || local[1] >= nz as f64 {
            return None;
        }
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "a cell index inside the grid"
        )]
        let i = local[1] as usize * nx + local[0] as usize;
        let base = crate::wetness_at(ground, class, canopy, point, normal, pattern);
        let label = tree.labels()[i];
        if label == thrum_water_model::NO_HOLLOW {
            return Some(base);
        }
        let level = self.level(tree, self.holder(tree, label));
        let pit = tree.node(label).lower;
        if level > pit && point[1] < level {
            return Some(crate::Wetness {
                pores: 1.0,
                surface: 0.0,
                cover: 0.0,
            });
        }
        let l = label as usize;
        let fill = if self.band[l] > 0.0 {
            crate::band_fill(point[1] - level, self.band[l])
        } else {
            0.0
        };
        if self.mark[l] > pit && point[1] < self.mark[l] {
            let held = crate::Wetness {
                pores: self.held_fill(tree, &ground.classes[class], l),
                surface: 0.0,
                cover: 0.0,
            };
            return Some(held.with_band(fill));
        }
        Some(base.with_band(fill))
    }

    /// Cubic metres the ground of the body holds: the class's stores over its open
    /// ground, read at each cell's exposure, and the pores under every mark.
    pub fn ground_held(&self, tree: &Tree, ground: &GroundStores, class: usize) -> f64 {
        let c = ground.classes[class];
        let area = tree.ground().spacing().powi(2);
        let h = tree.hollow_count();
        let vol: [f64; 3] = std::array::from_fn(|r| ground.stores[class][r].water(c.pores));
        (0..h)
            .map(|l| self.hollow_ground(tree, ground, class, l))
            .sum::<f64>()
            + open_reading(tree, h, f64::NEG_INFINITY, &vol, area)
    }

    /// Cubic metres the ground draining to one hollow holds: the stores' reading of its
    /// open ground and the pores under its mark.
    pub fn hollow_ground(
        &self,
        tree: &Tree,
        ground: &GroundStores,
        class: usize,
        hollow: usize,
    ) -> f64 {
        let c = ground.classes[class];
        let area = tree.ground().spacing().powi(2);
        let vol: [f64; 3] = std::array::from_fn(|r| ground.stores[class][r].water(c.pores));
        open_reading(tree, hollow, self.mark[hollow], &vol, area) + self.under[hollow]
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

    /// Step one tick of `dt` seconds on ground of a stateless class: no pores and no
    /// relief. The per-hollow work is spread over `threads`; the result is the same bytes
    /// for any count.
    /// # Errors
    /// Refuses a class that keeps state, a negative or nonfinite rain, drying,
    /// percolation or step, a state that belongs to another tree, and no threads, without
    /// changing state.
    pub fn step(
        &mut self,
        tree: &Tree,
        rain: Rain,
        class: GroundClass,
        dt: f64,
        threads: usize,
        report: &mut TickReport,
    ) -> Result<(), RainError> {
        class.validate()?;
        if class.stores() {
            return Err(RainError::Input(
                "a ground class with pores or relief is stepped with its stores",
            ));
        }
        self.step_reading(tree, rain, Reading::Bare(class), dt, threads, report)
    }

    /// Step one tick of `dt` seconds on ground of `class` in the game's stores. The
    /// stores are stepped after this, once a tick: this reads them as they stand at the
    /// tick's start, and the flows the tick will give them.
    /// # Errors
    /// As [`RainState::step`], and refuses an unknown class or stores not stepped once
    /// since the last tick.
    pub fn step_with(
        &mut self,
        tree: &Tree,
        rain: Rain,
        ground: &GroundStores,
        class: usize,
        dt: f64,
        threads: usize,
        report: &mut TickReport,
    ) -> Result<(), RainError> {
        if class >= ground.classes.len() {
            return Err(RainError::Input("unknown ground class"));
        }
        if self.ground_tick.is_some_and(|t| ground.tick != t + 1) {
            return Err(RainError::Input(
                "the ground's stores were not stepped once since the last tick",
            ));
        }
        self.step_reading(
            tree,
            rain,
            Reading::Stores(ground, class),
            dt,
            threads,
            report,
        )?;
        self.ground_tick = Some(ground.tick);
        Ok(())
    }

    fn step_reading(
        &mut self,
        tree: &Tree,
        rain: Rain,
        reading: Reading<'_>,
        dt: f64,
        threads: usize,
        report: &mut TickReport,
    ) -> Result<(), RainError> {
        for v in [rain.flux, rain.drying, dt] {
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
        let (class, flows, stores) = match reading {
            Reading::Bare(c) => (c, [Flow::default(); 3], None),
            Reading::Stores(g, k) => (g.classes[k], g.flows(rain, dt)?[k], Some(g.stores[k])),
        };
        let fall = rain.flux / 1000.0 * dt;
        let bare = match reading {
            Reading::Bare(c) => Some(c.percolation / 1000.0 * dt),
            Reading::Stores(..) => None,
        };
        let soak = class.percolation / 1000.0 * dt;
        let area = tree.ground().spacing().powi(2);
        let h = tree.hollow_count();
        *report = TickReport {
            hollows: std::mem::take(&mut report.hollows),
            ..TickReport::default()
        };
        report.hollows.clear();
        report.hollows.resize(h, LeafGain::default());
        // The tick's footprints are those of its start: water stands where the level then
        // is above the bed.
        let start: Vec<f64> = (0..h)
            .map(|l| self.level(tree, self.holder(tree, id(l))))
            .collect();
        if let Some(stores) = stores {
            self.move_marks(tree, &start, &class, &stores, area, report);
        } else {
            self.mark.clone_from(&start);
        }
        // Rain goes to what it hits: water below the level at the tick's start, held
        // ground from there up to the mark, open ground above the mark.
        let (marks, under) = (&self.mark, &self.under);
        let levels = &start;
        let gather = |l: usize| {
            let g = tree.catchment.gather(l, levels[l], fall, bare, area);
            if bare.is_some() {
                return leaf(g, &flows, true);
            }
            let (_, w) = tree.catchment.band(l, marks[l], f64::INFINITY);
            let mut out = leaf(
                Gathered {
                    weights: w.map(|v| v * area),
                    ..g
                },
                &flows,
                false,
            );
            if marks[l] > levels[l] {
                // Held ground's pores are its share, by area, of the pores under the mark.
                let below = tree.catchment.band(l, f64::NEG_INFINITY, marks[l]).0 as f64;
                let room = (class.pores * below * area - under[l]).max(0.0) / (below * area);
                let (open, shaded) = tree.catchment.exposures(l, levels[l], marks[l]);
                let mut add = |e: f64, cells: f64| {
                    let [into, down, runoff] = held_take(&class, e * fall, room, dt);
                    out.on_held += e * fall * cells * area;
                    out.held_into += into * cells * area;
                    out.held_down += down * cells * area;
                    out.held_runoff += runoff * cells * area;
                };
                if open > 0 {
                    add(1.0, f64::from(open));
                }
                for e in shaded {
                    add(e, 1.0);
                }
            }
            out
        };
        let chunk = h.div_ceil(threads).max(1);
        std::thread::scope(|scope| {
            let mut rest = report.hollows.as_mut_slice();
            let mut first = 0;
            while !rest.is_empty() {
                let (part, tail) = rest.split_at_mut(chunk.min(rest.len()));
                let begin = first;
                first += part.len();
                rest = tail;
                let mut work = move || {
                    for (i, out) in part.iter_mut().enumerate() {
                        let g = gather(begin + i);
                        out.on_water = g.on_water;
                        out.runoff = g.runoff;
                        out.canopy = g.canopy;
                        out.ground = g.ground;
                        out.ground_dried = g.ground_dried;
                        out.on_held = g.on_held;
                        out.held_into = g.held_into;
                        out.held_down = g.held_down;
                        out.held_runoff = g.held_runoff;
                    }
                };
                if threads == 1 || rest.is_empty() {
                    work();
                } else {
                    scope.spawn(work);
                }
            }
        });
        let edge = leaf(
            tree.catchment
                .gather(h, f64::NEG_INFINITY, fall, bare, area),
            &flows,
            bare.is_some(),
        );
        report.edge_cells = LeafGain {
            on_water: 0.0,
            ..edge
        };
        let a = &mut self.account;
        a.rain += fall * area * tree.ground().heights().len() as f64;
        a.canopy += edge.canopy;
        a.ground += edge.ground;
        a.ground_dried += edge.ground_dried;
        a.edge += edge.runoff;
        for l in 0..h {
            let g = report.hollows[l];
            self.account.canopy += g.canopy;
            self.account.ground += g.ground + g.held_down;
            self.account.ground_dried += g.ground_dried;
            self.under[l] += g.held_into;
            let k = self.holder(tree, id(l));
            self.volume[k as usize] += g.on_water + g.runoff + g.held_runoff;
        }
        let dry = rain.drying / 1000.0 * dt;
        if stores.is_some() {
            self.lose_to_beds(tree, dry, &class, dt, area, report);
        } else {
            self.lose(tree, dry, soak, report);
        }
        self.settle(tree, report);
        self.account.edge += report.edge_spill;
        // Each hollow's damp band follows the fall of its level over the tick.
        for (l, &before) in start.iter().enumerate().take(h) {
            let after = self.level(tree, self.holder(tree, id(l)));
            let rim = tree.nodes[l].sill - after;
            self.band[l] = crate::ground::band_step(
                self.band[l],
                before,
                after,
                dt,
                rain.drying,
                class.pores,
                rim,
            );
        }
        self.tick += 1;
        Ok(())
    }

    /// Each hollow's mark against its puddle's level at the tick's start. Ground a level
    /// has risen over goes under the mark: its surface water joins the puddle and its
    /// pore water the pores under the mark. Ground a level has fallen from is due back to
    /// the stores in its class's state: its pore water from its share, by area, of the
    /// pores under the mark, what the share holds beyond the class's fill counted as
    /// soaked, and what it lacks, with the class's surface water, from the puddle. A
    /// puddle that holds less keeps the ground: nothing moves and the mark stays. Hollows
    /// that share a puddle draw on it in their order.
    #[expect(
        clippy::float_cmp,
        reason = "only a level that differs at all from the mark moves ground"
    )]
    fn move_marks(
        &mut self,
        tree: &Tree,
        now: &[f64],
        class: &GroundClass,
        stores: &[Store; 3],
        area: f64,
        report: &mut TickReport,
    ) {
        for (l, &new) in now.iter().enumerate() {
            let old = self.mark[l];
            if old == new {
                continue;
            }
            let (n, w) = tree.catchment.band(l, old.min(new), old.max(new));
            let surface: f64 = (0..3).map(|r| w[r] * stores[r].surface).sum::<f64>() * area;
            let pore: f64 = (0..3).map(|r| w[r] * stores[r].fill).sum::<f64>() * class.pores * area;
            let k = self.holder(tree, id(l)) as usize;
            let g = &mut report.hollows[l];
            if new > old {
                self.volume[k] += surface;
                self.under[l] += pore;
                g.joined = surface;
                g.under = pore;
            } else if n > 0 {
                let below = tree.catchment.band(l, f64::NEG_INFINITY, old).0;
                let share = self.under[l] * n as f64 / below as f64;
                let topped = (pore - share).max(0.0);
                let need = surface + topped;
                if self.volume[k] < need {
                    g.short = need - self.volume[k];
                    continue;
                }
                self.volume[k] -= need;
                self.under[l] = if n == below {
                    0.0
                } else {
                    self.under[l] - share
                };
                g.left = surface;
                g.topped = topped;
                g.soaked = (share - pore).max(0.0);
                self.account.soaked += g.soaked;
            }
            self.mark[l] = new;
        }
    }

    /// Drying and the beds' take from each puddle's water surface after the tick's rain
    /// and runoff. In each hollow the bed takes the class's soak rate over the water there
    /// while the pores under the mark have room, of which they keep what they have room
    /// for and the rest goes down, at no more than the percolation rate; over full pores,
    /// and on ground with none, it takes the percolation rate. A puddle that holds less
    /// than the tick's drying and take gives each its share of what it holds and is empty.
    fn lose_to_beds(
        &mut self,
        tree: &Tree,
        dry: f64,
        class: &GroundClass,
        dt: f64,
        area: f64,
        report: &mut TickReport,
    ) {
        let (soak, perc) = (class.soak / 1000.0 * dt, class.percolation / 1000.0 * dt);
        let h = report.hollows.len();
        let mut want = vec![0.0; self.volume.len()];
        let holders: Vec<usize> = (0..h).map(|l| self.holder(tree, id(l)) as usize).collect();
        for (l, &k) in holders.iter().enumerate() {
            let v = self.volume[k];
            if v <= 0.0 {
                continue;
            }
            let level = tree.level(id(k), v);
            let surface = tree.catchment.band(l, f64::NEG_INFINITY, level).0 as f64 * area;
            let below = tree.catchment.band(l, f64::NEG_INFINITY, self.mark[l]).0 as f64;
            let room = (class.pores * below * area - self.under[l]).max(0.0);
            let take = if room > 0.0 {
                (soak * surface).min(room + perc * surface)
            } else {
                perc * surface
            };
            let g = &mut report.hollows[l];
            g.bed = take;
            g.into = take.min(room);
            g.dried = dry * surface;
            want[k] += g.dried + take;
        }
        for (l, &k) in holders.iter().enumerate() {
            let g = &mut report.hollows[l];
            if want[k] > self.volume[k] {
                let share = self.volume[k] / want[k];
                g.bed *= share;
                g.into *= share;
                g.dried *= share;
            }
            self.under[l] += g.into;
            report.dried += g.dried;
            report.percolated += g.bed - g.into;
        }
        for (v, w) in self.volume.iter_mut().zip(want) {
            *v = if w >= *v { 0.0 } else { *v - w };
        }
        self.account.dried += report.dried;
        self.account.percolated += report.percolated;
    }

    /// Drying and percolation from each puddle's water surface after the tick's rain, on
    /// ground of a stateless class.
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

/// One catchment's gains: by the cells' own take on stateless ground, or from the
/// stores' flows at the open cells' weights.
fn leaf(g: Gathered, flows: &[Flow; 3], bare: bool) -> LeafGain {
    let mut out = LeafGain {
        on_water: g.water,
        canopy: g.canopy,
        ..LeafGain::default()
    };
    if bare {
        out.runoff = g.runoff;
        out.ground = g.ground;
    } else {
        for (w, f) in g.weights.iter().zip(flows) {
            out.runoff += w * f.runoff;
            out.ground += w * f.percolated;
            out.ground_dried += w * f.dried;
        }
    }
    out
}

/// A node's or hollow's number; the profile's counts keep every count within `u32`.
pub(crate) fn id(k: usize) -> u32 {
    u32::try_from(k).expect("counts fit the profile's limits")
}
