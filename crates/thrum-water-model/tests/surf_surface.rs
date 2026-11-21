//! Original answers remain exact; surf supplies all derivatives and dry support.
use crate::support;
use support::fixtures::{body, field, grid, profile};
use support::untransformed;
use thrum_water_model::{BodyKind, Map, SurfProfile, compare};

#[test]
fn disabled_surf_and_off_bed_are_exact_and_the_missing_transform_is_detected() {
    let make = || {
        let mut b = body("water", BodyKind::Sea, 0.0);
        b.gains = [1.0; 4];
        Map::prepare(
            grid([121, 121], |x, _| (x - 100.0) * 0.1),
            vec![b],
            profile(),
        )
        .unwrap()
    };
    let f = field(0.5);
    let original = make();
    let mut surf = make();
    surf.enable_surf(SurfProfile::default()).unwrap();
    for p in [[-10.0, 10.0], [130.0, 30.0]] {
        for t in [0.0, 0.3, 1.7] {
            assert_eq!(
                original.surface(&f, p, t).unwrap(),
                surf.surface(&f, p, t).unwrap()
            );
        }
    }
    let oracle = |p, t| surf.surface(&f, p, t as f64 / 60.0);
    // Map, limit, section and cut are retained; only the shore transform is absent.
    let missing = untransformed::Untransformed {
        map: &original,
        field: &f,
        section: thrum_water_model::BreakerSection::first(),
    };
    let comparison = compare(
        &oracle,
        &missing,
        &[[60.0, 60.0], [80.0, 60.0], [95.0, 60.0]],
        &[0, 30, 90],
    )
    .unwrap();
    assert!(comparison.surface.height > 0.1);
    assert_ne!(comparison.surface_differences.len(), 0);
    for p in [[60.3, 60.3], [80.3, 60.3]] {
        let t = 0.3;
        let h = 1e-3;
        let sample = surf.surface(&f, p, t).unwrap().sample;
        let a = surf.surface(&f, p, t - h).unwrap().sample.height;
        let b = surf.surface(&f, p, t + h).unwrap().sample.height;
        assert!((sample.height_dt - (b - a) / (2.0 * h)).abs() < 0.01);
        for axis in 0..2 {
            let mut a = p;
            a[axis] -= h;
            let mut b = p;
            b[axis] += h;
            let actual = (surf.surface(&f, b, t).unwrap().sample.height
                - surf.surface(&f, a, t).unwrap().sample.height)
                / (2.0 * h);
            assert!((sample.slope[axis] - actual).abs() < 0.02);
        }
        assert!(sample.velocity.iter().all(|v| v.is_finite()));
    }
    assert!(!surf.surface(&f, [110.0, 60.0], 0.0).unwrap().wet);
}
