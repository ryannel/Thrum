//! M2.5: a drop's rings follow from water's dispersion with surface tension, depth and
//! viscosity. The test integrates the response itself, with its own Bessel function (a
//! power series and Hankel's expansion, where the model uses rational approximations)
//! and its own wavenumbers, and takes the energy's closed form from its own algebra.
use std::f64::consts::PI;
use thrum_rain_model::{Raindrop, Ring, RingProfile, Water};

const G: f64 = 9.81;
const TEN: f64 = 7.28e-5;
const NU: f64 = 1.0e-6;
const RHO: f64 = 1000.0;

/// J0 by its power series below 12 and Hankel's asymptotic expansion above.
fn bessel_j0(x: f64) -> f64 {
    let x = x.abs();
    if x < 12.0 {
        let (mut term, mut sum) = (1.0, 1.0);
        let q = -0.25 * x * x;
        for m in 1..80 {
            term *= q / f64::from(m * m);
            sum += term;
            if term.abs() < 1e-17 * sum.abs().max(1e-30) {
                break;
            }
        }
        sum
    } else {
        // P and Q of order 0: sums over (4·0 - (2j-1)²) / (8x) products.
        let mu = 0.0;
        let (mut p, mut q, mut t) = (1.0, 0.0, 1.0);
        for k in 1..12 {
            let kf = f64::from(k);
            t *= (mu - (2.0 * kf - 1.0).powi(2)) / (kf * 8.0 * x);
            if k % 2 == 1 {
                q += if (k / 2) % 2 == 0 { t } else { -t };
            } else {
                p += if (k / 2) % 2 == 1 { -t } else { t };
            }
        }
        let chi = x - PI / 4.0;
        (2.0 / (PI * x)).sqrt() * (p * chi.cos() - q * chi.sin())
    }
}

fn omega(k: f64, d: f64) -> f64 {
    ((G * k + TEN * k.powi(3)) * (k * d).tanh()).sqrt()
}

fn damping(k: f64, d: f64) -> f64 {
    let x = 2.0 * k * d;
    let bed = if x < 50.0 {
        k * (NU * omega(k, d) / 2.0).sqrt() / x.sinh()
    } else {
        0.0
    };
    2.0 * NU * k * k + bed
}

/// The test's own ring: wavenumbers every 1 rad/m to 2,600.
fn height(r: f64, t: f64, d: f64, impulse: f64, a: f64) -> f64 {
    let mut s = 0.0;
    for i in 1..=2600 {
        let k = f64::from(i);
        let w = omega(k, d);
        s += impulse
            * (-0.5 * k * k * a * a).exp()
            * (k * (k * d).tanh() / w)
            * (w * t).sin()
            * (-damping(k, d) * t).exp()
            * bessel_j0(k * r)
            * k;
    }
    -s / (2.0 * PI * RHO)
}

fn three_mm() -> (f64, f64, f64) {
    let d = 3.0e-3;
    let mass = RHO * PI / 6.0 * d * d * d;
    let speed = 9.65 - 10.3 * (-0.6_f64 * 3.0).exp();
    (mass, speed, 3.0 * 0.5 * d)
}

#[test]
fn the_test_bessel_function_is_right() {
    // Zeros and a value from tables.
    for (x, v) in [
        (2.404_825_557_7, 0.0),
        (5.520_078_110_3, 0.0),
        (1.0, 0.765_197_686_6),
        (30.634_606_468, 0.0),
    ] {
        assert!(
            (bessel_j0(x) - v).abs() < 1e-7,
            "J0({x}) = {}",
            bessel_j0(x)
        );
    }
}

#[test]
fn the_least_group_speed_is_water_s() {
    let (model, k) = Water::default().least_group_speed(f64::INFINITY);
    // The test's own: the group speed by a central difference, scanned finely.
    let group = |k: f64| (omega(k + 1e-3, 1e3) - omega(k - 1e-3, 1e3)) / 2e-3;
    let own = (200..2000)
        .map(|i| group(f64::from(i) * 0.25))
        .fold(f64::INFINITY, f64::min);
    println!("least group speed: model {model:.5} m/s at k {k:.1}, test {own:.5}");
    assert!((model - 0.178).abs() <= 0.003, "{model}");
    assert!((model - own).abs() < 1e-4);
}

#[test]
fn a_drop_s_rings_carry_the_closed_form_s_energy() {
    let (mass, speed, a) = three_mm();
    let p = mass * speed;
    // A Gaussian impulse P exp(-r²/2a²)/(2πa²) on deep water starts the potential
    // -p/ρ; its kinetic energy, by Parseval, is P²/(4πρ) ∫ k² exp(-k²a²) dk, and that
    // integral is √π/(4a³).
    let closed = p * p / (16.0 * PI.sqrt() * RHO * a.powi(3));
    let drop = Raindrop { diameter: 3.0e-3 };
    let ring = Ring::of(Water::default(), drop, 3.0, f64::INFINITY);
    let model = ring.energy();
    let share = model / (0.5 * mass * speed * speed);
    println!(
        "energy: model {model:e} J, closed form {closed:e} J; {:.2}% of the drop's",
        100.0 * share
    );
    assert!((model / closed - 1.0).abs() < 0.02);
    assert!((0.008..=0.014).contains(&share), "{share}");
    assert!((ring.impulse - p).abs() < 1e-15);
}

#[test]
fn rings_at_0_8_s_leave_calm_water_inside_and_shallow_water_lowers_them() {
    let (mass, speed, a) = three_mm();
    let p = mass * speed;
    let water = Water::default();
    let ring = Ring::of(water, Raindrop { diameter: 3.0e-3 }, 3.0, f64::INFINITY);
    let radii: Vec<f64> = (1..90).map(|i| 0.004 * f64::from(i)).collect();
    let own: Vec<f64> = radii.iter().map(|&r| height(r, 0.8, 10.0, p, a)).collect();
    let model: Vec<f64> = radii.iter().map(|&r| ring.height(r, 0.8)).collect();
    let peak = own.iter().fold(0.0_f64, |m, h| m.max(h.abs()));
    let model_peak = model.iter().fold(0.0_f64, |m, h| m.max(h.abs()));
    println!(
        "highest ring at 0.8 s: model {:.4} mm, test {:.4} mm",
        model_peak * 1e3,
        peak * 1e3
    );
    assert!((peak - 0.14e-3).abs() <= 0.2 * 0.14e-3, "{peak}");
    assert!((model_peak - peak).abs() <= 0.02 * peak);
    for (i, &r) in radii.iter().enumerate() {
        assert!(
            (model[i] - own[i]).abs() <= 0.02 * peak,
            "at {r} m: {} against {}",
            model[i],
            own[i]
        );
        if r <= 0.11 {
            assert!(
                model[i].abs() < 0.1 * peak,
                "calm water at {r} m: {}",
                model[i]
            );
        }
    }
    // Depth: the highest ring at 0.4 s on 5, 2 and 1 mm over that on deep water.
    let highest = |d: f64| {
        (1..90)
            .map(|i| height(0.004 * f64::from(i), 0.4, d, p, a).abs())
            .fold(0.0_f64, f64::max)
    };
    let deep = highest(10.0);
    let profile = RingProfile::default();
    let table = profile.table();
    for (d, want) in [(0.005, 0.76), (0.002, 0.41), (0.001, 0.25)] {
        let own = highest(d) / deep;
        let model = profile.response(d);
        let read = table.response(d);
        println!(
            "depth {} mm: test {own:.3}, model {model:.3}, table {read:.3}",
            d * 1e3
        );
        assert!(
            (own - want).abs() <= 0.05,
            "test's own ratio {own} at {d} m"
        );
        assert!((model - own).abs() <= 0.02 && (read - model).abs() < 1e-12);
    }
    assert_eq!(table.response(0.0), 0.0);
    assert_eq!(table.response(0.05), 1.0);
}

#[test]
fn the_table_holds_each_class_s_ring_as_the_model_integrates_it() {
    let profile = RingProfile::default();
    let table = profile.table();
    let water = Water::default();
    // The middle class's representative drop, recovered from its height at one point is
    // not needed: check the table against a direct sum for a drop of each class's mean
    // diameter at 10 mm/h, which the profile names.
    for (c, &[lo, hi]) in profile.classes.iter().enumerate() {
        let (mut n, mut sum) = (0.0, 0.0);
        let mut d = lo + 0.025;
        while d < hi {
            let m = thrum_rain_model::marshall_palmer(10.0, d) * 0.05;
            n += m;
            sum += m * d;
            d += 0.05;
        }
        let ring = Ring::of(
            water,
            Raindrop {
                diameter: sum / n / 1000.0,
            },
            3.0,
            f64::INFINITY,
        );
        for (r, t) in [(0.03, 0.2), (0.08, 0.4), (0.15, 0.8)] {
            let direct = ring.height(r, t);
            let [read, _, _] = table.ring(c, r, t);
            let scale = ring.highest(t, 0.3, 0.004);
            assert!(
                (read - direct).abs() < 0.03 * scale,
                "class {c} at {r} m, {t} s: {read} against {direct}"
            );
        }
    }
    let shares: f64 = table.classes.iter().map(|c| c.share).sum();
    assert!((shares - 1.0).abs() < 1e-12);
}
