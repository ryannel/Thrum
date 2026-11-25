//! Interface paint lists in output pixels.

pub use thrum_render::sprite::{InterfaceConfig, PaintList, SpriteRow};

/// Build rows in drawing order for one output extent.
pub struct PaintListBuilder {
    extent: [u32; 2],
    rows: Vec<SpriteRow>,
}

impl PaintListBuilder {
    /// Start an empty paint list for this frame's layout extent.
    pub const fn new(extent: [u32; 2]) -> Self {
        Self {
            extent,
            rows: Vec::new(),
        }
    }

    /// Paint a row above all previously added rows.
    pub fn push(&mut self, row: SpriteRow) {
        self.rows.push(row);
    }

    /// Freeze the paint list so a game can retain and reuse it across frames.
    #[must_use]
    pub fn finish(self) -> PaintList {
        PaintList {
            extent: self.extent,
            rows: self.rows.into(),
        }
    }
}
