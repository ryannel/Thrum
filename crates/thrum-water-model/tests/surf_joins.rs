//! M1's source and mouth seams, with the complete surf capability enabled.
use crate::support;
use support::fixtures::{field, profile};
use support::joined::declarations;
use thrum_water_model::{Map, SurfProfile};

#[test]
fn transformed_and_shaped_surface_meets_at_both_ends_and_stage_extremes() {
    let mut field = field(0.5);
    field.layers[1] = thrum_sea_model::LongWaves::new(
        vec![thrum_sea_model::Component {
            wave_vector: [1.0, 0.0],
            amplitude: 0.04,
            phase: 0.3,
        }],
        0.0,
    )
    .unwrap();
    for source in [-1.0, 1.0] {
        for tide in [-1.0, 1.0] {
            for depth in [0.25, 4.0] {
                let (bed, bodies) = declarations(source, tide, depth);
                let mut map = Map::prepare(bed, bodies, profile()).unwrap();
                map.enable_surf(SurfProfile::default()).unwrap();
                for x in [20.0, 44.0] {
                    for z in [9.3, 10.0, 10.7] {
                        for time in [0.0, 0.7, 2.3] {
                            let left = map.surface(&field, [x - 1e-7, z], time).unwrap();
                            let right = map.surface(&field, [x + 1e-7, z], time).unwrap();
                            assert!(
                                (left.sample.height - right.sample.height).abs() < 1e-5,
                                "end {x}, depth {depth}: {left:?} {right:?}"
                            );
                            assert_eq!(left.wet, right.wet);
                            assert!((left.depth - right.depth).abs() < 1e-5);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn outlet_does_not_replace_the_lakes_own_wave_sources() {
    let mut field = field(0.5);
    field.layers[0] =
        thrum_sea_model::LongWaves::new(field.layers[0].components().to_vec(), 0.5).unwrap();
    for length in [5.0, 100.0] {
        let (bed, mut bodies) = declarations(0.0, 0.0, 4.0);
        let thrum_water_model::BodyKind::Chart(chart) = &mut bodies[2].kind else {
            unreachable!("chart fixture");
        };
        for end in &mut chart.ends {
            end.length = length;
        }
        bodies[0].gains[0] = 0.5;
        bodies[0].fetch = 10.0;
        let mut map = Map::prepare(bed, bodies, profile()).unwrap();
        map.enable_surf(SurfProfile::default()).unwrap();
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for tick in 0..32 {
            let t = f64::from(tick) * 0.25;
            let height = map.surface(&field, [10.0, 10.0], t).unwrap().sample.height;
            lo = lo.min(height);
            hi = hi.max(height);
            for x in [20.0, 24.0, 25.0, 44.0] {
                let a = map.surface(&field, [x - 1e-7, 10.0], t).unwrap();
                let b = map.surface(&field, [x + 1e-7, 10.0], t).unwrap();
                assert!(
                    (a.sample.height - b.sample.height).abs() < 1e-5,
                    "nonzero lake waves step at {x}: {a:?} {b:?}"
                );
            }
        }
        assert!(
            hi - lo > 0.1,
            "a declared nonzero lake train became flat: {lo}..{hi}"
        );
    }
}
