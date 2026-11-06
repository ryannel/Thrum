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
    SourceTick {
        epoch,
        tick,
        timeline: 0,
    }
}

#[test]
fn repeating_one_tick_preserves_its_prior_pose_for_display_interpolation() {
    let mut extractor = RenderExtractor::new(1, 1);
    extractor.extract(source(1, 4), [0.0; 3], |sink| {
        assert!(sink.push(0, 1, 1, [0.0, 0.0, 0.0], 0));
    });
    extractor.extract(source(1, 5), [0.0; 3], |sink| {
        assert!(sink.push(0, 1, 1, [1.0, 0.0, 0.0], 0));
    });
    assert_eq!(extractor.frame().previous()[0][0], 0.0);
    for _ in 0..4 {
        let report = extractor.extract(source(1, 5), [0.0; 3], |sink| {
            assert!(sink.push(0, 1, 1, [1.0, 0.0, 0.0], 0));
        });
        assert_eq!(report.history_reused, 1);
        assert_eq!(extractor.frame().previous()[0][0], 0.0);
        assert_eq!(extractor.frame().current()[0][0], 1.0);
    }
}

#[test]
fn replay_generation_blocks_adjacent_pose_reuse() {
    let mut extractor = RenderExtractor::new(1, 1);
    extractor.extract(source(1, 2), [0.0; 3], |sink| {
        assert!(sink.push(0, 1, 1, [1.0, 0.0, 0.0], 0));
    });
    let report = extractor.extract(
        SourceTick {
            epoch: 1,
            tick: 3,
            timeline: 1,
        },
        [0.0; 3],
        |sink| assert!(sink.push(0, 1, 1, [5.0, 0.0, 0.0], 0)),
    );
    assert_eq!(report.history_reused, 0);
    assert_eq!(extractor.frame().previous()[0], [5.0, 0.0, 0.0]);
}

#[test]
fn prior_sample_requires_adjacent_tick_stable_id_and_visual_revision() {
    let mut extractor = RenderExtractor::new(4, 4);
    let first = extractor.extract(source(7, 40), [100.0, 0.0, 0.0], |sink| {
        assert!(sink.push(0, 1, 1, [101.0, 0.0, 0.0], 11));
        assert!(sink.push(1, 1, 2, [102.0, 0.0, 0.0], 22));
    });
    assert_eq!(first.accepted, 2);
    assert_eq!(first.history_reused, 0);
    let second = extractor.extract(source(7, 41), [101.0, 0.0, 0.0], |sink| {
        assert!(sink.push(0, 1, 1, [103.0, 0.0, 0.0], 11));
        assert!(sink.push(1, 1, 2, [104.0, 0.0, 0.0], 23));
        assert!(sink.push(2, 1, 3, [105.0, 0.0, 0.0], 11));
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
        assert!(sink.push(0, 1, 1, [106.0, 0.0, 0.0], 11));
    });
    assert_eq!(skipped.history_reused, 0);
    assert_eq!(extractor.frame().previous(), extractor.frame().current());
    let restarted = extractor.extract(source(8, 44), [0.0; 3], |sink| {
        assert!(sink.push(0, 1, 1, [0.0; 3], 11));
    });
    assert_eq!(restarted.history_reused, 0);
}

#[test]
fn visual_generation_change_resets_prior_pose_even_at_the_same_world_tick() {
    let mut extractor = RenderExtractor::new(1, 1);
    extractor.extract(source(1, 10), [0.0; 3], |sink| {
        sink.push(0, 1, 7, [1.0, 0.0, 0.0], 3);
    });
    let adjacent = extractor.extract(source(1, 11), [0.0; 3], |sink| {
        sink.push(0, 1, 7, [2.0, 0.0, 0.0], 3);
    });
    assert_eq!(adjacent.history_reused, 1);

    extractor.invalidate_history();
    assert_eq!(extractor.frame().source(), None);
    let replacement = extractor.extract(source(1, 11), [0.0; 3], |sink| {
        sink.push(0, 1, 7, [2.0, 0.0, 0.0], 3);
    });
    assert_eq!(replacement.history_reused, 0);
    assert_eq!(extractor.frame().history_valid(), &[false]);
    assert_eq!(extractor.frame().previous(), extractor.frame().current());
    let next = extractor.extract(source(1, 12), [0.0; 3], |sink| {
        sink.push(0, 1, 7, [3.0, 0.0, 0.0], 3);
    });
    assert_eq!(next.history_reused, 1);
}

#[test]
fn changed_identity_cannot_inherit_pose_even_if_a_generation_is_reused() {
    let mut extractor = RenderExtractor::new(1, 1);
    extractor.extract(source(1, 1), [0.0; 3], |sink| {
        sink.push(0, 1, 7, [1.0, 0.0, 0.0], 3);
    });
    let report = extractor.extract(source(1, 2), [0.0; 3], |sink| {
        sink.push(0, 1, 8, [2.0, 0.0, 0.0], 3);
    });
    assert_eq!(report.history_reused, 0);
    assert_eq!(extractor.frame().previous(), extractor.frame().current());
}

#[test]
fn slot_reuse_invalidates_pose_and_retired_slots_are_reported_once() {
    let mut extractor = RenderExtractor::new(2, 2);
    extractor.extract(source(1, 0), [0.0; 3], |sink| {
        assert!(sink.push(0, 1, 999, [1.0, 0.0, 0.0], 1));
        assert!(sink.push(1, 1, 3, [2.0, 0.0, 0.0], 1));
    });
    let reused = extractor.extract(source(1, 1), [0.0; 3], |sink| {
        assert!(sink.push(0, 2, 1, [4.0, 0.0, 0.0], 1));
    });
    assert_eq!(reused.history_reused, 0);
    assert_eq!(extractor.frame().generation_at_slot(0), Some(2));
    assert!(!extractor.frame().contains_slot(1));
    assert_eq!(extractor.frame().previous(), extractor.frame().current());
    extractor.extract(source(1, 2), [0.0; 3], |sink| {
        assert!(sink.push(0, 2, 1, [5.0, 0.0, 0.0], 1));
    });
    assert!(!extractor.frame().contains_slot(1));
    assert_eq!(extractor.frame().history_valid(), &[true]);
}

#[test]
fn presentation_overflow_invalid_coordinates_and_duplicates_degrade_with_counts() {
    let mut extractor = RenderExtractor::new(2, 2);
    let report = extractor.extract(source(1, 1), [0.0; 3], |sink| {
        assert!(!sink.push(0, 1, 1, [f64::NAN, 0.0, 0.0], 1));
        assert!(sink.push(0, 1, 1, [1.0, 0.0, 0.0], 1));
        assert!(!sink.push(0, 1, 1, [2.0, 0.0, 0.0], 1));
        assert!(sink.push(1, 1, 2, [3.0, 0.0, 0.0], 1));
        assert!(!sink.push(2, 1, 3, [4.0, 0.0, 0.0], 1));
    });
    assert_eq!(report.accepted, 2);
    assert_eq!(report.dropped_invalid, 1);
    assert_eq!(report.dropped_duplicate, 1);
    assert_eq!(report.dropped_capacity, 1);
    assert_eq!(extractor.frame().ids(), &[1, 2]);
}

#[test]
fn sparse_visuals_can_occupy_high_physical_slots() {
    let mut extractor = RenderExtractor::new(10_000, 2);
    let report = extractor.extract(source(1, 1), [0.0; 3], |sink| {
        assert!(sink.push(9_000, 1, 7, [1.0, 0.0, 0.0], 1));
    });
    assert_eq!(report.accepted, 1);
    assert_eq!(extractor.frame().slots(), &[9_000]);
    assert!(extractor.frame().contains_slot(9_000));
}

#[test]
fn ten_thousand_visuals_reuse_all_allocations_after_warmup() {
    let mut extractor = RenderExtractor::new(10_000, 10_000);
    for tick in 0..2 {
        extractor.extract(source(1, tick), [0.0; 3], |sink| {
            for slot in 0..10_000u32 {
                assert!(sink.push(slot, 1, u128::from(slot), [0.0; 3], 1));
            }
        });
    }
    ALLOCATIONS.with(|count| count.set(0));
    COUNTING.with(|active| active.set(true));
    for tick in 2..62 {
        let report = extractor.extract(source(1, tick), [0.0; 3], |sink| {
            for slot in 0..10_000u32 {
                // The last hundred identities churn and reuse physical slots.
                let id = if slot < 9_900 {
                    u128::from(slot)
                } else {
                    10_000 + u128::from(tick) * 100 + u128::from(slot)
                };
                sink.push(
                    slot,
                    if slot < 9_900 { 1 } else { tick + 1 },
                    id,
                    [0.0; 3],
                    1,
                );
            }
        });
        std::hint::black_box(report);
    }
    COUNTING.with(|active| active.set(false));
    assert_eq!(ALLOCATIONS.with(Cell::get), 0);
    assert_eq!(extractor.frame().len(), 10_000);
}

#[test]
fn adjacent_patch_preserves_pose_and_repairs_churned_dense_rows() {
    let mut extractor = RenderExtractor::new(4, 4);
    extractor.extract(source(1, 1), [0.0; 3], |sink| {
        sink.push(0, 1, 10, [0.0; 3], 1);
        sink.push(1, 1, 11, [1.0, 0.0, 0.0], 1);
        sink.push(2, 1, 12, [2.0, 0.0, 0.0], 1);
    });
    let report = extractor.extract_adaptive(
        source(1, 2),
        Some(source(1, 1)),
        [0.0; 3],
        |_| panic!("adjacent endpoint must use the patch"),
        |sink| {
            sink.set(0, 1, 10, [0.5, 0.0, 0.0], 1);
            sink.remove(1);
            sink.set(3, 1, 13, [3.0, 0.0, 0.0], 1);
        },
    );
    assert_eq!(report.accepted, 3);
    assert_eq!(report.history_reused, 2);
    let frame = extractor.frame();
    assert!(!frame.contains_slot(1));
    assert!(frame.contains_slot(2));
    let moved = frame.slots().iter().position(|slot| *slot == 0).unwrap();
    assert_eq!(frame.previous()[moved], [0.0; 3]);
    assert_eq!(frame.current()[moved], [0.5, 0.0, 0.0]);
    let spawned = frame.slots().iter().position(|slot| *slot == 3).unwrap();
    assert!(!frame.history_valid()[spawned]);
    assert_eq!(frame.previous()[spawned], frame.current()[spawned]);
    let next = extractor.extract_adaptive(
        source(1, 3),
        Some(source(1, 2)),
        [0.0; 3],
        |_| panic!("adjacent endpoint must use the patch"),
        |_| {},
    );
    assert_eq!(next.history_reused, 3);
    assert_eq!(extractor.frame().previous(), extractor.frame().current());
}

#[test]
fn skipped_page_or_origin_change_rebuilds_instead_of_patching() {
    let mut extractor = RenderExtractor::new(2, 2);
    extractor.extract(source(1, 1), [0.0; 3], |sink| {
        sink.push(0, 1, 10, [1.0, 0.0, 0.0], 1);
    });
    let skipped = extractor.extract_adaptive(
        source(1, 3),
        Some(source(1, 2)),
        [0.0; 3],
        |sink| assert!(sink.push(0, 1, 10, [3.0, 0.0, 0.0], 1)),
        |_| panic!("skipped source must rebuild"),
    );
    assert_eq!(skipped.history_reused, 0);
    let shifted = extractor.extract_adaptive(
        source(1, 4),
        Some(source(1, 3)),
        [1.0, 0.0, 0.0],
        |sink| assert!(sink.push(0, 1, 10, [4.0, 0.0, 0.0], 1)),
        |_| panic!("origin change must rebuild"),
    );
    assert_eq!(shifted.history_reused, 1);
    assert_eq!(extractor.frame().previous()[0], [3.0, 0.0, 0.0]);
    assert_eq!(extractor.frame().current()[0], [3.0, 0.0, 0.0]);
}

#[test]
fn patch_visual_replacement_and_overflow_degrade_then_rebuild() {
    let mut extractor = RenderExtractor::new(2, 1);
    extractor.extract(source(1, 1), [0.0; 3], |sink| {
        sink.push(0, 1, 10, [1.0, 0.0, 0.0], 1);
    });
    let changed = extractor.extract_adaptive(
        source(1, 2),
        Some(source(1, 1)),
        [0.0; 3],
        |sink| {
            assert!(sink.push(0, 1, 10, [2.0, 0.0, 0.0], 2));
            assert!(!sink.push(1, 1, 11, [3.0, 0.0, 0.0], 1));
        },
        |sink| {
            assert!(sink.set(0, 1, 10, [2.0, 0.0, 0.0], 2));
            assert!(!sink.set(1, 1, 11, [3.0, 0.0, 0.0], 1));
        },
    );
    assert_eq!(changed.dropped_capacity, 1);
    assert_eq!(changed.history_reused, 0);
    assert_eq!(extractor.frame().previous(), extractor.frame().current());
    let rebuilt = extractor.extract_adaptive(
        source(1, 3),
        Some(source(1, 2)),
        [0.0; 3],
        |sink| assert!(sink.push(1, 1, 11, [3.0, 0.0, 0.0], 1)),
        |_| panic!("an overflowed page must rebuild after a slot retires"),
    );
    assert_eq!(rebuilt.accepted, 1);
    assert_eq!(extractor.frame().slots(), &[1]);
    let invalid = extractor.extract_adaptive(
        source(1, 4),
        Some(source(1, 3)),
        [0.0; 3],
        |_| panic!("adjacent endpoint must patch"),
        |sink| assert!(!sink.set(1, 1, 11, [f64::NAN, 0.0, 0.0], 1)),
    );
    assert_eq!(invalid.dropped_invalid, 1);
    assert!(extractor.frame().is_empty());
}

#[test]
fn low_slot_replacement_at_capacity_uses_full_page_order() {
    let mut extractor = RenderExtractor::new(2, 1);
    extractor.extract(source(1, 1), [0.0; 3], |sink| {
        assert!(sink.push(1, 1, 11, [1.0, 0.0, 0.0], 1));
    });
    let report = extractor.extract_adaptive(
        source(1, 2),
        Some(source(1, 1)),
        [0.0; 3],
        |sink| assert!(sink.push(0, 1, 10, [2.0, 0.0, 0.0], 1)),
        |sink| {
            assert!(!sink.set(0, 1, 10, [2.0, 0.0, 0.0], 1));
            sink.remove(1);
        },
    );
    assert_eq!(report.dropped_capacity, 0);
    assert_eq!(extractor.frame().slots(), &[0]);
    assert_eq!(extractor.frame().previous(), extractor.frame().current());
}

#[test]
fn ten_thousand_adjacent_visuals_patch_without_allocations() {
    let mut extractor = RenderExtractor::new(10_000, 10_000);
    extractor.extract(source(1, 1), [0.0; 3], |sink| {
        for slot in 0..10_000_u32 {
            sink.push(slot, 1, u128::from(slot), [0.0; 3], 1);
        }
    });
    ALLOCATIONS.with(|count| count.set(0));
    COUNTING.with(|active| active.set(true));
    for tick in 2..62 {
        let report = extractor.extract_adaptive(
            source(1, tick),
            Some(source(1, tick - 1)),
            [0.0; 3],
            |_| panic!("warm adjacent page must patch"),
            |sink| {
                for slot in 0..1_500_u32 {
                    sink.set(slot, 1, u128::from(slot), [tick as f64, 0.0, 0.0], 1);
                }
            },
        );
        assert_eq!(report.accepted, 10_000);
        assert_eq!(report.history_reused, 10_000);
    }
    COUNTING.with(|active| active.set(false));
    assert_eq!(ALLOCATIONS.with(Cell::get), 0);
}
