//! Embedded frames are repacked on four-texel boundaries so their gutters survive every mip.

use super::GpuRect;
use crate::{
    error::RenderError,
    shader_constants::{
        SRGB_ENCODED_KNEE, SRGB_GAMMA_F32, SRGB_OFFSET, SRGB_SCALE, SRGB_SLOPE_F32,
    },
    sprite::{EmbeddedSheet, NamedSheet, SHEET_GUTTER as GUTTER, SHEET_MAX_MIP},
};

const MIP_LEVELS: u32 = SHEET_MAX_MIP + 1;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

#[derive(Clone, Copy)]
struct Placement {
    frame: GpuRect,
    block: [u32; 2],
}

fn padded(size: u32) -> u32 {
    size.next_multiple_of(GUTTER) + 2 * GUTTER
}

fn pack(sheet: &NamedSheet, limits: &wgpu::Limits) -> Option<(Vec<Placement>, [u32; 3])> {
    let max_side = limits.max_texture_dimension_2d / GUTTER * GUTTER;
    let widest = sheet.sheet.frames.iter().map(|r| padded(r.size[0])).max()?;
    let tallest = sheet.sheet.frames.iter().map(|r| padded(r.size[1])).max()?;
    if widest.max(tallest) > max_side {
        return None;
    }
    // Start near the source width. A tall row or a full page advances the shelf
    // or the layer, so an unusually shaped frame cannot block the rest.
    let width = widest.max(sheet.sheet.extent[0].next_multiple_of(GUTTER));
    let height = tallest.max(sheet.sheet.extent[1].next_multiple_of(GUTTER));
    let width = width.max(64).min(max_side);
    let height = height.max(64).min(max_side);
    let mut placements = Vec::with_capacity(sheet.sheet.frames.len());
    let mut tiles = vec![false; sheet.sheet.frames.len()];
    for &index in &sheet.tiles {
        *tiles.get_mut(index as usize)? = true;
    }
    let kind = sheet.sampling as u32;
    let (mut x, mut y, mut shelf, mut page) = (0, 0, 0, 0);
    let mut used_width = 0;
    let mut used_height = 0;
    for (rect, tile) in sheet.sheet.frames.iter().zip(tiles) {
        let block = rect.size.map(padded);
        if x + block[0] > width {
            x = 0;
            y += shelf;
            shelf = 0;
        }
        if y + block[1] > height {
            x = 0;
            y = 0;
            shelf = 0;
            page += 1;
            if page >= limits.max_texture_array_layers {
                return None;
            }
        }
        placements.push(Placement {
            frame: GpuRect {
                origin: [x + GUTTER, y + GUTTER],
                size: rect.size,
                page,
                kind,
                tile: u32::from(tile),
                padding: 0,
            },
            block,
        });
        x += block[0];
        shelf = shelf.max(block[1]);
        used_width = used_width.max(x);
        used_height = used_height.max(y + shelf);
    }
    Some((placements, [used_width, used_height, page + 1]))
}

pub(super) fn upload(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    sheet: &NamedSheet,
    table: &mut Vec<GpuRect>,
) -> Result<(wgpu::Texture, wgpu::TextureView), RenderError> {
    let (placements, [width, height, pages]) = pack(sheet, &device.limits())
        .ok_or_else(|| RenderError::InvalidSpriteSheet(sheet.name.clone()))?;
    let texture = texture(device, width, height, pages, MIP_LEVELS);
    for (rect, placement) in sheet.sheet.frames.iter().zip(placements) {
        write_frame(queue, &texture, &sheet.sheet, *rect, placement);
        table.push(placement.frame);
    }
    Ok(view(texture))
}

pub(super) fn placeholder(device: &wgpu::Device) -> (wgpu::Texture, wgpu::TextureView) {
    view(texture(device, 1, 1, 1, 1))
}

fn texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    pages: u32,
    levels: u32,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("embedded sprite sheet"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: pages,
        },
        mip_level_count: levels,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

fn view(texture: wgpu::Texture) -> (wgpu::Texture, wgpu::TextureView) {
    let view = texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    });
    (texture, view)
}

fn linear(rgba: &[u8]) -> [f32; 4] {
    let alpha = f32::from(rgba[3]) / 255.0;
    let mut values = [alpha; 4];
    for (dest, &code) in values[..3].iter_mut().zip(rgba) {
        let srgb = f32::from(code) / 255.0;
        *dest = alpha
            * if srgb <= SRGB_ENCODED_KNEE {
                srgb / SRGB_SLOPE_F32
            } else {
                ((srgb + SRGB_OFFSET) / SRGB_SCALE).powf(SRGB_GAMMA_F32)
            };
    }
    values
}

fn write_frame(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    sheet: &EmbeddedSheet,
    rect: crate::sprite::SpriteRect,
    placement: Placement,
) {
    let [width, height] = placement.block;
    let [origin_x, origin_y] = placement.frame.origin.map(|v| v - GUTTER);
    let tile = placement.frame.tile != 0;
    let mut pixels = vec![[0.0; 4]; width as usize * height as usize];
    for y in 0..height {
        for x in 0..width {
            let in_frame = x >= GUTTER
                && y >= GUTTER
                && x < GUTTER + rect.size[0]
                && y < GUTTER + rect.size[1];
            if in_frame || tile {
                let column = rect.origin[0] + x.saturating_sub(GUTTER).min(rect.size[0] - 1);
                let row = rect.origin[1] + y.saturating_sub(GUTTER).min(rect.size[1] - 1);
                let index = (row as usize * sheet.extent[0] as usize + column as usize) * 4;
                pixels[(y * width + x) as usize] = linear(&sheet.rgba[index..index + 4]);
            }
        }
    }
    let mut half_pixels = Vec::with_capacity(pixels.len());
    for mip in 0..MIP_LEVELS {
        let w = width >> mip;
        let h = height >> mip;
        if tile {
            extend_tile_edges(&mut pixels, [w, h], rect.size, mip);
        }
        half_pixels.clear();
        half_pixels.extend(
            pixels
                .iter()
                .map(|rgba| rgba.map(|value| half::f16::from_f32(value).to_bits())),
        );
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: mip,
                origin: wgpu::Origin3d {
                    x: origin_x >> mip,
                    y: origin_y >> mip,
                    z: placement.frame.page,
                },
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&half_pixels),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w * 8),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        if mip + 1 < MIP_LEVELS {
            box_mip(&mut pixels, w, h);
        }
    }
}

fn extend_tile_edges(
    pixels: &mut [[f32; 4]],
    [width, height]: [u32; 2],
    logical: [u32; 2],
    mip: u32,
) {
    let margin = GUTTER >> mip;
    let [right, bottom] = logical.map(|side| margin + side.div_ceil(1 << mip) - 1);
    for y in 0..height {
        for x in 0..width {
            let source = (y.clamp(margin, bottom) * width + x.clamp(margin, right)) as usize;
            pixels[(y * width + x) as usize] = pixels[source];
        }
    }
}

fn box_mip(pixels: &mut Vec<[f32; 4]>, width: u32, height: u32) {
    let width = width as usize;
    let reduced = width / 2;
    for y in 0..height as usize / 2 {
        for x in 0..reduced {
            let top = 2 * (y * width + x);
            let bottom = top + width;
            pixels[y * reduced + x] = std::array::from_fn(|channel| {
                (pixels[top][channel]
                    + pixels[top + 1][channel]
                    + pixels[bottom][channel]
                    + pixels[bottom + 1][channel])
                    * 0.25
            });
        }
    }
    pixels.truncate(reduced * height as usize / 2);
}
