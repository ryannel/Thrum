//! M1.1: on the test valley, discharge is kept through every section, the pool stands level at
//! the weir relation's height over its sill, the water quickens through the narrows, and the
//! hollow beside the channel stays dry. Every expected figure is worked out here from
//! continuity and the weir relation, not read from the solver.
#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "cell indices of positive extents"
)]
use crate::support;
use support::valley::{CENTRE, CREST, DISCHARGE, HOLLOW, NARROWS, NOTCH, bed, valley};
use thrum_river_model::{Profile, Solution, solve};

const G: f64 = 9.81;

fn column(s: &Solution, x: f64) -> usize {
    let g = s.grid();
    ((x - g.origin()[0]) / g.spacing()).round() as usize
}

/// Wet width, discharge and wet area through the column of cells at `x`.
fn section(s: &Solution, x: f64) -> (f64, f64, f64) {
    let g = s.grid();
    let [nx, nz] = g.size();
    let c = column(s, x);
    let dx = g.spacing();
    let mut width = 0.0;
    let mut q = 0.0;
    let mut area = 0.0;
    for z in 0..nz {
        let i = z * nx + c;
        if s.wet[i] {
            width += dx;
            q += s.state.qx[i] * dx;
            area += s.state.depth[i] * dx;
        }
    }
    (width, q, area)
}

#[test]
fn the_valley_keeps_its_discharge_fills_its_pool_to_the_weir_and_quickens_in_the_narrows() {
    let (grid, declaration) = valley(0.5);
    let s = solve(&grid, &declaration, &Profile::default()).unwrap();
    let r = &s.report;
    println!(
        "valley: {} steps, {} cell updates, {:.2} s, {} wet cells, level rate {:.1e}, imbalance {:.1e}",
        r.steps, r.cell_updates, r.seconds, r.wet_cells, r.level_rate, r.imbalance
    );
    let [nx, nz] = grid.size();
    let dx = grid.spacing();

    // Continuity: every section below the spring carries the spring's discharge.
    let mut worst = 0.0_f64;
    for c in 0..=nx {
        let x = grid.origin()[0] + (c as f64 - 0.5) * dx;
        if !(4.0..=41.8).contains(&x) {
            continue;
        }
        let through: f64 = (0..nz).map(|z| s.x_flux[z * (nx + 1) + c]).sum();
        worst = worst.max((through / DISCHARGE - 1.0).abs());
    }
    println!(
        "discharge through every section within {:.3}% of the spring's",
        100.0 * worst
    );
    assert!(worst < 0.01);

    // The pool, away from where the stream enters it, is level, and stands at the weir
    // relation's height over its sill: a head of about 1.5 critical depths over a broad crest.
    let critical = (DISCHARGE * DISCHARGE / (G * NOTCH * NOTCH)).cbrt();
    let pool: Vec<f64> = (0..nx * nz)
        .filter(|&i| {
            let p = grid.point(i);
            s.wet[i] && (12.5..=17.5).contains(&p[0])
        })
        .map(|i| s.level(i))
        .collect();
    let (lo, hi) = pool
        .iter()
        .fold((f64::MAX, f64::MIN), |(a, b), &v| (a.min(v), b.max(v)));
    let mean = pool.iter().sum::<f64>() / pool.len() as f64;
    let over = (mean - CREST) / critical;
    println!(
        "pool: {} cells level to {:.1} mm, {:.3} m over the sill, {over:.2} critical depths ({critical:.3} m)",
        pool.len(),
        1000.0 * (hi - lo),
        mean - CREST
    );
    assert!(hi - lo < 0.03);
    assert!((1.3..=1.9).contains(&over));

    // The narrows: continuity puts the speed up by at least the ratio of the wet widths.
    let (w_above, q_above, a_above) = section(&s, 27.0);
    let mid = f64::midpoint(NARROWS[0], NARROWS[1]);
    let (w_narrow, q_narrow, a_narrow) = section(&s, mid);
    let (u_above, u_narrow) = (q_above / a_above, q_narrow / a_narrow);
    println!(
        "above the narrows {w_above:.2} m wide at {u_above:.2} m/s; in it {w_narrow:.2} m at {u_narrow:.2} m/s"
    );
    assert!(u_narrow / u_above >= w_above / w_narrow - 0.2);

    // The hollow: lower than the pool's water, cut off by ground, and dry.
    let hollow: Vec<usize> = (0..nx * nz)
        .filter(|&i| {
            let p = grid.point(i);
            (p[0] - HOLLOW[0]).hypot(p[1] - HOLLOW[1]) < 1.0
        })
        .collect();
    let bottom = hollow
        .iter()
        .map(|&i| grid.heights()[i])
        .fold(f64::MAX, f64::min);
    let ridge = (0..40)
        .map(|k| bed(HOLLOW[0], CENTRE + 2.5 + 0.025 * f64::from(k)))
        .fold(f64::MIN, f64::max);
    assert!(
        bottom < lo && ridge > hi,
        "the hollow is lower than the water and cut off from it"
    );
    assert!(hollow.iter().all(|&i| s.state.depth[i] == 0.0 && !s.wet[i]));
}
