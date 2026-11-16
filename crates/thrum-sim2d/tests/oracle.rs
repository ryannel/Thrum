//! Full-enumeration query oracle across dense scenes and table sizes.
use thrum_core::profile::{Number as N, Vector};
use thrum_sim2d::{Circle, CircleIndex, Shape};

fn v(x: i32, y: i32) -> Vector {
    Vector {
        x: N::from_i32(x),
        y: N::from_i32(y),
    }
}

fn compare(bodies: &[Circle], size: N, shapes: &[Shape]) {
    // Padding changes the hash table's power-of-two capacity, but cannot hit any query.
    for padding in [0, bodies.len().max(2)] {
        let mut input = bodies.to_vec();
        for i in 0..padding {
            input.push(Circle {
                slot: u32::try_from(bodies.len() + i).expect("valid fixture"),
                centre: v(100_000, 100_000),
                radius: N::ZERO,
            });
        }
        let mut index = CircleIndex::new(size).expect("valid fixture");
        index.build(&input).expect("valid fixture");
        for &shape in shapes {
            let mut expected: Vec<_> = input.iter().filter(|body| shape.intersects(body)).collect();
            expected.sort_unstable_by_key(|body| {
                if body.radius > size {
                    (false, 0, 0, body.slot)
                } else {
                    (
                        true,
                        (body.centre.y / size).floor().to_i32(),
                        (body.centre.x / size).floor().to_i32(),
                        body.slot,
                    )
                }
            });
            let expected: Vec<_> = expected.iter().map(|body| body.slot).collect();
            let mut actual = Vec::new();
            index
                .query(shape, |body| actual.push(body.slot))
                .expect("valid fixture");
            assert_eq!(actual, expected, "size {size:?}, {shape:?}");
        }
        for centre in [v(0, 0), v(16_384, -16_384), v(-3, 7)] {
            for (distance, k) in [(0, 0), (0, 5), (4, 7), (40_000, 10_100)] {
                let distance = N::from_i32(distance);
                let mut expected: Vec<_> = input
                    .iter()
                    .filter_map(|body| {
                        let x = body.centre.x - centre.x;
                        let y = body.centre.y - centre.y;
                        let d = x * x + y * y;
                        (d <= distance * distance).then_some((d, body.slot))
                    })
                    .collect();
                expected.sort_unstable_by(|a, b| {
                    a.0.partial_cmp(&b.0)
                        .expect("valid fixture")
                        .then(a.1.cmp(&b.1))
                });
                expected.truncate(k);
                let mut actual = Vec::new();
                index
                    .nearest(centre, distance, k, &mut actual)
                    .expect("valid fixture");
                assert_eq!(actual, expected);
            }
        }
    }
}

#[test]
fn generated_sequences_match_full_enumeration() {
    let shapes = [
        Shape::Circle {
            centre: v(0, 0),
            radius: N::ZERO,
        },
        Shape::Circle {
            centre: v(0, 0),
            radius: N::from_i32(4),
        },
        Shape::Circle {
            centre: v(0, 0),
            radius: N::from_i32(40_000),
        },
        Shape::Box {
            minimum: v(-3, -2),
            maximum: v(6, 4),
        },
        Shape::Box {
            minimum: v(-20_000, -20_000),
            maximum: v(20_000, 20_000),
        },
        Shape::Capsule {
            start: v(-10, -3),
            end: v(10, 3),
            radius: N::ONE,
        },
        Shape::Capsule {
            start: v(-20_000, 20_000),
            end: v(20_000, -20_000),
            radius: N::from_i32(4),
        },
        Shape::Capsule {
            start: v(0, 0),
            end: v(0, 0),
            radius: N::ZERO,
        },
    ];
    let dense: Vec<_> = (0..10_000)
        .map(|slot| {
            let i = i32::try_from(slot / 3).expect("valid fixture");
            Circle {
                slot,
                centre: if slot % 997 == 0 {
                    v(16_384, -16_384)
                } else {
                    v(i % 57 - 28, i / 57 - 28)
                },
                radius: N::from_i32(i32::try_from(slot % 9).expect("valid fixture"))
                    / N::from_i32(2),
            }
        })
        .collect();
    for size in [N::ONE / N::from_i32(16), N::ONE, N::from_i32(4)] {
        compare(&[], size, &shapes);
        compare(&dense[..1], size, &shapes);
        compare(&dense, size, &shapes);
    }
    for seed in 0..24_i32 {
        let fractional: Vec<_> = (0..37_u32)
            .map(|slot| {
                let i = i32::try_from(slot).expect("small scene");
                let centre = v(
                    (i * 71 + seed * 29) % 113 - 56,
                    (i * 17 + seed * 43) % 127 - 63,
                );
                Circle {
                    slot,
                    centre: Vector {
                        x: centre.x / N::from_i32(7),
                        y: centre.y / N::from_i32(11),
                    },
                    radius: N::from_i32((i + seed) % 13) / N::from_i32(9),
                }
            })
            .collect();
        for size in [N::ONE / N::from_i32(16), N::ONE, N::from_i32(4)] {
            compare(&fractional, size, &shapes);
        }
    }
}
