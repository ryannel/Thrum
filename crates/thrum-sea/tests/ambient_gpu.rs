//! Metal ambient field conformance.
use crate::support;

// The full grid at each clock age is one test, so the runner spreads them over the cores. The
// frame script this test also ran is `frames_gpu`'s test, which ran the same function.
#[test]
fn metal_ambient_matches_independent_model_early() {
    support::conformance(false, support::Age::Early, "ambient");
}

#[test]
fn metal_ambient_matches_independent_model_a_day_in() {
    support::conformance(false, support::Age::Day, "ambient");
}

#[test]
fn compressed_crests_converge_on_native_and_portable_profiles() {
    use thrum_sea_model::{Cascade, Component, LongWaves, Spectrum};
    for portable in [false, true] {
        let Some(mut h) = support::Harness::with_profile(
            portable,
            thrum_sea::Profile {
                choppiness: 4.0,
                ..Default::default()
            },
        ) else {
            return;
        };
        let c = Component {
            wave_vector: [0.25, 0.0],
            amplitude: 0.89,
            phase: -0.25 * support::ANCHOR[0],
        };
        let s = Spectrum {
            long: vec![c],
            cascades: std::array::from_fn(|i| {
                Cascade::new([160.0, 37.0, 9.0][i], &[]).expect("empty band")
            }),
            tail_slope_variance: 0.0,
        };
        let points: Vec<_> = (-100..=100)
            .map(|i| {
                [
                    support::ANCHOR[0] + f64::from(i) * 0.002,
                    support::ANCHOR[1],
                ]
            })
            .collect();
        for time in [0.0, 86_400.006_25] {
            let out = h.frame(&s, time, &points, true).expect("submitted");
            support::check_long(&mut h, &out, &s, time, &points, false);
        }
        let mut crossing = s.clone();
        crossing.long = vec![
            Component {
                amplitude: 0.6,
                ..c
            },
            Component {
                wave_vector: [0.0, 0.25],
                amplitude: 0.6,
                phase: -0.25 * support::ANCHOR[1],
            },
        ];
        let out = h
            .frame(&crossing, 0.0, &points, true)
            .expect("crossing waves");
        support::check_long(&mut h, &out, &crossing, 0.0, &points, false);
        let mut refused = s.clone();
        refused.long[0].amplitude = 0.9;
        let mut encoder = h
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        assert!(matches!(
            h.sea
                .upload(
                    &h.queue,
                    &mut encoder,
                    &refused,
                    0.0,
                    support::ANCHOR,
                    &points
                )
                .expect_err("the contraction boundary stays excluded"),
            thrum_sea::AmbientError::NonContractive
        ));
        // The previous 24-iteration cap visibly misstates the slope near this
        // compressed crest. This is an independent scalar negative control.
        let point = 0.05;
        let mut offset = 0.0;
        for _ in 0..24 {
            offset = 4.0 * c.amplitude * (0.25_f64 * (point + offset)).sin();
        }
        let phase = 0.25 * (point + offset);
        let old_slope =
            -0.25 * c.amplitude * phase.sin() / (1.0 - 4.0 * 0.25 * c.amplitude * phase.cos());
        let oracle = LongWaves::new(vec![c], 4.0).expect("contractive");
        let expected = oracle
            .evaluate([support::ANCHOR[0] + point, support::ANCHOR[1]], 0.0)
            .expect("converged");
        assert!((old_slope - expected.slope[0]).abs() > 0.002);
        h.save(if portable {
            "compressed-portable"
        } else {
            "compressed-native"
        });
    }
}
