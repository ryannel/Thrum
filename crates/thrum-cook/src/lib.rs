//! Authoring-only side of Thrum's content-addressed asset loop. Shipping builds never link this
//! package; they read its output through `thrum-asset`.
//!
//! One cook cycle: a [`StatCache`] scan finds changed sources (hashing only files whose stamp
//! changed), each asset's action key (cooker version, profile, settings and exact input hashes)
//! is looked up in the action cache, misses are cooked in parallel into the BLAKE3 [`Cas`], and a
//! new manifest is published by atomically replacing `HEAD`. [`watch`] drives cycles from
//! filesystem events, and [`DevServer`] serves the store to browsers and remote devices.

mod error;
pub use error::{OriginError, TextureEncodeError, TripwireError};
mod cas;
mod cooker;
pub mod demo;
mod font;
mod mesh;
mod server;
mod sheet;
mod stat_cache;
mod texture;
pub mod tripwire;
mod watch;
mod write;

use std::fmt;

pub use cas::{Cas, Durability};
pub use cooker::{CookConfig, CookReport, Cooker};
pub use server::{AllowedOrigins, DevServer, Publications};
pub use stat_cache::{RACY_WINDOW_NS, ScanCounts, StatCache};
pub use texture::{Quality, encode_volume};
pub use watch::{DEBOUNCE, MAX_BATCH, WatchEvent, watch};

/// Failure of the cooker itself. Per-asset failures are reported in [`CookReport::failed`].
#[derive(Debug)]
pub enum CookError {
    /// Retry the source on the next sweep; it changed after planning.
    SourceChanged,
    /// Filesystem failure.
    Io(std::io::Error),
    /// A source file could not be imported.
    Source(String),
    /// A mesh source could not be parsed or its buffers imported.
    MeshImport {
        /// Source path when the import is from a file rather than planned bytes.
        path: Option<std::path::PathBuf>,
        /// Original glTF parser or importer failure.
        source: gltf::Error,
    },
    /// An authored texture could not be decoded.
    ImageImport(image::ImageError),
    /// A texture encoder refused its configuration or input.
    TextureEncode(TextureEncodeError),
    /// A cooked product failed runtime validation.
    Product(thrum_asset::AssetError),
    /// The filesystem watcher failed.
    Watch(notify::Error),
    /// The filesystem event stream ended; restart the watcher.
    WatchDisconnected,
    /// A profile cannot represent this source, so the asset publishes without that profile's
    /// product and loaders select its portable one.
    Refused(String),
}

impl fmt::Display for CookError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceChanged => {
                f.write_str("source changed during cook; retry on the next sweep")
            }
            Self::Io(error) => write!(f, "cook I/O failed: {error}"),
            Self::Source(reason) => write!(f, "source import failed: {reason}"),
            Self::MeshImport { path, source } => {
                if let Some(path) = path {
                    write!(f, "source import failed: {}: {source}", path.display())
                } else {
                    write!(f, "source import failed: {source}")
                }
            }
            Self::ImageImport(source) => write!(f, "source image import failed: {source}"),
            Self::TextureEncode(source) => write!(f, "texture encode failed: {source}"),
            Self::Product(error) => write!(f, "cooked product refused: {error}"),
            Self::Watch(reason) => write!(f, "watcher failed: {reason}"),
            Self::WatchDisconnected => {
                f.write_str("watcher event stream disconnected; restart the watcher")
            }
            Self::Refused(reason) => write!(f, "profile refused: {reason}"),
        }
    }
}

impl std::error::Error for CookError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            Self::Product(error) => Some(error),
            Self::Watch(error) => Some(error),
            Self::MeshImport { source, .. } => Some(source),
            Self::ImageImport(error) => Some(error),
            Self::TextureEncode(error) => Some(error),
            _ => None,
        }
    }
}

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
