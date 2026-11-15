//! Beds and brute-force oracles shared by the rain model's tests. Nothing here calls the
//! code under test except to build what a test then checks against its own arithmetic.
#![expect(
    clippy::missing_panics_doc,
    reason = "test support panics on a malformed fixture"
)]
#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    reason = "fixture cell indices are small and checked by the grid they index"
)]
use std::collections::VecDeque;
use thrum_rain_model::{Canopy, Ground, GroundClass, Profile, Rain, RainState, TickReport, Tree};

/// Millimetres an hour as kilograms a square metre a second.
pub fn mm_h(v: f64) -> f64 {
    v / 3600.0
}

/// A small deterministic generator for seeded cases.
pub struct Lcg(pub u64);

impl Lcg {
    /// The next value, an integer below `n`.
    pub fn below(&mut self, n: usize) -> usize {
        let v = self.uniform() * n as f64;
        (0..n).rev().find(|&k| k as f64 <= v).unwrap_or(0)
    }

    /// The next value, uniform in [0, 1).
    pub fn uniform(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// A grid of heights from a function of the cell's centre in metres.
pub fn ground(n: [usize; 2], cell: f64, f: impl Fn(f64, f64) -> f64) -> Ground {
    let heights = (0..n[1])
        .flat_map(|j| (0..n[0]).map(move |i| (i, j)))
        .map(|(i, j)| f((i as f64 + 0.5) * cell, (j as f64 + 0.5) * cell))
        .collect();
    Ground::new([0.0, 0.0], cell, n, heights).expect("valid fixture")
}

/// The PUDDLES design's bed (part 1): 3.2 m square at 5 cm, three bowls and a shallow
/// fourth, a rut, a slope of 4 mm a metre and small relief.
pub fn design_bed() -> Ground {
    let mut r = Lcg(7);
    let waves: Vec<[f64; 4]> = (0..6)
        .map(|_| {
            [
                r.uniform() * (628.0 / 100.0),
                r.uniform() * (628.0 / 100.0),
                2.0 + (r.uniform() * 5.0).floor(),
                2.0 + (r.uniform() * 5.0).floor(),
            ]
        })
        .collect();
    ground([64, 64], 0.05, |x, y| {
        let mut z = 0.060 + 0.004 * x;
        for (cx, cy, rad, dep) in [
            (0.9, 0.9, 0.55, 0.045),
            (1.75, 1.05, 0.40, 0.030),
            (2.3, 2.2, 0.60, 0.050),
            (1.0, 2.3, 0.35, 0.018),
        ] {
            z -= dep * (-((x - cx).powi(2) + (y - cy).powi(2)) / (rad * rad)).exp();
        }
        if 0.4 < x && x < 2.9 {
            z -= 0.012 * (-((y - 1.6) / 0.12).powi(2)).exp();
        }
        for [pa, pb, ka, kb] in &waves {
            z += 0.0012
                * (pa + ka * (628.0 / 100.0) * x / 3.2).sin()
                * (pb + kb * (628.0 / 100.0) * y / 3.2).sin();
        }
        z
    })
}

/// A seeded random bed of bumps and dips, `n` cells square at 5 cm.
pub fn random_bed(seed: u64, n: usize) -> Ground {
    let mut r = Lcg(seed);
    let side = n as f64 * 0.05;
    let bumps: Vec<[f64; 4]> = (0..10 + r.below(14))
        .map(|_| {
            [
                r.uniform() * side,
                r.uniform() * side,
                0.08 + r.uniform() * 0.25,
                (r.uniform() - 0.7) * 0.06,
            ]
        })
        .collect();
    let tilt = [(r.uniform() - 0.5) * 0.02, (r.uniform() - 0.5) * 0.02];
    let rim = 0.02 + r.uniform() * 0.04;
    let noise: Vec<f64> = (0..n * n).map(|_| r.uniform() * 0.002).collect();
    let g = ground([n, n], 0.05, |x, y| {
        // A raised rim, so the dips inside join into trees before water leaves.
        let (u, v) = (2.0 * x / side - 1.0, 2.0 * y / side - 1.0);
        let mut z = tilt[0] * x + tilt[1] * y + rim * (u.powi(4) + v.powi(4));
        for [cx, cy, rad, amp] in &bumps {
            z += amp * (-((x - cx).powi(2) + (y - cy).powi(2)) / (rad * rad)).exp();
        }
        z
    });
    let heights = g.heights().iter().zip(&noise).map(|(h, e)| h + e).collect();
    Ground::new(g.origin(), g.spacing(), g.size(), heights).expect("valid fixture")
}

/// Prepare a bed under open sky with the default profile.
pub fn prepare(g: &Ground) -> Tree {
    Tree::prepare(g, &Canopy::open(), &Profile::default()).expect("valid fixture")
}

/// Strict pits of a bed: interior cells lower than all four neighbours.
pub fn pits(g: &Ground) -> Vec<usize> {
    let z = g.heights();
    (0..z.len())
        .filter(|&i| !g.edge(i) && g.neighbours(i).all(|j| z[j] > z[i]))
        .collect()
}

/// The lowest pass from a cell to every other: the least, over 4-connected paths, of the
/// highest cell on the path. A Dijkstra on the bottleneck, written for the tests.
pub fn minimax_from(g: &Ground, start: usize) -> Vec<f64> {
    let z = g.heights();
    let mut best = vec![f64::INFINITY; z.len()];
    best[start] = z[start];
    let mut done = vec![false; z.len()];
    let mut heap = std::collections::BinaryHeap::new();
    heap.push((std::cmp::Reverse(Ord64(z[start])), start));
    while let Some((std::cmp::Reverse(Ord64(d)), i)) = heap.pop() {
        if done[i] {
            continue;
        }
        done[i] = true;
        for j in g.neighbours(i) {
            let e = d.max(z[j]);
            if e < best[j] {
                best[j] = e;
                heap.push((std::cmp::Reverse(Ord64(e)), j));
            }
        }
    }
    best
}

/// A height ordered totally, for a heap.
#[derive(PartialEq, Clone, Copy)]
pub struct Ord64(pub f64);
impl Eq for Ord64 {}
impl PartialOrd for Ord64 {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Ord64 {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&o.0)
    }
}

/// Wet cells by a flood from each pit, never crossing ground at or above its level.
/// `level_of_pit` gives the level of the water standing over each pit.
pub fn flood_from_pits(g: &Ground, pits: &[(usize, f64)]) -> Vec<Option<f64>> {
    let z = g.heights();
    let mut wet: Vec<Option<f64>> = vec![None; z.len()];
    for &(p, level) in pits {
        if z[p] >= level {
            continue;
        }
        let mut q = VecDeque::from([p]);
        let mut seen = vec![false; z.len()];
        seen[p] = true;
        while let Some(i) = q.pop_front() {
            wet[i] = Some(wet[i].map_or(level, |l: f64| l.max(level)));
            for j in g.neighbours(i) {
                if !seen[j] && z[j] < level {
                    seen[j] = true;
                    q.push_back(j);
                }
            }
        }
    }
    wet
}

/// Step a state `ticks` times.
pub fn run(
    tree: &Tree,
    state: &mut RainState,
    rain: Rain,
    class: GroundClass,
    dt: f64,
    ticks: usize,
) -> TickReport {
    let mut report = TickReport::default();
    for _ in 0..ticks {
        state
            .step(tree, rain, class, dt, 1, &mut report)
            .expect("valid fixture");
    }
    report
}

/// The residual of the water account, as a share of the rain.
pub fn residual(state: &RainState) -> f64 {
    let a = state.account();
    let out = state.held() - a.initial + a.canopy + a.ground + a.percolated + a.dried + a.edge;
    (a.rain - out).abs() / a.rain.max(1e-300)
}

/// A pothole of 4 by 4 cells, 20 mm deep with a flat floor, in a walled region of
/// `width` by `depth` cells whose other cells slope down to it; it spills through a notch in
/// the wall into a moat that leaves over the edge. Its catchment is the whole region.
pub fn pothole(width: usize, depth: usize) -> Ground {
    let n = 24;
    let (x0, z0) = (8, 8);
    ground([n, n], 0.05, |x, y| {
        let (i, j) = (
            ((x / 0.05) - 0.5).round() as i64,
            ((y / 0.05) - 0.5).round() as i64,
        );
        let (ix, jz) = (i - x0, j - z0);
        let inside = |w: usize, a: i64| a >= 0 && a < w as i64;
        if inside(4, ix) && inside(4, jz) {
            -0.02
        } else if inside(width, ix) && inside(depth, jz) {
            let dx = (ix - 3).max(0) as f64;
            let dz = (jz - 3).max(0) as f64;
            0.001 + 0.001 * (dx + dz)
        } else if (ix, jz) == (-1, 2) {
            0.0
        } else if (-1..=width as i64).contains(&ix) && (-1..=depth as i64).contains(&jz) {
            1.0
        } else {
            -1.0
        }
    })
}

/// A strip `rows` cells across between walls, whose height along x runs through control
/// points (cell column, height) and is lowest along the strip's middle row.
pub fn strip(points: &[(f64, f64)], columns: usize, rows: usize) -> Ground {
    let cell = 0.05;
    let middle = (rows as f64 - 1.0) / 2.0;
    ground([columns, rows], cell, |x, y| {
        let row = y / cell - 0.5;
        if row < 0.5 || row > rows as f64 - 1.5 {
            return 0.10;
        }
        let c = x / cell - 0.5;
        let along =
            points
                .windows(2)
                .find(|w| c <= w[1].0)
                .map_or(points[points.len() - 1].1, |w| {
                    let ((x0, z0), (x1, z1)) = (w[0], w[1]);
                    z0 + (z1 - z0) * ((c - x0) / (x1 - x0)).clamp(0.0, 1.0)
                });
        along + 0.002 * ((row - middle) / middle).powi(2)
    })
}

/// Two hollows a sill apart at 0 mm; they leave over a pass at 15 mm.
pub fn pair() -> Ground {
    strip(
        &[
            (0.0, 0.10),
            (1.0, 0.06),
            (6.0, -0.02),
            (11.0, 0.0),
            (16.0, -0.025),
            (21.0, 0.015),
            (25.0, -0.05),
        ],
        26,
        10,
    )
}

/// An egg-crate of many small hollows inside a raised rim, `n` cells square at 5 cm.
pub fn many_hollows(n: usize) -> Ground {
    let side = n as f64 * 0.05;
    ground([n, n], 0.05, |x, y| {
        let (u, v) = (2.0 * x / side - 1.0, 2.0 * y / side - 1.0);
        0.03 * (u.powi(4) + v.powi(4))
            + 0.01 * (x * 9.0).sin() * (y * 7.0).sin()
            + 0.004 * (x * 23.0 + y * 3.0).sin()
    })
}
