//! Display-referred 2D scenes. World X points right and world Y points up.

use std::sync::Arc;

mod graph;
mod pass;
pub(crate) use graph::{PreparedSprites, SpriteTarget};

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
    ) -> Result<[f32; 2], crate::synthetic::RenderError> {
        let scale = extent.map(|side| self.pixels_per_unit / side as f32 * 2.0);
        if self.centre.into_iter().all(f64::is_finite)
            && scale
                .into_iter()
                .all(|value| value.is_finite() && value > 0.0)
        {
            Ok(scale)
        } else {
            Err(crate::synthetic::RenderError::InvalidView)
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
    /// Pixel-art sheet resident for this scene.
    pub sheet: EmbeddedSheet,
    /// Maximum number of valid rows retained from any frame's list.
    pub row_capacity: u32,
}

/// One frame row, with a top-left pivot and signed scale for flipping each axis.
#[derive(Clone, Copy, Debug)]
pub struct SpriteRow {
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
            frame,
            position,
            rotation: 0.0,
            scale: [1.0; 2],
            tint: [1.0; 4],
        }
    }
}

/// View and rows sampled together by the game. Rows draw in supplied order.
#[derive(Clone, Copy)]
pub struct SpriteFrame<'a> {
    /// Unjittered view used by every row.
    pub view: SpriteView,
    /// Back-to-front frame rows, borrowed during host preparation.
    pub rows: &'a [SpriteRow],
}
