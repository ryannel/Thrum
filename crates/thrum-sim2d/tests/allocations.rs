//! Count warmed index operations; counts are observations, never assertions.
#![expect(
    unsafe_code,
    reason = "test-only allocator forwards unchanged operations to System"
)]
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    io::Write,
};
use thrum_core::profile::{Number as N, Vector};
use thrum_sim2d::{Circle, CircleIndex, Shape};

thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static COUNT: Cell<u64> = const { Cell::new(0) };
}
struct Counter;
#[global_allocator]
static ALLOCATOR: Counter = Counter;
fn count() {
    TRACK.with(|track| {
        if track.get() {
            COUNT.with(|n| n.set(n.get() + 1));
        }
    });
}
// SAFETY: Every pointer, layout and size is forwarded unchanged to System.
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        // SAFETY: Forward the caller's valid allocation layout.
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count();
        // SAFETY: Forward the caller's valid allocation layout.
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count();
        // SAFETY: The original System pointer and layout and requested size are unchanged.
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: The pointer and layout came from System and are unchanged.
        unsafe { System.dealloc(ptr, layout) }
    }
}
fn measured(name: &str, mut run: impl FnMut()) {
    run();
    COUNT.with(|n| n.set(0));
    TRACK.with(|t| t.set(true));
    run();
    TRACK.with(|t| t.set(false));
    writeln!(
        std::io::stdout().lock(),
        "{name}: {} allocations per warmed call",
        COUNT.with(Cell::get)
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
