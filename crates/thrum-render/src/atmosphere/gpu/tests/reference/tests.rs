use super::*;
use geometry::{HEIGHT, RADIUS};
use quadrature::Settings;
use std::f64::consts::PI;

mod falsifiers;

fn vacuum() -> Medium {
    Medium {
        rayleigh: 0.0,
        mie_scattering: 0.0,
        mie_extinction: 0.0,
        ozone: 0.0,
        ground_albedo: 0.1,
    }
}

#[test]
fn frozen_rng_open_uniforms_and_separate_streams() {
    assert_eq!(random::uniform(0), 2.0_f64.powi(-53));
    assert_eq!(random::uniform(u64::MAX), 1.0 - 2.0_f64.powi(-53));
    let mut rng = random::Rng::new(0);
    assert_eq!(rng.next(), 0xe220_a839_7b1d_cdaf);
    assert_eq!(rng.next(), 0x6e78_9e6a_a1b9_65f4);
    let mut a = random::Streams::new(123, 0, 0);
    let mut b = random::Streams::new(123, 0, 0);
    for _ in 0..1000 {
        assert_eq!(a.path.next(), b.path.next());
        // Consuming shadow samples cannot change the geometry stream.
        let u = a.shadow.open();
        assert!(u > 0.0 && u < 1.0);
    }
    let mut batch = random::Streams::new(123, 1, 0);
    assert_ne!(a.path.next(), batch.path.next());
}

#[test]
fn phase_normalization_moments_and_backward_sampling_convention() {
    type Phase = fn(f64) -> f64;
    let phases: [(Phase, f64, f64); 2] = [(random::rayleigh, 0.0, 0.4), (random::hg, 0.8, 0.76)];
    for (phase, expected_mean, expected_second) in phases {
        for (power, expected) in [(0, 1.0), (1, expected_mean), (2, expected_second)] {
            let value = quadrature::integrate(&[-1.0, 1.0], Settings::default(), |mu| {
                Ok(2.0 * PI * phase(mu) * mu.powi(power))
            })
            .unwrap();
            assert!(
                (value.value - expected).abs() < 1e-8,
                "moment {power}: {}",
                value.value
            );
        }
    }
    assert!(
        random::hg(1.0) > random::hg(-1.0) * 700.0,
        "forward peak is along the backward ray axis"
    );
    let axis = Vec3::new(0.3, 0.4, 0.5).unit();
    for (mie, expected_mean, expected_second) in [(false, 0.0, 0.4), (true, 0.8, 0.76)] {
        let mut rng = random::Rng::new(456);
        let mut budget = Budget { remaining: 100_000 };
        let (mut mean, mut second) = (0.0, 0.0);
        for _ in 0..20_000 {
            let sample = random::phase(axis, mie, &mut rng, &mut budget).unwrap();
            assert!((sample.length() - 1.0).abs() < 1e-12);
            let mu = axis.dot(sample);
            mean += mu / 20_000.0;
            second += mu * mu / 20_000.0;
        }
        assert!((mean - expected_mean).abs() < 0.015, "sample mean {mean}");
        assert!(
            (second - expected_second).abs() < 0.015,
            "sample second moment {second}"
        );
    }
}

#[test]
fn sphere_visibility_shadow_splits_and_beer_lambert_are_independent() {
    let origin = Vec3::new(0.0, RADIUS + 2.0, 0.0);
    let up = Vec3::new(0.0, 1.0, 0.0);
    let down = up * (-1.0);
    let top = geometry::boundary(origin, up);
    assert!(!top.ground);
    assert!((top.distance - (HEIGHT - 2.0)).abs() < 1e-6);
    let floor = geometry::boundary(origin, down);
    assert!(floor.ground);
    assert!((floor.distance - 2.0).abs() < 1e-8);
    let tangent = Vec3::new(1.0, 0.0, 0.0);
    assert!(!geometry::boundary(origin, tangent).ground);
    assert!(geometry::boundary(origin, Vec3::new(1.0, -0.01, 0.0).unit()).ground);
    let points = geometry::splits(origin, up, top.distance, Some(tangent));
    assert!(points.windows(2).all(|pair| pair[1] > pair[0]));
    for height in [10_000.0, 25_000.0, 40_000.0] {
        assert!(
            points
                .iter()
                .any(|&distance| (distance - (height - 2.0)).abs() < 1e-6)
        );
    }
    let medium = Medium {
        rayleigh: 1e-5,
        ..vacuum()
    };
    let tau =
        transport::optical_depth(origin, up, top.distance, medium, Settings::default()).unwrap();
    let expected_tau = 1e-5 * 8000.0 * ((-2.0_f64 / 8000.0).exp() - (-HEIGHT / 8000.0).exp());
    assert!((tau - expected_tau).abs() < 1e-10);

    // Same ratio-tracking body as Sun connections, against analytic constant
    // Beer-Lambert attenuation and independently integrated optical depth.
    let sigma = 0.002;
    let length = 500.0;
    let integral =
        quadrature::integrate(&[0.0, length], Settings::default(), |_| Ok(sigma)).unwrap();
    assert!((integral.value - sigma * length).abs() < 1e-12);
    let expected = (-sigma * length).exp();
    let mut total = 0.0;
    for index in 0..8192 {
        let mut rng = random::Streams::new(12, index % 8, index / 8);
        total += transport::ratio_transmittance(
            length,
            0.004,
            |_| sigma,
            &mut rng.shadow,
            &mut Budget { remaining: 100_000 },
        )
        .unwrap()
            / 8192.0;
    }
    assert!(
        (total - expected).abs() < 0.012,
        "ratio {total} vs analytic {expected}"
    );
    let medium = Medium::band(1);
    let exact = transport::exact_sun(origin, up, medium, Settings::default()).unwrap();
    let mut sum = 0.0;
    for index in 0..4096 {
        let mut rng = random::Streams::new(22, index % 8, index / 8);
        sum += transport::sampled_sun(
            origin,
            up,
            medium,
            &mut rng.shadow,
            &mut Budget {
                remaining: EVENT_LIMIT,
            },
        )
        .unwrap()
            / 4096.0;
    }
    assert!(
        (sum - exact).abs() < 0.015,
        "Earth profile ratio {sum} vs adaptive {exact}"
    );
    let mut rng = random::Streams::new(1, 0, 0);
    assert_eq!(
        transport::sampled_sun(
            origin,
            down,
            medium,
            &mut rng.shadow,
            &mut Budget {
                remaining: EVENT_LIMIT
            }
        )
        .unwrap(),
        0.0
    );
}

fn check_stats(stats: &Stats) {
    let sum = stats.direct_ground.mean + stats.by_order.iter().map(|order| order.mean).sum::<f64>();
    assert!((sum - stats.radiance.mean).abs() < 1e-12);
    assert!((stats.atmosphere.mean + stats.ground.mean - stats.radiance.mean).abs() < 1e-12);
    assert!(stats.direct_volume.mean <= stats.by_order[0].mean + 1e-12);
    assert!(stats.ground_vertices.mean <= stats.ground.mean + 1e-12);
    assert!(stats.largest_path >= stats.radiance.mean);
    assert!(stats.largest_event <= stats.largest_path);
    assert!(stats.nonzero_paths <= stats.paths);
    assert!(
        stats
            .nonzero_by_order
            .iter()
            .all(|&count| count <= stats.nonzero_paths)
    );
    assert_eq!(
        stats.zero_batches,
        stats.batch_means.iter().filter(|&&v| v == 0.0).count()
    );
    for value in stats.by_order.iter().chain([
        &stats.radiance,
        &stats.direct_volume,
        &stats.direct_ground,
        &stats.ground_vertices,
        &stats.ground,
        &stats.atmosphere,
    ]) {
        assert!(value.mean.is_finite() && value.mean >= 0.0);
        assert!(value.standard_error.is_finite() && value.standard_error >= 0.0);
    }
}

#[test]
fn vacuum_lambert_ground_and_zero_batches_are_explicit() {
    let case = Case::degrees(0, 45.0, -90.0, 0.0, 2.0);
    let (camera, sun, view) = case.vectors().unwrap();
    let direct = transport::single(
        camera,
        sun,
        view,
        vacuum(),
        transport::SingleSettings::default(),
    )
    .unwrap();
    let expected = 0.1 / PI * 45.0_f64.to_radians().sin();
    assert_eq!(direct.volume, 0.0);
    assert!((direct.unscattered_ground - expected).abs() < 1e-14);
    assert_eq!(direct.radiance, direct.unscattered_ground);
    assert_eq!(direct.estimated_error, 0.0);
    let stats = estimate_medium(case, vacuum(), 64, 987, EVENT_LIMIT).unwrap();
    check_stats(&stats);
    assert!((stats.radiance.mean - expected).abs() < 1e-14);
    assert_eq!(stats.nonzero_paths, 64);
    assert_eq!(stats.nonzero_by_order, [64, 0, 0, 0, 0, 0]);
    assert_eq!(stats.max_interactions, 1);
    assert_eq!(stats.seed, 987);
    let dark = estimate_medium(
        Case::degrees(0, 45.0, 90.0, 0.0, 2.0),
        vacuum(),
        64,
        987,
        EVENT_LIMIT,
    )
    .unwrap();
    check_stats(&dark);
    assert_eq!(dark.zero_batches, 8);
    assert_eq!(dark.nonzero_paths, 0);
    assert_eq!(dark.radiance.mean, 0.0);
    assert_eq!(dark.radiance.standard_error, 0.0);
}

#[test]
fn adaptive_refinement_and_direct_first_order_monte_carlo_agree() {
    let case = Case::degrees(1, 45.0, 15.0, 90.0, 2.0);
    let first = deterministic_single(case).unwrap();
    let (camera, sun, view) = case.vectors().unwrap();
    let refined = transport::single(
        camera,
        sun,
        view,
        Medium::band(1),
        transport::SingleSettings {
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
        },
    )
    .unwrap();
    assert!((first.radiance - refined.radiance).abs() < 1e-8);
    let stats = estimate(case, 4096, 0x1234_5678).unwrap();
    check_stats(&stats);
    assert!(
        stats.max_interactions > 4,
        "RR retains unbounded higher-order paths"
    );
    assert!(stats.by_order[4].mean > 0.0);
    assert!(
        stats.ground.mean > 0.0,
        "indirect ground paths remain present"
    );
    assert!(stats.atmosphere.mean > 0.0);
    assert!(
        (stats.direct_volume.mean - first.volume).abs()
            <= 6.0 * stats.direct_volume.standard_error + 1e-7,
        "direct MC {:?}, independent quadrature {}",
        stats.direct_volume,
        first.volume
    );
    assert_eq!(first.unscattered_ground, 0.0);
    assert_eq!(stats.direct_ground.mean, 0.0);
    let down = Case::degrees(0, 45.0, -45.0, 0.0, 2000.0);
    let direct = deterministic_single(down).unwrap();
    let coupled = estimate(down, 2048, 0x9988).unwrap();
    check_stats(&coupled);
    assert!(coupled.direct_ground.mean > 0.0);
    assert!(
        (coupled.direct_ground.mean - direct.unscattered_ground).abs()
            <= 6.0 * coupled.direct_ground.standard_error + 1e-7
    );
    assert!(
        (coupled.direct_volume.mean - direct.volume).abs()
            <= 6.0 * coupled.direct_volume.standard_error + 1e-7
    );
    // Near-terminator refinement exercises exact shadow and ozone subdivisions.
    let twilight = Case::degrees(2, -3.0, 1.0, 0.0, 2.0);
    let a = deterministic_single(twilight).unwrap();
    let (camera, sun, view) = twilight.vectors().unwrap();
    let b = transport::single(
        camera,
        sun,
        view,
        Medium::band(2),
        transport::SingleSettings {
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
        },
    )
    .unwrap();
    assert!((a.volume - b.volume).abs() < 1e-8);
}

#[test]
fn safety_caps_refuse_the_whole_estimate_instead_of_zeroing_samples() {
    let case = Case::degrees(0, 45.0, 15.0, 0.0, 2.0);
    let error = estimate_medium(case, Medium::band(0), 64, 3, 0).unwrap_err();
    assert!(matches!(error.reason, Reason::EventLimit));
    assert_eq!(error.completed_paths, 0);
    assert_eq!(error.requested_paths, 64);
    assert!(matches!(
        estimate(case, 7, 3).unwrap_err().reason,
        Reason::InvalidPathCount
    ));
    assert!(matches!(
        estimate(Case { band: 3, ..case }, 64, 3)
            .unwrap_err()
            .reason,
        Reason::InvalidCase
    ));
    let error = quadrature::integrate(
        &[0.0, 1.0],
        Settings {
            evaluations: 0,
            ..Settings::default()
        },
        Ok,
    )
    .unwrap_err();
    assert!(matches!(error.reason, Reason::QuadratureLimit));
}
