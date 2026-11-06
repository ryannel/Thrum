use super::*;
use transport::{Query, TraceOptions};

fn path(case: Case, medium: Medium, index: usize, options: TraceOptions) -> PathValue {
    let (camera, sun, view) = case.vectors().expect("declared finite reference fixture");
    let mut streams = random::Streams::new(0x7711_2200, index % 8, index / 8);
    transport::trace(
        camera,
        sun,
        view,
        medium,
        &mut streams,
        &mut Budget {
            remaining: EVENT_LIMIT,
        },
        options,
    )
    .expect("bounded finite transport query resolves")
}

#[test]
fn finite_interaction_query_preserves_each_order_with_roulette() {
    let case = Case::degrees(0, 60.0, 15.0, 90.0, 4000.0);
    let medium = Medium {
        rayleigh: 2e-4,
        ground_albedo: 0.0,
        ..vacuum()
    };
    let mut difference = [[0.0; 8]; 2];
    let mut baseline_fifth = 0.0;
    for index in 0..8192 {
        let baseline = path(
            case,
            medium,
            index,
            TraceOptions {
                query: Query::ThroughInteractions(5),
                roulette_survival: 1.0,
            },
        );
        baseline_fifth += baseline.orders[5] / 8192.0;
        for (comparison, survival) in [0.6, 0.8].into_iter().enumerate() {
            let sampled = path(
                case,
                medium,
                index,
                TraceOptions {
                    query: Query::ThroughInteractions(5),
                    roulette_survival: survival,
                },
            );
            assert!(sampled.interactions <= 5 && baseline.interactions <= 5);
            assert_eq!(
                &sampled.orders[..5],
                &baseline.orders[..5],
                "matched prefix through order4"
            );
            difference[comparison][index % 8] += (sampled.orders[5] - baseline.orders[5]) / 1024.0;
        }
    }
    assert!(
        baseline_fifth > 1e-5,
        "the fifth-order comparison must carry signal"
    );
    for (survival, difference) in [0.6, 0.8].into_iter().zip(difference) {
        let delta = summarize(difference);
        assert!(
            delta.mean.abs() <= 6.0 * delta.standard_error + baseline_fifth * 1e-4,
            "survival={survival}, fifth={baseline_fifth}, paired delta={delta:?}"
        );
    }
}

#[test]
fn shared_roulette_matches_independent_geometric_emission_series() {
    // Independent closed form: event n emits (1/2)^(n-1). This tests the exact
    // helper used by full transport and strongly rejects missing 1/p weighting.
    for survival in [0.6, 0.8, 1.0] {
        let mut batches = [[0.0; 9]; 8];
        for index in 0..8192 {
            let mut rng = random::Streams::new(0x1234, index % 8, index / 8);
            let mut weight = 1.0;
            for event in 1..=9 {
                batches[index % 8][event - 1] += weight / 1024.0;
                weight *= 0.5;
                if event >= 4 && !transport::roulette(&mut weight, survival, &mut rng.path) {
                    break;
                }
            }
        }
        let orders: [Estimate; 9] = std::array::from_fn(|order| {
            summarize(std::array::from_fn(|batch| batches[batch][order]))
        });
        for (order, estimate) in orders.into_iter().enumerate() {
            let expected = 0.5_f64.powi(i32::try_from(order).unwrap());
            assert!(
                (estimate.mean - expected).abs() <= 6.0 * estimate.standard_error + 1e-14,
                "p={survival}, order={}, expected={expected}, measured={estimate:?}",
                order + 1
            );
        }
    }
}

#[test]
fn cosine_sampling_and_exactly_one_ground_interaction_scale_linearly() {
    let normal = Vec3::new(0.3, 0.4, 0.5).unit();
    let tangent = normal.cross(Vec3::new(0.0, 1.0, 0.0)).unit();
    let mut rng = random::Rng::new(9876);
    let (mut mean, mut second, mut side) = (0.0, 0.0, 0.0);
    for _ in 0..16384 {
        let sample = random::cosine(normal, &mut rng);
        let mu = normal.dot(sample);
        assert!((0.0..=1.0 + 1e-14).contains(&mu));
        mean += mu / 16384.0;
        second += mu * mu / 16384.0;
        side += tangent.dot(sample) / 16384.0;
    }
    assert!((mean - 2.0 / 3.0).abs() < 0.01);
    assert!((second - 0.5).abs() < 0.01);
    assert!(side.abs() < 0.01);

    let case = Case::degrees(0, 45.0, -45.0, 0.0, 2000.0);
    let medium = Medium {
        ground_albedo: 0.125,
        ..Medium::band(0)
    };
    let brighter = Medium {
        ground_albedo: 0.25,
        ..medium
    };
    let options = TraceOptions {
        query: Query::ThroughInteractions(8),
        roulette_survival: 1.0,
    };
    let (mut vertex, mut subsequent_volume) = (0.0, 0.0);
    for index in 0..2048 {
        let a = path(case, medium, index, options);
        let b = path(case, brighter, index, options);
        assert_eq!(
            a.interactions, b.interactions,
            "rho cannot change this fixed sampling policy"
        );
        assert!(
            (b.one_ground_vertex - 2.0 * a.one_ground_vertex).abs() <= a.one_ground_vertex * 1e-12
        );
        assert!(
            (b.one_ground_volume - 2.0 * a.one_ground_volume).abs() <= a.one_ground_volume * 1e-12,
            "subsequent medium contributions require the continuation rho"
        );
        vertex += a.one_ground_vertex;
        subsequent_volume += a.one_ground_volume;
    }
    assert!(vertex > 1.0, "ground-vertex signal is exercised");
    assert!(
        subsequent_volume > 1e-3,
        "post-ground atmospheric signal is exercised"
    );
}

#[test]
fn adverse_twilight_radiance_refines_below_the_old_absolute_floor() {
    let refined = transport::SingleSettings {
        optical_depth: Settings {
            absolute: 1e-11,
            relative: 1e-10,
            ..Settings::default()
        },
        radiance: Settings {
            absolute: 1e-16,
            relative: 1e-10,
            ..Settings::default()
        },
        radiance_subdivisions: 4,
    };
    let mut faint_signal = false;
    for band in 0..3 {
        for (sun, view, azimuth) in [
            (-6.0, 1.0, 0.0),
            (-9.0, 15.0, 90.0),
            (-12.0, 1.0, 0.0),
            (-12.0, 15.0, 0.0),
            (-12.0, 15.0, 45.0),
        ] {
            let case = Case::degrees(band, sun, view, azimuth, 2.0);
            let normal = deterministic_single(case).unwrap();
            let (camera, sun_direction, view_direction) = case.vectors().unwrap();
            let tight = transport::single(
                camera,
                sun_direction,
                view_direction,
                Medium::band(band),
                refined,
            )
            .unwrap();
            // Agreement is independent of either integrator's own error
            // estimate: a loosened integration floor cannot widen this check.
            let agreement = 1e-16 + tight.volume.abs() * 1e-5;
            assert!(
                (normal.volume - tight.volume).abs() <= agreement,
                "{case:?}: normal={normal:?}, refined={tight:?}, agreement={agreement}"
            );
            faint_signal |= tight.volume > 0.0 && tight.volume < 1e-9;
            eprintln!(
                "twilight refinement band={band} sun={sun} view={view} az={azimuth}: normal={:.12e} refined={:.12e} outer_error={:.3e}",
                normal.volume, tight.volume, normal.estimated_error
            );
        }
    }
    assert!(
        faint_signal,
        "this must exercise faint nonzero transport, not only zero rows"
    );
}
