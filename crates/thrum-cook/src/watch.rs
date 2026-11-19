//! Filesystem-event driven cook cycles.
//!
//! Events are hints, not truth: each batch only names paths for the stat cache to re-check, and a
//! dropped-event (`need_rescan`) notice triggers a full scan. A periodic stat-cache scan also
//! reconciles delayed/missing events and retries files that were unstable during an earlier scan.
//! A batch closes after the tree has been quiet for the debounce interval (or after a maximum
//! wait during continuous writes). Sweeps defer recent writes and both paths retry unstable
//! reads; neither can infer an external editor's complete multi-file save boundary.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant, SystemTime};

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

// Wait after a sweep completes, independently of notice traffic. At most 1/11 of the time is
// spent sweeping, even when a tree is too large for the floor; no additional thread is needed.
const SWEEP_FLOOR: Duration = Duration::from_millis(500);
const SWEEP_MULTIPLIER: u32 = 10;

enum Notice {
    Event(notify::Result<notify::Event>),
    Unavailable(notify::Error),
}

fn sweep_wait(floor: Duration, elapsed: Duration) -> Duration {
    floor.max(elapsed.saturating_mul(SWEEP_MULTIPLIER))
}

/// Run an initial full cook, then cook on source events and periodic reconciliation until
/// `stop` is set. Reconciliation reuses the stat and action caches for unchanged sources.
///
/// Native notice registration runs alongside the cook loop. While it is pending or if it
/// fails, periodic reconciliation still publishes changes.
///
/// # Errors
/// Failure to resolve the source or start the notice worker, or unexpected loss of the worker.
/// Cook failures and native registration errors are delivered through `on_cycle`.
pub fn watch(
    cooker: &mut Cooker,
    source: &std::path::Path,
    stop: &AtomicBool,
    on_cycle: impl FnMut(&WatchEvent),
) -> Result<(), CookError> {
    // FSEvents reports canonical paths (for example /private/var for /var).
    let root = source.canonicalize()?;
    watch_with_registration(cooker, &root, stop, on_cycle, |root, sender| {
        let mut watcher = notify::recommended_watcher(move |event| {
            sender.send(Notice::Event(event)).ok();
        })?;
        watcher.watch(root, RecursiveMode::Recursive)?;
        Ok(watcher)
    })
}

fn watch_with_registration<T>(
    cooker: &mut Cooker,
    root: &std::path::Path,
    stop: &AtomicBool,
    on_cycle: impl FnMut(&WatchEvent),
    register: impl FnOnce(&std::path::Path, mpsc::Sender<Notice>) -> notify::Result<T> + Send + 'static,
) -> Result<(), CookError> {
    let (sender, receiver) = mpsc::channel();
    let (_stop_notices, stopped) = mpsc::channel::<()>();
    let notice_root = root.to_owned();
    // Native registration and teardown can both block on the OS. Dropping
    // the stop sender lets the worker clean up when the OS call returns.
    thread::Builder::new()
        .name("cooker filesystem notices".into())
        .spawn(move || {
            let _watcher = match register(&notice_root, sender.clone()) {
                Ok(watcher) => Some(watcher),
                Err(error) => {
                    sender.send(Notice::Unavailable(error)).ok();
                    None
                }
            };
            // Keep both the watcher and the sender alive until the cook loop exits,
            // including when registration failed and the loop only sweeps.
            stopped.recv().ok();
        })?;
    run_cycles(cooker, root, stop, &receiver, Some(SWEEP_FLOOR), on_cycle).map(|_| ())
}

fn run_cycles(
    cooker: &mut Cooker,
    root: &std::path::Path,
    stop: &AtomicBool,
    receiver: &mpsc::Receiver<Notice>,
    sweep_floor: Option<Duration>,
    mut on_cycle: impl FnMut(&WatchEvent),
) -> Result<usize, CookError> {
    let first_event = Instant::now();
    on_cycle(&WatchEvent {
        first_event,
        paths: 0,
        report: cooker.cook_all(),
    });
    let mut next_sweep = sweep_floor.map(|floor| Instant::now() + floor);
    let mut sweeps = 0;
    while !stop.load(Ordering::Relaxed) {
        let event = match receiver.recv_timeout(Duration::from_millis(50)) {
            Ok(event) => Some(event),
            Err(mpsc::RecvTimeoutError::Timeout) => None,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(CookError::WatchDisconnected);
            }
        };
        if let Some(event) = event {
            let first_event = Instant::now();
            let mut paths: Vec<PathBuf> = Vec::new();
            let mut rescan = false;
            let mut received_notice = false;
            let mut absorb = |notice| match notice {
                Notice::Event(event) => {
                    received_notice = true;
                    match event {
                        Ok(event) => {
                            rescan |= event.need_rescan();
                            paths.extend(event.paths);
                        }
                        Err(_) => rescan = true,
                    }
                }
                Notice::Unavailable(error) => on_cycle(&WatchEvent {
                    first_event,
                    paths: 0,
                    report: Err(CookError::Watch(error)),
                }),
            };
            absorb(event);
            while first_event.elapsed() < MAX_BATCH {
                match receiver.recv_timeout(DEBOUNCE) {
                    Ok(event) => absorb(event),
                    Err(_) => break,
                }
            }
            if received_notice {
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
        }
        if next_sweep.is_some_and(|deadline| Instant::now() >= deadline) {
            let first_event = Instant::now();
            let report = cooker.sweep(SystemTime::now());
            sweeps += 1;
            let elapsed = first_event.elapsed();
            next_sweep = sweep_floor.map(|floor| Instant::now() + sweep_wait(floor, elapsed));
            if let Some(report) = report.transpose() {
                on_cycle(&WatchEvent {
                    first_event,
                    paths: 0,
                    report,
                });
            }
        }
    }
    Ok(sweeps)
}

#[cfg(test)]
mod tests;
