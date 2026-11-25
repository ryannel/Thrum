//! A font table and its pixel-kind sheets validated as one immutable product.

use std::collections::HashSet;
use std::ops::Range;
use std::sync::Arc;

use crate::container::Container;
use crate::layout::{
    FONT_MAGIC, FontComposition, FontGlyph, FontKern, FontSize, NO_OUTLINE, SheetFrame, font_tag,
};
use crate::{AssetError, ProductBytes, SheetKind, SheetProduct};

/// Validated font table and sheets from one content address.
#[derive(Debug)]
pub struct FontProduct {
    bytes: ProductBytes,
    sizes: Range<usize>,
    glyphs: Range<usize>,
    kerning: Range<usize>,
    compositions: Range<usize>,
    sheets: [Option<Arc<SheetProduct>>; 3],
    sheet_bytes: [usize; 3],
}

/// Borrowed metrics and frames for one cooked size and scale.
#[derive(Clone, Copy, Debug)]
pub struct FontStrike<'a> {
    /// Line metrics and scale.
    pub metrics: &'a FontSize,
    /// First entry is the missing-glyph box, followed by scalars in ascending order.
    pub glyphs: &'a [FontGlyph],
    /// Nonzero adjustments in lexicographic order.
    pub kerning: &'a [FontKern],
    /// Available compositions in lexicographic order.
    pub compositions: &'a [FontComposition],
    /// Pixel-kind sheet for this size and scale.
    pub sheet: &'a SheetProduct,
}

impl FontProduct {
    /// Check the font table and every referenced sheet before exposing the product.
    ///
    /// # Errors
    /// Returns the named invalid font section, metric, glyph, or sheet.
    pub fn open(bytes: ProductBytes) -> Result<Self, AssetError> {
        let table_error = |_| AssetError::Malformed("font table truncated or malformed");
        let container = Container::parse(&bytes, FONT_MAGIC).map_err(table_error)?;
        let sizes = container
            .slice::<FontSize>(font_tag::SIZES)
            .map_err(table_error)?;
        let glyphs = container
            .slice::<FontGlyph>(font_tag::GLYPHS)
            .map_err(table_error)?;
        let kerning = container
            .slice::<FontKern>(font_tag::KERNING)
            .map_err(table_error)?;
        let compositions = container
            .slice::<FontComposition>(font_tag::COMPOSE)
            .map_err(table_error)?;
        let Some(face) = sizes.first() else {
            return Err(AssetError::Malformed("font table has no sizes"));
        };
        if face.pixel == 1 && sizes.len() != 1 {
            return Err(AssetError::Malformed(
                "pixel font has more than one cooked size",
            ));
        }
        let mut sheets = [None, None, None];
        let mut sheet_bytes = [0; 3];
        let mut seen = HashSet::new();
        for size in sizes {
            let scale = size.scale.wrapping_sub(1) as usize;
            if scale >= sheets.len()
                || size.pixel > 1
                || size.pixel != face.pixel
                || size.pixel == 1 && scale != 0
            {
                return Err(AssetError::Malformed("font scale"));
            }
            if !seen.insert((size.points.to_bits(), size.scale)) {
                return Err(AssetError::Malformed("font duplicate size and scale"));
            }
            if sheets[scale].is_none() {
                let section = container
                    .range::<u8>(font_tag::SHEETS[scale])
                    .map_err(|_| AssetError::Malformed("font sheet missing for listed scale"))?;
                sheet_bytes[scale] = section.len();
                let sheet = SheetProduct::open(bytes.section(section))?;
                if sheet.kind() != SheetKind::Pixel || sheet.density().to_bits() != 1.0f32.to_bits()
                {
                    return Err(AssetError::Malformed(
                        "font sheet must be pixel kind at unit density",
                    ));
                }
                if !sheet.frames().all(|(_, frame)| {
                    frame
                        .pivot
                        .iter()
                        .all(|value| value.abs() <= 32_768.0 && value.fract() == 0.0)
                }) {
                    return Err(AssetError::Malformed(
                        "font sheet pivot is not a whole output pixel",
                    ));
                }
                sheets[scale] = Some(Arc::new(sheet));
            }
            let count = sheets[scale]
                .as_ref()
                .map_or(0, |sheet| sheet.frames().len());
            validate_size(size, glyphs, kerning, count)?;
        }
        if !compositions
            .windows(2)
            .all(|pair| pair[0].pair < pair[1].pair)
            || compositions.iter().any(|entry| {
                entry
                    .pair
                    .into_iter()
                    .chain([entry.scalar])
                    .any(|value| char::from_u32(value).is_none() || value == 0)
            })
        {
            return Err(AssetError::Malformed("font composition table"));
        }
        Ok(Self {
            sizes: container
                .range::<FontSize>(font_tag::SIZES)
                .map_err(table_error)?,
            glyphs: container
                .range::<FontGlyph>(font_tag::GLYPHS)
                .map_err(table_error)?,
            kerning: container
                .range::<FontKern>(font_tag::KERNING)
                .map_err(table_error)?,
            compositions: container
                .range::<FontComposition>(font_tag::COMPOSE)
                .map_err(table_error)?,
            bytes,
            sheets,
            sheet_bytes,
        })
    }

    /// Every cooked size and scale.
    pub fn sizes(&self) -> &[FontSize] {
        self.records(&self.sizes)
    }

    /// The exact cooked size at the given scale.
    pub fn strike(&self, points: f32, scale: u32) -> Option<FontStrike<'_>> {
        let metrics = self
            .sizes()
            .iter()
            .find(|size| size.points.to_bits() == points.to_bits() && size.scale == scale)?;
        Some(FontStrike {
            metrics,
            glyphs: span(self.records(&self.glyphs), metrics.glyphs)?,
            kerning: span(self.records(&self.kerning), metrics.kerning)?,
            compositions: self.records(&self.compositions),
            sheet: self.sheet(scale)?,
        })
    }

    /// Pixel sheet for a cooked scale. Pixel fonts have only scale one.
    pub fn sheet(&self, scale: u32) -> Option<&Arc<SheetProduct>> {
        self.sheets.get(scale.checked_sub(1)? as usize)?.as_ref()
    }

    /// Container bytes excluding the three sheets.
    pub fn table_bytes(&self) -> usize {
        self.bytes.len() - self.sheet_bytes.iter().sum::<usize>()
    }

    fn records<T: bytemuck::Pod>(&self, range: &Range<usize>) -> &[T] {
        // All four immutable table sections were type- and alignment-checked at open.
        bytemuck::cast_slice(&self.bytes[range.clone()])
    }

    /// Encoded sheet bytes indexed by scale minus one.
    pub fn sheet_bytes(&self) -> [usize; 3] {
        self.sheet_bytes
    }
}

impl FontStrike<'_> {
    /// Look up a glyph by its scalar.
    pub fn glyph(&self, scalar: char) -> Option<&FontGlyph> {
        self.glyphs
            .binary_search_by_key(&u32::from(scalar), |entry| entry.scalar)
            .ok()
            .map(|index| &self.glyphs[index])
    }

    /// The corresponding pixel sheet frame.
    pub fn frame(&self, index: u32) -> Option<&SheetFrame> {
        self.sheet.frame_at(index)
    }
}

fn span<T>(entries: &[T], [start, count]: [u32; 2]) -> Option<&[T]> {
    let start = start as usize;
    entries.get(start..start.checked_add(count as usize)?)
}

fn validate_size(
    size: &FontSize,
    glyphs: &[FontGlyph],
    kerning: &[FontKern],
    count: usize,
) -> Result<(), AssetError> {
    if ![size.points, size.ascent, size.descent, size.gap]
        .into_iter()
        .all(f32::is_finite)
    {
        return Err(AssetError::Malformed("font metric is not finite"));
    }
    if [size.points, size.ascent, size.descent, size.gap]
        .into_iter()
        .any(|metric| metric.abs() > 32_768.0)
        || size.points <= 0.0
        || size.ascent < 0.0
        || size.descent < 0.0
        || size.ascent + size.descent + size.gap <= 0.0
        || size.widest_digit < 0
    {
        return Err(AssetError::Malformed("font size or line metrics"));
    }
    let glyphs = span(glyphs, size.glyphs)
        .filter(|glyphs| glyphs.first().is_some_and(|glyph| glyph.scalar == 0))
        .ok_or(AssetError::Malformed(
            "font glyph table range or missing-glyph box",
        ))?;
    let kerning =
        span(kerning, size.kerning).ok_or(AssetError::Malformed("font kerning table range"))?;
    if glyphs.iter().any(|glyph| {
        glyph.frame as usize >= count
            || glyph.outline != NO_OUTLINE && glyph.outline as usize >= count
    }) {
        return Err(AssetError::Malformed(
            "font glyph names a missing sheet frame",
        ));
    }
    if !glyphs
        .windows(2)
        .all(|pair| pair[0].scalar < pair[1].scalar)
        || glyphs
            .iter()
            .any(|glyph| glyph.advance < 0 || char::from_u32(glyph.scalar).is_none())
    {
        return Err(AssetError::Malformed("font glyph table"));
    }
    if !kerning.windows(2).all(|pair| pair[0].pair < pair[1].pair)
        || kerning.iter().any(|entry| {
            entry.pair.into_iter().any(|scalar| {
                scalar == 0
                    || glyphs
                        .binary_search_by_key(&scalar, |glyph| glyph.scalar)
                        .is_err()
            })
        })
    {
        return Err(AssetError::Malformed("font kerning table"));
    }
    Ok(())
}
