//! Validate every index of the table, and retain M1's swash closed form.
use crate::support;
use std::f64::consts::TAU;
use support::fixtures::{body, field, grid, profile};
use thrum_water_model::{BodyKind, BreakerSection, Map, SectionPoint, SurfProfile};

#[test]
fn rejects_departure_at_zero_nonmonotone_throw_and_throwing_bores() {
    let row = || {
        (0..32)
            .map(|i| SectionPoint {
                height: (TAU * f64::from(i) / 32.0).cos(),
                throw: 0.0,
            })
            .collect::<Vec<_>>()
    };
    let good = vec![row(), row(), row()];
    assert!(BreakerSection::new(vec![0.0, 1.0, 2.0], good.clone(), 2.0).is_ok());
    let mut bad = good.clone();
    bad[0][0].height += 0.1;
    assert!(BreakerSection::new(vec![0.0, 1.0, 2.0], bad, 2.0).is_err());
    let mut bad = good.clone();
    bad[1][16].throw = 2.0;
    assert!(BreakerSection::new(vec![0.0, 1.0, 2.0], bad, 2.0).is_err());
    let mut bad = good.clone();
    bad[2][0].throw = 1.0;
    assert!(BreakerSection::new(vec![0.0, 1.0, 2.0], bad, 2.0).is_err());
    let mut bad = good;
    bad[2][16].height = -0.9;
    assert!(BreakerSection::new(vec![0.0, 1.0, 2.0], bad, 2.0).is_err());
}

fn crosses(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
    let side = |a: [f64; 2], b: [f64; 2], c: [f64; 2]| {
        (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    };
    side(a, b, c) * side(a, b, d) < -1e-12 && side(c, d, a) * side(c, d, b) < -1e-12
}

#[test]
fn every_index_is_a_noncrossing_curve_and_the_first_profile_overhangs() {
    let section = BreakerSection::first();
    let mut overhang = false;
    for q in 0..=200 {
        let index = f64::from(q) / 100.0;
        let curve: Vec<_> = (0..=512)
            .map(|i| {
                let phase = TAU * f64::from(i) / 512.0;
                let s = section.sample(phase, index);
                [phase * 25.0 / TAU + 0.7 * s.throw, 0.7 * s.height]
            })
            .collect();
        if (index - 1.0).abs() < 0.01 {
            overhang |= curve.windows(2).any(|w| w[1][0] < w[0][0]);
        }
        for (i, a) in curve.windows(2).enumerate() {
            for b in curve.windows(2).skip(i + 2) {
                assert!(!crosses(a[0], a[1], b[0], b[1]), "crossing at {index}");
            }
        }
    }
    assert!(overhang);
}

#[test]
fn surf_keeps_the_beach_waterline_closed_form() {
    let mut f = field(0.5);
    // A resolved short wave can change the supported height, but its
    // geometry must not move either crossing of the inner swash.
    let short_amplitude = 0.16_f64;
    f.layers[1] = thrum_sea_model::LongWaves::new(
        vec![thrum_sea_model::Component {
            wave_vector: [TAU / 4.0, 0.0],
            amplitude: short_amplitude,
            phase: 0.37,
        }],
        0.0,
    )
    .unwrap();
    let mut b = body("shore", BodyKind::Sea, 0.0);
    b.gains = [1.0; 4];
    let mut map =
        Map::prepare(grid([101, 5], |x, _| (x - 50.0) * 0.1), vec![b], profile()).unwrap();
    map.enable_surf(SurfProfile::default()).unwrap();
    let c = 0.6 / (2.0 * 2.0_f64.sqrt());
    let swash = (0.7 * 2.0 * 2.0_f64.sqrt() * short_amplitude.hypot(0.5)).min(1.5);
    for sign in [-1.0, 1.0] {
        let expected = sign * c * swash / (1.0 + sign * c);
        let x = 50.0 + expected / 0.1;
        let component = f.layers[0].components()[0];
        let (lag, _) = map
            .shore_transform(0)
            .unwrap()
            .sample(0, 60.0, 0.0, [50.0, 2.0]);
        let phase = if sign > 0.0 { 0.0 } else { TAU / 2.0 };
        let time = (component.wave_vector[0] * (50.0 + lag) - phase) / component.frequency();
        let surface = map.surface(&f, [x, 2.0], time).unwrap();
        assert!((surface.sample.height - expected).abs() < 1e-9);
        let (mut lo, mut hi) = (45.0, 55.0);
        for _ in 0..50 {
            let x = f64::midpoint(lo, hi);
            if map.surface(&f, [x, 2.0], time).unwrap().wet {
                lo = x;
            } else {
                hi = x;
            }
        }
        assert!(((lo + hi) * 0.05 - 5.0 - expected).abs() < 1e-9);
    }
}
