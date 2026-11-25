//! Cook font source folders into a validated table and ordinary pixel-kind sheets.

use std::path::Path;

use image::{Rgba, RgbaImage};
use serde::Deserialize;
use thrum_asset::Profile;
use thrum_asset::layout::{
    FONT_MAGIC, FontComposition, FontGlyph, FontKern, FontSize, NO_OUTLINE, font_tag,
};
use unicode_normalization::char::decompose_canonical;

use crate::texture::Quality;
use crate::write::ContainerWriter;
use crate::{CookError, sheet};

mod outline;
mod pixel;

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
enum Description {
    Outline {
        file: String,
        sizes: Vec<Size>,
        #[serde(default = "all_scales")]
        scales: Vec<u32>,
        characters: Option<String>,
    },
    Pixel {
        file: String,
        cell: [u32; 2],
        baseline: u32,
        characters: Vec<pixel::GridEntry>,
        #[serde(default)]
        outline: bool,
    },
}

#[derive(Clone, Copy, Deserialize)]
#[serde(deny_unknown_fields)]
struct Size {
    points: f32,
    #[serde(default)]
    outline: f32,
}

fn all_scales() -> Vec<u32> {
    vec![1, 2, 3]
}

#[derive(Default)]
struct Font {
    sizes: Vec<FontSize>,
    glyphs: Vec<FontGlyph>,
    kerning: Vec<FontKern>,
    sheets: [Vec<(RgbaImage, [f32; 2])>; 3],
}

impl Font {
    fn add_frames(
        &mut self,
        scale: u32,
        fill: (RgbaImage, [f32; 2]),
        outline: Option<(RgbaImage, [f32; 2])>,
        scalar: u32,
        advance: i32,
    ) -> Result<(), CookError> {
        let frames = &mut self.sheets[scale as usize - 1];
        let frame = index(frames.len())?;
        frames.push(fill);
        let outline = match outline {
            Some(outline) => {
                let index = index(frames.len())?;
                frames.push(outline);
                index
            }
            None => NO_OUTLINE,
        };
        self.glyphs.push(FontGlyph {
            scalar,
            frame,
            outline,
            advance,
        });
        Ok(())
    }

    fn encode(self) -> Result<Vec<u8>, CookError> {
        let mut compositions: Vec<_> = self
            .glyphs
            .iter()
            .filter_map(|glyph| composition(glyph.scalar))
            .collect();
        compositions.sort_unstable_by_key(|entry| entry.pair);
        compositions.dedup_by_key(|entry| entry.pair);
        let mut writer = ContainerWriter::new(FONT_MAGIC)
            .section(font_tag::SIZES, &self.sizes)
            .section(font_tag::GLYPHS, &self.glyphs)
            .section(font_tag::KERNING, &self.kerning)
            .section(font_tag::COMPOSE, &compositions);
        for (tag, glyphs) in font_tag::SHEETS.into_iter().zip(self.sheets) {
            if !glyphs.is_empty() {
                let sheet = sheet::pack_font(glyphs)?.encode(Profile::Portable, Quality::Fast)?;
                writer = writer.section(tag, &sheet);
            }
        }
        Ok(writer.finish())
    }
}

fn composition(scalar: u32) -> Option<FontComposition> {
    let mut parts = [0; 2];
    let mut len = 0;
    decompose_canonical(char::from_u32(scalar)?, |part| {
        if let Some(slot) = parts.get_mut(len) {
            *slot = u32::from(part);
        }
        len += 1;
    });
    (len == 2).then_some(FontComposition {
        pair: parts,
        scalar,
    })
}

fn missing_box(width: u32, height: u32) -> RgbaImage {
    RgbaImage::from_fn(width, height, |x, y| {
        let border = x == 0 || x + 1 == width || y == 0 || y + 1 == height;
        Rgba([255, 255, 255, if border { 255 } else { 0 }])
    })
}

fn index(value: usize) -> Result<u32, CookError> {
    u32::try_from(value).map_err(|_| invalid("font table is too large"))
}

fn invalid(reason: impl std::fmt::Display) -> CookError {
    CookError::Source(format!("font.json: {reason}"))
}

pub(crate) fn is_description(path: &Path) -> bool {
    path.file_name().is_some_and(|name| name == "font.json")
}

pub(crate) fn is_input(path: &Path) -> bool {
    path.extension().is_some_and(|extension| {
        ["ttf", "otf", "png"]
            .iter()
            .any(|ext| extension.eq_ignore_ascii_case(ext))
    })
}

pub(crate) fn cook<'a>(
    description: &[u8],
    inputs: impl Iterator<Item = Result<(&'a Path, Vec<u8>), CookError>>,
) -> Result<Vec<u8>, CookError> {
    let description: Description = serde_json::from_slice(description).map_err(invalid)?;
    let (Description::Outline { file, .. } | Description::Pixel { file, .. }) = &description;
    let bytes = inputs
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .find(|(path, _)| path.file_name().is_some_and(|name| name == file.as_str()))
        .ok_or_else(|| invalid(format!("missing font input {file}")))?
        .1;
    let mut font = Font::default();
    match description {
        Description::Outline {
            sizes,
            scales,
            characters,
            ..
        } => {
            outline::cook(&mut font, &bytes, &sizes, &scales, characters.as_deref())?;
        }
        Description::Pixel {
            cell,
            baseline,
            characters,
            outline,
            ..
        } => {
            pixel::cook(&mut font, &bytes, cell, baseline, characters, outline)?;
        }
    }
    font.encode()
}
