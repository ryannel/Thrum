//! Count heap allocations made on one thread.
//!
//! A `#[global_allocator]` cannot live in a package every test links, so a test binary that counts declares its
//! own: `#[global_allocator] static ALLOCATOR: CountingAllocator = CountingAllocator;`. Counting is off until a
//! thread calls [`count`], and only that thread's allocations are counted.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static COUNT: Cell<u64> = const { Cell::new(0) };
    static BYTES: Cell<u64> = const { Cell::new(0) };
    static LARGEST: Cell<u64> = const { Cell::new(0) };
}

/// Allocations counted on one thread. A reallocation counts as one allocation of its new size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Allocations {
    /// Calls to allocate, allocate zeroed or reallocate.
    pub count: u64,
    /// Bytes those calls asked for.
    pub bytes: u64,
    /// The most bytes one of those calls asked for.
    pub largest: u64,
}

/// The system allocator, counting allocations on a thread inside [`count`].
#[derive(Clone, Copy, Debug, Default)]
pub struct CountingAllocator;

fn record(size: usize) {
    // `try_with` fails only while the thread's locals are torn down, when nothing is being counted.
    COUNTING
        .try_with(|counting| {
            if counting.get() {
                COUNT
                    .try_with(|count| count.set(count.get() + 1))
                    .unwrap_or_default();
                BYTES
                    .try_with(|bytes| bytes.set(bytes.get() + size as u64))
                    .unwrap_or_default();
                LARGEST
                    .try_with(|largest| largest.set(largest.get().max(size as u64)))
                    .unwrap_or_default();
            }
        })
        .unwrap_or_default();
}

#[expect(
    unsafe_code,
    reason = "a global allocator is an unsafe trait; this one only forwards to the system allocator"
)]
// SAFETY: Every operation and layout is forwarded unchanged to the system allocator.
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        // SAFETY: The caller's layout is forwarded unchanged.
        unsafe { System.alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record(layout.size());
        // SAFETY: The caller's layout is forwarded unchanged.
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        record(new_size);
        // SAFETY: The pointer and layout came from the system allocator; the new size is forwarded unchanged.
        unsafe { System.realloc(ptr, layout, new_size) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: The pointer and layout came from the system allocator.
        unsafe { System.dealloc(ptr, layout) }
    }
}

/// Turns counting off when [`count`] returns or unwinds.
struct Stop;

impl Drop for Stop {
    fn drop(&mut self) {
        COUNTING.with(|counting| counting.set(false));
    }
}

/// Run `work` and return what it allocated on the calling thread. Counts nothing unless the test binary's global
/// allocator is a [`CountingAllocator`].
///
/// # Panics
/// Fails if called again inside `work`.
pub fn count<R>(work: impl FnOnce() -> R) -> (R, Allocations) {
    assert!(!COUNTING.with(Cell::get), "allocation counts do not nest");
    COUNT.with(|count| count.set(0));
    BYTES.with(|bytes| bytes.set(0));
    LARGEST.with(|largest| largest.set(0));
    COUNTING.with(|counting| counting.set(true));
    let stop = Stop;
    let result = work();
    drop(stop);
    let counted = Allocations {
        count: COUNT.with(Cell::get),
        bytes: BYTES.with(Cell::get),
        largest: LARGEST.with(Cell::get),
    };
    (result, counted)
}
