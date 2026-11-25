//! On-disk font table, in output pixels of each size and scale.

use bytemuck::{Pod, Zeroable};

/// Container signature for font products.
pub const FONT_MAGIC: [u8; 8] = *b"THRFONT1";

/// Font product section tags.
pub mod font_tag {
    /// Metrics, one record for each size and scale.
    pub const SIZES: [u8; 4] = *b"SIZE";
    /// Character widths and frame indices.
    pub const GLYPHS: [u8; 4] = *b"GLYP";
    /// Non-zero adjustments for pairs.
    pub const KERNING: [u8; 4] = *b"KERN";
    /// Canonical compositions within this font's character set.
    pub const COMPOSE: [u8; 4] = *b"COMP";
    /// The embedded pixel-kind sheet for each scale, in scale order.
    pub const SHEETS: [[u8; 4]; 3] = [*b"SHT1", *b"SHT2", *b"SHT3"];
}

/// Absent outline frame.
pub const NO_OUTLINE: u32 = u32::MAX;

/// Table record for a single cooked size and scale.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct FontSize {
    /// Authored point size.
    pub points: f32,
    /// Cooked scale from one to three. Pixel fonts are cooked at one.
    pub scale: u32,
    /// Baseline distance below the top of the line, in output pixels.
    pub ascent: f32,
    /// Distance below baseline, in output pixels.
    pub descent: f32,
    /// Spacing between lines, in output pixels.
    pub gap: f32,
    /// Rounded width of the widest available ASCII digit.
    pub widest_digit: i32,
    /// Start and length in the character table (first entry is the missing-glyph box).
    pub glyphs: [u32; 2],
    /// Start and length in the kerning table.
    pub kerning: [u32; 2],
    /// One for a grid source, zero for an outline source.
    pub pixel: u32,
}

/// A character's frame and whole-pixel advance.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct FontGlyph {
    /// Unicode scalar value, or zero for the missing-glyph box.
    pub scalar: u32,
    /// Frame index in the corresponding scale's sheet.
    pub frame: u32,
    /// Outline frame index or the no-outline sentinel.
    pub outline: u32,
    /// Whole-pixel distance to the next pen position.
    pub advance: i32,
}

/// A whole-pixel adjustment for a pair.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct FontKern {
    /// The pair's Unicode scalars.
    pub pair: [u32; 2],
    /// Adjustment to the first glyph's advance.
    pub pixels: i32,
}

/// A base letter and combining mark with a cooked precomposed glyph.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct FontComposition {
    /// Base and mark.
    pub pair: [u32; 2],
    /// Their composed scalar.
    pub scalar: u32,
}
