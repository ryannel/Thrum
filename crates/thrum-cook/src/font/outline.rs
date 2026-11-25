//! Outline glyphs are light-hinted, rasterised at the requested size, and packed without resampling.

use std::collections::BTreeSet;

use std::sync::Arc;

use harfrust::{Buffer, Direction, Feature, Script, ShapeOptions, ShapePlan, ShaperFont, shape};
use image::{Rgba, RgbaImage};
use skrifa::instance::{LocationRef, Size as RasterSize};
use skrifa::outline::{DrawSettings, HintingInstance, OutlinePen, SmoothMode, Target};
use skrifa::raw::types::Tag;
use skrifa::{FontRef, GlyphId, MetadataProvider};
use thrum_asset::layout::{FontKern, FontSize};
use zeno::{Command, Join, Mask, Stroke};

use super::{Font, Size, index, invalid, missing_box};
use crate::CookError;

const PUNCTUATION: &str = "–—‘’“”…•×−€←↑→↓";

#[expect(
    clippy::cast_precision_loss,
    reason = "font-unit and bounded pixel metrics are precisely representable"
)]
pub(super) fn cook(
    output: &mut Font,
    bytes: Vec<u8>,
    sizes: &[Size],
    scales: &[u32],
    characters: Option<&str>,
) -> Result<(), CookError> {
    let bytes: Arc<dyn AsRef<[u8]> + Send + Sync> = Arc::new(bytes);
    let face = FontRef::new(bytes.as_ref().as_ref())
        .map_err(|error| invalid(format!("font file does not parse: {error}")))?;
    if sizes.is_empty() || sizes.len() > 32 {
        return Err(invalid("expected one to 32 font sizes"));
    }
    let mut points_seen = BTreeSet::new();
    for size in sizes {
        if !size.points.is_finite() || !(0.0..=256.0).contains(&size.points) || size.points <= 0.0 {
            return Err(invalid("font size must be positive and at most 256 points"));
        }
        if !points_seen.insert(size.points.to_bits()) {
            return Err(invalid("duplicate font size"));
        }
        if !size.outline.is_finite() || !(0.0..=16.0).contains(&size.outline) {
            return Err(invalid("font outline must be between 0 and 16 points"));
        }
    }
    let unique_scales: BTreeSet<_> = scales.iter().copied().collect();
    if unique_scales.is_empty()
        || unique_scales.len() != scales.len()
        || !unique_scales.iter().all(|scale| (1..=3).contains(scale))
    {
        return Err(invalid("font scales must be unique integers from 1 to 3"));
    }
    let mapping = face.charmap();
    let glyphs = repertoire(characters)
        .into_iter()
        .filter_map(|character| match mapping.map(character) {
            Some(glyph) => Some(Ok((character, glyph))),
            None if characters.is_some() => Some(Err(invalid(format!(
                "font lacks listed character {character:?}"
            )))),
            None => None,
        })
        .collect::<Result<Vec<_>, _>>()?;
    if glyphs.len() > 2048 {
        return Err(invalid("font character list exceeds 2048 characters"));
    }
    let shaping_face = harfrust::Font::new(Arc::clone(&bytes), 0)
        .ok_or_else(|| invalid("font file does not parse for shaping"))?;
    let pairs = kern_pairs(&face, &shaping_face, &glyphs)?;
    for scale in unique_scales {
        for size in sizes {
            let pixels = size.points * scale as f32;
            let mut strike = OutlineStrike::new(&face, pixels, size.outline * scale as f32)?;
            strike.cook(output, &glyphs, &pairs, size.points, scale)?;
        }
    }
    Ok(())
}

fn repertoire(characters: Option<&str>) -> BTreeSet<char> {
    if let Some(characters) = characters {
        return characters.chars().collect();
    }
    (' '..='~')
        .chain('\u{a0}'..='\u{17f}')
        .chain(PUNCTUATION.chars())
        .collect()
}

fn kern_pairs(
    face: &FontRef<'_>,
    shaping_face: &harfrust::Font,
    glyphs: &[(char, GlyphId)],
) -> Result<Vec<([u32; 2], f32)>, CookError> {
    let shaper = ShaperFont::new(shaping_face);
    let features = [
        Feature::new(Tag::new(b"liga"), 0, ..),
        Feature::new(Tag::new(b"clig"), 0, ..),
        Feature::new(Tag::new(b"kern"), 1, ..),
    ];
    let script = Some(Script::LATIN);
    let plan = ShapePlan::new(
        shaping_face,
        Direction::LeftToRight,
        script,
        None,
        &features,
    );
    let mut buffer = Buffer::new();
    let units = shaping_face.units_per_em();
    if units == 0 {
        return Err(invalid("font has zero units per em"));
    }
    let metrics = face.glyph_metrics(RasterSize::unscaled(), LocationRef::default());
    let widths = glyphs
        .iter()
        .map(|&(_, glyph)| {
            metrics
                .advance_width(glyph)
                .ok_or_else(|| invalid("font has no horizontal advance"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut pairs = Vec::new();
    #[expect(
        clippy::cast_precision_loss,
        reason = "font design units fit in the mantissa"
    )]
    for (first_index, &(first, first_id)) in glyphs.iter().enumerate() {
        for (second_index, &(second, second_id)) in glyphs.iter().enumerate() {
            buffer.set_direction(Direction::LeftToRight);
            buffer.set_script(script);
            buffer.push(u32::from(first), 0);
            buffer.push(u32::from(second), 1);
            shape(
                &shaper,
                &mut buffer,
                ShapeOptions::new().features(&features).plan(Some(&plan)),
            )
            .map_err(invalid)?;
            if let [left, right] = buffer.glyph_infos()
                && left.glyph_id == first_id.to_u32()
                && right.glyph_id == second_id.to_u32()
                && let [left, right] = buffer.glyph_positions()
            {
                let adjustment = (left.x_advance + right.x_advance) as f32
                    - widths[first_index]
                    - widths[second_index];
                if adjustment != 0.0 {
                    pairs.push((
                        [u32::from(first), u32::from(second)],
                        adjustment / f32::from(units),
                    ));
                }
            }
            buffer.clear();
        }
    }
    Ok(pairs)
}

struct OutlineStrike<'a> {
    glyphs: skrifa::outline::OutlineGlyphCollection<'a>,
    metrics: skrifa::metrics::GlyphMetrics<'a>,
    line: skrifa::metrics::Metrics,
    hint: HintingInstance,
    pixels: f32,
    pixels_per_unit: f32,
    width: f32,
    path: Path,
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    reason = "raster metrics are bounded output pixels; advances and kerns are rounded as specified"
)]
impl<'a> OutlineStrike<'a> {
    fn new(face: &FontRef<'a>, pixels: f32, width: f32) -> Result<Self, CookError> {
        let size = RasterSize::new(pixels);
        let location = LocationRef::default();
        let glyphs = face.outline_glyphs();
        let target = Target::Smooth {
            mode: SmoothMode::Light,
            preserve_linear_metrics: true,
            symmetric_rendering: true,
        };
        let hint = HintingInstance::new(&glyphs, size, location, target).map_err(invalid)?;
        Ok(Self {
            glyphs,
            metrics: face.glyph_metrics(RasterSize::unscaled(), location),
            pixels_per_unit: pixels
                / f32::from(face.metrics(RasterSize::unscaled(), location).units_per_em),
            line: face.metrics(size, location),
            hint,
            pixels,
            width,
            path: Path(Vec::new()),
        })
    }

    fn cook(
        &mut self,
        output: &mut Font,
        glyphs: &[(char, GlyphId)],
        pairs: &[([u32; 2], f32)],
        points: f32,
        scale: u32,
    ) -> Result<(), CookError> {
        let glyph_start = index(output.glyphs.len())?;
        let kern_start = index(output.kerning.len())?;
        let width = (self.pixels * 0.55).round().max(3.0) as u32;
        let height = (self.pixels * 0.7).round().max(3.0) as u32;
        self.path = rectangle(width as f32, height as f32);
        let fill = (missing_box(width, height), [0.0, height as f32]);
        let stroke = (self.width > 0.0)
            .then(|| bitmap(&self.path, self.width))
            .transpose()?;
        output.add_frames(scale, fill, stroke, 0, width.cast_signed() + 1)?;

        let mut widest_digit = 0;
        for &(character, glyph_id) in glyphs {
            let units = self
                .metrics
                .advance_width(glyph_id)
                .ok_or_else(|| invalid(format!("missing advance for {character:?}")))?;
            let advance = (units * self.pixels_per_unit).round() as i32;
            self.path.0.clear();
            if let Some(glyph) = self.glyphs.get(glyph_id) {
                glyph
                    .draw(DrawSettings::hinted(&self.hint, false), &mut self.path)
                    .map_err(|error| invalid(format!("drawing {character:?}: {error}")))?;
            }
            let fill = bitmap(&self.path, 0.0)?;
            let fill = match self.metrics.bounds(glyph_id) {
                Some(bounds) if bounds.x_max > bounds.x_min && bounds.y_max > bounds.y_min => {
                    let frame = [
                        (bounds.x_min * self.pixels_per_unit).floor() as i32,
                        (-bounds.y_max * self.pixels_per_unit).floor() as i32,
                        (bounds.x_max * self.pixels_per_unit).ceil() as i32,
                        (-bounds.y_min * self.pixels_per_unit).ceil() as i32,
                    ];
                    include_bounds(fill, frame)?
                }
                _ => fill,
            };
            let stroke = (self.width > 0.0)
                .then(|| bitmap(&self.path, self.width))
                .transpose()?;
            output.add_frames(scale, fill, stroke, u32::from(character), advance)?;
            if character.is_ascii_digit() {
                widest_digit = widest_digit.max(advance);
            }
        }
        output
            .kerning
            .extend(pairs.iter().filter_map(|&(pair, em)| {
                let pixels = (em * self.pixels).round() as i32;
                (pixels != 0).then_some(FontKern { pair, pixels })
            }));
        output.sizes.push(FontSize {
            points,
            scale,
            ascent: self.line.ascent,
            descent: -self.line.descent,
            gap: self.line.leading,
            widest_digit,
            glyphs: [glyph_start, index(output.glyphs.len())? - glyph_start],
            kerning: [kern_start, index(output.kerning.len())? - kern_start],
            pixel: 0,
        });
        Ok(())
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "placement is bounded to one sheet page"
)]
fn bitmap(path: &Path, outline: f32) -> Result<(RgbaImage, [f32; 2]), CookError> {
    if path.0.is_empty() {
        return Ok((RgbaImage::new(1, 1), [0.0, 0.0]));
    }
    let mut mask = Mask::new(path.0.as_slice());
    let mut stroke = Stroke::new(outline * 2.0);
    if outline > 0.0 {
        mask.style(stroke.join(Join::Round));
    }
    let mut dimensions = [0; 2];
    mask.inspect(|_, width, height| dimensions = [width, height]);
    if dimensions.into_iter().any(|side| side > 2040) {
        return Err(invalid("glyph is larger than a sheet page"));
    }
    let (alpha, placement) = mask.render();
    let image = RgbaImage::from_fn(placement.width.max(1), placement.height.max(1), |x, y| {
        let coverage = alpha
            .get((y * placement.width + x) as usize)
            .copied()
            .unwrap_or(0);
        Rgba([255, 255, 255, coverage])
    });
    Ok((image, [-(placement.left as f32), -(placement.top as f32)]))
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "raster pivot is an integer inside a font frame"
)]
fn include_bounds(
    (image, pivot): (RgbaImage, [f32; 2]),
    [left, top, right, bottom]: [i32; 4],
) -> Result<(RgbaImage, [f32; 2]), CookError> {
    let x = -pivot[0] as i32;
    let y = -pivot[1] as i32;
    let width = i32::try_from(image.width()).map_err(invalid)?;
    let height = i32::try_from(image.height()).map_err(invalid)?;
    let [left, top, right, bottom] = [
        left.min(x),
        top.min(y),
        right.max(x + width),
        bottom.max(y + height),
    ];
    if [right - left, bottom - top] == [width, height] {
        return Ok((image, pivot));
    }
    let span = |a, b| {
        u32::try_from(i64::from(b) - i64::from(a))
            .ok()
            .filter(|side| *side <= 2040)
    };
    let width = span(left, right).ok_or_else(|| invalid("glyph bounds exceed a sheet page"))?;
    let height = span(top, bottom).ok_or_else(|| invalid("glyph bounds exceed a sheet page"))?;
    let mut padded = RgbaImage::new(width, height);
    image::imageops::replace(
        &mut padded,
        &image,
        i64::from(x) - i64::from(left),
        i64::from(y) - i64::from(top),
    );
    Ok((padded, [-(left as f32), -(top as f32)]))
}

// Flip the y-up outline once, when constructing the y-down sheet path.
struct Path(Vec<Command>);

impl OutlinePen for Path {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.push(Command::MoveTo([x, -y].into()));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.0.push(Command::LineTo([x, -y].into()));
    }

    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        self.0
            .push(Command::QuadTo([cx, -cy].into(), [x, -y].into()));
    }

    fn curve_to(&mut self, cx: f32, cy: f32, dx: f32, dy: f32, x: f32, y: f32) {
        self.0.push(Command::CurveTo(
            [cx, -cy].into(),
            [dx, -dy].into(),
            [x, -y].into(),
        ));
    }

    fn close(&mut self) {
        self.0.push(Command::Close);
    }
}

fn rectangle(width: f32, height: f32) -> Path {
    let mut path = Path(Vec::new());
    path.move_to(0.0, 0.0);
    path.line_to(width, 0.0);
    path.line_to(width, height);
    path.line_to(0.0, height);
    path.close();
    path
}
