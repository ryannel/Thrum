//! Import a sprite-sheet folder and build deterministic pages, independent of the runtime device.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use image::RgbaImage;
use serde::Deserialize;
use thrum_asset::layout::{
    SHEET_GUTTER as GUTTER, SHEET_MAGIC, SHEET_MAX_MIP, SheetFrame, SheetMeta, sheet_tag,
};
use thrum_asset::{Ktx2Format, Profile};

use crate::CookError;
use crate::texture::{self, Quality};
use crate::write::{ContainerWriter, ktx2_layers};

mod pages;

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Sampling {
    Pixel,
    Smooth,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Description {
    kind: Sampling,
    density: f32,
    #[serde(default = "default_page_size")]
    page_size: u32,
    #[serde(default)]
    grids: BTreeMap<String, [u32; 2]>,
    #[serde(default)]
    pivots: BTreeMap<String, [f32; 2]>,
    #[serde(default)]
    tiles: BTreeSet<String>,
}

fn default_page_size() -> u32 {
    2048
}

pub(crate) fn is_description(path: &Path) -> bool {
    path.file_name().is_some_and(|name| name == "sheet.json")
}

pub(crate) fn is_png(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("png"))
}

pub(crate) struct PreparedSheet {
    records: Vec<SheetFrame>,
    names: Vec<u8>,
    width: u32,
    height: u32,
    /// Each mip is in layer order, with a contiguous image per layer.
    mips: [Vec<u8>; SHEET_MAX_MIP as usize + 1],
    meta: SheetMeta,
}

struct Frame {
    name: String,
    image: usize,
    cell: [u32; 2],
    rect: SheetFrame,
}

fn invalid(reason: impl Into<String>) -> CookError {
    CookError::Source(reason.into())
}

pub(crate) fn prepare<'a>(
    description: &[u8],
    sources: impl Iterator<Item = Result<(&'a Path, Vec<u8>), CookError>>,
) -> Result<PreparedSheet, CookError> {
    let description: Description = serde_json::from_slice(description)
        .map_err(|error| invalid(format!("sheet.json: {error}")))?;
    if !description.density.is_finite() || description.density <= 0.0 {
        return Err(invalid("sheet.json: density must be finite and positive"));
    }
    if !(12..=2048).contains(&description.page_size)
        || !description.page_size.is_multiple_of(GUTTER)
    {
        return Err(invalid(
            "sheet.json: page_size must be a multiple of four from 12 to 2048",
        ));
    }
    let mut images = Vec::new();
    let mut frames = Vec::new();
    let mut files = BTreeSet::new();
    let mut names = BTreeSet::new();
    for source in sources {
        let (path, bytes) = source?;
        let file = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| invalid(format!("invalid sheet image name: {}", path.display())))?;
        let stem = path
            .file_stem()
            .and_then(|name| name.to_str())
            .ok_or_else(|| invalid(format!("invalid sheet frame name: {file}")))?;
        files.insert(file);
        let image = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
            .map_err(|error| invalid(format!("{file}: {error}")))?
            .into_rgba8();
        let [w, h] = [image.width(), image.height()];
        let grid = description.grids.get(file);
        let size = grid.copied().unwrap_or([w, h]);
        if size.contains(&0) || !w.is_multiple_of(size[0]) || !h.is_multiple_of(size[1]) {
            return Err(invalid(format!("{file}: grid cells must divide the image")));
        }
        let cols = w / size[0];
        for y in 0..h / size[1] {
            for x in 0..cols {
                let name =
                    grid.map_or_else(|| stem.to_owned(), |_| format!("{stem}.{}", y * cols + x));
                if !names.insert(name.clone()) {
                    return Err(invalid(format!("duplicate sheet frame name {name}")));
                }
                let cell = [x * size[0], y * size[1]];
                let rect = frame_rect(&description, &name, &image, cell, size)?;
                frames.push(Frame {
                    name,
                    image: images.len(),
                    cell,
                    rect,
                });
            }
        }
        images.push(image);
    }
    for file in description.grids.keys() {
        if !files.contains(file.as_str()) {
            return Err(invalid(format!("sheet.json: missing image {file}")));
        }
    }
    for name in description.pivots.keys().chain(&description.tiles) {
        if !names.contains(name) {
            return Err(invalid(format!("sheet.json: missing frame {name}")));
        }
    }
    if frames.is_empty() {
        return Err(invalid("sheet folder has no PNG frames"));
    }
    frames.sort_unstable_by(|a, b| a.name.cmp(&b.name));
    pages::pack(&description, frames, &images)
}

#[expect(
    clippy::cast_precision_loss,
    reason = "source texel coordinates are evaluated as f32 in the renderer"
)]
fn frame_rect(
    description: &Description,
    name: &str,
    image: &RgbaImage,
    cell: [u32; 2],
    source_size: [u32; 2],
) -> Result<SheetFrame, CookError> {
    let tile = description.tiles.contains(name);
    let (mut trim, mut end) = (source_size, [0, 0]);
    for y in 0..source_size[1] {
        for x in 0..source_size[0] {
            if image.get_pixel(cell[0] + x, cell[1] + y).0[3] != 0 {
                trim = [trim[0].min(x), trim[1].min(y)];
                end = [end[0].max(x + 1), end[1].max(y + 1)];
            }
        }
    }
    // A tile retains its authored footprint, including any transparent outer texels.
    if tile {
        (trim, end) = ([0, 0], source_size);
    } else if end == [0, 0] {
        (trim, end) = ([0, 0], [1, 1]);
    }
    let size = [end[0] - trim[0], end[1] - trim[1]];
    let pivot = description
        .pivots
        .get(name)
        .copied()
        .unwrap_or_else(|| source_size.map(|side| side as f32 / 2.0));
    if !pivot.into_iter().all(f32::is_finite) {
        return Err(invalid(format!("{name}: pivot must be finite")));
    }
    Ok(SheetFrame {
        trim,
        size,
        source_size,
        pivot,
        tile: u32::from(tile),
        ..bytemuck::Zeroable::zeroed()
    })
}

impl PreparedSheet {
    pub(crate) fn encode(&self, profile: Profile, quality: Quality) -> Result<Vec<u8>, CookError> {
        let pages = match (self.meta.kind, profile) {
            (1, Profile::DesktopBc | Profile::AppleAstc) => self.compress(profile, quality)?,
            _ => ktx2_layers(
                Ktx2Format::Rgba8Srgb,
                self.width,
                self.height,
                0,
                self.meta.pages,
                &self.mips,
            ),
        };
        Ok(ContainerWriter::new(SHEET_MAGIC)
            .section(sheet_tag::META, &[self.meta])
            .section(sheet_tag::FRAMES, &self.records)
            .section(sheet_tag::NAMES, &self.names)
            .section(sheet_tag::PAGES, &pages)
            .finish())
    }

    fn compress(&self, profile: Profile, quality: Quality) -> Result<Vec<u8>, CookError> {
        let format = if profile == Profile::DesktopBc {
            Ktx2Format::Bc7Srgb
        } else {
            Ktx2Format::Astc4x4Srgb
        };
        let levels: Result<Vec<_>, CookError> = self
            .mips
            .iter()
            .enumerate()
            .map(|(mip, bytes)| {
                let (w, h) = (self.width >> mip, self.height >> mip);
                let stride = (w * h * 4) as usize;
                let mut out = Vec::new();
                for layer in bytes.chunks_exact(stride) {
                    let encoded = if profile == Profile::DesktopBc {
                        let settings = match quality {
                            Quality::Fast => intel_tex_2::bc7::alpha_ultra_fast_settings(),
                            Quality::Final => intel_tex_2::bc7::alpha_basic_settings(),
                        };
                        texture::compress_rgba(w, h, layer, |surface, out| {
                            intel_tex_2::bc7::compress_blocks_into(&settings, surface, out);
                        })
                    } else {
                        let preset = match quality {
                            Quality::Fast => astcenc_rs::PRESET_FAST,
                            Quality::Final => astcenc_rs::PRESET_THOROUGH,
                        };
                        texture::compress_astc(w, h, layer, false, preset)?
                    };
                    out.extend(encoded);
                }
                Ok(out)
            })
            .collect();
        Ok(ktx2_layers(
            format,
            self.width,
            self.height,
            0,
            self.meta.pages,
            &levels?,
        ))
    }
}
