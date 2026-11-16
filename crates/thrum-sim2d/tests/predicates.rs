//! Hand-worked intersection and nearest-order cases.
use thrum_core::profile::{Number as N, Vector};
use thrum_sim2d::{Circle, CircleIndex, Shape};

fn v(x: i32, y: i32) -> Vector {
    Vector {
        x: N::from_i32(x),
        y: N::from_i32(y),
    }
}

#[test]
fn hand_worked_closed_geometry() {
    let circle = Circle {
        slot: 1,
        centre: v(0, 0),
        radius: N::ONE,
    };
    for (shape, expected) in [
        (
            Shape::Circle {
                centre: v(2, 0),
                radius: N::ONE,
            },
            true,
        ),
        (
            Shape::Circle {
                centre: v(2, 0),
                radius: N::ONE - N::ONE / N::from_i32(1024),
            },
            false,
        ),
        (
            Shape::Circle {
                centre: v(1, 0),
                radius: N::ZERO,
            },
            true,
        ),
        (
            Shape::Circle {
                centre: v(2, 0),
                radius: N::ZERO,
            },
            false,
        ),
        (
            Shape::Box {
                minimum: v(1, -1),
                maximum: v(3, 1),
            },
            true,
        ),
        (
            Shape::Box {
                minimum: v(2, -1),
                maximum: v(3, 1),
            },
            false,
        ),
        (
            Shape::Box {
                minimum: v(1, 0),
                maximum: v(1, 0),
            },
            true,
        ),
        (
            Shape::Box {
                minimum: v(1, 1),
                maximum: v(1, 1),
            },
            false,
        ),
        (
            Shape::Capsule {
                start: v(-3, 2),
                end: v(3, 2),
                radius: N::ONE,
            },
            true,
        ),
        (
            Shape::Capsule {
                start: v(-3, 2),
                end: v(3, 2),
                radius: N::ONE - N::ONE / N::from_i32(1024),
            },
            false,
        ),
        (
            Shape::Capsule {
                start: v(2, 0),
                end: v(2, 0),
                radius: N::ONE,
            },
            true,
        ),
        (
            Shape::Capsule {
                start: v(2, 0),
                end: v(2, 0),
                radius: N::ZERO,
            },
            false,
        ),
        (
            Shape::Capsule {
                start: v(-3, 1),
                end: v(3, 1),
                radius: N::ZERO,
            },
            true,
        ),
    ] {
        assert_eq!(shape.intersects(&circle), expected, "{shape:?}");
    }
    let point = Circle {
        radius: N::ZERO,
        ..circle
    };
    assert!(
        Shape::Circle {
            centre: v(0, 0),
            radius: N::ZERO
        }
        .intersects(&point)
    );
    assert!(
        !Shape::Capsule {
            start: v(1, 0),
            end: v(1, 0),
            radius: N::ZERO
        }
        .intersects(&point)
    );
}

#[test]
fn nearest_hand_worked_ties_and_cutoff() {
    let mut index = CircleIndex::new(N::ONE).expect("valid fixture");
    index
        .build(&[
            Circle {
                slot: 2,
                centre: v(1, 0),
                radius: N::ZERO,
            },
            Circle {
                slot: 7,
                centre: v(-1, 0),
                radius: N::ZERO,
            },
            Circle {
                slot: 9,
                centre: v(0, 0),
                radius: N::ZERO,
            },
            Circle {
                slot: 10,
                centre: v(0, 2),
                radius: N::from_i32(5),
            },
        ])
        .expect("valid fixture");
    let mut out = Vec::new();
    index
        .nearest(v(0, 0), N::ONE, 3, &mut out)
        .expect("valid fixture");
    assert_eq!(out, [(N::ZERO, 9), (N::ONE, 2), (N::ONE, 7)]);
    index
        .nearest(v(0, 0), N::ONE, 2, &mut out)
        .expect("valid fixture");
    assert_eq!(out, [(N::ZERO, 9), (N::ONE, 2)]);
    index
        .nearest(v(0, 0), N::ONE, 0, &mut out)
        .expect("valid fixture");
    assert_eq!(out, []);
}
