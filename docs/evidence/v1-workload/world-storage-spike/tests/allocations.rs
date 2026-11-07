use std::{
    alloc::{GlobalAlloc, Layout, System},
    sync::atomic::{AtomicU64, Ordering},
};
use thrum_world_storage_spike::{Adapter, Bevy, Custom, Hecs};

struct Count;
static CALLS: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);

unsafe impl GlobalAlloc for Count {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        CALLS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        unsafe { System.alloc(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, old: Layout, new_size: usize) -> *mut u8 {
        CALLS.fetch_add(1, Ordering::Relaxed);
        BYTES.fetch_add(new_size as u64, Ordering::Relaxed);
        unsafe { System.realloc(ptr, old, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOC: Count = Count;

fn sample<A: Adapter>(name: &str) -> u64 {
    let mut world = A::new();
    for _ in 0..600 {
        world.tick();
    }
    let calls = CALLS.load(Ordering::Relaxed);
    let bytes = BYTES.load(Ordering::Relaxed);
    for _ in 0..600 {
        world.tick();
    }
    let delta = CALLS.load(Ordering::Relaxed) - calls;
    let bytes = BYTES.load(Ordering::Relaxed) - bytes;
    println!(
        "{name}: 600 warm ticks then 600 measured; allocation calls={delta}, requested bytes={bytes}"
    );
    delta
}

#[test]
fn steady_tick_allocations() {
    // One integration test keeps the process-global counter isolated from parallel tests.
    assert_eq!(sample::<Custom>("custom"), 0);
    assert_eq!(sample::<Hecs>("hecs"), 0);
    assert_eq!(sample::<Bevy>("bevy"), 0);
}
