//! M1's source and mouth seams, with the complete surf capability enabled.
#[path = "support/joined.rs"]
mod fixture;
#[expect(dead_code, reason = "shared constructors serve map tests")]
#[path = "support/fixtures.rs"]
mod support;
use fixture::declarations;
use support::{field, profile};
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
