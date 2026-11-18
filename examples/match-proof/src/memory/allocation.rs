//! Thread-local allocation accounting for warmed proof advances.
#![expect(
    unsafe_code,
    reason = "The proof-only allocator forwards allocations to System"
)]

use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
};

thread_local! {
    static COUNT: Cell<Option<(u64, u64)>> = const { Cell::new(None) };
}

struct Counter;

#[global_allocator]
static ALLOCATOR: Counter = Counter;

fn count(size: usize) {
    let _ = COUNT.try_with(|counter| {
        if let Some((calls, bytes)) = counter.get() {
            counter.set(Some((calls + 1, bytes + size as u64)));
        }
    });
}

// SAFETY: All operations forward the original storage and layout unchanged to System.
unsafe impl GlobalAlloc for Counter {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count(layout.size());
        // SAFETY: System receives the caller's original layout.
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        count(layout.size());
        // SAFETY: System receives the caller's original layout.
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count(size);
        // SAFETY: The pointer and layout came from System.
        unsafe { System.realloc(ptr, layout, size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: The pointer and layout came from System.
        unsafe { System.dealloc(ptr, layout) }
    }
}

pub(super) fn begin() {
    COUNT.set(Some((0, 0)));
}

pub(super) fn finish() -> (u64, u64) {
    COUNT.replace(None).expect("allocation sample started")
}
