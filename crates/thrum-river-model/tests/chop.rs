//! M3.4, ruling 3's chop: the flow's chop is written as two flow gains in each sample. Its rms
//! height is `c d min(S, S_cap)` from the depth and the friction slope `S = n² u² / d^(4/3)`,
//! with `c = 2` and `S_cap = 0.6 / (4c)` as the design gives them: none at rest; in the valley
//! of M1.1 at least 20 times larger in the riffle than in the pool; never more than 0.15 of the
//! depth. It is all in the short band at 0.15 m of depth and all in the long band at 1 m, and
//! between, the long band's share of the variance follows the logarithm of the depth.
//!
//! The expected heights are worked out here from Manning's relation at the solver's own depths
//! and speeds, with this file's arithmetic, not the solver's.
#![expect(
    clippy::cast_precision_loss,
    reason = "test arithmetic; the test reports what it checked"
)]
use crate::support;
use support::valley::{CENTRE, DISCHARGE, MANNING, valley};
use thrum_river_model::{Profile, key_for_samples, solve, solve_samples, write};
use thrum_water_model::RiverFile;

/// The design's constants.
const C: f64 = 2.0;
const S_CAP: f64 = 0.6 / (4.0 * C);
const SHALLOW: f64 = 0.15;
const DEEP: f64 = 1.0;

/// The chop's two band heights at a depth and speed, by the design's law.
fn expected(depth: f64, speed: f64) -> [f64; 2] {
    let s = MANNING * MANNING * speed * speed / depth.powf(4.0 / 3.0);
    let eta = C * depth * s.min(S_CAP);
    let long = ((depth / SHALLOW).ln() / (DEEP / SHALLOW).ln()).clamp(0.0, 1.0);
    [eta * (1.0 - long).sqrt(), eta * long.sqrt()]
}

#[test]
fn the_chop_follows_the_friction_slope_and_is_far_rougher_in_the_riffle_than_the_pool() {
    let (grid, declaration) = valley(0.5);
    let profile = Profile::default();
    let s = solve(&grid, &declaration, &profile).unwrap();
    let rms = profile.band_rms;
    let z = grid.heights();
    let (mut worst, mut most_share, mut checked) = (0.0f64, 0.0f64, 0);
    let (mut riffle, mut pool) = (Vec::new(), Vec::new());
    for (i, _) in z.iter().enumerate() {
        let gains = s.gains[i];
        if !s.wet[i] {
            // A dry cell carries none.
            assert_eq!(gains, [0.0; 2]);
            continue;
        }
        let d = s.level(i) - z[i];
        let u = s.current(i);
        let speed = u[0].hypot(u[1]);
        let want = expected(d, speed);
        let got = [gains[0] * rms[0], gains[1] * rms[1]];
        for b in 0..2 {
            worst = worst.max((got[b] - want[b]).abs() / want[0].hypot(want[1]).max(1e-12));
        }
        let eta = got[0].hypot(got[1]);
        most_share = most_share.max(eta / d);
        // All short at 0.15 m and under, all long at 1 m and over.
        if d <= SHALLOW {
            assert_eq!(gains[1], 0.0, "depth {d}");
        }
        if d >= DEEP {
            assert_eq!(gains[0], 0.0, "depth {d}");
        }
        checked += 1;
        // The riffle: the head run, below where the spring's water spreads and above the
        // pool. The pool: its water away from where the stream enters, as M1.1 takes it.
        let p = grid.point(i);
        let centre = (p[1] - CENTRE).abs() < 0.5;
        if (3.0..9.0).contains(&p[0]) {
            riffle.push((eta, centre));
        } else if (12.5..17.5).contains(&p[0]) {
            pool.push((eta, centre));
        }
    }
    let mean = |v: &[(f64, bool)]| v.iter().map(|e| e.0).sum::<f64>() / v.len() as f64;
    let (r, q) = (mean(&riffle), mean(&pool));
    let rms =
        |v: &[(f64, bool)]| (v.iter().map(|e| e.0 * e.0).sum::<f64>() / v.len() as f64).sqrt();
    let line = |v: &[(f64, bool)]| {
        let c: Vec<_> = v.iter().copied().filter(|e| e.1).collect();
        mean(&c)
    };
    println!(
        "{checked} wet cells: the gains give the law's heights to {worst:e} of the whole; the chop \
         is at most {most_share:.3} of the depth"
    );
    println!(
        "riffle ({} cells) {:.2} mm rms on average, pool ({} cells) {:.3} mm: {:.1} times; \
         by the root mean square over each, {:.1} times; along the centre line, where the stream's \
         jet crosses the pool, {:.1} times",
        riffle.len(),
        r * 1e3,
        pool.len(),
        q * 1e3,
        r / q,
        rms(&riffle) / rms(&pool),
        line(&riffle) / line(&pool)
    );
    assert!(worst < 1e-9);
    assert!(most_share <= 0.15 + 1e-12);
    assert!(r >= 20.0 * q);
}

#[test]
fn the_law_s_edges_rest_the_cap_and_the_bands() {
    let profile = Profile::default();
    // None at rest, nor with no depth.
    assert_eq!(profile.chop_heights(0.5, 0.0, MANNING), [0.0; 2]);
    assert_eq!(profile.chop_heights(0.0, 1.0, MANNING), [0.0; 2]);
    // All in the short band at 0.15 m, all in the long at 1 m, and the variance shared by the
    // logarithm of the depth between.
    for speed in [0.3, 1.0, 3.0] {
        let at = |d: f64| profile.chop_heights(d, speed, MANNING);
        assert_eq!(at(SHALLOW)[1], 0.0);
        assert_eq!(at(DEEP)[0], 0.0);
        for d in [0.05, 0.15, 0.3, 0.5, 1.0, 2.0] {
            let (got, want) = (at(d), expected(d, speed));
            for b in 0..2 {
                assert!((got[b] - want[b]).abs() <= 1e-12 * want[0].hypot(want[1]).max(1e-12));
            }
        }
    }
    // Over the cap it holds at the depth limit: four times the rms is 0.6 of the depth.
    let fast = profile.chop_heights(0.2, 6.0, MANNING);
    let eta = fast[0].hypot(fast[1]);
    println!(
        "a 0.2 m chute at 6 m/s: chop {:.1} mm rms, {:.3} of the depth",
        eta * 1e3,
        eta / 0.2
    );
    assert!((4.0 * eta / 0.2 - 0.6).abs() < 1e-12);
    // The gains are the heights over each band's own rms.
    let g = profile.chop_gains(0.4, 1.2, MANNING);
    let h = profile.chop_heights(0.4, 1.2, MANNING);
    assert_eq!(g, [h[0] / profile.band_rms[0], h[1] / profile.band_rms[1]]);
}

#[test]
fn each_sample_of_a_river_file_carries_its_own_gains() {
    let (grid, declaration) = support::section::short_reach();
    let profile = Profile {
        threads: 1,
        ..Profile::default()
    };
    let flows = vec![vec![0.5 * DISCHARGE], vec![DISCHARGE]];
    let s = solve_samples(&grid, &declaration, &profile, &flows).unwrap();
    let key = key_for_samples(&grid, &declaration, &profile, &flows);
    let bytes = write(&s.file(key).unwrap());
    let file = RiverFile::read(&bytes, &key).unwrap();
    for (k, sample) in s.solutions.iter().enumerate() {
        let mut compared = 0;
        for r in &file.reaches {
            let gains = if k == 0 {
                &r.gains
            } else {
                &r.upper[k - 1].gains
            };
            for (i, &in_footprint) in r.footprint.iter().enumerate() {
                if in_footprint && sample.wet[i] {
                    assert_eq!(gains[i], sample.gains[i], "sample {k}, cell {i}");
                    compared += 1;
                }
            }
        }
        assert!(compared > 0, "sample {k} has no wet footprint cells");
        println!("sample {k}: {compared} wet footprint cells compared");
    }
    // A channel cell midway along the slope, away from the spring and the pool.
    let slope_cell = grid.cell([4.25, 0.0]).unwrap();
    assert!(file.reaches.iter().any(|r| r.footprint[slope_cell]));
    let slope_gain = |k: usize| {
        let sample = &s.solutions[k];
        assert!(sample.wet[slope_cell]);
        let [short, long] = sample.gains[slope_cell];
        short.hypot(long)
    };
    let (half, whole) = (slope_gain(0), slope_gain(1));
    println!("slope gain {half:.6} at half the discharge and {whole:.6} at the whole");
    assert!(half > 0.0 && whole > half);
}
