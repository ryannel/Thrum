use std::sync::{Arc, RwLock};

use thrum_asset::SheetProduct;

/// A cooked page array and the table from the same content product.
#[derive(Debug)]
pub struct SpriteSheetPage {
    /// Manifest name.
    pub name: String,
    /// Read-only 2D array view for this generation.
    pub view: wgpu::TextureView,
    /// Validated CPU frame table and sheet metadata.
    pub product: Arc<SheetProduct>,
}

/// All cooked sheets offered together by content residency.
#[derive(Debug)]
pub struct SpriteSheetPublication {
    /// Store generation selected for the recording.
    pub generation: u64,
    /// Device generation that owns the views.
    pub device_generation: u64,
    /// Named sheets from this publication.
    pub sheets: Vec<SpriteSheetPage>,
    /// Stamp required by interface lists laid out from this publication.
    pub paint_stamp: Option<super::PaintStamp>,
}

/// Read-only connection to residency, installed by the game.
pub trait SpriteSheetSource: std::fmt::Debug + wgpu::WasmNotSendSync {
    /// Return the same snapshot while the offered publication remains unchanged.
    fn sheet_publication(&self) -> Option<Arc<SpriteSheetPublication>>;
}

/// A named cooked sheet registered with a sprite scene.
#[derive(Clone, Debug)]
pub struct CookedSpriteSheet {
    pub(super) name: String,
    pub(super) slot: u32,
    pub(super) indexed: bool,
    pub(super) names: Arc<RwLock<Vec<String>>>,
}

impl CookedSpriteSheet {
    pub(super) fn new(name: String, slot: u32, indexed: bool) -> Self {
        Self {
            name,
            slot,
            indexed,
            names: Arc::default(),
        }
    }

    /// Sheet slot for a sprite row.
    pub fn slot(&self) -> u32 {
        self.slot
    }

    /// Stable handle to a frame name, valid even while its picture is absent.
    ///
    /// # Panics
    /// If more than 2³² frame names are registered with this sheet.
    pub fn frame(&self, name: &str) -> u32 {
        let mut names = self
            .names
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(index) = names.iter().position(|known| known == name) {
            return u32::try_from(index).expect("registered frame index");
        }
        let index = u32::try_from(names.len()).expect("registered frame index");
        names.push(name.to_owned());
        index
    }
}
