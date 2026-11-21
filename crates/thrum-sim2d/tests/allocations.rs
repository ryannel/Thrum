//! Count warmed index operations; counts are observations, never assertions.
use std::io::Write;
use thrum_core::profile::{Number as N, Vector};
use thrum_sim2d::{Circle, CircleIndex, Shape};

#[global_allocator]
static ALLOCATOR: thrum_test_support::CountingAllocator = thrum_test_support::CountingAllocator;

fn measured(name: &str, mut run: impl FnMut()) {
    run();
    let ((), counted) = thrum_test_support::allocations::count(run);
    writeln!(
        std::io::stdout().lock(),
        "{name}: {} allocations per warmed call",
        counted.count
    )
    .expect("allocation report");
}
fn v(x: i32, y: i32) -> Vector {
    Vector {
        x: N::from_i32(x),
        y: N::from_i32(y),
    }
}
#[test]
fn warmed_calls_at_ten_thousand() {
    let bodies: Vec<_> = (0..10_000)
        .map(|slot| Circle {
            slot,
            centre: v(
                i32::try_from(slot % 100).expect("valid fixture"),
                i32::try_from(slot / 100).expect("valid fixture"),
            ),
            radius: N::from_i32(if slot % 997 == 0 { 6 } else { 1 }),
        })
        .collect();
    let mut index = CircleIndex::new(N::from_i32(4)).expect("valid fixture");
    index.build(&bodies).expect("valid fixture");
    measured("build", || index.build(&bodies).expect("valid fixture"));
    for (name, shape) in [
        (
            "circle",
            Shape::Circle {
                centre: v(50, 50),
                radius: N::from_i32(2),
            },
        ),
        (
            "box",
            Shape::Box {
                minimum: v(45, 45),
                maximum: v(55, 55),
            },
        ),
        (
            "capsule",
            Shape::Capsule {
                start: v(45, 45),
                end: v(55, 55),
                radius: N::ONE,
            },
        ),
        (
            "wide circle",
            Shape::Circle {
                centre: v(50, 50),
                radius: N::from_i32(16_777_216),
            },
        ),
    ] {
        measured(name, || {
            index
                .query(shape, |body| {
                    std::hint::black_box(body);
                })
                .expect("valid fixture");
        });
    }
    let mut out = Vec::with_capacity(8);
    measured("nearest-k", || {
        index
            .nearest(v(50, 50), N::from_i32(8), 8, &mut out)
            .expect("valid fixture");
    });
}
