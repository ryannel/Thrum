//! Watch a creator's parameter and game files and wake the event loop when a save settles, so
//! it runs the same reload as `L`. A polling thread reads each file's metadata every tick and
//! its bytes only when that changes, so in-place saves, rename-saves, symlinks and files
//! missing at start need no special cases and no platform watcher. The thread never touches
//! the frame path: it reads at most a few small files and sets bits that the event loop takes.

use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
        mpsc::{self, RecvTimeoutError},
    },
    thread::JoinHandle,
    time::{Duration, Instant, SystemTime},
};

/// Interval between metadata reads. A save is delivered after its bytes hold for one tick.
pub(crate) const TICK: Duration = Duration::from_millis(100);
/// How long a file may stay missing, as in a delete-then-create save, before the reload runs
/// and reports it.
pub(crate) const MISSING_GRACE: Duration = Duration::from_millis(500);

/// Which kinds of file changed since the event loop last took them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Kinds(u8);

impl Kinds {
    /// The parameter file.
    pub(crate) const PARAMS: Self = Self(1);
    /// The game's own files, which [`crate::Game::reload`] reads.
    pub(crate) const GAME: Self = Self(2);
    /// Everything `L` reloads.
    pub(crate) const ALL: Self = Self(3);

    pub(crate) const fn params(self) -> bool {
        self.0 & Self::PARAMS.0 != 0
    }

    pub(crate) const fn game(self) -> bool {
        self.0 & Self::GAME.0 != 0
    }

    pub(crate) const fn is_empty(self) -> bool {
        self.0 == 0
    }
}

/// What a reload acts on: the window's app or a live capture.
pub(crate) trait Reload {
    /// The renderer has stopped or the run is exiting, so a late save is dropped.
    fn stopped(&self) -> bool;
    /// Read and apply the parameter file, reporting what happened; `saved` says a save, not
    /// `L`, asked for it.
    /// # Errors
    /// A failure that ends the run; a bad file is reported, not returned.
    fn reload_params(&mut self, saved: bool) -> Result<(), String>;
    /// Run the game's reload of its own files.
    fn reload_game(&mut self);
    /// Request a frame so the result is drawn.
    fn redraw(&self);
}

/// The one reload body behind `L` and saved files: the parameter file if it changed, then the
/// game's files if they changed, then a frame. A parameter-only save never reloads the game.
pub(crate) fn deliver(host: &mut impl Reload, kinds: Kinds, saved: bool) -> Result<(), String> {
    if host.stopped() || kinds.is_empty() {
        return Ok(());
    }
    if kinds.params() {
        host.reload_params(saved)?;
    }
    if kinds.game() {
        host.reload_game();
    }
    host.redraw();
    Ok(())
}

/// One watched file: its metadata at the last tick, the bytes read then, and the bytes last
/// delivered (`None` while missing).
struct Watched {
    path: PathBuf,
    kind: Kinds,
    stamp: Option<Stamp>,
    seen: Option<Vec<u8>>,
    seen_at: Instant,
    delivered: Option<Vec<u8>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct Stamp {
    len: u64,
    modified: Option<SystemTime>,
    inode: u64,
}

fn stamp(path: &Path) -> Option<Stamp> {
    use std::os::unix::fs::MetadataExt;
    let metadata = std::fs::metadata(path).ok()?;
    Some(Stamp {
        len: metadata.len(),
        modified: metadata.modified().ok(),
        inode: metadata.ino(),
    })
}

impl Watched {
    /// Baseline the file as it is now, so only later saves are delivered.
    fn new(path: PathBuf, kind: Kinds, now: Instant) -> Self {
        let stamp = stamp(&path);
        let bytes = stamp.and_then(|_| std::fs::read(&path).ok());
        Self {
            path,
            kind,
            stamp,
            seen: bytes.clone(),
            seen_at: now,
            delivered: bytes,
        }
    }

    /// Whether the file settled on bytes other than those last delivered.
    fn poll(&mut self, now: Instant) -> bool {
        let stamp = stamp(&self.path);
        if stamp != self.stamp {
            // Still changing: read now and deliver once it holds for a tick.
            self.stamp = stamp;
            self.seen = stamp.and_then(|_| std::fs::read(&self.path).ok());
            self.seen_at = now;
            return false;
        }
        if self.seen == self.delivered
            || (self.seen.is_none() && now.duration_since(self.seen_at) < MISSING_GRACE)
        {
            return false;
        }
        self.delivered.clone_from(&self.seen);
        true
    }
}

/// A running watch. Dropping it stops the thread and waits for it, after which the wake is
/// never called again.
pub(crate) struct FileWatch {
    pending: Arc<AtomicU8>,
    stop: Option<mpsc::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl FileWatch {
    /// Baseline `files` on the calling thread, then watch them every `tick`, calling `wake`
    /// after each settled change. Returns `None` when there is nothing to watch.
    /// # Errors
    /// The thread could not be spawned.
    pub(crate) fn start(
        files: Vec<(PathBuf, Kinds)>,
        tick: Duration,
        wake: Box<dyn Fn() + Send>,
    ) -> std::io::Result<Option<Self>> {
        if files.is_empty() {
            return Ok(None);
        }
        let now = Instant::now();
        let mut watched: Vec<Watched> = files
            .into_iter()
            .map(|(path, kind)| Watched::new(path, kind, now))
            .collect();
        let pending = Arc::new(AtomicU8::new(0));
        let shared = Arc::clone(&pending);
        let (stop, stopped) = mpsc::channel::<()>();
        let thread = std::thread::Builder::new()
            .name("thrum-file-watch".to_owned())
            .spawn(move || {
                while let Err(RecvTimeoutError::Timeout) = stopped.recv_timeout(tick) {
                    let now = Instant::now();
                    let changed = watched
                        .iter_mut()
                        .filter_map(|file| file.poll(now).then_some(file.kind.0))
                        .fold(0, |all, kind| all | kind);
                    if changed != 0 {
                        shared.fetch_or(changed, Ordering::AcqRel);
                        wake();
                    }
                }
            })?;
        Ok(Some(Self {
            pending,
            stop: Some(stop),
            thread: Some(thread),
        }))
    }

    /// Take the kinds changed since the last call.
    pub(crate) fn take(&self) -> Kinds {
        Kinds(self.pending.swap(0, Ordering::AcqRel))
    }

    /// Stop the thread without waiting for a tick and join it.
    pub(crate) fn stop(&mut self) {
        drop(self.stop.take());
        if let Some(thread) = self.thread.take() {
            drop(thread.join());
        }
    }
}

impl Drop for FileWatch {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
#[path = "file_watch_tests.rs"]
mod tests;
