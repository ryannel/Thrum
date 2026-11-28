use thrum_core::profile::{Number, StateEncode};
use thrum_sim3d::{
    DOMAIN_BOUND, QueryError, Shape, SweepGap, Vector3, depth, distance, overlap, ray, sweep,
    sweep_two,
};

use crate::cases::{FORWARD, STOP, capsule, cube, n, ratio, sphere, triangle, v, x};

#[test]
fn shapes_that_reach_the_domain_limits_are_admitted() {
    let bound = DOMAIN_BOUND;
    let face = n(bound) - ratio(1, 2);
    let near_corner = bound - 48;
    assert_eq!(bound, 32_768);
    let denominator = thrum_sim3d::CONTACT_TOLERANCE_DENOMINATOR;
    assert!(denominator >= 64 && (denominator & (denominator - 1)) == 0);
    assert_eq!(SweepGap::Tolerance.units(), ratio(1, denominator));
    assert_eq!(SweepGap::HalfTolerance.units(), ratio(1, 2 * denominator));

    for sx in [-1, 1] {
        for sy in [-1, 1] {
            for sz in [-1, 1] {
                let corner = v(sx * bound, sy * bound, sz * bound);
                let outward = v(sx, 0, 0);
                let center = v(sx * near_corner, sy * near_corner, sz * near_corner);
                let side = v(sx * bound, sy * near_corner, sz * near_corner);
                let shapes = [
                    (sphere(STOP, n(bound)), v(sx * bound, 0, 0), -outward),
                    (sphere(center, n(48)), side, -outward),
                    (
                        capsule(
                            [
                                Vector3::new(n(sx) * face, STOP.y, STOP.z),
                                Vector3::new(n(-sx) * face, STOP.y, STOP.z),
                            ],
                            ratio(1, 2),
                        ),
                        v(sx * bound, 0, 0),
                        -outward,
                    ),
                    (
                        capsule(
                            [
                                center,
                                v(-sx * near_corner, sy * near_corner, sz * near_corner),
                            ],
                            n(48),
                        ),
                        side,
                        -outward,
                    ),
                    (cube(STOP, n(bound)), corner, -outward),
                    (
                        triangle([
                            corner,
                            v(sx * bound, -sy * bound, sz * bound),
                            v(sx * bound, sy * bound, -sz * bound),
                        ]),
                        corner - outward,
                        outward,
                    ),
                ];
                for (shape, start, motion) in shapes {
                    let point = sphere(start, Number::ZERO);
                    for (first, second) in [(point, shape), (shape, point)] {
                        let gap = distance(first, second).unwrap();
                        assert!(gap.is_finite());
                        assert_eq!(overlap(first, second).unwrap(), gap == Number::ZERO);
                        let penetration = depth(first, second).unwrap();
                        assert!(
                            penetration.depth.is_finite()
                                && penetration.normal.length().is_finite()
                        );
                    }
                    let hit = ray(start, motion, shape).unwrap().unwrap();
                    assert!(
                        hit.fraction.is_finite()
                            && hit.normal.length().is_finite()
                            && hit.point.length().is_finite()
                            && hit.depth.is_finite()
                    );
                    let hit = sweep(point, motion, shape, SweepGap::Tolerance)
                        .unwrap()
                        .unwrap();
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

#[test]
fn half_gap_checks_each_motion_and_the_exact_relative_length() {
    let mover = sphere(STOP, n(0));
    let struck = sphere(v(0, 20, 0), n(0));
    let gap = SweepGap::HalfTolerance;
    let limit = v(0, 0, 4096);
    let excess = v(1, 1, 4096);
    let tiny = x(ratio(1, 1 << 24));
    for (first, second) in [
        (excess, STOP),
        (STOP, excess),
        (v(1, 1, 2048), v(0, 0, -2048)),
        (limit + tiny, STOP),
        (limit, v(0, 0, -1) * ratio(1, 1 << 24)),
    ] {
        assert_eq!(
            sweep_two(mover, first, struck, second, gap),
            Err(QueryError::HalfGapMotionTooLong),
            "{first:?} {second:?}"
        );
    }
    for (first, second) in [
        (limit, STOP),
        (STOP, limit),
        (limit / n(2), -limit / n(2)),
        (v(1, 1, 4095), STOP),
    ] {
        assert!(sweep_two(mover, first, struck, second, gap).is_ok());
    }
}

#[test]
fn invalid_inputs_have_typed_errors() {
    let valid = sphere(STOP, Number::ZERO);
    let out = n(DOMAIN_BOUND + 1);
    let infinite = n(1) / n(0);
    let sub_grid = x(ratio(1, 2048));
    let bound = n(DOMAIN_BOUND);
    for (shape, fault) in [
        (sphere(x(out), n(0)), QueryError::OutsideDomain),
        (sphere(x(infinite), n(0)), QueryError::NonFinite),
        (sphere(STOP, n(-1)), QueryError::NegativeRadius),
        (
            sphere(x(n(DOMAIN_BOUND)), ratio(1, 4)),
            QueryError::OutsideDomain,
        ),
        (sphere(sub_grid, bound), QueryError::OutsideDomain),
        (sphere(-sub_grid, bound), QueryError::OutsideDomain),
        (capsule([STOP, sub_grid], bound), QueryError::OutsideDomain),
        (cube(sub_grid, bound), QueryError::OutsideDomain),
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
    let face = x(bound);
    assert_eq!(
        ray(face, sub_grid, valid),
        Err(QueryError::MotionOutsideDomain)
    );
    assert_eq!(
        sweep(sphere(-face, n(0)), -sub_grid, valid, SweepGap::Tolerance),
        Err(QueryError::MotionOutsideDomain)
    );
    let triangle = Shape::Triangle {
        corners: [STOP, FORWARD, v(0, 1, 0)],
    };
    assert_eq!(
        sweep(triangle, FORWARD, valid, SweepGap::Tolerance),
        Err(QueryError::TriangleMover)
    );
}
