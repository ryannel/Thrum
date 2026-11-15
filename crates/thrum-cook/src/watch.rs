//! Filesystem-event driven cook cycles.
//!
//! Events are hints, not truth: each batch only names paths for the stat cache to re-check, and a
//! dropped-event (`need_rescan`) notice triggers a full scan. A periodic stat-cache scan also
//! reconciles delayed/missing events and retries files that were unstable during an earlier scan.
//! A batch closes after the tree has been quiet for the debounce interval (or after a maximum
//! wait during continuous writes), so a
//! file still being written is seen as unstable and retried rather than cooked half-written.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use notify::{RecursiveMode, Watcher};

use crate::{CookError, CookReport, Cooker};

/// One completed watch cycle.
#[derive(Debug)]
pub struct WatchEvent {
    /// When the first event arrived, or when a periodic reconciliation began.
    pub first_event: Instant,
    /// Number of distinct paths in the batch.
    pub paths: usize,
    /// Result of the cook cycle.
    pub report: Result<CookReport, CookError>,
}

/// Quiet interval that closes a batch.
pub const DEBOUNCE: Duration = Duration::from_millis(8);
/// Longest a batch waits for quiet during continuous writes.
pub const MAX_BATCH: Duration = Duration::from_millis(250);

// Authoring-only fallback: event batches keep their incremental fast path. At most once every
// two seconds, walk metadata to recover changes the backend never delivered. The stat and action
// caches reuse unchanged hashes and products; no additional watcher thread or runtime work.
const RECONCILE: Duration = Duration::from_secs(2);

/// Run an initial full cook, then cook on every batch of source changes until `stop` is set.
///
/// # Errors
/// Watcher start-up failure or event-stream disconnection. Cook failures are delivered through
/// `on_cycle`.
pub fn watch(
    cooker: &mut Cooker,
    source: &std::path::Path,
    stop: &AtomicBool,
    on_cycle: impl FnMut(&WatchEvent),
) -> Result<(), CookError> {
    let (sender, receiver) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |event| {
        // The receiver only disappears when watching stops.
        sender.send(event).ok();
    })
    .map_err(|error| CookError::Watch(error.to_string()))?;
    // FSEvents reports canonical paths (for example /private/var for /var).
    let root = source.canonicalize()?;
    watcher
        .watch(&root, RecursiveMode::Recursive)
        .map_err(|error| CookError::Watch(error.to_string()))?;
    run_cycles(cooker, &root, stop, &receiver, on_cycle)
}

fn run_cycles(
    cooker: &mut Cooker,
    root: &std::path::Path,
    stop: &AtomicBool,
    receiver: &mpsc::Receiver<notify::Result<notify::Event>>,
    mut on_cycle: impl FnMut(&WatchEvent),
) -> Result<(), CookError> {
    let first_event = Instant::now();
    on_cycle(&WatchEvent {
        first_event,
        paths: 0,
        report: cooker.cook_all(),
    });
    let mut reconciled = Instant::now();
    while !stop.load(Ordering::Relaxed) {
        let event = match receiver.recv_timeout(Duration::from_millis(50)) {
            Ok(event) => Some(event),
            Err(mpsc::RecvTimeoutError::Timeout) => None,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(CookError::Watch("event stream disconnected".into()));
            }
        };
        if event.is_none() && reconciled.elapsed() < RECONCILE {
            continue;
        }
        let first_event = Instant::now();
        let mut paths: Vec<PathBuf> = Vec::new();
        let mut rescan = false;
        let mut absorb = |event: notify::Result<notify::Event>| match event {
            Ok(event) => {
                rescan |= event.need_rescan();
                paths.extend(event.paths);
            }
            Err(_) => rescan = true,
        };
        let has_event = event.is_some();
        if let Some(event) = event {
            absorb(event);
        }
        while has_event && first_event.elapsed() < MAX_BATCH {
            match receiver.recv_timeout(DEBOUNCE) {
                Ok(event) => absorb(event),
                Err(_) => break,
            }
        }
        paths.sort();
        paths.dedup();
        let relative: Vec<PathBuf> = paths
            .iter()
            .filter_map(|path| {
                path.strip_prefix(root)
                    .ok()
                    .map(std::path::Path::to_path_buf)
            })
            .collect();
        let report = if rescan || reconciled.elapsed() >= RECONCILE {
            let report = cooker.cook_all();
            reconciled = Instant::now();
            report
        } else {
            cooker.cook_changed(&relative)
        };
        on_cycle(&WatchEvent {
            first_event,
            paths: relative.len(),
            report,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
