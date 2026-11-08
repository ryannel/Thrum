//! Test-only cap on GPU tests that hold timestamp query sets at once.
//!
//! On the Metal runtime measured here (wgpu 30.0.1, Apple GPU), a process could hold 32 live
//! timestamp counter sample buffers, shared by every wgpu device in it; creating the 33rd lost the
//! device that asked for it. Other backends and devices are unqualified. Each frame host holds one
//! timestamp query set per in-flight slot, and the heaviest lib test (host, detached candidate and
//! replacement) holds 8 at once, so lib tests run with at most [`CONCURRENT`] timestamp tests live.

use std::cell::RefCell;
use std::sync::{Condvar, Mutex, PoisonError};

/// Timestamp-holding tests that may run at once; with the heaviest holding 8 sets, 24 of the
/// measured 32.
const CONCURRENT: usize = 3;

static ACTIVE: Mutex<usize> = Mutex::new(0);
static RELEASED: Condvar = Condvar::new();

struct Permit;

impl Drop for Permit {
    fn drop(&mut self) {
        let mut active = ACTIVE.lock().unwrap_or_else(PoisonError::into_inner);
        *active -= 1;
        RELEASED.notify_one();
    }
}

std::thread_local! {
    /// Held by the test's thread until it exits, so the permit outlives every host, detached
    /// candidate and replacement the test creates, including on a panic.
    static HELD: RefCell<Option<Permit>> = const { RefCell::new(None) };
}

/// Wait for a timestamp permit for the calling test thread; later calls on that thread reuse it.
pub(crate) fn hold() {
    HELD.with(|held| {
        let mut held = held.borrow_mut();
        if held.is_some() {
            return;
        }
        let mut active = ACTIVE.lock().unwrap_or_else(PoisonError::into_inner);
        while *active >= CONCURRENT {
            active = RELEASED
                .wait(active)
                .unwrap_or_else(PoisonError::into_inner);
        }
        *active += 1;
        *held = Some(Permit);
    });
}
