//! Runtime side of Thrum's content-addressed asset loop.
//!
//! A cooker (the authoring-only `thrum-cook` package) writes immutable products into a
//! content-addressed store named by their BLAKE3 hash, then publishes a manifest by atomically
//! replacing one small `HEAD` record. This crate is everything a running game needs and nothing
//! more: it reads `HEAD`, maps the manifest and products (memory-mapped on native, aligned owned
//! bytes elsewhere), and validates their structure in constant time before handing out typed,
//! zero-copy views. Deep checks and hash verification are separate, explicit calls.
//!
//! Targets that cannot map the store (the browser) load the same products from the authoring dev
//! server through [`DevClient`], which verifies every fetched byte against its content address.
//!
//! Formats are little-endian. Every section of a Thrum container starts on a 64-byte boundary, so
//! mapped bytes cast directly to the GPU-ready element types. Textures and baked volumes are KTX2
//! files, whose levels are uploaded straight from the mapping.

#[cfg(target_endian = "big")]
compile_error!("Thrum cooked products are little-endian");

mod bytes;
mod container;
mod error;
mod hash;
mod head;
pub mod ktx2;
pub mod layout;
mod manifest;
mod mesh;
mod profile;
mod remote;
mod store;

pub use bytes::ProductBytes;
pub use error::AssetError;
pub use hash::ContentHash;
pub use head::Head;
pub use ktx2::{Ktx2, Ktx2Format, Ktx2Level};
pub use manifest::{Manifest, ManifestEntry};
pub use mesh::MeshProduct;
pub use profile::{ProductKind, Profile};
#[cfg(target_arch = "wasm32")]
pub use remote::BrowserFetch;
pub use remote::{DevClient, Fetch, FetchReply, RemoteGeneration};
pub use store::{AssetStore, HeadWatcher};
