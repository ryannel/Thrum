//! Simultaneous wavelength and stage blends, compared with fresh individual solves.
#[expect(
    dead_code,
    reason = "shared independent dispersion is used by shore_transform"
)]
#[path = "support/shore.rs"]
mod support;
use support::{bed, config};
use thrum_water_model::{Lag, ShoreTransform};

#[test]
fn both_blends_against_component_own_solve_and_nearest_negative_control() {
    for slope in [10.0, 30.0] {
        // Narrow alongshore dimension would let zero-lag side sources erase the
        // physical integral. Sample far from them, including the shallow shelf.
        let b = bed([0.0, -240.0], 2.0, [241, 241], |x, _| (x - 240.0) / slope);
        let transform = ShoreTransform::prepare(b.clone(), config()).unwrap();
        let mut worst = [0.0_f64; 2];
        let mut nearest = 0.0_f64;
        for r in 0..5 {
            for f in [0.0_f64, 0.3, 0.5, 0.7, 1.0] {
                let wavelength = 20.0 * 2.0_f64.powf(f64::from(r) + f);
                for q in 0..=40 {
                    let stage = f64::from(q) * 0.05;
                    let own = Lag::solve(&b, config(), wavelength, [1.0, 0.0], stage).unwrap();
                    let near = Lag::solve(
                        &b,
                        config(),
                        20.0 * 2.0_f64.powf((f64::from(r) + f).round()),
                        [1.0, 0.0],
                        (stage * 2.0).round() / 2.0,
                    )
                    .unwrap();
                    for d in 0..80 {
                        let depth = 0.5 + f64::from(d) * 0.05;
                        let x = 240.0 + (stage - depth) * slope;
                        let grad = |lag: &Lag| {
                            1.0 + (lag.sample(&b, [x + 2.0, 0.0]).0
                                - lag.sample(&b, [x - 2.0, 0.0]).0)
                                / 4.0
                        };
                        let expected = grad(&own);
                        let blend = 1.0
                            + (transform.sample(0, wavelength, stage, [x + 2.0, 0.0]).0
                                - transform.sample(0, wavelength, stage, [x - 2.0, 0.0]).0)
                                / 4.0;
                        let band = usize::from(depth >= 1.0 - 1e-9);
                        worst[band] = worst[band].max((blend / expected - 1.0).abs());
                        if band == 1 {
                            nearest = nearest.max((grad(&near) / expected - 1.0).abs());
                        }
                    }
                }
            }
        }
        eprintln!("slope {slope}: blend {worst:?}, nearest {nearest}");
        assert!(worst[0] < 0.13 && worst[1] < 0.07);
        assert!(nearest > 0.13);
        for stage in [0.5, 1.0, 1.5] {
            let a = transform
                .sample(0, 80.0 * 2.0_f64.sqrt(), stage - 1e-8, [240.0 - slope, 0.0])
                .0;
            let b = transform
                .sample(0, 80.0 * 2.0_f64.sqrt(), stage + 1e-8, [240.0 - slope, 0.0])
                .0;
            assert!((a - b).abs() < 1e-4);
        }
    }
}
