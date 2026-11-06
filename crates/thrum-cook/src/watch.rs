//! Filesystem-event driven cook cycles.
//!
//! Events are hints, not truth: each batch only names paths for the stat cache to re-check, and a
//! dropped-event (`need_rescan`) notice triggers a full scan. A batch closes after the tree has
//! been quiet for the debounce interval (or after a maximum wait during continuous writes), so a
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
    /// When the first filesystem event of the batch arrived.
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

/// Run an initial full cook, then cook on every batch of source changes until `stop` is set.
///
/// # Errors
/// Watcher start-up failure. Cook failures are delivered through `on_cycle`.
pub fn watch(
    cooker: &mut Cooker,
    source: &std::path::Path,
    stop: &AtomicBool,
    mut on_cycle: impl FnMut(&WatchEvent),
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
    let first_event = Instant::now();
    on_cycle(&WatchEvent {
        first_event,
        paths: 0,
        report: cooker.cook_all(),
    });
    while !stop.load(Ordering::Relaxed) {
        let Ok(event) = receiver.recv_timeout(Duration::from_millis(50)) else {
            continue;
        };
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
        absorb(event);
        while first_event.elapsed() < MAX_BATCH {
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
                path.strip_prefix(&root)
                    .ok()
                    .map(std::path::Path::to_path_buf)
            })
            .collect();
        let report = if rescan {
            cooker.cook_all()
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
