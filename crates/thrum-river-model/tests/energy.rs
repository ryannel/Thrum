//! M1.4: the energy account closes; the chart's loss is the power that breaks the surface and
//! lies at the ledge, the rock and below the narrows, and not on a uniform channel. Friction's
//! power is worked out here from Manning's relation, not read from the solver's account.
#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "cell indices of positive extents"
)]
use crate::support;
use support::{section, valley};
use thrum_river_model::{Profile, Solution, solve};

const G: f64 = 9.81;
const RHO: f64 = 1000.0;

/// Friction's power from Manning's relation, `ρ g |q| n² |u|² / h^(4/3)`, W/m².
fn manning_power(s: &Solution, i: usize, n: f64) -> f64 {
    let h = s.state.depth[i];
    if s.is_held(i) || h <= 1e-6 {
        return 0.0;
    }
    let q = s.state.qx[i].hypot(s.state.qz[i]);
    let u = q / h;
    RHO * G * q * n * n * u * u / h.powf(4.0 / 3.0)
}

/// Head lost cell by cell, times discharge, against discharge times the head between the
/// spring and the mouth (the heads of the cells that pour into the mouth, by their discharge).
fn account(s: &Solution, spring: &[usize], mouth_column: usize, q: f64) -> (f64, f64) {
    let g = s.grid();
    let [nx, nz] = g.size();
    let area = g.spacing() * g.spacing();
    let lost: f64 = s.lost.iter().sum::<f64>() * area / (RHO * G);
    let source = spring.iter().map(|&i| s.level(i)).sum::<f64>() / spring.len() as f64;
    let mut out = 0.0;
    let mut through = 0.0;
    for z in 0..nz {
        let f = s.x_flux[z * (nx + 1) + mouth_column];
        if f > 0.0 {
            out += f * s.head(z * nx + mouth_column - 1);
            through += f;
        }
    }
    (lost, q * source - q * out / through)
}

/// Part 2's stream in section: riffle at 3%, a pool behind a sill, a chute at 12%, a ledge of
/// 0.4 m, a run at 1% and a glide at 0.5%.
fn stream_bed(dx: f64) -> Vec<f64> {
    let n = (120.0 / dx).round() as usize;
    (0..n)
        .map(|i| {
            let x = (i as f64 + 0.5) * dx;
            if x < 25.0 {
                12.0 - 0.03 * x
            } else if x < 45.0 {
                11.25 - 0.8 * (std::f64::consts::PI * (x - 25.0) / 20.0).sin().powi(2)
            } else if x < 60.0 {
                11.25 - 0.12 * (x - 45.0)
            } else if x < 85.0 {
                9.45 - 0.4 - 0.01 * (x - 60.0)
            } else {
                8.80 - 0.005 * (x - 85.0)
            }
        })
        .collect()
}

#[test]
fn in_part_twos_section_the_loss_lies_at_the_ledge_and_the_account_closes() {
    let (dx, q, n) = (0.5, 0.15, 0.04);
    let bed = stream_bed(dx);
    let len = bed.len();
    let last = bed[len - 1];
    let (grid, declaration) = section::section(&bed, dx, q, last - 1.0, last + 0.45, n);
    let s = solve(&grid, &declaration, &section::profile()).unwrap();
    let nx = grid.size()[0];
    let cell = |x: f64| nx + 1 + (x / dx) as usize;
    let most = |a: f64, b: f64| (cell(a)..cell(b)).map(|i| s.loss[i]).fold(0.0, f64::max);
    let (ledge, chute, glide, pool) = (
        most(58.0, 66.0),
        most(47.0, 57.5),
        most(87.0, 118.0),
        most(27.0, 43.0),
    );
    println!("loss, W/m2: ledge {ledge:.1}, chute {chute:.1}, glide {glide:.2}, pool {pool:.2}");
    assert!(ledge >= 5.0 * chute && ledge >= 50.0 * glide && pool < 0.02 * ledge);
    let (lost, between) = account(&s, &[nx + 1], 1 + len, q * dx);
    println!(
        "account: {lost:.5} against {between:.5} m3/s.m ({:.3}% apart)",
        100.0 * (lost / between - 1.0)
    );
    assert!((lost / between - 1.0).abs() < 0.02);
}

#[test]
fn in_the_valley_the_head_lost_beyond_friction_lies_round_the_rock_and_below_the_narrows_and_ledge()
{
    use valley::{CENTRE, DISCHARGE, LEDGE, MANNING, NARROWS, ROCK};
    let (grid, declaration) = valley::valley(0.25);
    let s = solve(&grid, &declaration, &Profile::default()).unwrap();
    let [nx, nz] = grid.size();
    let spring: Vec<usize> = (0..nx * nz)
        .filter(|&i| {
            let p = grid.point(i);
            (p[0] - 2.5).hypot(p[1] - CENTRE) <= 0.75
        })
        .collect();
    let mouth = ((42.0 - grid.origin()[0]) / grid.spacing()).round() as usize;
    let mouth = mouth.min(nx);
    let (lost, between) = account(&s, &spring, mouth, DISCHARGE);
    println!(
        "account: {lost:.5} against {between:.5} m3/s.m ({:.3}% apart)",
        100.0 * (lost / between - 1.0)
    );
    assert!((lost / between - 1.0).abs() < 0.02);

    let area = grid.spacing() * grid.spacing();
    let mut total = 0.0;
    let mut inside = 0.0;
    let (mut wet, mut wet_inside) = (0, 0);
    for i in 0..nx * nz {
        if !s.wet[i] {
            continue;
        }
        let p = grid.point(i);
        let beyond = (s.lost[i] - manning_power(&s, i, MANNING)).max(0.0) * area;
        let near = (p[0] - ROCK[0]).hypot(p[1] - ROCK[1]) < 1.5
            || (ROCK[0]..ROCK[0] + 4.0).contains(&p[0])
            || (NARROWS[0]..NARROWS[1] + 5.0).contains(&p[0])
            || (LEDGE..LEDGE + 3.5).contains(&p[0]);
        total += beyond;
        wet += 1;
        if near {
            inside += beyond;
            wet_inside += 1;
        }
    }
    let (share, cells) = (inside / total, f64::from(wet_inside) / f64::from(wet));
    println!(
        "head lost beyond friction: {:.0}% of it in {:.0}% of the wet cells",
        100.0 * share,
        100.0 * cells
    );
    assert!(share >= 0.6 && cells < 0.4);
}

#[test]
fn a_straight_uniform_channel_breaks_no_surface() {
    let (dx, q, n, slope, cells) = (0.25, 0.2, 0.03, 0.005_f64, 240);
    let normal = (q * n / slope.sqrt()).powf(0.6);
    let bed: Vec<f64> = (0..cells).map(|k| -slope * (k as f64 + 0.5) * dx).collect();
    let end = -slope * cells as f64 * dx;
    let (grid, declaration) = section::section(&bed, dx, q, end - 1.0, end + normal, n);
    let s = solve(&grid, &declaration, &section::profile()).unwrap();
    let nx = grid.size()[0];
    let middle = nx + 1 + cells / 4..nx + 1 + 3 * cells / 4;
    let friction: f64 = middle.clone().map(|i| manning_power(&s, i, n)).sum();
    let loss: f64 = middle.clone().map(|i| s.loss[i]).sum();
    let depth = section::depths(&s, cells)[cells / 2];
    println!(
        "uniform channel at {depth:.4} m (normal {normal:.4} m): loss {:.3}% of friction's power",
        100.0 * loss / friction
    );
    assert!(loss < 0.01 * friction);
}
