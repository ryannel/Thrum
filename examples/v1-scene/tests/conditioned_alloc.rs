//! Zero-allocation assertion for warmed four-peer conditioner and W1 ticks.

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

use v1_scene::conditioned::ConditionedGame;

thread_local! {
    static TRACK: Cell<bool> = const { Cell::new(false) };
    static ALLOCS: Cell<u64> = const { Cell::new(0) };
}

struct CountingAllocator;

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

fn count() {
    let _ = TRACK.try_with(|active| {
        if active.get() {
            let _ = ALLOCS.try_with(|value| value.set(value.get() + 1));
        }
    });
}

#[expect(
    unsafe_code,
    reason = "test-only allocator observes warmed four-peer ticks"
)]
// SAFETY: The allocator forwards every operation and layout unchanged to System.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();
        // SAFETY: The layout is forwarded unchanged.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count();
        // SAFETY: The layout is forwarded unchanged.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        count();
        // SAFETY: Pointer, layout and size are forwarded unchanged.
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: Pointer and layout came from System.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[test]
fn warmed_conditioned_world_ticks_make_no_rust_heap_allocations() {
    let mut game = ConditionedGame::new().expect("prepared four-peer game");
    for _ in 0..10 {
        game.step().expect("warm tick");
    }
    ALLOCS.with(|value| value.set(0));
    TRACK.with(|active| active.set(true));
    for _ in 0..500 {
        game.step().expect("measured tick");
    }
    TRACK.with(|active| active.set(false));
    assert_eq!(ALLOCS.with(Cell::get), 0);
    for _ in 0..90 {
        game.step().expect("remaining tick");
    }
    game.finish().expect("independent controlled-pose trace");
}
