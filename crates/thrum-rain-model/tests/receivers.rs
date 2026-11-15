//! M1.13: each tick's rain on a cell goes to one receiver, and each receiver's gain is
//! right by itself. The test sums each receiver over cells from the bed, the levels and the
//! shelters alone: which cells drain to the hollow is known from how the bed is made, and
//! exposure from the shelters' rectangles, without the model.
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

/// What each shelter withholds at a cell: its loss of share times the part of the square of
/// the edge width round the cell that its rectangle covers. The two do not overlap.
fn withheld(x: f64, z: f64) -> [f64; 2] {
    let half = 0.5 * Profile::default().edge_width;
    let cover = |r: [f64; 6]| {
        let ox = ((x + half).min(r[2]) - (x - half).max(r[0])).max(0.0);
        let oz = ((z + half).min(r[3]) - (z - half).max(r[1])).max(0.0);
        (1.0 - r[5]) * ox * oz / (4.0 * half * half)
    };
    [cover(ROOF), cover(CROWN)]
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
            let [roof, crown] = withheld(x, z);
            let e = 1.0 - roof - crown;
            s.canopy += fall * a * (roof + crown);
            let rain = e * fall;
            if inner(i, j) && g.heights()[j * N + i] < level {
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
        &[shelter("roof", ROOF), shelter("crown", CROWN)],
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
        let class = GroundClass { percolation };
        let mut state = RainState::new(&tree);
        let mut report = TickReport::default();
        let mut checked = 0;
        for t in 0..4000 {
            let level = state.levels(&tree)[0];
            let wet = state.held() > 0.0;
            state
                .step(&tree, rain, class, 1.0, 1, &mut report)
                .expect("valid test case");
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
            let fall = rain.flux / 1000.0;
            let total = fall * (N * N) as f64 * CELL * CELL;
            let s = sums(g, level, fall, percolation / 1000.0);
            let h = report.hollows[0];
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
            assert!(s.canopy > 0.0 && s.on_water > 0.0 && s.runoff > 0.0 && s.edge > 0.0);
            // The roof's edge shelters part of the water: less than open rain reaches it.
            assert!(s.on_water < fall * covered as f64 * CELL * CELL);
            checked += 1;
        }
        assert_eq!(checked, 3);
    }
}
