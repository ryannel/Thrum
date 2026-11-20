//! A scratch folder that removes itself.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// A new, empty folder under the system's temporary folder, removed with its contents when dropped, including
/// when the test panics.
#[derive(Debug)]
pub struct Scratch {
    path: PathBuf,
}

impl Scratch {
    /// Create a folder whose name starts with `name`, unique to this process and call.
    ///
    /// # Panics
    /// Fails if the folder cannot be created.
    #[must_use]
    pub fn new(name: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "thrum-{name}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        // A folder left by an earlier process with the same id is stale.
        if path.exists() {
            std::fs::remove_dir_all(&path).expect("remove stale scratch folder");
        }
        std::fs::create_dir_all(&path).expect("create scratch folder");
        Self { path }
    }

    /// The folder.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// A path inside the folder.
    #[must_use]
    pub fn join(&self, relative: impl AsRef<Path>) -> PathBuf {
        self.path.join(relative)
    }
}

impl AsRef<Path> for Scratch {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        // While a test unwinds its own failure is the one to report; a second panic would abort the run.
        if let Err(error) = std::fs::remove_dir_all(&self.path)
            && !std::thread::panicking()
        {
            panic!(
                "scratch folder {} not removed: {error}",
                self.path.display()
            );
        }
    }
}
