//! A2 render-column lifecycle and steady-state allocation checks.
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

use thrum_render::extract::{RenderExtractor, SourceTick};

thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

struct CountingAllocator;
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

#[expect(
    unsafe_code,
    reason = "test-only allocation observer delegates every allocation to System"
)]
// SAFETY: This observer forwards the original pointer and layout unchanged to
// System. Its thread-local counter has no ownership of allocated memory.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        COUNTING.with(|active| {
            if active.get() {
                ALLOCATIONS.with(|count| count.set(count.get() + 1));
            }
        });
        // SAFETY: The allocator contract and layout are forwarded unchanged.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: The pointer/layout pair came from the delegated allocator.
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        COUNTING.with(|active| {
            if active.get() {
                ALLOCATIONS.with(|count| count.set(count.get() + 1));
            }
        });
        // SAFETY: The pointer/layout pair and requested size are forwarded.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

fn source(epoch: u128, tick: u64) -> SourceTick {
    SourceTick { epoch, tick }
}

#[test]
fn prior_sample_requires_adjacent_tick_stable_id_and_visual_revision() {
    let mut extractor = RenderExtractor::new(4);
    let first = extractor.extract(source(7, 40), [100.0, 0.0, 0.0], |sink| {
        assert!(sink.push(1, [101.0, 0.0, 0.0], 11));
        assert!(sink.push(2, [102.0, 0.0, 0.0], 22));
    });
    assert_eq!(first.accepted, 2);
    assert_eq!(first.history_reused, 0);
    let second = extractor.extract(source(7, 41), [101.0, 0.0, 0.0], |sink| {
        assert!(sink.push(1, [103.0, 0.0, 0.0], 11));
        assert!(sink.push(2, [104.0, 0.0, 0.0], 23));
        assert!(sink.push(3, [105.0, 0.0, 0.0], 11));
    });
    assert_eq!(second.history_reused, 1);
    let frame = extractor.frame();
    assert_eq!(frame.source(), Some(source(7, 41)));
    assert_eq!(frame.origin(), [101.0, 0.0, 0.0]);
    assert_eq!(frame.previous_origin(), [100.0, 0.0, 0.0]);
    assert_eq!(frame.ids(), &[1, 2, 3]);
    assert_eq!(
        frame.current(),
        &[[2.0, 0.0, 0.0], [3.0, 0.0, 0.0], [4.0, 0.0, 0.0]]
    );
    assert_eq!(frame.previous()[0], [1.0, 0.0, 0.0]);
    assert_eq!(frame.history_valid(), &[true, false, false]);
    let skipped = extractor.extract(source(7, 43), [101.0, 0.0, 0.0], |sink| {
        assert!(sink.push(1, [106.0, 0.0, 0.0], 11));
    });
    assert_eq!(skipped.history_reused, 0);
    assert_eq!(extractor.frame().previous(), extractor.frame().current());
    let restarted = extractor.extract(source(8, 44), [0.0; 3], |sink| {
        assert!(sink.push(1, [0.0; 3], 11));
    });
    assert_eq!(restarted.history_reused, 0);
}

#[test]
fn presentation_overflow_invalid_coordinates_and_duplicates_degrade_with_counts() {
    let mut extractor = RenderExtractor::new(2);
    let report = extractor.extract(source(1, 1), [0.0; 3], |sink| {
        assert!(!sink.push(1, [f64::NAN, 0.0, 0.0], 1));
        assert!(sink.push(1, [1.0, 0.0, 0.0], 1));
        assert!(!sink.push(1, [2.0, 0.0, 0.0], 1));
        assert!(sink.push(2, [3.0, 0.0, 0.0], 1));
        assert!(!sink.push(3, [4.0, 0.0, 0.0], 1));
    });
    assert_eq!(report.accepted, 2);
    assert_eq!(report.dropped_invalid, 1);
    assert_eq!(report.dropped_duplicate, 1);
    assert_eq!(report.dropped_capacity, 1);
    assert_eq!(extractor.frame().ids(), &[1, 2]);
}

#[test]
fn ten_thousand_visuals_reuse_all_allocations_after_warmup() {
    let mut extractor = RenderExtractor::new(10_000);
    for tick in 0..2 {
        extractor.extract(source(1, tick), [0.0; 3], |sink| {
            for id in 0..10_000u128 {
                assert!(sink.push(id, [0.0; 3], 1));
            }
        });
    }
    ALLOCATIONS.with(|count| count.set(0));
    COUNTING.with(|active| active.set(true));
    for tick in 2..62 {
        let report = extractor.extract(source(1, tick), [0.0; 3], |sink| {
            for id in 0..10_000u128 {
                // The last hundred IDs churn without changing the live count.
                let id = if id < 9_900 {
                    id
                } else {
                    10_000 + u128::from(tick) * 100 + id
                };
                sink.push(id, [0.0; 3], 1);
            }
        });
        std::hint::black_box(report);
    }
    COUNTING.with(|active| active.set(false));
    assert_eq!(ALLOCATIONS.with(Cell::get), 0);
    assert_eq!(extractor.frame().len(), 10_000);
}
