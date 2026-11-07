//! Single-writer content-addressed product store and atomic `HEAD` publication.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use thrum_asset::{AssetStore, ContentHash, Head};

/// How strongly a write must survive failure.
///
/// Every mode keeps process-crash atomicity: products and `HEAD` become visible only by rename
/// after their bytes are complete, so a reader never sees a partial product or a `HEAD` naming a
/// product that is not yet in place. `Power` additionally syncs file data and the containing
/// directory (a full flush on macOS), which costs milliseconds per publish. Dev stores are
/// derived and rebuildable, so the edit loop uses `Process`; the cooker re-verifies the published
/// closure when it opens a store.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Durability {
    /// Atomic visibility only.
    Process,
    /// Atomic visibility plus data and directory sync before each rename completes.
    Power,
}

/// Writer side of an [`AssetStore`]. One cooker owns a store at a time.
#[derive(Debug)]
pub struct Cas {
    store: AssetStore,
    durability: Durability,
    epoch: u32,
    /// Exclusive writer lock on the store, held for this writer's lifetime.
    _lock: fs::File,
}

static TEMPORARY: AtomicU64 = AtomicU64::new(0);

impl Cas {
    /// Open (creating directories as needed) a store root for writing.
    ///
    /// # Errors
    /// I/O failure creating the store layout, or another writer already holds the store.
    pub fn open(root: impl Into<PathBuf>, durability: Durability) -> std::io::Result<Self> {
        let store = AssetStore::new(root);
        fs::create_dir_all(store.root().join("cas").join("tmp"))?;
        // One writer per store: two publishers would both claim the next generation.
        let lock = fs::File::options()
            .create(true)
            .truncate(false)
            .write(true)
            .open(store.root().join("LOCK"))?;
        lock.try_lock().map_err(|error| match error {
            fs::TryLockError::WouldBlock => {
                std::io::Error::other("store is locked by another cooker")
            }
            fs::TryLockError::Error(error) => error,
        })?;
        let epoch = Self::epoch_of(&store)?;
        Ok(Self {
            store,
            durability,
            epoch,
            _lock: lock,
        })
    }

    /// The store instance's epoch, created with the store and written into every `HEAD`.
    fn epoch_of(store: &AssetStore) -> std::io::Result<u32> {
        let path = store.root().join("EPOCH");
        if let Ok(bytes) = fs::read(&path)
            && let Ok(bytes) = <[u8; 4]>::try_from(bytes.as_slice())
        {
            return Ok(u32::from_le_bytes(bytes));
        }
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos());
        let seed = blake3::hash(
            &[
                nanos.to_le_bytes().as_slice(),
                &std::process::id().to_le_bytes(),
                store.root().as_os_str().as_encoded_bytes(),
            ]
            .concat(),
        );
        let epoch = u32::from_le_bytes(seed.as_bytes()[..4].try_into().expect("four bytes"));
        fs::write(&path, epoch.to_le_bytes())?;
        Ok(epoch)
    }

    /// Epoch to record in this store's publications.
    pub fn epoch(&self) -> u32 {
        self.epoch
    }

    /// Reader view of the same store.
    pub fn store(&self) -> &AssetStore {
        &self.store
    }

    /// Whether a product is already stored.
    pub fn contains(&self, hash: &ContentHash) -> bool {
        self.store.product_path(hash).is_file()
    }

    /// Store bytes under their BLAKE3 name; identical bytes are stored once.
    ///
    /// # Errors
    /// I/O failure writing or renaming.
    pub fn put(&self, bytes: &[u8]) -> std::io::Result<ContentHash> {
        let hash = ContentHash::of(bytes);
        let path = self.store.product_path(&hash);
        // An existing file is trusted only if its bytes still match its name.
        if path.is_file() {
            if fs::read(&path).is_ok_and(|existing| hash.verify(&existing).is_ok()) {
                return Ok(hash);
            }
            fs::remove_file(&path)?;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        self.write_atomically(&path, bytes, true)?;
        Ok(hash)
    }

    /// Atomically select a new manifest.
    ///
    /// # Errors
    /// I/O failure; the previous `HEAD` stays selected.
    pub fn publish(&self, head: &Head) -> std::io::Result<()> {
        self.write_atomically(&self.store.head_path(), &head.encode(), false)
    }

    fn write_atomically(&self, path: &Path, bytes: &[u8], read_only: bool) -> std::io::Result<()> {
        let serial = TEMPORARY.fetch_add(1, Ordering::Relaxed);
        let temporary = self
            .store
            .root()
            .join("cas")
            .join("tmp")
            .join(format!("{}-{serial}", std::process::id()));
        let result = (|| {
            let mut file = fs::File::create(&temporary)?;
            file.write_all(bytes)?;
            if self.durability == Durability::Power {
                file.sync_all()?;
            }
            if read_only {
                let mut permissions = file.metadata()?.permissions();
                permissions.set_readonly(true);
                fs::set_permissions(&temporary, permissions)?;
            }
            drop(file);
            fs::rename(&temporary, path)?;
            if self.durability == Durability::Power
                && let Some(parent) = path.parent()
            {
                fs::File::open(parent)?.sync_all()?;
            }
            Ok(())
        })();
        if result.is_err() {
            // Ignore a missing temporary; the original error is the one worth reporting.
            fs::remove_file(&temporary).ok();
        }
        result
    }
}
