use std::path::{Path, PathBuf};

use crate::layout::HEAD_LEN;
use crate::{AssetError, ContentHash, Head, Manifest, ProductBytes};

/// A content-addressed product store on the local filesystem.
///
/// Layout: `HEAD` (the 64-byte current publication) and `cas/<first two hex>/<64 hex>` immutable
/// products. The runtime only reads; the cooker is the single writer.
#[derive(Clone, Debug)]
pub struct AssetStore {
    root: PathBuf,
}

impl AssetStore {
    /// Name of the publication record inside the store root.
    pub const HEAD_FILE: &'static str = "HEAD";

    /// Refer to a store root; nothing is read until asked.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Store root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Path of a product named by `hash`.
    pub fn product_path(&self, hash: &ContentHash) -> PathBuf {
        let hex = hash.to_hex();
        self.root.join("cas").join(&hex[..2]).join(hex)
    }

    /// Path of the `HEAD` record.
    pub fn head_path(&self) -> PathBuf {
        self.root.join(Self::HEAD_FILE)
    }

    /// Read the current publication, or `None` before the first publish.
    ///
    /// # Errors
    /// I/O failures other than absence, and malformed records.
    pub fn read_head(&self) -> Result<Option<Head>, AssetError> {
        match std::fs::read(self.head_path()) {
            Ok(bytes) => Head::decode(&bytes).map(Some),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    /// Map a product read-only. Structure is checked by the typed view that consumes it.
    ///
    /// # Errors
    /// I/O failure, including a missing product.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn map(&self, hash: &ContentHash) -> Result<ProductBytes, AssetError> {
        let file = std::fs::File::open(self.product_path(hash))?;
        Ok(ProductBytes::map_immutable(&file)?)
    }

    /// Map and validate the manifest a publication selects.
    ///
    /// # Errors
    /// I/O failure, a malformed manifest, or a manifest whose generation disagrees with `head`.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_manifest(&self, head: &Head) -> Result<Manifest, AssetError> {
        let manifest = Manifest::parse(self.map(&head.manifest)?)?;
        if manifest.generation() == head.generation {
            Ok(manifest)
        } else {
            Err(AssetError::Malformed(
                "manifest generation differs from HEAD",
            ))
        }
    }
}

/// Cheap change detector for a running game: one 64-byte read per poll.
///
/// The game's loader thread calls [`HeadWatcher::poll`] at its own cadence (for example once per
/// frame or every few milliseconds). It never touches the render thread.
#[derive(Debug)]
pub struct HeadWatcher {
    store: AssetStore,
    seen: Option<u64>,
}

impl HeadWatcher {
    /// Watch a store; the first poll reports its current publication.
    pub fn new(store: AssetStore) -> Self {
        Self { store, seen: None }
    }

    /// The new publication, when one has replaced the last one reported.
    ///
    /// # Errors
    /// As [`AssetStore::read_head`]. A refused record leaves the watcher's state unchanged.
    pub fn poll(&mut self) -> Result<Option<Head>, AssetError> {
        let Some(head) = self.store.read_head()? else {
            return Ok(None);
        };
        if self.seen.is_some_and(|seen| head.generation <= seen) {
            return Ok(None);
        }
        self.seen = Some(head.generation);
        Ok(Some(head))
    }

    /// Store being watched.
    pub fn store(&self) -> &AssetStore {
        &self.store
    }
}

const _: () = assert!(HEAD_LEN == 64);
