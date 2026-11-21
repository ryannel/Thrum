//! Plane beach waterline from the independent regular-wave closed form.
use crate::support;
use support::fixtures::{body, close, field, grid, profile};
use thrum_water_model::{BodyKind, Map};

#[test]
fn waterline_runs_to_closed_form_extrema_and_stops_with_zero_ratio() {
    let f = field(0.5);
    for ratio in [0.0, 0.7] {
        let mut p = profile();
        p.swash_ratio = ratio;
        let mut b = body("beach", BodyKind::Sea, 0.0);
        b.gains = [1.0; 4];
        let map = Map::prepare(grid([101, 5], |x, _| (x - 50.0) * 0.1), vec![b], p).unwrap();
        // For h = c(s-bed) at the crest, h=bed gives bed=c*s/(1+c).
        // At the trough h=-c(s-bed), bed=-c*s/(1-c).
        let c = 0.6 / (2.0 * 2.0_f64.sqrt());
        let swash = (ratio * 2.0 * 2.0_f64.sqrt() * 0.5).min(p.margin_height);
        for sign in [-1.0, 1.0] {
            let expected = sign * c * swash / (1.0 + sign * c);
            let x = 50.0 + expected / 0.1;
            // Choose phase to put the regular wave's crest/trough at the expected root.
            let component = f.layers[0].components()[0];
            let phase = if sign > 0.0 {
                0.0
            } else {
                std::f64::consts::PI
            };
            let time = (component.wave_vector[0] * x - phase) / component.frequency();
            close(
                map.surface(&f, [x, 2.0], time).unwrap().sample.height,
                expected,
            );
            let mut lo = 45.0;
            let mut hi = 55.0;
            for _ in 0..60 {
                let mid = f64::midpoint(lo, hi);
                if map.surface(&f, [mid, 2.0], time).unwrap().wet {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            close((lo + hi) * 0.05 - 5.0, expected);
        }
        let margin = map.sample([55.0, 2.0]).unwrap();
        assert!(margin.depth < 0.0);
        if ratio > 0.0 {
            let mut seen_wet = false;
            let mut seen_dry = false;
            for t in 0..120 {
                let s = map.surface(&f, [50.5, 2.0], f64::from(t) * 0.1).unwrap();
                seen_wet |= s.wet;
                seen_dry |= !s.wet;
            }
            assert!(seen_wet && seen_dry);
        }
    }
}

#[test]
fn margin_never_descends_into_a_hollow_beyond_a_ridge() {
    let bed = grid([31, 5], |x, _| {
        if x <= 10.0 {
            (x - 5.0) * 0.1
        } else if x <= 15.0 {
            0.5 + (x - 10.0) * 0.1
        } else {
            1.0 - (x - 15.0) * 0.04
        }
    });
    let map = Map::prepare(bed, vec![body("shore", BodyKind::Sea, 0.0)], profile()).unwrap();
    assert!(map.sample([14.0, 2.0]).is_some());
    for z in [2.0, 2.3] {
        assert!(map.sample([14.9, z]).is_some());
        assert!(map.sample([15.0, z]).is_some());
        for x in [15.1, 15.5, 15.9, 16.0, 20.0] {
            assert!(
                map.sample([x, z]).is_none(),
                "the margin must not run downhill behind its ridge at {x}, {z}"
            );
        }
    }
}

#[test]
fn basin_margin_stops_at_the_crest_between_centres_too() {
    let bed = grid([31, 7], |x, z| {
        let along = if x < 2.0 {
            4.0 - 3.0 * x
        } else if x <= 15.0 {
            (x - 5.0) * 0.1
        } else {
            1.0 - (x - 15.0) * 0.04
        };
        along.max(((z - 3.0).abs() - 2.0) * 4.0)
    });
    let map = Map::prepare(
        bed,
        vec![body("hollow", BodyKind::Basin { seed: [3.0, 3.0] }, 0.0)],
        profile(),
    )
    .unwrap();
    for z in [3.0, 3.3] {
        assert!(map.sample([14.9, z]).is_some());
        for x in [15.1, 15.5, 15.9, 20.0] {
            assert!(
                map.sample([x, z]).is_none(),
                "downhill basin margin {x}, {z}"
            );
        }
    }
}
