//! The solver in section against closed forms this test works out itself, as in part 1 of
//! `tests/fixtures/rivers/river_proof.py`: water at rest, Bernoulli's branches and
//! the jump condition over a bump, a bed integrated for a chosen depth with friction, and
//! Manning's normal depth for a thin sheet on steep ground.
#![expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::unwrap_used,
    clippy::print_stdout,
    reason = "test arithmetic over cell indices; helpers report what they measured"
)]
use crate::support;
use support::section::{depths, profile, section};
use thrum_river_model::{Grid, Manning, State, Stepper, solve};

const G: f64 = 9.81;

fn bisect(f: impl Fn(f64) -> f64, mut a: f64, mut b: f64) -> f64 {
    let fa = f(a);
    for _ in 0..200 {
        let m = f64::midpoint(a, b);
        if (f(m) > 0.0) == (fa > 0.0) {
            a = m;
        } else {
            b = m;
        }
    }
    f64::midpoint(a, b)
}

#[test]
fn water_at_rest_over_a_rough_bed_with_dry_ground_stays_at_rest() {
    let (nx, nz, dx) = (48, 32, 0.25);
    let heights: Vec<f64> = (0..nx * nz)
        .map(|i| {
            let (x, z) = ((i % nx) as f64 * dx, (i / nx) as f64 * dx);
            0.5 * (-((x - 6.0).powi(2) + (z - 4.0).powi(2)) / 2.0).exp()
                + 0.06 * (x * 3.7).sin().powi(2)
                + 0.05 * (z * 2.3 + x).cos()
                + if (x - 2.0).abs() < 0.6 && (z - 5.0).abs() < 0.6 {
                    0.6
                } else {
                    0.0
                }
        })
        .collect();
    let grid = Grid::new([0.0; 2], dx, [nx, nz], heights.clone()).unwrap();
    let level = 0.35;
    let mut state = State::at_rest(&grid, |_| level);
    let dry = state.depth.iter().filter(|&&h| h == 0.0).count();
    assert!(dry > 40, "the bed has dry ground: {dry} cells");
    let mut stepper = Stepper::new(&grid, &Manning::Uniform(0.05), 0.45, 2).unwrap();
    for _ in 0..2000 {
        stepper.step(&mut state);
    }
    let mut worst_level = 0.0_f64;
    let mut worst_q = 0.0_f64;
    for (i, _) in heights.iter().enumerate().take(nx * nz) {
        if heights[i] < level {
            worst_level = worst_level.max((state.depth[i] + heights[i] - level).abs());
        } else {
            worst_level = worst_level.max(state.depth[i]);
        }
        worst_q = worst_q.max(state.qx[i].abs()).max(state.qz[i].abs());
    }
    println!("at rest after 2000 steps: level {worst_level:.2e} m, discharge {worst_q:.2e} m2/s");
    assert!(worst_level < 1e-12 && worst_q < 1e-12);
}

/// SWASHES' transcritical flow over a bump with a jump: q = 0.18 m²/s, 0.33 m downstream.
fn bump_error(dx: f64) -> (f64, f64, f64, f64) {
    let (length, q, hd) = (25.0, 0.18, 0.33);
    let bump = |x: f64| {
        if 8.0 < x && x < 12.0 {
            0.2 - 0.05 * (x - 10.0).powi(2)
        } else {
            0.0
        }
    };
    let hc = (q * q / G).cbrt();
    let energy = |h: f64, zb: f64| h + q * q / (2.0 * G * h * h) + zb;
    let ec = energy(hc, 0.2);
    let ed = energy(hd, 0.0);
    let slow = |e: f64, zb: f64| bisect(|h| energy(h, zb) - e, hc, 5.0);
    let fast = |e: f64, zb: f64| bisect(|h| energy(h, zb) - e, 1e-4, hc);
    // The jump stands where the fast branch's momentum meets the slow branch's.
    let momentum = |x: f64| {
        let (h1, h2) = (fast(ec, bump(x)), slow(ed, bump(x)));
        q * q * (1.0 / h1 - 1.0 / h2) + 0.5 * G * (h1 * h1 - h2 * h2)
    };
    let xs = bisect(momentum, 11.27, 11.99);
    let exact = |x: f64| {
        if x < 10.0 {
            slow(ec, bump(x))
        } else if x < xs {
            fast(ec, bump(x))
        } else {
            slow(ed, bump(x))
        }
    };
    let n = (length / dx).round() as usize;
    let bed: Vec<f64> = (0..n).map(|k| bump((k as f64 + 0.5) * dx)).collect();
    let (grid, declaration) = section(&bed, dx, q, -0.5, hd, 0.0);
    let s = solve(&grid, &declaration, &profile()).unwrap();
    let h = depths(&s, n);
    let mean = (0..n)
        .map(|k| (h[k] - exact((k as f64 + 0.5) * dx)).abs())
        .sum::<f64>()
        / n as f64;
    // The jump is where the depth rises most from one cell to the next.
    let jump = (0..n - 1)
        .max_by(|&a, &b| (h[a + 1] - h[a]).total_cmp(&(h[b + 1] - h[b])))
        .unwrap();
    let place = (jump + 1) as f64 * dx;
    println!(
        "bump at {dx} m cells: mean depth error {mean:.4} m ({:.2}% of critical), jump at {place:.3} m against {xs:.3} m, {} steps",
        100.0 * mean / hc,
        s.report.steps
    );
    (mean / hc, place, xs, dx)
}

#[test]
fn a_flow_over_a_bump_has_its_jump_where_momentum_puts_it() {
    let (coarse, place, xs, dx) = bump_error(0.25);
    assert!(
        coarse < 0.03,
        "mean depth error {coarse} of the critical depth"
    );
    assert!(
        (place - xs).abs() <= 2.0 * dx,
        "jump at {place} against {xs}"
    );
    let (fine, place, xs, dx) = bump_error(0.125);
    assert!(fine < coarse, "finer cells {fine} against {coarse}");
    assert!(
        (place - xs).abs() <= 2.0 * dx,
        "jump at {place} against {xs}"
    );
}

/// A bed integrated so that the steady depth with friction is `depth(x)`:
/// dz/dx = (Fr² − 1) dh/dx − n² q² / h^(10/3).
fn made_bed(
    depth: &dyn Fn(f64) -> f64,
    slope_of: &dyn Fn(f64) -> f64,
    q: f64,
    n: f64,
    len: f64,
    dx: f64,
) -> Vec<f64> {
    let cells = (len / dx).round() as usize;
    let m = 40;
    let fine = dx / f64::from(m);
    let slope = |x: f64| {
        let h = depth(x);
        (q * q / (G * h.powi(3)) - 1.0) * slope_of(x) - n * n * q * q / h.powf(10.0 / 3.0)
    };
    let mut z = vec![0.0];
    let mut acc = 0.0;
    for k in 0..cells * m as usize {
        let x = k as f64 * fine;
        acc += f64::midpoint(slope(x), slope(x + fine)) * fine;
        z.push(acc);
    }
    (0..cells)
        .map(|i| z[i * m as usize + m as usize / 2])
        .collect()
}

/// Largest depth error over the measured stretch, as a share of the base depth.
fn friction_error(fast: bool, dx: f64) -> f64 {
    let (q, n, period) = (0.3, 0.03, 100.0);
    let (base, amp) = if fast { (0.12, 0.02) } else { (0.5, 0.1) };
    // A fast flow is reached the way a river reaches one: from slow water through a crest.
    let lead = if fast { 20.0 } else { 0.0 };
    let (start, top) = (0.4, lead);
    let hs = move |x: f64| base + amp * (2.0 * std::f64::consts::PI * (x - lead) / period).sin();
    let dhs = move |x: f64| {
        amp * 2.0 * std::f64::consts::PI / period
            * (2.0 * std::f64::consts::PI * (x - lead) / period).cos()
    };
    let g = move |x: f64| {
        if x < top {
            (1.0 - x / top).powi(3)
        } else {
            0.0
        }
    };
    let dg = move |x: f64| {
        if x < top {
            -3.0 * (1.0 - x / top).powi(2) / top
        } else {
            0.0
        }
    };
    let depth = move |x: f64| hs(x) + (start - hs(x)) * g(x);
    let slope_of = move |x: f64| dhs(x) * (1.0 - g(x)) + (start - hs(x)) * dg(x);
    let len = lead + period;
    let bed = made_bed(&depth, &slope_of, q, n, len, dx);
    let cells = bed.len();
    let last = *bed.last().unwrap();
    let (grid, declaration) = if fast {
        section(&bed, dx, q, last - 3.0, last - 2.9, n)
    } else {
        let end = last + depth(len) - (bed[cells - 2] - last) * 0.5;
        section(&bed, dx, q, last - 1.0, end, n)
    };
    let s = solve(&grid, &declaration, &profile()).unwrap();
    let h = depths(&s, cells);
    let from = ((lead + if fast { 5.0 } else { 1.0 }) / dx) as usize;
    let to = cells - (2.0 / dx) as usize;
    let worst = (from..to)
        .map(|k| (h[k] - depth((k as f64 + 0.5) * dx)).abs())
        .fold(0.0, f64::max);
    println!(
        "friction, {} at {dx} m cells: largest depth error {:.2}% of the depth, {} steps",
        if fast { "fast" } else { "slow" },
        100.0 * worst / base,
        s.report.steps
    );
    worst / base
}

#[test]
fn a_slow_flow_with_friction_holds_its_made_depth() {
    assert!(friction_error(false, 0.25) < 0.01);
}

#[test]
fn a_fast_flow_with_friction_holds_its_made_depth_and_converges() {
    let coarse = friction_error(true, 0.25);
    let fine = friction_error(true, 0.125);
    assert!(coarse < 0.08, "fast flow at 0.25 m cells off by {coarse}");
    assert!(
        fine < coarse * 2.0 / 3.0,
        "0.125 m cells {fine} against {coarse}"
    );
}

#[test]
fn a_thin_sheet_on_steep_ground_stands_at_its_normal_depth() {
    let (slope, q, n, dx, cells) = (0.3, 0.02, 0.05, 0.25, 80);
    // The bed falls 0.075 m a cell, three times the sheet's depth.
    let normal = (q * n / f64::sqrt(slope)).powf(0.6);
    let bed: Vec<f64> = (0..cells).map(|k| -(k as f64 + 0.5) * dx * slope).collect();
    let last = bed[cells - 1];
    let (grid, declaration) = section(&bed, dx, q, last - 3.0, last - 2.9, n);
    let s = solve(&grid, &declaration, &profile()).unwrap();
    let h = depths(&s, cells);
    let stretch = &h[cells / 2..cells - 5];
    let mean = stretch.iter().sum::<f64>() / stretch.len() as f64;
    println!(
        "sheet: normal depth {normal:.4} m, solved {mean:.4} m ({:+.1}%)",
        100.0 * (mean / normal - 1.0)
    );
    assert!((mean / normal - 1.0).abs() < 0.08);
}
