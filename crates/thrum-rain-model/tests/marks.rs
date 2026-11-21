//! M2.11, M2.12 and M2.13: a hollow's mark, the pores under it, held ground and the bed
//! under a puddle.
//!
//! Every tick of every case goes through [`World::tick`]. Before the model steps, it works
//! out each entry the tick should make, cell by cell, from the bed, the shelters, the
//! class's stores and the state as it stands: the puddles' water, the levels, the marks
//! and the pores under them. Water stands on a cell where its hollow's level at the
//! tick's start is above its bed; the mark is read only to tell held ground from open
//! ground. It then steps the model and compares each entry by itself, the marks, the pores
//! under them and the puddles, and closes the account. No entry may be more than its donor
//! held before it.
#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "small counts of cells, hollows and ticks"
)]
#![expect(
    clippy::float_cmp,
    reason = "entries that must be exactly nothing, and marks that are the level's own bits"
)]
use crate::support;
use support::{ground, mm_h, pair};
use thrum_rain_model::{
    Canopy, Ground, GroundClass, GroundStores, Profile, Rain, RainState, Shelter, TickReport, Tree,
    band_fill,
};
use thrum_water_model::NO_HOLLOW;

/// A class of `soak`, `percolation` (mm/h), `pores` and `relief` (mm).
fn class(soak: f64, percolation: f64, pores: f64, relief: f64) -> GroundClass {
    GroundClass {
        soak: mm_h(soak),
        percolation: mm_h(percolation),
        pores: pores / 1000.0,
        relief: relief / 1000.0,
        steepest: 1.0,
        share: 0.47,
        porosity: 1.0,
        film_roughness: 0.05,
    }
}

fn rain(flux: f64, drying: f64) -> Rain {
    Rain {
        flux: mm_h(flux),
        drying: mm_h(drying),
    }
}

/// The test's own weights of the stored reaches for an exposure.
fn weights(e: f64) -> [f64; 3] {
    if e >= 0.5 {
        [(e - 0.5) / 0.5, 1.0 - (e - 0.5) / 0.5, 0.0]
    } else if e >= 0.25 {
        [0.0, (e - 0.25) / 0.25, 1.0 - (e - 0.25) / 0.25]
    } else {
        [0.0, 0.0, e / 0.25]
    }
}

/// The level of `volume` standing over cells of these heights, sorted upward.
fn level_of(heights: &[f64], volume: f64, area: f64) -> f64 {
    let mut held = 0.0;
    for (k, &z) in heights.iter().enumerate() {
        let next = heights.get(k + 1).copied().unwrap_or(f64::INFINITY);
        let room = (next - z) * (k + 1) as f64 * area;
        if held + room >= volume {
            return z + (volume - held) / ((k + 1) as f64 * area);
        }
        held += room;
    }
    unreachable!("the last cell's room is unbounded")
}

/// `model` against the test's own figure, within 1e-9 of it.
fn close(what: &str, model: f64, test: f64, at: &str) {
    assert!(
        (model - test).abs() <= 1e-9 * test.abs().max(model.abs()) + 1e-18,
        "{what} {at}: the model gives {model:e}, the test {test:e}"
    );
}

/// One hollow's entries in a tick, cubic metres: the test's own figures.
#[derive(Clone, Copy, Debug, Default)]
struct Entries {
    joined: f64,
    under: f64,
    left: f64,
    topped: f64,
    soaked: f64,
    short: f64,
    on_water: f64,
    on_held: f64,
    held_into: f64,
    held_down: f64,
    held_runoff: f64,
    runoff: f64,
    ground: f64,
    ground_dried: f64,
    bed: f64,
    into: f64,
    dried: f64,
    /// Cells held through the tick: between the level at its start and the mark.
    held_cells: usize,
    /// Cells of water at the tick's start.
    water_cells: usize,
    /// The puddle's water after the tick's rain and runoff, before the bed and drying.
    filled: f64,
    /// The water's surface over this hollow then.
    surface: f64,
    /// Room in the pores under the mark after the marks moved, and then before the bed.
    room: f64,
    room_at_bed: f64,
}

/// A hollows body with one class, its state and the test's knowledge of its cells.
struct World {
    tree: Tree,
    canopy: Canopy,
    class: GroundClass,
    stores: GroundStores,
    state: RainState,
    report: TickReport,
    /// Each hollow's cells, sorted by height.
    cells: Vec<Vec<usize>>,
    /// Each cell's exposure at the ground.
    exposure: Vec<f64>,
    area: f64,
    ticks: u64,
    name: String,
}

impl World {
    fn new(name: &str, bed: &Ground, shelters: &[Shelter], c: GroundClass) -> Self {
        let [nx, nz] = bed.size();
        let grid = [
            (nx as f64 * bed.spacing() / 0.1).ceil() as usize,
            (nz as f64 * bed.spacing() / 0.1).ceil() as usize,
        ];
        let canopy = if shelters.is_empty() {
            Canopy::open()
        } else {
            Canopy::prepare([0.0, 0.0], grid, shelters, &Profile::default())
                .expect("valid test case")
        };
        let tree = Tree::prepare(bed, &canopy, &Profile::default()).expect("valid test case");
        let heights = bed.heights();
        let mut cells = vec![Vec::new(); tree.hollow_count()];
        for (i, &l) in tree.labels().iter().enumerate() {
            if l != NO_HOLLOW {
                cells[l as usize].push(i);
            }
        }
        for list in &mut cells {
            list.sort_by(|&a, &b| heights[a].total_cmp(&heights[b]));
        }
        let exposure = (0..heights.len())
            .map(|i| {
                let [x, z] = bed.centre(i);
                canopy.exposure([x, heights[i], z])
            })
            .collect();
        Self {
            state: RainState::new(&tree),
            stores: GroundStores::new(&[c]).expect("valid test case"),
            report: TickReport::default(),
            area: bed.spacing() * bed.spacing(),
            tree,
            canopy,
            class: c,
            cells,
            exposure,
            ticks: 0,
            name: name.into(),
        }
    }

    fn hollows(&self) -> usize {
        self.cells.len()
    }

    fn z(&self, i: usize) -> f64 {
        self.tree.ground().heights()[i]
    }

    /// Cells of a hollow with ground at or above `lo` and below `hi`.
    fn band(&self, l: usize, lo: f64, hi: f64) -> impl Iterator<Item = usize> + '_ {
        self.cells[l]
            .iter()
            .copied()
            .filter(move |&i| self.z(i) >= lo && self.z(i) < hi)
    }

    /// The class's stores read at a cell's exposure: surface water in metres, and fill.
    fn reading(&self, i: usize) -> (f64, f64) {
        let w = weights(self.exposure[i]);
        (0..3).fold((0.0, 0.0), |(s, f), r| {
            let st = self.stores.store(0, r);
            (s + w[r] * st.surface, f + w[r] * st.fill)
        })
    }

    /// The test's own sum of what the ground holds: open ground cell by cell from the
    /// stores, and the pores under each mark.
    fn ground_water(&self) -> f64 {
        let marks = self.state.marks();
        let mut sum: f64 = self.state.pores_under().iter().sum();
        for (i, &l) in self.tree.labels().iter().enumerate() {
            if l == NO_HOLLOW || self.z(i) >= marks[l as usize] {
                let (s, f) = self.reading(i);
                sum += (s + f * self.class.pores) * self.area;
            }
        }
        sum
    }

    /// Whether a hollow holds ground now: its puddle's level stands below its mark and
    /// there is ground between.
    fn holds(&self, l: usize) -> usize {
        let level = self.state.levels(&self.tree)[l];
        self.band(l, level, self.state.marks()[l]).count()
    }

    /// One tick: the test's entries from the state before it, then the model's against
    /// them. Returns the test's entries for each hollow.
    #[expect(
        clippy::too_many_lines,
        reason = "one tick's entries in the design's order"
    )]
    fn tick(&mut self, rain: Rain, dt: f64) -> Vec<Entries> {
        let (c, a) = (self.class, self.area);
        let h = self.hollows();
        let at = format!("in {} at tick {}", self.name, self.ticks);
        let holder: Vec<u32> = (0..h)
            .map(|l| self.state.holder(&self.tree, l as u32))
            .collect();
        let mut volume = vec![0.0; self.tree.node_count()];
        for (k, v) in self.state.puddles() {
            volume[k as usize] = v;
        }
        let start = self.state.levels(&self.tree);
        let mut marks = self.state.marks().to_vec();
        let mut under = self.state.pores_under().to_vec();
        let mut e = vec![Entries::default(); h];
        // 1. The marks, in the hollows' order.
        for l in 0..h {
            let (level, mark) = (start[l], marks[l]);
            let k = holder[l] as usize;
            let cells: Vec<usize> = self.band(l, level.min(mark), level.max(mark)).collect();
            let s: f64 = cells.iter().map(|&i| self.reading(i).0).sum::<f64>() * a;
            let q: f64 = cells.iter().map(|&i| self.reading(i).1).sum::<f64>() * c.pores * a;
            if level > mark {
                // Receipts: the ground's film to the puddle, its pore water under the mark.
                volume[k] += s;
                under[l] += q;
                (e[l].joined, e[l].under) = (s, q);
                marks[l] = level;
            } else if level < mark && !cells.is_empty() {
                let below = self.band(l, f64::NEG_INFINITY, mark).count();
                let share = under[l] * cells.len() as f64 / below as f64;
                let topped = (q - share).max(0.0);
                if volume[k] < s + topped {
                    // The puddle cannot pay: the ground is held and nothing moves.
                    e[l].short = s + topped - volume[k];
                } else {
                    // No entry is more than its donor holds when it is made.
                    assert!(s + topped <= volume[k] && share <= under[l] * (1.0 + 1e-12));
                    volume[k] -= s + topped;
                    under[l] -= share;
                    e[l].left = s;
                    e[l].topped = topped;
                    e[l].soaked = (share - q).max(0.0);
                    marks[l] = level;
                }
            } else {
                marks[l] = level;
            }
        }
        // 2. In: each cell's rain to the first thing it hits. The open stores' flows this
        // tick are those of the stores stepped by themselves.
        let fall = rain.flux / 1000.0 * dt;
        let mut stepped = self.stores.clone();
        stepped.step(rain, dt).expect("valid test case");
        let flow: [[f64; 3]; 3] = std::array::from_fn(|r| {
            let (was, now) = (self.stores.totals(0, r), stepped.totals(0, r));
            [
                now.runoff - was.runoff,
                now.percolated - was.percolated,
                now.dried - was.dried,
            ]
        });
        let (soak, perc) = (c.soak / 1000.0 * dt, c.percolation / 1000.0 * dt);
        for l in 0..h {
            let covered = self.band(l, f64::NEG_INFINITY, marks[l]).count() as f64 * a;
            let room = if covered > 0.0 {
                (c.pores * covered - under[l]).max(0.0) / covered
            } else {
                0.0
            };
            e[l].room = room * covered;
            for &i in &self.cells[l] {
                let z = self.z(i);
                if z < start[l] {
                    // Water: stopped first by what is above the water's surface.
                    let [x, y] = self.tree.ground().centre(i);
                    e[l].on_water += self.canopy.exposure([x, start[l], y]) * fall * a;
                    e[l].water_cells += 1;
                } else if z < marks[l] {
                    // Held ground: its share of the pores under the mark, and no film.
                    let water = self.exposure[i] * fall;
                    let (take, into) = if room > 0.0 {
                        let take = water.min(soak).min(room + perc);
                        (take, take.min(room))
                    } else {
                        (water.min(perc), 0.0)
                    };
                    e[l].on_held += water * a;
                    e[l].held_into += into * a;
                    e[l].held_down += (take - into) * a;
                    e[l].held_runoff += (water - take) * a;
                    e[l].held_cells += 1;
                } else {
                    let w = weights(self.exposure[i]);
                    for r in 0..3 {
                        e[l].runoff += w[r] * flow[r][0] * a;
                        e[l].ground += w[r] * flow[r][1] * a;
                        e[l].ground_dried += w[r] * flow[r][2] * a;
                    }
                }
            }
            assert!(e[l].held_into <= e[l].room * (1.0 + 1e-12) + 1e-24, "{at}");
            volume[holder[l] as usize] += e[l].on_water + e[l].held_runoff + e[l].runoff;
            under[l] += e[l].held_into;
        }
        // 3. Out: drying and the bed's take from the water's surface after the rain.
        let dry = rain.drying / 1000.0 * dt;
        let mut want = vec![0.0; volume.len()];
        for l in 0..h {
            let k = holder[l] as usize;
            e[l].filled = volume[k];
            if volume[k] <= 0.0 {
                continue;
            }
            let mut heights: Vec<f64> = (0..h)
                .filter(|&m| holder[m] as usize == k)
                .flat_map(|m| self.cells[m].iter().map(|&i| self.z(i)))
                .collect();
            heights.sort_by(f64::total_cmp);
            let level = level_of(&heights, volume[k], a).min(self.tree.node(holder[l]).sill);
            let surface = self.band(l, f64::NEG_INFINITY, level).count() as f64 * a;
            let covered = self.band(l, f64::NEG_INFINITY, marks[l]).count() as f64 * a;
            let room = (c.pores * covered - under[l]).max(0.0);
            // With room, the soak rate; over full pores, and with none, the percolation.
            let take = if room > 0.0 {
                (soak * surface).min(room + perc * surface)
            } else {
                perc * surface
            };
            (e[l].surface, e[l].room_at_bed) = (surface, room);
            (e[l].bed, e[l].into, e[l].dried) = (take, take.min(room), dry * surface);
            want[k] += take + dry * surface;
        }
        for l in 0..h {
            let k = holder[l] as usize;
            if want[k] > volume[k] {
                // Less than the tick's drying and take: each its share, and empty.
                let share = volume[k] / want[k];
                e[l].bed *= share;
                e[l].into *= share;
                e[l].dried *= share;
            }
            under[l] += e[l].into;
        }
        for k in 0..volume.len() {
            assert!(want[k].min(volume[k]) <= volume[k]);
            volume[k] = if want[k] >= volume[k] {
                0.0
            } else {
                volume[k] - want[k]
            };
        }
        // The model's tick, and the stores' after it.
        let account = *self.state.account();
        let held_before = self.state.held() + self.ground_water();
        self.state
            .step_with(&self.tree, rain, &self.stores, 0, dt, 1, &mut self.report)
            .expect("valid test case");
        self.stores = stepped;
        self.ticks += 1;
        for l in 0..h {
            let (m, t) = (self.report.hollows[l], e[l]);
            let at = format!("of hollow {l} {at}");
            for (what, model, test) in [
                ("surface water joined", m.joined, t.joined),
                ("pore water under the mark", m.under, t.under),
                ("surface water left", m.left, t.left),
                ("pores topped up", m.topped, t.topped),
                ("soaked", m.soaked, t.soaked),
                ("short", m.short, t.short),
                ("rain on water", m.on_water, t.on_water),
                ("rain on held ground", m.on_held, t.on_held),
                ("held ground into pores", m.held_into, t.held_into),
                ("held ground down", m.held_down, t.held_down),
                ("held ground runoff", m.held_runoff, t.held_runoff),
                ("open runoff", m.runoff, t.runoff),
                ("open ground down", m.ground, t.ground),
                ("open ground dried", m.ground_dried, t.ground_dried),
                ("bed's take", m.bed, t.bed),
                ("bed into pores", m.into, t.into),
                ("puddle dried", m.dried, t.dried),
                (
                    "pores under the mark",
                    self.state.pores_under()[l],
                    under[l],
                ),
            ] {
                close(what, model, test, &at);
            }
            assert!(self.state.pores_under()[l] >= 0.0, "{at}");
            assert_eq!(self.state.marks()[l], marks[l], "mark {at}");
            assert!(
                t.on_held == 0.0
                    || (t.held_into + t.held_down + t.held_runoff - t.on_held).abs()
                        <= 1e-12 * t.on_held
            );
        }
        // The donors lost what the receivers gained: the puddles hold the test's water.
        let spilled = self.report.edge_spill;
        close(
            "all puddles",
            self.state.held(),
            volume.iter().sum::<f64>() - spilled,
            &at,
        );
        if self.report.merges == 0 && self.report.partings == 0 && spilled == 0.0 {
            let mut now = vec![0.0; volume.len()];
            for (k, v) in self.state.puddles() {
                now[k as usize] = v;
            }
            let moved =
                (0..volume.len()).any(|k| volume[k] > self.tree.capacity(k as u32) * (1.0 - 1e-12));
            if !moved {
                for k in 0..volume.len() {
                    close("a puddle", now[k], volume[k], &at);
                }
            }
        }
        // The ground is the test's own sum, and the account closes on rain alone.
        let own = self.ground_water();
        close(
            "ground",
            self.state.ground_held(&self.tree, &self.stores, 0),
            own,
            &at,
        );
        let acc = self.state.account();
        let now = self.state.held() + own;
        let residual = acc.rain + acc.initial - now - acc.sinks();
        assert!(
            residual.abs() <= 1e-9 * (acc.rain + acc.initial),
            "account {at}: {residual:e}"
        );
        // Rain is the tick's only source.
        let body = self.tree.ground().heights().len() as f64 * a;
        let sunk = acc.sinks() - account.sinks();
        assert!(
            now + sunk <= held_before + fall * body + 1e-9 * (acc.rain + acc.initial),
            "water made {at}"
        );
        e
    }
}

/// The third review's floor: 8 by 8 level cells of 5 cm inside a rim 4 cm high.
fn floor() -> Ground {
    ground([10, 10], 0.05, |x, y| {
        let edge = |v: f64| !(0.05..0.45).contains(&v);
        if edge(x) || edge(y) { 0.04 } else { 0.0 }
    })
}

/// A crown over the half of a bed below `x`, passing 0.4 of the rain, 2 m up.
fn crown(x: f64, depth: f64) -> Shelter {
    Shelter {
        name: "crown".into(),
        triangles: vec![
            [[-1.0, 2.0, -1.0], [x, 2.0, -1.0], [x, 2.0, depth + 1.0]],
            [
                [-1.0, 2.0, -1.0],
                [x, 2.0, depth + 1.0],
                [-1.0, 2.0, depth + 1.0],
            ],
        ],
        share: 0.4,
    }
}

/// Part 11d's dish: 16 by 16 cells of 5 cm with a bed of 0.02 r² m, inside a ring of
/// cells at 4 mm that it spills over, half under the crown.
fn dish() -> Ground {
    ground([18, 18], 0.05, |x, y| {
        let inside = |v: f64| (0.05..0.85).contains(&v);
        if inside(x) && inside(y) {
            0.0201 * ((x - 0.45).powi(2) + (y - 0.45).powi(2))
        } else {
            0.004
        }
    })
}

/// The dish after 600 s at 100 mm/h and then no rain and no drying, at tick `at`: its
/// puddle has shrunk and cannot pay for the ground it has left, whose class still stands
/// a film, so the hollow holds that ground, over full pores.
fn held_dish(at: u32) -> World {
    let mut w = World::new(
        "the dish",
        &dish(),
        &[crown(0.25, 0.9)],
        class(15.0, 5.0, 3.0, 3.0),
    );
    for t in 0..at {
        w.tick(rain(if t < 600 { 100.0 } else { 0.0 }, 0.0), 1.0);
    }
    w
}

/// The cells of a world's first hollow that are water, held ground and open ground now,
/// found from the bed, the level and, between held and open ground, the mark.
fn receivers(w: &World) -> [Vec<usize>; 3] {
    let level = w.state.levels(&w.tree)[0];
    let mark = w.state.marks()[0];
    [
        w.band(0, f64::NEG_INFINITY, level).collect(),
        w.band(0, level, mark).collect(),
        w.band(0, level.max(mark), f64::INFINITY).collect(),
    ]
}

/// M2.11. Part 11d's dish holds ground over full pores round a few cells of water when
/// rain starts again. At 1 mm/h, under the class's 5 mm/h percolation rate, the puddle
/// gains from the rain only what falls on its water and all of the held ground's rain
/// goes down. A model that gave the puddle the rain on held ground would add many times
/// that, though its total closes too.
#[test]
fn rain_on_held_ground_under_the_percolation_rate_all_goes_down() {
    let mut w = held_dish(1200);
    let a = w.area;
    let [water, held, _] = receivers(&w);
    assert!(
        !water.is_empty() && held.len() >= 40,
        "the dish holds ground"
    );
    let full = w.class.pores * (water.len() + held.len()) as f64 * a;
    close(
        "full pores",
        w.state.pores_under()[0],
        full,
        "under the mark",
    );
    let fall = mm_h(1.0) / 1000.0;
    // The test's own sums, from the bed, the shelter and the level.
    let on_water: f64 = water
        .iter()
        .map(|&i| {
            let [x, y] = w.tree.ground().centre(i);
            w.canopy.exposure([x, w.state.levels(&w.tree)[0], y]) * fall * a
        })
        .sum();
    let on_held: f64 = held.iter().map(|&i| w.exposure[i] * fall * a).sum();
    let before = w.state.held();
    let e = w.tick(rain(1.0, 0.0), 1.0)[0];
    close("rain on water", e.on_water, on_water, "the dish at 1 mm/h");
    close(
        "rain on held ground",
        e.on_held,
        on_held,
        "the dish at 1 mm/h",
    );
    assert_eq!(e.held_cells, held.len());
    // All of it goes down: none to the pores, which are full, and none to the puddle.
    assert_eq!((e.held_into, e.held_runoff), (0.0, 0.0));
    close(
        "held ground down",
        e.held_down,
        on_held,
        "the dish at 1 mm/h",
    );
    // The open ground stands less film than its relief holds, so it sends no runoff, and
    // the puddle's gain is the rain on its water less what its bed took.
    assert_eq!(e.runoff, 0.0);
    close(
        "the puddle's change",
        w.state.held() - before + e.bed,
        on_water,
        "the dish at 1 mm/h",
    );
    assert!(on_held > 5.0 * on_water, "{on_held:e} against {on_water:e}");
    println!(
        "1 mm/h: {} cells of water take {on_water:.6e} m3, {} held cells take {on_held:.6e}, all down",
        water.len(),
        held.len()
    );
}

/// M2.11. The same dish at 100 mm/h: the held ground passes the percolation rate down
/// over its area and the rest of its rain runs to the puddle.
#[test]
fn rain_on_held_ground_over_the_percolation_rate_runs_to_the_puddle() {
    let mut w = held_dish(1200);
    let a = w.area;
    let [_, held, _] = receivers(&w);
    let fall = mm_h(100.0) / 1000.0;
    let on_held: f64 = held.iter().map(|&i| w.exposure[i] * fall * a).sum();
    let down = mm_h(5.0) / 1000.0 * held.len() as f64 * a;
    let before = w.state.held();
    let e = w.tick(rain(100.0, 0.0), 1.0)[0];
    close(
        "rain on held ground",
        e.on_held,
        on_held,
        "the dish at 100 mm/h",
    );
    close(
        "held ground down",
        e.held_down,
        down,
        "the dish at 100 mm/h",
    );
    close(
        "held runoff",
        e.held_runoff,
        on_held - down,
        "the dish at 100 mm/h",
    );
    assert_eq!(e.held_into, 0.0);
    close(
        "the puddle's change",
        w.state.held() - before,
        e.on_water + e.held_runoff + e.runoff - e.bed,
        "the dish at 100 mm/h",
    );
    println!(
        "100 mm/h: held ground takes {on_held:.6e} m3, {down:.6e} down, {:.6e} to the puddle",
        on_held - down
    );
}

/// M2.11. A level floor half under a crown, on ground that soaks 15 mm/h and passes none
/// down, under 10 mm/h: its puddle empties into the pores in the rain and cannot pay for
/// the floor, which is held with room in its pores. The pores gain all the held ground's
/// rain, by the exposure at the ground, and the puddle none of it.
#[test]
fn held_ground_with_room_in_its_pores_takes_its_rain() {
    let mut w = World::new(
        "the floor under a crown",
        &floor(),
        &[crown(0.25, 0.5)],
        class(15.0, 0.0, 3.0, 0.1),
    );
    let a = w.area;
    let mut ticks = 0;
    for _ in 0..3000 {
        let [water, held, _] = receivers(&w);
        let under = w.state.pores_under()[0];
        let fall = mm_h(10.0) / 1000.0;
        let on_held: f64 = held.iter().map(|&i| w.exposure[i] * fall * a).sum();
        let e = w.tick(rain(10.0, 0.0), 1.0)[0];
        if !held.is_empty() && water.is_empty() && e.short > 0.0 && e.room > 64.0 * fall * a {
            // Held through the tick, with room under every cell for all its rain.
            assert!(on_held > 0.0 && on_held < 64.0 * fall * a, "part shelter");
            close("rain on held ground", e.on_held, on_held, "on the floor");
            close("into its pores", e.held_into, on_held, "on the floor");
            assert_eq!((e.held_down, e.held_runoff, e.on_water), (0.0, 0.0, 0.0));
            close(
                "the pores' gain",
                w.state.pores_under()[0] - under,
                on_held + e.into,
                "on the floor",
            );
            ticks += 1;
        }
    }
    println!("held with room in its pores, in the rain, for {ticks} ticks");
    assert!(ticks > 100);
}

/// M2.12. The third review's twelve cases, each with three hours of drying. Every tick's
/// entries are the test's own, no entry is more than its donor held, and the account
/// closes on rain alone; each case ends dry, with nothing held and all the rain in the
/// sinks.
#[test]
fn the_reviews_floors_move_their_marks_with_no_water_made_or_owed() {
    let (mut joined, mut left, mut soaked, mut topped, mut puddles) = (0, 0, 0, 0, 0);
    for soak in [1.0, 30.0] {
        for relief in [0.1, 3.0] {
            for shower in [50, 150, 500] {
                let name = format!("the floor, soak {soak} mm/h, relief {relief} mm, {shower} s");
                let mut w = World::new(&name, &floor(), &[], class(soak, 0.0, 3.0, relief));
                let (mut deepest, mut into, mut held) = (0.0_f64, 0.0, 0);
                for t in 0..shower + 3 * 3600 {
                    let wet = t < shower;
                    let e = w.tick(
                        rain(if wet { 60.0 } else { 0.0 }, if wet { 0.0 } else { 20.0 }),
                        1.0,
                    )[0];
                    deepest = deepest.max(w.state.held());
                    into += e.into;
                    held += usize::from(e.short > 0.0);
                    joined += usize::from(e.joined > 0.0);
                    left += usize::from(e.left > 0.0);
                    soaked += usize::from(e.soaked > 0.0);
                    topped += usize::from(e.topped > 0.0);
                }
                let acc = *w.state.account();
                println!(
                    "{name}: rain {:.3e} m3, deepest puddle {deepest:.3e}, into the pores under it {into:.3e}, soaked {:.3e}, held ticks {held}",
                    acc.rain, acc.soaked
                );
                puddles += usize::from(deepest > 0.0);
                assert_eq!(w.state.held(), 0.0, "{name}");
                assert_eq!(w.holds(0), 0, "{name}");
                assert!(
                    w.ground_water() < 1e-6 * acc.rain,
                    "{name}: the ground dries"
                );
                assert!((acc.sinks() - acc.rain).abs() < 1e-6 * acc.rain, "{name}");
                if soak == 1.0 && relief == 0.1 && shower == 50 {
                    // The floor's own cells drain to it: 64 cells of the shower, less the
                    // film that stood before the puddle and the soak rate under it.
                    let fell = 64.0 * w.area * mm_h(60.0) / 1000.0 * 50.0;
                    assert!(
                        deepest > 0.9 * fell,
                        "the puddle keeps its water: {deepest:e}"
                    );
                }
            }
        }
    }
    println!(
        "puddles in {puddles} cases; ticks with film joined {joined}, left {left}, pores topped up {topped}, soaked {soaked}"
    );
    assert!(puddles >= 8 && joined > 0 && soaked > 0);
}

/// The wetness at a cell's centre, on its ground.
fn wetness_at(w: &World, i: usize) -> thrum_rain_model::Wetness {
    let [x, y] = w.tree.ground().centre(i);
    w.state
        .wetness(
            &w.tree,
            &w.stores,
            0,
            &w.canopy,
            [x, w.z(i), y],
            [0.0, 1.0, 0.0],
            None,
        )
        .expect("on the body")
}

/// Check what the wetness function gives held ground, water and open ground now.
fn check_wetness(w: &World) {
    let [water, held, open] = receivers(w);
    let level = w.state.levels(&w.tree)[0];
    let covered = (water.len() + held.len()) as f64 * w.area;
    let fill = w.state.pores_under()[0] / (w.class.pores * covered);
    for &i in &held {
        let got = wetness_at(w, i);
        let band = band_fill(w.z(i) - level, w.state.bands()[0]);
        assert_eq!(
            (got.surface, got.cover),
            (0.0, 0.0),
            "held ground stands no film"
        );
        assert!(
            (got.pores - fill.max(band)).abs() < 1e-12,
            "{} {fill}",
            got.pores
        );
    }
    for &i in &water {
        let got = wetness_at(w, i);
        assert_eq!((got.pores, got.surface), (1.0, 0.0));
    }
    for &i in &open {
        let got = wetness_at(w, i);
        let (s, f) = w.reading(i);
        let band = band_fill(w.z(i) - level, w.state.bands()[0]);
        assert!((got.surface - s.min(w.class.relief)).abs() < 1e-15);
        assert!((got.pores - f.max(band)).abs() < 1e-12);
    }
}

/// M2.12. The dish holds its ground for more than one tick, with rain falling on it
/// while it is held, and hands all of it over on the first tick the puddle can pay: here
/// when the film on the open ground has gone down, so that nothing is due. While it is
/// held nothing moves, and the wetness function gives that ground the fill of the pores
/// under the mark and no surface water, beside open ground that stands a film.
#[test]
fn held_ground_is_kept_through_rain_and_handed_over_when_the_puddle_can_pay() {
    let mut w = held_dish(900);
    let (mut held_ticks, mut rained_on, mut handed) = (0, 0, None);
    for t in 900..3000 {
        let mark = w.state.marks()[0];
        let under = w.state.pores_under()[0];
        let [_, held, open] = receivers(&w);
        // Light rain falls on the held ground for two minutes.
        let flux = if (1000..1120).contains(&t) { 1.0 } else { 0.0 };
        let e = w.tick(rain(flux, 0.0), 1.0)[0];
        if e.short > 0.0 {
            // Held: nothing moved, and the mark stayed.
            assert_eq!(w.state.marks()[0], mark);
            assert_eq!((e.left, e.topped, e.soaked, e.joined), (0.0, 0.0, 0.0, 0.0));
            assert_eq!(e.held_cells, held.len());
            close(
                "pores under the mark",
                w.state.pores_under()[0],
                under + e.into,
                "held",
            );
            held_ticks += 1;
            rained_on += usize::from(e.on_held > 0.0);
            if t % 50 == 0 {
                check_wetness(&w);
                assert!(wetness_at(&w, open[0]).surface > 0.0 || w.reading(open[0]).0 == 0.0);
            }
        } else if handed.is_none() && !held.is_empty() {
            // The first tick the puddle could pay: all the held ground at once.
            assert_eq!(w.holds(0), 0);
            assert_eq!(
                w.state.marks()[0],
                w.state.levels(&w.tree)[0].max(w.state.marks()[0])
            );
            handed = Some((t, held.len(), e.left, e.topped, e.soaked));
        }
    }
    let (t, cells, left, topped, soaked) = handed.expect("the ground is handed over");
    println!(
        "held for {held_ticks} ticks from 900, {rained_on} of them in rain; {cells} cells handed over at tick {t}: left {left:e}, topped up {topped:e}, soaked {soaked:e}"
    );
    assert!(held_ticks > 1000 && rained_on >= 100);
    // Nothing was due by then: the class's film had gone and its pores were as full.
    assert!(left == 0.0 && topped == 0.0 && w.stores.store(0, 0).surface == 0.0);
}

/// M2.12. Held ground is also handed over when rain refills the puddle: heavy rain on
/// the held dish brings its level back over the mark within the shower, and from then the
/// hollow holds no ground.
#[test]
fn held_ground_is_freed_when_rain_refills_the_puddle() {
    let mut w = held_dish(900);
    assert!(w.holds(0) > 0);
    let mut freed = None;
    for t in 0..600 {
        w.tick(rain(100.0, 0.0), 1.0);
        if freed.is_none() && w.holds(0) == 0 {
            freed = Some(t);
        }
        if freed.is_some() {
            assert_eq!(w.holds(0), 0);
        }
    }
    println!(
        "freed {} ticks into the rain",
        freed.expect("the puddle refills")
    );
}

/// M2.12. Two hollows that share a puddle through a merge and a parting, on ground with
/// pores and 8 mm of relief that passes 30 mm/h down. Heavy rain joins them; when it stops
/// the shared level falls over ground that still stands a film, until they part, and
/// drying empties both. Each hollow's entries are the test's own every tick; while they
/// share a puddle each takes film from its ground and leaves film on it, drawing on the
/// one puddle in their order.
#[test]
fn hollows_that_share_a_puddle_draw_on_it_through_a_merge_and_a_parting() {
    let mut w = World::new("the pair", &pair(), &[], class(60.0, 30.0, 3.0, 8.0));
    assert_eq!(w.hollows(), 2);
    let (mut merges, mut partings) = (0, 0);
    let (mut joined, mut left) = ([0, 0], [0, 0]);
    for t in 0..12_000 {
        let shared = w.state.holder(&w.tree, 0) == w.state.holder(&w.tree, 1);
        let e = w.tick(
            match t {
                0..600 => rain(200.0, 0.0),
                600..4000 => rain(0.0, 0.0),
                _ => rain(0.0, 30.0),
            },
            1.0,
        );
        merges += w.report.merges;
        partings += w.report.partings;
        if shared {
            for l in 0..2 {
                joined[l] += usize::from(e[l].joined > 0.0);
                left[l] += usize::from(e[l].left > 0.0);
            }
        }
    }
    println!(
        "merges {merges}, partings {partings}; while shared, each hollow's ticks with film joined {joined:?} and left {left:?}"
    );
    assert!(merges >= 1 && partings >= 1);
    assert!(joined.iter().chain(&left).all(|&n| n > 0));
    assert_eq!(w.state.held(), 0.0);
    assert!(w.state.pores_under().iter().all(|&u| u < 1e-12));
}

/// M2.13. The bed under a poured puddle on the level floor, with no rain. While the pores
/// under the mark have room the bed takes the soak rate over the water's surface and the
/// pores keep it; over full pores it takes the percolation rate, also where that is above
/// the soak rate; on ground with no pores it takes the percolation rate from the first
/// tick. Then drying is turned on, and on its last tick the puddle gives the bed and the
/// drying each its share of what it holds and is empty.
#[test]
fn the_bed_takes_the_soak_rate_with_room_and_the_percolation_rate_without() {
    for (soak, perc, pores) in [(15.0, 5.0, 3.0), (1.0, 15.0, 3.0), (15.0, 5.0, 0.0)] {
        let name =
            format!("the poured floor, soak {soak}, percolation {perc} mm/h, pores {pores} mm");
        let mut w = World::new(&name, &floor(), &[], class(soak, perc, pores, 0.1));
        let surface = 64.0 * w.area;
        w.state
            .pour(&w.tree, 0, 0.02 * surface)
            .expect("valid test case");
        let room = pores / 1000.0 * surface;
        let (at_soak, at_perc) = (mm_h(soak) / 1000.0 * surface, mm_h(perc) / 1000.0 * surface);
        // The pores fill in their room over the soak rate, in seconds.
        let fills = if pores > 0.0 {
            (room / at_soak).round() as usize
        } else {
            0
        };
        let (mut filling, mut full) = (0, 0);
        let mut last = None;
        for t in 0..40_000 {
            let before = w.state.held();
            let under = w.state.pores_under()[0];
            let drying = if t > fills + 600 { 20.0 } else { 0.0 };
            let e = w.tick(rain(0.0, drying), 1.0)[0];
            let dried = mm_h(drying) / 1000.0 * surface;
            if t == 0 {
                // The mark rose over dry ground: nothing to join.
                assert_eq!((e.joined, e.under), (0.0, 0.0));
            }
            if w.state.held() > 0.0 {
                if t + 1 < fills {
                    close("bed with room", e.bed, at_soak, &name);
                    close("into the pores", e.into, at_soak, &name);
                    filling += 1;
                } else if t > fills {
                    close("bed over full pores", e.bed, at_perc, &name);
                    assert_eq!(e.into, 0.0, "{name}");
                    full += 1;
                }
                if t == fills + 1 && pores > 0.0 {
                    close("full pores", w.state.pores_under()[0], room, &name);
                }
                close("the puddle", before - w.state.held(), e.bed + dried, &name);
                close("the pores", w.state.pores_under()[0] - under, e.into, &name);
            } else if last.is_none() {
                // Less than the tick's drying and take: each its share, and empty.
                assert!(before > 0.0 && before < at_perc + dried, "{name}");
                close("the last of the puddle", e.bed + e.dried, before, &name);
                close(
                    "the bed's share",
                    e.bed,
                    before * at_perc / (at_perc + dried),
                    &name,
                );
                close(
                    "the drying's share",
                    e.dried,
                    before * dried / (at_perc + dried),
                    &name,
                );
                last = Some(t);
            }
        }
        println!(
            "{name}: {filling} ticks at the soak rate ({at_soak:.4e} m3), {full} at the percolation rate ({at_perc:.4e}); empty at tick {}",
            last.expect("the puddle empties")
        );
        assert_eq!(filling, fills.saturating_sub(1), "{name}");
        assert!(full > 500, "{name}");
    }
}

/// M2.13. From dry, the third review's floor under 60 mm/h: 3 mm of pores that soak
/// 1 mm/h and 0.1 mm of relief. Once the puddle stands over the floor it gains each tick
/// the rain on it and the runoff to it, less 1 mm/h over its surface. A model that filled
/// the pores under it at once would leave it nothing.
#[test]
fn a_puddle_on_dry_deep_pores_keeps_its_rain_less_the_soak_rate() {
    let mut w = World::new("the dry floor", &floor(), &[], class(1.0, 0.0, 3.0, 0.1));
    let surface = 64.0 * w.area;
    let fall = mm_h(60.0) / 1000.0 * surface;
    let soak = mm_h(1.0) / 1000.0 * surface;
    let mut standing = 0;
    for _ in 0..500 {
        let before = w.state.held();
        let [water, ..] = receivers(&w);
        let e = w.tick(rain(60.0, 0.0), 1.0)[0];
        if water.len() == 64 && e.joined == 0.0 {
            close("rain on the puddle", e.on_water, fall, "on the dry floor");
            // Every cell that drains to the floor is under it: no runoff reaches it.
            assert_eq!(e.runoff + e.held_runoff, 0.0);
            close("the bed's take", e.bed, soak, "on the dry floor");
            close(
                "the puddle's gain",
                w.state.held() - before,
                fall - soak,
                "on the dry floor",
            );
            standing += 1;
        } else {
            // Before it stands: the open floor's runoff, and the film that joins it.
            close(
                "the puddle's gain",
                w.state.held() - before,
                e.runoff + e.joined + e.on_water - e.bed,
                "on the dry floor",
            );
        }
    }
    let room = 0.003 * surface;
    println!(
        "the puddle stood for {standing} of 500 ticks and holds {:.4e} m3; the pores under it hold {:.4e} of {room:.4e}",
        w.state.held(),
        w.state.pores_under()[0]
    );
    assert!(standing > 480);
    assert!(w.state.held() > 0.95 * 490.0 * (fall - soak));
    assert!(w.state.pores_under()[0] < 0.1 * room);
}
