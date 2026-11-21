//! Optional ground inputs preserve existing calls, not stored expected numbers.
use crate::ground_support;
use crate::support;
use thrum_cloud_model::{Density, Weather};

#[test]
fn absent_and_zero_are_bit_identical_for_cells_deck_and_placed() {
    let field = support::field(17, 0.4);
    let surface = ground_support::shore([0.0; 2]);
    let ground = ground_support::table(vec![ground_support::row(); 2]);
    let shape = support::shape();
    for kind in [0.0, 1.0] {
        let mut state = support::state(0.9, kind);
        state
            .add(support::directed(&shape, 1, [0.0; 2]))
            .expect("shape");
        state.step(1, 500.0).expect("step");
        ground_support::controls(&mut state, vec![0.0; 2], 0.0);
        for (s, g) in [
            (None, None),
            (Some(&surface), None),
            (None, Some(&ground)),
            (Some(&surface), Some(&ground)),
        ] {
            let weather = Weather {
                field: &field,
                surface: s,
                ground: g,
            };
            for x in -5..=5 {
                for y in [0.0, 20.0, 710.0, 760.0, 1000.0] {
                    let p = [f64::from(x) * 30.0, y, 0.0];
                    assert_eq!(weather.column(p, &state), field.query(p, &state));
                    for density in [Density::Full, Density::Coarse, Density::Uneroded] {
                        assert_eq!(
                            weather
                                .density(p, &state, density, |_| Some(&shape))
                                .to_bits(),
                            field
                                .density(p, &state, density, |_| Some(&shape), 1.0)
                                .to_bits()
                        );
                        let end = [p[0] + 300.0, 2000.0, p[2]];
                        let tau = weather
                            .optical_depth(p, end, &state, 200, density, |_| Some(&shape))
                            .expect("path");
                        let length = 300.0_f64.hypot(2000.0 - y);
                        let expected: f64 = (0..200)
                            .map(|n| {
                                let t = (f64::from(n) + 0.5) / 200.0;
                                let q = [p[0] + 300.0 * t, y + (2000.0 - y) * t, 0.0];
                                field.density(q, &state, density, |_| Some(&shape), 1.0) * length
                                    / 200.0
                            })
                            .sum();
                        assert_eq!(tau.value.to_bits(), expected.to_bits());
                        let old = Weather {
                            field: &field,
                            surface: None,
                            ground: None,
                        };
                        assert_eq!(
                            weather
                                .sun_path(
                                    p,
                                    &state,
                                    10.0,
                                    0.0,
                                    2.0,
                                    2000.0,
                                    200,
                                    density,
                                    |_| Some(&shape)
                                )
                                .expect("sun")
                                .optical_depth
                                .value
                                .to_bits(),
                            old.sun_path(
                                p,
                                &state,
                                10.0,
                                0.0,
                                2.0,
                                2000.0,
                                200,
                                density,
                                |_| Some(&shape)
                            )
                            .expect("sun")
                            .optical_depth
                            .value
                            .to_bits()
                        );
                    }
                }
            }
        }
    }
}
