//! Domain refusal and bounded wide-query behavior.
use thrum_core::profile::{Number as N, Vector};
use thrum_sim2d::{Circle, CircleIndex, QueryError, Shape};
fn v(x: i32, y: i32) -> Vector {
    Vector {
        x: N::from_i32(x),
        y: N::from_i32(y),
    }
}

#[test]
fn limits_refuse_without_replacing_and_wide_queries_are_bounded() {
    let limit = 16_777_216;
    assert!(matches!(
        CircleIndex::new(N::ONE / N::from_i32(32)),
        Err(QueryError::CellSize)
    ));
    let mut index = CircleIndex::new(N::ONE / N::from_i32(16)).expect("valid fixture");
    let bodies = [
        Circle {
            slot: 3,
            centre: v(-limit, limit),
            radius: N::ZERO,
        },
        Circle {
            slot: 9,
            centre: v(limit, -limit),
            radius: N::from_i32(limit),
        },
    ];
    index.build(&bodies).expect("valid fixture");
    assert_eq!(
        index.build(&[Circle {
            centre: v(limit + 2, 0),
            ..bodies[0]
        }]),
        Err(QueryError::Body(3))
    );
    assert_eq!(
        index.build(&[Circle {
            radius: N::from_i32(limit + 2),
            ..bodies[0]
        }]),
        Err(QueryError::Body(3))
    );
    for shape in [
        Shape::Circle {
            centre: v(0, 0),
            radius: N::from_i32(limit),
        },
        Shape::Box {
            minimum: v(-limit, -limit),
            maximum: v(limit, limit),
        },
        Shape::Capsule {
            start: v(-limit, limit),
            end: v(limit, -limit),
            radius: N::ZERO,
        },
    ] {
        let expected: Vec<_> = [bodies[1], bodies[0]]
            .iter()
            .filter(|b| shape.intersects(b))
            .map(|b| b.slot)
            .collect();
        let mut out = Vec::new();
        index
            .query(shape, |b| out.push(b.slot))
            .expect("valid fixture");
        assert_eq!(out, expected); // Billions of cells per axis at the minimum size.
    }
    for shape in [
        Shape::Circle {
            centre: v(limit + 2, 0),
            radius: N::ZERO,
        },
        Shape::Circle {
            centre: v(0, 0),
            radius: N::from_i32(-1),
        },
        Shape::Box {
            minimum: v(0, 0),
            maximum: v(limit + 2, 0),
        },
        Shape::Box {
            minimum: v(1, 0),
            maximum: v(0, 0),
        },
        Shape::Capsule {
            start: v(0, 0),
            end: v(0, limit + 2),
            radius: N::ONE,
        },
    ] {
        assert_eq!(
            index.query(shape, |_| panic!("invalid query visited")),
            Err(QueryError::Query)
        );
    }
    let mut out = Vec::new();
    index
        .nearest(v(limit, -limit), N::ZERO, 2, &mut out)
        .expect("valid fixture");
    assert_eq!(out, [(N::ZERO, 9)]);
    assert_eq!(
        index.nearest(v(0, 0), N::from_i32(limit + 2), 1, &mut out),
        Err(QueryError::Query)
    );
}
