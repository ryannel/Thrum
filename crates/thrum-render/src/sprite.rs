//! Display-referred 2D scenes. World X points right and world Y points up.

use std::sync::Arc;

mod graph;
mod pass;
mod world;
pub(crate) use graph::{PreparedSprites, SpriteTarget};
pub use world::{SpritePose, SpriteSink};

pub(crate) const SHEET_MAX_MIP: u32 = 2;
pub(crate) const SHEET_GUTTER: u32 = 1 << SHEET_MAX_MIP;

/// One frame's 2D camera. The host supplies the drawable's physical extent.
#[derive(Clone, Copy, Debug)]
pub struct SpriteView {
    /// World point at the centre of the output.
    pub centre: [f64; 2],
    /// Physical output pixels per world unit, independent of display density.
    pub pixels_per_unit: f32,
}

impl SpriteView {
    pub(crate) fn clip_scale(
        self,
        extent: [u32; 2],
    ) -> Result<[f32; 2], crate::error::RenderError> {
        let scale = extent.map(|side| self.pixels_per_unit / side as f32 * 2.0);
        let limit = f64::from(f32::MAX) / 16.0;
        if self
            .centre
            .into_iter()
            .all(|axis| (-limit..=limit).contains(&axis))
            && scale
                .into_iter()
                .all(|value| value.is_finite() && value > 0.0)
        {
            Ok(scale)
        } else {
            Err(crate::error::RenderError::InvalidView)
        }
    }
}

/// A rectangle in the embedded sheet. Texel coordinates begin at the top left.
#[derive(Clone, Copy, Debug)]
pub struct SpriteRect {
    /// First texel's column and row.
    pub origin: [u32; 2],
    /// Width and height in texels.
    pub size: [u32; 2],
}

/// Immutable authored RGBA8 sRGB pixels and their frame rectangles.
#[derive(Clone, Debug)]
pub struct EmbeddedSheet {
    /// Physical dimensions of the page.
    pub extent: [u32; 2],
    /// Top-to-bottom, tightly packed straight-alpha texels.
    pub rgba: Arc<[u8]>,
    /// Frame rectangles, in the order addressed by sprite rows.
    pub frames: Arc<[SpriteRect]>,
    /// Texels per world unit before applying the row's scale.
    pub texels_per_unit: f32,
}

/// Preparation-time assets and maximum frame-row count for a 2D scene.
#[derive(Clone, Debug)]
pub struct SpriteScene {
    /// Sheets in slot order, addressed by sprite rows.
    pub sheets: Vec<NamedSheet>,
    /// Number of physical world slots available to extraction.
    pub world_capacity: u32,
    /// Maximum number of valid rows retained from any frame's list.
    pub row_capacity: u32,
    /// World-space tolerance for exchanges between nearby world rows.
    pub order_band: f64,
}

impl SpriteScene {
    /// Prepare a scene with one sheet and frame rows only.
    #[must_use]
    pub fn new(sheet: EmbeddedSheet, row_capacity: u32) -> Self {
        Self {
            sheets: vec![NamedSheet::pixel("default", sheet)],
            world_capacity: 0,
            row_capacity,
            order_band: 0.0,
        }
    }
}

/// Filtering for an embedded sprite sheet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u32)]
pub enum SpriteSampling {
    /// Coverage sampling preserves the authored grid at magnification.
    #[default]
    Pixel,
    /// Linear filtering between texels and mip levels.
    Smooth,
    /// Point sampling for the coverage proof's control.
    #[doc(hidden)]
    NearestControl,
    /// Bilinear sampling for the coverage proof's control.
    #[doc(hidden)]
    BilinearControl,
}

/// One embedded sheet and its identity in preparation diagnostics.
#[derive(Clone, Debug)]
pub struct NamedSheet {
    /// Name reported for invalid sheets or an excess sheet slot.
    pub name: String,
    /// Authored pixels and their frames.
    pub sheet: EmbeddedSheet,
    /// Sampling applied to this sheet.
    pub sampling: SpriteSampling,
    /// Indices of frames that must share their outside edges with adjacent tiles.
    pub tiles: Vec<u32>,
}

impl NamedSheet {
    /// An embedded pixel sheet with transparent margins around its frames.
    #[must_use]
    pub fn pixel(name: impl Into<String>, sheet: EmbeddedSheet) -> Self {
        Self {
            name: name.into(),
            sheet,
            sampling: SpriteSampling::Pixel,
            tiles: Vec::new(),
        }
    }
}

/// Measurements from one recorded sprite frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct SpriteFrameStats {
    /// Accepted world rows visible to the pass (before GPU clipping).
    pub world_rows: u32,
    /// Accepted frame rows.
    pub frame_rows: u32,
    /// World row records written to the queue.
    pub world_upload_rows: u32,
    /// Payload of world row writes, in bytes.
    pub world_upload_bytes: usize,
    /// Payload of order-list writes, in bytes.
    pub order_upload_bytes: usize,
    /// CPU time spent updating and merging the retained order.
    pub order_cpu: std::time::Duration,
    /// Sum of projected quad area clipped to the output (overdraw included).
    pub shaded_pixels: f64,
    /// Draws recorded by the sprite pass.
    pub draws: u32,
}

/// One frame row, with a top-left pivot and signed scale for flipping each axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpriteRow {
    /// Embedded sheet slot.
    pub sheet: u32,
    /// Layers draw in increasing order.
    pub layer: i32,
    /// Optional explicit sort value in world units. Defaults to minus Y.
    pub sort: Option<f64>,
    /// Straight linear flash RGB and replacement amount.
    pub flash: [f32; 4],
    /// Fraction of source alpha suppressed after shading.
    pub additive: f32,
    /// Rectangle index in the sheet's frame table.
    pub frame: u32,
    /// World position of the top-left pivot.
    pub position: [f64; 2],
    /// Counterclockwise rotation about that pivot, in radians.
    pub rotation: f32,
    /// Local X and Y scale.
    pub scale: [f32; 2],
    /// Straight linear RGBA multiplier. RGB may boost a texel above white.
    pub tint: [f32; 4],
}

impl SpriteRow {
    /// A native-size, unrotated row with unchanged colour.
    #[must_use]
    pub const fn new(frame: u32, position: [f64; 2]) -> Self {
        Self {
            sheet: 0,
            layer: 0,
            sort: None,
            flash: [0.0; 4],
            additive: 0.0,
            frame,
            position,
            rotation: 0.0,
            scale: [1.0; 2],
            tint: [1.0; 4],
        }
    }
}

/// View and frame rows sampled together by the game.
#[derive(Clone, Copy)]
pub struct SpriteFrame<'a> {
    /// Unjittered view used by every row.
    pub view: SpriteView,
    /// Frame rows borrowed during host preparation, with list position breaking ties.
    pub rows: &'a [SpriteRow],
}
