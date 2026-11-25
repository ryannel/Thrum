//! Read-only products selected by residency at the preceding submission boundary.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use thrum_asset::{ContentHash, FontProduct, Profile};
use thrum_render::sprite::{
    PaintStamp, SpriteSheetPage, SpriteSheetPublication, SpriteSheetSource,
};

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
    /// Validated frame table from the exact product that supplied the page array.
    pub sheet: Option<Arc<thrum_asset::SheetProduct>>,
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
    sheets: Option<Arc<SpriteSheetPublication>>,
    serial: u64,
    font_candidate: Option<Arc<FontCandidate>>,
    font_committed: Option<Arc<FontCandidate>>,
    font_answer: Option<Result<(), String>>,
}

impl SpriteSheetSource for ProductLease {
    fn sheet_publication(&self) -> Option<Arc<SpriteSheetPublication>> {
        self.0.lock().ok().and_then(|state| state.sheets.clone())
    }
}

impl ProductLease {
    /// Snapshot for this frame. Empty during first load and after device replacement.
    pub fn offered(&self) -> Option<Arc<LentSet>> {
        self.0.lock().ok().and_then(|state| state.offered.clone())
    }

    pub(crate) fn clear(&self) {
        if let Ok(mut set) = self.0.lock() {
            set.offered = None;
            set.sheets = None;
            set.font_candidate = None;
            set.font_committed = None;
            set.font_answer = None;
        }
    }

    pub(crate) fn offer(
        &self,
        generation: u64,
        mut products: Vec<LentProduct>,
        paint_stamp: Option<PaintStamp>,
    ) {
        if let Ok(mut state) = self.0.lock() {
            for product in &mut products {
                let old = state.offered.as_ref().and_then(|set| {
                    set.products.iter().find(|old| {
                        old.name == product.name
                            && old.hash == product.hash
                            && old.portable_hash == product.portable_hash
                            && old.device_generation == product.device_generation
                            && old.texture == product.texture
                    })
                });
                product.serial = if let Some(old) = old {
                    old.serial
                } else {
                    state.serial = state.serial.wrapping_add(1);
                    state.serial
                };
            }
            let sheets: Vec<_> = products
                .iter()
                .filter_map(|entry| {
                    Some(SpriteSheetPage {
                        name: entry.name.clone(),
                        view: entry.view.clone(),
                        product: entry.sheet.clone()?,
                    })
                })
                .collect();
            state.sheets = products
                .first()
                .filter(|_| !sheets.is_empty())
                .map(|first| {
                    Arc::new(SpriteSheetPublication {
                        generation,
                        device_generation: first.device_generation,
                        sheets,
                        paint_stamp,
                    })
                });
            state.offered = Some(Arc::new(LentSet {
                generation,
                products,
            }));
        }
    }
}

/// Shared demand for the output scale of a scene's font sheets.
#[derive(Clone, Debug)]
pub struct FontScale(Arc<AtomicU32>);

impl Default for FontScale {
    fn default() -> Self {
        Self(Arc::new(AtomicU32::new(1)))
    }
}

impl FontScale {
    /// Request the scale for a new publication. Returns false unless it is one, two or three.
    pub fn set(&self, scale: u32) -> bool {
        if !(1..=3).contains(&scale) {
            return false;
        }
        self.0.store(scale, Ordering::Relaxed);
        true
    }

    /// Latest scale requested; residency can still be showing the previous one.
    pub fn get(&self) -> u32 {
        self.0.load(Ordering::Relaxed)
    }
}

/// CPU inputs from the exact publication that is uploading or was submitted.
#[derive(Debug)]
pub struct FontCandidate {
    /// Paint stamp matching these tables and sheet scale.
    pub stamp: PaintStamp,
    /// Manifest names and validated font tables, in lease order.
    pub fonts: Vec<(String, Arc<FontProduct>)>,
}

impl FontCandidate {
    /// Find the validated table of a named font.
    pub fn font(&self, name: &str) -> Option<&FontProduct> {
        self.fonts
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, font)| font.as_ref())
    }
}

impl ProductLease {
    /// Pending tables to lay out beside the interface currently drawn.
    pub fn font_candidate(&self) -> Option<Arc<FontCandidate>> {
        self.0.lock().ok()?.font_candidate.clone()
    }

    /// Tables whose switch has been submitted.
    pub fn committed_font(&self) -> Option<Arc<FontCandidate>> {
        self.0.lock().ok()?.font_committed.clone()
    }

    /// Report that a prepared layout exists, or refuse the whole pending publication by name.
    pub fn answer_font(&self, stamp: PaintStamp, result: Result<(), String>) {
        if let Ok(mut state) = self.0.lock()
            && state
                .font_candidate
                .as_ref()
                .is_some_and(|candidate| candidate.stamp == stamp)
            && state.font_answer.is_none()
        {
            state.font_answer = Some(result);
        }
    }

    pub(crate) fn prepare_font(&self, candidate: Option<FontCandidate>) {
        if let Ok(mut state) = self.0.lock() {
            state.font_candidate = candidate.map(Arc::new);
            state.font_answer = None;
        }
    }

    pub(crate) fn font_ready(&self, stamp: Option<PaintStamp>) -> bool {
        stamp.is_none_or(|stamp| {
            self.0.lock().is_ok_and(|state| {
                state
                    .font_candidate
                    .as_ref()
                    .is_some_and(|candidate| candidate.stamp == stamp)
                    && matches!(state.font_answer, Some(Ok(())))
            })
        })
    }

    pub(crate) fn font_refusal(&self, stamp: Option<PaintStamp>) -> Option<String> {
        let state = self.0.lock().ok()?;
        if state
            .font_candidate
            .as_ref()
            .map(|candidate| candidate.stamp)
            != stamp
        {
            return None;
        }
        state.font_answer.as_ref()?.as_ref().err().cloned()
    }

    pub(crate) fn commit_font(&self, stamp: Option<PaintStamp>) {
        if let Ok(mut state) = self.0.lock()
            && stamp.is_some()
            && state
                .font_candidate
                .as_ref()
                .map(|candidate| candidate.stamp)
                == stamp
        {
            state.font_committed = state.font_candidate.take();
            state.font_answer = None;
        }
    }
}
