use thrum_core::profile::{Number, StateEncode};
use thrum_sim3d::{
    DOMAIN_BOUND, QueryError, Shape, SweepGap, Vector3, depth, distance, overlap, ray, sweep,
};

use crate::cases::{FORWARD, STOP, capsule, cube, n, ratio, sphere, triangle, v, x};

#[test]
fn shapes_that_reach_the_domain_limits_are_admitted() {
    let bound = DOMAIN_BOUND;
    let face = n(bound) - ratio(1, 2);
    let center = sphere(STOP, Number::ZERO);
    for sx in [-1, 1] {
        for sy in [-1, 1] {
            for sz in [-1, 1] {
                let corner = v(sx * bound, sy * bound, sz * bound);
                let inward = v(-sx, -sy, -sz);
                let here = corner + inward;
                let shapes = [
                    sphere(STOP, n(bound)),
                    capsule(
                        [
                            Vector3::new(n(sx) * face, STOP.y, STOP.z),
                            Vector3::new(n(-sx) * face, STOP.y, STOP.z),
                        ],
                        ratio(1, 2),
                    ),
                    cube(STOP, n(bound)),
                    triangle([
                        corner,
                        v(sx * bound, -sy * bound, sz * bound),
                        v(sx * bound, sy * bound, -sz * bound),
                    ]),
                ];
                for shape in shapes {
                    assert!(distance(center, shape).unwrap().is_finite());
                    assert!(distance(shape, center).unwrap().is_finite());
                    assert_eq!(
                        overlap(shape, center).unwrap(),
                        distance(shape, center).unwrap() == Number::ZERO
                    );
                    let penetration = depth(center, shape).unwrap();
                    assert!(
                        penetration.depth.is_finite() && penetration.normal.length().is_finite()
                    );
                    if let Some(hit) = ray(here, -inward, shape).unwrap() {
                        assert!(
                            hit.fraction.is_finite()
                                && hit.normal.length().is_finite()
                                && hit.point.length().is_finite()
                        );
                    }
                    let mover = sphere(here, Number::ZERO);
                    if let Some(hit) = sweep(mover, -inward, shape, SweepGap::Tolerance).unwrap() {
                        assert!(
                            hit.fraction.is_finite()
                                && hit.normal.length().is_finite()
                                && hit.point.length().is_finite()
                                && hit.depth.is_finite()
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn invalid_inputs_have_typed_errors() {
    let valid = sphere(STOP, Number::ZERO);
    let out = n(DOMAIN_BOUND + 1);
    let infinite = n(1) / n(0);
    for (shape, fault) in [
        (sphere(x(out), n(0)), QueryError::OutsideDomain),
        (sphere(x(infinite), n(0)), QueryError::NonFinite),
        (sphere(STOP, n(-1)), QueryError::NegativeRadius),
        (
            sphere(x(n(DOMAIN_BOUND)), ratio(1, 4)),
            QueryError::OutsideDomain,
        ),
        (capsule([STOP, x(out)], n(0)), QueryError::OutsideDomain),
        (capsule([STOP, FORWARD], n(-1)), QueryError::NegativeRadius),
        (cube(STOP, out), QueryError::OutsideDomain),
        (
            triangle([STOP, FORWARD, x(infinite)]),
            QueryError::NonFinite,
        ),
        (triangle([STOP, FORWARD, x(out)]), QueryError::OutsideDomain),
    ] {
        assert_eq!(distance(shape, valid), Err(fault));
        assert_eq!(distance(valid, shape), Err(fault));
        assert_eq!(overlap(shape, valid), Err(fault));
        assert_eq!(depth(shape, valid), Err(fault));
        assert_eq!(ray(STOP, FORWARD, shape), Err(fault));
        assert_eq!(
            sweep(valid, FORWARD, shape, SweepGap::Tolerance),
            Err(fault)
        );
    }
    assert_eq!(
        ray(STOP, x(out), valid),
        Err(QueryError::MotionOutsideDomain)
    );
    assert_eq!(ray(x(out), FORWARD, valid), Err(QueryError::OutsideDomain));
    assert_eq!(ray(STOP, x(infinite), valid), Err(QueryError::NonFinite));
    assert_eq!(
        sweep(valid, x(out), valid, SweepGap::Tolerance),
        Err(QueryError::MotionOutsideDomain)
    );
    assert_eq!(
        sweep(valid, x(infinite), valid, SweepGap::Tolerance),
        Err(QueryError::NonFinite)
    );
    assert_eq!(
        sweep(valid, v(4097, 0, 0), valid, SweepGap::HalfTolerance),
        Err(QueryError::HalfGapMotionTooLong)
    );
    assert!(sweep(valid, v(4096, 0, 0), valid, SweepGap::HalfTolerance).is_ok());
    let triangle = Shape::Triangle {
        corners: [STOP, FORWARD, v(0, 1, 0)],
    };
    assert_eq!(
        sweep(triangle, FORWARD, valid, SweepGap::Tolerance),
        Err(QueryError::TriangleMover)
    );
}
