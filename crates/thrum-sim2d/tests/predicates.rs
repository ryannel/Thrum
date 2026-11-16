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
    let just_outside = Vector {
        x: N::ONE + N::ONE / N::from_i32(1024),
        y: N::ZERO,
    };
    assert!(
        !Shape::Box {
            minimum: just_outside,
            maximum: v(2, 1)
        }
        .intersects(&circle)
    );
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

#[test]
fn tiny_shapes_do_not_gain_hits_when_squares_underflow() {
    let tiny = N::ONE
        / N::from_i32(1_000_000)
        / N::from_i32(1_000_000)
        / N::from_i32(1_000_000)
        / N::from_i32(1_000_000);
    let p = |x, y| Vector {
        x: N::from_i32(x) * tiny,
        y: N::from_i32(y) * tiny,
    };
    let point = |x, y| Circle {
        slot: 0,
        centre: p(x, y),
        radius: N::ZERO,
    };
    let disc = Shape::Circle {
        centre: p(0, 0),
        radius: tiny,
    };
    assert!(disc.intersects(&point(1, 0)));
    assert!(!disc.intersects(&point(1, 1)));
    assert!(
        !Shape::Circle {
            centre: p(0, 0),
            radius: N::ZERO
        }
        .intersects(&point(-1, 0))
    );
    let bounds = Shape::Box {
        minimum: p(0, 0),
        maximum: p(0, 0),
    };
    assert!(!bounds.intersects(&point(-1, 0)));
    assert!(bounds.intersects(&Circle {
        radius: tiny,
        ..point(1, 0)
    }));
    assert!(!bounds.intersects(&Circle {
        radius: tiny,
        ..point(1, 1)
    }));
    let capsule = Shape::Capsule {
        start: p(-2, 0),
        end: p(2, 0),
        radius: N::ZERO,
    };
    assert!(capsule.intersects(&point(0, 0)));
    assert!(capsule.intersects(&point(2, 0)));
    assert!(!capsule.intersects(&point(3, 0)));
    assert!(!capsule.intersects(&point(0, 1)));
    assert!(capsule.intersects(&Circle {
        radius: tiny,
        ..point(0, 1)
    }));
}
