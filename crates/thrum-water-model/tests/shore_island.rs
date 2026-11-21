//! Closed land, arrival direction and independently traced sector exposure.
use crate::support;
use std::f64::consts::{FRAC_PI_4, TAU};
use support::shore::{bed, config};
use thrum_water_model::ShoreTransform;

fn ground(x: f64, z: f64) -> f64 {
    -40.0 + 46.0 * (-(x * x + z * z) / (2.0 * 150.0_f64.powi(2))).exp()
}

// Analytic circular outline above highest stage; no bed samples or model ray.
fn expected(p: [f64; 2]) -> f64 {
    let radius = (-2.0 * 150.0_f64.powi(2) * (40.0_f64 / 46.0).ln()).sqrt();
    (0..17)
        .filter(|&s| {
            let a = FRAC_PI_4 * (f64::from(s) / 16.0 - 0.5);
            let dot = p[0] * a.cos() + p[1] * a.sin();
            let perpendicular = p[0] * a.sin() - p[1] * a.cos();
            dot <= 0.0 || perpendicular.abs() >= radius
        })
        .count() as f64
        / 17.0
}

#[test]
fn waves_arrive_round_land_and_exposure_closes_far_downwind() {
    // 1,000 m along the wind, which the far-downwind point at 480 m needs, and 600 m across it,
    // for 60% of the cells of the 1,000 m square this replaced. The island is 160 m across at
    // the shore, and an exposure ray that leaves the strip cannot come back to it, so every
    // exposure is the square's; the worst arrival angle, 59.175 degrees, is the square's too.
    let b = bed([-500.0, -300.0], 4.0, [251, 151], ground);
    let mut c = config();
    c.stage.max = 0.0;
    let transform = ShoreTransform::prepare(b, c).unwrap();
    let mut worst = 0.0_f64;
    for d in [1.0_f64, 2.0, 3.9] {
        let r = (-2.0 * 150.0_f64.powi(2) * ((40.0 - d) / 46.0).ln()).sqrt();
        for az in 0..72 {
            let angle = TAU * f64::from(az) / 72.0;
            let p = [r * angle.cos(), r * angle.sin()];
            let exposure = transform.exposure(0, 0.0, p);
            assert!(
                (exposure - expected(p)).abs() < 0.09,
                "{p:?}: {exposure} vs {}",
                expected(p)
            );
            if exposure == 0.0 {
                continue;
            }
            let grad = std::array::from_fn::<_, 2, _>(|axis| {
                let mut a = p;
                a[axis] -= 2.0;
                let mut b = p;
                b[axis] += 2.0;
                f64::from(axis == 0)
                    + (transform.sample(0, 80.0, 0.0, b).0 - transform.sample(0, 80.0, 0.0, a).0)
                        / 4.0
            });
            let inward = -grad[0] * angle.cos() - grad[1] * angle.sin();
            let angle = (inward / grad[0].hypot(grad[1]))
                .clamp(-1.0, 1.0)
                .acos()
                .to_degrees();
            worst = worst.max(angle);
            assert!(angle < 75.0, "angle {angle} at {p:?}, exposure {exposure}");
        }
    }
    assert_eq!(transform.exposure(0, 0.0, [-100.0, 0.0]), 1.0);
    assert_eq!(transform.exposure(0, 0.0, [100.0, 0.0]), 0.0);
    assert!(transform.exposure(0, 0.0, [480.0, 0.0]) > 0.4);
    eprintln!("maximum angle off inward normal: {worst:.3} deg");
}
