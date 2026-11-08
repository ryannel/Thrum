//! Image to KTX2 texture product, and Thrum volume source to KTX2 3D volume product.
//!
//! Colour textures are sRGB by default; a `.linear.` infix (for example `rock_n.linear.png`)
//! marks data such as normal maps. Mips are filtered in linear light, so sRGB colour does not
//! darken with distance. Block compression runs in parallel over 4-row strips.

use rayon::prelude::*;
use thrum_asset::{Ktx2Format, Profile};

use crate::CookError;
use crate::write::ktx2;

/// Encoder effort; the dev loop favours latency, release builds favour quality.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Quality {
    /// Fast encoder settings for edit iteration.
    Fast,
    /// Slower, higher-quality encoder settings for shipping products.
    Final,
}

/// Volume source magic: `TVOL`, then width, height and depth as `u32`, then `u8` density voxels
/// in x-fastest order.
pub(crate) const VOLUME_MAGIC: [u8; 4] = *b"TVOL";
const VOLUME_HEADER: usize = 16;

/// Cook an encoded image (PNG or JPEG) into a mip-mapped KTX2 texture for `profile`.
pub(crate) fn cook_texture(
    bytes: &[u8],
    linear: bool,
    profile: Profile,
    quality: Quality,
) -> Result<Vec<u8>, CookError> {
    let image = image::load_from_memory(bytes)
        .map_err(|error| CookError::Source(error.to_string()))?
        .to_rgba8();
    let (width, height) = image.dimensions();
    if profile != Profile::Portable {
        whole_blocks(profile, width, height)?;
    }
    let mips = mip_chain(width, height, image.into_raw(), !linear);
    let has_alpha = mips[0]
        .2
        .as_chunks::<4>()
        .0
        .iter()
        .any(|texel| texel[3] != 255);
    let (format, levels): (Ktx2Format, Vec<Vec<u8>>) = match profile {
        Profile::Portable => (
            if linear {
                Ktx2Format::Rgba8Unorm
            } else {
                Ktx2Format::Rgba8Srgb
            },
            mips.into_iter().map(|m| m.2).collect(),
        ),
        Profile::DesktopBc => {
            let settings = match (quality, has_alpha) {
                (Quality::Fast, false) => intel_tex_2::bc7::opaque_ultra_fast_settings(),
                (Quality::Fast, true) => intel_tex_2::bc7::alpha_ultra_fast_settings(),
                (Quality::Final, false) => intel_tex_2::bc7::opaque_basic_settings(),
                (Quality::Final, true) => intel_tex_2::bc7::alpha_basic_settings(),
            };
            let encode = |surface: &intel_tex_2::RgbaSurface<'_>, out: &mut [u8]| {
                intel_tex_2::bc7::compress_blocks_into(&settings, surface, out);
            };
            let format = if linear {
                Ktx2Format::Bc7Unorm
            } else {
                Ktx2Format::Bc7Srgb
            };
            (
                format,
                mips.iter()
                    .map(|(w, h, data)| compress_rgba(*w, *h, data, encode))
                    .collect(),
            )
        }
        Profile::AppleAstc => {
            let preset = if quality == Quality::Fast {
                astcenc_rs::PRESET_FAST
            } else {
                astcenc_rs::PRESET_THOROUGH
            };
            let format = if linear {
                Ktx2Format::Astc4x4Unorm
            } else {
                Ktx2Format::Astc4x4Srgb
            };
            let levels = mips
                .iter()
                .map(|(w, h, data)| compress_astc(*w, *h, data, linear, preset))
                .collect::<Result<_, _>>()?;
            (format, levels)
        }
    };
    Ok(ktx2(format, width, height, 0, &levels))
}

/// Cook a `TVOL` density volume into a mip-mapped 3D KTX2 for `profile`: BC4 sliced 3D on
/// desktop, R8 elsewhere.
pub(crate) fn cook_volume(bytes: &[u8], profile: Profile) -> Result<Vec<u8>, CookError> {
    let (width, height, depth, voxels) = parse_volume(bytes)?;
    if profile == Profile::DesktopBc {
        whole_blocks(profile, width, height)?;
    }
    let mut levels = vec![(width, height, depth, voxels.to_vec())];
    while let Some(&(w, h, d, ref data)) = levels.last() {
        if w == 1 && h == 1 && d == 1 {
            break;
        }
        levels.push(downsample_volume(w, h, d, data));
    }
    let (format, data) = match profile {
        Profile::DesktopBc => (
            Ktx2Format::Bc4Unorm,
            levels
                .iter()
                .map(|(w, h, d, data)| compress_bc4_slices(*w, *h, *d, data))
                .collect::<Vec<_>>(),
        ),
        Profile::Portable | Profile::AppleAstc => (
            Ktx2Format::R8Unorm,
            levels.into_iter().map(|level| level.3).collect(),
        ),
    };
    Ok(ktx2(format, width, height, depth, &data))
}

/// GPU APIs require a block-compressed base level to span whole 4×4 blocks, so a compressed
/// product of any other size could never be created; refuse it by name instead.
fn whole_blocks(profile: Profile, width: u32, height: u32) -> Result<(), CookError> {
    if width.is_multiple_of(4) && height.is_multiple_of(4) {
        return Ok(());
    }
    Err(CookError::Refused(format!(
        "{} needs a base size that is a multiple of 4 texels; {width}×{height} is served by the \
         portable product",
        profile.name()
    )))
}

/// Validate a `TVOL` source and return its extent and voxels.
pub(crate) fn parse_volume(bytes: &[u8]) -> Result<(u32, u32, u32, &[u8]), CookError> {
    let header = bytes
        .get(..VOLUME_HEADER)
        .ok_or_else(|| CookError::Source("volume header".into()))?;
    if header[..4] != VOLUME_MAGIC {
        return Err(CookError::Source("volume magic".into()));
    }
    let read = |at: usize| {
        u32::from_le_bytes([header[at], header[at + 1], header[at + 2], header[at + 3]])
    };
    let (width, height, depth) = (read(4), read(8), read(12));
    let voxels = u64::from(width) * u64::from(height) * u64::from(depth);
    if voxels == 0
        || width > 4096
        || height > 4096
        || depth > 4096
        || bytes.len() as u64 != VOLUME_HEADER as u64 + voxels
    {
        return Err(CookError::Source("volume extent".into()));
    }
    Ok((width, height, depth, &bytes[VOLUME_HEADER..]))
}

/// Encode a `TVOL` source file.
pub fn encode_volume(width: u32, height: u32, depth: u32, voxels: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(VOLUME_HEADER + voxels.len());
    out.extend_from_slice(&VOLUME_MAGIC);
    for value in [width, height, depth] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.extend_from_slice(voxels);
    out
}

fn srgb_to_linear(value: u8) -> f32 {
    let c = f32::from(value) / 255.0;
    if c <= 0.040_45 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "clamped to [0, 255] before conversion"
)]
fn to_u8(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn linear_to_srgb(value: f32) -> u8 {
    let c = value.clamp(0.0, 1.0);
    to_u8(if c <= 0.003_130_8 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    })
}

/// Full chain largest first as `(width, height, rgba)`; 2×2 box filter with edge clamping.
fn mip_chain(width: u32, height: u32, rgba: Vec<u8>, srgb: bool) -> Vec<(u32, u32, Vec<u8>)> {
    let decode = |value: u8, channel: usize| {
        if srgb && channel < 3 {
            srgb_to_linear(value)
        } else {
            f32::from(value) / 255.0
        }
    };
    let encode = |value: f32, channel: usize| {
        if srgb && channel < 3 {
            linear_to_srgb(value)
        } else {
            to_u8(value)
        }
    };
    let mut chain = vec![(width, height, rgba)];
    let mut linear: Vec<f32> = chain[0]
        .2
        .iter()
        .enumerate()
        .map(|(i, &v)| decode(v, i % 4))
        .collect();
    let (mut w, mut h) = (width, height);
    while w > 1 || h > 1 {
        let (next_w, next_h) = ((w / 2).max(1), (h / 2).max(1));
        let mut next = vec![0.0_f32; (next_w * next_h * 4) as usize];
        for y in 0..next_h {
            for x in 0..next_w {
                for channel in 0..4 {
                    let mut sum = 0.0;
                    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        let sx = (2 * x + dx).min(w - 1);
                        let sy = (2 * y + dy).min(h - 1);
                        sum += linear[((sy * w + sx) * 4) as usize + channel];
                    }
                    next[((y * next_w + x) * 4) as usize + channel] = sum / 4.0;
                }
            }
        }
        let bytes = next
            .iter()
            .enumerate()
            .map(|(i, &v)| encode(v, i % 4))
            .collect();
        chain.push((next_w, next_h, bytes));
        linear = next;
        (w, h) = (next_w, next_h);
    }
    chain
}

/// ARM's reference ASTC encoder, 4×4 blocks. Strips of 32 rows encode in parallel, each with a
/// single-threaded encoder context reused per worker.
fn compress_astc(
    width: u32,
    height: u32,
    rgba: &[u8],
    linear: bool,
    preset: astcenc_rs::Preset,
) -> Result<Vec<u8>, CookError> {
    const STRIP_ROWS: u32 = 32;
    let profile = if linear {
        astcenc_rs::Profile::LdrRgba
    } else {
        astcenc_rs::Profile::LdrSrgb
    };
    let configure = || {
        astcenc_rs::ConfigBuilder::new()
            .with_profile(profile)
            .with_preset(preset)
            .with_block_size(astcenc_rs::Extents::new(4, 4))
            .build()
            .and_then(|config| astcenc_rs::Context::with_threads(1, config))
            .map_err(|error| CookError::Source(format!("ASTC encoder: {error:?}")))
    };
    let row_bytes = width as usize * 4;
    let strips: Vec<(u32, &[u8])> = rgba
        .chunks(row_bytes * STRIP_ROWS as usize)
        .map(|rows| (u32::try_from(rows.len() / row_bytes).unwrap_or(0), rows))
        .collect();
    let encoded: Vec<Result<Vec<u8>, CookError>> = strips
        .par_iter()
        .map_init(configure, |context, (rows, data)| {
            let context = context
                .as_mut()
                .map_err(|error| CookError::Source(error.to_string()))?;
            let image = astcenc_rs::Image {
                extents: astcenc_rs::Extents::new(width, *rows),
                data: vec![*data],
            };
            context
                .compress(&image, astcenc_rs::Swizzle::rgba())
                .map_err(|error| CookError::Source(format!("ASTC encoder: {error:?}")))
        })
        .collect();
    let mut out = Vec::with_capacity(
        usize::try_from(Ktx2Format::Astc4x4Srgb.level_bytes(width, height, 1)).unwrap_or(0),
    );
    for strip in encoded {
        out.extend_from_slice(&strip?);
    }
    Ok(out)
}

/// Pad to whole 4×4 blocks by edge replication, then encode 4-row strips in parallel.
fn compress_rgba(
    width: u32,
    height: u32,
    rgba: &[u8],
    encode: impl Fn(&intel_tex_2::RgbaSurface<'_>, &mut [u8]) + Sync,
) -> Vec<u8> {
    let (padded_width, padded_height) = (width.next_multiple_of(4), height.next_multiple_of(4));
    let padded = pad(width, height, padded_width, padded_height, 4, rgba);
    let strip_bytes = (padded_width / 4) as usize * 16;
    let mut out = vec![0; strip_bytes * (padded_height / 4) as usize];
    let row_bytes = padded_width as usize * 4;
    out.par_chunks_mut(strip_bytes)
        .enumerate()
        .for_each(|(strip, blocks)| {
            let data = &padded[strip * 4 * row_bytes..(strip + 1) * 4 * row_bytes];
            let surface = intel_tex_2::RgbaSurface {
                data,
                width: padded_width,
                height: 4,
                stride: padded_width * 4,
            };
            encode(&surface, blocks);
        });
    out
}

fn compress_bc4_slices(width: u32, height: u32, depth: u32, voxels: &[u8]) -> Vec<u8> {
    let (padded_width, padded_height) = (width.next_multiple_of(4), height.next_multiple_of(4));
    let slice = (width * height) as usize;
    let slice_out = (padded_width / 4 * padded_height / 4) as usize * 8;
    let mut out = vec![0; slice_out * depth as usize];
    out.par_chunks_mut(slice_out)
        .enumerate()
        .for_each(|(z, blocks)| {
            let padded = pad(
                width,
                height,
                padded_width,
                padded_height,
                1,
                &voxels[z * slice..(z + 1) * slice],
            );
            let surface = intel_tex_2::RSurface {
                data: &padded,
                width: padded_width,
                height: padded_height,
                stride: padded_width,
            };
            intel_tex_2::bc4::compress_blocks_into(&surface, blocks);
        });
    out
}

fn pad(
    width: u32,
    height: u32,
    padded_width: u32,
    padded_height: u32,
    channels: usize,
    data: &[u8],
) -> Vec<u8> {
    let mut out = vec![0; padded_width as usize * padded_height as usize * channels];
    for y in 0..padded_height {
        let source_y = y.min(height - 1) as usize;
        for x in 0..padded_width {
            let source = (source_y * width as usize + x.min(width - 1) as usize) * channels;
            let target = (y as usize * padded_width as usize + x as usize) * channels;
            out[target..target + channels].copy_from_slice(&data[source..source + channels]);
        }
    }
    out
}

fn downsample_volume(width: u32, height: u32, depth: u32, data: &[u8]) -> (u32, u32, u32, Vec<u8>) {
    let (nw, nh, nd) = ((width / 2).max(1), (height / 2).max(1), (depth / 2).max(1));
    let at = |x: u32, y: u32, z: u32| u32::from(data[((z * height + y) * width + x) as usize]);
    let mut out = Vec::with_capacity((nw * nh * nd) as usize);
    for z in 0..nd {
        for y in 0..nh {
            for x in 0..nw {
                let mut sum = 0;
                for (dx, dy, dz) in [
                    (0, 0, 0),
                    (1, 0, 0),
                    (0, 1, 0),
                    (1, 1, 0),
                    (0, 0, 1),
                    (1, 0, 1),
                    (0, 1, 1),
                    (1, 1, 1),
                ] {
                    sum += at(
                        (2 * x + dx).min(width - 1),
                        (2 * y + dy).min(height - 1),
                        (2 * z + dz).min(depth - 1),
                    );
                }
                out.push(u8::try_from((sum + 4) / 8).unwrap_or(u8::MAX));
            }
        }
    }
    (nw, nh, nd, out)
}
