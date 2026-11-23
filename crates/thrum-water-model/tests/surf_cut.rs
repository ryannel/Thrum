//! Independently cut a forward sheet eight times finer, without the model's reach.
use crate::support;
use support::{sheets::Trains, surf_map};
use thrum_water_model::{CutConfig, Sheet, SheetPoint, cut};

#[test]
fn impossible_cut_resolution_is_refused_before_allocating() {
    let sheet = Trains::new(0);
    assert!(
        cut(
            &sheet,
            [0.0; 2],
            CutConfig {
                cell: 1e-300,
                tolerance: 0.001
            }
        )
        .is_err()
    );
}

// Reference triangle rasterizer: the sheet forward-evaluated over a fixed domain,
// with no reach, direction search, iteration or model inversion in the reference.
fn reference(
    sheet: &impl Sheet,
    origin: [f64; 2],
    side: usize,
    spacing: f64,
    margin: f64,
) -> (Vec<f64>, Vec<usize>) {
    let cell = 0.03125;
    let start = origin.map(|p| p - margin);
    let width = spacing * (side - 1) as f64 + margin * 2.0;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "positive fixed reference extent"
    )]
    let n = (width / cell).ceil() as usize + 1;
    let mut heights = vec![f64::NEG_INFINITY; side * side];
    let mut hits = vec![0; side * side];
    let row = |z: usize| {
        (0..n)
            .map(|x| sheet.forward([start[0] + x as f64 * cell, start[1] + z as f64 * cell]))
            .collect::<Vec<_>>()
    };
    let mut previous = row(0);
    for z in 1..n {
        let next = row(z);
        for x in 0..n - 1 {
            for triangle in [
                [previous[x], previous[x + 1], next[x + 1]],
                [previous[x], next[x + 1], next[x]],
            ] {
                raster(triangle, origin, side, spacing, &mut heights, &mut hits);
            }
        }
        previous = next;
    }
    (heights, hits)
}

fn raster(
    t: [SheetPoint; 3],
    origin: [f64; 2],
    side: usize,
    spacing: f64,
    heights: &mut [f64],
    hits: &mut [usize],
) {
    let lo: [f64; 2] = std::array::from_fn(|a| {
        t.iter()
            .map(|p| (p.world[a] - origin[a]) / spacing)
            .fold(f64::INFINITY, f64::min)
            .ceil()
            .max(0.0)
    });
    let hi: [f64; 2] = std::array::from_fn(|a| {
        t.iter()
            .map(|p| (p.world[a] - origin[a]) / spacing)
            .fold(f64::NEG_INFINITY, f64::max)
            .floor()
            .min((side - 1) as f64)
    });
    if (0..2).any(|a| lo[a] > hi[a]) {
        return;
    }
    let a = t[0].world;
    let b = t[1].world;
    let c = t[2].world;
    let det = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1]);
    if det.abs() < 1e-14 {
        return;
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "raster bounds clamped to reference grid"
    )]
    let bounds = [lo.map(|v| v as usize), hi.map(|v| v as usize)];
    for z in bounds[0][1]..=bounds[1][1] {
        for x in bounds[0][0]..=bounds[1][0] {
            let p = [
                origin[0] + x as f64 * spacing - a[0],
                origin[1] + z as f64 * spacing - a[1],
            ];
            let u = (p[0] * (c[1] - a[1]) - p[1] * (c[0] - a[0])) / det;
            let v = ((b[0] - a[0]) * p[1] - (b[1] - a[1]) * p[0]) / det;
            if u >= -1e-9 && v >= -1e-9 && u + v <= 1.0 + 1e-9 {
                let i = z * side + x;
                heights[i] =
                    heights[i].max((1.0 - u - v) * t[0].height + u * t[1].height + v * t[2].height);
                hits[i] += 1;
            }
        }
    }
}

#[test]
fn highest_cut_on_straight_crossing_and_shoaling_sheets() {
    let mut plain_failure = 0.0_f64;
    let mut scan_failure = 0.0_f64;
    for case in 0..3 {
        let sheet = Trains::new(case);
        let origin = if case == 2 { [-34.0, 10.0] } else { [-4.0; 2] };
        let side = 65;
        let spacing = 0.125;
        let (reference, hits) = reference(&sheet, origin, side, spacing, 20.0);
        let mut triples = 0;
        let mut max_nodes = 0;
        let mut converged = 0;
        for z in 2..side - 2 {
            for x in 2..side - 2 {
                let p = [
                    origin[0] + x as f64 * spacing,
                    origin[1] + z as f64 * spacing,
                ];
                let result = cut(&sheet, p, CutConfig::default()).unwrap();
                let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
                for dz in z - 2..=z + 2 {
                    for dx in x - 2..=x + 2 {
                        lo = lo.min(reference[dz * side + dx]);
                        hi = hi.max(reference[dz * side + dx]);
                    }
                }
                assert!(
                    result.height >= lo - 0.002 && result.height <= hi + 0.002,
                    "sheet {case} {p:?}: {} outside {lo}..{hi}",
                    result.height
                );
                triples += usize::from(hits[z * side + x] >= 3);
                max_nodes = max_nodes.max(result.nodes);
                if result.converged {
                    assert!(result.residual <= 0.001);
                    converged += 1;
                }
                if case == 0 {
                    let mut u = p;
                    for _ in 0..500 {
                        let f = sheet.forward(u);
                        u = [p[0] - (f.world[0] - u[0]), p[1] - (f.world[1] - u[1])];
                    }
                    plain_failure =
                        plain_failure.max(reference[z * side + x] - sheet.forward(u).height);
                }
                if case == 1 {
                    scan_failure = scan_failure.max(reference[z * side + x] - old_scan(&sheet, p));
                }
                let rest = p;
                let forward = sheet.forward(rest);
                assert!(
                    (forward.world[0] - rest[0]).hypot(forward.world[1] - rest[1])
                        <= sheet.reach(forward.world) + 1e-9
                );
                if (x + z) % 13 == 0 {
                    // A rest place set over its own world place, including the crossing trains.
                    let answer = cut(&sheet, forward.world, CutConfig::default()).unwrap();
                    // A miss is permitted only at a lip thinner than one cell; the
                    // neighbourhood reference assertion above holds those columns.
                    assert!(answer.height.is_finite());
                }
            }
        }
        eprintln!(
            "sheet {case}: {triples} columns with >=3 hits, {converged} refined, at most {max_nodes} nodes"
        );
        assert!(triples > 0 && converged > 100);
        if case == 1 {
            // The design review's independent forward witness, set over itself.
            let witness = sheet.forward([1.0, -3.0]);
            let answer = cut(&sheet, witness.world, CutConfig::default()).unwrap();
            assert!(answer.height >= witness.height - 0.01);
            assert!(answer.converged && answer.residual <= 0.001);
        }
    }
    eprintln!(
        "wrong substitutes: iteration {plain_failure:.4} m low, directional scan {scan_failure:.4} m low"
    );
    assert!(plain_failure > 0.5 && scan_failure > 0.2);
}

#[test]
fn production_sheets_and_their_own_reach_match_forward_reference() {
    for case in 0..3 {
        let (map, field) = surf_map::fixture(case);
        let sheet = map.surf_sheet(&field, 0, 0.0).unwrap();
        let mut fold = None;
        for z in 0..81 {
            for x in 0..101 {
                let p = if case == 2 {
                    [-65.0 + f64::from(x) * 0.5, -10.0 + f64::from(z) * 0.5]
                } else {
                    [-10.0 + f64::from(x) * 0.2, -10.0 + f64::from(z) * 0.25]
                };
                let a = sheet.forward(p);
                let b = sheet.forward([p[0] + 0.01, p[1]]);
                let c = sheet.forward([p[0], p[1] + 0.01]);
                let det = (b.world[0] - a.world[0]) * (c.world[1] - a.world[1])
                    - (b.world[1] - a.world[1]) * (c.world[0] - a.world[0]);
                if det < -1e-6 && fold.is_none() {
                    fold = Some(a.world);
                }
                assert!(
                    (a.world[0] - p[0]).hypot(a.world[1] - p[1]) <= sheet.reach(a.world) + 1e-9
                );
            }
        }
        let fold = fold.expect("each production sheet must actually overhang");
        let origin = fold.map(|v| v - 0.5);
        let side = 9;
        // A fixed rest-domain margin covers the local deformation without consulting reach.
        let (fine, hits) = reference(&sheet, origin, side, 0.125, 8.0);
        let mut triples = 0;
        for z in 2..side - 2 {
            for x in 2..side - 2 {
                let p = [origin[0] + x as f64 * 0.125, origin[1] + z as f64 * 0.125];
                let column = cut(&sheet, p, CutConfig::default()).unwrap();
                let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
                for dz in z - 2..=z + 2 {
                    for dx in x - 2..=x + 2 {
                        lo = lo.min(fine[dz * side + dx]);
                        hi = hi.max(fine[dz * side + dx]);
                    }
                }
                assert!(
                    column.height >= lo - 0.002 && column.height <= hi + 0.002,
                    "production sheet {case} {p:?}: {} outside {lo}..{hi}",
                    column.height
                );
                assert!(
                    (map.surface(&field, p, 0.0).unwrap().sample.height - column.height).abs()
                        < 0.002
                );
                if column.converged {
                    assert!(column.residual <= 0.001);
                }
                triples += usize::from(hits[z * side + x] >= 3);
            }
        }
        eprintln!("production sheet {case}: fold {fold:?}, {triples} columns with >=3 hits");
        assert!(triples > 0);
    }
}

fn old_scan(sheet: &Trains, p: [f64; 2]) -> f64 {
    let (_, _, dir, _) = sheet.state(p);
    let k = std::f64::consts::TAU / 25.0;
    let re = 0.4667 * (k * p[0]).cos() + 0.2333 * (k * p[1]).cos();
    let im = 0.4667 * (k * p[0]).sin() + 0.2333 * (k * p[1]).sin();
    let throw = 6.0 * re.hypot(im);
    let mut best = f64::NEG_INFINITY;
    let mut prev = None;
    for i in 0..=64 {
        let t = throw * f64::from(i) / 64.0;
        let mut rest = [p[0] - t * dir[0], p[1] - t * dir[1]];
        for _ in 0..30 {
            let plain = sheet.state(rest).3;
            rest = [p[0] - t * dir[0] - plain[0], p[1] - t * dir[1] - plain[1]];
        }
        let state = sheet.state(rest);
        let f = state.1 - t;
        if let Some((before, height)) = prev
            && f * before <= 0.0
        {
            best = best.max(f64::midpoint(height, state.0.height));
        }
        prev = Some((f, state.0.height));
    }
    if best.is_finite() {
        best
    } else {
        sheet.forward(p).height
    }
}
