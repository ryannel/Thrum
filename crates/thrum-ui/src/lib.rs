//! Interface paint lists in output pixels.

mod text;
mod tree;
pub use text::{Alignment, Elision, RunGlyph, TextError, TextOptions, TextRun};
pub use tree::{
    Anchor, Axis, Box2, Font, Image, Insets, Node, NodeId, NodeKind, PaintInputs, Size, SizeRule,
    Text, Tree, TreeError, TreeStats, Viewport,
};

pub use thrum_render::sprite::{
    InterfaceConfig, InterfaceSheets, PaintList, PaintStamp, SpriteRow,
};

/// Build rows in drawing order for one output extent.
pub struct PaintListBuilder {
    extent: [u32; 2],
    rows: Vec<SpriteRow>,
    stamp: Option<PaintStamp>,
}

impl PaintListBuilder {
    /// Start an empty paint list for this frame's layout extent.
    pub const fn new(extent: [u32; 2]) -> Self {
        Self {
            extent,
            rows: Vec::new(),
            stamp: None,
        }
    }

    /// Bind this list to the publication and scale it was laid out from.
    pub const fn stamp(&mut self, stamp: PaintStamp) {
        self.stamp = Some(stamp);
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
            stamp: self.stamp,
            rows: self.rows.into(),
        }
    }
}
