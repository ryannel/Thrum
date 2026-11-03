//! Fixture correctness and zero-allocation assertion for the representative workload.
// The test allocator only observes this thread while explicitly armed. GlobalAlloc
// requires unsafe delegation to System; production thrum-core still denies unsafe.
#![allow(
    unsafe_code,
    reason = "test-only counting allocator delegates to System"
)]

#[path = "support/v1_workload.rs"]
mod support;

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<u64> = const { Cell::new(0) };
}

struct CountingAllocator;

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

// SAFETY: Each allocator operation delegates to System with the same arguments.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        COUNTING.with(|active| {
            if active.get() {
                ALLOCATIONS.with(|count| count.set(count.get() + 1));
            }
        });
        // SAFETY: The layout is passed through unchanged to System.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: The pointer and layout came from the matching System allocation.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[test]
fn representative_scaffold_is_deterministic_and_allocation_free_after_setup() {
    let mut fixture = support::Workload::new();
    assert_eq!(fixture.entity_count(), 10_000);
    for _ in 0..60 {
        fixture.step();
    }
    let start_position = fixture.first_position();
    let start_churn = fixture.churned;
    // Initialize TLS before arming so that the measured scope contains only steps.
    ALLOCATIONS.with(|count| count.set(0));
    COUNTING.with(|active| active.set(true));
    for _ in 0..600 {
        fixture.step();
    }
    COUNTING.with(|active| active.set(false));
    assert_eq!(ALLOCATIONS.with(Cell::get), 0);
    assert!(fixture.first_position() > start_position);
    assert_eq!(fixture.churned - start_churn, 1_000);
    assert!(fixture.link.dropped > 0);
    assert!(fixture.link.delivered.iter().all(|count| *count > 0));
    assert_eq!(fixture.link.min_one_way_ticks, 2);
    assert_eq!(fixture.link.max_one_way_ticks, 4);
    let sent = fixture.tick * support::PEER_COUNT as u64;
    let delivered: u64 = fixture.link.delivered.iter().sum();
    assert_eq!(
        delivered + fixture.link.dropped + fixture.link.in_flight() as u64,
        sent
    );
    assert!(fixture.link.dropped * 100 > sent);
    assert!(fixture.link.dropped * 100 < sent * 3);
}
