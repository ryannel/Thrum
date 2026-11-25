//! Validated, content-addressed sprite-sheet table and pages.

use std::collections::HashSet;
use std::ops::Range;

use crate::container::Container;
use crate::layout::{SHEET_MAGIC, SHEET_MAX_MIP, SheetFrame, SheetMeta, sheet_tag};
use crate::{AssetError, Ktx2, Ktx2Format, ProductBytes};

/// Sampling rules carried by a cooked sprite sheet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SheetKind {
    /// Coverage filtering, with uncompressed source texels.
    Pixel,
    /// Trilinear filtering.
    Smooth,
}

/// A validated sprite-sheet product. The table and the pages share one product identity.
#[derive(Debug)]
pub struct SheetProduct {
    bytes: ProductBytes,
    texture: Ktx2,
    meta: SheetMeta,
    kind: SheetKind,
    table: Range<usize>,
    names: Range<usize>,
}

impl SheetProduct {
    /// Validate the container, table and texture before exposing any sprite frame.
    ///
    /// # Errors
    /// Returns a named error for invalid metadata, frames or page data.
    pub fn open(bytes: ProductBytes) -> Result<Self, AssetError> {
        let container = Container::parse(&bytes, SHEET_MAGIC)?;
        let meta = *container.one::<SheetMeta>(sheet_tag::META)?;
        let kind = match meta.kind {
            0 => SheetKind::Pixel,
            1 => SheetKind::Smooth,
            _ => return Err(AssetError::Malformed("sheet kind")),
        };
        if !meta.density.is_finite() || meta.density <= 0.0 {
            return Err(AssetError::Malformed("sheet density"));
        }
        let table_error = |_| AssetError::Malformed("sheet frame table");
        let table = container
            .range::<SheetFrame>(sheet_tag::FRAMES)
            .map_err(table_error)?;
        let frames = container
            .slice::<SheetFrame>(sheet_tag::FRAMES)
            .map_err(table_error)?;
        let names = container.range::<u8>(sheet_tag::NAMES)?;
        if frames.is_empty() || frames.len() != meta.frames as usize {
            return Err(AssetError::Malformed("sheet frame table length"));
        }
        let pages = container.range::<u8>(sheet_tag::PAGES)?;
        let texture = Ktx2::open_section(bytes.clone(), pages).map_err(|error| match error {
            AssetError::Malformed("KTX2 level range") => {
                AssetError::Malformed("sheet page count or texture ranges")
            }
            error => error,
        })?;
        if texture.is_3d() || meta.pages == 0 || texture.array_layers() != meta.pages {
            return Err(AssetError::Malformed("sheet page count"));
        }
        if texture.levels().len() != (SHEET_MAX_MIP + 1) as usize
            || !matches!(
                (kind, texture.format()),
                (_, Ktx2Format::Rgba8Srgb)
                    | (
                        SheetKind::Smooth,
                        Ktx2Format::Bc7Srgb | Ktx2Format::Astc4x4Srgb
                    )
            )
        {
            return Err(AssetError::Malformed("sheet mip count or pixel format"));
        }
        let first = texture.levels()[0];
        let mut seen = HashSet::with_capacity(frames.len());
        for frame in frames {
            let name = frame_name(&bytes[names.clone()], frame)
                .filter(|name| !name.is_empty())
                .ok_or(AssetError::Malformed("sheet frame name"))?;
            if !seen.insert(name) {
                return Err(AssetError::Malformed("duplicate sheet frame name"));
            }
            if frame.page >= meta.pages
                || !inside(frame.origin, frame.size, [first.width, first.height])
            {
                return Err(AssetError::Malformed(
                    "sheet frame rectangle outside its page",
                ));
            }
            if !inside(frame.trim, frame.size, frame.source_size) || frame.tile > 1 {
                return Err(AssetError::Malformed("sheet frame trim or tile mark"));
            }
            if !frame.pivot.into_iter().all(f32::is_finite) {
                return Err(AssetError::Malformed("sheet frame pivot"));
            }
        }
        Ok(Self {
            bytes,
            texture,
            meta,
            kind,
            table,
            names,
        })
    }

    /// Sampling kind.
    pub fn kind(&self) -> SheetKind {
        self.kind
    }

    /// Source texels per world unit.
    pub fn density(&self) -> f32 {
        self.meta.density
    }

    /// Validated page array, with premultiplied RGB encoded for sRGB sampling.
    pub fn texture(&self) -> &Ktx2 {
        &self.texture
    }

    /// Look up a frame by its table index.
    pub fn frame_at(&self, index: u32) -> Option<&SheetFrame> {
        let frames: &[SheetFrame] = bytemuck::cast_slice(&self.bytes[self.table.clone()]);
        frames.get(index as usize)
    }

    /// Authored names and placement records.
    #[expect(
        clippy::missing_panics_doc,
        reason = "the immutable table and its UTF-8 names were validated at open"
    )]
    pub fn frames(&self) -> impl ExactSizeIterator<Item = (&str, &SheetFrame)> {
        let frames: &[SheetFrame] = bytemuck::cast_slice(&self.bytes[self.table.clone()]);
        let names = &self.bytes[self.names.clone()];
        frames.iter().map(move |frame| {
            (
                frame_name(names, frame).expect("validated sheet frame name"),
                frame,
            )
        })
    }
}

fn frame_name<'a>(names: &'a [u8], frame: &SheetFrame) -> Option<&'a str> {
    let start = frame.name_offset as usize;
    let end = start.checked_add(frame.name_len as usize)?;
    std::str::from_utf8(names.get(start..end)?).ok()
}

fn inside(origin: [u32; 2], size: [u32; 2], extent: [u32; 2]) -> bool {
    (0..2).all(|axis| {
        size[axis] != 0
            && origin[axis]
                .checked_add(size[axis])
                .is_some_and(|end| end <= extent[axis])
    })
}
