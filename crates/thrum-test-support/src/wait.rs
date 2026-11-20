//! Wait for a condition a test cannot block on, with a deadline.

use std::time::{Duration, Instant};

/// How long [`until`] sleeps between attempts.
pub const INTERVAL: Duration = Duration::from_millis(1);

/// Call `attempt` until it returns a value, sleeping [`INTERVAL`] between calls, and fail the test once `limit`
/// has passed. `what` names the awaited event in the failure.
///
/// The last attempt runs after the deadline, so a value that arrived while the thread slept is still taken.
///
/// # Panics
/// Fails with `what` when no attempt returned a value within `limit`.
pub fn until<T>(what: &str, limit: Duration, mut attempt: impl FnMut() -> Option<T>) -> T {
    let deadline = Instant::now() + limit;
    loop {
        let expired = Instant::now() >= deadline;
        if let Some(value) = attempt() {
            return value;
        }
        assert!(!expired, "{what}: not reached within {limit:?}");
        std::thread::sleep(INTERVAL);
    }
}
