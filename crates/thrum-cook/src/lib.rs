//! Authoring-only side of Thrum's content-addressed asset loop. Shipping builds never link this
//! package; they read its output through `thrum-asset`.
//!
//! One cook cycle: a [`StatCache`] scan finds changed sources (hashing only files whose stamp
//! changed), each asset's action key (cooker version, profile, settings and exact input hashes)
//! is looked up in the action cache, misses are cooked in parallel into the BLAKE3 [`Cas`], and a
//! new manifest is published by atomically replacing `HEAD`. [`watch`] drives cycles from
//! filesystem events, and [`DevServer`] serves the store to browsers and remote devices.

mod cas;
mod cooker;
pub mod demo;
mod mesh;
mod server;
mod stat_cache;
mod texture;
mod watch;
mod write;

use std::fmt;

pub use cas::{Cas, Durability};
pub use cooker::{CookConfig, CookReport, Cooker};
pub use server::{DevServer, Publications};
pub use stat_cache::{RACY_WINDOW_NS, ScanCounts, StatCache};
pub use texture::{Quality, encode_volume};
pub use watch::{DEBOUNCE, MAX_BATCH, WatchEvent, watch};

/// Failure of the cooker itself. Per-asset failures are reported in [`CookReport::failed`].
#[derive(Debug)]
pub enum CookError {
    /// Filesystem failure.
    Io(std::io::Error),
    /// A source file could not be imported.
    Source(String),
    /// A cooked product failed runtime validation.
    Product(thrum_asset::AssetError),
    /// The filesystem watcher failed.
    Watch(String),
}

impl fmt::Display for CookError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "cook I/O failed: {error}"),
            Self::Source(reason) => write!(f, "source import failed: {reason}"),
            Self::Product(error) => write!(f, "cooked product refused: {error}"),
            Self::Watch(reason) => write!(f, "watcher failed: {reason}"),
        }
    }
}

impl std::error::Error for CookError {}

impl From<std::io::Error> for CookError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<thrum_asset::AssetError> for CookError {
    fn from(error: thrum_asset::AssetError) -> Self {
        Self::Product(error)
    }
}
