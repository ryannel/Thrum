//! Read-only products selected by residency at the preceding submission boundary.

use std::sync::{Arc, Mutex};

use thrum_asset::{ContentHash, Profile};

/// One product a consumer needs. Names and dimensions are fixed before preparation.
#[derive(Clone, Debug)]
pub struct LeaseRequest {
    /// Manifest name, without the source extension.
    pub name: String,
    /// Required texture dimension; D2 and D3 are supported.
    pub dimension: wgpu::TextureDimension,
}

/// A resident sampled product. It carries no loader or store authority.
#[derive(Clone, Debug)]
pub struct LentProduct {
    /// Manifest name.
    pub name: String,
    /// Exact product identity, suitable for a CPU reader's pinned map.
    pub hash: ContentHash,
    /// Portable bytes of this same publication, for a CPU reader pinned independently of HEAD.
    pub portable_hash: ContentHash,
    /// Selected encoding profile.
    pub profile: Profile,
    /// Device generation that owns the view.
    pub device_generation: u64,
    /// Changes only when this product or its device changes.
    pub serial: u64,
    /// Read-only binding view, retained across publications with unchanged identity.
    pub view: wgpu::TextureView,
    /// Description of the sampled texture.
    pub texture: wgpu::Texture,
}

/// A whole set selected for the next frame, never a partially uploaded publication.
#[derive(Clone, Debug)]
pub struct LentSet {
    /// Store generation shown by every reader in this frame.
    pub generation: u64,
    /// Products in request order.
    pub products: Vec<LentProduct>,
}

/// Read-only handle for a contribution. Residency alone publishes or clears it.
///
/// Read once during extraction/upload, bind the returned views and compare each serial with
/// the consumer's last history-advancing submission. Commit those observed serials only in
/// `submitted(true)`. A discarded recording and `submitted(false)` capture must keep the
/// invalidation pending. Drop old bindings on a device generation change, even while empty.
#[derive(Clone, Debug, Default)]
pub struct ProductLease(Arc<Mutex<LeaseState>>);

#[derive(Debug, Default)]
struct LeaseState {
    offered: Option<Arc<LentSet>>,
    serial: u64,
}

impl ProductLease {
    /// Snapshot for this frame. Empty during first load and after device replacement.
    pub fn offered(&self) -> Option<Arc<LentSet>> {
        self.0.lock().ok().and_then(|state| state.offered.clone())
    }

    pub(crate) fn clear(&self) {
        if let Ok(mut set) = self.0.lock() {
            set.offered = None;
        }
    }

    pub(crate) fn offer(&self, generation: u64, mut products: Vec<LentProduct>) {
        if let Ok(mut state) = self.0.lock() {
            for product in &mut products {
                let old = state.offered.as_ref().and_then(|set| {
                    set.products.iter().find(|old| {
                        old.name == product.name
                            && old.hash == product.hash
                            && old.portable_hash == product.portable_hash
                            && old.device_generation == product.device_generation
                    })
                });
                product.serial = if let Some(old) = old {
                    old.serial
                } else {
                    state.serial = state.serial.wrapping_add(1);
                    state.serial
                };
            }
            state.offered = Some(Arc::new(LentSet {
                generation,
                products,
            }));
        }
    }
}
