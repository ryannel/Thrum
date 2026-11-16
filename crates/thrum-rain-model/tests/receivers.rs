//! M1.13: each tick's rain on a cell goes to one receiver, and each receiver's gain is
//! right by itself. The test sums each receiver over cells from the bed, the levels and the
//! shelters alone: which cells drain to the hollow is known from how the bed is made, and
//! exposure from the shelters' rectangles, without the model. A low shelter stands in the
//! bowl, so the water rises from under it to over it: rain on water is stopped first by what
//! is above the water's surface, and rain on bare ground by what is above the ground.
pub mod support;
use support::{ground, mm_h};
use thrum_rain_model::{Canopy, GroundClass, Profile, Rain, RainState, Shelter, TickReport, Tree};

const N: usize = 60;
const CELL: f64 = 0.05;

/// Cells 10 to 49 in each axis drain to one bowl inside a ridge two cells wide; the rest
/// drain over the edge.
fn inner(i: usize, j: usize) -> bool {
    (10..50).contains(&i) && (10..50).contains(&j)
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "cell indices of points inside the grid"
)]
fn bed() -> thrum_rain_model::Ground {
    ground([N, N], CELL, |x, y| {
        let (i, j) = ((x / CELL) as usize, (y / CELL) as usize);
        let ring = |a: usize, b: usize, lo: usize, hi: usize| {
            (a == lo || a == hi || b == lo || b == hi)
                && (lo..=hi).contains(&a)
                && (lo..=hi).contains(&b)
        };
        if ring(i, j, 10, 49) {
            0.05
        } else if ring(i, j, 9, 50) {
            0.06
        } else if inner(i, j) {
            0.02 * ((x - 1.5).powi(2) + (y - 1.5).powi(2)) - 0.03
        } else {
            let to_edge = i.min(j).min(N - 1 - i).min(N - 1 - j);
            0.001 * to_edge as f64
        }
    })
}

/// A rectangle shelter and its share: x0, z0, x1, z1, height, share.
const ROOF: [f64; 6] = [1.5, 1.2, 3.5, 1.8, 2.5, 0.0];
const CROWN: [f64; 6] = [0.6, 0.6, 1.2, 1.4, 3.0, 0.4];
/// Low enough that the puddle rises past it: the bed under it lies between -0.026 and
/// -0.010 m; its edges lie on canopy column edges, as the others do.
const SLAB: [f64; 6] = [0.9, 1.9, 1.3, 2.3, -0.005, 0.2];
const SHELTERS: [[f64; 6]; 3] = [ROOF, CROWN, SLAB];

fn shelter(name: &str, r: [f64; 6]) -> Shelter {
    let [x0, z0, x1, z1, y, share] = r;
    Shelter {
        name: name.into(),
        triangles: vec![
            [[x0, y, z0], [x1, y, z0], [x1, y, z1]],
            [[x0, y, z0], [x1, y, z1], [x0, y, z1]],
        ],
        share,
    }
}

/// What the shelters above a height withhold from rain falling to a point at it: each
/// one's loss of share times the part of the square of the edge width round the point that
/// its rectangle covers. The rectangles do not overlap, so these add.
fn withheld(x: f64, z: f64, height: f64) -> f64 {
    let half = 0.5 * Profile::default().edge_width;
    SHELTERS
        .iter()
        .filter(|r| r[4] > height)
        .map(|r| {
            let ox = ((x + half).min(r[2]) - (x - half).max(r[0])).max(0.0);
            let oz = ((z + half).min(r[3]) - (z - half).max(r[1])).max(0.0);
            (1.0 - r[5]) * ox * oz / (4.0 * half * half)
        })
        .sum()
}

#[derive(Debug, Default)]
struct Sums {
    canopy: f64,
    on_water: f64,
    runoff: f64,
    ground: f64,
    edge: f64,
}

/// The test's own sums over cells for one tick, from the levels at the tick's start.
fn sums(g: &thrum_rain_model::Ground, level: f64, fall: f64, soak: f64) -> Sums {
    let a = CELL * CELL;
    let mut s = Sums::default();
    for j in 0..N {
        for i in 0..N {
            let (x, z) = ((i as f64 + 0.5) * CELL, (j as f64 + 0.5) * CELL);
            let bed = g.heights()[j * N + i];
            let wet = inner(i, j) && bed < level;
            // The first thing the rain meets: the water's surface over a wet cell.
            let lost = withheld(x, z, if wet { level } else { bed });
            s.canopy += fall * a * lost;
            let rain = (1.0 - lost) * fall;
            if wet {
                s.on_water += rain * a;
            } else if inner(i, j) {
                s.runoff += (rain - soak).max(0.0) * a;
                s.ground += rain.min(soak) * a;
            } else {
                s.edge += (rain - soak).max(0.0) * a;
                s.ground += rain.min(soak) * a;
            }
        }
    }
    s
}

fn close(what: &str, model: f64, test: f64, rain: f64) {
    assert!(
        (model - test).abs() <= 1e-9 * rain,
        "{what}: model {model} against {test}"
    );
}

#[test]
fn each_receiver_gains_its_own_cells_rain() {
    let canopy = Canopy::prepare(
        [-1.0, -1.0],
        [60, 60],
        &[
            shelter("roof", ROOF),
            shelter("crown", CROWN),
            shelter("slab", SLAB),
        ],
        &Profile::default(),
    )
    .expect("valid test case");
    let tree = Tree::prepare(&bed(), &canopy, &Profile::default()).expect("valid test case");
    assert_eq!(tree.hollow_count(), 1);
    let g = tree.ground();
    let rain = Rain {
        flux: mm_h(30.0),
        drying: 0.0,
    };
    for percolation in [0.0, mm_h(5.0)] {
        let class = GroundClass::bare(percolation);
        let mut state = RainState::new(&tree);
        let mut report = TickReport::default();
        let mut checked = 0;
        let (mut under_slab, mut over_slab) = (false, false);
        for t in 0..4000 {
            let level = state.levels(&tree)[0];
            let wet = state.held() > 0.0;
            state
                .step(&tree, rain, class, 1.0, 1, &mut report)
                .expect("valid test case");
            let fall = rain.flux / 1000.0;
            let total = fall * (N * N) as f64 * CELL * CELL;
            let s = sums(g, level, fall, percolation / 1000.0);
            let h = report.hollows[0];
            // Every tick, so the water's surface is checked as it passes the slab.
            close(
                "canopy",
                h.canopy + report.edge_cells.canopy,
                s.canopy,
                total,
            );
            close("puddle", h.on_water, s.on_water, total);
            close("runoff", h.runoff, s.runoff, total);
            close(
                "ground",
                h.ground + report.edge_cells.ground,
                s.ground,
                total,
            );
            close("edge", report.edge_cells.runoff, s.edge, total);
            if wet {
                under_slab |= level < SLAB[4] && covered_under_slab(g, level) > 0;
                over_slab |= level > SLAB[4];
            }
            if ![300, 1500, 3999].contains(&t) {
                continue;
            }
            assert!(wet, "a puddle stands at the check");
            // The puddle covers part of its hollow, under the roof's edge.
            let covered = (0..g.heights().len())
                .filter(|&i| inner(i % N, i / N) && g.heights()[i] < level)
                .count();
            assert!(
                covered > 50 && covered < 1600,
                "{covered} cells under water"
            );
            assert!(s.canopy > 0.0 && s.on_water > 0.0 && s.runoff > 0.0 && s.edge > 0.0);
            // The roof's edge shelters part of the water: less than open rain reaches it.
            assert!(s.on_water < fall * covered as f64 * CELL * CELL);
            checked += 1;
        }
        assert!(
            under_slab && over_slab,
            "the water's surface passes the slab: under {under_slab}, over {over_slab}"
        );
        assert_eq!(checked, 3);
    }
}

/// Wet cells under the slab's rectangle at a level.
fn covered_under_slab(g: &thrum_rain_model::Ground, level: f64) -> usize {
    (0..g.heights().len())
        .filter(|&i| {
            let (x, z) = (
                (i % N) as f64 * CELL + 0.5 * CELL,
                (i / N) as f64 * CELL + 0.5 * CELL,
            );
            (SLAB[0]..SLAB[2]).contains(&x)
                && (SLAB[1]..SLAB[3]).contains(&z)
                && g.heights()[i] < level
        })
        .count()
}

/// One opaque shelter 1 cm up over a whole bed: a flat floor of 8 by 8 cells inside a rim
/// 4 cm high. Water 5 mm deep is under the shelter, so the canopy takes the rain over the
/// floor; water 2 cm deep stands over it, so the water takes that rain and the canopy none.
/// The rim is above the shelter either way, so it is open.
#[test]
fn water_over_a_low_shelter_takes_the_rain() {
    let n = 10;
    let cell = 0.05;
    let heights = (0..n * n)
        .map(|i| {
            let (x, y) = (i % n, i / n);
            if x == 0 || y == 0 || x == n - 1 || y == n - 1 {
                0.04
            } else {
                0.0
            }
        })
        .collect();
    let bed =
        thrum_rain_model::Ground::new([0.0, 0.0], cell, [n, n], heights).expect("valid test case");
    let low = shelter("low", [-0.5, -0.5, 1.0, 1.0, 0.01, 0.0]);
    let profile = Profile::default();
    let canopy =
        Canopy::prepare([-0.5, -0.5], [15, 15], &[low], &profile).expect("valid test case");
    let tree = Tree::prepare(&bed, &canopy, &profile).expect("valid test case");
    assert_eq!(tree.hollow_count(), 1);
    let rain = Rain {
        flux: mm_h(100.0),
        drying: 0.0,
    };
    let class = GroundClass::bare(0.0);
    let fall = rain.flux / 1000.0;
    let floor = 64.0 * cell * cell * fall;
    for (depth, over) in [(0.005, false), (0.02, true)] {
        let mut state = RainState::new(&tree);
        state
            .pour(&tree, 0, tree.volume(0, depth))
            .expect("valid test case");
        let level = state.levels(&tree)[0];
        assert!((level - depth).abs() < 1e-9, "level {level}");
        let mut report = TickReport::default();
        state
            .step(&tree, rain, class, 1.0, 1, &mut report)
            .expect("valid test case");
        let h = report.hollows[0];
        let (water, canopy) = if over { (floor, 0.0) } else { (0.0, floor) };
        close("puddle", h.on_water, water, floor);
        close("canopy", h.canopy + report.edge_cells.canopy, canopy, floor);
        close(
            "runoff",
            h.runoff + report.edge_cells.runoff,
            36.0 * cell * cell * fall,
            floor,
        );
    }
}

/// A class of `soak`, `percolation` (mm/h), `pores` and `relief` (mm).
fn stored(soak: f64, percolation: f64, pores: f64, relief: f64) -> GroundClass {
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

/// M2.11, first case. A box whose floor is in two halves, 2 cm apart, inside a rim 5 cm
/// high, under open sky. Its ground takes all the rain it is given: the pores hold more
/// than the run brings and soak faster than it falls. A puddle is poured on the low half,
/// with water enough to fill the pores it covers; it stands on the low half only. It rises by the rain on its own surface and nothing else; the uncovered
/// ground's stores are those of open ground stepped by the test alone. A model that
/// passed all rain through the ground's stores would leave the puddle still.
#[test]
fn rain_on_a_puddle_is_the_puddles_and_open_ground_keeps_its_own() {
    let (n, cell) = (12usize, 0.05);
    let heights = (0..n * n)
        .map(|i| {
            let (x, y) = (i % n, i / n);
            if x == 0 || y == 0 || x == n - 1 || y == n - 1 {
                0.05
            } else if x < n / 2 {
                0.0
            } else {
                0.02
            }
        })
        .collect();
    let bed =
        thrum_rain_model::Ground::new([0.0, 0.0], cell, [n, n], heights).expect("valid test case");
    let tree = Tree::prepare(&bed, &Canopy::open(), &Profile::default()).expect("valid test case");
    assert_eq!(tree.hollow_count(), 1);
    let class = stored(60.0, 0.0, 5.0, 1.0);
    let mut ground = thrum_rain_model::GroundStores::new(&[class]).expect("valid test case");
    let mut state = RainState::new(&tree);
    let low: u32 = 5 * 10; // the low half's cells
    state
        .pour(
            &tree,
            0,
            (0.008 + class.pores) * f64::from(low) * cell * cell,
        )
        .expect("valid test case");
    let rain = Rain {
        flux: mm_h(30.0),
        drying: 0.0,
    };
    let mut report = TickReport::default();
    // The test's own open store: water into the pores at the rain, under the soak rate.
    let (mut fill, mut surface) = (0.0_f64, 0.0_f64);
    let fall = rain.flux / 1000.0;
    let covered = |level: f64| {
        tree.ground()
            .heights()
            .iter()
            .filter(|&&z| z < level)
            .count()
    };
    let mut rise = 0.0;
    let ticks = 400;
    for t in 0..ticks {
        let before = state.held();
        let level = state.levels(&tree)[0];
        state
            .step_with(&tree, rain, &ground, 0, 1.0, 1, &mut report)
            .expect("valid test case");
        ground.step(rain, 1.0).expect("valid test case");
        let take = fall
            .min(class.soak / 1000.0)
            .min((1.0 - fill) * class.pores);
        fill += take / class.pores;
        surface += fall - take;
        let s = ground.store(0, 0);
        assert!((s.fill - fill).abs() < 1e-12 && (s.surface - surface).abs() < 1e-15);
        let own = fall * covered(level) as f64 * cell * cell;
        assert_eq!(
            covered(level),
            low as usize,
            "the puddle covers the low half at {t} s"
        );
        let h = report.hollows[0];
        close("puddle", h.on_water, own, own);
        close("runoff", h.runoff, 0.0, own);
        if t == 0 {
            // The pour's water fills the pores it covers, once.
            let room = class.pores * f64::from(low) * cell * cell;
            close("filled", h.filled, room, room);
            close("rise", state.held() - before, own - room, room);
            continue;
        }
        close("rise", state.held() - before, own, own);
        assert_eq!(h.joined + h.filled + h.left + h.soaked, 0.0);
        rise += state.held() - before;
    }
    // What a model that passed all rain through the ground's stores would give: the
    // ground takes it all, nothing runs off, and the puddle does not rise.
    let all_through_ground = 0.0;
    let expected = fall * f64::from(ticks - 1) * f64::from(low) * cell * cell;
    assert!((rise - expected).abs() < 1e-12 && (rise - all_through_ground).abs() > 0.5 * expected);
}

/// M2.11, second case. A bowl under a crown that lets 0.4 of the rain through over part
/// of it, on paving. The puddle rises over ground in the shower and falls from it as it
/// dries and goes down through its bed. Each tick, the test finds the cells whose cover changed between the level that
/// last set it and the level at the tick's start, reads the class's stores at each cell's
/// exposure itself, and checks each transfer by itself: surface water joined, pores
/// filled, surface water left behind, and pores soaked.
#[test]
fn water_that_covers_or_leaves_ground_moves_its_stores_by_their_own_entries() {
    let canopy = Canopy::prepare(
        [-1.0, -1.0],
        [60, 60],
        &[shelter("crown", CROWN)],
        &Profile::default(),
    )
    .expect("valid test case");
    let tree = Tree::prepare(&bed(), &canopy, &Profile::default()).expect("valid test case");
    let g = tree.ground();
    let a = CELL * CELL;
    // Pores that fill more slowly than the relief, so water covers ground whose pores
    // still have room; and that pass water down once full, so after the shower the
    // puddle falls from ground that still holds surface water.
    let class = stored(30.0, 10.0, 3.0, 3.0);
    let mut ground = thrum_rain_model::GroundStores::new(&[class]).expect("valid test case");
    let mut state = RainState::new(&tree);
    let mut report = TickReport::default();
    let exposure: Vec<f64> = (0..g.heights().len())
        .map(|i| {
            let [x, z] = g.centre(i);
            canopy.exposure([x, g.heights()[i], z])
        })
        .collect();
    let mut seen = [0_usize; 4];
    let mut shaded = false;
    for t in 0..6000 {
        let rain = if t < 700 {
            Rain {
                flux: mm_h(60.0),
                drying: 0.0,
            }
        } else {
            Rain {
                flux: 0.0,
                drying: mm_h(20.0),
            }
        };
        let old = state.cover_levels()[0];
        let now = state.levels(&tree)[0];
        let read = |e: f64| {
            let s = |r: usize| ground.store(0, r);
            let lerp = |a: thrum_rain_model::Store, b: thrum_rain_model::Store, w: f64| {
                (
                    a.surface + (b.surface - a.surface) * w,
                    a.fill + (b.fill - a.fill) * w,
                )
            };
            if e >= 0.5 {
                lerp(s(1), s(0), (e - 0.5) / 0.5)
            } else if e >= 0.25 {
                lerp(s(2), s(1), (e - 0.25) / 0.25)
            } else {
                lerp(thrum_rain_model::Store::default(), s(2), e / 0.25)
            }
        };
        let (mut surface, mut room) = (0.0, 0.0);
        for (i, &z) in g.heights().iter().enumerate() {
            if tree.labels()[i] != 0 || z < old.min(now) || z >= old.max(now) {
                continue;
            }
            let (sw, f) = read(exposure[i]);
            surface += sw * a;
            room += (1.0 - f) * class.pores * a;
            shaded |= exposure[i] < 1.0 && sw > 0.0;
        }
        state
            .step_with(&tree, rain, &ground, 0, 1.0, 1, &mut report)
            .expect("valid test case");
        ground.step(rain, 1.0).expect("valid test case");
        assert_eq!(state.cover_levels()[0], now);
        let h = report.hollows[0];
        // Within 1e-9 of the figure, or of a tick's shower on the body where it is none.
        let shower = mm_h(60.0) / 1000.0 * (N * N) as f64 * a;
        let scale = surface.max(room).max(shower);
        let (joined, filled, left, soaked) = if now > old {
            (surface, room, 0.0, 0.0)
        } else if now < old {
            (0.0, 0.0, surface, room)
        } else {
            (0.0, 0.0, 0.0, 0.0)
        };
        close("joined", h.joined, joined, scale);
        close("filled", h.filled, filled, scale);
        close("left", h.left, left, scale);
        close("soaked", h.soaked, soaked, scale);
        for (n, v) in seen.iter_mut().zip([joined, filled, left, soaked]) {
            *n += usize::from(v > 0.0);
        }
    }
    assert!(
        seen.iter().all(|&n| n > 10) && shaded,
        "ticks with each entry {seen:?}, shaded ground {shaded}"
    );
    assert_eq!(state.account().unpaid, 0.0);
}
