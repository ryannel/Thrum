//! An independently interpolated 80 m / nine-height band, held to fixed ceilings.
use crate::ground_support;
use crate::support;
use thrum_cloud_model::{Density, Fidelity, Light, Sample, Weather};

#[test]
fn placed_sun_paths_erode_column_forms_with_and_without_ground() {
    let shape = support::shape();
    let surface = ground_support::flat(0.0);
    let mut row = ground_support::row();
    row.depth = 850.0;
    let ground = ground_support::table(vec![row]);
    for erosion in [0.0, 0.4, 1.0] {
        let field = support::field(5, erosion);
        for envelope in [0.35, 1.0] {
            let mut state = support::state(0.0, 0.0);
            let mut directed = support::directed(&shape, 1, [0.0; 2]);
            directed.target = envelope;
            state.add(directed).expect("placed shape");
            state.step(1, 10.0).expect("form shape");
            ground_support::controls(&mut state, vec![1.0], 0.0);
            for resident in [false, true] {
                let volume = |_| resident.then_some(&shape);
                for with_ground in [false, true] {
                    let weather = Weather {
                        field: &field,
                        surface: with_ground.then_some(&surface),
                        ground: with_ground.then_some(&ground),
                    };
                    for elevation in [4.0_f64, 10.0, 30.0, 89.9] {
                        for density in [Density::Full, Density::Coarse, Density::Uneroded] {
                            let e = elevation.to_radians();
                            let sample = Sample {
                                tick: 1,
                                origin: [0.0, 650.0, 3.0],
                                direction: [e.cos(), e.sin(), 0.0],
                                length: 250.0 / e.sin(),
                                steps: 4000,
                            };
                            let cloud = field.optical_depth(&sample, &state, density, volume, 1.0);
                            let step = sample.length / f64::from(sample.steps);
                            let fog: f64 = (0..sample.steps)
                                .map(|i| {
                                    let distance = (f64::from(i) + 0.5) * step;
                                    let p = std::array::from_fn(|j| {
                                        sample.origin[j] + sample.direction[j] * distance
                                    });
                                    if with_ground {
                                        ground.density(&surface, &field, p, &state, density) * step
                                    } else {
                                        0.0
                                    }
                                })
                                .sum();
                            let actual = weather
                                .sun_path(
                                    sample.origin,
                                    &state,
                                    elevation,
                                    0.0,
                                    2.0,
                                    900.0,
                                    sample.steps,
                                    density,
                                    volume,
                                )
                                .expect("placed Sun reference");
                            support::close(actual.optical_depth.value, cloud + fog, 1e-11);
                            assert!(!actual.surface_blocked);
                            if elevation > 89.0 {
                                if matches!(density, Density::Uneroded) {
                                    assert!(cloud > 0.0);
                                }
                                if with_ground {
                                    assert!(fog > 0.0);
                                }
                                assert_eq!(
                                    actual.optical_depth.fidelity,
                                    if resident {
                                        Fidelity::Exact
                                    } else {
                                        Fidelity::Footprint
                                    }
                                );
                                println!(
                                    "erosion {erosion}, envelope {envelope}, resident {resident}, ground {with_ground}, {density:?}: Sun tau {}, independent tau {}",
                                    actual.optical_depth.value,
                                    cloud + fog
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn placed_density_and_both_medium_entries_share_column_erosion() {
    let shape = support::shape();
    let surface = ground_support::flat(0.0);
    let mut row = ground_support::row();
    row.depth = 850.0;
    let ground = ground_support::table(vec![row]);
    let light = Light {
        phases: [0.1, 0.2],
        sun_depths: [2.0, 4.0],
        forward_share: 0.7,
        broad_weight: 0.5,
        plane_cosines: [0.2, 0.3],
        vertical_depths: [1.0, 3.0],
        ambient: [0.2, 0.1],
        mean_cosine: 0.8,
    };
    for erosion in [0.0, 0.4, 1.0] {
        let field = support::field(5, erosion);
        let mut state = support::formed(&shape);
        ground_support::controls(&mut state, vec![1.0], 0.0);
        for resident in [false, true] {
            let volume = |_| resident.then_some(&shape);
            for with_ground in [false, true] {
                let weather = Weather {
                    field: &field,
                    surface: with_ground.then_some(&surface),
                    ground: with_ground.then_some(&ground),
                };
                for x in -6..=6 {
                    for y in 0..12 {
                        let p = [f64::from(x) * 8.0, 700.0 + f64::from(y) * 10.0, 3.0];
                        for density in [Density::Full, Density::Coarse, Density::Uneroded] {
                            let cloud = field.density(p, &state, density, volume, 1.0);
                            let fog = if with_ground {
                                ground.density(&surface, &field, p, &state, density)
                            } else {
                                0.0
                            };
                            assert_eq!(weather.density(p, &state, density, volume), cloud + fog);
                        }
                        let a = weather.medium(p, &state, volume, light);
                        let b = weather.medium_sources(p, &state, volume, light, |_| light);
                        support::close(a.extinction, b.extinction, 1e-15);
                        support::close(a.solar_kernel, b.solar_kernel, 1e-15);
                        support::close(a.ambient_source, b.ambient_source, 1e-15);
                        let expected =
                            light.coefficients(weather.density(p, &state, Density::Coarse, volume));
                        assert_eq!(a.extinction, expected.extinction);
                        assert_eq!(a.solar_kernel, expected.solar_kernel);
                        assert_eq!(a.ambient_source, expected.ambient_source);
                    }
                }
            }
        }
    }
}

#[test]
fn band_tracks_slant_paths_and_rejects_wrong_substitutes() {
    let field = support::field(2, 0.0);
    let surface = ground_support::flat(0.0);
    let mut row = ground_support::row();
    row.depth = 78.0;
    row.softness = 42.0;
    row.liquid_water = (3.912 / 150.0) * 2.0 * 1000.0 * row.droplet_radius / 3.0;
    row.drifting = true;
    row.centre = [1500.0, 0.0];
    row.radius = [1700.0, 20_000.0];
    let ground = ground_support::table(vec![row]);
    let mut state = support::state(0.0, 0.0);
    ground_support::controls(&mut state, vec![1.0], 0.0);
    let w = Weather {
        field: &field,
        surface: Some(&surface),
        ground: Some(&ground),
    };
    let mut rejected_twice = 0;
    for elevation in [30.0_f64, 10.0, 4.0] {
        let reference = |x: f64, y: f64| {
            w.sun_path(
                [x, y, 0.0],
                &state,
                elevation,
                0.0,
                2.0,
                78.0,
                600,
                Density::Coarse,
                |_| None,
            )
            .expect("Sun reference")
            .optical_depth
            .value
        };
        let grid: Vec<Vec<_>> = (0..=75)
            .map(|i| {
                (0..9)
                    .map(|k| reference(-1500.0 + 80.0 * f64::from(i), 78.0 * f64::from(k) / 8.0))
                    .collect()
            })
            .collect();
        let band = |x: f64, y: f64| {
            let fx = (x + 1500.0) / 80.0;
            let fy = y / 78.0 * 8.0;
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "probes are inside grid"
            )]
            let (i, k) = ((fx.floor() as usize).min(74), (fy.floor() as usize).min(7));
            let (u, v) = (fx - i as f64, fy - k as f64);
            (grid[i][k] * (1.0 - u) + grid[i + 1][k] * u) * (1.0 - v)
                + (grid[i][k + 1] * (1.0 - u) + grid[i + 1][k + 1] * u) * v
        };
        let mut worst = 0.0_f64;
        let mut square = 0.0;
        let mut wrong_level = 0;
        let mut wrong_twice = 0;
        let mut far_side_wrong = 0;
        for j in 0..400_u32 {
            let h = |seed: u32| {
                let mut v = j.wrapping_mul(0x9e37_79b1).wrapping_add(seed);
                v ^= v >> 16;
                v = v.wrapping_mul(0x7feb_352d);
                f64::from(v) / 4_294_967_296.0
            };
            let x = -1400.0 + 5800.0 * h(7);
            let y = if j % 2 == 0 { 60.0 * h(91) } else { 0.0 };
            let expected = (-reference(x, y)).exp();
            let product = (-band(x, y)).exp();
            let mut lo = f64::INFINITY;
            let mut hi = 0.0_f64;
            for dx in [-80.0, -40.0, 0.0, 40.0, 80.0] {
                for dy in [-9.75, 0.0, 9.75] {
                    let t = (-reference(x + dx, (y + dy).clamp(0.0, 78.0))).exp();
                    lo = lo.min(t);
                    hi = hi.max(t);
                }
            }
            // Design ceiling, unchanged, with one node along each grid axis.
            let inside = |v: f64| v >= lo - (0.05 * lo + 0.005) && v <= hi + (0.05 * hi + 0.005);
            assert!(
                inside(product),
                "Sun {elevation}, ({x},{y}): {product}, interval {lo}..{hi}"
            );
            let vertical = w
                .optical_depth(
                    [x, y, 0.0],
                    [x, 78.0, 0.0],
                    &state,
                    600,
                    Density::Coarse,
                    |_| None,
                )
                .expect("vertical")
                .value;
            let level = (-vertical / elevation.to_radians().sin()).exp();
            if !inside(level) {
                wrong_level += 1;
                if x < -200.0 && y == 0.0 {
                    far_side_wrong += 1;
                }
            }
            if !inside((-2.0 * band(x, y)).exp()) {
                wrong_twice += 1;
            }
            let error = (product - expected).abs();
            worst = worst.max(error);
            square += error * error;
        }
        println!(
            "Sun {elevation} degrees: RMS {:.6}, worst {worst:.6}; wrong level {wrong_level}, far-side ground {far_side_wrong}, twice {wrong_twice}; design proof RMS 0.006 to 0.011",
            (square / 400.0).sqrt()
        );
        if elevation <= 10.0 {
            assert!(wrong_level > 0 && far_side_wrong > 0);
        }
        rejected_twice += wrong_twice;
    }
    assert!(rejected_twice > 0);
}

#[test]
fn surface_occlusion_is_separate_and_clouds_share_the_path() {
    let field = support::field(2, 0.0);
    let surface = ground_support::flat(800.0);
    let ground = ground_support::table(vec![ground_support::row()]);
    let state = support::state(1.0, 1.0);
    let w = Weather {
        field: &field,
        surface: Some(&surface),
        ground: Some(&ground),
    };
    for density in [Density::Full, Density::Coarse] {
        let a = w
            .sun_path(
                [0.0, 790.0, 0.0],
                &state,
                10.0,
                0.0,
                2.0,
                1200.0,
                1000,
                density,
                |_| None,
            )
            .expect("cloud ray");
        assert!(a.surface_blocked);
        assert!(a.optical_depth.value > 0.0);
        let b = Weather {
            field: &field,
            surface: None,
            ground: None,
        }
        .sun_path(
            [0.0, 790.0, 0.0],
            &state,
            10.0,
            0.0,
            2.0,
            1200.0,
            1000,
            density,
            |_| None,
        )
        .expect("without terrain");
        assert!(!b.surface_blocked);
        assert_eq!(a.optical_depth.value, b.optical_depth.value);
    }
}
