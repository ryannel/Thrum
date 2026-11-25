//! Pixel fonts keep their authored cell grid and can grow a one-texel silhouette.

use image::{Rgba, RgbaImage};
use serde::Deserialize;
use thrum_asset::layout::FontSize;

use super::{Font, index, invalid, missing_box};
use crate::CookError;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct GridEntry {
    character: char,
    cell: [u32; 2],
    advance: i32,
}

#[expect(
    clippy::cast_precision_loss,
    reason = "authored dimensions are bounded to a sheet page"
)]
pub(super) fn cook(
    font: &mut Font,
    bytes: &[u8],
    [width, height]: [u32; 2],
    baseline: u32,
    mut cells: Vec<GridEntry>,
    outline: bool,
) -> Result<(), CookError> {
    if width == 0 || height == 0 || width > 1024 || height > 1024 || baseline > height {
        return Err(invalid("pixel font cell size or baseline"));
    }
    let grid = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .map_err(invalid)?
        .into_rgba8();
    if !grid.width().is_multiple_of(width) || !grid.height().is_multiple_of(height) {
        return Err(invalid("pixel font cells must divide the image"));
    }
    let pivot = [0.0, baseline as f32];
    let add = |font: &mut Font, bitmap: RgbaImage, scalar, advance| {
        let stroke = outline.then(|| (grow(&bitmap), [1.0, pivot[1] + 1.0]));
        font.add_frames(1, (bitmap, pivot), stroke, scalar, advance)
    };
    add(
        font,
        missing_box(width, height),
        0,
        i32::try_from(width).map_err(invalid)?,
    )?;
    let mut widest_digit = 0;
    cells.sort_unstable_by_key(|cell| cell.character);
    let mut previous = '\0';
    for GridEntry {
        character,
        cell: [x, y],
        advance,
    } in cells
    {
        if character == previous
            || advance < 0
            || x >= grid.width() / width
            || y >= grid.height() / height
        {
            return Err(invalid(format!(
                "invalid or duplicate pixel font cell for {character:?}"
            )));
        }
        previous = character;
        let image =
            image::imageops::crop_imm(&grid, x * width, y * height, width, height).to_image();
        add(font, image, u32::from(character), advance)?;
        if character.is_ascii_digit() {
            widest_digit = widest_digit.max(advance);
        }
    }
    font.sizes.push(FontSize {
        points: height as f32,
        scale: 1,
        ascent: baseline as f32,
        descent: (height - baseline) as f32,
        gap: 0.0,
        widest_digit,
        glyphs: [0, index(font.glyphs.len())?],
        kerning: [0, 0],
        pixel: 1,
    });
    Ok(())
}

pub(super) fn grow(bitmap: &RgbaImage) -> RgbaImage {
    let width = bitmap.width();
    let height = bitmap.height();
    RgbaImage::from_fn(width + 2, height + 2, |x, y| {
        let mut alpha = 0;
        for row in y.saturating_sub(2)..y.saturating_add(1).min(height) {
            for col in x.saturating_sub(2)..x.saturating_add(1).min(width) {
                alpha = alpha.max(bitmap.get_pixel(col, row).0[3]);
            }
        }
        Rgba([255, 255, 255, alpha])
    })
}
